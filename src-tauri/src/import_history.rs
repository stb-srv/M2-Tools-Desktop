use rusqlite::Connection;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ImportBatch {
    pub id: i64,
    pub module_name: String,
    pub item_type: i32,
    pub created_at: String,
    pub vnums: Vec<u32>,
    pub had_effects: bool,
}

/// Records a completed Modul-Importer batch so it can be undone later (even
/// after restarting the app) - see `commands::undo_import_batch`. Written
/// once, right after the import pipeline finishes successfully.
pub fn record_batch(
    conn: &Connection,
    module_name: &str,
    item_type: i32,
    vnums: &[u32],
    had_effects: bool,
) -> Result<i64, String> {
    let vnums_json = serde_json::to_string(vnums).map_err(|e| e.to_string())?;
    let created_at = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO module_import_batches (module_name, item_type, created_at, vnums_json, had_effects) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![module_name, item_type, created_at, vnums_json, had_effects as i32],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

pub fn list_batches(conn: &Connection) -> Result<Vec<ImportBatch>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, module_name, item_type, created_at, vnums_json, had_effects \
             FROM module_import_batches ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let vnums_json: String = row.get(4)?;
            let had_effects: i32 = row.get(5)?;
            Ok(ImportBatch {
                id: row.get(0)?,
                module_name: row.get(1)?,
                item_type: row.get(2)?,
                created_at: row.get(3)?,
                vnums: serde_json::from_str(&vnums_json).unwrap_or_default(),
                had_effects: had_effects != 0,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn get_batch(conn: &Connection, id: i64) -> Result<Option<ImportBatch>, String> {
    Ok(list_batches(conn)?.into_iter().find(|b| b.id == id))
}

pub fn delete_batch_record(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM module_import_batches WHERE id = ?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE module_import_batches (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                module_name TEXT NOT NULL,
                item_type INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                vnums_json TEXT NOT NULL,
                had_effects INTEGER NOT NULL DEFAULT 0
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn records_and_lists_and_deletes_a_batch() {
        let conn = scratch_conn();
        let id = record_batch(&conn, "FrostEdge", 1, &[500000, 500001, 500002], true).expect("record failed");

        let batches = list_batches(&conn).expect("list failed");
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].id, id);
        assert_eq!(batches[0].module_name, "FrostEdge");
        assert_eq!(batches[0].vnums, vec![500000, 500001, 500002]);
        assert!(batches[0].had_effects);

        let fetched = get_batch(&conn, id).expect("get failed").expect("batch missing");
        assert_eq!(fetched.module_name, "FrostEdge");

        delete_batch_record(&conn, id).expect("delete failed");
        assert!(list_batches(&conn).expect("list failed").is_empty());
    }

    #[test]
    fn lists_newest_first() {
        let conn = scratch_conn();
        record_batch(&conn, "First", 1, &[1], false).unwrap();
        record_batch(&conn, "Second", 1, &[2], false).unwrap();
        let batches = list_batches(&conn).expect("list failed");
        assert_eq!(batches[0].module_name, "Second");
        assert_eq!(batches[1].module_name, "First");
    }
}
