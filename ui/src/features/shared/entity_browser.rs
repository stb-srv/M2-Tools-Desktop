//! Immer gefüllte, paginierte Item-/Mob-Liste mit Suchfeld (leer = gesamter
//! Bestand nach vnum sortiert), optional aus dem lokalen Cache.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::lib::format::{de_int, de_time};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EntityRow {
    pub vnum: i64,
    pub name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Item,
    Mob,
}

impl EntityKind {
    pub fn id(self) -> &'static str {
        match self {
            EntityKind::Item => "item",
            EntityKind::Mob => "mob",
        }
    }
}

#[derive(Deserialize)]
struct EntityBrowsePage {
    rows: Vec<EntityRow>,
    total: i64,
}

#[derive(Clone, Deserialize)]
struct CacheMeta {
    #[allow(dead_code)]
    kind: String,
    synced_at: String,
    #[allow(dead_code)]
    row_count: i64,
}

const PAGE_SIZE_OPTIONS: [i64; 4] = [20, 50, 100, 200];
const DEFAULT_PAGE_SIZE: i64 = 20;

/// Rendert Zusatzinhalt pro Zeile (z.B. ein Item-Icon oder ein Status-Badge).
pub type RowRenderer = Arc<dyn Fn(&EntityRow) -> AnyView + Send + Sync>;

#[component]
pub fn EntityBrowser(
    kind: EntityKind,
    #[prop(into)] on_pick: Callback<EntityRow>,
    #[prop(into, optional)] pick_label: Option<String>,
    #[prop(optional)] auto_focus: bool,
    #[prop(into, optional)] max_height_class: Option<String>,
    /// Wird beim Mounten vorbelegt und sofort gesucht.
    #[prop(into, optional)] initial_query: Option<String>,
    /// Feuert, wenn sich die angezeigte Seite ändert.
    #[prop(into, optional)] on_rows_change: Option<Callback<Vec<EntityRow>>>,
    /// Vor dem Namen einer Zeile (z.B. Item-Icon).
    #[prop(optional)] render_leading: Option<RowRenderer>,
    /// Nach Name/vnum, vor dem Auswählen-Knopf (z.B. Status-Badge).
    #[prop(optional)] render_trailing: Option<RowRenderer>,
) -> impl IntoView {
    let pick_label = pick_label.unwrap_or_else(|| "Auswählen".into());
    let max_height_class = max_height_class.unwrap_or_else(|| "max-h-64".into());
    let query = RwSignal::new(initial_query.unwrap_or_default());
    let page = RwSignal::new(0i64);
    let page_size = RwSignal::new(DEFAULT_PAGE_SIZE);
    let refresh_key = RwSignal::new(0u32);
    let rows = RwSignal::new(Vec::<EntityRow>::new());
    let total = RwSignal::new(0i64);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let use_cache = RwSignal::new(false);
    let cache_meta = RwSignal::new(None::<CacheMeta>);
    let syncing = RwSignal::new(false);
    let sync_error = RwSignal::new(None::<String>);
    // Nur die Antwort der jeweils letzten Anfrage zählt (Schutz vor
    // überholten Antworten bei schnellem Tippen/Blättern).
    let request_id = StoredValue::new(0u64);

    spawn_local(async move {
        let meta = invoke::<Option<CacheMeta>>("get_entity_cache_meta", &json!({ "kind": kind.id() })).await;
        cache_meta.set(meta.ok().flatten());
    });

    let sync_cache = move || {
        syncing.set(true);
        sync_error.set(None);
        spawn_local(async move {
            match invoke::<CacheMeta>("sync_entity_cache", &json!({ "kind": kind.id() })).await {
                Ok(meta) => {
                    cache_meta.set(Some(meta));
                    if use_cache.get_untracked() {
                        refresh_key.update(|k| *k += 1);
                    }
                }
                Err(e) => sync_error.set(Some(e)),
            }
            syncing.set(false);
        });
    };

    Effect::new(move |_| {
        let cached = use_cache.get();
        let p = page.get();
        let size = page_size.get();
        refresh_key.track();
        let q = query.get_untracked().trim().to_string();
        let command = if cached {
            "browse_entities_cached"
        } else if kind == EntityKind::Item {
            "browse_items"
        } else {
            "browse_mobs"
        };
        let id = request_id.get_value() + 1;
        request_id.set_value(id);
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            let res = invoke::<EntityBrowsePage>(
                command,
                &json!({
                    "kind": kind.id(),
                    "query": if q.is_empty() { None } else { Some(q) },
                    "offset": p * size,
                    "limit": size,
                }),
            )
            .await;
            if request_id.get_value() != id {
                return;
            }
            match res {
                Ok(r) => {
                    rows.set(r.rows);
                    total.set(r.total);
                }
                Err(e) => {
                    error.set(Some(e));
                    rows.set(vec![]);
                    total.set(0);
                }
            }
            loading.set(false);
        });
    });

    Effect::new(move |_| {
        let r = rows.get();
        if let Some(cb) = on_rows_change {
            cb.run(r);
        }
    });

    let run_search = move || {
        page.set(0);
        refresh_key.update(|k| *k += 1);
    };

    let total_pages = move || ((total.get() + page_size.get() - 1) / page_size.get()).max(1);

    view! {
        <div class="space-y-2">
            <div class="flex gap-2">
                <input
                    autofocus=auto_focus
                    prop:value=move || query.get()
                    on:input=move |e| query.set(event_target_value(&e))
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            run_search()
                        }
                    }
                    placeholder="Nach Name oder VNUM filtern (leer = alle durchblättern)…"
                    class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <Button variant=ButtonVariant::Outline on_click=move |_| run_search() disabled=loading>
                    <Icon name="Search" class="size-4"/>
                </Button>
            </div>

            <div class="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                {move || cache_meta.get().map(|meta| {
                    view! {
                        <button
                            type="button"
                            on:click=move |_| {
                                use_cache.update(|v| *v = !*v);
                                page.set(0);
                            }
                            class=move || if use_cache.get() {
                                "flex items-center gap-1 rounded-full border px-2 py-0.5 border-amber-500/50 bg-amber-500/10 text-amber-700 dark:text-amber-400"
                            } else {
                                "flex items-center gap-1 rounded-full border px-2 py-0.5 border-border"
                            }
                            title="Zwischen Live-Daten und dem lokalen Cache umschalten"
                        >
                            <Icon name=Signal::derive(move || if use_cache.get() { "Zap" } else { "Wifi" }) class="size-3"/>
                            {move || if use_cache.get() {
                                format!("Cache (Stand {})", de_time(&meta.synced_at))
                            } else {
                                "Live".to_string()
                            }}
                        </button>
                    }
                })}
                <button
                    type="button"
                    on:click=move |_| sync_cache()
                    disabled=move || syncing.get()
                    class="flex items-center gap-1 hover:text-foreground disabled:opacity-60"
                >
                    <Icon name="RefreshCw" class=Signal::derive(move || if syncing.get() { "size-3 animate-spin".to_string() } else { "size-3".to_string() })/>
                    {move || if syncing.get() { "Synchronisiere…" } else { "Cache jetzt synchronisieren" }}
                </button>
                {move || sync_error.get().map(|e| view! { <span class="text-destructive">{e}</span> })}
            </div>

            {move || error.get().map(|e| view! { <p class="text-xs text-destructive">{e}</p> })}

            <div class=format!("space-y-1 overflow-y-auto rounded-md border border-border {max_height_class}")>
                <Show when=move || loading.get() && rows.with(|r| r.is_empty())>
                    <p class="flex items-center gap-1.5 p-2 text-sm text-muted-foreground">
                        <Icon name="Loader2" class="size-3.5 animate-spin"/>
                        " Lade…"
                    </p>
                </Show>
                <Show when=move || !loading.get() && rows.with(|r| r.is_empty()) && error.with(|e| e.is_none())>
                    <p class="p-2 text-sm text-muted-foreground">"Keine Treffer."</p>
                </Show>
                <For each=move || rows.get() key=|r| r.vnum let:r>
                    {
                        let leading = render_leading.as_ref().map(|f| f(&r));
                        let trailing = render_trailing.as_ref().map(|f| f(&r));
                        let row = r.clone();
                        let label = pick_label.clone();
                        view! {
                            <div class="flex items-center justify-between gap-2 px-2 py-1 text-sm hover:bg-muted">
                                <span class="flex min-w-0 items-center gap-2">
                                    {leading}
                                    <span class="truncate">
                                        {if r.name.is_empty() {
                                            view! { <span class="text-muted-foreground">"(kein Name)"</span> }.into_any()
                                        } else {
                                            r.name.clone().into_any()
                                        }}
                                        " "
                                        <span class="text-muted-foreground">"#"{r.vnum}</span>
                                        {trailing}
                                    </span>
                                </span>
                                <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| on_pick.run(row.clone())>
                                    {label}
                                </Button>
                            </div>
                        }
                    }
                </For>
            </div>

            <Show when=move || { total.get() > 0 }>
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                    <span>
                        {move || de_int(total.get())} " Treffer"
                        {move || (total_pages() > 1).then(|| format!(" · Seite {}/{}", page.get() + 1, total_pages()))}
                    </span>
                    <div class="flex items-center gap-2">
                        <label class="flex items-center gap-1">
                            "pro Seite"
                            <select
                                on:change=move |e| {
                                    page_size.set(event_target_value(&e).parse().unwrap_or(DEFAULT_PAGE_SIZE));
                                    page.set(0);
                                }
                                class="rounded-md border border-border bg-background px-1 py-0.5 text-xs"
                            >
                                {PAGE_SIZE_OPTIONS.iter().map(|s| {
                                    let s = *s;
                                    view! { <option value=s.to_string() selected=move || page_size.get() == s>{s}</option> }
                                }).collect_view()}
                            </select>
                        </label>
                        <Show when=move || { total_pages() > 1 }>
                            <div class="flex gap-1">
                                <Button
                                    variant=ButtonVariant::Outline
                                    size=ButtonSize::IconSm
                                    on_click=move |_| page.update(|p| *p = (*p - 1).max(0))
                                    disabled=Signal::derive(move || page.get() == 0 || loading.get())
                                >
                                    <Icon name="ChevronLeft" class="size-3.5"/>
                                </Button>
                                <Button
                                    variant=ButtonVariant::Outline
                                    size=ButtonSize::IconSm
                                    on_click=move |_| page.update(|p| *p = (*p + 1).min(total_pages() - 1))
                                    disabled=Signal::derive(move || page.get() >= total_pages() - 1 || loading.get())
                                >
                                    <Icon name="ChevronRight" class="size-3.5"/>
                                </Button>
                            </div>
                        </Show>
                    </div>
                </div>
            </Show>
        </div>
    }
}
