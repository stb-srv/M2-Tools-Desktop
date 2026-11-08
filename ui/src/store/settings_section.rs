//! Aktiver Tab der Einstellungen - eigener Store, damit z.B. die Startseite
//! direkt auf "Einstellungen → Verbindung" springen kann.

use leptos::prelude::*;
use std::sync::OnceLock;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SettingsSection {
    General,
    Connection,
    ServerPaths,
    Client,
    Notifications,
    Updates,
}

static SECTION: OnceLock<RwSignal<SettingsSection>> = OnceLock::new();

pub(super) fn init() {
    let _ = SECTION.set(RwSignal::new(SettingsSection::General));
}

pub fn settings_section() -> RwSignal<SettingsSection> {
    *SECTION.get().expect("store::init() nicht aufgerufen")
}

pub fn open_settings(section: SettingsSection) {
    settings_section().set(section);
    super::navigation::nav().set_section(super::navigation::Section::Settings);
}
