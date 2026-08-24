//! Lokaler Read-Cache für item_proto/mob_proto (Idee #4 der Session vom
//! 2026-08-24, siehe [[m2manager_activity_log]]-Nachfolgeplan) - siehe
//! `entity_cache.rs` für die eigentliche SQLite-Logik. `EntityBrowser.tsx`
//! ist die einzige Stelle, die diese Kommandos aufruft.

use super::support::require_pool;
use crate::db::shop::{fetch_all_entity_names, EntityBrowsePage};
use crate::entity_cache::{self, CacheMeta};
use crate::state::AppState;
use tauri::State;

fn table_for(kind: &str) -> Result<&'static str, String> {
    match kind {
        "item" => Ok("item_proto"),
        "mob" => Ok("mob_proto"),
        other => Err(format!("Unbekannte Cache-Art: {other}")),
    }
}

#[tauri::command]
pub async fn sync_entity_cache(state: State<'_, AppState>, kind: String) -> Result<CacheMeta, String> {
    let table = table_for(&kind)?;
    let pool = require_pool(&state).await?;
    let rows = fetch_all_entity_names(&pool, table).await?;
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    entity_cache::replace_all(&conn, &kind, &rows)?;
    entity_cache::get_meta(&conn, &kind)?.ok_or_else(|| "Cache-Sync fehlgeschlagen.".to_string())
}

#[tauri::command]
pub fn get_entity_cache_meta(state: State<'_, AppState>, kind: String) -> Result<Option<CacheMeta>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    entity_cache::get_meta(&conn, &kind)
}

#[tauri::command]
pub fn browse_entities_cached(
    state: State<'_, AppState>,
    kind: String,
    query: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<EntityBrowsePage, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    entity_cache::browse(&conn, &kind, query.as_deref(), offset, limit)
}
