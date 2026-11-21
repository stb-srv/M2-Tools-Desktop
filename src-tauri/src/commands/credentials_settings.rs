//! Credential-Speicherung (Windows Credential Manager), generische
//! Key/Value-Einstellungen, und MySQL-Verbindungsaufbau.

use crate::credentials;
use crate::db::mysql::{self, MysqlConfig};
use crate::settings;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn store_credential(account: String, secret: String) -> Result<(), String> {
    credentials::store_secret(&account, &secret)
}

/// Verrät dem Webview nur, OB ein (nicht leeres) Geheimnis gespeichert ist,
/// nie den Klartext. Das Backend holt Passwörter bei Bedarf selbst.
#[tauri::command]
pub fn has_credential(account: String) -> Result<bool, String> {
    Ok(credentials::get_secret(&account).map(|s| !s.is_empty()).unwrap_or(false))
}

/// Leeres Passwort vom Webview = „gespeichertes Passwort verwenden“.
fn resolve_password(password: String, account: &str) -> String {
    if password.is_empty() {
        credentials::get_secret(account).unwrap_or_default()
    } else {
        password
    }
}

#[tauri::command]
pub fn delete_credential(account: String) -> Result<(), String> {
    credentials::delete_secret(&account)
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
    settings::set_path(&conn, &key, &value)?;
    crate::connection_profiles::mirror_setting(&conn, &key, &value)
}

#[tauri::command]
pub async fn test_mysql_connection(config: MysqlConfig, password: String) -> Result<(), String> {
    mysql::test_connection(&config, &resolve_password(password, "mysql_password")).await
}

#[tauri::command]
pub async fn connect_mysql(
    state: State<'_, AppState>,
    config: MysqlConfig,
    password: String,
) -> Result<(), String> {
    let pool = mysql::connect(&config, &resolve_password(password, "mysql_password")).await?;
    *state.mysql_pool.lock().await = Some(pool);
    Ok(())
}

#[tauri::command]
pub async fn is_mysql_connected(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.mysql_pool.lock().await.is_some())
}
