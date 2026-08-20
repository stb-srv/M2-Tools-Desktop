//! Shop-Editor + Item-Proto-Explorer's Browse-Endpoint + die geteilten
//! Item-/Mob-Browse-Commands, die der `EntityBrowser` (Item Editor,
//! Refine-Editor, Mob-Proto-Editor, Quest Builder, ...) generisch nutzt -
//! alle über `db/shop.rs`, daher hier statt bei ihren jeweiligen
//! Feature-Modulen.

use crate::db::item_explorer::{self, ItemProtoPage};
use crate::db::shop::{self, DatabaseStats, EntityBrowsePage, ItemSearchResult, ShopItem, ShopSummary};
use crate::settings;
use crate::state::AppState;
use tauri::State;

use super::support::require_pool;

#[tauri::command]
pub async fn get_database_stats(state: State<'_, AppState>) -> Result<DatabaseStats, String> {
    let pool = require_pool(&state).await?;
    shop::get_stats(&pool).await
}

#[tauri::command]
pub async fn list_shops(state: State<'_, AppState>) -> Result<Vec<ShopSummary>, String> {
    let pool = require_pool(&state).await?;
    shop::list_shops(&pool).await
}

#[tauri::command]
pub async fn get_shop_items(
    state: State<'_, AppState>,
    shop_vnum: i32,
) -> Result<Vec<ShopItem>, String> {
    let pool = require_pool(&state).await?;
    shop::get_shop_items(&pool, shop_vnum).await
}

#[tauri::command]
pub async fn search_items(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ItemSearchResult>, String> {
    let pool = require_pool(&state).await?;
    shop::search_items(&pool, &query, 50).await
}

#[tauri::command]
pub async fn search_mobs(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<ItemSearchResult>, String> {
    let pool = require_pool(&state).await?;
    shop::search_mobs(&pool, &query, 50).await
}

/// Paginated item browse/search used by pickers - see `shop::browse_items`
/// for why this exists alongside `search_items` (that one returns nothing
/// when a numeric query doesn't match an exact vnum, with no way to browse
/// what's actually there instead).
#[tauri::command]
pub async fn browse_items(
    state: State<'_, AppState>,
    query: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<EntityBrowsePage, String> {
    let pool = require_pool(&state).await?;
    shop::browse_items(&pool, query.as_deref(), offset, limit).await
}

#[tauri::command]
pub async fn browse_mobs(
    state: State<'_, AppState>,
    query: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<EntityBrowsePage, String> {
    let pool = require_pool(&state).await?;
    shop::browse_mobs(&pool, query.as_deref(), offset, limit).await
}

#[tauri::command]
pub async fn browse_item_proto(
    state: State<'_, AppState>,
    query: Option<String>,
    type_filter: Option<i8>,
    offset: i64,
    limit: i64,
) -> Result<ItemProtoPage, String> {
    let pool = require_pool(&state).await?;
    item_explorer::browse_item_proto(&pool, query.as_deref(), type_filter, offset, limit).await
}

#[tauri::command]
pub async fn update_shop_item_count(
    state: State<'_, AppState>,
    shop_vnum: i32,
    item_vnum: i32,
    count: i32,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    shop::update_shop_item_count(&pool, shop_vnum, item_vnum, count).await
}

#[tauri::command]
pub async fn add_shop_item(
    state: State<'_, AppState>,
    shop_vnum: i32,
    item_vnum: i32,
    count: i32,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    shop::add_shop_item(&pool, shop_vnum, item_vnum, count).await
}

#[tauri::command]
pub async fn remove_shop_item(
    state: State<'_, AppState>,
    shop_vnum: i32,
    item_vnum: i32,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    shop::remove_shop_item(&pool, shop_vnum, item_vnum).await
}

#[tauri::command]
pub async fn sync_shop_stack_sizes(
    state: State<'_, AppState>,
    shop_vnum: Option<i32>,
    count: i32,
) -> Result<u64, String> {
    let pool = require_pool(&state).await?;
    shop::sync_stack_sizes(&pool, shop_vnum, count).await
}

#[tauri::command]
pub async fn delete_shop(state: State<'_, AppState>, shop_vnum: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    shop::delete_shop(&pool, shop_vnum).await
}

#[tauri::command]
pub async fn rename_shop(
    state: State<'_, AppState>,
    shop_vnum: i32,
    name: String,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    shop::rename_shop(&pool, shop_vnum, &name).await
}

#[tauri::command]
pub async fn create_shop(
    state: State<'_, AppState>,
    name: String,
    npc_vnum: i16,
) -> Result<i32, String> {
    let pool = require_pool(&state).await?;
    shop::create_shop(&pool, &name, npc_vnum).await
}

#[tauri::command]
pub fn get_shop_default_max(
    state: State<'_, AppState>,
    shop_vnum: Option<i32>,
) -> Result<i32, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    if let Some(vnum) = shop_vnum {
        if let Some(value) = settings::get_path(&conn, &format!("shop_editor_max_shop_{vnum}"))? {
            return Ok(value.parse().unwrap_or(200));
        }
    }
    let global = settings::get_path(&conn, "shop_editor_global_max")?;
    Ok(global.and_then(|v| v.parse().ok()).unwrap_or(200))
}

#[tauri::command]
pub fn set_shop_default_max(
    state: State<'_, AppState>,
    shop_vnum: Option<i32>,
    value: i32,
) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    let key = match shop_vnum {
        Some(vnum) => format!("shop_editor_max_shop_{vnum}"),
        None => "shop_editor_global_max".to_string(),
    };
    settings::set_path(&conn, &key, &value.to_string())
}
