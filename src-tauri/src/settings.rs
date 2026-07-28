use rusqlite::Connection;
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
        );",
    )
    .map_err(|e| e.to_string())?;
    Ok(conn)
}
