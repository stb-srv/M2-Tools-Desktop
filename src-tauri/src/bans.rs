use rusqlite::Connection;
use serde::Serialize;

// Zeitgesteuerte Account-Sperren gibt es serverseitig nicht (siehe
// db/account.rs / commands.rs::ban_account - der Login-Server vergleicht nur
// `status == "OK"`, sonst wird der Wert wörtlich als Fehlermeldung gezeigt;
// keine Ablaufspalte, kein Cron). Diese lokale Tabelle emulier das: ein
// Eintrag merkt sich, wann eine Sperre automatisch wieder aufgehoben werden
// soll. Die tatsächliche Aufhebung passiert nur, wenn M2Manager läuft (siehe
// `due_bans` / commands.rs::process_due_bans, aufgerufen beim Öffnen des
// Account-Managers) - es gibt keinen Hintergrunddienst außerhalb der App.

#[derive(Debug, Clone, Serialize)]
pub struct BanRecord {
    pub id: i64,
    pub account_id: i64,
    pub login: String,
    pub reason: String,
    pub banned_at: String,
    pub unban_at: Option<String>,
    pub active: bool,
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<BanRecord> {
    let active: i32 = row.get(6)?;
    Ok(BanRecord {
        id: row.get(0)?,
        account_id: row.get(1)?,
        login: row.get(2)?,
        reason: row.get(3)?,
        banned_at: row.get(4)?,
        unban_at: row.get(5)?,
        active: active != 0,
    })
}

/// `days` = None bedeutet dauerhaft (kein `unban_at`, keine automatische
/// Aufhebung). Legt nur den lokalen Merker an - das Setzen von
/// `account.account.status` passiert separat in `commands::ban_account`,
/// bevor diese Funktion aufgerufen wird.
pub fn create_ban(conn: &Connection, account_id: i64, login: &str, reason: &str, days: Option<i64>) -> Result<i64, String> {
    let now = chrono::Local::now();
    let banned_at = now.to_rfc3339();
    let unban_at = days.map(|d| (now + chrono::Duration::days(d)).to_rfc3339());
    conn.execute(
        "INSERT INTO account_bans (account_id, login, reason, banned_at, unban_at, active) \
         VALUES (?1, ?2, ?3, ?4, ?5, 1)",
        rusqlite::params![account_id, login, reason, banned_at, unban_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Aktive Sperren zuerst, jeweils neueste zuerst.
pub fn list_bans(conn: &Connection) -> Result<Vec<BanRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, account_id, login, reason, banned_at, unban_at, active \
             FROM account_bans ORDER BY active DESC, id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_record).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn deactivate_ban(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("UPDATE account_bans SET active = 0 WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Aktive Sperren, deren `unban_at` bereits verstrichen ist - der Aufrufer
/// (commands::process_due_bans) setzt für jeden Treffer den echten
/// `account.account.status` zurück und deaktiviert danach den Eintrag hier.
pub fn due_bans(conn: &Connection) -> Result<Vec<BanRecord>, String> {
    let now = chrono::Local::now().to_rfc3339();
    let mut stmt = conn
        .prepare(
            "SELECT id, account_id, login, reason, banned_at, unban_at, active \
             FROM account_bans WHERE active = 1 AND unban_at IS NOT NULL AND unban_at <= ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([&now], row_to_record).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE account_bans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                account_id INTEGER NOT NULL,
                login TEXT NOT NULL,
                reason TEXT NOT NULL,
                banned_at TEXT NOT NULL,
                unban_at TEXT,
                active INTEGER NOT NULL DEFAULT 1
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn creates_lists_and_deactivates_a_ban() {
        let conn = scratch_conn();
        let id = create_ban(&conn, 42, "tester", "Cheating", Some(7)).expect("create failed");

        let bans = list_bans(&conn).expect("list failed");
        assert_eq!(bans.len(), 1);
        assert_eq!(bans[0].id, id);
        assert_eq!(bans[0].account_id, 42);
        assert_eq!(bans[0].login, "tester");
        assert!(bans[0].active);
        assert!(bans[0].unban_at.is_some());

        deactivate_ban(&conn, id).expect("deactivate failed");
        let bans = list_bans(&conn).expect("list failed");
        assert!(!bans[0].active);
    }

    #[test]
    fn permanent_ban_has_no_unban_at_and_is_never_due() {
        let conn = scratch_conn();
        create_ban(&conn, 1, "permabanned", "Betrug", None).expect("create failed");
        let bans = list_bans(&conn).expect("list failed");
        assert!(bans[0].unban_at.is_none());
        assert!(due_bans(&conn).expect("due failed").is_empty());
    }

    #[test]
    fn a_ban_in_the_past_is_due_a_future_one_is_not() {
        let conn = scratch_conn();
        let now = chrono::Local::now();
        conn.execute(
            "INSERT INTO account_bans (account_id, login, reason, banned_at, unban_at, active) \
             VALUES (1, 'past', 'x', ?1, ?2, 1)",
            rusqlite::params![now.to_rfc3339(), (now - chrono::Duration::days(1)).to_rfc3339()],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO account_bans (account_id, login, reason, banned_at, unban_at, active) \
             VALUES (2, 'future', 'x', ?1, ?2, 1)",
            rusqlite::params![now.to_rfc3339(), (now + chrono::Duration::days(1)).to_rfc3339()],
        )
        .unwrap();

        let due = due_bans(&conn).expect("due failed");
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].login, "past");
    }
}
