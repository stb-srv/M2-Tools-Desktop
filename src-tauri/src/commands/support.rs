//! Private helpers shared by 2+ domain submodules below. Anything used by
//! only one submodule stays local to that file instead of landing here.

use crate::credentials;
use crate::settings;
use crate::ssh::{SshAuth, SshConfig};
use crate::state::AppState;
use crate::webhook;
use tauri::State;

/// Reassembles the SSH connection from what the setup wizard stored: metadata
/// in the local settings DB, secrets in the Windows Credential Manager.
pub(super) fn stored_ssh_auth(state: &State<'_, AppState>) -> Result<(SshConfig, SshAuth), String> {
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

pub(super) async fn require_pool(state: &State<'_, AppState>) -> Result<sqlx::MySqlPool, String> {
    state
        .mysql_pool
        .lock()
        .await
        .clone()
        .ok_or_else(|| "Keine aktive MySQL-Verbindung. Bitte zuerst verbinden.".to_string())
}

pub(super) fn client_path_setting(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, "client_path")?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "Kein Client-Pfad konfiguriert.".to_string())
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
pub(super) fn safe_relative_path(relative_path: &str) -> Result<&str, String> {
    if relative_path.is_empty()
        || relative_path.starts_with('/')
        || relative_path.split('/').any(|seg| seg.is_empty() || seg == "..")
    {
        return Err(format!("Ungültiger relativer Pfad: \"{relative_path}\"."));
    }
    Ok(relative_path)
}

// ---- Server-Quellcode Bauen & Einspielen ----
//
// Baut nie direkt im Live-Quellbaum, immer in einer separaten Arbeitskopie
// (siehe build_deploy.rs) - deshalb sind diese beiden Commands unabhängig
// vom riskanten Einspiel-Schritt (deploy_history.rs) gefahrlos nutzbar.
pub(super) fn build_deploy_setting(state: &State<'_, AppState>, key: &str, default: &str) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, key)?
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string()))
}

// ---- Item Editor: gemeinsame Pfad-Einstellung, auch vom Modul-Importer
// gebraucht (Waffen/Rüstungs-Import legt neue Items an, ohne den Item Editor
// selbst zu öffnen). ----
pub(super) fn item_editor_setting(state: &State<'_, AppState>, key: &str) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, key)?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| format!("Einstellung '{key}' ist nicht gesetzt. Bitte unter Einstellungen konfigurieren."))
}

// ---- Webhook-Benachrichtigungen ----
pub(super) fn webhook_url(state: &State<'_, AppState>) -> Result<Option<String>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, "webhook_url")?.filter(|v| !v.is_empty()))
}

pub(super) async fn notify_webhook_best_effort(state: &State<'_, AppState>, message: &str) {
    if let Ok(Some(url)) = webhook_url(state) {
        let _ = webhook::send_webhook_message(&url, message).await;
    }
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
}
