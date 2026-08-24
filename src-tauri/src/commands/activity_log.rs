//! Zentrales Änderungsprotokoll.
//!
//! `log_activity` wird bewusst vom Frontend aufgerufen - einmal pro
//! abgeschlossener logischer Nutzeraktion (siehe `src/lib/logActivity.ts`),
//! nicht aus jedem einzelnen Rust-Schreib-Kommando heraus. Grund: mehrere
//! Module (allen voran der Item Editor) verketten mehrere Tauri-Kommandos zu
//! einer einzigen Nutzeraktion (Item speichern → DB-Insert → Beschreibung →
//! Icon → Repack → Deploy) - würde jedes einzelne Kommando selbst loggen,
//! entstünden mehrere Log-Zeilen pro Klick. Es gibt zudem keinen
//! Interceptor/Middleware-Punkt um `invoke_handler!` in lib.rs, über den sich
//! das zentral abfangen ließe.
//!
//! `list_activity_feed` mischt das generische `activity_log` (für alles, was
//! sonst kein eigenes Verlauf hätte) lesend mit den beiden bereits
//! bestehenden, spezialisierten Verläufen (`deploy_history`/
//! `import_history`) zu einer gemeinsamen Zeitleiste - diese beiden Module
//! bleiben unverändert und bekommen keine zusätzlichen `activity_log`-Zeilen
//! (sonst doppelt sichtbar).

use crate::activity_log;
use crate::deploy_history;
use crate::import_history;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct UndoRef {
    pub kind: String, // "rollback" | "undo_import"
    pub id: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityEntry {
    pub id: String,
    pub created_at: String,
    pub module: String,
    pub action: String,
    pub summary: String,
    pub source: String, // "log" | "deploy" | "import"
    pub undo: Option<UndoRef>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivityFeedPage {
    pub entries: Vec<ActivityEntry>,
    pub total: i64,
}

#[tauri::command]
pub fn log_activity(
    state: State<'_, AppState>,
    module: String,
    action: String,
    target_kind: Option<String>,
    target_ref: Option<String>,
    summary: String,
) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    activity_log::record(
        &conn,
        &module,
        &action,
        target_kind.as_deref(),
        target_ref.as_deref(),
        &summary,
    )?;
    Ok(())
}

fn note_suffix(note: &Option<String>) -> String {
    match note {
        Some(n) if !n.is_empty() => format!(" — {n}"),
        _ => String::new(),
    }
}

#[tauri::command]
pub fn list_activity_feed(
    state: State<'_, AppState>,
    module: Option<String>,
    search: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<ActivityFeedPage, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;

    let mut entries: Vec<ActivityEntry> = activity_log::list(&conn, module.as_deref(), search.as_deref())?
        .into_iter()
        .map(|r| ActivityEntry {
            id: format!("log:{}", r.id),
            created_at: r.created_at,
            module: r.module,
            action: r.action,
            summary: r.summary,
            source: "log".to_string(),
            undo: None,
        })
        .collect();

    // deploy_history/import_history sind klein genug (gelegentliche
    // Operationen, keine Massenaktion) - Filterung nach dem Laden in Rust,
    // statt eine zweite SQL-Filterlogik für diese beiden Tabellen zu bauen.
    let search_lower = search.as_deref().map(|s| s.to_lowercase());
    let module_matches = |m: &str| module.as_deref().map_or(true, |filter| filter == m);

    if module_matches("build-deploy") {
        for d in deploy_history::list_deploys(&conn)? {
            let summary = match d.kind.as_str() {
                "rollback" => format!("Rückgängig gemacht: {}{}", d.targets.join(", "), note_suffix(&d.note)),
                _ => format!("Bauen & Einspielen: {}{}", d.targets.join(", "), note_suffix(&d.note)),
            };
            if let Some(term) = &search_lower {
                if !summary.to_lowercase().contains(term.as_str()) {
                    continue;
                }
            }
            let undo = if d.kind == "deploy" {
                Some(UndoRef { kind: "rollback".to_string(), id: d.id })
            } else {
                None
            };
            entries.push(ActivityEntry {
                id: format!("deploy:{}", d.id),
                created_at: d.created_at,
                module: "build-deploy".to_string(),
                action: d.kind,
                summary,
                source: "deploy".to_string(),
                undo,
            });
        }
    }

    if module_matches("module-importer") {
        for b in import_history::list_batches(&conn)? {
            let summary = format!("{} importiert ({} Item(e))", b.module_name, b.vnums.len());
            if let Some(term) = &search_lower {
                if !summary.to_lowercase().contains(term.as_str()) {
                    continue;
                }
            }
            entries.push(ActivityEntry {
                id: format!("import:{}", b.id),
                created_at: b.created_at,
                module: "module-importer".to_string(),
                action: "import".to_string(),
                summary,
                source: "import".to_string(),
                undo: Some(UndoRef { kind: "undo_import".to_string(), id: b.id }),
            });
        }
    }

    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    let total = entries.len() as i64;
    let offset = offset.max(0) as usize;
    let limit = limit.max(0) as usize;
    let page = entries.into_iter().skip(offset).take(limit).collect();

    Ok(ActivityFeedPage { entries: page, total })
}
