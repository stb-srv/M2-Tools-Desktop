use crate::db::shop::{EntityBrowsePage, ItemSearchResult};
use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

// Lokaler Spiegel von item_proto/mob_proto (nur vnum+Name), damit
// EntityBrowser.tsx wahlweise ohne MySQL-Netzwerk-Latenz durchblättern kann
// - siehe [[m2manager_activity_log]]-Plan-Nachfolger, Idee #4. Bewusst nur
// vnum+Name (kein Voll-Mirror aller Spalten): Bearbeiten liest immer live
// (get_item_proto/get_table_row), der Cache füttert ausschließlich die
// Auswahl-/Durchblätter-Liste - eine veraltete Anzeige dort ist harmlos
// (man sieht evtl. einen alten Namen), ein veralteter Bearbeiten-Formular-
// Inhalt wäre gefährlich, daher bewusst nicht gecacht.

#[derive(Debug, Clone, Serialize)]
pub struct CacheMeta {
    pub kind: String,
    pub synced_at: String,
    pub row_count: i64,
}

fn table_for(kind: &str) -> Result<&'static str, String> {
    match kind {
        "item" => Ok("entity_cache_item"),
        "mob" => Ok("entity_cache_mob"),
        other => Err(format!("Unbekannte Cache-Art: {other}")),
    }
}

/// Ersetzt den kompletten Cache-Inhalt für `kind` durch `rows` - ein
/// Voll-Sync, kein inkrementelles Update (die Quelle liefert ohnehin immer
/// den kompletten Tabelleninhalt, siehe `db::shop::fetch_all_entity_names`).
pub fn replace_all(conn: &Connection, kind: &str, rows: &[(u32, String)]) -> Result<(), String> {
    let table = table_for(kind)?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    tx.execute(&format!("DELETE FROM {table}"), [])
        .map_err(|e| e.to_string())?;
    {
        let mut stmt = tx
            .prepare(&format!("INSERT INTO {table} (vnum, name) VALUES (?1, ?2)"))
            .map_err(|e| e.to_string())?;
        for (vnum, name) in rows {
            stmt.execute(rusqlite::params![vnum, name]).map_err(|e| e.to_string())?;
        }
    }
    let synced_at = chrono::Local::now().to_rfc3339();
    tx.execute(
        "INSERT INTO entity_cache_meta (kind, synced_at, row_count) VALUES (?1, ?2, ?3) \
         ON CONFLICT(kind) DO UPDATE SET synced_at = excluded.synced_at, row_count = excluded.row_count",
        rusqlite::params![kind, synced_at, rows.len() as i64],
    )
    .map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}

pub fn get_meta(conn: &Connection, kind: &str) -> Result<Option<CacheMeta>, String> {
    table_for(kind)?;
    conn.query_row(
        "SELECT kind, synced_at, row_count FROM entity_cache_meta WHERE kind = ?1",
        [kind],
        |row| {
            Ok(CacheMeta {
                kind: row.get(0)?,
                synced_at: row.get(1)?,
                row_count: row.get(2)?,
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

/// Gleiche Filter-/Paginierungs-Semantik wie `db::shop::browse_table` (leere
/// Suche = alles nach vnum sortiert, reine Ziffern = Teilstring auf der vnum
/// als Text, sonst Teilstring auf dem Namen) - nur gegen die lokale
/// SQLite-Kopie statt gegen die entfernte MySQL.
pub fn browse(conn: &Connection, kind: &str, query: Option<&str>, offset: i64, limit: i64) -> Result<EntityBrowsePage, String> {
    let table = table_for(kind)?;
    let trimmed = query.map(str::trim).filter(|q| !q.is_empty());
    let (where_clause, like_value): (&str, Option<String>) = match trimmed {
        None => ("", None),
        Some(q) if q.chars().all(|c| c.is_ascii_digit()) => ("WHERE CAST(vnum AS TEXT) LIKE ?1", Some(format!("%{q}%"))),
        Some(q) => (
            "WHERE LOWER(name) LIKE LOWER(?1)",
            Some(format!("%{}%", q.replace('%', "\\%").replace('_', "\\_"))),
        ),
    };

    let count_sql = format!("SELECT COUNT(*) FROM {table} {where_clause}");
    let total: i64 = if let Some(v) = &like_value {
        conn.query_row(&count_sql, [v], |row| row.get(0))
    } else {
        conn.query_row(&count_sql, [], |row| row.get(0))
    }
    .map_err(|e| e.to_string())?;

    let rows_sql = format!("SELECT vnum, name FROM {table} {where_clause} ORDER BY vnum LIMIT ?{n} OFFSET ?{n2}",
        n = if like_value.is_some() { 2 } else { 1 },
        n2 = if like_value.is_some() { 3 } else { 2 });
    let mut stmt = conn.prepare(&rows_sql).map_err(|e| e.to_string())?;
    let row_iter = |row: &rusqlite::Row| -> rusqlite::Result<ItemSearchResult> {
        Ok(ItemSearchResult {
            vnum: row.get(0)?,
            name: row.get(1)?,
        })
    };
    let rows: Vec<ItemSearchResult> = if let Some(v) = &like_value {
        stmt.query_map(rusqlite::params![v, limit, offset], row_iter)
    } else {
        stmt.query_map(rusqlite::params![limit, offset], row_iter)
    }
    .map_err(|e| e.to_string())?
    .collect::<Result<_, _>>()
    .map_err(|e| e.to_string())?;

    Ok(EntityBrowsePage { rows, total })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE entity_cache_item (vnum INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE entity_cache_mob (vnum INTEGER PRIMARY KEY, name TEXT NOT NULL);
             CREATE TABLE entity_cache_meta (kind TEXT PRIMARY KEY, synced_at TEXT NOT NULL, row_count INTEGER NOT NULL);",
        )
        .expect("create tables");
        conn
    }

    #[test]
    fn replace_all_populates_rows_and_meta() {
        let conn = scratch_conn();
        replace_all(&conn, "item", &[(1, "Schwert".to_string()), (2, "Schild".to_string())]).unwrap();

        let page = browse(&conn, "item", None, 0, 10).unwrap();
        assert_eq!(page.total, 2);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].vnum, 1);

        let meta = get_meta(&conn, "item").unwrap().unwrap();
        assert_eq!(meta.row_count, 2);
    }

    #[test]
    fn replace_all_clears_previous_content() {
        let conn = scratch_conn();
        replace_all(&conn, "item", &[(1, "Alt".to_string())]).unwrap();
        replace_all(&conn, "item", &[(2, "Neu".to_string())]).unwrap();

        let page = browse(&conn, "item", None, 0, 10).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].vnum, 2);
    }

    #[test]
    fn browse_filters_by_name_substring() {
        let conn = scratch_conn();
        replace_all(&conn, "mob", &[(1, "Wildhund".to_string()), (2, "Wolf".to_string())]).unwrap();
        let page = browse(&conn, "mob", Some("wild"), 0, 10).unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.rows[0].name, "Wildhund");
    }

    #[test]
    fn browse_filters_numeric_query_as_vnum_substring() {
        let conn = scratch_conn();
        replace_all(&conn, "item", &[(7000, "A".to_string()), (17001, "B".to_string()), (2, "C".to_string())]).unwrap();
        let page = browse(&conn, "item", Some("700"), 0, 10).unwrap();
        assert_eq!(page.total, 2);
    }

    #[test]
    fn browse_paginates() {
        let conn = scratch_conn();
        let rows: Vec<_> = (1..=5).map(|v| (v, format!("Item{v}"))).collect();
        replace_all(&conn, "item", &rows).unwrap();
        let page = browse(&conn, "item", None, 2, 2).unwrap();
        assert_eq!(page.total, 5);
        assert_eq!(page.rows.len(), 2);
        assert_eq!(page.rows[0].vnum, 3);
    }

    #[test]
    fn unknown_kind_is_rejected() {
        let conn = scratch_conn();
        assert!(replace_all(&conn, "npc", &[]).is_err());
        assert!(browse(&conn, "npc", None, 0, 10).is_err());
    }
}
