//! Handbuch-Fenster (aus src/features/manual/Manual.tsx), geöffnet mit
//! `index.html?view=manual#<modul-id>`. Seiten-IDs entsprechen 1:1 den
//! Bereichs-IDs aus store/navigation - dadurch kann jedes Modul per
//! `open_manual(section)` direkt auf seine eigene Seite verlinken.

pub mod doc_nav;

use crate::components::markdown::{Markdown, DOC_STYLE};
use doc_nav::{page_content, DocSidebar, NavEntry, NavPage, Pages};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

pub const PAGES: Pages = &[
    ("intro", include_str!("docs/intro.md")),
    ("dashboard", include_str!("docs/uebersicht/dashboard.md")),
    ("server-control", include_str!("docs/uebersicht/server-control.md")),
    ("build-deploy", include_str!("docs/uebersicht/build-deploy.md")),
    ("item-editor", include_str!("docs/items/item-editor.md")),
    ("item-proto-explorer", include_str!("docs/items/item-proto-explorer.md")),
    ("module-importer", include_str!("docs/items/module-importer.md")),
    ("refine-editor", include_str!("docs/items/refine-editor.md")),
    ("box-editor", include_str!("docs/items/box-editor.md")),
    ("cube-editor", include_str!("docs/items/cube-editor.md")),
    ("shop-editor", include_str!("docs/shops-monster/shop-editor.md")),
    ("mob-proto-editor", include_str!("docs/shops-monster/mob-proto-editor.md")),
    ("mob-drop-editor", include_str!("docs/shops-monster/mob-drop-editor.md")),
    ("drop-generator", include_str!("docs/shops-monster/drop-generator.md")),
    ("quest-builder", include_str!("docs/quests-welt/quest-builder.md")),
    ("regen-editor", include_str!("docs/quests-welt/regen-editor.md")),
    ("locale-editor", include_str!("docs/quests-welt/locale-editor.md")),
    ("server-events", include_str!("docs/server-accounts/server-events.md")),
    ("account-manager", include_str!("docs/server-accounts/account-manager.md")),
    ("gm-manager", include_str!("docs/server-accounts/gm-manager.md")),
    ("guild-manager", include_str!("docs/server-accounts/guild-manager.md")),
    ("economy-dashboard", include_str!("docs/server-accounts/economy-dashboard.md")),
    ("db-explorer", include_str!("docs/server-accounts/db-explorer.md")),
    ("broadcast-system", include_str!("docs/server-accounts/broadcast-system.md")),
    ("weather-control", include_str!("docs/server-accounts/weather-control.md")),
    ("backup-browser", include_str!("docs/backups/backup-browser.md")),
    ("db-backups", include_str!("docs/backups/db-backups.md")),
    ("activity-log", include_str!("docs/backups/activity-log.md")),
    ("tga-converter", include_str!("docs/assets/tga-converter.md")),
    ("icon-browser", include_str!("docs/assets/icon-browser.md")),
    ("model-viewer", include_str!("docs/assets/model-viewer.md")),
    ("settings", include_str!("docs/settings.md")),
];

const fn p(id: &'static str, label: &'static str) -> NavPage {
    NavPage { id, label }
}

/// Gruppierung + Beschriftung wie die Sidebar des Hauptfensters (bewusst
/// feste deutsche Texte, das Fenster hat kein i18n).
fn nav() -> Vec<NavEntry> {
    vec![
        NavEntry::Page(p("intro", "Einführung")),
        NavEntry::Group {
            label: "Übersicht",
            children: vec![p("dashboard", "Dashboard"), p("server-control", "Server-Steuerung"), p("build-deploy", "Quellcode Bauen & Einspielen")],
        },
        NavEntry::Group {
            label: "Items & Ausrüstung",
            children: vec![
                p("item-editor", "Item Editor"),
                p("item-proto-explorer", "Item-Übersicht"),
                p("module-importer", "Item-Importer"),
                p("refine-editor", "Aufwertungs-Editor"),
                p("cube-editor", "Cube-Editor"),
            ],
        },
        NavEntry::Group {
            label: "Shops & Monster",
            children: vec![
                p("shop-editor", "Shop-Editor"),
                p("mob-proto-editor", "Mob-Proto-Editor"),
                p("drop-generator", "Drop-Generator"),
                p("mob-drop-editor", "↳ Mob-Drops"),
                p("box-editor", "↳ Item-Gruppen (Kisten)"),
            ],
        },
        NavEntry::Group {
            label: "Quests & Welt",
            children: vec![p("quest-builder", "Quest Builder"), p("regen-editor", "Regen-Datei-Editor"), p("locale-editor", "Locale-Verwaltung")],
        },
        NavEntry::Group {
            label: "Server & Accounts",
            children: vec![
                p("server-events", "Server-Events"),
                p("weather-control", "↳ Tag/Nacht & Schnee"),
                p("account-manager", "Account-Verwaltung"),
                p("gm-manager", "↳ GM-Verwaltung"),
                p("guild-manager", "Gilden-Verwaltung"),
                p("economy-dashboard", "Wirtschafts-Dashboard"),
                p("db-explorer", "Datenbank-Explorer"),
                p("broadcast-system", "Broadcast-System"),
            ],
        },
        NavEntry::Group {
            label: "Backups",
            children: vec![p("backup-browser", "Backups (Dateien)"), p("db-backups", "↳ Datenbank-Backups"), p("activity-log", "Änderungsprotokoll")],
        },
        NavEntry::Group {
            label: "Client-Assets",
            children: vec![p("tga-converter", "TGA Konverter"), p("icon-browser", "Icon-Browser"), p("model-viewer", "3D-Modell-Viewer")],
        },
        NavEntry::Page(p("settings", "Einstellungen")),
    ]
}

/// Seite aus einem URL-Hash (`#item-editor`) - nur bekannte Seiten.
pub fn section_from_hash(hash: &str) -> Option<&'static str> {
    let id = hash.strip_prefix('#').unwrap_or(hash);
    PAGES.iter().find(|(p, _)| *p == id).map(|(p, _)| *p)
}

fn current_hash() -> String {
    web_sys::window().and_then(|w| w.location().hash().ok()).unwrap_or_default()
}

#[derive(Deserialize)]
struct NavigatePayload {
    section: String,
}

#[component]
pub fn Manual() -> impl IntoView {
    // Deep-Link beim ersten Öffnen: open_manual(section) übergibt die
    // Zielseite als URL-Hash.
    let current = RwSignal::new(section_from_hash(&current_hash()).unwrap_or("intro").to_string());

    // Hash-Wechsel (z.B. interne `#seite`-Links) springen zur Seite.
    let hash_listener = window_event_listener(leptos::ev::hashchange, move |_| {
        if let Some(id) = section_from_hash(&current_hash()) {
            current.set(id.to_string());
        }
    });
    on_cleanup(move || hash_listener.remove());

    // Fenster war schon offen und ein Modul ruft erneut "Hilfe" auf -
    // open_manual sendet dann nur ein Event. Schlägt listen fehl (keine
    // Tauri-Brücke), bleibt das Fenster trotzdem nutzbar.
    let unlisten = StoredValue::new_local(None::<crate::tauri::Unlisten>);
    spawn_local(async move {
        if let Ok(u) = crate::tauri::listen::<NavigatePayload>("manual-navigate", move |ev| {
            if page_content(PAGES, &ev.section).is_some() {
                current.try_set(ev.section);
            }
        })
        .await
        {
            if let Some(u) = unlisten.try_set_value(Some(u)).flatten() {
                u.unlisten();
            }
        }
    });
    on_cleanup(move || {
        unlisten.try_with_value(|u| {
            if let Some(u) = u {
                u.unlisten()
            }
        });
    });

    let content = Signal::derive(move || current.with(|c| page_content(PAGES, c).unwrap_or("").to_string()));

    view! {
        <div class="flex h-screen bg-background text-foreground">
            <DocSidebar title="Handbuch" nav=nav() pages=PAGES current=current/>
            <main class="min-w-0 flex-1 overflow-y-auto">
                <div class="mx-auto max-w-3xl px-8 py-8">
                    <Markdown content=content style=DOC_STYLE/>
                </div>
            </main>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_nav_entry_has_a_page_and_vice_versa() {
        let flat = doc_nav::flatten(&nav());
        assert_eq!(flat.len(), PAGES.len());
        for p in &flat {
            assert!(page_content(PAGES, p.id).is_some(), "{}", p.id);
        }
    }

    #[test]
    fn hash_parsing() {
        assert_eq!(section_from_hash("#item-editor"), Some("item-editor"));
        assert_eq!(section_from_hash("#unbekannt"), None);
        assert_eq!(section_from_hash(""), None);
    }
}
