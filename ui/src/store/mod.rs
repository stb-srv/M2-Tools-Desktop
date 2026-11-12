//! Globaler App-Zustand (Ersatz für die zustand-Stores). Jeder Store wird
//! einmal in [`init`] außerhalb jedes reaktiven Owners angelegt und lebt
//! damit für die ganze Laufzeit; Zugriff über `nav()`, `theme()` usw.

pub mod client_unpack;
pub mod connection_profiles;
pub mod navigation;
pub mod settings_section;
pub mod theme;
pub mod update;

pub fn init() {
    navigation::init();
    theme::init();
    update::init();
    settings_section::init();
    connection_profiles::init();
    client_unpack::init();
}
