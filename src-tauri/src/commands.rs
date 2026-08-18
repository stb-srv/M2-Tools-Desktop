use crate::credentials;
use crate::db::account;
use crate::db::event_flags;
use crate::db::mysql::{self, MysqlConfig};
use crate::db::explorer::{self, ColumnInfo, TableInfo, TableRows};
use crate::db::item::{self, ItemBrief, ItemProtoFull, ItemProtoInput};
use crate::drop_item_group;
use crate::etc_drop;
use crate::db::item_explorer::{self, ItemProtoPage};
use crate::db::shop::{self, DatabaseStats, EntityBrowsePage, ItemSearchResult, ShopItem, ShopSummary};
use crate::gr2::{self, ModelInfo};
use crate::icons;
use crate::import_history;
use crate::itemdesc;
use crate::mobdrop;
use crate::modulescan::{self, ScannedModule};
use crate::msm;
use crate::packtools;
use crate::backups;
use crate::bans;
use crate::broadcast;
use crate::build_deploy;
use crate::locale;
use crate::mapdata;
use crate::db_backup;
use crate::deploy_history;
use crate::quest;
use crate::refine;
use crate::regen;
use crate::resources;
use crate::webhook;
use crate::settings;
use crate::cube;
use crate::special_item_group;
use crate::ssh::{self, SshAuth, SshConfig};
use russh_sftp::client::SftpSession;
use crate::state::AppState;
use crate::system_installs::{self, FileAction, InstalledFile, TargetKind};
use crate::system_patch::{self, InsertionResolution, PatchOp, Placement};
use crate::system_scan::{self, ScannedFile};
use crate::weather;
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

// ---- Server-Quellcode Bauen & Einspielen ----
//
// Baut nie direkt im Live-Quellbaum, immer in einer separaten Arbeitskopie
// (siehe build_deploy.rs) - deshalb sind diese beiden Commands unabhängig
// vom riskanten Einspiel-Schritt (deploy_history.rs) gefahrlos nutzbar.

fn build_deploy_setting(state: &State<'_, AppState>, key: &str, default: &str) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, key)?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string()))
}

#[tauri::command]
pub fn list_build_targets() -> build_deploy::BuildTargets {
    build_deploy::build_targets()
}

#[tauri::command]
pub async fn sync_build_source(app: AppHandle, state: State<'_, AppState>) -> Result<ServerCommandResult, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let live = build_deploy_setting(&state, "build_live_source_root", "/usr/home/source/server")?;
    let scratch = build_deploy_setting(&state, "build_scratch_source_root", "/usr/home/m2manager_build/server")?;
    let command = build_deploy::sync_script(&live, &scratch);
    let result = ssh::run_command_streaming(&config, &auth, &command, |chunk| {
        let _ = app.emit("build-deploy-output", chunk);
    })
    .await?;
    Ok(ServerCommandResult { output: result.output, exit_status: result.exit_status })
}

#[tauri::command]
pub async fn run_source_build(
    app: AppHandle,
    state: State<'_, AppState>,
    options: build_deploy::BuildOptions,
) -> Result<ServerCommandResult, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let command = build_deploy::build_script(&options);
    let result = ssh::run_command_streaming(&config, &auth, &command, |chunk| {
        let _ = app.emit("build-deploy-output", chunk);
    })
    .await?;
    Ok(ServerCommandResult { output: result.output, exit_status: result.exit_status })
}

/// Zwei `ps`-Momentaufnahmen im Abstand einiger Sekunden nach dem Starten -
/// eine einzelne "läuft überhaupt etwas"-Prüfung würde eine Absturzschleife
/// (Prozess stirbt, ein Supervisor startet ihn mit neuer PID sofort wieder)
/// fälschlich als gesund melden. Nur "stabil", wenn beide Aufnahmen exakt
/// dieselbe PID-Menge zeigen.
async fn verify_liveness(
    app: &AppHandle,
    state: &State<'_, AppState>,
    config: &ssh::SshConfig,
    auth: &ssh::SshAuth,
) -> Result<bool, String> {
    let names = server_process_names(state)?;
    let wait: u64 = build_deploy_setting(state, "deploy_liveness_wait_seconds", "8")?
        .parse()
        .unwrap_or(8);
    let recheck: u64 = build_deploy_setting(state, "deploy_liveness_recheck_seconds", "5")?
        .parse()
        .unwrap_or(5);

    tokio::time::sleep(std::time::Duration::from_secs(wait)).await;
    let first = ssh::run_command_streaming(config, auth, "ps -axo pid,pcpu,pmem,rss,comm", |_| {}).await?;
    let first_pids: std::collections::HashSet<u32> =
        resources::parse_and_filter(&first.output, &names).into_iter().map(|p| p.pid).collect();

    tokio::time::sleep(std::time::Duration::from_secs(recheck)).await;
    let second = ssh::run_command_streaming(config, auth, "ps -axo pid,pcpu,pmem,rss,comm", |_| {}).await?;
    let second_pids: std::collections::HashSet<u32> =
        resources::parse_and_filter(&second.output, &names).into_iter().map(|p| p.pid).collect();

    let ok = !first_pids.is_empty() && first_pids == second_pids;
    let message = if ok {
        format!("\n[OK] {} Prozess(e) stabil (PIDs: {:?})\n", first_pids.len(), first_pids)
    } else {
        format!(
            "\n[WARNUNG] Kein stabiler Prozess gefunden - erste Prüfung: {:?}, zweite: {:?}\n",
            first_pids, second_pids
        )
    };
    let _ = app.emit("build-deploy-output", message);
    Ok(ok)
}

/// Stoppt den Server, sichert die aktuellen Live-Programmdateien, kopiert
/// die frisch gebauten aus der Arbeitskopie ein, startet neu und prüft, ob
/// die erwarteten Prozesse stabil laufen. `targets` ist eine Teilmenge aus
/// `["game", "db"]`. Betrifft bei "game" sofort ALLE Channels + den
/// Login-Server (gemeinsame Programmdatei per Symlink, live verifiziert) -
/// das Frontend muss vor dem Aufruf eine explizite, eingetippte Bestätigung
/// einholen (kein Soft-Gate hier im Backend, das ist bewusst UI-Sache).
#[tauri::command]
pub async fn run_deploy(
    app: AppHandle,
    state: State<'_, AppState>,
    targets: Vec<String>,
    note: String,
) -> Result<deploy_history::DeployRecord, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let scratch = build_deploy_setting(&state, "build_scratch_source_root", "/usr/home/m2manager_build/server")?;
    let live_game = build_deploy_setting(&state, "build_live_game_binary", "/usr/home/source/server/game/game")?;
    let live_db = build_deploy_setting(&state, "build_live_db_binary", "/usr/home/source/server/db/db")?;
    let stop_cmd = build_deploy_setting(&state, "server_cmd_stop", "")?;
    let start_cmd = build_deploy_setting(&state, "server_cmd_start", "")?;
    if stop_cmd.is_empty() || start_cmd.is_empty() {
        return Err(
            "server_cmd_stop/server_cmd_start sind nicht gesetzt - bitte erst in der Server-Steuerung einrichten."
                .to_string(),
        );
    }

    let emit_stage = |stage: &str| {
        let _ = app.emit("build-deploy-output", format!("\n=== {stage} ===\n"));
    };
    let emit_output = |chunk: &str| {
        let _ = app.emit("build-deploy-output", chunk);
    };

    emit_stage("1/5 Server stoppen (alle Channels + Login-Server)");
    ssh::run_command_streaming(&config, &auth, &stop_cmd, emit_output).await?;

    emit_stage("2/5 Aktuelle Live-Programmdateien sichern");
    let game_backup = if targets.iter().any(|t| t == "game") {
        ssh::backup_remote_binary(&config, &auth, &live_game).await?
    } else {
        None
    };
    let db_backup = if targets.iter().any(|t| t == "db") {
        ssh::backup_remote_binary(&config, &auth, &live_db).await?
    } else {
        None
    };

    emit_stage("3/5 Neu gebaute Programmdateien einspielen");
    let mut copy_cmds = Vec::new();
    if targets.iter().any(|t| t == "game") {
        copy_cmds.push(format!("cp -p '{scratch}/game/game' '{live_game}'"));
    }
    if targets.iter().any(|t| t == "db") {
        copy_cmds.push(format!("cp -p '{scratch}/db/db' '{live_db}'"));
    }
    if copy_cmds.is_empty() {
        return Err("Keine Ziele ausgewählt (weder 'game' noch 'db').".to_string());
    }
    ssh::run_command_streaming(&config, &auth, &copy_cmds.join(" && "), emit_output).await?;

    let deploy_id = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        deploy_history::record_deploy(
            &conn,
            "deploy",
            &targets,
            game_backup.as_deref(),
            db_backup.as_deref(),
            &note,
            None,
        )?
    };

    emit_stage("4/5 Server starten (alle Channels + Login-Server)");
    ssh::run_command_streaming(&config, &auth, &start_cmd, emit_output).await?;

    emit_stage("5/5 Live-Prüfung");
    let ok = verify_liveness(&app, &state, &config, &auth).await?;

    {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        deploy_history::update_deploy_success(&conn, deploy_id, ok)?;
    }

    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    deploy_history::get_deploy(&conn, deploy_id)?.ok_or_else(|| "Deploy-Eintrag nicht gefunden".to_string())
}

/// Spiegelbild von `run_deploy`: stoppt, kopiert die gesicherte Datei
/// zurück (Backup bleibt zusätzlich liegen, falls ein zweiter Rückroll
/// nötig wird), startet neu, prüft die Stabilität, protokolliert einen
/// neuen `"rollback"`-Verlauf-Eintrag. `deploy_id` fehlt = jüngster
/// `"deploy"`-Eintrag wird verwendet.
#[tauri::command]
pub async fn run_rollback(
    app: AppHandle,
    state: State<'_, AppState>,
    deploy_id: Option<i64>,
) -> Result<deploy_history::DeployRecord, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let live_game = build_deploy_setting(&state, "build_live_game_binary", "/usr/home/source/server/game/game")?;
    let live_db = build_deploy_setting(&state, "build_live_db_binary", "/usr/home/source/server/db/db")?;
    let stop_cmd = build_deploy_setting(&state, "server_cmd_stop", "")?;
    let start_cmd = build_deploy_setting(&state, "server_cmd_start", "")?;
    if stop_cmd.is_empty() || start_cmd.is_empty() {
        return Err(
            "server_cmd_stop/server_cmd_start sind nicht gesetzt - bitte erst in der Server-Steuerung einrichten."
                .to_string(),
        );
    }

    let target_record = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        match deploy_id {
            Some(id) => deploy_history::get_deploy(&conn, id)?,
            None => deploy_history::latest_deploy(&conn)?,
        }
    }
    .ok_or_else(|| "Kein Einspiel-Verlauf gefunden, auf den zurückgerollt werden könnte.".to_string())?;

    let emit_stage = |stage: &str| {
        let _ = app.emit("build-deploy-output", format!("\n=== {stage} ===\n"));
    };
    let emit_output = |chunk: &str| {
        let _ = app.emit("build-deploy-output", chunk);
    };

    emit_stage("1/4 Server stoppen (alle Channels + Login-Server)");
    ssh::run_command_streaming(&config, &auth, &stop_cmd, emit_output).await?;

    emit_stage("2/4 Gesicherte Programmdatei(en) zurückkopieren");
    let mut copy_cmds = Vec::new();
    if let Some(backup) = &target_record.game_backup_path {
        copy_cmds.push(format!("cp -p '{backup}' '{live_game}'"));
    }
    if let Some(backup) = &target_record.db_backup_path {
        copy_cmds.push(format!("cp -p '{backup}' '{live_db}'"));
    }
    if copy_cmds.is_empty() {
        return Err("Der gewählte Verlauf-Eintrag hat keine Sicherungsdatei(en) hinterlegt.".to_string());
    }
    ssh::run_command_streaming(&config, &auth, &copy_cmds.join(" && "), emit_output).await?;

    let rollback_id = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        deploy_history::record_deploy(
            &conn,
            "rollback",
            &target_record.targets,
            target_record.game_backup_path.as_deref(),
            target_record.db_backup_path.as_deref(),
            &format!("Rückgängig-machen von Eintrag #{}", target_record.id),
            Some(target_record.id),
        )?
    };

    emit_stage("3/4 Server starten (alle Channels + Login-Server)");
    ssh::run_command_streaming(&config, &auth, &start_cmd, emit_output).await?;

    emit_stage("4/4 Live-Prüfung");
    let ok = verify_liveness(&app, &state, &config, &auth).await?;

    {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        deploy_history::update_deploy_success(&conn, rollback_id, ok)?;
    }

    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    deploy_history::get_deploy(&conn, rollback_id)?.ok_or_else(|| "Rollback-Eintrag nicht gefunden".to_string())
}

#[tauri::command]
pub fn list_deploy_history(state: State<'_, AppState>) -> Result<Vec<deploy_history::DeployRecord>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    deploy_history::list_deploys(&conn)
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

// ---- Kisten-Editor (special_item_group.txt - GIFTBOX-Beutetabellen) ----

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

// ---- Drop-Generator: common_drop_item.txt / etc_drop_item.txt /
// drop_item_group.txt (siehe common_drop.rs/etc_drop.rs/drop_item_group.rs
// für die verifizierten Datei-Formate) - gleiches SFTP-Lade/Speicher-Muster
// wie cube.txt/special_item_group.txt: Rundreise-Sanity-Check vor jedem
// Hochladen, Backup vor jedem Überschreiben (`write_remote_file_with_backup`).

fn common_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "common_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/common_drop_item.txt".to_string()))
}

// Live gegen den echten Dev-Server verifiziert (2026-08-18, nach einem
// realen Ladefehler): entgegen der ursprünglichen, aus dem generischen
// game-src-Checkout abgeleiteten Annahme (24-Tab-Felder-pro-Zeile,
// Rang-Level-Brackets - `ReadCommonDropItemFile` in
// `item_manager_read_tables.cpp`) benutzt dieser Fork für
// `common_drop_item.txt` tatsächlich **dieselbe Group/Mob/Type-Grammatik
// wie `mob_drop_item.txt`** (echter Datei-Inhalt: `Group\tMetinStein1\n{\n
// \tMob\t8001\n\tType\tdrop\n\t1\t19\t1\t100\n}`) - der generische
// Quellcode-Checkout entspricht an dieser Stelle offenbar nicht dem, was auf
// diesem Server tatsächlich läuft (individuelle Fork-Anpassung). Wiederverwendet
// deshalb direkt `mobdrop::parse`/`serialize` (identischer `MobDropGroup`-Typ)
// statt eines eigenen, nachweislich falschen Parsers.
#[tauri::command]
pub async fn read_common_drop_file(state: State<'_, AppState>) -> Result<Vec<mobdrop::MobDropGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = common_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    mobdrop::parse(&content)
}

#[tauri::command]
pub async fn write_common_drop_file(
    state: State<'_, AppState>,
    groups: Vec<mobdrop::MobDropGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = common_drop_file_path(&state)?;
    let content = mobdrop::serialize(&groups);
    mobdrop::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

fn etc_drop_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "etc_drop_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/etc_drop_item.txt".to_string()))
}

#[tauri::command]
pub async fn read_etc_drop_file(state: State<'_, AppState>) -> Result<Vec<etc_drop::EtcDropEntry>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = etc_drop_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    etc_drop::parse(&content)
}

#[tauri::command]
pub async fn write_etc_drop_file(
    state: State<'_, AppState>,
    entries: Vec<etc_drop::EtcDropEntry>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = etc_drop_file_path(&state)?;
    let content = etc_drop::serialize(&entries);
    etc_drop::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

fn drop_item_group_file_path(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "drop_item_group_file_path")?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "/usr/home/game/share/drop_item_group.txt".to_string()))
}

#[tauri::command]
pub async fn read_drop_item_group_file(
    state: State<'_, AppState>,
) -> Result<Vec<drop_item_group::DropItemGroup>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = drop_item_group_file_path(&state)?;
    let content = ssh::read_remote_file(&config, &auth, &path).await?;
    drop_item_group::parse(&content)
}

#[tauri::command]
pub async fn write_drop_item_group_file(
    state: State<'_, AppState>,
    groups: Vec<drop_item_group::DropItemGroup>,
) -> Result<Option<String>, String> {
    let (config, auth) = stored_ssh_auth(&state)?;
    let path = drop_item_group_file_path(&state)?;
    let content = drop_item_group::serialize(&groups);
    drop_item_group::parse(&content)?;
    ssh::write_remote_file_with_backup(&config, &auth, &path, &content).await
}

/// Löst die per `EntityBrowser` gewählte vnum in den echten internen
/// `item_proto.name` auf - für Etc-Drops, siehe `etc_drop.rs`.
#[tauri::command]
pub async fn get_item_internal_name(state: State<'_, AppState>, vnum: u32) -> Result<String, String> {
    let pool = require_pool(&state).await?;
    item::get_item_internal_name(&pool, vnum).await
}

/// Rückwärtssuche für die Anzeige bestehender Etc-Drop-Einträge - siehe
/// `etc_drop.rs`.
#[tauri::command]
pub async fn find_item_by_internal_name(state: State<'_, AppState>, name: String) -> Result<Option<ItemBrief>, String> {
    let pool = require_pool(&state).await?;
    item::find_item_by_internal_name(&pool, &name).await
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

/// Writes arbitrary text to a local path the user picked via a save dialog -
/// used for CSV exports of search/filter results. Unlike the various
/// `write_local_*` commands above, this makes no assumption about the
/// content's format (no parse-back check), since the destination is always a
/// fresh export file the user is about to create, not an existing config
/// file being overwritten.
#[tauri::command]
pub fn export_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| format!("Datei konnte nicht geschrieben werden: {e}"))
}

/// Guards against a caller-supplied `relative_path` escaping the trusted
/// base directory it gets joined onto (`{dir}/{relative_path}`) via `..`
/// segments or an absolute path - used by every quest/regen file command
/// that takes a `relative_path` directly from the frontend (read/write/
/// delete quest file, read/write regen file). `create_quest_file` builds its
/// own `relative_path` from already-sanitized `category`/`name` and doesn't
/// need this. Without it, a malformed or malicious value could read,
/// overwrite, or delete an arbitrary file on the server outside the
/// intended quest/regen directory.
fn safe_relative_path(relative_path: &str) -> Result<&str, String> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.split('/').any(|seg| seg.is_empty() || seg == "..")
    {
        return Err(format!("Ungültiger relativer Pfad: \"{relative_path}\"."));
    }
    Ok(relative_path)
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

// ---- Broadcast-System ----
//
// The message list itself is purely local (app-owned SQLite, see
// `broadcast.rs`) - deploying it is just generating one quest file from that
// list on the frontend (`broadcastQuest.ts`) and pushing it through the
// already-existing `create_quest_file`/`write_quest_file` commands above,
// then reloading via the already-existing `run_server_command` +
// `server_cmd_reload_quests` setting (same as Server Control/Quest
// Builder's "Quests reloaden"/"Kompilieren & Neuladen"). No new SSH logic.

#[tauri::command]
pub fn list_broadcast_messages(state: State<'_, AppState>) -> Result<Vec<broadcast::BroadcastMessage>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::list_messages(&conn)
}

#[tauri::command]
pub fn create_broadcast_message(
    state: State<'_, AppState>,
    text: String,
    interval_minutes: i64,
) -> Result<i64, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::create_message(&conn, &text, interval_minutes)
}

#[tauri::command]
pub fn update_broadcast_message(
    state: State<'_, AppState>,
    id: i64,
    text: String,
    interval_minutes: i64,
) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::update_message(&conn, id, &text, interval_minutes)
}

#[tauri::command]
pub fn set_broadcast_message_enabled(state: State<'_, AppState>, id: i64, enabled: bool) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::set_enabled(&conn, id, enabled)
}

#[tauri::command]
pub fn delete_broadcast_message(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    broadcast::delete_message(&conn, id)
}

// ---- Tag/Nacht & Schnee ----
//
// Gleiches Prinzip wie das Broadcast-System: lokaler SQLite-Zustand
// (weather.rs), Deploy generiert eine Quest-Datei und schaltet sie live per
// bereits vorhandenem `create_quest_file`/`write_quest_file` +
// `run_server_command` scharf - kein Server-Neustart nötig (siehe
// weather.rs für das Warum: ein reiner DB-Write wäre hier wirkungslos).

#[tauri::command]
pub fn get_weather_state(state: State<'_, AppState>) -> Result<weather::WeatherState, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    weather::get_state(&conn)
}

#[tauri::command]
pub fn set_weather_state(
    state: State<'_, AppState>,
    night_enabled: bool,
    snow_enabled: bool,
) -> Result<weather::WeatherState, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    weather::set_state(&conn, night_enabled, snow_enabled)
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

// ---- Server-Events (globale Event-Flags, siehe db/event_flags.rs) ----

#[tauri::command]
pub async fn list_event_flags(state: State<'_, AppState>) -> Result<Vec<event_flags::EventFlagRow>, String> {
    let pool = require_pool(&state).await?;
    event_flags::list_event_flags(&pool).await
}

#[tauri::command]
pub async fn set_event_flag(state: State<'_, AppState>, name: String, value: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    event_flags::set_event_flag(&pool, &name, value).await
}

#[tauri::command]
pub async fn delete_event_flag(state: State<'_, AppState>, name: String) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    event_flags::delete_event_flag(&pool, &name).await
}

// ---- Account-Verwaltung ----
//
// Eigene, gezielte Commands statt der generischen DB-Explorer-Zeilen-CRUD -
// Passwörter müssen über MySQLs eigene PASSWORD()-Funktion gesetzt werden,
// nicht als Klartext-Spaltenwert (siehe db/account.rs), das kann der
// generische Insert/Update-Pfad nicht leisten.

#[tauri::command]
pub async fn list_accounts(
    state: State<'_, AppState>,
    search: String,
    limit: i64,
    offset: i64,
) -> Result<Vec<account::AccountSummary>, String> {
    let pool = require_pool(&state).await?;
    account::list_accounts(&pool, &search, limit, offset).await
}

#[tauri::command]
pub async fn count_accounts(state: State<'_, AppState>, search: String) -> Result<i64, String> {
    let pool = require_pool(&state).await?;
    account::count_accounts(&pool, &search).await
}

#[tauri::command]
pub async fn create_account(
    state: State<'_, AppState>,
    login: String,
    password: String,
    empire: Option<i8>,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    account::create_account(&pool, &login, &password, empire).await
}

#[tauri::command]
pub async fn reset_account_password(
    state: State<'_, AppState>,
    id: i32,
    new_password: String,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    account::reset_password(&pool, id, &new_password).await
}

// ---- Zeitgesteuerte Sperren (siehe bans.rs) ----
//
// Serverseitig gibt es dafür keinen Mechanismus - `status` ist ein freier
// String, den der Login-Server nur gegen "OK" vergleicht und sonst wörtlich
// als Fehlermeldung zeigt (siehe db/account.rs::set_status). Die
// Zeitsteuerung selbst ist rein lokal in M2Manager (SQLite `account_bans`)
// und greift nur, solange/wann immer die App läuft - kein Server-Cron.

/// Prüft die reale Spaltenbreite von `account.account.status` (nicht
/// geraten) und lehnt eine zu lange Sperr-Nachricht vorher ab, statt sie
/// stillschweigend von MySQL abschneiden zu lassen.
async fn validate_status_length(pool: &sqlx::MySqlPool, value: &str) -> Result<(), String> {
    let columns = explorer::get_columns(pool, "account", "account").await?;
    let Some(status_col) = columns.iter().find(|c| c.name == "status") else {
        return Ok(());
    };
    if let Some(max) = status_col
        .data_type
        .split('(')
        .nth(1)
        .and_then(|s| s.split(')').next())
        .and_then(|s| s.parse::<usize>().ok())
    {
        if value.chars().count() > max {
            return Err(format!(
                "Nachricht ist zu lang ({} Zeichen, die Spalte erlaubt maximal {}).",
                value.chars().count(),
                max
            ));
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn ban_account(
    state: State<'_, AppState>,
    account_id: i32,
    login: String,
    message: String,
    days: Option<i64>,
) -> Result<(), String> {
    if message.trim().is_empty() {
        return Err("Sperr-Nachricht darf nicht leer sein.".to_string());
    }
    let pool = require_pool(&state).await?;
    validate_status_length(&pool, &message).await?;
    account::set_status(&pool, account_id, &message).await?;
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    bans::create_ban(&conn, account_id as i64, &login, &message, days)?;
    Ok(())
}

#[tauri::command]
pub async fn unban_account(state: State<'_, AppState>, account_id: i32, ban_id: Option<i64>) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    account::set_status(&pool, account_id, "OK").await?;
    if let Some(ban_id) = ban_id {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        bans::deactivate_ban(&conn, ban_id)?;
    }
    Ok(())
}

#[tauri::command]
pub fn list_account_bans(state: State<'_, AppState>) -> Result<Vec<bans::BanRecord>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    bans::list_bans(&conn)
}

/// Setzt jede fällige Sperre automatisch zurück - aufgerufen beim Öffnen des
/// Account-Managers (siehe AccountManager.tsx), nicht per Hintergrunddienst.
#[tauri::command]
pub async fn process_due_bans(state: State<'_, AppState>) -> Result<u32, String> {
    let pool = require_pool(&state).await?;
    let due = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        bans::due_bans(&conn)?
    };
    let count = due.len() as u32;
    for record in due {
        account::set_status(&pool, record.account_id as i32, "OK").await?;
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        bans::deactivate_ban(&conn, record.id)?;
    }
    Ok(count)
}

// ---- Guthaben anpassen (Yang / unverifizierte Konto-Zusatzwährung) ----

#[tauri::command]
pub async fn adjust_player_gold(state: State<'_, AppState>, player_id: i32, delta: i64) -> Result<i64, String> {
    let pool = require_pool(&state).await?;
    account::adjust_player_gold(&pool, player_id, delta).await
}

/// `column` wird hier - nicht erst in db/account.rs - gegen eine frisch
/// geholte, echte Spaltenliste von `account.account` geprüft (Name UND
/// numerischer Typ), bevor sie in SQL interpoliert wird.
#[tauri::command]
pub async fn adjust_account_numeric_column(
    state: State<'_, AppState>,
    account_id: i32,
    column: String,
    delta: i64,
) -> Result<i64, String> {
    let pool = require_pool(&state).await?;
    let columns = explorer::get_columns(&pool, "account", "account").await?;
    const NUMERIC_TYPE_PREFIXES: &[&str] = &["int", "tinyint", "smallint", "mediumint", "bigint", "decimal", "float", "double"];
    let is_valid = columns.iter().any(|c| {
        c.name == column
            && !c.is_primary_key
            && NUMERIC_TYPE_PREFIXES.iter().any(|p| c.data_type.to_lowercase().starts_with(p))
    });
    if !is_valid {
        return Err(format!("Spalte '{column}' existiert nicht oder ist nicht numerisch."));
    }
    account::adjust_account_numeric_column(&pool, account_id, &column, delta).await
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

/// Paginated item browse/search used by pickers - see `shop::browse_items`
/// for why this exists alongside `search_items` (that one returns nothing
/// when a numeric query doesn't match an exact vnum, with no way to browse
/// what's actually there instead).
#[tauri::command]
pub async fn browse_items(
    state: State<'_, AppState>,
    query: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<EntityBrowsePage, String> {
    let pool = require_pool(&state).await?;
    shop::browse_items(&pool, query.as_deref(), offset, limit).await
}

#[tauri::command]
pub async fn browse_mobs(
    state: State<'_, AppState>,
    query: Option<String>,
    offset: i64,
    limit: i64,
) -> Result<EntityBrowsePage, String> {
    let pool = require_pool(&state).await?;
    shop::browse_mobs(&pool, query.as_deref(), offset, limit).await
}

#[tauri::command]
pub async fn browse_item_proto(
    state: State<'_, AppState>,
    query: Option<String>,
    type_filter: Option<i8>,
    offset: i64,
    limit: i64,
) -> Result<ItemProtoPage, String> {
    let pool = require_pool(&state).await?;
    item_explorer::browse_item_proto(&pool, query.as_deref(), type_filter, offset, limit).await
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

// ---- Item-Beschreibung (locale/<lang>/itemdesc.txt - client-seitig, keine
// item_proto-Spalte) ----

#[tauri::command]
pub fn get_item_desc(state: State<'_, AppState>, vnum: u32) -> Result<Option<itemdesc::ItemDescEntry>, String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    itemdesc::read_entry(&client_path, vnum)
}

#[tauri::command]
pub fn write_item_desc(
    state: State<'_, AppState>,
    vnum: u32,
    description: String,
    summary: String,
) -> Result<(), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    itemdesc::write_entry(&client_path, vnum, &description, &summary)
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

/// Scan for the icon-only importer (accessories with no 3D model at all -
/// shoes, necklaces, shields, earrings, bracelets, ...): just every image
/// file under the folder, no weapon/armor detection.
#[tauri::command]
pub fn scan_icon_folder(path: String) -> Result<Vec<String>, String> {
    modulescan::scan_icon_folder(std::path::Path::new(&path))
}

#[tauri::command]
pub fn import_weapon_model(
    state: State<'_, AppState>,
    module_name: String,
    vnum: u32,
    source_abs: String,
    texture_sources: Vec<String>,
) -> Result<(String, String), String> {
    let client_path = item_editor_setting(&state, "client_path")?;
    let (dest, virtual_path) = packtools::import_custom_weapon_model(
        &client_path,
        &module_name,
        vnum,
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
    // Fetched *before* the delete below so the item's own refine linkage
    // is still readable - used afterward to also drop its recipe if this
    // was the last item using it (e.g. undoing a Module-Importer refine
    // chain, see refine.rs's module doc). A recipe still shared by other
    // items (`delete_recipe` errors on that) is deliberately left alone;
    // the error is swallowed since that's the expected/correct outcome,
    // not a failure of this teardown.
    let refine_set: Option<u16> = sqlx::query("SELECT refine_set FROM player.item_proto WHERE vnum = ?")
        .bind(vnum)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .map(|row| sqlx::Row::try_get(&row, "refine_set").unwrap_or_default());

    item::delete_item_proto(pool, vnum).await?;
    packtools::remove_item_list_entries(client_path, vnum)?;
    packtools::delete_item_icon(client_path, vnum)?;

    if let Some(refine_set) = refine_set {
        if refine_set != 0 {
            let _ = refine::delete_recipe(pool, refine_set as i32).await;
        }
    }
    Ok(())
}

/// Rolls back one item created earlier in an import batch that then failed
/// (e.g. the repack step erroring out after several items were already
/// created). Deliberately skips `pack_item_icons`/`regenerate_item_proto` -
/// unlike `remove_single_item`, this runs *from inside* a failure handler,
/// so repacking again here would only add another risky operation on top
/// of whatever already went wrong; the next successful import's own repack
/// step naturally cleans up the icon.epk/item_proto inconsistency this
/// leaves behind in the meantime (harmless: nothing references vnum
/// without a DB row or item_list.txt entry, both removed here).
#[tauri::command]
pub async fn rollback_created_item(state: State<'_, AppState>, vnum: u32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    let client_path = item_editor_setting(&state, "client_path")?;
    teardown_item(&pool, &client_path, vnum).await
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

// ---- Refine-Editor (Aufwertungs-Ketten) ----

#[tauri::command]
pub async fn get_refine_chain(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Vec<refine::RefineChainNode>, String> {
    let pool = require_pool(&state).await?;
    refine::get_refine_chain(&pool, vnum).await
}

#[tauri::command]
pub async fn get_refine_recipe(
    state: State<'_, AppState>,
    id: i32,
) -> Result<Option<refine::RefineRecipe>, String> {
    let pool = require_pool(&state).await?;
    refine::get_recipe(&pool, id).await
}

#[tauri::command]
pub async fn save_refine_recipe(
    state: State<'_, AppState>,
    id: Option<i32>,
    cost: i32,
    prob: i16,
    materials: Vec<refine::RefineMaterial>,
) -> Result<i32, String> {
    let pool = require_pool(&state).await?;
    refine::save_recipe(&pool, id, cost, prob, &materials).await
}

#[tauri::command]
pub async fn delete_refine_recipe(state: State<'_, AppState>, id: i32) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    refine::delete_recipe(&pool, id).await
}

#[tauri::command]
pub async fn set_item_refine_link(
    state: State<'_, AppState>,
    vnum: u32,
    refine_set: u16,
    refined_vnum: u32,
) -> Result<(), String> {
    let pool = require_pool(&state).await?;
    refine::set_item_refine_link(&pool, vnum, refine_set, refined_vnum).await
}

#[tauri::command]
pub async fn find_refine_shop_sources(
    state: State<'_, AppState>,
    vnum: u32,
) -> Result<Vec<refine::ShopSource>, String> {
    let pool = require_pool(&state).await?;
    refine::find_shop_sources(&pool, vnum).await
}

// ---- System-Installer ----
//
// Baut fertige Community-"Systeme" (Server-/Client-Erweiterungen wie
// ResizeWindow oder ein Admin-Panel-Modul) automatisiert ein - siehe
// system_patch.rs für die verifizierte Paket-Konvention (search/add-
// Marker) und system_installs.rs für die Zielort-Entscheidung
// (Server-Quellcode live über SSH wie jedes andere Server-Datei-Werkzeug
// hier, Client-Quellcode lokal im binary_src_path-Checkout, Client-
// Installationsdateien im bestehenden client_path). Eigenständiges Modul,
// leicht wieder entfernbar: nichts Bestehendes wird hier verändert außer
// diesem einen Bereich.

fn binary_src_path_setting(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, "binary_src_path")?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "Kein lokaler Client-Quellcode-Pfad (binary_src_path) konfiguriert.".to_string())
}

async fn read_target_content(
    state: &State<'_, AppState>,
    category: TargetKind,
    path: &str,
    shared_sftp: Option<&SftpSession>,
) -> Result<Option<String>, String> {
    match category {
        TargetKind::LiveServer => {
            if let Some(sftp) = shared_sftp {
                ssh::read_file_if_exists_via(sftp, path).await
            } else {
                let (config, auth) = stored_ssh_auth(state)?;
                ssh::read_remote_file_if_exists(&config, &auth, path).await
            }
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            let p = std::path::Path::new(path);
            if !p.exists() {
                return Ok(None);
            }
            if p.is_dir() {
                return Err(format!(
                    "\"{path}\" ist ein Ordner, keine Datei - bitte den vollständigen Pfad zur Zieldatei angeben (inkl. Dateiname)."
                ));
            }
            // Reale Client-Quellcode-/Installationsdateien sind wie die
            // Quest-Dateien auf dem Server oft nicht UTF-8 sondern
            // Windows-1252 (z.B. deutsche Umlaute in Kommentaren) - `
            // read_to_string` bricht dabei hart ab ("stream did not contain
            // valid UTF-8"), reale Live-Meldung eines Nutzers. Nutzt
            // denselben Fallback wie die SSH-Seite.
            let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
            Ok(Some(ssh::decode_bytes(bytes)))
        }
    }
}

async fn write_target_with_backup(
    state: &State<'_, AppState>,
    category: TargetKind,
    path: &str,
    content: &str,
    shared_sftp: Option<&SftpSession>,
) -> Result<Option<String>, String> {
    match category {
        TargetKind::LiveServer => {
            if let Some(sftp) = shared_sftp {
                ssh::write_file_with_backup_via(sftp, path, content).await
            } else {
                let (config, auth) = stored_ssh_auth(state)?;
                ssh::write_remote_file_with_backup(&config, &auth, path, content).await
            }
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            let p = std::path::Path::new(path);
            // Read before backup_file() renames the file away, same
            // rationale as write_remote_file_with_backup - a local client
            // source/install file can be Windows-1252 (see ssh::encode_matching)
            // just as easily as a remote one.
            let original_bytes = std::fs::read(p).ok();
            let backup = packtools::backup_file(p)?;
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(p, ssh::encode_matching(content, original_bytes.as_deref()))
                .map_err(|e| e.to_string())?;
            Ok(backup.map(|b| b.display().to_string()))
        }
    }
}

async fn delete_target(state: &State<'_, AppState>, category: TargetKind, path: &str) -> Result<(), String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(state)?;
            ssh::delete_remote_file(&config, &auth, path).await
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            let p = std::path::Path::new(path);
            if p.exists() {
                std::fs::remove_file(p).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
    }
}

async fn restore_from_backup(
    state: &State<'_, AppState>,
    category: TargetKind,
    backup_path: &str,
    target_path: &str,
) -> Result<(), String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(state)?;
            // Byte-exact, same reasoning as restore_remote_backup above -
            // this is a straight "put the backup back" operation, no text
            // interpretation needed or wanted.
            let content = ssh::read_remote_file_bytes(&config, &auth, backup_path).await?;
            ssh::write_remote_file_bytes_with_backup(&config, &auth, target_path, &content).await?;
            Ok(())
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            std::fs::copy(backup_path, target_path).map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

/// Liest ein lokal ausgewähltes Systempaket ein und klassifiziert/parst
/// jede enthaltene Datei - reines Dateisystem-Lesen, kein SSH nötig.
#[tauri::command]
pub fn scan_system_package(root: String) -> Result<Vec<ScannedFile>, String> {
    system_scan::scan_system_package(std::path::Path::new(&root))
}

/// Sucht die echte Zieldatei zu einem im Systempaket vorkommenden
/// Dateinamen - Server live über SSH, Client-Quellcode/-Installation lokal.
/// Liefert alle Treffer, der Aufrufer entscheidet bei Mehrdeutigkeit.
#[tauri::command]
pub async fn find_system_target(
    state: State<'_, AppState>,
    category: TargetKind,
    filename: String,
) -> Result<Vec<String>, String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(&state)?;
            let root = build_deploy_setting(&state, "build_live_source_root", "/usr/home/source/server")?;
            ssh::find_remote_file_by_name(&config, &auth, &root, &filename).await
        }
        TargetKind::LocalClientSource => {
            let root = binary_src_path_setting(&state)?;
            Ok(system_scan::find_local_file_by_name(std::path::Path::new(&root), &filename))
        }
        TargetKind::LocalClientInstall => {
            let root = client_path_setting(&state)?;
            Ok(system_scan::find_local_file_by_name(std::path::Path::new(&root), &filename))
        }
    }
}

/// Wie `find_system_target`, aber löst viele Dateinamen einer Kategorie in
/// einem Rutsch auf (ein Verzeichnis-Durchlauf bzw. ein `find`-Aufruf statt
/// einer pro Datei) - der eigentliche Grund für den Befehl: `client_path`
/// zeigt real oft auf einen kompletten Client-Ordner mit mehreren
/// zehntausend Dateien, ein Systempaket mit einem Dutzend Client-Dateien
/// hätte diesen Baum sonst ein Dutzend Mal komplett durchsucht. Wird beim
/// initialen Scannen eines Systempakets genutzt, `find_system_target`
/// bleibt für die gezielte Einzelsuche ("Erneut suchen").
#[tauri::command]
pub async fn find_system_targets_batch(
    state: State<'_, AppState>,
    category: TargetKind,
    filenames: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(&state)?;
            let root = build_deploy_setting(&state, "build_live_source_root", "/usr/home/source/server")?;
            ssh::find_remote_files_by_names(&config, &auth, &root, &filenames).await
        }
        TargetKind::LocalClientSource => {
            let root = binary_src_path_setting(&state)?;
            let wanted: std::collections::HashSet<String> = filenames.into_iter().collect();
            Ok(system_scan::find_local_files_by_names(std::path::Path::new(&root), &wanted))
        }
        TargetKind::LocalClientInstall => {
            let root = client_path_setting(&state)?;
            let wanted: std::collections::HashSet<String> = filenames.into_iter().collect();
            Ok(system_scan::find_local_files_by_names(std::path::Path::new(&root), &wanted))
        }
    }
}

/// Liest den aktuellen Inhalt eines bestätigten Zielpfads - Grundlage für
/// die Anker-Suche/Vorschau, bevor irgendetwas geschrieben wird.
#[tauri::command]
pub async fn read_system_target_file(
    state: State<'_, AppState>,
    category: TargetKind,
    path: String,
) -> Result<Option<String>, String> {
    read_target_content(&state, category, &path, None).await
}

/// Reine Anker-/Einfüge-Auflösung (keine Datei-I/O) - lässt das Frontend
/// live in der Vorschau anzeigen, ob/wo ein Block automatisch übernommen
/// werden könnte, ohne dafür extra einen SSH-Roundtrip zu brauchen (der
/// Inhalt wurde vorher schon einmal per `read_system_target_file` geholt).
#[tauri::command]
pub fn resolve_system_insertion(
    haystack: String,
    scope: Option<String>,
    anchor: String,
    placement: Placement,
) -> InsertionResolution {
    system_patch::resolve_insertion(&haystack, scope.as_deref(), &anchor, placement)
}

#[derive(serde::Deserialize)]
pub struct PlannedFile {
    pub target_path: String,
    pub category: TargetKind,
    /// Nur bereits vom Nutzer bestätigte `SearchInsert`/`AppendToEnd`-Blöcke
    /// - `FreeformInstruction` oder ein Block mit unsicherer Anker-Auflösung
    /// gehört hier nicht rein, das bleibt in der UI zur manuellen Prüfung.
    pub ops: Vec<PatchOp>,
}

#[derive(serde::Serialize)]
pub struct ApplyInstallResult {
    pub install_id: i64,
    pub warnings: Vec<String>,
}

async fn apply_one_file(
    state: &State<'_, AppState>,
    file: &PlannedFile,
    shared_sftp: Option<&SftpSession>,
) -> Result<(InstalledFile, Option<String>), String> {
    let existing = read_target_content(state, file.category, &file.target_path, shared_sftp).await?;
    let existed = existing.is_some();
    let mut content = existing.unwrap_or_default();

    for op in &file.ops {
        match op {
            PatchOp::AppendToEnd { code } => {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(code);
                content.push('\n');
            }
            PatchOp::SearchInsert { scope, anchor, placement, code } => {
                match system_patch::resolve_insertion(&content, scope.as_deref(), anchor, *placement) {
                    InsertionResolution::Ready { line, .. } => {
                        content = system_patch::splice_lines(&content, line, code);
                    }
                    InsertionResolution::ReadyReplace { start_line, end_line, .. } => {
                        content = system_patch::replace_lines(&content, start_line, end_line, code);
                    }
                    InsertionResolution::NeedsReview { reason } => {
                        return Err(format!("{}: {reason} - bitte manuell prüfen.", file.target_path));
                    }
                }
            }
            PatchOp::FreeformInstruction { .. } => {
                return Err(format!(
                    "{}: Freitext-Block kann nicht automatisch angewendet werden.",
                    file.target_path
                ));
            }
        }
    }

    let warning = system_patch::check_structural_balance(&file.target_path, &content)
        .map(|w| format!("{}: {w}", file.target_path));

    let backup_path =
        write_target_with_backup(state, file.category, &file.target_path, &content, shared_sftp).await?;
    let installed = InstalledFile {
        target_path: file.target_path.clone(),
        target_kind: file.category,
        backup_path,
        action: if existed { FileAction::Patched } else { FileAction::Created },
    };
    Ok((installed, warning))
}

/// Schreibt alle übergebenen (bereits bestätigten) Änderungen in einem
/// Rutsch, mit Backup vor jedem Schreiben, und legt einen Verlaufs-Eintrag
/// an - die Grundlage für "Rückgängig machen". Bricht bei der ersten Datei
/// ab, die doch nicht automatisch anwendbar ist (sollte praktisch nicht
/// vorkommen, da das Frontend vorher nur geprüfte Blöcke schickt) - alles,
/// was bis dahin schon erfolgreich geschrieben wurde, landet trotzdem im
/// Verlauf, damit dafür kein Rückgängig-machen fehlt.
#[tauri::command]
pub async fn apply_system_install(
    state: State<'_, AppState>,
    system_name: String,
    files: Vec<PlannedFile>,
) -> Result<ApplyInstallResult, String> {
    // One shared SFTP session for every LiveServer file in this run instead
    // of a fresh SSH connect+login per file - real live report (2026-08-11):
    // a 10-server-file package meant 20 separate connect+login round-trips
    // (read+write each), and a single stalled one among those froze the
    // entire "Anwenden" step forever with no error shown (see the SSH
    // timeout added in ssh.rs for the other half of that fix). Only opened
    // if at least one file actually needs it - a purely client-side package
    // shouldn't require a live SSH connection at all.
    let shared_sftp = if files.iter().any(|f| f.category == TargetKind::LiveServer) {
        let (config, auth) = stored_ssh_auth(&state)?;
        Some(ssh::open_sftp(&config, &auth).await?)
    } else {
        None
    };

    let mut installed_files = Vec::new();
    let mut warnings = Vec::new();
    let mut first_error: Option<String> = None;

    for file in &files {
        match apply_one_file(&state, file, shared_sftp.as_ref()).await {
            Ok((installed, warning)) => {
                installed_files.push(installed);
                if let Some(w) = warning {
                    warnings.push(w);
                }
            }
            Err(e) => {
                first_error = Some(e);
                break;
            }
        }
    }

    let install_id = if !installed_files.is_empty() {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        Some(system_installs::record_install(&conn, &system_name, &installed_files)?)
    } else {
        None
    };

    if let Some(err) = first_error {
        return Err(match install_id {
            Some(id) => format!(
                "{err} (bereits geschriebene Dateien wurden unter Verlauf-Eintrag #{id} gesichert - dort ggf. rückgängig machen)"
            ),
            None => err,
        });
    }

    Ok(ApplyInstallResult { install_id: install_id.unwrap_or(0), warnings })
}

#[tauri::command]
pub fn list_system_installs(state: State<'_, AppState>) -> Result<Vec<system_installs::SystemInstall>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    system_installs::list_installs(&conn)
}

/// Stellt jede Datei eines Verlaufs-Eintrags aus ihrem Backup wieder her
/// (bzw. löscht neu angelegte Dateien) und entfernt danach den Eintrag -
/// Vorbild `undo_import_batch`.
#[tauri::command]
pub async fn undo_system_install(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let install = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        system_installs::get_install(&conn, id)?
            .ok_or_else(|| format!("System-Installation {id} nicht gefunden (bereits entfernt?)"))?
    };

    for file in &install.files {
        match file.action {
            FileAction::Created => {
                delete_target(&state, file.target_kind, &file.target_path).await?;
            }
            FileAction::Patched => {
                let backup_path = file.backup_path.as_ref().ok_or_else(|| {
                    format!("Kein Backup für {} hinterlegt - manuelles Wiederherstellen nötig.", file.target_path)
                })?;
                restore_from_backup(&state, file.target_kind, backup_path, &file.target_path).await?;
            }
        }
    }

    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    system_installs::delete_install_record(&conn, id)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Regression test for a real finding: read_quest_file/write_quest_file/
    // delete_quest_file/read_regen_file/write_regen_file all joined a
    // caller-supplied relative_path directly onto a trusted base dir with no
    // validation - a "../" segment could escape the intended quest/regen
    // directory entirely.
    #[test]
    fn safe_relative_path_rejects_traversal_and_absolute_paths() {
        assert!(safe_relative_path("Biologie/Biochecker.lua").is_ok());
        assert!(safe_relative_path("../../../etc/passwd").is_err());
        assert!(safe_relative_path("Biologie/../../../etc/passwd").is_err());
        assert!(safe_relative_path("/etc/passwd").is_err());
        assert!(safe_relative_path("").is_err());
        assert!(safe_relative_path("Biologie//Biochecker.lua").is_err());
    }

    async fn real_pool() -> sqlx::MySqlPool {
        let conn = rusqlite::Connection::open(
            r"C:\Users\DevSteven\AppData\Roaming\com.m2manager.app\m2manager.sqlite",
        )
        .expect("open settings db");
        let get = |key: &str| -> Option<String> {
            conn.query_row("SELECT value FROM paths WHERE key = ?1", [key], |r| r.get(0))
                .ok()
        };
        let host = get("mysql_host").expect("mysql_host not configured on this machine");
        let port: u16 = get("mysql_port").unwrap_or_else(|| "3306".into()).parse().unwrap();
        let user = get("mysql_username").expect("mysql_username not configured");
        let password = keyring::Entry::new("m2manager", "mysql_password")
            .unwrap()
            .get_password()
            .expect("mysql_password credential not stored");
        let url = format!("mysql://{user}:{password}@{host}:{port}/player");
        sqlx::mysql::MySqlPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("connect to dev DB")
    }

    // Regression test for the Module Importer's refine-chain feature
    // (2026-08-06): undoing an item that owned its own private (not
    // shared) refine recipe must also drop that now-unused recipe, not
    // just the item - otherwise every rolled-back or undone chain leaves
    // orphaned refine_proto rows behind forever. A recipe still used by
    // *another* item must survive the same teardown untouched.
    #[tokio::test]
    async fn teardown_item_drops_its_own_unshared_recipe_but_not_a_shared_one() {
        let pool = real_pool().await;
        let client_path = r"C:\Users\DevSteven\Desktop\Client";

        // High throwaway vnums, well outside any real item range on this
        // server (verified in earlier sessions to top out well under
        // 900000) - safe test fixture that can't collide with real data.
        let owner_vnum = 900001u32;
        let sharer_vnum = 900002u32;
        for v in [owner_vnum, sharer_vnum] {
            let _ = item::delete_item_proto(&pool, v).await;
        }

        let recipe_id = refine::save_recipe(&pool, None, 1000, 90, &[]).await.expect("create recipe");

        let mut item = crate::db::item::ItemProtoInput {
            vnum: owner_vnum,
            vnum_range: 0,
            name: "test_teardown_owner".into(),
            locale_name: "TeardownTestOwner".into(),
            r#type: 1,
            subtype: 0,
            weight: 0,
            size: 1,
            antiflag: 0,
            flag: 0,
            wearflag: 0,
            immuneflag: 0,
            gold: 0,
            shop_buy_price: 0,
            refined_vnum: 0,
            refine_set: recipe_id as u16,
            magic_pct: 0,
            limittype0: 0,
            limitvalue0: 0,
            limittype1: 0,
            limitvalue1: 0,
            applytype0: 0,
            applyvalue0: 0,
            applytype1: 0,
            applyvalue1: 0,
            applytype2: 0,
            applyvalue2: 0,
            applytype3: 0,
            applyvalue3: 0,
            value0: 0,
            value1: 0,
            value2: 0,
            value3: 0,
            value4: 0,
            value5: 0,
            socket0: 0,
            socket1: 0,
            socket2: 0,
            socket3: 0,
            socket4: 0,
            socket5: 0,
            specular: 0,
            socket_pct: 0,
            addon_type: 0,
        };
        item::create_item_proto(&pool, &item).await.expect("create owner item");
        item.vnum = sharer_vnum;
        item.name = "test_teardown_sharer".into();
        item::create_item_proto(&pool, &item).await.expect("create sharer item");

        teardown_item(&pool, client_path, owner_vnum).await.expect("teardown owner");

        assert!(
            !item::vnum_exists(&pool, owner_vnum).await.unwrap(),
            "owner item must be gone"
        );
        assert!(
            item::vnum_exists(&pool, sharer_vnum).await.unwrap(),
            "sharer item must be untouched"
        );
        let recipe_after = refine::get_recipe(&pool, recipe_id).await.unwrap();
        assert!(
            recipe_after.is_some(),
            "recipe must survive - still used by the sharer item"
        );

        teardown_item(&pool, client_path, sharer_vnum).await.expect("teardown sharer");
        assert!(!item::vnum_exists(&pool, sharer_vnum).await.unwrap());
        let recipe_gone = refine::get_recipe(&pool, recipe_id).await.unwrap();
        assert!(
            recipe_gone.is_none(),
            "recipe must be dropped once the last item using it is gone"
        );
    }
}

