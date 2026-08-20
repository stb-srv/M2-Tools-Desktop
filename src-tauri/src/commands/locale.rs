//! Locale-String-Verwaltung.
//!
//! Format verified against the real server file (share/translate.lua, ~8860
//! lines) - see locale.rs. Deliberately a "namespace at a time" API: reading/
//! writing the whole file every time would be wasteful and, more
//! importantly, `write_namespace` only touches its own namespace's lines,
//! so the frontend never needs to hold (or risk corrupting) the other ~8800
//! lines it doesn't understand.

use super::support::stored_ssh_auth;
use crate::locale;
use crate::quest;
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use tauri::State;

fn locale_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "locale_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/translate.lua".to_string()))
}

#[tauri::command]
pub async fn list_locale_namespaces(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = locale_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    Ok(locale::list_namespaces(&content))
}

#[tauri::command]
pub async fn read_locale_namespace(
    state: State<'_, AppState>,
    namespace: String,
) -> Result<Vec<locale::LocaleEntry>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = locale_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    Ok(locale::read_namespace(&content, &namespace))
}

#[tauri::command]
pub async fn write_locale_namespace(
    state: State<'_, AppState>,
    namespace: String,
    entries: Vec<locale::LocaleEntry>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = locale_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    let updated = locale::write_namespace(&content, &namespace, &entries)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &updated).await
}

#[tauri::command]
pub async fn create_locale_namespace(
    state: State<'_, AppState>,
    namespace: String,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = locale_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    let updated = locale::create_namespace(&content, &namespace)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &updated).await
}

#[tauri::command]
pub fn sanitize_locale_namespace(name: String) -> String {
    quest::sanitize_identifier(&name)
}
