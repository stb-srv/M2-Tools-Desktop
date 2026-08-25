//! Tauri IPC commands, split by domain (mirrors the `db/` submodule split).
//! Every submodule's public items are re-exported here so callers keep using
//! `commands::foo` regardless of which file `foo` actually lives in -
//! `lib.rs`'s `invoke_handler![tauri::generate_handler![commands::foo, ...]]`
//! list does not need to change when a command moves between these files.

mod support;

mod account;
mod activity_log;
mod backups;
mod box_special_item;
mod broadcast_weather;
mod credentials_settings;
mod db_explorer;
mod economy;
mod entity_cache;
mod guild;
mod health_check;
mod item;
mod item_presets;
mod item_usage;
mod locale;
mod misc;
mod mob_drop;
mod module_importer;
mod quest;
mod refine;
mod regen;
mod shop;
mod ssh_server;
mod system_installer;

pub use account::*;
pub use activity_log::*;
pub use backups::*;
pub use box_special_item::*;
pub use broadcast_weather::*;
pub use credentials_settings::*;
pub use db_explorer::*;
pub use economy::*;
pub use entity_cache::*;
pub use guild::*;
pub use health_check::*;
pub use item::*;
pub use item_presets::*;
pub use item_usage::*;
pub use locale::*;
pub use misc::*;
pub use mob_drop::*;
pub use module_importer::*;
pub use quest::*;
pub use refine::*;
pub use regen::*;
pub use shop::*;
pub use ssh_server::*;
pub use system_installer::*;
