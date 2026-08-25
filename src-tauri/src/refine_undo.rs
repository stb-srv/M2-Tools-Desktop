//! "Letzte Änderung rückgängig machen" für den Refine-Editor. Anders als
//! Box-/Cube-Editor (Datei-basiert, ein SFTP-Backup existiert schon vor
//! jedem Überschreiben) ist `refine_proto` reine DB-Zeilen ohne Datei-Backup
//! - hier wird deshalb der vorherige Zeilenstand vor jeder destruktiven
//! Schreibaktion (Update/Löschen/Anlegen) in eine eigene kleine lokale
//! SQLite-Tabelle gespiegelt (`refine_undo`, siehe `settings.rs::init_db`).
//! Bewusst nur die *letzte* Änderung (Single-Row-Tabelle, `id = 1`
//! erzwungen) statt eines vollen Verlaufs - das deckt den eigentlich
//! angefragten Anwendungsfall ("ups, falsch gespeichert") ab, ohne eine
//! komplette Undo-Stack-Infrastruktur für ein Nischen-Feature zu bauen.

use crate::refine::RefineRecipe;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineUndoState {
    pub action: String, // "update" | "delete" | "create"
    pub recipe_id: i32,
    /// The recipe's full state *before* the action - `None` for "create"
    /// (there is no "before", the recipe didn't exist yet).
    pub prior: Option<RefineRecipe>,
}

/// Overwrites the single stored undo snapshot - each new destructive action
/// replaces whatever was recorded before it, since only the last change is
/// undoable by design.
pub fn record(conn: &Connection, action: &str, recipe_id: i32, prior: Option<&RefineRecipe>) -> Result<(), String> {
    let prior_json = prior
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO refine_undo (id, recipe_id, action, prior_json, created_at) VALUES (1, ?1, ?2, ?3, ?4) \
         ON CONFLICT(id) DO UPDATE SET recipe_id = excluded.recipe_id, action = excluded.action, \
         prior_json = excluded.prior_json, created_at = excluded.created_at",
        rusqlite::params![recipe_id, action, prior_json, chrono::Local::now().to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Reads and clears the stored snapshot in one step ("consume once") so a
/// second undo attempt without a new change in between correctly reports
/// "nothing to undo" instead of re-applying the same restore.
pub fn take(conn: &Connection) -> Result<Option<RefineUndoState>, String> {
    let row: Option<(i32, String, Option<String>)> = conn
        .query_row(
            "SELECT recipe_id, action, prior_json FROM refine_undo WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    let Some((recipe_id, action, prior_json)) = row else {
        return Ok(None);
    };
    let prior = prior_json
        .map(|j| serde_json::from_str(&j))
        .transpose()
        .map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM refine_undo WHERE id = 1", [])
        .map_err(|e| e.to_string())?;
    Ok(Some(RefineUndoState { action, recipe_id, prior }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::refine::RefineMaterial;

    fn test_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE refine_undo (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                recipe_id INTEGER NOT NULL,
                action TEXT NOT NULL,
                prior_json TEXT,
                created_at TEXT NOT NULL
            );",
        )
        .unwrap();
        conn
    }

    fn sample_recipe(id: i32) -> RefineRecipe {
        RefineRecipe {
            id,
            cost: 1000,
            prob: 50,
            materials: vec![RefineMaterial { vnum: 42, count: 3, locale_name: None }],
            used_by_count: 0,
        }
    }

    #[test]
    fn take_returns_none_when_nothing_recorded() {
        let conn = test_conn();
        assert!(take(&conn).unwrap().is_none());
    }

    #[test]
    fn round_trips_an_update_snapshot() {
        let conn = test_conn();
        let prior = sample_recipe(7);
        record(&conn, "update", 7, Some(&prior)).unwrap();
        let taken = take(&conn).unwrap().unwrap();
        assert_eq!(taken.action, "update");
        assert_eq!(taken.recipe_id, 7);
        assert_eq!(taken.prior.unwrap().cost, 1000);
    }

    #[test]
    fn take_consumes_the_snapshot_so_a_second_call_returns_none() {
        let conn = test_conn();
        record(&conn, "create", 9, None).unwrap();
        assert!(take(&conn).unwrap().is_some());
        assert!(take(&conn).unwrap().is_none());
    }

    #[test]
    fn a_new_record_overwrites_the_previous_one() {
        let conn = test_conn();
        record(&conn, "update", 1, Some(&sample_recipe(1))).unwrap();
        record(&conn, "delete", 2, Some(&sample_recipe(2))).unwrap();
        let taken = take(&conn).unwrap().unwrap();
        assert_eq!(taken.recipe_id, 2);
        assert_eq!(taken.action, "delete");
    }
}
