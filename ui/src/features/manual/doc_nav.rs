//! Gemeinsame Seitenleiste (Gruppen, Suche, aktive Seite) für die beiden
//! Dokumentations-Fenster Handbuch und Quest-Wiki - beide hatten im TS
//! dieselbe `NavEntry`/`filterNav`-Logik und dasselbe Markup.

use crate::components::Icon;
use leptos::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NavPage {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NavEntry {
    Page(NavPage),
    Group { label: &'static str, children: Vec<NavPage> },
}

/// Eine eingebettete Markdown-Seite.
pub type Pages = &'static [(&'static str, &'static str)];

pub fn page_content(pages: Pages, id: &str) -> Option<&'static str> {
    pages.iter().find(|(p, _)| *p == id).map(|(_, c)| *c)
}

pub fn flatten(nav: &[NavEntry]) -> Vec<NavPage> {
    nav.iter()
        .flat_map(|e| match e {
            NavEntry::Page(p) => vec![*p],
            NavEntry::Group { children, .. } => children.clone(),
        })
        .collect()
}

/// Filtert nach Beschriftung oder Seiteninhalt (ohne Groß-/Kleinschreibung).
pub fn filter_nav(nav: &[NavEntry], pages: Pages, query: &str) -> Vec<NavEntry> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return nav.to_vec();
    }
    let matches = |p: &NavPage| {
        p.label.to_lowercase().contains(&q) || page_content(pages, p.id).unwrap_or("").to_lowercase().contains(&q)
    };
    nav.iter()
        .filter_map(|e| match e {
            NavEntry::Page(p) => matches(p).then_some(e.clone()),
            NavEntry::Group { label, children } => {
                let children: Vec<NavPage> = children.iter().filter(|c| matches(c)).copied().collect();
                (!children.is_empty()).then_some(NavEntry::Group { label, children })
            }
        })
        .collect()
}

fn nav_button(page: NavPage, current: RwSignal<String>) -> impl IntoView {
    view! {
        <button
            on:click=move |_| current.set(page.id.to_string())
            class=move || {
                if current.with(|c| c == page.id) {
                    "block w-full rounded-md px-2 py-1 text-left text-sm bg-primary text-primary-foreground"
                } else {
                    "block w-full rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
                }
            }
        >
            {page.label}
        </button>
    }
}

/// Seitenleiste mit Titel, Suchfeld und gefilterter Navigation.
#[component]
pub fn DocSidebar(title: &'static str, nav: Vec<NavEntry>, pages: Pages, current: RwSignal<String>) -> impl IntoView {
    let query = RwSignal::new(String::new());
    let filtered = Memo::new(move |_| query.with(|q| filter_nav(&nav, pages, q)));
    view! {
        <aside class="flex w-64 shrink-0 flex-col gap-2 overflow-y-auto border-r border-border p-3">
            <div class="flex items-center gap-2 px-1 pb-1">
                <Icon name="BookOpen" class="size-5 text-muted-foreground"/>
                <span class="font-semibold">{title}</span>
            </div>
            <div class="relative">
                <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                <input
                    prop:value=move || query.get()
                    on:input=move |e| query.set(event_target_value(&e))
                    placeholder="Durchsuchen…"
                    class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                />
            </div>
            <nav class="flex-1 space-y-0.5">
                {move || {
                    filtered
                        .get()
                        .into_iter()
                        .map(|entry| match entry {
                            NavEntry::Page(p) => nav_button(p, current).into_any(),
                            NavEntry::Group { label, children } => view! {
                                <div class="pt-2">
                                    <div class="px-2 pb-1 text-xs font-medium text-muted-foreground">{label}</div>
                                    {children.into_iter().map(|c| nav_button(c, current)).collect_view()}
                                </div>
                            }
                            .into_any(),
                        })
                        .collect_view()
                }}
                <Show when=move || filtered.with(|f| f.is_empty())>
                    <p class="px-2 py-1 text-sm text-muted-foreground">"Keine Treffer."</p>
                </Show>
            </nav>
        </aside>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGES: Pages = &[("a", "# Alpha\nWolf"), ("b", "# Beta"), ("c", "# Gamma")];

    fn nav() -> Vec<NavEntry> {
        vec![
            NavEntry::Page(NavPage { id: "a", label: "Eins" }),
            NavEntry::Group { label: "G", children: vec![NavPage { id: "b", label: "Zwei" }, NavPage { id: "c", label: "Drei" }] },
        ]
    }

    #[test]
    fn filters_by_label_and_content() {
        assert_eq!(filter_nav(&nav(), PAGES, "  ").len(), 2);
        assert_eq!(filter_nav(&nav(), PAGES, "wolf"), vec![NavEntry::Page(NavPage { id: "a", label: "Eins" })]);
        assert_eq!(
            filter_nav(&nav(), PAGES, "drei"),
            vec![NavEntry::Group { label: "G", children: vec![NavPage { id: "c", label: "Drei" }] }]
        );
        assert!(filter_nav(&nav(), PAGES, "xyz").is_empty());
        assert_eq!(flatten(&nav()).len(), 3);
    }
}
