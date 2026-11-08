//! Cube-Editor: Rezepte aus `cube.txt` bearbeiten (Portierung von
//! `src/features/cube-editor`).

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::regen_editor::num_input::{js_number_string, NumInput};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use crate::util::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::{invoke, invoke0};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CubeValue {
    vnum: f64,
    count: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct CubeRecipe {
    npc_vnums: Vec<f64>,
    items: Vec<CubeValue>,
    rewards: Vec<CubeValue>,
    percent: f64,
    gold: f64,
}

type Icons = RwSignal<HashMap<i64, Option<String>>>;

fn ensure_icon(icons: Icons, vnum: f64, kind: EntityKind) {
    let key = vnum as i64;
    if kind != EntityKind::Item || icons.with_untracked(|m| m.contains_key(&key)) {
        return;
    }
    spawn_local(async move {
        let url = invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await.ok().flatten();
        icons.update(|m| {
            m.insert(key, url);
        });
    });
}

#[component]
fn Field(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
            {label}
            {children()}
        </label>
    }
}

#[component]
fn EntityLabel(#[prop(into)] vnum: Signal<f64>, kind: EntityKind, icons: Icons) -> impl IntoView {
    Effect::new(move |_| ensure_icon(icons, vnum.get(), kind));
    view! {
        <span class="flex items-center gap-1.5">
            {move || {
                if kind != EntityKind::Item {
                    return None;
                }
                let key = vnum.get() as i64;
                icons
                    .with(|m| m.get(&key).cloned().flatten())
                    .filter(|u| !u.is_empty())
                    .map(|u| view! { <img src=u alt="" class="size-5 [image-rendering:pixelated]"/> })
            }}
            <span class="text-xs text-muted-foreground">"#" {move || js_number_string(vnum.get())}</span>
        </span>
    }
}

// Picks a vnum via the shared EntityBrowser, shown inline below a
// "Hinzufügen…"-toggle - kept collapsed by default since a cube recipe can
// have several NPC/material/reward rows at once.
#[component]
fn AddPicker(kind: EntityKind, #[prop(into)] on_pick: Callback<f64>) -> impl IntoView {
    let open = RwSignal::new(false);
    view! {
        <div class="flex flex-col gap-1">
            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| open.update(|o| *o = !*o)>
                <Icon name="Search" class="size-3.5"/>
                {if kind == EntityKind::Mob { " NPC suchen…" } else { " Item suchen…" }}
            </Button>
            <Show when=move || open.get()>
                <div class="w-96 rounded-md border border-border bg-card p-2 shadow-md">
                    <EntityBrowser
                        kind=kind
                        pick_label="Hinzufügen"
                        auto_focus=true
                        max_height_class="max-h-56"
                        on_pick=move |r: EntityRow| {
                            on_pick.run(r.vnum as f64);
                            open.set(false);
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ValueList {
    Items,
    Rewards,
}

impl ValueList {
    fn get(self, r: &CubeRecipe) -> &Vec<CubeValue> {
        match self {
            ValueList::Items => &r.items,
            ValueList::Rewards => &r.rewards,
        }
    }
    fn get_mut(self, r: &mut CubeRecipe) -> &mut Vec<CubeValue> {
        match self {
            ValueList::Items => &mut r.items,
            ValueList::Rewards => &mut r.rewards,
        }
    }
}

#[component]
fn ValueListEditor(
    title: &'static str,
    list: ValueList,
    selected: Memo<Option<CubeRecipe>>,
    #[prop(into)] update: Callback<Box<dyn FnOnce(&mut CubeRecipe) + Send + Sync>>,
    icons: Icons,
) -> impl IntoView {
    let len = Memo::new(move |_| selected.with(|s| s.as_ref().map(|r| list.get(r).len()).unwrap_or(0)));
    view! {
        <div class="space-y-2">
            <h3 class="text-sm font-medium text-muted-foreground">{title} " (" {move || len.get()} ")"</h3>
            <For each=move || 0..len.get() key=|i| *i let:i>
                {
                    let value = move || selected.with(|s| s.as_ref().and_then(|r| list.get(r).get(i).cloned()));
                    view! {
                        <div class="flex items-center gap-2 rounded-md border border-border p-2">
                            <EntityLabel vnum=Signal::derive(move || value().map(|v| v.vnum).unwrap_or(0.0)) kind=EntityKind::Item icons=icons/>
                            <Field label="Anzahl">
                                <NumInput
                                    value=Signal::derive(move || value().map(|v| v.count).unwrap_or(0.0))
                                    on_change=move |count: f64| {
                                        update.run(Box::new(move |r: &mut CubeRecipe| {
                                            if let Some(v) = list.get_mut(r).get_mut(i) {
                                                v.count = count;
                                            }
                                        }))
                                    }
                                    class="w-20 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                />
                            </Field>
                            <Button
                                variant=ButtonVariant::Outline
                                size=ButtonSize::Sm
                                on_click=move |_| {
                                    update.run(Box::new(move |r: &mut CubeRecipe| {
                                        let v = list.get_mut(r);
                                        if i < v.len() {
                                            v.remove(i);
                                        }
                                    }))
                                }
                            >
                                <Icon name="Trash2" class="size-3.5"/>
                            </Button>
                        </div>
                    }
                }
            </For>
            <AddPicker
                kind=EntityKind::Item
                on_pick=move |vnum: f64| {
                    update.run(Box::new(move |r: &mut CubeRecipe| list.get_mut(r).push(CubeValue { vnum, count: 1.0 })))
                }
            />
        </div>
    }
}

#[component]
pub fn CubeEditor() -> impl IntoView {
    let recipes = RwSignal::new(None::<Vec<CubeRecipe>>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let selected_index = RwSignal::new(None::<usize>);
    let icons: Icons = RwSignal::new(HashMap::new());

    let delete_confirm = RwSignal::new(None::<usize>);
    let saving = RwSignal::new(false);
    let save_confirm = RwSignal::new(false);
    let undo_confirm = RwSignal::new(false);
    let undoing = RwSignal::new(false);

    let loaded_snapshot = RwSignal::new(None::<Vec<CubeRecipe>>);
    let dirty = Memo::new(move |_| {
        recipes.with(|r| r.is_some()) && recipes.with(|r| loaded_snapshot.with(|s| r != s))
    });
    Effect::new(move |_| report_section_dirty(Section::CubeEditor, dirty.get()));
    on_cleanup(|| report_section_dirty(Section::CubeEditor, false));

    let load = move || {
        selected_index.set(None);
        loading.set(true);
        load_error.set(None);
        spawn_local(async move {
            match invoke0::<Vec<CubeRecipe>>("read_cube_file").await {
                Ok(loaded) => {
                    loaded_snapshot.set(Some(loaded.clone()));
                    recipes.set(Some(loaded));
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };
    load();

    let update_recipe = move |index: usize, f: Box<dyn FnOnce(&mut CubeRecipe) + Send + Sync>| {
        recipes.update(|r| {
            if let Some(rec) = r.as_mut().and_then(|v| v.get_mut(index)) {
                f(rec);
            }
        });
    };

    let add_recipe = move || {
        let len = recipes.with_untracked(|r| r.as_ref().map(|v| v.len()).unwrap_or(0));
        recipes.update(|r| {
            r.get_or_insert_with(Vec::new).push(CubeRecipe {
                npc_vnums: vec![],
                items: vec![],
                rewards: vec![],
                percent: 100.0,
                gold: 0.0,
            })
        });
        selected_index.set(Some(len));
    };

    let delete_recipe = move |index: usize| {
        recipes.update(|r| {
            if let Some(v) = r {
                if index < v.len() {
                    v.remove(index);
                }
            }
        });
        selected_index.set(None);
        delete_confirm.set(None);
    };

    let save = move || {
        save_confirm.set(false);
        let current = recipes.get_untracked();
        saving.set(true);
        spawn_local(async move {
            match invoke::<Option<String>>("write_cube_file", &json!({ "recipes": current })).await {
                Ok(backup) => {
                    toast::success(&match backup {
                        Some(b) if !b.is_empty() => format!("Gespeichert (Backup: {b})"),
                        _ => "Gespeichert.".to_string(),
                    });
                    let n = current.as_ref().map(|v| v.len()).unwrap_or(0);
                    loaded_snapshot.set(current);
                    log_activity(
                        Section::CubeEditor.into(),
                        "save",
                        &format!("cube.txt gespeichert ({n} Rezept(e))"),
                        Some("file"),
                        None,
                    );
                }
                Err(e) => toast::error(&e),
            }
            saving.set(false);
        });
    };

    use_save_shortcut(Section::CubeEditor, move || dirty.get_untracked() && !saving.get_untracked(), move || save_confirm.set(true));

    // "Letzte Änderung rückgängig machen" - jedes Speichern legt serverseitig
    // schon ein Backup an (siehe write_cube_file), das restauriert
    // undo_cube_write hier nur noch.
    let undo = move || {
        undo_confirm.set(false);
        undoing.set(true);
        spawn_local(async move {
            match invoke0::<String>("undo_cube_write").await {
                Ok(backup_path) => {
                    toast::success(&format!("Wiederhergestellt aus: {backup_path}"));
                    log_activity(
                        Section::CubeEditor.into(),
                        "restore",
                        &format!("cube.txt auf vorherigen Stand zurückgesetzt ({backup_path})"),
                        Some("file"),
                        None,
                    );
                    load();
                }
                Err(e) => toast::error(&e),
            }
            undoing.set(false);
        });
    };

    let selected = Memo::new(move |_| {
        selected_index.get().and_then(|i| recipes.with(|r| r.as_ref().and_then(|v| v.get(i).cloned())))
    });
    let update_selected = Callback::new(move |f: Box<dyn FnOnce(&mut CubeRecipe) + Send + Sync>| {
        if let Some(i) = selected_index.get_untracked() {
            update_recipe(i, f);
        }
    });
    let npc_len = Memo::new(move |_| selected.with(|s| s.as_ref().map(|r| r.npc_vnums.len()).unwrap_or(0)));
    let no_recipes = Signal::derive(move || recipes.with(|r| r.is_none()));

    view! {
        <div class="space-y-6 pb-10">
            <PageHeader module=Section::CubeEditor title="Cube-Editor"/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Bearbeitet " <code>"cube.txt"</code>
                " - die Rezepte für das \"Cube\" (Verwandlung/Kombination) an einem NPC: bestimmte NPCs nehmen Material-Items entgegen und geben mit einer Erfolgschance eine Belohnung zurück, optional gegen Gold."
            </p>
            <p class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Anders als der Kisten-/Aufwertungs-Editor braucht " <code>"cube.txt"</code>
                    " laut Server-Quellcode" <strong>" keinen Neustart"</strong> " - der Ingame-GM-Befehl "
                    <code>"/reload c"</code> " lädt die Rezepte sofort neu (" <code>"Cube_init()"</code>
                    "). Das genaue Datei-Format wurde diese Session nur gegen den Quellcode geprüft, nicht gegen eine echte "
                    <code>"cube.txt"</code>
                    " byte-verifiziert - beim ersten Speichern also die Datei vor/nach dem Schreiben einmal gegenkontrollieren."
                </span>
            </p>

            <div class="flex items-center gap-2">
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon
                        name="RefreshCw"
                        class=Signal::derive(move || format!("size-4 {}", if loading.get() { "animate-spin" } else { "" }))
                    />
                    " Neu laden"
                </Button>
                <Button variant=ButtonVariant::Outline on_click=move |_| add_recipe() disabled=no_recipes>
                    <Icon name="Plus" class="size-4"/>
                    " Neues Rezept"
                </Button>
                <Button
                    on_click=move |_| save_confirm.set(true)
                    disabled=Signal::derive(move || saving.get() || no_recipes.get())
                >
                    {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                </Button>
                <Button
                    variant=ButtonVariant::Outline
                    on_click=move |_| undo_confirm.set(true)
                    disabled=Signal::derive(move || undoing.get() || no_recipes.get())
                >
                    <Icon name="Undo2" class="size-4"/>
                    {move || if undoing.get() { " Setze zurück…" } else { " Letzte Änderung rückgängig machen" }}
                </Button>
            </div>
            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <Show when=move || !no_recipes.get()>
                <div class="flex gap-4">
                    <div class="w-64 shrink-0 space-y-1 rounded-lg border border-border p-2">
                        <Show when=move || recipes.with(|r| r.as_ref().is_some_and(|v| v.is_empty()))>
                            <p class="p-2 text-sm text-muted-foreground">"Keine Rezepte."</p>
                        </Show>
                        {move || {
                            recipes
                                .get()
                                .unwrap_or_default()
                                .into_iter()
                                .enumerate()
                                .map(|(i, r)| {
                                    let reward = r.rewards.first().map(|v| format!(" → #{}", js_number_string(v.vnum)));
                                    view! {
                                        <button
                                            on:click=move |_| selected_index.set(Some(i))
                                            class=move || format!(
                                                "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm {}",
                                                if selected_index.get() == Some(i) { "bg-primary text-primary-foreground" } else { "hover:bg-muted" },
                                            )
                                        >
                                            <Icon name="Beaker" class="size-4 shrink-0"/>
                                            <span class="flex-1 truncate">"Rezept #" {i + 1} {reward}</span>
                                            <span class="text-xs opacity-70">{js_number_string(r.percent)} "%"</span>
                                        </button>
                                    }
                                })
                                .collect_view()
                        }}
                    </div>

                    <div class="flex-1 space-y-4 rounded-lg border border-border p-4">
                        <Show
                            when=move || selected.with(|s| s.is_some())
                            fallback=|| view! { <p class="text-sm text-muted-foreground">"Rezept links auswählen oder neu anlegen."</p> }
                        >
                            <div class="flex flex-wrap items-end gap-3">
                                <Field label="Erfolgschance (%)">
                                    <NumInput
                                        min="0"
                                        max="100"
                                        value=Signal::derive(move || selected.with(|s| s.as_ref().map(|r| r.percent).unwrap_or(0.0)))
                                        on_change=move |v: f64| update_selected.run(Box::new(move |r: &mut CubeRecipe| r.percent = v))
                                        class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </Field>
                                <Field label="Gold-Kosten">
                                    <NumInput
                                        min="0"
                                        value=Signal::derive(move || selected.with(|s| s.as_ref().map(|r| r.gold).unwrap_or(0.0)))
                                        on_change=move |v: f64| update_selected.run(Box::new(move |r: &mut CubeRecipe| r.gold = v))
                                        class="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                </Field>
                                <Button
                                    variant=ButtonVariant::Destructive
                                    size=ButtonSize::Sm
                                    on_click=move |_| delete_confirm.set(selected_index.get_untracked())
                                >
                                    <Icon name="Trash2" class="size-3.5"/>
                                    " Rezept löschen"
                                </Button>
                            </div>

                            <div class="space-y-2">
                                <h3 class="text-sm font-medium text-muted-foreground">
                                    "Anbietende NPCs (" {move || npc_len.get()} ")"
                                </h3>
                                <For each=move || 0..npc_len.get() key=|i| *i let:i>
                                    <div class="flex items-center gap-2 rounded-md border border-border p-2">
                                        <EntityLabel
                                            vnum=Signal::derive(move || selected.with(|s| s.as_ref().and_then(|r| r.npc_vnums.get(i).copied()).unwrap_or(0.0)))
                                            kind=EntityKind::Mob
                                            icons=icons
                                        />
                                        <Button
                                            variant=ButtonVariant::Outline
                                            size=ButtonSize::Sm
                                            on_click=move |_| update_selected.run(Box::new(move |r: &mut CubeRecipe| {
                                                if i < r.npc_vnums.len() {
                                                    r.npc_vnums.remove(i);
                                                }
                                            }))
                                        >
                                            <Icon name="Trash2" class="size-3.5"/>
                                        </Button>
                                    </div>
                                </For>
                                <AddPicker
                                    kind=EntityKind::Mob
                                    on_pick=move |vnum: f64| update_selected.run(Box::new(move |r: &mut CubeRecipe| r.npc_vnums.push(vnum)))
                                />
                            </div>

                            <ValueListEditor title="Benötigte Materialien" list=ValueList::Items selected=selected update=update_selected icons=icons/>
                            <ValueListEditor title="Belohnung" list=ValueList::Rewards selected=selected update=update_selected icons=icons/>
                        </Show>
                    </div>
                </div>
            </Show>

            {move || delete_confirm.get().map(|idx| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-center gap-2 text-sm font-medium">
                            <Icon name="AlertTriangle" class="size-4 text-destructive"/>
                            {format!(" Rezept #{} wirklich entfernen?", idx + 1)}
                        </p>
                        <p class="text-xs text-muted-foreground">
                            "Wird erst beim \"Speichern\" wirklich auf den Server geschrieben - bis dahin rückgängig machbar durch nicht speichern/neu laden."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(None)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| delete_recipe(idx)>"Entfernen"</Button>
                        </div>
                    </div>
                </div>
            })}

            <Show when=move || save_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">"cube.txt jetzt überschreiben?"</p>
                        <p class="text-xs text-muted-foreground">"Ein Backup der aktuellen Datei wird vorher auf dem Server angelegt."</p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| save_confirm.set(false)>"Abbrechen"</Button>
                            <Button on_click=move |_| save()>"Speichern"</Button>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when=move || undo_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm font-medium">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            "Letzte gespeicherte Änderung an cube.txt rückgängig machen?"
                        </p>
                        <p class="text-xs text-muted-foreground">
                            "Stellt die zuletzt gesicherte Vorgängerversion wieder her (der aktuelle Stand wird dabei selbst zuerst gesichert). Betrifft nur, was zuletzt tatsächlich gespeichert wurde - nicht unfertige, noch ungespeicherte Eingaben hier im Editor."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| undo_confirm.set(false)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| undo() disabled=undoing>
                                {move || if undoing.get() { "Setze zurück…" } else { "Rückgängig machen" }}
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
