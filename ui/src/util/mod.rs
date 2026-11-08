//! Kleine, seitenübergreifende Helfer (Ersatz für `src/lib/*.ts`).

// Allgemeine Helfer-API: nicht jede Funktion/Variante wird aktuell genutzt.
#![allow(dead_code)]

pub mod connection;
pub mod csv;
pub mod format;
pub mod storage;

use crate::store::navigation::ModuleId;
use serde_json::json;

/// Verknüpft CSS-Klassen, leere Teile werden ausgelassen (Ersatz für `cn()`).
pub fn cn(parts: &[&str]) -> String {
    parts.iter().filter(|s| !s.is_empty()).copied().collect::<Vec<_>>().join(" ")
}

/// Schreibt einen Eintrag ins zentrale Änderungsprotokoll - bewusst
/// fire-and-forget: ein fehlgeschlagener Log-Eintrag darf die bereits
/// erfolgreiche Nutzeraktion nie nachträglich als Fehler erscheinen lassen.
/// Ein Aufruf pro abgeschlossener *logischer* Aktion, nicht pro Kommando.
pub fn log_activity(module: ModuleId, action: &str, summary: &str, target_kind: Option<&str>, target_ref: Option<&str>) {
    let args = json!({
        "module": module.id(),
        "action": action,
        "targetKind": target_kind,
        "targetRef": target_ref,
        "summary": summary,
    });
    leptos::task::spawn_local(async move {
        let _ = crate::tauri::invoke_unit("log_activity", &args).await;
    });
}

/// Öffnet das Handbuch in einem eigenen Fenster (oder fokussiert es und
/// springt per Event zur Seite des Moduls).
pub fn open_manual(section: Option<ModuleId>) {
    leptos::task::spawn_local(async move {
        let url = match section {
            Some(s) => format!("index.html?view=manual#{}", s.id()),
            None => "index.html?view=manual".to_string(),
        };
        match crate::tauri::open_window("manual", &url, "Handbuch - M2Manager", 1100.0, 800.0).await {
            Ok(true) => {
                if let Some(s) = section {
                    let _ = crate::tauri::emit("manual-navigate", &json!({ "section": s.id() })).await;
                }
            }
            Ok(false) => {}
            Err(e) => crate::components::toast::error(&e),
        }
    });
}

/// Strg+S / Cmd+S löst `on_save` aus, solange `enabled()` wahr ist und
/// `section` der aktive Bereich ist (alle besuchten Bereiche bleiben
/// versteckt gemountet - ohne die Prüfung würde ein verstecktes Modul den
/// Tastendruck abfangen).
pub fn use_save_shortcut(
    section: crate::store::navigation::Section,
    enabled: impl Fn() -> bool + 'static,
    on_save: impl Fn() + 'static,
) {
    use leptos::prelude::*;
    let handle = window_event_listener(leptos::ev::keydown, move |e| {
        if (e.ctrl_key() || e.meta_key()) && e.key().to_lowercase() == "s" {
            if crate::store::navigation::nav().section.get_untracked() != section {
                return;
            }
            if !enabled() {
                return;
            }
            e.prevent_default();
            on_save();
        }
    });
    on_cleanup(move || handle.remove());
}
