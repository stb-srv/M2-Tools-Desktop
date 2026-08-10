use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct BroadcastMessage {
    pub id: i64,
    pub text: String,
    pub interval_minutes: i64,
    pub enabled: bool,
    pub revision: i64,
    pub created_at: String,
}

pub fn list_messages(conn: &Connection) -> Result<Vec<BroadcastMessage>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, text, interval_minutes, enabled, revision, created_at \
             FROM broadcast_messages ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let enabled: i64 = row.get(3)?;
            Ok(BroadcastMessage {
                id: row.get(0)?,
                text: row.get(1)?,
                interval_minutes: row.get(2)?,
                enabled: enabled != 0,
                revision: row.get(4)?,
                created_at: row.get(5)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn create_message(conn: &Connection, text: &str, interval_minutes: i64) -> Result<i64, String> {
    if text.trim().is_empty() {
        return Err("Text darf nicht leer sein.".to_string());
    }
    if interval_minutes < 1 {
        return Err("Intervall muss mindestens 1 Minute betragen.".to_string());
    }
    let created_at = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO broadcast_messages (text, interval_minutes, enabled, revision, created_at) \
         VALUES (?1, ?2, 1, 1, ?3)",
        rusqlite::params![text, interval_minutes, created_at],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Bumps `revision` on every edit - the generated quest file derives unique
/// timer/flag names from `<id>_<revision>`, so an edit always arms a fresh
/// timer on the next deploy+reload instead of colliding with whatever the
/// live server still has queued for the old name (see plan: `Reload()`
/// clears the name->NPC map but doesn't cancel already-pending timer events).
pub fn update_message(
    conn: &Connection,
    id: i64,
    text: &str,
    interval_minutes: i64,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Err("Text darf nicht leer sein.".to_string());
    }
    if interval_minutes < 1 {
        return Err("Intervall muss mindestens 1 Minute betragen.".to_string());
    }
    conn.execute(
        "UPDATE broadcast_messages SET text = ?1, interval_minutes = ?2, revision = revision + 1 \
         WHERE id = ?3",
        rusqlite::params![text, interval_minutes, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE broadcast_messages SET enabled = ?1, revision = revision + 1 WHERE id = ?2",
        rusqlite::params![enabled as i64, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_message(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM broadcast_messages WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE broadcast_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                text TEXT NOT NULL,
                interval_minutes INTEGER NOT NULL,
                enabled INTEGER NOT NULL DEFAULT 1,
                revision INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn creates_lists_and_deletes_a_message() {
        let conn = scratch_conn();
        let id = create_message(&conn, "Willkommen auf dem Server!", 30).expect("create failed");

        let messages = list_messages(&conn).expect("list failed");
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].id, id);
        assert_eq!(messages[0].text, "Willkommen auf dem Server!");
        assert_eq!(messages[0].interval_minutes, 30);
        assert!(messages[0].enabled);
        assert_eq!(messages[0].revision, 1);

        delete_message(&conn, id).expect("delete failed");
        assert!(list_messages(&conn).expect("list failed").is_empty());
    }

    #[test]
    fn update_bumps_revision() {
        let conn = scratch_conn();
        let id = create_message(&conn, "Alter Text", 10).expect("create failed");

        update_message(&conn, id, "Neuer Text", 20).expect("update failed");

        let messages = list_messages(&conn).expect("list failed");
        assert_eq!(messages[0].text, "Neuer Text");
        assert_eq!(messages[0].interval_minutes, 20);
        assert_eq!(messages[0].revision, 2);
    }

    #[test]
    fn set_enabled_toggles_and_bumps_revision() {
        let conn = scratch_conn();
        let id = create_message(&conn, "Text", 5).expect("create failed");

        set_enabled(&conn, id, false).expect("set_enabled failed");

        let messages = list_messages(&conn).expect("list failed");
        assert!(!messages[0].enabled);
        assert_eq!(messages[0].revision, 2);
    }

    #[test]
    fn rejects_empty_text_and_zero_interval() {
        let conn = scratch_conn();
        assert!(create_message(&conn, "   ", 10).is_err());
        assert!(create_message(&conn, "Text", 0).is_err());
    }
}
