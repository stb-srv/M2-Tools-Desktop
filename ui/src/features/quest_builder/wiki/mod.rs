//! Quest-Wiki-Fenster (`index.html?view=wiki`, aus wiki/QuestWiki.tsx).
//!
//! Inhalt 1:1 übernommen aus https://github.com/wielandino/metin2-quest-docs
//! (gerendert unter https://wielandino.github.io/metin2-quest-docs/) - siehe
//! Quellenangabe am Ende jeder Seite. Nur zum eigenen Nachschlagen innerhalb
//! dieses Projekts, nicht zur Weiterverbreitung.

use crate::components::markdown::{Markdown, DOC_STYLE};
use crate::components::Icon;
use crate::features::manual::doc_nav::{flatten, page_content, DocSidebar, NavEntry, NavPage, Pages};
use leptos::prelude::*;
use leptos::task::spawn_local;

pub const PAGES: Pages = &[
    ("intro", include_str!("docs/intro.md")),
    ("getting-started", include_str!("docs/getting-started.md")),
    ("core-concepts", include_str!("docs/core-concepts.md")),
    ("api-reference/player-api", include_str!("docs/api-reference/player-api.md")),
    ("api-reference/npc-api", include_str!("docs/api-reference/npc-api.md")),
    ("api-reference/item-api", include_str!("docs/api-reference/item-api.md")),
    ("api-reference/guild-api", include_str!("docs/api-reference/guild-api.md")),
    ("api-reference/game-api", include_str!("docs/api-reference/game-api.md")),
    ("api-reference/party-api", include_str!("docs/api-reference/party-api.md")),
    ("api-reference/dungeon-api", include_str!("docs/api-reference/dungeon-api.md")),
    ("api-reference/other-apis", include_str!("docs/api-reference/other-apis.md")),
    ("user-interface", include_str!("docs/user-interface.md")),
    ("advanced-topics", include_str!("docs/advanced-topics.md")),
    ("best-practices", include_str!("docs/best-practices.md")),
    ("reference", include_str!("docs/reference.md")),
];

const fn p(id: &'static str, label: &'static str) -> NavPage {
    NavPage { id, label }
}

/// Reihenfolge/Gruppierung aus der sidebars.js des Original-Repos, eigene
/// deutsche Kurztitel.
fn nav() -> Vec<NavEntry> {
    vec![
        NavEntry::Page(p("intro", "Einführung")),
        NavEntry::Page(p("getting-started", "Erste Schritte")),
        NavEntry::Page(p("core-concepts", "Grundkonzepte")),
        NavEntry::Group {
            label: "API-Referenz",
            children: vec![
                p("api-reference/player-api", "Player API (pc.*)"),
                p("api-reference/npc-api", "NPC API"),
                p("api-reference/item-api", "Item API"),
                p("api-reference/guild-api", "Guild API"),
                p("api-reference/game-api", "Game API"),
                p("api-reference/party-api", "Party API"),
                p("api-reference/dungeon-api", "Dungeon API"),
                p("api-reference/other-apis", "Weitere APIs"),
            ],
        },
        NavEntry::Page(p("user-interface", "Benutzeroberfläche")),
        NavEntry::Page(p("advanced-topics", "Fortgeschrittene Themen")),
        NavEntry::Page(p("best-practices", "Best Practices")),
        NavEntry::Page(p("reference", "Kurzreferenz")),
    ]
}

fn open_url(url: &'static str) {
    spawn_local(async move {
        let _ = crate::tauri::open_url(url).await;
    });
}

#[component]
pub fn QuestWiki() -> impl IntoView {
    let current = RwSignal::new("intro".to_string());
    let all_pages = flatten(&nav());
    let current_label = move || {
        current.with(|c| all_pages.iter().find(|p| p.id == c).map(|p| p.label.to_string()).unwrap_or_else(|| c.clone()))
    };
    let content = Signal::derive(move || current.with(|c| page_content(PAGES, c).unwrap_or("").to_string()));

    view! {
        <div class="flex h-screen bg-background text-foreground">
            <DocSidebar title="Quest-Wiki" nav=nav() pages=PAGES current=current/>
            <main class="min-w-0 flex-1 overflow-y-auto">
                <div class="mx-auto max-w-3xl px-8 py-8">
                    <Markdown content=content style=DOC_STYLE/>

                    <footer class="mt-12 border-t border-border pt-4 text-xs text-muted-foreground">
                        <p>
                            "Inhalt (\"" {current_label} "\") 1:1 übernommen aus dem "
                            <button class="underline" on:click=move |_| open_url("https://github.com/wielandino/metin2-quest-docs")>
                                "Metin2 Quest Docs-Repository von wielandino"
                                <Icon name="ExternalLink" class="ml-0.5 inline size-3"/>
                            </button>
                            " (gerendert unter "
                            <button class="underline" on:click=move |_| open_url("https://wielandino.github.io/metin2-quest-docs/intro/")>
                                "wielandino.github.io/metin2-quest-docs"
                                <Icon name="ExternalLink" class="ml-0.5 inline size-3"/>
                            </button>
                            "). Alle Rechte am Originaltext liegen beim Autor - diese Kopie dient nur dem eigenen Nachschlagen innerhalb dieses Projekts."
                        </p>
                    </footer>
                </div>
            </main>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_nav_entry_has_a_page() {
        let flat = flatten(&nav());
        assert_eq!(flat.len(), PAGES.len());
        for p in &flat {
            assert!(page_content(PAGES, p.id).is_some(), "{}", p.id);
        }
    }
}
