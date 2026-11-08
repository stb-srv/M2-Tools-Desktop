//! Einheitliche Lese-/Schreib-/Test-Logik für SSH- und MySQL-Verbindung
//! (Setup-Assistent, Einstellungen, "Neues Profil"-Dialog). Alle Werte landen
//! in den kanonischen Settings-Keys + Windows Credential Manager.

use crate::tauri::{invoke, invoke_unit};
use serde_json::{json, Value};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SshAuthMode {
    #[default]
    Password,
    Key,
}

impl SshAuthMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SshAuthMode::Password => "password",
            SshAuthMode::Key => "key",
        }
    }
    pub fn parse(s: &str) -> Self {
        if s == "key" {
            SshAuthMode::Key
        } else {
            SshAuthMode::Password
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SshValues {
    pub host: String,
    pub port: String,
    pub username: String,
    pub auth_mode: SshAuthMode,
    pub password: String,
    pub key_path: String,
    pub key_passphrase: String,
}

impl Default for SshValues {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: "22".into(),
            username: String::new(),
            auth_mode: SshAuthMode::Password,
            password: String::new(),
            key_path: String::new(),
            key_passphrase: String::new(),
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct MysqlValues {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
}

impl Default for MysqlValues {
    fn default() -> Self {
        Self { host: String::new(), port: "3306".into(), username: String::new(), password: String::new() }
    }
}

fn ssh_auth(v: &SshValues) -> Value {
    match v.auth_mode {
        SshAuthMode::Password => json!({ "type": "password", "password": v.password }),
        SshAuthMode::Key => json!({
            "type": "private_key",
            "path": v.key_path,
            "passphrase": if v.key_passphrase.is_empty() { Value::Null } else { Value::String(v.key_passphrase.clone()) },
        }),
    }
}

fn port_or(s: &str, d: u16) -> u16 {
    let s = s.trim();
    if s.is_empty() {
        d
    } else {
        s.parse().unwrap_or(d)
    }
}

pub fn mysql_config(v: &MysqlValues) -> Value {
    json!({ "host": v.host, "port": port_or(&v.port, 3306), "username": v.username, "database": Value::Null })
}

pub fn ssh_complete(v: &SshValues) -> bool {
    !v.host.trim().is_empty() && !v.username.trim().is_empty() && (v.auth_mode == SshAuthMode::Password || !v.key_path.is_empty())
}

pub fn mysql_complete(v: &MysqlValues) -> bool {
    !v.host.trim().is_empty() && !v.username.trim().is_empty()
}

pub async fn get_setting(key: &str) -> Option<String> {
    invoke::<Option<String>>("get_setting", &json!({ "key": key })).await.ok().flatten()
}

pub async fn set_setting(key: &str, value: &str) -> Result<(), String> {
    invoke_unit("set_setting", &json!({ "key": key, "value": value })).await
}

async fn secret(account: &str) -> Option<String> {
    invoke::<String>("get_credential", &json!({ "account": account })).await.ok()
}

/// Liest die aktuell aktive Verbindung (inkl. gespeicherter Passwörter).
pub async fn load_stored_connection() -> (SshValues, MysqlValues) {
    let ssh = SshValues {
        host: get_setting("ssh_host").await.unwrap_or_default(),
        port: get_setting("ssh_port").await.unwrap_or_else(|| "22".into()),
        username: get_setting("ssh_username").await.unwrap_or_default(),
        auth_mode: get_setting("ssh_auth_mode").await.map(|s| SshAuthMode::parse(&s)).unwrap_or_default(),
        password: secret("ssh_password").await.unwrap_or_default(),
        key_path: get_setting("ssh_key_path").await.unwrap_or_default(),
        key_passphrase: secret("ssh_key_passphrase").await.unwrap_or_default(),
    };
    let mysql = MysqlValues {
        host: get_setting("mysql_host").await.unwrap_or_default(),
        port: get_setting("mysql_port").await.unwrap_or_else(|| "3306".into()),
        username: get_setting("mysql_username").await.unwrap_or_default(),
        password: secret("mysql_password").await.unwrap_or_default(),
    };
    (ssh, mysql)
}

pub async fn test_ssh(v: &SshValues) -> Result<(), String> {
    invoke_unit(
        "test_ssh_connection",
        &json!({
            "config": { "host": v.host, "port": port_or(&v.port, 22), "username": v.username },
            "auth": ssh_auth(v),
        }),
    )
    .await
}

pub async fn test_mysql(v: &MysqlValues) -> Result<(), String> {
    invoke_unit("test_mysql_connection", &json!({ "config": mysql_config(v), "password": v.password })).await
}

/// Schreibt die SSH-Werte als aktive Verbindung (ohne Test).
pub async fn save_ssh(v: &SshValues) -> Result<(), String> {
    set_setting("ssh_host", v.host.trim()).await?;
    set_setting("ssh_port", if v.port.trim().is_empty() { "22" } else { v.port.trim() }).await?;
    set_setting("ssh_username", v.username.trim()).await?;
    set_setting("ssh_auth_mode", v.auth_mode.as_str()).await?;
    match v.auth_mode {
        SshAuthMode::Password => {
            invoke_unit("store_credential", &json!({ "account": "ssh_password", "secret": v.password })).await?;
        }
        SshAuthMode::Key => {
            set_setting("ssh_key_path", &v.key_path).await?;
            if !v.key_passphrase.is_empty() {
                invoke_unit("store_credential", &json!({ "account": "ssh_key_passphrase", "secret": v.key_passphrase })).await?;
            } else {
                // Sonst bliebe die Passphrase eines vorher aktiven Servers hängen.
                let _ = invoke_unit("delete_credential", &json!({ "account": "ssh_key_passphrase" })).await;
            }
        }
    }
    Ok(())
}

/// Schreibt die MySQL-Werte als aktive Verbindung (ohne Test).
pub async fn save_mysql(v: &MysqlValues) -> Result<(), String> {
    set_setting("mysql_host", v.host.trim()).await?;
    set_setting("mysql_port", if v.port.trim().is_empty() { "3306" } else { v.port.trim() }).await?;
    set_setting("mysql_username", v.username.trim()).await?;
    invoke_unit("store_credential", &json!({ "account": "mysql_password", "secret": v.password })).await
}

/// Baut den MySQL-Pool im Backend neu auf.
pub async fn connect_mysql(v: &MysqlValues) -> Result<(), String> {
    invoke_unit("connect_mysql", &json!({ "config": mysql_config(v), "password": v.password })).await
}

/// Der MySQL-Pool lebt nur im Speicher des Rust-Prozesses - nach jedem
/// Neustart wird er hier aus den gespeicherten Werten neu aufgebaut, damit
/// keine Seite "nicht verbunden" zeigt, obwohl alles konfiguriert ist.
pub async fn auto_connect_mysql() -> bool {
    if crate::tauri::invoke0::<bool>("is_mysql_connected").await.unwrap_or(false) {
        return true;
    }
    let (Some(host), Some(username)) = (get_setting("mysql_host").await, get_setting("mysql_username").await) else {
        return false;
    };
    let port = get_setting("mysql_port").await.unwrap_or_else(|| "3306".into());
    let Some(password) = secret("mysql_password").await else { return false };
    if host.is_empty() || username.is_empty() || password.is_empty() {
        return false;
    }
    connect_mysql(&MysqlValues { host, port, username, password }).await.is_ok()
}
