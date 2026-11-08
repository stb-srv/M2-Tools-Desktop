//! M2Manager-Oberfläche in Rust (Leptos, als WebAssembly im Tauri-Fenster).
//!
//! Ein Bundle für alle Fenster: `?view=manual` zeigt das Handbuch,
//! `?view=wiki` das Quest-Wiki, sonst die Haupt-App.

// Während der Portierung sind viele Helfer noch ungenutzt.
#![allow(dead_code)]

// `lib` ist hier ein normales Modul (Helfer), kein Crate-Typ.
#[allow(clippy::module_inception)]
mod lib;

mod app;
mod components;
mod features;
mod i18n;
mod layout;
mod store;
mod tauri;

use leptos::prelude::*;

fn view_param() -> Option<String> {
    let search = web_sys::window()?.location().search().ok()?;
    search.trim_start_matches('?').split('&').find_map(|kv| kv.strip_prefix("view=").map(str::to_string))
}

fn main() {
    console_error_panic_hook::set_once();
    // Globale Stores außerhalb jedes Owners anlegen, damit sie nie entsorgt werden.
    i18n::init();
    store::init();
    components::toast::init();

    match view_param().as_deref() {
        Some("manual") => mount_to_body(|| view! { <features::manual::Manual/> <components::toast::Toaster/> }),
        Some("wiki") => mount_to_body(|| view! { <features::quest_builder::QuestWiki/> <components::toast::Toaster/> }),
        _ => mount_to_body(|| view! { <app::App/> <components::toast::Toaster/> }),
    }
}
