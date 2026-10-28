use crate::credentials;
use crate::settings;
use rusqlite::Connection;
use serde::Serialize;

// Named snapshots of the app's SSH+MySQL connection settings, letting a user
// switch between e.g. a Dev- and a Live-Server without retyping everything.
// Deliberately NOT a new parallel connection surface: every SSH/DB command in
// this app already reads its connection details fresh from the same fixed
// settings keys (ssh_host, mysql_host, ...) and credential-manager accounts
// (ssh_password, mysql_password, ...) on every call (see
// commands/support.rs::stored_ssh_auth/mysql_connection_settings) - so
// "activating" a profile just copies its saved values back into those exact
// same canonical keys. No command anywhere else in the app needs to know
// profiles exist at all.

#[derive(Debug, Clone, Serialize)]
pub struct ConnectionProfile {
    pub id: i64,
    pub name: String,
    pub ssh_host: String,
    pub ssh_port: String,
    pub ssh_username: String,
    pub ssh_auth_mode: String,
    pub ssh_key_path: Option<String>,
    pub mysql_host: String,
    pub mysql_port: String,
    pub mysql_username: String,
}

/// Settings key holding the id of the profile whose values currently sit in
/// the canonical keys - set by saving or activating a profile, cleared when
/// that profile is deleted. Lets the UI show which server is active.
pub const ACTIVE_PROFILE_KEY: &str = "active_connection_profile_id";

/// Settings that belong to one particular game server rather than to this
/// PC: file locations on the server, monitoring and server-control details.
/// Each profile keeps its own copy, so switching from e.g. Dev to Live also
/// switches these. Local things (client path, tools, theme) stay global.
pub const PER_SERVER_KEYS: &[&str] = &[
    "mob_drop_file_path",
    "common_drop_file_path",
    "etc_drop_file_path",
    "drop_item_group_file_path",
    "special_item_group_file_path",
    "cube_file_path",
    "quest_dir",
    "regen_base_dir",
    "locale_file_path",
    "server_process_names",
    "server_disk_path",
    "server_workdir",
    "server_cmd_start",
    "server_cmd_stop",
    "server_cmd_clear_logs",
    "server_cmd_reload_quests",
    "server_restart_delay_seconds",
    "db_backup_dir",
    "db_backup_databases",
    "db_backup_schedule_hours",
];

fn ssh_password_key(id: i64) -> String {
    format!("profile_ssh_password_{id}")
}
fn ssh_key_passphrase_key(id: i64) -> String {
    format!("profile_ssh_key_passphrase_{id}")
}
fn mysql_password_key(id: i64) -> String {
    format!("profile_mysql_password_{id}")
}

/// Snapshots the currently active connection settings+credentials under
/// `name` - upsert, saving under an existing name overwrites it (same
/// convention as `item_presets::save`). Secrets are copied best-effort: not
/// every setup has all three (key-based SSH has no `ssh_password`, key auth
/// without a passphrase has none either), a missing one just isn't copied.
pub fn save_current_as_profile(conn: &Connection, name: &str) -> Result<i64, String> {
    let require = |key: &str| -> Result<String, String> {
        settings::get_path(conn, key)?
            .ok_or_else(|| format!("Einstellung '{key}' ist nicht gesetzt - erst eine Verbindung einrichten."))
    };

    let ssh_host = require("ssh_host")?;
    let ssh_port = require("ssh_port")?;
    let ssh_username = require("ssh_username")?;
    let ssh_auth_mode = require("ssh_auth_mode")?;
    let ssh_key_path = settings::get_path(conn, "ssh_key_path")?;
    let mysql_host = require("mysql_host")?;
    let mysql_port = require("mysql_port")?;
    let mysql_username = require("mysql_username")?;

    conn.execute(
        "INSERT INTO connection_profiles
            (name, ssh_host, ssh_port, ssh_username, ssh_auth_mode, ssh_key_path, mysql_host, mysql_port, mysql_username)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
         ON CONFLICT(name) DO UPDATE SET
            ssh_host = excluded.ssh_host,
            ssh_port = excluded.ssh_port,
            ssh_username = excluded.ssh_username,
            ssh_auth_mode = excluded.ssh_auth_mode,
            ssh_key_path = excluded.ssh_key_path,
            mysql_host = excluded.mysql_host,
            mysql_port = excluded.mysql_port,
            mysql_username = excluded.mysql_username",
        rusqlite::params![
            name, ssh_host, ssh_port, ssh_username, ssh_auth_mode, ssh_key_path, mysql_host, mysql_port, mysql_username
        ],
    )
    .map_err(|e| e.to_string())?;

    let id: i64 = conn
        .query_row(
            "SELECT id FROM connection_profiles WHERE name = ?1",
            rusqlite::params![name],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if let Ok(secret) = credentials::get_secret("ssh_password") {
        let _ = credentials::store_secret(&ssh_password_key(id), &secret);
    }
    if let Ok(secret) = credentials::get_secret("ssh_key_passphrase") {
        let _ = credentials::store_secret(&ssh_key_passphrase_key(id), &secret);
    }
    if let Ok(secret) = credentials::get_secret("mysql_password") {
        let _ = credentials::store_secret(&mysql_password_key(id), &secret);
    }

    for key in PER_SERVER_KEYS {
        if let Some(value) = settings::get_path(conn, key)? {
            set_profile_setting(conn, id, key, &value)?;
        }
    }

    // The snapshot is by definition what's active right now.
    settings::set_path(conn, ACTIVE_PROFILE_KEY, &id.to_string())?;

    Ok(id)
}

pub fn list(conn: &Connection) -> Result<Vec<ConnectionProfile>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, ssh_host, ssh_port, ssh_username, ssh_auth_mode, ssh_key_path, mysql_host, mysql_port, mysql_username
             FROM connection_profiles ORDER BY name COLLATE NOCASE",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], row_to_profile)
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

fn row_to_profile(row: &rusqlite::Row) -> rusqlite::Result<ConnectionProfile> {
    Ok(ConnectionProfile {
        id: row.get(0)?,
        name: row.get(1)?,
        ssh_host: row.get(2)?,
        ssh_port: row.get(3)?,
        ssh_username: row.get(4)?,
        ssh_auth_mode: row.get(5)?,
        ssh_key_path: row.get(6)?,
        mysql_host: row.get(7)?,
        mysql_port: row.get(8)?,
        mysql_username: row.get(9)?,
    })
}

fn get_profile(conn: &Connection, id: i64) -> Result<ConnectionProfile, String> {
    conn.query_row(
        "SELECT id, name, ssh_host, ssh_port, ssh_username, ssh_auth_mode, ssh_key_path, mysql_host, mysql_port, mysql_username
         FROM connection_profiles WHERE id = ?1",
        rusqlite::params![id],
        row_to_profile,
    )
    .map_err(|e| e.to_string())
}

/// Copies the profile's settings+credentials back into the canonical keys
/// every existing SSH/DB command already reads. Deliberately does NOT
/// reconnect itself (no `mysql_pool` access here) - the frontend triggers
/// that the same way any other settings change already would.
pub fn activate(conn: &Connection, id: i64) -> Result<(), String> {
    let profile = get_profile(conn, id)?;

    settings::set_path(conn, "ssh_host", &profile.ssh_host)?;
    settings::set_path(conn, "ssh_port", &profile.ssh_port)?;
    settings::set_path(conn, "ssh_username", &profile.ssh_username)?;
    settings::set_path(conn, "ssh_auth_mode", &profile.ssh_auth_mode)?;
    if let Some(key_path) = &profile.ssh_key_path {
        settings::set_path(conn, "ssh_key_path", key_path)?;
    }
    settings::set_path(conn, "mysql_host", &profile.mysql_host)?;
    settings::set_path(conn, "mysql_port", &profile.mysql_port)?;
    settings::set_path(conn, "mysql_username", &profile.mysql_username)?;

    if let Ok(secret) = credentials::get_secret(&ssh_password_key(id)) {
        let _ = credentials::store_secret("ssh_password", &secret);
    }
    if let Ok(secret) = credentials::get_secret(&ssh_key_passphrase_key(id)) {
        let _ = credentials::store_secret("ssh_key_passphrase", &secret);
    }
    if let Ok(secret) = credentials::get_secret(&mysql_password_key(id)) {
        let _ = credentials::store_secret("mysql_password", &secret);
    }

    // Profiles saved before per-server settings existed have none stored -
    // leave the current values alone for those instead of wiping them.
    let stored = profile_settings(conn, id)?;
    if !stored.is_empty() {
        for key in PER_SERVER_KEYS {
            match stored.iter().find(|(k, _)| k == key) {
                Some((_, value)) => settings::set_path(conn, key, value)?,
                // Not set for this server -> fall back to the built-in default.
                None => settings::delete_path(conn, key)?,
            }
        }
    }

    settings::set_path(conn, ACTIVE_PROFILE_KEY, &id.to_string())?;

    Ok(())
}

fn set_profile_setting(conn: &Connection, id: i64, key: &str, value: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO connection_profile_settings (profile_id, key, value) VALUES (?1, ?2, ?3)
         ON CONFLICT(profile_id, key) DO UPDATE SET value = excluded.value",
        rusqlite::params![id, key, value],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn profile_settings(conn: &Connection, id: i64) -> Result<Vec<(String, String)>, String> {
    let mut stmt = conn
        .prepare("SELECT key, value FROM connection_profile_settings WHERE profile_id = ?1")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map(rusqlite::params![id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

/// Called after every `set_setting`: a per-server setting changed in the UI
/// is written into the active profile too, so it survives switching away
/// and back. Anything else is ignored.
pub fn mirror_setting(conn: &Connection, key: &str, value: &str) -> Result<(), String> {
    if !PER_SERVER_KEYS.contains(&key) {
        return Ok(());
    }
    match active_id(conn)? {
        Some(id) => set_profile_setting(conn, id, key, value),
        None => Ok(()),
    }
}

/// Id of the currently active profile, if any (and if it still exists).
pub fn active_id(conn: &Connection) -> Result<Option<i64>, String> {
    let Some(raw) = settings::get_path(conn, ACTIVE_PROFILE_KEY)? else {
        return Ok(None);
    };
    let Ok(id) = raw.parse::<i64>() else {
        return Ok(None);
    };
    Ok(get_profile(conn, id).ok().map(|p| p.id))
}

pub fn delete(conn: &Connection, id: i64) -> Result<(), String> {
    conn.execute("DELETE FROM connection_profiles WHERE id = ?1", rusqlite::params![id])
        .map_err(|e| e.to_string())?;
    // Explicit instead of relying on ON DELETE CASCADE, which SQLite only
    // honours with `PRAGMA foreign_keys = ON`.
    conn.execute(
        "DELETE FROM connection_profile_settings WHERE profile_id = ?1",
        rusqlite::params![id],
    )
    .map_err(|e| e.to_string())?;
    let _ = credentials::delete_secret(&ssh_password_key(id));
    let _ = credentials::delete_secret(&ssh_key_passphrase_key(id));
    let _ = credentials::delete_secret(&mysql_password_key(id));
    if settings::get_path(conn, ACTIVE_PROFILE_KEY)?.as_deref() == Some(id.to_string().as_str()) {
        settings::set_path(conn, ACTIVE_PROFILE_KEY, "")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory db");
        conn.execute_batch(
            "CREATE TABLE connection_profiles (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                ssh_host TEXT NOT NULL,
                ssh_port TEXT NOT NULL,
                ssh_username TEXT NOT NULL,
                ssh_auth_mode TEXT NOT NULL,
                ssh_key_path TEXT,
                mysql_host TEXT NOT NULL,
                mysql_port TEXT NOT NULL,
                mysql_username TEXT NOT NULL
            );
            CREATE TABLE connection_profile_settings (
                profile_id INTEGER NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (profile_id, key)
            );
            CREATE TABLE paths (key TEXT PRIMARY KEY, value TEXT NOT NULL);",
        )
        .expect("create tables");
        conn
    }

    fn seed_settings(conn: &Connection) {
        settings::set_path(conn, "ssh_host", "dev.example.com").unwrap();
        settings::set_path(conn, "ssh_port", "22").unwrap();
        settings::set_path(conn, "ssh_username", "game").unwrap();
        settings::set_path(conn, "ssh_auth_mode", "password").unwrap();
        settings::set_path(conn, "mysql_host", "127.0.0.1").unwrap();
        settings::set_path(conn, "mysql_port", "3306").unwrap();
        settings::set_path(conn, "mysql_username", "root").unwrap();
    }

    // Credential reads/writes below hit the real Windows Credential Manager
    // (same as credentials.rs's own `store_then_get_roundtrip` test) - always
    // clean up the profile-scoped entries afterwards so a test run doesn't
    // leave stray keyring entries behind, and never assert on their presence
    // since whether ssh_password/mysql_password happen to already be set on
    // the dev machine running this test is not under test control here.

    #[test]
    fn saves_current_settings_and_lists_them_back() {
        let conn = scratch_conn();
        seed_settings(&conn);
        let id = save_current_as_profile(&conn, "Dev-Server").unwrap();
        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, id);
        assert_eq!(profiles[0].ssh_host, "dev.example.com");
        assert_eq!(profiles[0].mysql_username, "root");
        let _ = delete(&conn, id);
    }

    #[test]
    fn saving_under_existing_name_overwrites_instead_of_duplicating() {
        let conn = scratch_conn();
        seed_settings(&conn);
        let id1 = save_current_as_profile(&conn, "Server").unwrap();
        settings::set_path(&conn, "ssh_host", "live.example.com").unwrap();
        let id2 = save_current_as_profile(&conn, "Server").unwrap();
        assert_eq!(id1, id2);
        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].ssh_host, "live.example.com");
        let _ = delete(&conn, id1);
    }

    #[test]
    fn activate_writes_profile_settings_back_into_canonical_keys() {
        let conn = scratch_conn();
        seed_settings(&conn);
        let id = save_current_as_profile(&conn, "Dev-Server").unwrap();
        settings::set_path(&conn, "ssh_host", "something-else.example.com").unwrap();
        activate(&conn, id).unwrap();
        assert_eq!(
            settings::get_path(&conn, "ssh_host").unwrap(),
            Some("dev.example.com".to_string())
        );
        let _ = delete(&conn, id);
    }

    #[test]
    fn missing_required_setting_refuses_to_save_a_profile() {
        let conn = scratch_conn();
        // ssh_host never set.
        assert!(save_current_as_profile(&conn, "Unvollständig").is_err());
    }

    #[test]
    fn delete_removes_only_the_targeted_profile() {
        let conn = scratch_conn();
        seed_settings(&conn);
        let id = save_current_as_profile(&conn, "weg").unwrap();
        let id2 = save_current_as_profile(&conn, "bleibt").unwrap();
        delete(&conn, id).unwrap();
        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "bleibt");
        let _ = delete(&conn, id2);
    }

    #[test]
    fn tracks_the_active_profile_across_save_activate_and_delete() {
        let conn = scratch_conn();
        seed_settings(&conn);
        assert_eq!(active_id(&conn).unwrap(), None);
        let dev = save_current_as_profile(&conn, "Dev").unwrap();
        assert_eq!(active_id(&conn).unwrap(), Some(dev));
        let live = save_current_as_profile(&conn, "Live").unwrap();
        assert_eq!(active_id(&conn).unwrap(), Some(live));
        activate(&conn, dev).unwrap();
        assert_eq!(active_id(&conn).unwrap(), Some(dev));
        // Deleting an inactive profile leaves the active one alone...
        delete(&conn, live).unwrap();
        assert_eq!(active_id(&conn).unwrap(), Some(dev));
        // ...deleting the active one clears it.
        delete(&conn, dev).unwrap();
        assert_eq!(active_id(&conn).unwrap(), None);
    }

    #[test]
    fn server_paths_follow_the_active_profile() {
        let conn = scratch_conn();
        seed_settings(&conn);
        settings::set_path(&conn, "quest_dir", "/dev/quest").unwrap();
        let dev = save_current_as_profile(&conn, "Dev").unwrap();

        // Live server: different path, and one key Dev never had.
        settings::set_path(&conn, "quest_dir", "/live/quest").unwrap();
        settings::set_path(&conn, "server_disk_path", "/live").unwrap();
        let live = save_current_as_profile(&conn, "Live").unwrap();

        activate(&conn, dev).unwrap();
        assert_eq!(settings::get_path(&conn, "quest_dir").unwrap().as_deref(), Some("/dev/quest"));
        assert_eq!(settings::get_path(&conn, "server_disk_path").unwrap(), None);

        // A change while Dev is active lands in Dev's copy only.
        settings::set_path(&conn, "quest_dir", "/dev/quest2").unwrap();
        mirror_setting(&conn, "quest_dir", "/dev/quest2").unwrap();

        activate(&conn, live).unwrap();
        assert_eq!(settings::get_path(&conn, "quest_dir").unwrap().as_deref(), Some("/live/quest"));
        assert_eq!(settings::get_path(&conn, "server_disk_path").unwrap().as_deref(), Some("/live"));

        activate(&conn, dev).unwrap();
        assert_eq!(settings::get_path(&conn, "quest_dir").unwrap().as_deref(), Some("/dev/quest2"));

        let _ = delete(&conn, dev);
        let _ = delete(&conn, live);
    }

    #[test]
    fn legacy_profile_without_stored_paths_keeps_current_paths() {
        let conn = scratch_conn();
        seed_settings(&conn);
        let id = save_current_as_profile(&conn, "Alt").unwrap();
        conn.execute("DELETE FROM connection_profile_settings", []).unwrap();
        settings::set_path(&conn, "quest_dir", "/bleibt").unwrap();
        activate(&conn, id).unwrap();
        assert_eq!(settings::get_path(&conn, "quest_dir").unwrap().as_deref(), Some("/bleibt"));
        let _ = delete(&conn, id);
    }

    #[test]
    fn mirror_ignores_global_settings() {
        let conn = scratch_conn();
        seed_settings(&conn);
        let id = save_current_as_profile(&conn, "Dev").unwrap();
        mirror_setting(&conn, "client_path", "C:/Client").unwrap();
        assert!(profile_settings(&conn, id).unwrap().iter().all(|(k, _)| k != "client_path"));
        let _ = delete(&conn, id);
    }
}
