use rusqlite::Connection;
use serde::Serialize;

// Verlauf jedes "Server-Quellcode Bauen & Einspielen"-Vorgangs - siehe
// commands::run_deploy/run_rollback. Modelliert nach import_history.rs'
// module_import_batches-Muster (gleiche Bibliothek, gleiche Struktur), aber
// eine eigene Tabelle, da hier ein völlig anderer Vorgang festgehalten wird
// (Programmdatei ersetzen statt Item-Import) mit eigenen Feldern
// (Sicherungspfade, Erfolg der Live-Prüfung, Rückgängig-machen-Verkettung).

#[derive(Debug, Clone, Serialize)]
pub struct DeployRecord {
    pub id: i64,
    pub kind: String,
    pub targets: Vec<String>,
    pub created_at: String,
    pub game_backup_path: Option<String>,
    pub db_backup_path: Option<String>,
    pub note: Option<String>,
    pub success: Option<bool>,
    pub rolled_back_from: Option<i64>,
}

/// Legt einen neuen Verlauf-Eintrag an - `kind` ist `"deploy"` oder
/// `"rollback"`. `success` startet immer als `NULL` (unbekannt/läuft noch)
/// und wird per `update_deploy_success` nachgetragen, sobald die Live-Prüfung
/// nach dem Neustart durchgelaufen ist.
#[allow(clippy::too_many_arguments)]
pub fn record_deploy(
    conn: &Connection,
    kind: &str,
    targets: &[String],
    game_backup_path: Option<&str>,
    db_backup_path: Option<&str>,
    note: &str,
    rolled_back_from: Option<i64>,
) -> Result<i64, String> {
    let targets_json = serde_json::to_string(targets).map_err(|e| e.to_string())?;
    let created_at = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO deploy_history \
         (kind, targets_json, created_at, game_backup_path, db_backup_path, note, success, rolled_back_from) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, NULL, ?7)",
        rusqlite::params![kind, targets_json, created_at, game_backup_path, db_backup_path, note, rolled_back_from],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn update_deploy_success(conn: &Connection, id: i64, success: bool) -> Result<(), String> {
    conn.execute(
        "UPDATE deploy_history SET success = ?1 WHERE id = ?2",
        rusqlite::params![success as i32, id],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<DeployRecord> {
    let targets_json: String = row.get(2)?;
    let success: Option<i32> = row.get(7)?;
    Ok(DeployRecord {
        id: row.get(0)?,
        kind: row.get(1)?,
        targets: serde_json::from_str(&targets_json).unwrap_or_default(),
        created_at: row.get(3)?,
        game_backup_path: row.get(4)?,
        db_backup_path: row.get(5)?,
        note: row.get(6)?,
        success: success.map(|v| v != 0),
        rolled_back_from: row.get(8)?,
    })
}

pub fn list_deploys(conn: &Connection) -> Result<Vec<DeployRecord>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, kind, targets_json, created_at, game_backup_path, db_backup_path, note, success, rolled_back_from \
             FROM deploy_history ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], row_to_record).map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_deploy(conn: &Connection, id: i64) -> Result<Option<DeployRecord>, String> {
    Ok(list_deploys(conn)?.into_iter().find(|d| d.id == id))
}

/// Neuestes `"deploy"` (nicht `"rollback"`) - der übliche Standard für
/// "auf welchen Stand soll ich zurückrollen, wenn der Nutzer keinen
/// bestimmten Verlauf-Eintrag ausgewählt hat".
pub fn latest_deploy(conn: &Connection) -> Result<Option<DeployRecord>, String> {
    Ok(list_deploys(conn)?.into_iter().find(|d| d.kind == "deploy"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE deploy_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                kind TEXT NOT NULL,
                targets_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                game_backup_path TEXT,
                db_backup_path TEXT,
                note TEXT,
                success INTEGER,
                rolled_back_from INTEGER
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn records_lists_and_updates_success() {
        let conn = scratch_conn();
        let targets = vec!["game".to_string()];
        let id = record_deploy(&conn, "deploy", &targets, Some("/backup/game.bak"), None, "test note", None)
            .expect("record failed");

        let deploys = list_deploys(&conn).expect("list failed");
        assert_eq!(deploys.len(), 1);
        assert_eq!(deploys[0].id, id);
        assert_eq!(deploys[0].targets, vec!["game".to_string()]);
        assert_eq!(deploys[0].game_backup_path.as_deref(), Some("/backup/game.bak"));
        assert_eq!(deploys[0].success, None, "success should start unknown");

        update_deploy_success(&conn, id, true).expect("update failed");
        let fetched = get_deploy(&conn, id).expect("get failed").expect("missing");
        assert_eq!(fetched.success, Some(true));
    }

    #[test]
    fn latest_deploy_skips_rollback_rows() {
        let conn = scratch_conn();
        let targets = vec!["game".to_string()];
        let deploy_id = record_deploy(&conn, "deploy", &targets, None, None, "", None).unwrap();
        record_deploy(&conn, "rollback", &targets, None, None, "", Some(deploy_id)).unwrap();

        let latest = latest_deploy(&conn).expect("latest failed").expect("should find one");
        assert_eq!(latest.id, deploy_id);
        assert_eq!(latest.kind, "deploy");
    }

    #[test]
    fn lists_newest_first() {
        let conn = scratch_conn();
        let targets = vec!["db".to_string()];
        record_deploy(&conn, "deploy", &targets, None, None, "first", None).unwrap();
        record_deploy(&conn, "deploy", &targets, None, None, "second", None).unwrap();
        let deploys = list_deploys(&conn).expect("list failed");
        assert_eq!(deploys[0].note.as_deref(), Some("second"));
        assert_eq!(deploys[1].note.as_deref(), Some("first"));
    }
}
