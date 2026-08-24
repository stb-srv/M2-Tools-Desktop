use rusqlite::Connection;
use serde::Serialize;

// Zentrales Änderungsprotokoll - ergänzt deploy_history.rs/import_history.rs
// (die weiterhin eigenständig bleiben, siehe commands/activity_log.rs für die
// gemeinsame Übersicht) um alle anderen ~45 Schreib-Kommandos, die bisher gar
// kein Audit-Trail hatten. Wird bewusst vom Frontend aufgerufen (einmal pro
// abgeschlossener logischer Nutzeraktion), nicht aus jedem einzelnen
// Rust-Kommando heraus - siehe Plan/Kommentar in commands/activity_log.rs.

#[derive(Debug, Clone, Serialize)]
pub struct ActivityRecord {
    pub id: i64,
    pub created_at: String,
    pub module: String,
    pub action: String,
    pub target_kind: Option<String>,
    pub target_ref: Option<String>,
    pub summary: String,
}

pub fn record(
    conn: &Connection,
    module: &str,
    action: &str,
    target_kind: Option<&str>,
    target_ref: Option<&str>,
    summary: &str,
) -> Result<i64, String> {
    let created_at = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO activity_log (created_at, module, action, target_kind, target_ref, summary) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![created_at, module, action, target_kind, target_ref, summary],
    )
    .map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

fn row_to_record(row: &rusqlite::Row) -> rusqlite::Result<ActivityRecord> {
    Ok(ActivityRecord {
        id: row.get(0)?,
        created_at: row.get(1)?,
        module: row.get(2)?,
        action: row.get(3)?,
        target_kind: row.get(4)?,
        target_ref: row.get(5)?,
        summary: row.get(6)?,
    })
}

/// `module`/`search` filter server-side (SQL) since this is the only source
/// of the three merged in `list_activity_feed` that can grow large (every
/// non-history-having write command logs here) - `deploy_history`/
/// `import_history` stay small enough to filter in Rust after fetching all
/// rows (see commands/activity_log.rs).
pub fn list(conn: &Connection, module: Option<&str>, search: Option<&str>) -> Result<Vec<ActivityRecord>, String> {
    let search_pattern = search.map(|s| format!("%{s}%"));
    let mut stmt = conn
        .prepare(
            "SELECT id, created_at, module, action, target_kind, target_ref, summary \
             FROM activity_log \
             WHERE (?1 IS NULL OR module = ?1) \
               AND (?2 IS NULL OR summary LIKE ?2) \
             ORDER BY id DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![module, search_pattern], row_to_record)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                module TEXT NOT NULL,
                action TEXT NOT NULL,
                target_kind TEXT,
                target_ref TEXT,
                summary TEXT NOT NULL
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn records_and_lists() {
        let conn = scratch_conn();
        let id = record(
            &conn,
            "item-editor",
            "update",
            Some("item"),
            Some("3219"),
            "Item 3219 ('Schwert+9') aktualisiert",
        )
        .expect("record failed");

        let entries = list(&conn, None, None).expect("list failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, id);
        assert_eq!(entries[0].module, "item-editor");
        assert_eq!(entries[0].target_ref.as_deref(), Some("3219"));
    }

    #[test]
    fn lists_newest_first() {
        let conn = scratch_conn();
        record(&conn, "shop-editor", "create", None, None, "first").unwrap();
        record(&conn, "shop-editor", "create", None, None, "second").unwrap();
        let entries = list(&conn, None, None).expect("list failed");
        assert_eq!(entries[0].summary, "second");
        assert_eq!(entries[1].summary, "first");
    }

    #[test]
    fn filters_by_module() {
        let conn = scratch_conn();
        record(&conn, "item-editor", "update", None, None, "item change").unwrap();
        record(&conn, "shop-editor", "create", None, None, "shop change").unwrap();
        let entries = list(&conn, Some("shop-editor"), None).expect("list failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].module, "shop-editor");
    }

    #[test]
    fn filters_by_search_text() {
        let conn = scratch_conn();
        record(&conn, "item-editor", "update", None, None, "Item 3219 aktualisiert").unwrap();
        record(&conn, "item-editor", "delete", None, None, "Item 500 gelöscht").unwrap();
        let entries = list(&conn, None, Some("3219")).expect("list failed");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].summary, "Item 3219 aktualisiert");
    }
}
