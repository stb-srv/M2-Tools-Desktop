//! Bekannte SSH-Host-Keys (Trust on first use).
//!
//! Pro `host:port` wird der SHA256-Fingerprint des Server-Schlüssels
//! (OpenSSH-Format `SHA256:...`) in `known_hosts.json` im App-Datenordner
//! gespeichert. Beim ersten Kontakt wird der Schlüssel gemerkt, bei jeder
//! späteren Verbindung muss er übereinstimmen.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnownHost {
    pub host: String,
    pub port: u16,
    pub fingerprint: String,
    pub first_seen: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    Known,
    New,
    Mismatch { stored: String },
}

struct Store {
    path: Option<PathBuf>,
    entries: Vec<KnownHost>,
}

static STORE: OnceLock<Mutex<Store>> = OnceLock::new();
static APP: OnceLock<tauri::AppHandle> = OnceLock::new();

fn store() -> &'static Mutex<Store> {
    STORE.get_or_init(|| Mutex::new(Store { path: None, entries: Vec::new() }))
}

fn norm_host(host: &str) -> String {
    host.trim().to_lowercase()
}

/// Reine Entscheidungslogik (ohne I/O) - trägt bei `New` den Eintrag ein.
pub fn evaluate(entries: &mut Vec<KnownHost>, host: &str, port: u16, fingerprint: &str, now: &str) -> Decision {
    let host = norm_host(host);
    match entries.iter().find(|e| e.host == host && e.port == port) {
        Some(e) if e.fingerprint == fingerprint => Decision::Known,
        Some(e) => Decision::Mismatch { stored: e.fingerprint.clone() },
        None => {
            entries.push(KnownHost { host, port, fingerprint: fingerprint.to_string(), first_seen: now.to_string() });
            Decision::New
        }
    }
}

/// Beim App-Start aufrufen: Speicherort setzen und Datei laden.
pub fn init(app_data_dir: &std::path::Path, app: tauri::AppHandle) {
    let path = app_data_dir.join("known_hosts.json");
    let entries = std::fs::read_to_string(&path).ok().and_then(|s| serde_json::from_str::<Vec<KnownHost>>(&s).ok()).unwrap_or_default();
    if let Ok(mut s) = store().lock() {
        s.path = Some(path);
        s.entries = entries;
    }
    let _ = APP.set(app);
}

fn persist(s: &Store) {
    if let Some(path) = &s.path {
        if let Ok(json) = serde_json::to_string_pretty(&s.entries) {
            let _ = std::fs::write(path, json);
        }
    }
}

/// Prüft/merkt den Fingerprint. Bei `New` wird gespeichert und die UI per
/// Event `ssh-host-key-new` benachrichtigt.
pub fn check(host: &str, port: u16, fingerprint: &str) -> Decision {
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let decision = {
        let Ok(mut s) = store().lock() else { return Decision::Known };
        let d = evaluate(&mut s.entries, host, port, fingerprint, &now);
        if d == Decision::New {
            persist(&s);
        }
        d
    };
    if decision == Decision::New {
        if let Some(app) = APP.get() {
            let _ = app
                .emit("ssh-host-key-new", KnownHost { host: norm_host(host), port, fingerprint: fingerprint.to_string(), first_seen: now });
        }
    }
    decision
}

pub fn list() -> Vec<KnownHost> {
    store().lock().map(|s| s.entries.clone()).unwrap_or_default()
}

pub fn forget(host: &str, port: u16) -> bool {
    let host = norm_host(host);
    let Ok(mut s) = store().lock() else { return false };
    let before = s.entries.len();
    s.entries.retain(|e| !(e.host == host && e.port == port));
    let removed = s.entries.len() != before;
    if removed {
        persist(&s);
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_use_is_remembered_then_known() {
        let mut v = Vec::new();
        assert_eq!(evaluate(&mut v, "Example.com", 22, "SHA256:a", "t"), Decision::New);
        assert_eq!(v.len(), 1);
        assert_eq!(evaluate(&mut v, "example.com", 22, "SHA256:a", "t"), Decision::Known);
        assert_eq!(v.len(), 1);
    }

    #[test]
    fn changed_key_is_a_mismatch_and_not_overwritten() {
        let mut v = Vec::new();
        evaluate(&mut v, "h", 22, "SHA256:a", "t");
        assert_eq!(evaluate(&mut v, "h", 22, "SHA256:b", "t"), Decision::Mismatch { stored: "SHA256:a".into() });
        assert_eq!(v[0].fingerprint, "SHA256:a");
    }

    #[test]
    fn port_is_part_of_the_identity() {
        let mut v = Vec::new();
        evaluate(&mut v, "h", 22, "SHA256:a", "t");
        assert_eq!(evaluate(&mut v, "h", 2222, "SHA256:b", "t"), Decision::New);
    }
}
