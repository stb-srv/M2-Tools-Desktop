use crate::credentials;
use crate::db::mysql::{self, MysqlConfig};
use crate::db::explorer::{self, ColumnInfo, TableInfo, TableRows};
use crate::db::item::{self, ItemProtoFull, ItemProtoInput};
use crate::db::shop::{self, DatabaseStats, ItemSearchResult, ShopItem, ShopSummary};
use crate::gr2::{self, ModelInfo};
use crate::icons;
use crate::import_history;
use crate::mobdrop;
use crate::modulescan::{self, ScannedModule};
use crate::msm;
use crate::packtools;
use crate::backups;
use crate::locale;
use crate::mapdata;
use crate::db_backup;
use crate::quest;
use crate::regen;
use crate::resources;
use crate::webhook;
use crate::settings;
use crate::ssh::{self, SshAuth, SshConfig};
use crate::state::AppState;
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn test_ssh_connection(config: SshConfig, auth: SshAuth) -> Result<(), String> {
    ssh::test_connection(&config, &auth).await
}

#[tauri::command]
pub async fn test_mysql_connection(config: MysqlConfig, password: String) -> Result<(), String> {
    mysql::test_connection(&config, &password).await
}

/// Reassembles the SSH connection from what the setup wizard stored: metadata
/// in the local settings DB, secrets in the Windows Credential Manager.
fn stored_ssh_auth(state: &State<'_, AppState>) -> Result<(SshConfig, SshAuth), String> {
    let (host, port, username, auth_mode, key_path) = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        (
            settings::get_path(&conn, "ssh_host")?,
            settings::get_path(&conn, "ssh_port")?,
            settings::get_path(&conn, "ssh_username")?,
            settings::get_path(&conn, "ssh_auth_mode")?,
            settings::get_path(&conn, "ssh_key_path")?,
        )
    };

    let host = host.filter(|h| !h.is_empty()).ok_or_else(|| {
        "Keine SSH-Verbindung konfiguriert. Bitte unter Verbindungen einrichten.".to_string()
    })?;
    let username = username
        .filter(|u| !u.is_empty())
        .ok_or_else(|| "Kein SSH-Benutzername konfiguriert.".to_string())?;

    let config = SshConfig {
        host,
        port: port.and_then(|p| p.parse().ok()).unwrap_or(22),
        username,
    };

    let auth = if auth_mode.as_deref() == Some("key") {
        SshAuth::PrivateKey {
            path: key_path
                .filter(|p| !p.is_empty())
                .ok_or_else(|| "Kein SSH-Schlüsselpfad konfiguriert.".to_string())?,
            passphrase: credentials::get_secret("ssh_key_passphrase").ok(),
        }
    } else {
        SshAuth::Password {
            password: credentials::get_secret("ssh_password")
                .map_err(|_| "Kein gespeichertes SSH-Passwort gefunden.".to_string())?,
        }
    };

    Ok((config, auth))
}

#[derive(serde::Serialize, Clone)]
pub struct ServerCommandResult {
    pub output: String,
    pub exit_status: Option<u32>,
}

#[tauri::command]
pub async fn run_server_command(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    command: String,
) -> Result<ServerCommandResult, String> {
    let (config, auth) = stored_ssh_auth(&state)?;

    let result = ssh::run_command_streaming(&config, &auth, &command, |chunk| {
        let _ = app.emit("server-output", chunk);
    })
    .await?;

    Ok(ServerCommandResult {
        output: result.output,
        exit_status: result.exit_status,
    })
}

#[tauri::command]
pub async fn test_stored_ssh(state: State<'_, AppState>) -> Result<(), String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    ssh::test_connection(&config, &auth).await
}

// ---- Server-Ressourcen-Monitoring ----

fn server_process_names(state: &State<'_, AppState>) -> Result<Vec<String>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    let raw = settings::get_path(&conn, "server_process_names")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "game,db".to_string());
    Ok(raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect())
}

/// Reads CPU/RAM usage of the game server's processes via `ps` over the
/// existing SSH connection - see resources.rs for why this doesn't assume a
/// Linux /proc filesystem, and why the process names are a setting rather
/// than hardcoded.
#[tauri::command]
pub async fn get_server_resource_usage(
    state: State<'_, AppState>,
) -> Result<Vec<resources::ProcessUsage>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let names = server_process_names(&state)?;
    let result =
        ssh::run_command_streaming(&config, &auth, "ps -axo pid,pcpu,pmem,rss,comm", |_| {})
            .await?;
    Ok(resources::parse_and_filter(&result.output, &names))
}

#[derive(serde::Serialize)]
pub struct ServerOverview {
    pub ip_address: Option<String>,
    pub memory: Option<resources::MemoryInfo>,
    pub disk: Option<resources::DiskInfo>,
}

/// IP/host is just the already-configured SSH host - no extra round trip
/// needed, and no guessing at "the" server IP via e.g. `ifconfig` output
/// (which interface/address would even be "the" IP is host-specific and not
/// something this app can know better than what the user already typed in
/// under Einstellungen).
#[tauri::command]
pub async fn get_server_overview(state: State<'_, AppState>) -> Result<ServerOverview, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let disk_path = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        settings::get_path(&conn, "server_disk_path")?
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| "/usr/home/game".to_string())
    };
    let ip_address = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        settings::get_path(&conn, "ssh_host")?
    };

    let command = resources::build_system_info_command(&disk_path);
    let result = ssh::run_command_streaming(&config, &auth, &command, |_| {}).await?;
    let system_info = resources::parse_system_info_output(&result.output);

    Ok(ServerOverview {
        ip_address,
        memory: system_info.memory,
        disk: system_info.disk,
    })
}

// ---- Webhook-Benachrichtigungen ----

fn webhook_url(state: &State<'_, AppState>) -> Result<Option<String>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "webhook_url")?.filter(|v| !v.is_empty()))
}

/// Generic "send this message" command - callers (Server Control failures,
/// DB-backup failures, the frontend crash-watch) all go through this rather
/// than each having their own webhook wiring.
#[tauri::command]
pub async fn notify_webhook_message(
    state: State<'_, AppState>,
    message: String,
) -> Result<(), String> {
    let url = webhook_url(&state)?.ok_or_else(|| "Keine Webhook-URL konfiguriert.".to_string())?;
    webhook::send_webhook_message(&url, &message).await
}

#[tauri::command]
pub async fn send_test_webhook(state: State<'_, AppState>) -> Result<(), String> {
    let url = webhook_url(&state)?.ok_or_else(|| "Keine Webhook-URL konfiguriert.".to_string())?;
    webhook::send_webhook_message(&url, "M2Manager: Testnachricht ✅").await
}

// ---- Automatisierte Datenbank-Backups ----
//
// Runs mysqldump/mysql on the remote server over the existing SSH connection
// (see db_backup.rs for why the same MySQL credentials configured for this
// app's own DB connection are reused). Listing/deleting backup files reuses
// the already-generic `list_remote_dir`; only creating and restoring need
// their own commands.

fn db_backup_settings(state: &State<'_, AppState>) -> Result<(String, Vec<String>), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    let dir = settings::get_path(&conn, "db_backup_dir")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/m2manager_db_backups".to_string());
    let databases = settings::get_path(&conn, "db_backup_databases")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "account common player log website".to_string())
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();
    Ok((dir, databases))
}

fn mysql_connection_settings(state: &State<'_, AppState>) -> Result<(String, u16, String), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    let host = settings::get_path(&conn, "mysql_host")?
        .ok_or_else(|| "Keine MySQL-Verbindung konfiguriert.".to_string())?;
    let port: u16 = settings::get_path(&conn, "mysql_port")?
        .unwrap_or_else(|| "3306".to_string())
        .parse()
        .map_err(|_| "Ungültiger MySQL-Port.".to_string())?;
    let username = settings::get_path(&conn, "mysql_username")?
        .ok_or_else(|| "Keine MySQL-Verbindung konfiguriert.".to_string())?;
    Ok((host, port, username))
}

/// Creates a full mysqldump backup on the remote server. On failure, also
/// fires a webhook notification (if configured) - this is exactly the
/// "fehlgeschlagenes Backup" case the notification feature was requested
/// for, so it's wired in directly rather than left to the frontend to guess.
#[tauri::command]
pub async fn create_database_backup(state: State<'_, AppState>) -> Result<String, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let (dir, databases) = db_backup_settings(&state)?;
    let (host, port, username) = mysql_connection_settings(&state)?;
    let password = credentials::get_secret("mysql_password")
        .map_err(|_| "Kein MySQL-Passwort im Credential-Manager gefunden.".to_string())?;

    let filename = db_backup::backup_filename(chrono::Local::now());
    let command = db_backup::build_dump_command(
        &host, port, &username, &password, &databases, &dir, &filename,
    );

    let result = ssh::run_command_streaming(&config, &auth, &command, |_| {}).await;
    match result {
        Ok(r) if r.exit_status == Some(0) => Ok(format!("{dir}/{filename}")),
        Ok(r) => {
            let message = format!(
                "M2Manager: Datenbank-Backup fehlgeschlagen (Exit-Code {:?}):\n{}",
                r.exit_status, r.output
            );
            notify_webhook_best_effort(&state, &message).await;
            Err(message)
        }
        Err(e) => {
            let message = format!("M2Manager: Datenbank-Backup fehlgeschlagen: {e}");
            notify_webhook_best_effort(&state, &message).await;
            Err(e)
        }
    }
}

async fn notify_webhook_best_effort(state: &State<'_, AppState>, message: &str) {
    if let Ok(Some(url)) = webhook_url(state) {
        let _ = webhook::send_webhook_message(&url, message).await;
    }
}

/// Restores a database dump already sitting on the remote server (created by
/// `create_database_backup` or dropped in manually) - overwrites the current
/// contents of every database in the dump, hence the confirmation-heavy UI.
#[tauri::command]
pub async fn restore_database_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<(), String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let (host, port, username) = mysql_connection_settings(&state)?;
    let password = credentials::get_secret("mysql_password")
        .map_err(|_| "Kein MySQL-Passwort im Credential-Manager gefunden.".to_string())?;

    let command = db_backup::build_restore_command(&host, port, &username, &password, &backup_path);
    let result = ssh::run_command_streaming(&config, &auth, &command, |_| {}).await?;
    if result.exit_status != Some(0) {
        return Err(format!(
            "Wiederherstellung fehlgeschlagen (Exit-Code {:?}):\n{}",
            result.exit_status, result.output
        ));
    }
    Ok(())
}

/// Permanent delete (no backup-of-backup) - see `ssh::delete_remote_file`.
#[tauri::command]
pub async fn delete_database_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<(), String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    ssh::delete_remote_file(&config, &auth, &backup_path).await
}

fn mob_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "mob_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/mob_drop_item.txt".to_string()))
}

#[tauri::command]
pub async fn read_mob_drop_file(
    state: State<'_, AppState>,
) -> Result<Vec<mobdrop::MobDropGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = mob_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    mobdrop::parse(&content)
}

#[tauri::command]
pub async fn write_mob_drop_file(
    state: State<'_, AppState>,
    groups: Vec<mobdrop::MobDropGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = mob_drop_file_path(&state)?;
    let content = mobdrop::serialize(&groups);
    // Round-trip-check what we're about to write before touching the
    // server - refuse to upload something we couldn't parse back ourselves.
    mobdrop::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

#[tauri::command]
pub fn sanitize_mob_drop_group_name(name: String) -> String {
    mobdrop::sanitize_group_name(&name)
}

// ---- Mob Drop Editor: local file variant (syntax check/repair tab) ----

#[tauri::command]
pub fn read_local_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| format!("Datei konnte nicht gelesen werden: {e}"))
}

#[tauri::command]
pub fn parse_mob_drop_text(content: String) -> Result<Vec<mobdrop::MobDropGroup>, String> {
    mobdrop::parse(&content)
}

// ---- Quest Builder ----
//
// Source layout verified directly on the user's dev server over SFTP (see
// [[m2manager_quest_builder]] memory): quests are .lua files under
// share/quest/<Category>/<Name>.lua, listed in share/quest/quest_list -
// that list is what the server's own `make.py`/`qc_x64` compile step reads
// (already exposed as the "Quests reloaden" Server-Control action), so this
// only has to manage the source files and keep quest_list in sync.

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
    let (config, auth) = stored_ssh_auth(&state)?;
    let dir = quest_dir(&state)?;

    let list_path = format!("{dir}/quest_list");
    let list_content = ssh::read_remote_file(&config, &auth, &list_path).await?;
    let updated_list = quest::quest_list_remove(&list_content, &relative_path);
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

// ---- Remote directory browsing (shared by Regen-Datei-Editor + Backup-Browser) ----

#[tauri::command]
pub async fn list_remote_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<ssh::RemoteEntry>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    ssh::list_remote_dir(&config, &auth, &path).await
}

/// Restores a `m2manager_backups/<name>.<timestamp>.bak`/`.deleted` file
/// back to its original location (one directory up, original filename) -
/// itself going through the normal backup-before-overwrite path, so a
/// restore can never destroy whatever was there without leaving its own
/// recovery trail.
#[tauri::command]
pub async fn restore_remote_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<String, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let target_path = backups::target_path_for_backup(&backup_path)?;

    let content = ssh::read_remote_file(&config, &auth, &backup_path).await?;
    ssh::write_remote_file_with_backup(&config, &auth, &target_path, &content).await?;
    Ok(target_path)
}

/// Lets the Backup-Browser show what a restore would actually change before
/// the user commits to it, instead of restoring blind.
#[tauri::command]
pub async fn diff_remote_backup(
    state: State<'_, AppState>,
    backup_path: String,
) -> Result<backups::BackupDiff, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let target_path = backups::target_path_for_backup(&backup_path)?;

    let backup_content = ssh::read_remote_file(&config, &auth, &backup_path).await?;
    let current_content =
        ssh::read_remote_file_if_exists(&config, &auth, &target_path).await?;
    Ok(backups::BackupDiff {
        target_path,
        backup_content,
        current_content,
    })
}

// ---- Regen-Datei-Editor ----
//
// Verified against the real server (game-src/source/game/src/regen.cpp) and
// a real file (share/data/dungeon/dt_short/deviltower3_regen.txt) - see
// regen.rs. Files referenced by the Quest Builder's dungeon template live
// under `regen_base_dir` (default matches the server's `share/` folder,
// since floor regen paths are written like "data/dungeon/.../x_regen.txt").

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
// Renders the Metin2 client's own minimap tiles as a background for the
// Regen-Datei-Editor's spawn markers - see mapdata.rs for the DDS decoding/
// compositing and the client folder format (verified against a real client
// install, not guessed). Reuses the existing, already-validated `client_path`
// setting (see check_client_path) rather than adding a separate one.

fn client_path_setting(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, "client_path")?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "Kein Client-Pfad konfiguriert.".to_string())
}

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

// ---- Locale-String-Verwaltung ----
//
// Format verified against the real server file (share/translate.lua, ~8860
// lines) - see locale.rs. Deliberately a "namespace at a time" API: reading/
// writing the whole file every time would be wasteful and, more
// importantly, `write_namespace` only touches its own namespace's lines,
// so the frontend never needs to hold (or risk corrupting) the other ~8800
// lines it doesn't understand.

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

// ---- TGA Converter ----

#[tauri::command]
pub fn convert_image_to_tga(source_path: String, dest_path: String) -> Result<(), String> {
    crate::imageconv::convert_to_tga(&source_path, std::path::Path::new(&dest_path))
}

#[tauri::command]
pub fn preview_image_file(path: String) -> Result<String, String> {
    crate::imageconv::preview_as_data_url(&path)
}

#[tauri::command]
pub fn write_local_mob_drop_file(
    path: String,
    groups: Vec<mobdrop::MobDropGroup>,
) -> Result<Option<String>, String> {
    let content = mobdrop::serialize(&groups);
    // Same sanity check as the server write path - never write something we
    // couldn't parse back ourselves.
    mobdrop::parse(&content)?;
    let backup = packtools::backup_file(std::path::Path::new(&path))?;
    std::fs::write(&path, content).map_err(|e| format!("Datei konnte nicht geschrieben werden: {e}"))?;
    Ok(backup.map(|p| p.display().to_string()))
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
pub async fn get_database_stats(state: State<'_, AppState>) -> Result<DatabaseStats, String> {
    let pool = require_pool(&state).await?;
    shop::get_stats(&pool).await
}

#[tauri::command]
pub async fn list_databases(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let pool = require_pool(&state).await?;
    explorer::list_databases(&pool).await
}

#[tauri::command]
pub async fn list_tables(
    state: State<'_, AppState>,
    database: String,
) -> Result<Vec<TableInfo>, String> {
    let pool = require_pool(&state).await?;
    explorer::list_tables(&pool, &database).await
}

#[tauri::command]
pub async fn get_table_columns(
    state: State<'_, AppState>,
    database: String,
    table: String,
) -> Result<Vec<ColumnInfo>, String> {
    let pool = require_pool(&state).await?;
    explorer::get_columns(&pool, &database, &table).await
}

#[tauri::command]
pub async fn get_table_rows(
    state: State<'_, AppState>,
    database: String,
    table: String,
    limit: i64,
    offset: i64,
) -> Result<TableRows, String> {
    let pool = require_pool(&state).await?;
    explorer::get_rows(&pool, &database, &table, limit, offset).await
}

#[tauri::command]
pub async fn search_table_rows(
    state: State<'_, AppState>,
    database: String,
    table: String,
    column: String,
    query: String,
) -> Result<TableRows, String> {
    let pool = require_pool(&state).await?;
    explorer::search_rows(&pool, &database, &table, &column, &query, 200).await
}

// ---- Generic row read/write, built on the same introspection as the DB
// Explorer above - used by the Mob-Proto-Editor and Account-/Player-
// Verwaltung so those don't need this app to already know a table's exact
// column semantics (which was never verified against a live server for
// mob_proto/account/player, unlike item_proto). The primary-key column is
// auto-detected by the frontend from `get_table_columns`' `is_primary_key`
// flag rather than being guessed here. ----

#[tauri::command]
pub async fn get_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    pk_column: String,
    pk_value: String,
) -> Result<Option<TableRows>, String> {
    let pool = require_pool(&state).await?;
    explorer::get_row_by_pk(&pool, &database, &table, &pk_column, &pk_value).await
}

#[tauri::command]
pub async fn update_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    pk_column: String,
    pk_value: String,
    changes: Vec<(String, Option<String>)>,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    explorer::update_row(&pool, &database, &table, &pk_column, &pk_value, &changes).await
}

#[tauri::command]
pub async fn insert_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    values: Vec<(String, Option<String>)>,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    explorer::insert_row(&pool, &database, &table, &values).await
}

#[tauri::command]
pub async fn delete_table_row(
    state: State<'_, AppState>,
    database: String,
    table: String,
    pk_column: String,
    pk_value: String,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    explorer::delete_row(&pool, &database, &table, &pk_column, &pk_value).await
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
pub fn get_item_icon(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Option<String>, String> {
    let client_path = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        settings::get_path(&conn, "client_path")?
    }
    .ok_or_else(|| "Kein Client-Pfad konfiguriert.".to_string())?;
    icons::load_item_icon_data_url(&client_path, vnum)
}

#[tauri::command]
pub fn list_item_icon_files(state: State<'_, AppState>) -> Result<Vec<icons::IconFile>, String> {
    let client_path = client_path_setting(&state)?;
    Ok(icons::list_item_icon_files(&client_path))
}

#[tauri::command]
pub fn load_icon_file(absolute_path: String) -> Result<String, String> {
    icons::load_icon_file_data_url(&absolute_path)
}

#[tauri::command]
pub fn locate_npc_model(
    state: State<'_, AppState>,
    npc_vnum: i32,
    folder: Option<String>,
) -> Result<(String, String), String> {
    let (client_path, npclist_override) = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        (
            settings::get_path(&conn, "client_path")?,
            settings::get_path(&conn, "npclist_path")?,
        )
    };
    let client_path = client_path.ok_or_else(|| {
        "Kein Client-Pfad konfiguriert. Bitte in den Einstellungen setzen.".to_string()
    })?;

    let dll = gr2::find_granny_dll(&client_path)
        .ok_or_else(|| format!("granny2.dll nicht gefunden unter {client_path}"))?;

    // npclist.txt is the client's own mapping and covers shop NPCs, whose
    // mob_proto.folder is typically empty - fall back to the DB value only if
    // the client list has no entry.
    let from_list = gr2::find_npclist(&client_path, npclist_override.as_deref())
        .and_then(|list| gr2::lookup_npc_folder(&list, npc_vnum));

    let resolved = from_list
        .or(folder)
        .filter(|f| !f.is_empty())
        .ok_or_else(|| {
            format!(
                "Kein Modell-Ordner für NPC {npc_vnum} gefunden. \
                 npclist.txt wurde nicht gefunden oder enthält keinen Eintrag - \
                 Pfad ggf. in den Einstellungen setzen."
            )
        })?;

    let model = gr2::find_npc_model(&client_path, &resolved).ok_or_else(|| {
        format!("Kein .gr2-Modell für '{resolved}' im Client-Ordner gefunden")
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

// ---- Item Editor ----

#[tauri::command]
pub async fn item_vnum_exists(state: State<'_, AppState>, vnum: u32) -> Result<bool, String> {
    let pool = require_pool(&state).await?;
    item::vnum_exists(&pool, vnum).await
}

#[tauri::command]
pub async fn next_free_item_vnum(
    state: State<'_, AppState>,
    range_start: u32,
) -> Result<u32, String> {
    let pool = require_pool(&state).await?;
    item::next_free_vnum(&pool, range_start).await
}

#[tauri::command]
pub async fn get_item_proto(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Option<ItemProtoFull>, String> {
    let pool = require_pool(&state).await?;
    item::get_item_proto(&pool, vnum).await
}

#[tauri::command]
pub async fn create_item_proto(
    state: State<'_, AppState>,
    item: ItemProtoInput,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    item::create_item_proto(&pool, &item).await
}

#[tauri::command]
pub async fn update_item_proto(
    state: State<'_, AppState>,
    item: ItemProtoInput,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    item::update_item_proto(&pool, &item).await
}

#[tauri::command]
pub async fn delete_item_proto(state: State<'_, AppState>, vnum: u32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    item::delete_item_proto(&pool, vnum).await
}

fn item_editor_setting(state: &State<'_, AppState>, key: &str) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, key)?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("Einstellung '{key}' ist nicht gesetzt. Bitte unter Einstellungen konfigurieren."))
}

/// Checks every path the item-creation pipeline depends on *before* anything
/// destructive (including the DB insert) runs, so a misconfigured setting
/// fails loudly upfront instead of leaving a half-created item behind.
#[tauri::command]
pub fn validate_item_editor_setup(
    state: State<'_, AppState>,
    require_icon_tool: bool,
) -> Result<(), String> {
    let mut problems = Vec::new();

    match item_editor_setting(&state, "client_path") {
        Ok(path) if !std::path::Path::new(&path).is_dir() => {
            problems.push(format!("Client-Pfad existiert nicht: {path}"));
        }
        Err(e) => problems.push(e),
        _ => {}
    }

    if require_icon_tool {
        match item_editor_setting(&state, "eterpack_tool_path") {
            Ok(path) if !std::path::Path::new(&path).is_file() => {
                problems.push(format!("EterPackConsoleLz4.exe nicht gefunden unter: {path}"));
            }
            Err(e) => problems.push(e),
            _ => {}
        }
    }

    match item_editor_setting(&state, "mysql2proto_dir") {
        Ok(dir) => {
            let exe = std::path::Path::new(&dir).join("Mysql2Proto.exe");
            if !exe.is_file() {
                problems.push(format!("Mysql2Proto.exe nicht gefunden unter: {}", exe.display()));
            }
        }
        Err(e) => problems.push(e),
    }

    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

#[tauri::command]
pub fn write_item_icon(
    state: State<'_, AppState>,
    vnum: u32,
    source_path: String,
) -> Result<String, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let path = packtools::write_icon_tga(&client_path, vnum, &source_path)?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn pack_item_icons(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "icon").await
}

#[tauri::command]
pub fn write_item_model(
    state: State<'_, AppState>,
    vnum: u32,
    source_vnum: u32,
) -> Result<String, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let path = packtools::copy_weapon_model(&client_path, vnum, source_vnum)?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub async fn pack_item_models(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "item").await
}

#[tauri::command]
pub fn write_item_list_entry(
    state: State<'_, AppState>,
    vnum: u32,
    item_type: u32,
    icon_rel_path: String,
    model_rel_path: Option<String>,
) -> Result<Vec<String>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    packtools::upsert_item_list_entries(
        &client_path,
        vnum,
        item_type,
        &icon_rel_path,
        model_rel_path.as_deref(),
    )
}

#[tauri::command]
pub async fn regenerate_item_proto(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let tool_dir = item_editor_setting(&state, "mysql2proto_dir")?;
    let generated = packtools::run_mysql2proto(&app, &tool_dir).await?;
    Ok(generated.display().to_string())
}

#[tauri::command]
pub fn deploy_item_proto(
    state: State<'_, AppState>,
    generated_proto_path: String,
) -> Result<Vec<String>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    packtools::replace_client_item_proto(&client_path, &generated_proto_path)
}

// ---- Modul-Importer (beliebige Ausrüstungs-Pakete: Waffen + Rüstung) ----

#[tauri::command]
pub fn scan_module(path: String) -> Result<ScannedModule, String> {
    modulescan::scan_module(std::path::Path::new(&path))
}

#[tauri::command]
pub fn import_weapon_model(
    state: State<'_, AppState>,
    module_name: String,
    source_abs: String,
    texture_sources: Vec<String>,
) -> Result<(String, String), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let (dest, virtual_path) = packtools::import_custom_weapon_model(
        &client_path,
        &module_name,
        std::path::Path::new(&source_abs),
        &texture_sources,
    )?;
    Ok((dest.display().to_string(), virtual_path))
}

/// Allocates a single fresh `value3`/`ShapeIndex` for a whole armor piece -
/// call **once per item**, before looping `import_armor_model` over that
/// item's selected races. It must not be re-derived per race: the same
/// numeric index has to end up in every involved race's `.msm` (each
/// pointing at that race's own model) so the one `item_proto` row renders
/// consistently everywhere it's equippable; calling this per-race would
/// hand out a different, incompatible index each time since every previous
/// `.msm` write raises the observed maximum.
#[tauri::command]
pub async fn next_free_shape_index(state: State<'_, AppState>) -> Result<u32, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let pool = require_pool(&state).await?;
    let max_db_value3 = item::max_armor_value3(&pool).await?;
    Ok(msm::next_free_shape_index(&client_path, max_db_value3))
}

/// Imports a female armor body model and wires it into that race's `.msm`
/// under the given (already-allocated, see `next_free_shape_index`)
/// `shape_index` - see `msm.rs`'s module doc for why there is no male
/// equivalent.
#[tauri::command]
pub fn import_armor_model(
    state: State<'_, AppState>,
    module_name: String,
    race: String,
    source_abs: String,
    texture_sources: Vec<String>,
    shape_index: u32,
) -> Result<(), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let (_, model_rel, skin_rel) = packtools::import_custom_armor_model(
        &client_path,
        &module_name,
        &race,
        std::path::Path::new(&source_abs),
        &texture_sources,
    )?;
    msm::add_shape_data(&client_path, &race, shape_index, &model_rel, &skin_rel)
}

/// Repacks `<client>/pack/<folder_name>.epk` - a generic counterpart to
/// `pack_item_icons`/`pack_item_models`/`pack_item_effects` for the
/// module importer's other pack folders (e.g. `pc_warrior`, `pc_sura`).
#[tauri::command]
pub async fn pack_folder(app: AppHandle, state: State<'_, AppState>, folder_name: String) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, &folder_name).await
}

#[tauri::command]
pub fn import_effect_bundle(
    state: State<'_, AppState>,
    module_name: String,
    source_dir: String,
) -> Result<Vec<String>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let copied =
        packtools::import_effect_bundle(&client_path, &module_name, std::path::Path::new(&source_dir))?;
    Ok(copied.into_iter().map(|p| p.display().to_string()).collect())
}

#[tauri::command]
pub async fn pack_item_effects(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "effect").await
}

// ---- Modul-Importer: Verlauf & Rückgängig-machen ----

#[tauri::command]
pub fn record_import_batch(
    state: State<'_, AppState>,
    module_name: String,
    item_type: i32,
    vnums: Vec<u32>,
    had_effects: bool,
) -> Result<i64, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    import_history::record_batch(&conn, &module_name, item_type, &vnums, had_effects)
}

#[tauri::command]
pub fn list_import_batches(state: State<'_, AppState>) -> Result<Vec<import_history::ImportBatch>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    import_history::list_batches(&conn)
}

/// Fully undoes a single imported item: removes its `item_proto` row, its
/// `item_list.txt` entry (in every locale) and its icon file. Does *not*
/// touch the copied `.gr2`/effect files under `pack/.../custom/<module>/` -
/// those can be shared by other vnums imported from the same module run
/// (e.g. re-running an import overwrites the same destination path), so
/// deleting them per-vnum would risk breaking a sibling item. Leaving them
/// behind is harmless: without a DB row or `item_list.txt` entry, nothing
/// references them anymore.
async fn teardown_item(pool: &sqlx::MySqlPool, client_path: &str, vnum: u32) -> Result<(), String> {
    item::delete_item_proto(pool, vnum).await?;
    packtools::remove_item_list_entries(client_path, vnum)?;
    packtools::delete_item_icon(client_path, vnum)?;
    Ok(())
}

#[tauri::command]
pub async fn remove_single_item(app: AppHandle, state: State<'_, AppState>, vnum: u32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    let client_path = item_editor_setting(&state, "client_path")?;

    teardown_item(&pool, &client_path, vnum).await?;

    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "icon").await?;
    let generated = packtools::run_mysql2proto(&app, &item_editor_setting(&state, "mysql2proto_dir")?).await?;
    packtools::replace_client_item_proto(&client_path, &generated.display().to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn undo_import_batch(app: AppHandle, state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let batch = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        import_history::get_batch(&conn, id)?
            .ok_or_else(|| format!("Import-Batch {id} nicht gefunden (bereits entfernt?)"))?
    };

    let pool = require_pool(&state).await?;
    let client_path = item_editor_setting(&state, "client_path")?;

    for vnum in &batch.vnums {
        teardown_item(&pool, &client_path, *vnum).await?;
    }

    let tool_path = item_editor_setting(&state, "eterpack_tool_path")?;
    packtools::run_eterpack_pack(&app, &tool_path, "icon").await?;
    let generated = packtools::run_mysql2proto(&app, &item_editor_setting(&state, "mysql2proto_dir")?).await?;
    packtools::replace_client_item_proto(&client_path, &generated.display().to_string())?;

    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    import_history::delete_batch_record(&conn, id)?;
    Ok(())
}

