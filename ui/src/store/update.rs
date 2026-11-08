//! "Update verfügbar"-Zustand (Punkt in der Sidebar, Update-Tab).

use crate::lib::storage;
use crate::tauri::UpdateInfo;
use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AvailableUpdate {
    pub version: String,
    pub notes: Option<String>,
}

#[derive(Clone, Copy)]
pub struct UpdateStore {
    pub available: RwSignal<Option<AvailableUpdate>>,
    /// Das zuletzt gefundene Update-Objekt (für "Jetzt installieren").
    pub handle: StoredValue<Option<UpdateInfo>, LocalStorage>,
}

const PENDING_NOTES_KEY: &str = "m2manager-pending-update-notes";

static STORE: OnceLock<UpdateStore> = OnceLock::new();

pub(super) fn init() {
    let _ = STORE.set(UpdateStore { available: RwSignal::new(None), handle: StoredValue::new_local(None) });
}

pub fn update_store() -> UpdateStore {
    *STORE.get().expect("store::init() nicht aufgerufen")
}

impl UpdateStore {
    pub fn set_available(&self, info: Option<UpdateInfo>) {
        let avail = info.as_ref().map(|u| AvailableUpdate { version: u.version.clone(), notes: u.body.clone() });
        if let Some(a) = &avail {
            // Damit der "Was ist neu"-Dialog genau diese Notizen nach dem
            // Neustart auf der neuen Version einmal zeigen kann.
            storage::set_json(PENDING_NOTES_KEY, a);
        }
        self.handle.set_value(info);
        self.available.set(avail);
    }
}

pub fn read_pending_update_notes() -> Option<AvailableUpdate> {
    storage::get_json(PENDING_NOTES_KEY)
}
