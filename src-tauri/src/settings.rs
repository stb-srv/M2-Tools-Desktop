use rusqlite::{Connection, OptionalExtension};
use std::path::Path;

pub fn init_db(app_data_dir: &Path) -> Result<Connection, String> {
    std::fs::create_dir_all(app_data_dir).map_err(|e| e.to_string())?;
    let db_path = app_data_dir.join("m2manager.sqlite");
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS connections (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL,
            name TEXT NOT NULL,
            host TEXT NOT NULL,
            port INTEGER NOT NULL,
            username TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS paths (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS module_import_batches (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            module_name TEXT NOT NULL,
            item_type INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            vnums_json TEXT NOT NULL,
            had_effects INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS deploy_history (
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
    .map_err(|e| e.to_string())?;
    Ok(conn)
}

pub fn get_path(conn: &Connection, key: &str) -> Result<Option<String>, String> {
    conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |row| {
        row.get(0)
    })
    .optional()
    .map_err(|e| e.to_string())
}

pub fn set_path(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO paths (key, value) VALUES (?1, ?2) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        rusqlite::params![key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
