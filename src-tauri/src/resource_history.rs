use rusqlite::Connection;
use serde::Serialize;

// Verlauf für die Server-Ressourcen-Übersicht (Dashboard) - bisher zeigte
// `get_server_resource_usage`/`get_server_overview` nur den Live-Snapshot im
// Moment des Aufrufs. Ein Datenpunkt wird bei jedem bestehenden
// CrashWatch-Poll (alle 60s, siehe src/components/CrashWatch.tsx) zusätzlich
// hier abgelegt, statt einen zweiten Timer zu starten. Nur Felder loggen, die
// von den bestehenden Kommandos ohnehin schon berechnet werden - keine neue
// Server-Datenquelle erfinden.

#[derive(Debug, Clone, Serialize)]
pub struct ResourceHistoryPoint {
    pub id: i64,
    pub created_at: String,
    pub cpu_percent: f64,
    pub ram_used_bytes: Option<i64>,
    pub ram_total_bytes: Option<i64>,
    pub disk_capacity_percent: Option<i64>,
}

/// `cpu_percent` is the caller's aggregate (e.g. summed across the matched
/// game-server processes) - this module doesn't know about `ProcessUsage`
/// itself to avoid a dependency from a generic history log back onto the
/// SSH/process-parsing module.
pub fn record(
    conn: &Connection,
    cpu_percent: f64,
    ram_used_bytes: Option<i64>,
    ram_total_bytes: Option<i64>,
    disk_capacity_percent: Option<i64>,
) -> Result<i64, String> {
    let created_at = chrono::Local::now().to_rfc3339();
    conn.execute(
        "INSERT INTO resource_history (created_at, cpu_percent, ram_used_bytes, ram_total_bytes, disk_capacity_percent) \
         VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params![created_at, cpu_percent, ram_used_bytes, ram_total_bytes, disk_capacity_percent],
    )
    .map_err(|e| e.to_string())?;
    let id = conn.last_insert_rowid();
    prune(conn)?;
    Ok(id)
}

/// Keeps the table from growing forever - a 60s poll interval means ~1440
/// rows/day, so keeping the newest 2000 rows covers a bit over a day of
/// history without needing a separate cleanup job/cron.
const MAX_ROWS: i64 = 2000;

fn prune(conn: &Connection) -> Result<(), String> {
    conn.execute(
        "DELETE FROM resource_history WHERE id NOT IN (
            SELECT id FROM resource_history ORDER BY id DESC LIMIT ?1
        )",
        rusqlite::params![MAX_ROWS],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn row_to_point(row: &rusqlite::Row) -> rusqlite::Result<ResourceHistoryPoint> {
    Ok(ResourceHistoryPoint {
        id: row.get(0)?,
        created_at: row.get(1)?,
        cpu_percent: row.get(2)?,
        ram_used_bytes: row.get(3)?,
        ram_total_bytes: row.get(4)?,
        disk_capacity_percent: row.get(5)?,
    })
}

/// Returns the newest `limit` points, oldest first (chart-ready order).
pub fn list_recent(conn: &Connection, limit: i64) -> Result<Vec<ResourceHistoryPoint>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, created_at, cpu_percent, ram_used_bytes, ram_total_bytes, disk_capacity_percent \
             FROM resource_history ORDER BY id DESC LIMIT ?1",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![limit], row_to_point)
        .map_err(|e| e.to_string())?;
    let mut points = rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
    points.reverse();
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE resource_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                created_at TEXT NOT NULL,
                cpu_percent REAL NOT NULL,
                ram_used_bytes INTEGER,
                ram_total_bytes INTEGER,
                disk_capacity_percent INTEGER
            );",
        )
        .expect("create table");
        conn
    }

    #[test]
    fn records_and_lists_oldest_first() {
        let conn = scratch_conn();
        record(&conn, 12.5, Some(1000), Some(4000), Some(42)).unwrap();
        record(&conn, 15.0, Some(1100), Some(4000), Some(42)).unwrap();
        let points = list_recent(&conn, 10).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].cpu_percent, 12.5);
        assert_eq!(points[1].cpu_percent, 15.0);
    }

    #[test]
    fn list_recent_respects_limit() {
        let conn = scratch_conn();
        for i in 0..10 {
            record(&conn, i as f64, None, None, None).unwrap();
        }
        let points = list_recent(&conn, 3).unwrap();
        assert_eq!(points.len(), 3);
        // newest 3 (cpu 7,8,9), oldest first
        assert_eq!(points.iter().map(|p| p.cpu_percent).collect::<Vec<_>>(), vec![7.0, 8.0, 9.0]);
    }

    #[test]
    fn prunes_beyond_max_rows() {
        let conn = scratch_conn();
        conn.execute("DELETE FROM resource_history", []).unwrap();
        for i in 0..(MAX_ROWS + 50) {
            record(&conn, i as f64, None, None, None).unwrap();
        }
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM resource_history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, MAX_ROWS);
        // oldest rows are the ones dropped, newest survive
        let points = list_recent(&conn, MAX_ROWS);
        let points = points.unwrap();
        assert_eq!(points.last().unwrap().cpu_percent, (MAX_ROWS + 49) as f64);
    }

    #[test]
    fn optional_fields_can_be_null() {
        let conn = scratch_conn();
        record(&conn, 5.0, None, None, None).unwrap();
        let points = list_recent(&conn, 10).unwrap();
        assert_eq!(points[0].ram_used_bytes, None);
        assert_eq!(points[0].ram_total_bytes, None);
        assert_eq!(points[0].disk_capacity_percent, None);
    }
}
