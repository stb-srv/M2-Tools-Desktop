//! Server-Control: SSH connectivity, source build + deploy/rollback,
//! resource monitoring.

use super::support::{build_deploy_setting, stored_ssh_auth};
use crate::build_deploy;
use crate::deploy_history;
use crate::resources;
use crate::settings;
use crate::ssh::{self, SshAuth, SshConfig};
use crate::state::AppState;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn test_ssh_connection(config: SshConfig, auth: SshAuth) -> Result<(), String> {
    ssh::test_connection(&config, &auth).await
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
