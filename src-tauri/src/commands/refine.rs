//! Refine-Editor (Aufwertungs-Ketten).

use crate::refine;
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

#[tauri::command]
pub async fn get_refine_chain(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Vec<refine::RefineChainNode>, String> {
    let pool = require_pool(&state).await?;
    refine::get_refine_chain(&pool, vnum).await
}

#[tauri::command]
pub async fn get_refine_recipe(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<refine::RefineRecipe>, String> {
    let pool = require_pool(&state).await?;
    refine::get_recipe(&pool, id).await
}

#[tauri::command]
pub async fn save_refine_recipe(
    state: State<'_, AppState>,
    id: Option<i32>,
    cost: i32,
    prob: i16,
    materials: Vec<refine::RefineMaterial>,
) -> Result<i32, String> {
    let pool = require_pool(&state).await?;

    // Snapshot what this write is about to overwrite/create, for the
    // "letzte Änderung rückgängig machen" button - best-effort, a failure
    // here (e.g. local DB briefly locked) shouldn't block the actual save.
    if let Some(existing_id) = id {
        if let Ok(Some(prior)) = refine::get_recipe(&pool, existing_id).await {
            if let Ok(conn) = state.settings_db.lock() {
                let _ = crate::refine_undo::record(&conn, "update", existing_id, Some(&prior));
            }
        }
    }

    let new_id = refine::save_recipe(&pool, id, cost, prob, &materials).await?;

    if id.is_none() {
        if let Ok(conn) = state.settings_db.lock() {
            let _ = crate::refine_undo::record(&conn, "create", new_id, None);
        }
    }
    Ok(new_id)
}

#[tauri::command]
pub async fn delete_refine_recipe(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    if let Ok(Some(prior)) = refine::get_recipe(&pool, id).await {
        if let Ok(conn) = state.settings_db.lock() {
            let _ = crate::refine_undo::record(&conn, "delete", id, Some(&prior));
        }
    }
    refine::delete_recipe(&pool, id).await
}

/// Restores whatever `save_refine_recipe`/`delete_refine_recipe` most
/// recently overwrote - see `refine_undo.rs` for why this is DB-snapshot
/// based rather than a file-restore like the Box-/Cube-Editor's undo.
#[tauri::command]
pub async fn undo_last_refine_change(state: State<'_, AppState>) -> Result<String, String> {
    let snapshot = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        crate::refine_undo::take(&conn)?
    };
    let Some(snapshot) = snapshot else {
        return Err("Keine rückgängig zu machende Änderung vorhanden.".to_string());
    };
    let pool = require_pool(&state).await?;

    match snapshot.action.as_str() {
        "update" => {
            let prior = snapshot
                .prior
                .ok_or_else(|| "Ungültiger Undo-Zustand (fehlender vorheriger Stand).".to_string())?;
            refine::save_recipe(&pool, Some(snapshot.recipe_id), prior.cost, prior.prob, &prior.materials).await?;
            Ok(format!("Rezept #{} auf vorherigen Stand zurückgesetzt.", snapshot.recipe_id))
        }
        "delete" => {
            let prior = snapshot
                .prior
                .ok_or_else(|| "Ungültiger Undo-Zustand (fehlender vorheriger Stand).".to_string())?;
            // Nothing can still reference the old id (delete_recipe refuses
            // to delete an in-use recipe), so re-creating under a fresh id
            // is equivalent - there's no dangling reference to repoint.
            let new_id = refine::save_recipe(&pool, None, prior.cost, prior.prob, &prior.materials).await?;
            Ok(format!("Gelöschtes Rezept wiederhergestellt (neue ID: {new_id})."))
        }
        "create" => {
            refine::delete_recipe(&pool, snapshot.recipe_id).await?;
            Ok(format!("Neu angelegtes Rezept #{} wieder entfernt.", snapshot.recipe_id))
        }
        other => Err(format!("Unbekannte Undo-Aktion: {other}")),
    }
}

#[tauri::command]
pub async fn set_item_refine_link(
    state: State<'_, AppState>,
    vnum: u32,
    refine_set: u16,
    refined_vnum: u32,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    refine::set_item_refine_link(&pool, vnum, refine_set, refined_vnum).await
}

#[tauri::command]
pub async fn find_refine_shop_sources(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Vec<refine::ShopSource>, String> {
    let pool = require_pool(&state).await?;
    refine::find_shop_sources(&pool, vnum).await
}
