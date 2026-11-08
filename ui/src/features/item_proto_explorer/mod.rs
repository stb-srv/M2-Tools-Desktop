//! Eigener durchblätterbarer Überblick über die komplette `item_proto`-
//! Tabelle (mit Typ-Label statt Rohwert und Sprung zum Bearbeiten). Zwei
//! Ansichten derselben Daten: Liste (Tabelle mit kleinem Icon) und Kacheln
//! (großes Icon). Port von `src/features/item-proto-explorer`.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::item_editor::item_flags::{item_type_label, ITEM_TYPES};
use crate::features::shared::ItemUsageModal;
use crate::lib::csv::export_rows_as_csv;
use crate::lib::format::de_int;
use crate::lib::storage;
use crate::store::navigation::{nav, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ItemProtoSummary {
    vnum: i64,
    name: String,
    item_type: i64,
    subtype: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ItemProtoPage {
    rows: Vec<ItemProtoSummary>,
    total: i64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    List,
    Grid,
}

const PAGE_SIZE_OPTIONS: [i64; 4] = [24, 48, 96, 192];
const VIEW_MODE_KEY: &str = "m2manager-item-overview-view";

fn load_view_mode() -> ViewMode {
    if storage::get(VIEW_MODE_KEY).as_deref() == Some("grid") {
        ViewMode::Grid
    } else {
        ViewMode::List
    }
}

fn type_label(t: i64) -> String {
    item_type_label(t).map(str::to_string).unwrap_or_else(|| t.to_string())
}

#[component]
pub fn ItemProtoExplorer() -> impl IntoView {
    let view_mode = RwSignal::new(load_view_mode());
    let query = RwSignal::new(String::new());
    let type_filter = RwSignal::new(String::new());
    let page = RwSignal::new(0i64);
    let page_size = RwSignal::new(48i64);
    let refresh_key = RwSignal::new(0u32);

    let data = RwSignal::new(None::<ItemProtoPage>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let icons = RwSignal::new(HashMap::<i64, Option<String>>::new());

    let exporting = RwSignal::new(false);
    let export_error = RwSignal::new(None::<String>);
    let usage_vnum = RwSignal::new(None::<i64>);

    let request_id = StoredValue::new(0u64);

    let filter_args = move |offset: i64, limit: i64| {
        let q = query.get_untracked().trim().to_string();
        let tf = type_filter.get_untracked();
        json!({
            "query": if q.is_empty() { None } else { Some(q) },
            "typeFilter": if tf.is_empty() { None } else { Some(crate::features::item_editor::types::js_number(&tf) as i64) },
            "offset": offset,
            "limit": limit,
        })
    };

    // Im TS hängt die Abfrage auch direkt an `query` - jede Eingabe lädt neu.
    Effect::new(move |_| {
        query.track();
        type_filter.track();
        refresh_key.track();
        let p = page.get();
        let size = page_size.get();
        let args = filter_args(p * size, size);
        let id = request_id.get_value() + 1;
        request_id.set_value(id);
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            let res = invoke::<ItemProtoPage>("browse_item_proto", &args).await;
            if request_id.get_value() != id {
                return;
            }
            match res {
                Ok(r) => data.set(Some(r)),
                Err(e) => {
                    error.set(Some(e));
                    data.set(None);
                }
            }
            loading.set(false);
        });
    });

    Effect::new(move |_| {
        let Some(vnums) = data.with(|d| d.as_ref().map(|d| d.rows.iter().map(|r| r.vnum).collect::<Vec<_>>())) else {
            return;
        };
        let mut seen = HashSet::new();
        for vnum in vnums {
            if !seen.insert(vnum) || icons.with_untracked(|i| i.contains_key(&vnum)) {
                continue;
            }
            spawn_local(async move {
                let url = invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await.ok().flatten();
                icons.update(|i| {
                    i.insert(vnum, url);
                });
            });
        }
    });

    let run_search = move || {
        page.set(0);
        refresh_key.update(|k| *k += 1);
    };

    let change_view = move |next: ViewMode| {
        view_mode.set(next);
        storage::set(VIEW_MODE_KEY, if next == ViewMode::Grid { "grid" } else { "list" });
    };

    // Exportiert alle Treffer der aktuellen Suche/des Typ-Filters - eine
    // Anfrage mit `limit = total` (gleiche WHERE-Klausel wie die Seite).
    let export_all_matches = move || {
        let Some(total) = data.with_untracked(|d| d.as_ref().map(|d| d.total)) else {
            return;
        };
        export_error.set(None);
        exporting.set(true);
        let args = filter_args(0, total);
        spawn_local(async move {
            let res: Result<(), String> = async {
                let full = invoke::<ItemProtoPage>("browse_item_proto", &args).await?;
                let rows = full
                    .rows
                    .iter()
                    .map(|r| vec![r.vnum.to_string(), r.name.clone(), type_label(r.item_type), r.subtype.to_string()])
                    .collect();
                export_rows_as_csv(
                    "item_proto_export.csv",
                    vec!["VNUM".into(), "Name".into(), "Typ".into(), "Subtyp".into()],
                    rows,
                )
                .await?;
                Ok(())
            }
            .await;
            if let Err(e) = res {
                export_error.set(Some(e));
            }
            exporting.set(false);
        });
    };

    let total_pages = move || {
        data.with(|d| d.as_ref().map(|d| ((d.total + page_size.get() - 1) / page_size.get()).max(1)).unwrap_or(1))
    };
    let rows = move || data.with(|d| d.as_ref().map(|d| d.rows.clone()).unwrap_or_default());
    let icon_of = move |vnum: i64| icons.with(|i| i.get(&vnum).cloned().flatten());
    let open_editor = move || nav().set_section(Section::ItemEditor);

    let list_view = move || {
        view! {
            <div class="min-h-0 flex-1 overflow-y-auto rounded-md border border-border">
                <table class="w-full text-sm">
                    <thead class="sticky top-0 bg-card">
                        <tr class="border-b border-border text-left text-xs text-muted-foreground">
                            <th class="p-2 font-medium">"Icon"</th>
                            <th class="p-2 font-medium">"VNUM"</th>
                            <th class="p-2 font-medium">"Name"</th>
                            <th class="p-2 font-medium">"Typ"</th>
                            <th class="p-2 font-medium">"Subtyp"</th>
                            <th class="p-2 font-medium"></th>
                        </tr>
                    </thead>
                    <tbody>
                        <For each=rows key=|r| r.vnum let:r>
                            {
                                let vnum = r.vnum;
                                view! {
                                    <tr class="border-b border-border/50 hover:bg-muted">
                                        <td class="p-2">
                                            {move || match icon_of(vnum) {
                                                Some(src) => view! { <img src=src alt="" class="size-6 [image-rendering:pixelated]"/> }.into_any(),
                                                None => view! { <div class="size-6 bg-muted/60"></div> }.into_any(),
                                            }}
                                        </td>
                                        <td class="p-2 text-muted-foreground">{vnum}</td>
                                        <td class="p-2">
                                            {if r.name.is_empty() {
                                                view! { <span class="text-muted-foreground">"(kein Name)"</span> }.into_any()
                                            } else {
                                                r.name.clone().into_any()
                                            }}
                                        </td>
                                        <td class="p-2">{type_label(r.item_type)}</td>
                                        <td class="p-2 text-muted-foreground">{r.subtype}</td>
                                        <td class="p-2 text-right">
                                            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| usage_vnum.set(Some(vnum))>
                                                <Icon name="Link2" class="size-3.5"/>
                                                "Verwendung prüfen"
                                            </Button>
                                            " "
                                            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| open_editor()>
                                                "Im Item Editor öffnen"
                                            </Button>
                                        </td>
                                    </tr>
                                }
                            }
                        </For>
                        <Show when=move || loading.get()>
                            <tr>
                                <td colspan="6" class="p-3 text-center text-sm text-muted-foreground">
                                    <Icon name="Loader2" class="mx-auto size-4 animate-spin"/>
                                </td>
                            </tr>
                        </Show>
                        <Show when=move || !loading.get() && data.with(|d| d.as_ref().is_some_and(|d| d.rows.is_empty()))>
                            <tr>
                                <td colspan="6" class="p-3 text-center text-sm text-muted-foreground">"Keine Treffer."</td>
                            </tr>
                        </Show>
                    </tbody>
                </table>
            </div>
        }
    };

    let grid_view = move || {
        view! {
            <div class="min-h-0 flex-1 overflow-y-auto rounded-md border border-border p-2">
                <Show when=move || loading.get() && data.with(|d| d.is_none())>
                    <div class="flex h-full items-center justify-center">
                        <Icon name="Loader2" class="size-5 animate-spin text-muted-foreground"/>
                    </div>
                </Show>
                <Show when=move || !loading.get() && data.with(|d| d.as_ref().is_some_and(|d| d.rows.is_empty()))>
                    <p class="p-4 text-center text-sm text-muted-foreground">"Keine Treffer."</p>
                </Show>
                <Show when=move || data.with(|d| d.as_ref().is_some_and(|d| !d.rows.is_empty()))>
                    <div class="grid grid-cols-3 gap-2 sm:grid-cols-4 md:grid-cols-6 lg:grid-cols-8">
                        <For each=rows key=|r| r.vnum let:r>
                            {
                                let vnum = r.vnum;
                                view! {
                                    <div class="flex flex-col items-center gap-1 rounded-md border border-border p-2 hover:bg-muted">
                                        <div class="flex size-16 shrink-0 items-center justify-center rounded-md bg-muted/40">
                                            {move || match icon_of(vnum) {
                                                Some(src) => view! {
                                                    <img src=src alt="" class="max-h-full max-w-full object-contain [image-rendering:pixelated]"/>
                                                }
                                                .into_any(),
                                                None => view! { <span class="text-[10px] text-muted-foreground">"#" {vnum}</span> }.into_any(),
                                            }}
                                        </div>
                                        <span class="w-full truncate text-center text-xs font-medium" title=r.name.clone()>
                                            {if r.name.is_empty() {
                                                view! { <span class="text-muted-foreground">"(kein Name)"</span> }.into_any()
                                            } else {
                                                r.name.clone().into_any()
                                            }}
                                        </span>
                                        <span class="text-[10px] text-muted-foreground">"#" {vnum} " · " {type_label(r.item_type)}</span>
                                        <Button
                                            variant=ButtonVariant::Outline
                                            size=ButtonSize::Sm
                                            class="mt-1 w-full"
                                            on_click=move |_| open_editor()
                                        >
                                            "Bearbeiten"
                                        </Button>
                                    </div>
                                }
                            }
                        </For>
                    </div>
                </Show>
            </div>
        }
    };

    view! {
        <div class="flex h-full flex-col gap-3">
            <PageHeader module=Section::ItemProtoExplorer title="Item-Übersicht"/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Durchblättert die komplette " <code>"item_proto"</code>
                "-Tabelle als Liste oder als Kacheln mit großem Icon - zum Bearbeiten eines Items geht es weiter in den Item Editor."
            </p>

            <div class="flex flex-wrap items-center gap-2">
                <div class="relative flex-1 min-w-48">
                    <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                    <input
                        prop:value=move || query.get()
                        on:input=move |e| query.set(event_target_value(&e))
                        on:keydown=move |e| {
                            if e.key() == "Enter" {
                                run_search()
                            }
                        }
                        placeholder="Nach Name oder VNUM filtern…"
                        class="w-full rounded-md border border-border bg-background py-1.5 pl-7 pr-2 text-sm"
                    />
                </div>
                <select
                    on:change=move |e| {
                        type_filter.set(event_target_value(&e));
                        page.set(0);
                    }
                    class="rounded-md border border-border bg-background px-2 py-1.5 text-sm"
                >
                    <option value="" prop:selected=move || type_filter.with(|t| t.is_empty())>"Alle Typen"</option>
                    {ITEM_TYPES.iter().map(|t| {
                        let v = t.value.to_string();
                        let v2 = v.clone();
                        view! { <option value=v prop:selected=move || type_filter.with(|f| *f == v2)>{t.label}</option> }
                    }).collect_view()}
                </select>
                <Button variant=ButtonVariant::Outline on_click=move |_| run_search() disabled=loading>
                    <Icon name="Search" class="size-4"/>
                </Button>
                <div class="flex rounded-md border border-border">
                    {move || {
                        let v = view_mode.get();
                        view! {
                            <Button
                                variant=if v == ViewMode::List { ButtonVariant::Secondary } else { ButtonVariant::Ghost }
                                size=ButtonSize::Sm
                                on_click=move |_| change_view(ViewMode::List)
                                title="Als Liste anzeigen"
                            >
                                <Icon name="List" class="size-4"/>
                                "Liste"
                            </Button>
                            <Button
                                variant=if v == ViewMode::Grid { ButtonVariant::Secondary } else { ButtonVariant::Ghost }
                                size=ButtonSize::Sm
                                on_click=move |_| change_view(ViewMode::Grid)
                                title="Als Kacheln mit großem Icon anzeigen"
                            >
                                <Icon name="LayoutGrid" class="size-4"/>
                                "Kacheln"
                            </Button>
                        }
                    }}
                </div>
                <Button
                    variant=ButtonVariant::Outline
                    on_click=move |_| export_all_matches()
                    disabled=Signal::derive(move || {
                        data.with(|d| d.as_ref().is_none_or(|d| d.total == 0)) || exporting.get()
                    })
                    title="Exportiert alle Treffer der aktuellen Suche/des Filters als CSV, nicht nur die sichtbare Seite"
                >
                    <Icon name="Download" class="size-4"/>
                    {move || if exporting.get() { "Exportiere…" } else { "Alle Treffer exportieren" }}
                </Button>
            </div>

            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
            {move || export_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            {move || match view_mode.get() {
                ViewMode::List => list_view().into_any(),
                ViewMode::Grid => grid_view().into_any(),
            }}

            <Show when=move || data.with(|d| d.as_ref().is_some_and(|d| d.total > 0))>
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                    <span>
                        {move || de_int(data.with(|d| d.as_ref().map(|d| d.total).unwrap_or(0)))} " Treffer"
                        {move || (total_pages() > 1).then(|| format!(" · Seite {}/{}", page.get() + 1, total_pages()))}
                    </span>
                    <div class="flex items-center gap-2">
                        <label class="flex items-center gap-1">
                            "pro Seite"
                            <select
                                on:change=move |e| {
                                    page_size.set(event_target_value(&e).parse().unwrap_or(48));
                                    page.set(0);
                                }
                                class="rounded-md border border-border bg-background px-1 py-0.5 text-xs"
                            >
                                {PAGE_SIZE_OPTIONS.iter().map(|s| {
                                    let s = *s;
                                    view! { <option value=s.to_string() prop:selected=move || page_size.get() == s>{s}</option> }
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

            {move || usage_vnum.get().map(|v| view! { <ItemUsageModal vnum=v on_close=Callback::new(move |_| usage_vnum.set(None))/> })}
        </div>
    }
}
