//! Kisten-Editor (`special_item_group.txt`) und Cube-Editor (`cube.txt`) -
//! zwei kleine, thematisch benachbarte Datei-Editoren (siehe cube.rs' Modul-
//! Doku zum gemeinsamen `LocaleService_GetBasePath()`).

use super::support::stored_ssh_auth;
use crate::cube;
use crate::settings;
use crate::special_item_group;
use crate::ssh;
use crate::state::AppState;
use tauri::State;

fn special_item_group_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "special_item_group_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/special_item_group.txt".to_string()))
}

#[tauri::command]
pub async fn read_special_item_group_file(
    state: State<'_, AppState>,
) -> Result<Vec<special_item_group::SpecialItemGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = special_item_group_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    special_item_group::parse(&content)
}

#[tauri::command]
pub async fn write_special_item_group_file(
    state: State<'_, AppState>,
    groups: Vec<special_item_group::SpecialItemGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = special_item_group_file_path(&state)?;
    let content = special_item_group::serialize(&groups);
    // Round-trip-check what we're about to write before touching the
    // server - refuse to upload something we couldn't parse back ourselves.
    special_item_group::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

#[tauri::command]
pub fn sanitize_special_item_group_name(name: String) -> String {
    special_item_group::sanitize_group_name(&name)
}

/// "Letzte Änderung rückgängig machen" - `write_special_item_group_file`
/// already backs up the previous file version before every overwrite (see
/// `ssh::write_remote_file_with_backup`), so undo just means finding and
/// restoring the newest of those backups. Reuses the existing restore
/// command (itself backs up the *current* state first, so this can never
/// destroy data without its own recovery trail).
#[tauri::command]
pub async fn undo_special_item_group_write(state: State<'_, AppState>) -> Result<String, String> {
    let path = special_item_group_file_path(&state)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let backup_path = ssh::latest_own_backup(&config, &auth, &path)
        .await?
        .ok_or_else(|| "Keine frühere Sicherung von special_item_group.txt gefunden.".to_string())?;
    super::backups::restore_remote_backup(state, backup_path.clone()).await?;
    Ok(backup_path)
}

// ---- Cube-Editor (cube.txt - Verwandlung/Kombinations-Rezepte) ----
//
// Shares `LocaleService_GetBasePath()` with special_item_group.txt (both
// verified in source/game/src/{cube.cpp,item_manager_read_tables.cpp}), so
// this defaults to the sibling path of the already-known-real
// special_item_group.txt default rather than a guessed location.

fn cube_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "cube_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/cube.txt".to_string()))
}

#[tauri::command]
pub async fn read_cube_file(state: State<'_, AppState>) -> Result<Vec<cube::CubeRecipe>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = cube_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    cube::parse(&content)
}

#[tauri::command]
pub async fn write_cube_file(
    state: State<'_, AppState>,
    recipes: Vec<cube::CubeRecipe>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = cube_file_path(&state)?;
    let content = cube::serialize(&recipes);
    // Same round-trip sanity check as every other server-file editor here -
    // refuse to upload something we couldn't parse back ourselves.
    cube::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

/// Same "restore the newest own backup" undo as
/// `undo_special_item_group_write`, for `cube.txt`.
#[tauri::command]
pub async fn undo_cube_write(state: State<'_, AppState>) -> Result<String, String> {
    let path = cube_file_path(&state)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let backup_path = ssh::latest_own_backup(&config, &auth, &path)
        .await?
        .ok_or_else(|| "Keine frühere Sicherung von cube.txt gefunden.".to_string())?;
    super::backups::restore_remote_backup(state, backup_path.clone()).await?;
    Ok(backup_path)
}
