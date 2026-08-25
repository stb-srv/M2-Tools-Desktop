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

// Same default/setting as commands/quest.rs::quest_dir - duplicated locally
// rather than made `pub` there, matching this module's existing style of
// small private path helpers (see `locale_file_path` above).
fn quest_dir(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "quest_dir")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/quest".to_string()))
}

/// Cross-checks every `gameforge.<ns>.<key>` reference across all quest
/// source files against what's actually defined in translate.lua - see
/// `locale::check_completeness` for the pure diff logic.
#[tauri::command]
pub async fn check_locale_completeness(
    state: State<'_, AppState>,
) -> Result<locale::LocaleCompletenessReport, String> {
    let (config, auth) = stored_ssh_auth(&state)?;

    let locale_path = locale_file_path(&state)?;
    let locale_content = ssh::read_remote_file(&config, &auth, &locale_path).await?;
    let defined: Vec<(String, String)> = locale::list_namespaces(&locale_content)
        .into_iter()
        .flat_map(|ns| {
            let entries = locale::read_namespace(&locale_content, &ns);
            entries.into_iter().map(move |e| (ns.clone(), e.key))
        })
        .collect();

    let quest_dir = quest_dir(&state)?;
    let list_content = ssh::read_remote_file(&config, &auth, &format!("{quest_dir}/quest_list")).await?;
    let files = quest::parse_quest_list(&list_content);
    let paths: Vec<String> = files
        .iter()
        .map(|f| format!("{quest_dir}/{}", f.relative_path))
        .collect();
    let contents = ssh::read_remote_files(&config, &auth, &paths).await?;
    let quest_files: Vec<(String, String)> = files
        .iter()
        .zip(contents.iter())
        .filter_map(|(f, c)| c.as_ref().ok().map(|content| (f.relative_path.clone(), content.clone())))
        .collect();

    Ok(locale::check_completeness(&defined, &quest_files))
}
