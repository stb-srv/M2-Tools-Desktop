//! Quest Builder.
//!
//! Source layout verified directly on the user's dev server over SFTP (see
//! [[m2manager_quest_builder]] memory): quests are .lua files under
//! share/quest/<Category>/<Name>.lua, listed in share/quest/quest_list -
//! that list is what the server's own `make.py`/`qc_x64` compile step reads
//! (already exposed as the "Quests reloaden" Server-Control action), so this
//! only has to manage the source files and keep quest_list in sync.

use super::support::{safe_relative_path, stored_ssh_auth};
use crate::quest;
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use tauri::State;

fn quest_dir(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "quest_dir")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/quest".to_string()))
}

#[tauri::command]
pub async fn list_quest_files(state: State<'_, AppState>) -> Result<Vec<quest::QuestFile>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &format!("{dir}/quest_list")).await?;
    Ok(quest::parse_quest_list(&content))
}

#[tauri::command]
pub async fn read_quest_file(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<String, String> {
    let relative_path = safe_relative_path(&relative_path)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;
    ssh::read_remote_file(&config, &auth, &format!("{dir}/{relative_path}")).await
}

#[tauri::command]
pub async fn write_quest_file(
    state: State<'_, AppState>,
    relative_path: String,
    content: String,
) -> Result<Option<String>, String> {
    let relative_path = safe_relative_path(&relative_path)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;
    ssh::write_remote_file_with_backup(&config, &auth, &format!("{dir}/{relative_path}"), &content)
        .await
}

/// Creates a new quest source file and registers it in `quest_list` in the
/// same operation - a file that exists on disk but isn't listed there would
/// silently never get compiled by `make.py`, so the two must stay in sync.
#[tauri::command]
pub async fn create_quest_file(
    state: State<'_, AppState>,
    category: String,
    name: String,
    content: String,
    extension: Option<String>,
) -> Result<String, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;

    let category = quest::sanitize_identifier(&category);
    let name = quest::sanitize_identifier(&name);
    if category.is_empty() || name.is_empty() {
        return Err("Kategorie und Name dürfen nicht leer sein.".to_string());
    }
    // Regular quests compile as .lua; dungeon "Runs" use .quest on this
    // server (verified in share/quest/Runs/*.quest) - qc_x64 doesn't care
    // about the extension itself, only quest_list has to list the right one.
    let extension = extension.filter(|e| !e.is_empty()).unwrap_or_else(|| "lua".to_string());
    let relative_path = format!("{category}/{name}.{extension}");

    let list_path = format!("{dir}/quest_list");
    let list_content = ssh::read_remote_file(&config, &auth, &list_path).await?;
    if list_content.lines().any(|l| l.trim() == relative_path) {
        return Err(format!("'{relative_path}' existiert bereits in quest_list."));
    }

    ssh::write_remote_file_with_backup(&config, &auth, &format!("{dir}/{relative_path}"), &content)
        .await?;

    let updated_list = quest::quest_list_add(&list_content, &relative_path);
    ssh::write_remote_file_with_backup(&config, &auth, &list_path, &updated_list).await?;

    Ok(relative_path)
}

/// Removes a quest from `quest_list` and moves its source file into the SFTP
/// backup folder (never a hard delete - see `delete_remote_file_with_backup`).
#[tauri::command]
pub async fn delete_quest_file(
    state: State<'_, AppState>,
    relative_path: String,
) -> Result<(), String> {
    let relative_path = safe_relative_path(&relative_path)?;
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;

    let list_path = format!("{dir}/quest_list");
    let list_content = ssh::read_remote_file(&config, &auth, &list_path).await?;
    let updated_list = quest::quest_list_remove(&list_content, relative_path);
    ssh::write_remote_file_with_backup(&config, &auth, &list_path, &updated_list).await?;

    ssh::delete_remote_file_with_backup(&config, &auth, &format!("{dir}/{relative_path}")).await?;
    Ok(())
}

#[tauri::command]
pub fn sanitize_quest_identifier(name: String) -> String {
    quest::sanitize_identifier(&name)
}

/// Full-text search across every quest source file's *content*, not just
/// file/category names (`list_quest_files` only covers the latter). Reads
/// all files in one SFTP session via `read_remote_files` - opening a fresh
/// SSH connection per file (like the other quest commands do) would mean one
/// handshake per quest file, far too slow across a whole catalogue.
#[tauri::command]
pub async fn search_quest_files(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<quest::QuestSearchMatch>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;
    let list_content = ssh::read_remote_file(&config, &auth, &format!("{dir}/quest_list")).await?;
    let files = quest::parse_quest_list(&list_content);
    let paths: Vec<String> = files
        .iter()
        .map(|f| format!("{dir}/{}", f.relative_path))
        .collect();
    let contents = ssh::read_remote_files(&config, &auth, &paths).await?;
    Ok(quest::search_contents(&files, &contents, &query))
}
