use crate::credentials;
use crate::db::mysql::{self, MysqlConfig};
use crate::db::shop::{self, ItemSearchResult, ShopItem, ShopSummary};
use crate::gr2::{self, ModelInfo};
use crate::settings;
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

#[tauri::command]
pub async fn is_mysql_connected(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.mysql_pool.lock().await.is_some())
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
pub fn get_setting(state: State<'_, AppState>, key: String) -> Result<Option<String>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, &key)
}

#[tauri::command]
pub fn set_setting(
    state: State<'_, AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::set_path(&conn, &key, &value)
}

#[tauri::command]
pub fn check_client_path(path: String) -> bool {
    gr2::find_granny_dll(&path).is_some()
}

#[tauri::command]
pub fn locate_npc_model(
    state: State<'_, AppState>,
    folder: String,
) -> Result<(String, String), String> {
    let client_path = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        settings::get_path(&conn, "client_path")?
    }
    .ok_or_else(|| "Kein Client-Pfad konfiguriert. Bitte in den Einstellungen setzen.".to_string())?;

    let dll = gr2::find_granny_dll(&client_path)
        .ok_or_else(|| format!("granny2.dll nicht gefunden unter {client_path}"))?;
    let model = gr2::find_npc_model(&client_path, &folder).ok_or_else(|| {
        format!("Kein .gr2-Modell für '{folder}' im Client-Ordner gefunden")
    })?;
    Ok((dll, model))
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

