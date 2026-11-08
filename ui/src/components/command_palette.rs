//! Globaler Strg+K/Cmd+K-Launcher, um direkt zu einem Bereich zu springen,
//! ohne die Sidebar abzusuchen - nutzt dieselbe Bereichsliste wie die Sidebar
//! (`Section::ALL`), damit beide nie auseinanderlaufen.

use super::Icon;
use crate::i18n::t;
use crate::store::navigation::{nav, Category, Section};
use leptos::html;
use leptos::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub key: String,
    pub label_key: String,
    pub items: Vec<Section>,
}

/// Baut die Gruppen der Palette: bei Suchbegriff nur "Ergebnisse", sonst
/// Favoriten, zuletzt benutzt (ohne Favoriten), alle Kategorien und zuletzt
/// die Einstellungen.
pub fn build_groups(query: &str, label: impl Fn(Section) -> String, favorites: &[Section], recent: &[Section]) -> Vec<Group> {
    let q = query.trim().to_lowercase();
    if !q.is_empty() {
        let items: Vec<Section> =
            Section::ALL.iter().copied().filter(|s| label(*s).to_lowercase().contains(&q)).collect();
        return if items.is_empty() {
            vec![]
        } else {
            vec![Group { key: "results".into(), label_key: "nav.groups.results".into(), items }]
        };
    }
    let mut groups = vec![];
    if !favorites.is_empty() {
        groups.push(Group { key: "favorites".into(), label_key: "nav.groups.favorites".into(), items: favorites.to_vec() });
    }
    let recent_items: Vec<Section> = recent.iter().copied().filter(|s| !favorites.contains(s)).collect();
    if !recent_items.is_empty() {
        groups.push(Group { key: "recent".into(), label_key: "nav.groups.recent".into(), items: recent_items });
    }
    for c in Category::ORDER {
        let items: Vec<Section> = Section::ALL.iter().copied().filter(|s| s.category() == *c).collect();
        if !items.is_empty() {
            groups.push(Group { key: c.id().into(), label_key: c.label_key(), items });
        }
    }
    groups.push(Group {
        key: "settings".into(),
        label_key: Category::Settings.label_key(),
        items: vec![Section::Settings],
    });
    groups
}

#[component]
pub fn CommandPalette() -> impl IntoView {
    let n = nav();
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let active_index = RwSignal::new(0usize);
    let input_ref = NodeRef::<html::Input>::new();

    let groups = Memo::new(move |_| {
        let q = query.get();
        let fav = n.favorites.get();
        let rec = n.recent.get();
        build_groups(&q, |s| t(s.label_key()), &fav, &rec)
    });
    let flat = Memo::new(move |_| groups.with(|g| g.iter().flat_map(|g| g.items.clone()).collect::<Vec<_>>()));

    let handle = window_event_listener(leptos::ev::keydown, move |e| {
        if (e.meta_key() || e.ctrl_key()) && e.key().to_lowercase() == "k" {
            e.prevent_default();
            open.update(|v| *v = !*v);
        } else if e.key() == "Escape" && open.get_untracked() {
            open.set(false);
        }
    });
    on_cleanup(move || handle.remove());

    Effect::new(move |_| {
        if open.get() {
            query.set(String::new());
            active_index.set(0);
            request_animation_frame(move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.focus();
                }
            });
        }
    });

    let choose = move |index: usize| {
        let Some(section) = flat.with_untracked(|f| f.get(index).copied()) else { return };
        n.set_section(section);
        open.set(false);
    };

    let on_keydown = move |e: leptos::ev::KeyboardEvent| match e.key().as_str() {
        "ArrowDown" => {
            e.prevent_default();
            let len = flat.with_untracked(|f| f.len());
            // Wie Math.min(i + 1, len - 1) - bei leerer Liste bleibt es bei 0.
            active_index.update(|i| *i = (*i + 1).min(len.saturating_sub(1)));
        }
        "ArrowUp" => {
            e.prevent_default();
            active_index.update(|i| *i = i.saturating_sub(1));
        }
        "Enter" => {
            e.prevent_default();
            choose(active_index.get_untracked());
        }
        _ => {}
    };

    view! {
        <Show when=move || open.get()>
            <div
                class="fixed inset-0 z-50 flex items-start justify-center bg-black/50 pt-[15vh]"
                on:click=move |_| open.set(false)
            >
                <div
                    class="w-full max-w-lg overflow-hidden rounded-lg border border-border bg-card shadow-xl"
                    on:click=|e| e.stop_propagation()
                >
                    <div class="flex items-center gap-2 border-b border-border px-3 py-2">
                        <Icon name="Search" class="size-4 shrink-0 text-muted-foreground"/>
                        <input
                            node_ref=input_ref
                            prop:value=move || query.get()
                            on:input=move |e| {
                                query.set(event_target_value(&e));
                                active_index.set(0);
                            }
                            on:keydown=on_keydown
                            placeholder="Zu einem Bereich springen… (Strg+K)"
                            class="w-full bg-transparent py-1 text-sm outline-none"
                        />
                    </div>
                    <div class="max-h-96 overflow-y-auto p-1">
                        {move || {
                            let mut running = 0usize;
                            groups
                                .get()
                                .into_iter()
                                .map(|g| {
                                    let items = g
                                        .items
                                        .into_iter()
                                        .map(|section| {
                                            let index = running;
                                            running += 1;
                                            let is_active = move || active_index.get() == index;
                                            view! {
                                                <button
                                                    on:click=move |_| choose(index)
                                                    on:mouseenter=move |_| active_index.set(index)
                                                    class=move || if is_active() {
                                                        "flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm bg-primary text-primary-foreground"
                                                    } else {
                                                        "flex w-full items-center gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-muted"
                                                    }
                                                >
                                                    <Icon name=section.icon() class="size-4"/>
                                                    <span class="flex-1 truncate">{move || t(section.label_key())}</span>
                                                    <Show when=move || n.is_dirty(section)>
                                                        <span class=move || if is_active() {
                                                            "size-1.5 shrink-0 rounded-full bg-primary-foreground"
                                                        } else {
                                                            "size-1.5 shrink-0 rounded-full bg-amber-500"
                                                        }></span>
                                                    </Show>
                                                </button>
                                            }
                                        })
                                        .collect_view();
                                    let label_key = g.label_key;
                                    view! {
                                        <div>
                                            <div class="px-3 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                                                {move || t(&label_key)}
                                            </div>
                                            {items}
                                        </div>
                                    }
                                })
                                .collect_view()
                        }}
                        <Show when=move || groups.with(|g| g.is_empty())>
                            <p class="p-3 text-sm text-muted-foreground">"Keine Treffer."</p>
                        </Show>
                    </div>
                </div>
            </div>
        </Show>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(s: Section) -> String {
        s.id().to_string()
    }

    #[test]
    fn query_filters_by_label() {
        let g = build_groups("  EDITOR ", label, &[], &[]);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].key, "results");
        assert!(g[0].items.contains(&Section::ItemEditor));
        assert!(!g[0].items.contains(&Section::Dashboard));
        assert!(build_groups("gibtsnicht", label, &[], &[]).is_empty());
    }

    #[test]
    fn default_groups() {
        let g = build_groups("", label, &[Section::ShopEditor], &[Section::ShopEditor, Section::ActivityLog]);
        assert_eq!(g[0].key, "favorites");
        assert_eq!(g[1].key, "recent");
        assert_eq!(g[1].items, vec![Section::ActivityLog]);
        assert_eq!(g[2].key, "overview");
        let last = g.last().unwrap();
        assert_eq!(last.key, "settings");
        assert_eq!(last.items, vec![Section::Settings]);
        // Kategorien ohne Einstellungen, die kommen nur einmal am Ende.
        assert_eq!(g.iter().filter(|x| x.items.contains(&Section::Settings)).count(), 1);
    }
}
