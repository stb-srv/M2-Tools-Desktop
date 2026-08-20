//! Regen-Datei-Editor + Regen-Karteneditor (Karten-Ansicht).
//!
//! Verified against the real server (game-src/source/game/src/regen.cpp) and
//! a real file (share/data/dungeon/dt_short/deviltower3_regen.txt) - see
//! regen.rs. Files referenced by the Quest Builder's dungeon template live
//! under `regen_base_dir` (default matches the server's `share/` folder,
//! since floor regen paths are written like "data/dungeon/.../x_regen.txt").
//!
//! The map view renders the Metin2 client's own minimap tiles as a
//! background for the spawn markers - see mapdata.rs for the DDS decoding/
//! compositing and the client folder format.

use super::support::{client_path_setting, safe_relative_path, stored_ssh_auth};
use crate::mapdata;
use crate::regen;
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use tauri::{AppHandle, Manager, State};

fn regen_base_dir(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "regen_base_dir")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share".to_string()))
}

#[tauri::command]
pub async fn read_regen_file(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<Vec<regen::RegenLine>, String> {
    let relative_path = safe_relative_path(&relative_path)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let base = regen_base_dir(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &format!("{base}/{relative_path}")).await?;
    regen::parse(&content)
}

#[tauri::command]
pub async fn write_regen_file(
    state: State<'_, AppState>,
    relative_path: String,
    lines: Vec<regen::RegenLine>,
) -> Result<Option<String>, String> {
    let relative_path = safe_relative_path(&relative_path)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let base = regen_base_dir(&state)?;
    let content = regen::serialize(&lines);
    // Round-trip-check before writing, same principle as the Mob Drop Editor.
    regen::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &format!("{base}/{relative_path}"), &content)
        .await
}

// ---- Regen-Karteneditor (Karten-Ansicht) ----
//
// Reuses the existing, already-validated `client_path` setting (see
// check_client_path) rather than adding a separate one.

#[tauri::command]
pub fn list_client_map_folders(
    state: State<'_, AppState>,
) -> Result<Vec<mapdata::MapFolderInfo>, String> {
    let client_path = client_path_setting(&state)?;
    mapdata::list_map_folders(&client_path)
}

#[tauri::command]
pub fn get_regen_map_image(
    app: AppHandle,
    state: State<'_, AppState>,
    category: String,
    folder_name: String,
    force_rebuild: bool,
) -> Result<mapdata::RegenMapImage, String> {
    let client_path = client_path_setting(&state)?;
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("App-Datenordner nicht auffindbar: {e}"))?;
    mapdata::render_map(
        &app_data_dir,
        &client_path,
        &category,
        &folder_name,
        force_rebuild,
    )
}
