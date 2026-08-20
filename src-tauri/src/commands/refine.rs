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
    refine::save_recipe(&pool, id, cost, prob, &materials).await
}

#[tauri::command]
pub async fn delete_refine_recipe(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    refine::delete_recipe(&pool, id).await
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
