//! Zweiter globaler Modal-Launcher neben der Befehlspalette (Strg+K bleibt
//! für reine Bereichs-Navigation): Strg+Umschalt+F durchsucht echte Daten
//! (Items/Mobs/Quests) und trägt beim Auswählen eines Treffers eine vnum bzw.
//! einen Pfad ins Zielmodul (`go_to_with_selection`).

use super::skeleton::Spinner;
use super::Icon;
use crate::store::navigation::{nav, Section};
use crate::tauri::invoke;
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct EntityHit {
    pub vnum: i64,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct QuestSearchLine {
    pub line_number: i64,
    pub text: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct QuestSearchMatch {
    pub relative_path: String,
    #[allow(dead_code)]
    pub category: String,
    #[allow(dead_code)]
    pub name: String,
    pub lines: Vec<QuestSearchLine>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResultGroup {
    Item(Vec<EntityHit>),
    Mob(Vec<EntityHit>),
    Quest(Vec<QuestSearchMatch>),
}

/// Fasst die drei (einzeln fehlschlagbaren) Suchen zusammen - wie
/// `Promise.allSettled`: Fehler und leere Treffer werden ausgelassen.
pub fn collect_groups(
    items: Result<Vec<EntityHit>, String>,
    mobs: Result<Vec<EntityHit>, String>,
    quests: Result<Vec<QuestSearchMatch>, String>,
) -> Vec<ResultGroup> {
    let mut next = vec![];
    if let Ok(v) = items.filter_nonempty() {
        next.push(ResultGroup::Item(v));
    }
    if let Ok(v) = mobs.filter_nonempty() {
        next.push(ResultGroup::Mob(v));
    }
    if let Ok(v) = quests.filter_nonempty() {
        next.push(ResultGroup::Quest(v));
    }
    next
}

trait FilterNonEmpty<T> {
    fn filter_nonempty(self) -> Result<Vec<T>, ()>;
}

impl<T> FilterNonEmpty<T> for Result<Vec<T>, String> {
    fn filter_nonempty(self) -> Result<Vec<T>, ()> {
        match self {
            Ok(v) if !v.is_empty() => Ok(v),
            _ => Err(()),
        }
    }
}

#[derive(Default)]
struct Pending {
    done: u8,
    items: Option<Result<Vec<EntityHit>, String>>,
    mobs: Option<Result<Vec<EntityHit>, String>>,
    quests: Option<Result<Vec<QuestSearchMatch>, String>>,
}

#[component]
pub fn GlobalSearch() -> impl IntoView {
    let n = nav();
    let open = RwSignal::new(false);
    let query = RwSignal::new(String::new());
    let loading = RwSignal::new(false);
    let groups = RwSignal::new(Vec::<ResultGroup>::new());
    let input_ref = NodeRef::<html::Input>::new();

    let handle = window_event_listener(leptos::ev::keydown, move |e| {
        if (e.meta_key() || e.ctrl_key()) && e.shift_key() && e.key().to_lowercase() == "f" {
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
            groups.set(vec![]);
            request_animation_frame(move || {
                if let Some(el) = input_ref.get_untracked() {
                    let _ = el.focus();
                }
            });
        }
    });

    let run_search = move || {
        let q = query.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        loading.set(true);
        // Alle drei Suchen laufen parallel; ausgewertet wird, wenn alle fertig sind.
        let pending = Rc::new(RefCell::new(Pending::default()));
        let finish = {
            let pending = pending.clone();
            move || {
                let mut p = pending.borrow_mut();
                p.done += 1;
                if p.done < 3 {
                    return;
                }
                let next = collect_groups(
                    p.items.take().unwrap_or(Ok(vec![])),
                    p.mobs.take().unwrap_or(Ok(vec![])),
                    p.quests.take().unwrap_or(Ok(vec![])),
                );
                groups.set(next);
                loading.set(false);
            }
        };
        {
            let (pending, finish, q) = (pending.clone(), finish.clone(), q.clone());
            spawn_local(async move {
                let r = invoke::<Vec<EntityHit>>("search_items", &json!({ "query": q })).await;
                pending.borrow_mut().items = Some(r);
                finish();
            });
        }
        {
            let (pending, finish, q) = (pending.clone(), finish.clone(), q.clone());
            spawn_local(async move {
                let r = invoke::<Vec<EntityHit>>("search_mobs", &json!({ "query": q })).await;
                pending.borrow_mut().mobs = Some(r);
                finish();
            });
        }
        spawn_local(async move {
            let r = invoke::<Vec<QuestSearchMatch>>("search_quest_files", &json!({ "query": q })).await;
            pending.borrow_mut().quests = Some(r);
            finish();
        });
    };

    let pick = move |section: Section, target: String| {
        n.go_to_with_selection(section, target);
        open.set(false);
    };

    let group_view = move |g: ResultGroup| {
        let (icon, label) = match &g {
            ResultGroup::Item(_) => ("Package", "Items"),
            ResultGroup::Mob(_) => ("PawPrint", "Mobs"),
            ResultGroup::Quest(_) => ("ScrollText", "Quests"),
        };
        let rows = match g {
            ResultGroup::Quest(matches) => matches
                .into_iter()
                .map(|m| {
                    let path = m.relative_path.clone();
                    let first = m.lines.first().cloned();
                    view! {
                        <button
                            on:click=move |_| pick(Section::QuestBuilder, path.clone())
                            class="flex w-full flex-col items-start gap-0.5 rounded-md px-3 py-2 text-left text-sm hover:bg-muted"
                        >
                            <span class="truncate font-medium">{m.relative_path}</span>
                            {first.map(|l| view! {
                                <span class="truncate text-xs text-muted-foreground">
                                    {format!("Zeile {}: {}", l.line_number, l.text.trim())}
                                </span>
                            })}
                        </button>
                    }
                    .into_any()
                })
                .collect::<Vec<_>>(),
            ResultGroup::Item(hits) | ResultGroup::Mob(hits) => {
                let section = if label == "Items" { Section::ItemEditor } else { Section::MobProtoEditor };
                hits.into_iter()
                    .map(|r| {
                        let vnum = r.vnum;
                        let name = if r.name.is_empty() {
                            view! { <span class="text-muted-foreground">"(kein Name)"</span> }.into_any()
                        } else {
                            r.name.into_any()
                        };
                        view! {
                            <button
                                on:click=move |_| pick(section, vnum.to_string())
                                class="flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-left text-sm hover:bg-muted"
                            >
                                <span class="truncate">{name}</span>
                                <span class="shrink-0 text-xs text-muted-foreground">{format!("#{vnum}")}</span>
                            </button>
                        }
                        .into_any()
                    })
                    .collect::<Vec<_>>()
            }
        };
        view! {
            <div>
                <div class="flex items-center gap-1 px-3 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
                    <Icon name=icon class="size-3.5"/>
                    {label}
                </div>
                {rows}
            </div>
        }
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
                            on:input=move |e| query.set(event_target_value(&e))
                            on:keydown=move |e| {
                                if e.key() == "Enter" {
                                    run_search();
                                }
                            }
                            placeholder="Items, Mobs, Quests durchsuchen… (Strg+Umschalt+F)"
                            class="w-full bg-transparent py-1 text-sm outline-none"
                        />
                        <Show when=move || loading.get()>
                            <Spinner class="size-4 shrink-0 text-muted-foreground"/>
                        </Show>
                    </div>
                    <div class="max-h-96 overflow-y-auto p-1">
                        {move || groups.get().into_iter().map(group_view).collect_view()}
                        <Show when=move || !loading.get() && groups.with(|g| g.is_empty()) && !query.with(|q| q.trim().is_empty())>
                            <p class="p-3 text-sm text-muted-foreground">"Keine Treffer."</p>
                        </Show>
                        <Show when=move || query.with(|q| q.trim().is_empty())>
                            <p class="p-3 text-sm text-muted-foreground">
                                "Suchbegriff eingeben und Enter drücken - durchsucht Items, Mobs und Quest-Dateien gleichzeitig."
                            </p>
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

    #[test]
    fn groups_skip_errors_and_empty() {
        let hit = EntityHit { vnum: 1, name: "a".into() };
        let g = collect_groups(Ok(vec![hit.clone()]), Err("x".into()), Ok(vec![]));
        assert_eq!(g, vec![ResultGroup::Item(vec![hit.clone()])]);
        let g = collect_groups(Ok(vec![]), Ok(vec![hit.clone()]), Err("y".into()));
        assert_eq!(g, vec![ResultGroup::Mob(vec![hit])]);
        assert!(collect_groups(Err("a".into()), Err("b".into()), Err("c".into())).is_empty());
    }
}
