//! Webhook-Benachrichtigungen, automatisierte Datenbank-Backups, und die
//! generische Remote-Verzeichnis-Browsing/Restore-Basis für den
//! Backup-Browser. Webhook-Commands sitzen hier statt in `misc.rs`, weil ihr
//! einziger Aufrufer aus diesem Modul (`create_database_backup`) sie direkt
//! bei einem fehlgeschlagenen Backup feuert - im Original standen sie auch
//! im selben Datei-Abschnitt.

use crate::backups;
use crate::credentials;
use crate::db_backup;
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use crate::webhook;
use tauri::State;

use super::support::{notify_webhook_best_effort, require_pool, stored_ssh_auth, webhook_url};

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
        .unwrap_or_else(|| "account common player log".to_string())
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
    let (dir, configured_databases) = db_backup_settings(&state)?;
    let (host, port, username) = mysql_connection_settings(&state)?;
    let password = credentials::get_secret("mysql_password")
        .map_err(|_| "Kein MySQL-Passwort im Credential-Manager gefunden.".to_string())?;

    // mysqldump --databases aborts the *entire* dump the moment even one
    // named database doesn't exist (real live failure: the default guess
    // included "website", not present on this server) - check against what
    // actually exists first so a stale/wrong name only drops that one
    // database instead of failing the whole backup.
    let pool = require_pool(&state).await?;
    let real_databases = crate::db::explorer::list_databases(&pool).await?;
    let (databases, skipped) =
        db_backup::split_existing_databases(&configured_databases, &real_databases);
    if databases.is_empty() {
        return Err(format!(
            "Keine der konfigurierten Datenbanken existiert auf diesem Server: {}",
            configured_databases.join(", ")
        ));
    }

    let filename = db_backup::backup_filename(chrono::Local::now());
    let command = db_backup::build_dump_command(
        &host, port, &username, &password, &databases, &dir, &filename,
    );

    let result = ssh::run_command_streaming(&config, &auth, &command, |_| {}).await;
    match result {
        Ok(r) if r.exit_status == Some(0) => {
            let path = format!("{dir}/{filename}");
            if skipped.is_empty() {
                Ok(path)
            } else {
                Ok(format!(
                    "{path} (übersprungen, da auf diesem Server nicht vorhanden: {})",
                    skipped.join(", ")
                ))
            }
        }
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

// ---- Remote directory browsing (shared by Regen-Datei-Editor + Backup-Browser) ----

#[tauri::command]
pub async fn list_remote_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<ssh::RemoteEntry>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    ssh::list_remote_dir(&config, &auth, &path).await
}

/// Used by the DB-Backups page to list its own backup folder - see
/// `ssh::list_remote_dir_or_empty` for why a missing folder isn't an error
/// there specifically (unlike the generic `list_remote_dir` above, still
/// used by Regen-Datei-Editor/Backup-Browser where a missing user-configured
/// path really could be a typo worth surfacing).
#[tauri::command]
pub async fn list_backup_dir(
    state: State<'_, AppState>,
    path: String,
) -> Result<Vec<ssh::RemoteEntry>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    ssh::list_remote_dir_or_empty(&config, &auth, &path).await
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

    // Byte-exact, no text decode/encode round-trip - a backup file this app
    // itself listed could be anything a *different* editor wrote there (or
    // even something a user manually dropped into the same m2manager_backups
    // convention), and interpreting it as UTF-8/Windows-1252 text is lossy
    // for genuine binary content (see ssh::encode_matching's own doc).
    let content = ssh::read_remote_file_bytes(&config, &auth, &backup_path).await?;
    ssh::write_remote_file_bytes_with_backup(&config, &auth, &target_path, &content).await?;
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

    let backup_bytes = ssh::read_remote_file_bytes(&config, &auth, &backup_path).await?;
    let current_bytes = ssh::read_remote_file_bytes_if_exists(&config, &auth, &target_path).await?;

    let is_binary = ssh::looks_binary(&backup_bytes)
        || current_bytes.as_deref().is_some_and(ssh::looks_binary);

    Ok(backups::BackupDiff {
        target_path,
        backup_content: ssh::decode_bytes(backup_bytes),
        current_content: current_bytes.map(ssh::decode_bytes),
        is_binary,
    })
}
