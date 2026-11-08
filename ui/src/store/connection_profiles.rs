//! Server-Profile, geteilt von Startseite und Einstellungen. `revision`
//! zählt bei jedem Wechsel der aktiven Verbindung hoch, damit z.B. das
//! Dashboard seinen Status neu prüft.

use crate::util::connection::{connect_mysql, load_stored_connection, save_mysql, save_ssh, MysqlValues, SshValues};
use crate::tauri::{invoke0, invoke_unit};
use leptos::prelude::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Deserialize)]
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

#[derive(Clone, Copy)]
pub struct ConnectionProfiles {
    pub profiles: RwSignal<Vec<ConnectionProfile>>,
    pub active_id: RwSignal<Option<i64>>,
    pub loaded: RwSignal<bool>,
    pub revision: RwSignal<u32>,
}

static STORE: OnceLock<ConnectionProfiles> = OnceLock::new();

pub(super) fn init() {
    let _ = STORE.set(ConnectionProfiles {
        profiles: RwSignal::new(vec![]),
        active_id: RwSignal::new(None),
        loaded: RwSignal::new(false),
        revision: RwSignal::new(0),
    });
}

pub fn connection_profiles() -> ConnectionProfiles {
    *STORE.get().expect("store::init() nicht aufgerufen")
}

impl ConnectionProfiles {
    fn bump(&self) {
        self.revision.update(|r| *r += 1);
    }

    pub async fn reload(&self) -> Result<(), String> {
        let profiles = invoke0::<Vec<ConnectionProfile>>("list_connection_profiles").await?;
        let active = invoke0::<Option<i64>>("get_active_connection_profile_id").await?;
        self.profiles.set(profiles);
        self.active_id.set(active);
        self.loaded.set(true);
        Ok(())
    }

    pub async fn activate(&self, id: i64) -> Result<(), String> {
        invoke_unit("activate_connection_profile", &json!({ "id": id })).await?;
        // SSH liest seine Einstellungen bei jedem Befehl frisch, der MySQL-Pool
        // bleibt dagegen bis zum Neuverbinden am alten Server.
        let (_, mysql) = load_stored_connection().await;
        if !mysql.host.is_empty() && !mysql.username.is_empty() && !mysql.password.is_empty() {
            let _ = connect_mysql(&mysql).await;
        }
        self.reload().await?;
        self.bump();
        Ok(())
    }

    pub async fn create(&self, name: &str, ssh: &SshValues, mysql: &MysqlValues) -> Result<(), String> {
        save_ssh(ssh).await?;
        save_mysql(mysql).await?;
        invoke_unit("save_connection_profile", &json!({ "name": name })).await?;
        let _ = connect_mysql(mysql).await;
        self.reload().await?;
        self.bump();
        Ok(())
    }

    pub async fn remove(&self, id: i64) -> Result<(), String> {
        invoke_unit("delete_connection_profile", &json!({ "id": id })).await?;
        self.reload().await
    }

    /// Nach einer Änderung an SSH/MySQL das aktive Profil mitziehen.
    pub async fn sync_active(&self) -> Result<(), String> {
        let active = self.active_id.get_untracked();
        let name = self.profiles.with_untracked(|p| p.iter().find(|x| Some(x.id) == active).map(|x| x.name.clone()));
        if let Some(name) = name {
            invoke_unit("save_connection_profile", &json!({ "name": name })).await?;
        }
        self.reload().await?;
        self.bump();
        Ok(())
    }
}

