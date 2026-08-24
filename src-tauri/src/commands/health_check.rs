//! Config-Health-Check (Idee #2 der Session vom 2026-08-24, siehe
//! `C:\Users\DevSteven\.claude\plans\tranquil-roaming-pearl.md`). Reine
//! Orchestrierung bereits bestehender Bausteine - kein neues Non-Command-
//! Modul nötig: SSH/MySQL-Konnektivität + alle konfigurierten lokalen/
//! entfernten Pfade in einem Rutsch prüfen, statt dass ein falscher/fehlender
//! Pfad erst beim eigentlichen Nutzen eines Moduls als stiller Fehlschlag
//! auffällt (der Grundmuster-Bug hinter mehreren früheren echten Funden in
//! diesem Projekt, siehe System-Installer-Historie).

use super::support::stored_ssh_auth;
use crate::credentials;
use crate::db::mysql::{self, MysqlConfig};
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use serde::Serialize;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
pub struct HealthCheckResult {
    pub label: String,
    pub kind: String,
    pub status: String,
    pub detail: String,
}

fn result(label: &str, kind: &str, status: &str, detail: impl Into<String>) -> HealthCheckResult {
    HealthCheckResult {
        label: label.to_string(),
        kind: kind.to_string(),
        status: status.to_string(),
        detail: detail.into(),
    }
}

fn read_setting(conn: &rusqlite::Connection, key: &str) -> Option<String> {
    settings::get_path(conn, key)
        .ok()
        .flatten()
        .filter(|v| !v.is_empty())
}

const LOCAL_PATH_KEYS: &[(&str, &str)] = &[
    ("client_path", "Client-Ordner"),
    ("binary_src_path", "Client-Quellcode-Ordner"),
    ("npclist_path", "NPC-Liste"),
    ("eterpack_tool_path", "EterPackConsoleLz4.exe"),
    ("mysql2proto_dir", "Mysql2Proto-Ordner"),
];

const REMOTE_PATH_KEYS: &[(&str, &str)] = &[
    ("mob_drop_file_path", "mob_drop_item.txt"),
    ("common_drop_file_path", "common_drop_item.txt"),
    ("etc_drop_file_path", "etc_drop_item.txt"),
    ("drop_item_group_file_path", "drop_item_group.txt"),
    ("special_item_group_file_path", "special_item_group.txt"),
    ("cube_file_path", "cube.txt"),
    ("quest_dir", "Quest-Ordner"),
    ("regen_base_dir", "Regen-Datei-Ordner"),
    ("locale_file_path", "translate.lua"),
    ("build_live_source_root", "Server-Quellcode (Live)"),
    ("build_live_game_binary", "game-Programmdatei"),
    ("build_live_db_binary", "db-Programmdatei"),
    ("db_backup_dir", "DB-Backup-Ordner"),
    ("server_disk_path", "Festplatten-Prüfpfad"),
];

async fn check_ssh(state: &State<'_, AppState>) -> (HealthCheckResult, bool) {
    match stored_ssh_auth(state) {
        Err(e) => (result("SSH-Verbindung", "connection", "skipped", e), false),
        Ok((config, auth)) => match ssh::test_connection(&config, &auth).await {
            Ok(()) => (
                result(
                    "SSH-Verbindung",
                    "connection",
                    "ok",
                    format!("Verbunden mit {}@{}:{}", config.username, config.host, config.port),
                ),
                true,
            ),
            Err(e) => (result("SSH-Verbindung", "connection", "error", e), false),
        },
    }
}

async fn check_mysql(state: &State<'_, AppState>) -> HealthCheckResult {
    let (host, port, username) = {
        let conn = match state.settings_db.lock() {
            Ok(c) => c,
            Err(e) => return result("MySQL-Verbindung", "connection", "error", e.to_string()),
        };
        (
            read_setting(&conn, "mysql_host"),
            read_setting(&conn, "mysql_port"),
            read_setting(&conn, "mysql_username"),
        )
    };
    let (Some(host), Some(username)) = (host, username) else {
        return result("MySQL-Verbindung", "connection", "skipped", "Keine MySQL-Verbindung konfiguriert.");
    };
    let password = match credentials::get_secret("mysql_password") {
        Ok(p) => p,
        Err(_) => {
            return result(
                "MySQL-Verbindung",
                "connection",
                "skipped",
                "Kein gespeichertes MySQL-Passwort gefunden.",
            )
        }
    };
    let config = MysqlConfig {
        host,
        port: port.and_then(|p| p.parse().ok()).unwrap_or(3306),
        username,
        database: None,
    };
    match mysql::test_connection(&config, &password).await {
        Ok(()) => result(
            "MySQL-Verbindung",
            "connection",
            "ok",
            format!("Verbunden mit {}:{}", config.host, config.port),
        ),
        Err(e) => result("MySQL-Verbindung", "connection", "error", e),
    }
}

fn read_path_values(conn: &rusqlite::Connection, keys: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str, Option<String>)> {
    keys.iter().map(|(key, label)| (*key, *label, read_setting(conn, key))).collect()
}

fn check_local_path_values(values: &[(&'static str, &'static str, Option<String>)]) -> Vec<HealthCheckResult> {
    values
        .iter()
        .map(|(key, label, value)| match value {
            None => result(label, "local-path", "skipped", format!("Einstellung '{key}' ist nicht gesetzt.")),
            Some(path) => {
                if std::fs::metadata(path).is_ok() {
                    result(label, "local-path", "ok", path.clone())
                } else {
                    result(label, "local-path", "error", format!("Pfad nicht gefunden: {path}"))
                }
            }
        })
        .collect()
}

async fn check_remote_path_values(state: &State<'_, AppState>, ssh_ok: bool, values: Vec<(&'static str, &'static str, Option<String>)>) -> Vec<HealthCheckResult> {
    if !ssh_ok {
        return values
            .into_iter()
            .map(|(key, label, value)| match value {
                None => result(label, "remote-path", "skipped", format!("Einstellung '{key}' ist nicht gesetzt.")),
                Some(_) => result(
                    label,
                    "remote-path",
                    "skipped",
                    "SSH-Verbindung nicht erreichbar - konnte nicht geprüft werden.",
                ),
            })
            .collect();
    }

    let (config, auth) = match stored_ssh_auth(state) {
        Ok(v) => v,
        Err(e) => {
            return values
                .into_iter()
                .map(|(key, label, value)| match value {
                    None => result(label, "remote-path", "skipped", format!("Einstellung '{key}' ist nicht gesetzt.")),
                    Some(_) => result(label, "remote-path", "warning", e.clone()),
                })
                .collect()
        }
    };

    match ssh::open_sftp(&config, &auth).await {
        Ok(sftp) => {
            let mut out = Vec::with_capacity(values.len());
            for (key, label, value) in values {
                out.push(match value {
                    None => result(label, "remote-path", "skipped", format!("Einstellung '{key}' ist nicht gesetzt.")),
                    Some(path) => match sftp.try_exists(&path).await {
                        Ok(true) => result(label, "remote-path", "ok", path),
                        Ok(false) => result(label, "remote-path", "error", format!("Pfad nicht gefunden: {path}")),
                        Err(e) => result(label, "remote-path", "error", e.to_string()),
                    },
                });
            }
            out
        }
        Err(e) => values
            .into_iter()
            .map(|(key, label, value)| match value {
                None => result(label, "remote-path", "skipped", format!("Einstellung '{key}' ist nicht gesetzt.")),
                Some(_) => result(label, "remote-path", "warning", format!("SFTP-Sitzung konnte nicht geöffnet werden: {e}")),
            })
            .collect(),
    }
}

#[tauri::command]
pub async fn run_health_check(state: State<'_, AppState>) -> Result<Vec<HealthCheckResult>, String> {
    let mut results = Vec::new();

    let (ssh_result, ssh_ok) = check_ssh(&state).await;
    results.push(ssh_result);
    results.push(check_mysql(&state).await);

    let (local_values, remote_values) = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        (
            read_path_values(&conn, LOCAL_PATH_KEYS),
            read_path_values(&conn, REMOTE_PATH_KEYS),
        )
    };

    results.extend(check_local_path_values(&local_values));
    results.extend(check_remote_path_values(&state, ssh_ok, remote_values).await);

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_path_check_reports_ok_for_existing_path() {
        let dir = std::env::temp_dir();
        let values = vec![("client_path", "Client-Ordner", Some(dir.to_string_lossy().to_string()))];
        let results = check_local_path_values(&values);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "ok");
    }

    #[test]
    fn local_path_check_reports_error_for_missing_path() {
        let values = vec![(
            "client_path",
            "Client-Ordner",
            Some("Z:\\definitely\\not\\a\\real\\path\\m2manager_test".to_string()),
        )];
        let results = check_local_path_values(&values);
        assert_eq!(results[0].status, "error");
    }

    #[test]
    fn local_path_check_reports_skipped_for_unset_setting() {
        let values = vec![("client_path", "Client-Ordner", None)];
        let results = check_local_path_values(&values);
        assert_eq!(results[0].status, "skipped");
    }
}
