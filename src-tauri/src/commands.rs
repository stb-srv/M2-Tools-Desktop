use crate::credentials;
use crate::db::mysql::{self, MysqlConfig};
use crate::db::shop::{self, ItemSearchResult, ShopItem, ShopSummary};
use crate::gr2::{self, ModelInfo};
use crate::ssh::{self, SshConfig};
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub async fn test_ssh_connection(config: SshConfig, password: String) -> Result<(), String> {
    ssh::test_connection(&config, &password).await
}

#[tauri::command]
pub async fn test_mysql_connection(config: MysqlConfig, password: String) -> Result<(), String> {
    mysql::test_connection(&config, &password).await
}

#[tauri::command]
pub fn store_credential(account: String, secret: String) -> Result<(), String> {
    credentials::store_secret(&account, &secret)
}

#[tauri::command]
pub fn get_credential(account: String) -> Result<String, String> {
    credentials::get_secret(&account)
}

#[tauri::command]
pub fn delete_credential(account: String) -> Result<(), String> {
    credentials::delete_secret(&account)
}

#[tauri::command]
pub fn load_gr2_model(granny_dll_path: String, gr2_path: String) -> Result<ModelInfo, String> {
    gr2::parse(&granny_dll_path, &gr2_path)
}

#[tauri::command]
pub async fn connect_mysql(
    state: State<'_, AppState>,
    config: MysqlConfig,
    password: String,
) -> Result<(), String> {
    let pool = mysql::connect(&config, &password).await?;
    *state.mysql_pool.lock().await = Some(pool);
    Ok(())
}

async fn require_pool(state: &State<'_, AppState>) -> Result<sqlx::MySqlPool, String> {
    state
        .mysql_pool
        .lock()
        .await
        .clone()
        .ok_or_else(|| "Keine aktive MySQL-Verbindung. Bitte zuerst verbinden.".to_string())
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
pub async fn delete_shop(state: State<'_, AppState>, shop_vnum: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    shop::delete_shop(&pool, shop_vnum).await
}
