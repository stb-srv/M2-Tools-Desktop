//! Aufwertungs-Editor (Refine): komplette Aufwertungs-Kette eines Items
//! visualisieren und Rezepte bearbeiten (Portierung von
//! `src/features/refine-editor`).

pub mod logic;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::regen_editor::num_input::{js_number_string, NumInput};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use crate::lib::format::de_int;
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{nav, report_section_dirty, Section};
use crate::tauri::{invoke, invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use logic::{active_materials, initial_material_rows, parse_reuse_id, MaterialRow};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefineMaterial {
    pub vnum: f64,
    pub count: f64,
    pub locale_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct RefineRecipe {
    pub id: f64,
    pub cost: f64,
    pub prob: f64,
    pub materials: Vec<RefineMaterial>,
    pub used_by_count: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RefineChainNode {
    vnum: f64,
    locale_name: String,
    refine_set: f64,
    refined_vnum: f64,
    recipe: Option<RefineRecipe>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MobDropItem {
    item_vnum: f64,
    count: f64,
    percent: f64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct MobDropGroup {
    name: String,
    mob_vnum: f64,
    drop_type: String,
    items: Vec<MobDropItem>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ShopSource {
    shop_vnum: f64,
    shop_name: String,
    npc_vnum: f64,
    count: f64,
}

type IconCache = RwSignal<HashMap<i64, Option<String>>>;

fn n(v: f64) -> String {
    js_number_string(v)
}

#[component]
fn ItemIcon(vnum: f64, cache: IconCache) -> impl IntoView {
    let key = vnum as i64;
    if !cache.with_untracked(|c| c.contains_key(&key)) {
        spawn_local(async move {
            let url = invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await.ok().flatten();
            cache.update(|c| {
                c.insert(key, url);
            });
        });
    }
    view! {
        <div class="flex size-10 shrink-0 items-center justify-center rounded-md border border-border bg-muted/40">
            {move || match cache.with(|c| c.get(&key).cloned().flatten()).filter(|u| !u.is_empty()) {
                Some(url) => view! { <img src=url alt="" class="size-full object-contain"/> }.into_any(),
                None => view! { <span class="text-[9px] text-muted-foreground">"#" {n(vnum)}</span> }.into_any(),
            }}
        </div>
    }
}

#[derive(Clone, Copy)]
struct MobDrops {
    drops: RwSignal<Option<Vec<MobDropGroup>>>,
    loading: RwSignal<bool>,
}

impl MobDrops {
    fn load(self) {
        if self.drops.with_untracked(|d| d.is_some()) || self.loading.get_untracked() {
            return;
        }
        self.loading.set(true);
        spawn_local(async move {
            match invoke0::<Vec<MobDropGroup>>("read_mob_drop_file").await {
                Ok(d) => self.drops.set(Some(d)),
                // best-effort - material hints just show "not found" if this fails
                Err(_) => self.drops.set(Some(vec![])),
            }
            self.loading.set(false);
        });
    }
}

#[component]
fn MaterialSourceHint(material: RefineMaterial, mob_drops: MobDrops) -> impl IntoView {
    let open = RwSignal::new(false);
    let shop_sources = RwSignal::new(None::<Vec<ShopSource>>);
    let shop_loading = RwSignal::new(false);
    let shop_error = RwSignal::new(None::<String>);
    let vnum = material.vnum;

    let toggle = move |_| {
        let next = !open.get_untracked();
        open.set(next);
        if !next {
            return;
        }
        if mob_drops.drops.with_untracked(|d| d.is_none()) {
            mob_drops.load();
        }
        if shop_sources.with_untracked(|s| s.is_none()) {
            shop_loading.set(true);
            shop_error.set(None);
            spawn_local(async move {
                match invoke::<Vec<ShopSource>>("find_refine_shop_sources", &json!({ "vnum": vnum })).await {
                    Ok(s) => shop_sources.set(Some(s)),
                    // shopSources stays null on failure - so it can't be
                    // mistaken for a genuine "no shop sells this" result.
                    Err(e) => shop_error.set(Some(e)),
                }
                shop_loading.set(false);
            });
        }
    };

    let drops = Memo::new(move |_| {
        mob_drops.drops.with(|d| {
            d.as_deref()
                .unwrap_or(&[])
                .iter()
                .filter_map(|g| {
                    g.items.iter().find(|it| it.item_vnum == vnum).map(|drop| (g.name.clone(), g.mob_vnum, drop.percent))
                })
                .collect::<Vec<_>>()
        })
    });

    view! {
        <div class="inline-block">
            <button
                on:click=toggle
                class="flex items-center gap-1 text-xs text-muted-foreground underline decoration-dotted hover:text-foreground"
            >
                <Icon name="MapPin" class="size-3"/>
                " Woher bekomme ich das?"
            </button>
            <Show when=move || open.get()>
                <div class="mt-1 max-w-xs space-y-2 rounded-md border border-border bg-card p-2 text-xs">
                    <div>
                        <p class="mb-1 flex items-center gap-1 font-medium">
                            <Icon
                                name="Loader2"
                                class=Signal::derive(move || format!("size-3 {}", if mob_drops.loading.get() { "animate-spin" } else { "hidden" }))
                            />
                            " Mob-Drops"
                        </p>
                        <Show when=move || mob_drops.loading.get()>
                            <p class="text-muted-foreground">"Lade mob_drop_item.txt…"</p>
                        </Show>
                        <Show when=move || !mob_drops.loading.get() && drops.with(|d| d.is_empty())>
                            <p class="text-muted-foreground">"Kein Mob gefunden, der das droppt."</p>
                        </Show>
                        {move || drops.get().into_iter().map(|(name, mob_vnum, percent)| view! {
                            <p>
                                {name} " " <span class="text-muted-foreground">"#" {n(mob_vnum)}</span>
                                {format!(" — {}%", n(percent))}
                            </p>
                        }).collect_view()}
                    </div>
                    <div>
                        <p class="mb-1 flex items-center gap-1 font-medium">
                            <Icon name="Store" class="size-3"/>
                            " Shops"
                        </p>
                        <Show when=move || shop_loading.get()>
                            <p class="text-muted-foreground">"Lade…"</p>
                        </Show>
                        {move || (!shop_loading.get()).then(|| shop_error.get()).flatten().map(|e| view! {
                            <p class="text-destructive">"Fehler: " {e}</p>
                        })}
                        <Show when=move || {
                            !shop_loading.get()
                                && shop_error.with(|e| e.is_none())
                                && shop_sources.with(|s| s.as_ref().map(|v| v.len()).unwrap_or(0) == 0)
                        }>
                            <p class="text-muted-foreground">"Kein Shop gefunden."</p>
                        </Show>
                        {move || shop_sources.get().unwrap_or_default().into_iter().map(|s| view! {
                            <p>
                                {s.shop_name} " " <span class="text-muted-foreground">{format!("(NPC #{})", n(s.npc_vnum))}</span>
                            </p>
                        }).collect_view()}
                    </div>
                </div>
            </Show>
        </div>
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Snapshot {
    target_vnum: f64,
    cost: f64,
    prob: f64,
    materials: Vec<MaterialRow>,
}

#[component]
fn RecipeEditor(node: RefineChainNode, #[prop(into)] on_cancel: Callback<()>, #[prop(into)] on_saved: Callback<()>) -> impl IntoView {
    let node_vnum = node.vnum;
    let recipe = node.recipe.clone();
    let target_vnum = RwSignal::new(node.refined_vnum);
    let target_name = RwSignal::new(None::<String>);
    let cost = RwSignal::new(recipe.as_ref().map(|r| r.cost).unwrap_or(0.0));
    let prob = RwSignal::new(recipe.as_ref().map(|r| r.prob).unwrap_or(100.0));
    let materials = RwSignal::new(initial_material_rows(recipe.as_ref().map(|r| r.materials.as_slice()).unwrap_or(&[])));
    let reuse_id = RwSignal::new(String::new());
    let reuse_preview = RwSignal::new(None::<RefineRecipe>);
    let reuse_error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let material_picker_index = RwSignal::new(None::<usize>);
    let undoing = RwSignal::new(false);
    let undo_result = RwSignal::new(None::<String>);

    // Snapshot beim Mounten - Vergleich mit dem aktuellen Stand erkennt
    // ungespeicherte Änderungen ohne jeden einzelnen Setter zu markieren.
    let initial_snapshot = Snapshot {
        target_vnum: node.refined_vnum,
        cost: cost.get_untracked(),
        prob: prob.get_untracked(),
        materials: materials.get_untracked(),
    };
    let dirty = Memo::new(move |_| {
        let cur = Snapshot { target_vnum: target_vnum.get(), cost: cost.get(), prob: prob.get(), materials: materials.get() };
        cur != initial_snapshot
    });
    Effect::new(move |_| report_section_dirty(Section::RefineEditor, dirty.get()));
    on_cleanup(|| report_section_dirty(Section::RefineEditor, false));

    let used_by_others = recipe.as_ref().map(|r| r.used_by_count).unwrap_or(0.0) > 1.0;
    let has_recipe = recipe.is_some();
    let recipe_id = recipe.as_ref().map(|r| r.id);

    let load_reuse_preview = move || {
        reuse_error.set(None);
        reuse_preview.set(None);
        let id = match parse_reuse_id(&reuse_id.get_untracked()) {
            Some(id) => id,
            None => {
                reuse_error.set(Some("Bitte eine gültige Rezept-ID eingeben.".into()));
                return;
            }
        };
        spawn_local(async move {
            match invoke::<Option<RefineRecipe>>("get_refine_recipe", &json!({ "id": id })).await {
                Ok(Some(r)) => reuse_preview.set(Some(r)),
                Ok(None) => reuse_error.set(Some(format!("Rezept {} existiert nicht.", n(id)))),
                Err(e) => reuse_error.set(Some(e)),
            }
        });
    };

    let save_new_or_update = move |as_new: bool| {
        saving.set(true);
        error.set(None);
        let mats = active_materials(&materials.get_untracked());
        let id = if as_new { None } else { recipe_id };
        let c = cost.get_untracked();
        let p = prob.get_untracked();
        let target = target_vnum.get_untracked();
        spawn_local(async move {
            let res: Result<f64, String> = async {
                let new_id = invoke::<f64>(
                    "save_refine_recipe",
                    &json!({
                        "id": id,
                        "cost": c,
                        "prob": p,
                        "materials": mats.iter().map(|m| json!({ "vnum": m.vnum, "count": m.count, "locale_name": null })).collect::<Vec<_>>(),
                    }),
                )
                .await?;
                invoke_unit("set_item_refine_link", &json!({ "vnum": node_vnum, "refineSet": new_id, "refinedVnum": target }))
                    .await?;
                Ok(new_id)
            }
            .await;
            match res {
                Ok(new_id) => {
                    log_activity(
                        Section::RefineEditor.into(),
                        "save-recipe",
                        &format!("Aufwertungs-Rezept #{} für Item {} gespeichert (→ {})", n(new_id), n(node_vnum), n(target)),
                        Some("item"),
                        Some(&n(node_vnum)),
                    );
                    on_saved.run(());
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let apply_reuse = move || {
        let Some(preview) = reuse_preview.get_untracked() else { return };
        saving.set(true);
        error.set(None);
        let target = target_vnum.get_untracked();
        spawn_local(async move {
            match invoke_unit(
                "set_item_refine_link",
                &json!({ "vnum": node_vnum, "refineSet": preview.id, "refinedVnum": target }),
            )
            .await
            {
                Ok(()) => {
                    log_activity(
                        Section::RefineEditor.into(),
                        "reuse-recipe",
                        &format!("Item {} mit Rezept #{} verknüpft (→ {})", n(node_vnum), n(preview.id), n(target)),
                        Some("item"),
                        Some(&n(node_vnum)),
                    );
                    on_saved.run(());
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    use_save_shortcut(
        Section::RefineEditor,
        move || dirty.get_untracked() && !saving.get_untracked(),
        move || save_new_or_update(false),
    );

    // "Letzte Änderung rückgängig machen" - refine_proto hat kein
    // Datei-Backup, undo_last_refine_change stellt serverseitig den
    // vorherigen Zeilenstand wieder her (siehe refine_undo.rs).
    let undo_last_change = move || {
        undoing.set(true);
        error.set(None);
        undo_result.set(None);
        spawn_local(async move {
            match invoke0::<String>("undo_last_refine_change").await {
                Ok(message) => {
                    undo_result.set(Some(message.clone()));
                    log_activity(Section::RefineEditor.into(), "undo", &message, Some("item"), Some(&n(node_vnum)));
                    on_saved.run(());
                }
                Err(e) => error.set(Some(e)),
            }
            undoing.set(false);
        });
    };

    let clear_recipe = move || {
        saving.set(true);
        error.set(None);
        let target = target_vnum.get_untracked();
        spawn_local(async move {
            match invoke_unit("set_item_refine_link", &json!({ "vnum": node_vnum, "refineSet": 0, "refinedVnum": target })).await {
                Ok(()) => {
                    log_activity(
                        Section::RefineEditor.into(),
                        "clear-recipe",
                        &format!("Aufwertungs-Rezept für Item {} entfernt", n(node_vnum)),
                        Some("item"),
                        Some(&n(node_vnum)),
                    );
                    on_saved.run(());
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let used_warning = recipe.as_ref().filter(|_| used_by_others).map(|r| {
        view! {
            <p class="flex items-start gap-2 rounded-md bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                <Icon name="AlertTriangle" class="mt-0.5 size-3.5 shrink-0"/>
                {format!(
                    "Dieses Rezept (#{}) wird von {} Items verwendet. „Speichern\" ändert es für alle davon - „Als neues Rezept speichern\" trennt nur dieses Item ab.",
                    n(r.id),
                    n(r.used_by_count),
                )}
            </p>
        }
    });

    view! {
        <div class="space-y-3 rounded-md border border-primary/40 bg-primary/5 p-3 text-sm">
            <div class="flex items-center justify-between">
                <p class="font-medium">
                    {node.locale_name.clone()} " " <span class="text-muted-foreground">"#" {n(node_vnum)}</span> " bearbeiten"
                </p>
                <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| on_cancel.run(())>
                    <Icon name="X" class="size-4"/>
                </Button>
            </div>

            <div class="space-y-1">
                <p class="text-xs text-muted-foreground">"Ziel-Item (worauf wird geuppt, 0 = kein weiteres Upgrade)"</p>
                <div class="flex items-center gap-2">
                    <NumInput
                        value=target_vnum
                        on_change=move |v: f64| {
                            target_vnum.set(v);
                            target_name.set(None);
                        }
                        class="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    {move || target_name.get().filter(|t| !t.is_empty()).map(|t| view! { <span class="text-xs text-muted-foreground">{t}</span> })}
                </div>
                <EntityBrowser
                    kind=EntityKind::Item
                    on_pick=move |item: EntityRow| {
                        target_vnum.set(item.vnum as f64);
                        target_name.set(Some(item.name));
                    }
                />
            </div>

            {used_warning}

            <div class="grid grid-cols-2 gap-3">
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    <span class="flex items-center gap-1">
                        <Icon name="Coins" class="size-3.5"/>
                        " Kosten (Gold)"
                    </span>
                    <NumInput value=cost on_change=move |v| cost.set(v) class="rounded-md border border-border bg-background px-2 py-1 text-sm"/>
                </label>
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    <span class="flex items-center gap-1">
                        <Icon name="Percent" class="size-3.5"/>
                        " Erfolgschance (%)"
                    </span>
                    <NumInput
                        min="0"
                        max="100"
                        value=prob
                        on_change=move |v| prob.set(v)
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </label>
            </div>

            <div class="space-y-1">
                <p class="text-xs text-muted-foreground">"Materialien (bis zu 5, Anzahl 0 = leerer Slot)"</p>
                <For each=move || 0..materials.with(|m| m.len()) key=|i| *i let:i>
                    {
                        let row = move || materials.with(|m| m.get(i).cloned().unwrap_or_default());
                        view! {
                            <div class="flex items-center gap-2">
                                <span class="w-40 truncate text-sm">
                                    {move || {
                                        let m = row();
                                        if m.vnum != 0.0 { format!("{} (#{})", m.name, n(m.vnum)) } else { "— leer —".to_string() }
                                    }}
                                </span>
                                <NumInput
                                    min="0"
                                    value=Signal::derive(move || row().count)
                                    on_change=move |v: f64| materials.update(|m| {
                                        if let Some(r) = m.get_mut(i) {
                                            r.count = v;
                                        }
                                    })
                                    class="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                />
                                <Button
                                    variant=ButtonVariant::Outline
                                    size=ButtonSize::Sm
                                    on_click=move |_| material_picker_index.update(|p| *p = if *p == Some(i) { None } else { Some(i) })
                                >
                                    <Icon name="Search" class="size-3.5"/>
                                </Button>
                                <Show when=move || { row().vnum > 0.0 }>
                                    <Button
                                        variant=ButtonVariant::Outline
                                        size=ButtonSize::Sm
                                        on_click=move |_| materials.update(|m| {
                                            if let Some(r) = m.get_mut(i) {
                                                *r = MaterialRow::default();
                                            }
                                        })
                                    >
                                        <Icon name="Trash2" class="size-3.5"/>
                                    </Button>
                                </Show>
                            </div>
                        }
                    }
                </For>
                <Show when=move || material_picker_index.with(|p| p.is_some())>
                    <EntityBrowser
                        kind=EntityKind::Item
                        pick_label="Übernehmen"
                        on_pick=move |item: EntityRow| {
                            if let Some(idx) = material_picker_index.get_untracked() {
                                materials.update(|m| {
                                    if let Some(r) = m.get_mut(idx) {
                                        let count = if r.count == 0.0 { 1.0 } else { r.count };
                                        *r = MaterialRow { vnum: item.vnum as f64, name: item.name.clone(), count };
                                    }
                                });
                            }
                            material_picker_index.set(None);
                        }
                    />
                </Show>
            </div>

            <div class="flex flex-wrap items-center gap-2 border-t border-border pt-2">
                <Button size=ButtonSize::Sm disabled=saving on_click=move |_| save_new_or_update(false)>
                    <Icon name="Save" class="size-3.5"/>
                    {if has_recipe { " Speichern (bestehendes Rezept ändern)" } else { " Neues Rezept anlegen" }}
                </Button>
                {has_recipe.then(|| view! {
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline disabled=saving on_click=move |_| save_new_or_update(true)>
                        <Icon name="Plus" class="size-3.5"/>
                        " Als neues Rezept speichern"
                    </Button>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Destructive disabled=saving on_click=move |_| clear_recipe()>
                        "Rezept entfernen"
                    </Button>
                })}
                <Button
                    size=ButtonSize::Sm
                    variant=ButtonVariant::Outline
                    disabled=undoing
                    on_click=move |_| undo_last_change()
                    title="Setzt die zuletzt hier gespeicherte/angelegte/gelöschte Rezept-Zeile zurück"
                >
                    <Icon name="Undo2" class="size-3.5"/>
                    {move || if undoing.get() { " Setze zurück…" } else { " Letzte Änderung rückgängig machen" }}
                </Button>
            </div>
            {move || undo_result.get().filter(|m| !m.is_empty()).map(|m| view! { <p class="text-xs text-green-600">{m}</p> })}

            <details class="border-t border-border pt-2">
                <summary class="cursor-pointer text-xs text-muted-foreground">"Stattdessen bestehendes Rezept per ID wiederverwenden…"</summary>
                <div class="mt-2 flex items-center gap-2">
                    <input
                        prop:value=move || reuse_id.get()
                        on:input=move |e| reuse_id.set(event_target_value(&e))
                        placeholder="Rezept-ID"
                        class="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| load_reuse_preview()>
                        "Vorschau"
                    </Button>
                </div>
                {move || reuse_error.get().map(|e| view! { <p class="mt-1 text-xs text-destructive">{e}</p> })}
                {move || reuse_preview.get().map(|p| {
                    let mats = if p.materials.is_empty() {
                        "keine".to_string()
                    } else {
                        p.materials
                            .iter()
                            .map(|m| format!("{} x{}", m.locale_name.clone().unwrap_or_else(|| format!("#{}", n(m.vnum))), n(m.count)))
                            .collect::<Vec<_>>()
                            .join(", ")
                    };
                    view! {
                        <div class="mt-2 space-y-1 rounded-md bg-muted/40 p-2 text-xs">
                            <p>
                                {format!(
                                    "Kosten: {} Gold, Chance: {}%, verwendet von {} Item(s)",
                                    n(p.cost),
                                    n(p.prob),
                                    n(p.used_by_count),
                                )}
                            </p>
                            <p>"Materialien: " {mats}</p>
                            <Button size=ButtonSize::Sm disabled=saving on_click=move |_| apply_reuse()>
                                "Dieses Rezept übernehmen"
                            </Button>
                        </div>
                    }
                })}
            </details>

            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
        </div>
    }
}

#[component]
pub fn RefineEditor() -> impl IntoView {
    let chain = RwSignal::new(None::<Vec<RefineChainNode>>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let editing_vnum = RwSignal::new(None::<f64>);
    let icon_cache: IconCache = RwSignal::new(HashMap::new());
    let mob_drops = MobDrops { drops: RwSignal::new(None), loading: RwSignal::new(false) };

    let load_chain = move |vnum: f64| {
        loading.set(true);
        error.set(None);
        editing_vnum.set(None);
        spawn_local(async move {
            match invoke::<Vec<RefineChainNode>>("get_refine_chain", &json!({ "vnum": vnum })).await {
                Ok(c) => chain.set(Some(c)),
                Err(e) => {
                    error.set(Some(e));
                    chain.set(None);
                }
            }
            loading.set(false);
        });
    };

    let reload_first = move || {
        if let Some(first) = chain.with_untracked(|c| c.as_ref().and_then(|c| c.first().map(|n| n.vnum))) {
            load_chain(first);
        }
    };

    let chain_view = move || {
        chain.get().map(|nodes| {
            let len = nodes.len();
            let rows = nodes
                .into_iter()
                .enumerate()
                .map(|(i, node)| {
                    let vnum = node.vnum;
                    let orphan = node.refine_set > 0.0 && node.recipe.is_none();
                    let refine_set = node.refine_set;
                    let editor_node = node.clone();
                    let is_last = i + 1 >= len;
                    let arrow = match (&node.recipe, is_last) {
                        (_, true) => None,
                        (Some(r), false) => {
                            let mats = if r.materials.is_empty() {
                                view! { <span>"keine Materialien nötig"</span> }.into_any()
                            } else {
                                view! {
                                    <div class="flex flex-wrap items-center gap-2">
                                        {r.materials.iter().cloned().map(|m| view! {
                                            <span class="flex items-center gap-1 rounded-md border border-border px-1.5 py-0.5">
                                                {format!("{} x{}", m.locale_name.clone().unwrap_or_else(|| format!("#{}", n(m.vnum))), n(m.count))}
                                                <MaterialSourceHint material=m mob_drops=mob_drops/>
                                            </span>
                                        }).collect_view()}
                                    </div>
                                }
                                .into_any()
                            };
                            Some(view! {
                                <div class="ml-5 flex items-center gap-3 border-l-2 border-dashed border-border py-2 pl-5 text-xs text-muted-foreground">
                                    <Icon name="ArrowRight" class="size-4 shrink-0"/>
                                    <span class="flex items-center gap-1">
                                        <Icon name="Coins" class="size-3.5"/>
                                        {format!(" {} Gold", de_int(r.cost as i64))}
                                    </span>
                                    <span class="flex items-center gap-1">
                                        <Icon name="Percent" class="size-3.5"/>
                                        {format!(" {}%", n(r.prob))}
                                    </span>
                                    {mats}
                                </div>
                            }
                            .into_any())
                        }
                        (None, false) => Some(view! {
                            <div class="ml-5 flex items-center gap-2 border-l-2 border-dashed border-destructive/40 py-2 pl-5 text-xs text-destructive">
                                <Icon name="ArrowRight" class="size-4 shrink-0"/>
                                " Kein Rezept verknüpft - Upgrade nicht möglich, obwohl ein Ziel-Item gesetzt ist"
                            </div>
                        }
                        .into_any()),
                    };
                    view! {
                        <div>
                            <div class="flex items-center gap-3">
                                <ItemIcon vnum=vnum cache=icon_cache/>
                                <div class="flex-1">
                                    <p class="text-sm font-medium">
                                        {node.locale_name.clone()} " " <span class="text-muted-foreground">"#" {n(vnum)}</span>
                                    </p>
                                    {orphan.then(|| view! {
                                        <p class="flex items-center gap-1 text-xs text-destructive">
                                            <Icon name="AlertTriangle" class="size-3"/>
                                            {format!(" Rezept #{} nicht gefunden (verwaiste Verknüpfung)", n(refine_set))}
                                        </p>
                                    })}
                                </div>
                                <Button
                                    variant=ButtonVariant::Outline
                                    size=ButtonSize::Sm
                                    on_click=move |_| editing_vnum.update(|e| *e = if *e == Some(vnum) { None } else { Some(vnum) })
                                >
                                    <Icon name="Pencil" class="size-3.5"/>
                                    " Bearbeiten"
                                </Button>
                            </div>

                            <Show when=move || editing_vnum.get() == Some(vnum)>
                                <div class="mt-2 ml-[3.25rem]">
                                    <RecipeEditor
                                        node=editor_node.clone()
                                        on_cancel=move |_| editing_vnum.set(None)
                                        on_saved=move |_| reload_first()
                                    />
                                </div>
                            </Show>

                            {arrow}
                        </div>
                    }
                })
                .collect_view();
            view! {
                <section class="space-y-4 rounded-lg border border-border p-4">
                    <h2 class="text-sm font-medium text-muted-foreground">{format!("Aufwertungs-Kette ({len} Stufe(n))")}</h2>
                    <div class="flex flex-col gap-3">{rows}</div>
                </section>
            }
        })
    };

    view! {
        <div class="space-y-6 pb-10">
            <PageHeader module=Section::RefineEditor title="Aufwertungs-Editor (Refine)"/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Visualisiert die komplette Aufwertungs-Kette eines Items (Materialien, Gold-Kosten, Erfolgschance je Stufe) statt roher "
                <code>"refine_proto"</code> "-Zeilen. " <code>"refine_set"</code>
                " ist dabei eine reine Rezept-ID, die mit der Item-Vnum nichts zu tun hat - genau das macht diese Ansicht praktisch."
            </p>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Änderungen wirken erst nach einem Server-Neustart - der DB-Prozess lädt " <code>"refine_proto"</code>
                    " nur beim Start aus der Datenbank, anders als bei Item-Icons/-Modellen gibt es hier keinen Client-Repack. "
                    <button class="underline" on:click=move |_| nav().set_section(Section::ServerControl)>
                        "Zur Server-Steuerung"
                    </button>
                </span>
            </div>

            <p class="text-xs text-muted-foreground">
                "Andere Items können die Erfolgschance einer Aufwertung beeinflussen (Aufwertungs-Schriftrollen). "
                <button class="underline" on:click=move |_| nav().set_section(Section::ItemEditor)>
                    "Neue Aufwertungs-Schriftrolle erstellen"
                </button>
            </p>

            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Item wählen"</h2>
                <EntityBrowser kind=EntityKind::Item pick_label="Kette laden" on_pick=move |item: EntityRow| load_chain(item.vnum as f64)/>
            </section>

            <Show when=move || loading.get()>
                <p class="text-sm text-muted-foreground">"Lade Kette…"</p>
            </Show>
            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            {chain_view}
        </div>
    }
}
