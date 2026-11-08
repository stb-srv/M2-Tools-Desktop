//! common_drop_item.txt - Live gegen den echten Dev-Server verifiziert
//! (2026-08-18): die Datei benutzt auf diesem Server dieselbe
//! Group/Mob/Type-Grammatik wie mob_drop_item.txt (nicht das im generischen
//! Server-Quellcode gefundene Rang-Level-Bracket-Format). Dieser Editor ist
//! deshalb bewusst eine schlankere Variante des Mob-Drop-Editors (gleiche
//! Datenstruktur, weniger Komfortfunktionen wie Bulk-Edit/CSV-Export).

use super::join::join_all;
use super::ItemProtoName;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::mob_drop_editor::components::shared::{clamp_percent, fmt_num, js_number, js_number_or, num_sig, NumInput, MAX_PERCENT_ATTR};
use crate::features::mob_drop_editor::types::{MobDropGroup, MobDropItem};
use crate::features::shared::drop_chance::format_real_drop_chance;
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use crate::util::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};

fn empty_group() -> MobDropGroup {
    MobDropGroup { name: String::new(), mob_vnum: 0.0, drop_type: "drop".into(), items: vec![] }
}

#[component]
pub fn CommonDropEditor() -> impl IntoView {
    let groups = RwSignal::new(None::<Vec<MobDropGroup>>);
    let labels = RwSignal::new(HashMap::<i64, String>::new());
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let selected_index = RwSignal::new(None::<usize>);
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let confirm_open = RwSignal::new(false);
    let delete_confirm = RwSignal::new(None::<usize>);
    let item_picker = RwSignal::new(false);
    let creating = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let new_mob_vnum = RwSignal::new(String::new());

    let loaded_snapshot = RwSignal::new(None::<Vec<MobDropGroup>>);
    let dirty = Memo::new(move |_| groups.with(|g| g.is_some() && loaded_snapshot.with(|s| s != g)));
    Effect::new(move |_| report_section_dirty(Section::DropGenerator, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::DropGenerator, false));

    let with_group = move |i: usize| groups.with(|g| g.as_ref().and_then(|g| g.get(i)).cloned());

    let resolve_labels = move |list: Vec<MobDropGroup>| async move {
        let vnums: BTreeSet<i64> = list.iter().flat_map(|g| g.items.iter().map(|i| i.item_vnum as i64)).collect();
        let entries = join_all(vnums.into_iter().map(|vnum| async move {
            let label = match invoke::<Option<ItemProtoName>>("get_item_proto", &json!({ "vnum": vnum })).await {
                Ok(Some(proto)) => proto.locale_name,
                _ => format!("#{vnum} (nicht gefunden!)"),
            };
            (vnum, label)
        }))
        .await;
        labels.set(entries.into_iter().collect());
    };

    let load = move || {
        loading.set(true);
        load_error.set(None);
        save_ok.set(false);
        selected_index.set(None);
        spawn_local(async move {
            match invoke::<Vec<MobDropGroup>>("read_common_drop_file", &json!({})).await {
                Ok(list) => {
                    groups.set(Some(list.clone()));
                    loaded_snapshot.set(Some(list.clone()));
                    resolve_labels(list).await;
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let update_group = move |idx: usize, f: Box<dyn FnOnce(&mut MobDropGroup)>| {
        if groups.with_untracked(|g| g.is_none()) {
            return;
        }
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(idx)) {
                f(g)
            }
        });
        save_ok.set(false);
    };

    let update_item = move |gi: usize, ii: usize, f: Box<dyn FnOnce(&mut MobDropItem)>| {
        if groups.with_untracked(|g| g.is_none()) {
            return;
        }
        groups.update(|g| {
            if let Some(it) = g.as_mut().and_then(|g| g.get_mut(gi)).and_then(|g| g.items.get_mut(ii)) {
                f(it)
            }
        });
        save_ok.set(false);
    };

    let remove_item = move |gi: usize, ii: usize| {
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(gi)) {
                if ii < g.items.len() {
                    g.items.remove(ii);
                }
            }
        });
    };

    let add_item = move |row: EntityRow| {
        let Some(si) = selected_index.get_untracked() else { return };
        if groups.with_untracked(|g| g.is_none()) {
            return;
        }
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(si)) {
                g.items.push(MobDropItem { item_vnum: row.vnum as f64, count: 1.0, percent: 0.0 });
            }
        });
        labels.update(|l| {
            l.insert(row.vnum, row.name.clone());
        });
        item_picker.set(false);
    };

    let submit_new_group = move || {
        let mob_vnum = js_number(&new_mob_vnum.get_untracked());
        let name = new_name.get_untracked();
        if name.trim().is_empty() || !mob_vnum.is_finite() || mob_vnum <= 0.0 {
            return;
        }
        let len = groups.with_untracked(|g| g.as_ref().map(|g| g.len()));
        groups.update(|g| {
            g.get_or_insert_with(Vec::new).push(MobDropGroup { name: name.trim().to_string(), mob_vnum, ..empty_group() })
        });
        selected_index.set(Some(len.unwrap_or(0)));
        creating.set(false);
        new_name.set(String::new());
        new_mob_vnum.set(String::new());
    };

    let confirm_delete_group = move || {
        let Some(dc) = delete_confirm.get_untracked() else { return };
        if groups.with_untracked(|g| g.is_none()) {
            return;
        }
        groups.update(|g| {
            if let Some(g) = g.as_mut() {
                if dc < g.len() {
                    g.remove(dc);
                }
            }
        });
        match selected_index.get_untracked() {
            Some(si) if si == dc => selected_index.set(None),
            Some(si) if si > dc => selected_index.set(Some(si - 1)),
            _ => {}
        }
        delete_confirm.set(None);
    };

    let save = move || {
        let Some(gs) = groups.get_untracked() else { return };
        confirm_open.set(false);
        saving.set(true);
        save_error.set(None);
        spawn_local(async move {
            match invoke::<Option<String>>("write_common_drop_file", &json!({ "groups": gs })).await {
                Ok(_) => {
                    save_ok.set(true);
                    let n = gs.len();
                    loaded_snapshot.set(Some(gs));
                    log_activity(
                        Section::DropGenerator.into(),
                        "save",
                        &format!("common_drop_item.txt gespeichert ({n} Gruppe(n))"),
                        Some("common_drop_item.txt"),
                        None,
                    );
                }
                Err(e) => save_error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    use_save_shortcut(
        Section::DropGenerator,
        move || dirty.get_untracked() && !saving.get_untracked(),
        move || confirm_open.set(true),
    );

    let has_groups = Memo::new(move |_| groups.with(|g| g.is_some()));
    let group_count = move || groups.with(|g| g.as_ref().map(|g| g.len()).unwrap_or(0));
    let selected = Memo::new(move |_| selected_index.get().filter(|i| groups.with(|g| g.as_ref().is_some_and(|g| *i < g.len()))));

    view! {
        <div class="space-y-4 pb-10">
            <div class="rounded-lg border border-border p-3 text-sm text-muted-foreground">
                <p class="mb-1 font-medium text-foreground">"common_drop_item.txt — zusätzliche Mob-Drops"</p>
                <p>
                    "Live gegen diesen Server geprüft: die Datei benutzt hier dasselbe Format wie mob_drop_item.txt ("
                    <code>"Group / Mob / Type / Item-Zeilen"</code>
                    "), pro Mob-VNUM - nicht das generische Rang-Level-Format, das eine erste Version dieses Editors angenommen hatte. Beispiel, das bereits auf dem Server liegt: Gruppe \"MetinStein1\" für Mob 8001 mit Item 19, 100% Chance. In der Praxis eine zweite, separat gepflegte Drop-Tabelle - z.B. für Mobs, die nicht in mob_drop_item.txt gepflegt werden sollen."
                </p>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Ob diese Datei wie mob_drop_item.txt live per GM-Befehl neu ladbar ist, wurde nicht separat geprüft - im Zweifel nach dem Speichern einen Server-Neustart einplanen."
                </span>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-border bg-muted/30 p-3 text-sm text-muted-foreground">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Prozentwerte vermutlich nach derselben Formel wie mob_drop_item.txt (≈ Datei-Wert ÷ 4 reale Chance) - nicht separat verifiziert, da dieselbe Datenstruktur denselben Lade-/Auswerte-Code nahelegt."
                </span>
            </div>

            <div class="flex gap-2">
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon
                        name="RefreshCw"
                        class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })
                    />
                    "Laden"
                </Button>
                <Show when=move || has_groups.get()>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| creating.set(true)>
                        <Icon name="Plus" class="size-3.5"/>
                        "Mob hinzufügen"
                    </Button>
                </Show>
            </div>

            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <Show when=move || has_groups.get()>
                <div class="flex gap-4">
                    <div class="w-64 shrink-0 space-y-1">
                        <For each=move || 0..group_count() key=|i| *i let:idx>
                            <div
                                on:click=move |_| selected_index.set(Some(idx))
                                class=move || {
                                    if selected_index.get() == Some(idx) {
                                        "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted bg-muted"
                                    } else {
                                        "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted "
                                    }
                                }
                            >
                                <div class="font-medium">{move || with_group(idx).map(|g| g.name).unwrap_or_default()}</div>
                                <div class="text-xs text-muted-foreground">
                                    {move || with_group(idx).map(|g| format!("Mob: {} · {} Drops", fmt_num(g.mob_vnum), g.items.len())).unwrap_or_default()}
                                </div>
                            </div>
                        </For>
                        <Show when=move || group_count() == 0>
                            <p class="p-2 text-sm text-muted-foreground">"Keine Einträge."</p>
                        </Show>
                    </div>

                    <div class="min-w-0 flex-1 space-y-3">
                        <Show when=move || selected.get().is_none()>
                            <p class="text-sm text-muted-foreground">"Wähle links einen Eintrag aus."</p>
                        </Show>
                        {move || selected.get().map(|si| {
                            let item_count = move || groups.with(|g| g.as_ref().and_then(|g| g.get(si)).map(|g| g.items.len()).unwrap_or(0));
                            let item_field = move |ii: usize, f: fn(&MobDropItem) -> f64| {
                                groups.with(|g| g.as_ref().and_then(|g| g.get(si)).and_then(|g| g.items.get(ii)).map(f)).unwrap_or(0.0)
                            };
                            view! {
                                <div class="flex items-start justify-between rounded-lg border border-border p-3">
                                    <div class="flex flex-wrap items-end gap-3">
                                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                            "Gruppenname"
                                            <input
                                                prop:value=move || with_group(si).map(|g| g.name).unwrap_or_default()
                                                on:input=move |e| {
                                                    let v = event_target_value(&e);
                                                    update_group(si, Box::new(move |g| g.name = v));
                                                }
                                                class="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                            />
                                        </label>
                                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                            "Mob-VNUM"
                                            <NumInput
                                                value=num_sig(move || with_group(si).map(|g| g.mob_vnum).unwrap_or(0.0))
                                                on_input=move |v: String| update_group(si, Box::new(move |g| g.mob_vnum = js_number_or(&v, 0.0)))
                                                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                            />
                                        </label>
                                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                            "Type"
                                            <input
                                                prop:value=move || with_group(si).map(|g| g.drop_type).unwrap_or_default()
                                                on:input=move |e| {
                                                    let v = event_target_value(&e);
                                                    update_group(si, Box::new(move |g| g.drop_type = v));
                                                }
                                                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                            />
                                        </label>
                                    </div>
                                    <Button variant=ButtonVariant::Destructive size=ButtonSize::Sm on_click=move |_| delete_confirm.set(Some(si))>
                                        <Icon name="Trash2" class="size-4"/>
                                        "Löschen"
                                    </Button>
                                </div>

                                <div class="space-y-2 rounded-lg border border-border p-3">
                                    <div class="flex items-center justify-between">
                                        <h2 class="text-sm font-medium text-muted-foreground">{move || format!("Drops ({})", item_count())}</h2>
                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| item_picker.set(true)>
                                            <Icon name="Plus" class="size-3.5"/>
                                            "Item hinzufügen"
                                        </Button>
                                    </div>
                                    <For each=move || 0..item_count() key=|i| *i let:ii>
                                        <div class="flex items-center gap-3 rounded-md border border-border p-2">
                                            <span class="w-40 shrink-0 truncate text-xs text-muted-foreground">
                                                {move || {
                                                    let vnum = item_field(ii, |i| i.item_vnum);
                                                    labels.with(|l| l.get(&(vnum as i64)).cloned()).unwrap_or_else(|| format!("#{}", fmt_num(vnum)))
                                                }}
                                            </span>
                                            <label class="flex flex-col gap-0.5 text-xs text-muted-foreground">
                                                "Anzahl"
                                                <NumInput
                                                    min="1"
                                                    value=num_sig(move || item_field(ii, |i| i.count))
                                                    on_input=move |v: String| update_item(si, ii, Box::new(move |it| it.count = 1f64.max(js_number_or(&v, 1.0))))
                                                    class="w-16 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                                />
                                            </label>
                                            <label class="flex flex-col gap-0.5 text-xs text-muted-foreground">
                                                "Prozent"
                                                <NumInput
                                                    min="0"
                                                    max=MAX_PERCENT_ATTR
                                                    step="0.01"
                                                    value=num_sig(move || item_field(ii, |i| i.percent))
                                                    on_input=move |v: String| update_item(si, ii, Box::new(move |it| it.percent = clamp_percent(js_number_or(&v, 0.0))))
                                                    class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                                />
                                            </label>
                                            <span class="text-xs text-muted-foreground">
                                                {move || format!("≈{}", format_real_drop_chance(item_field(ii, |i| i.percent)))}
                                            </span>
                                            <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm class="ml-auto" on_click=move |_| remove_item(si, ii)>
                                                <Icon name="X" class="size-4"/>
                                            </Button>
                                        </div>
                                    </For>
                                    <Show when=move || item_count() == 0>
                                        <p class="text-sm text-muted-foreground">"Noch keine Drops."</p>
                                    </Show>
                                </div>
                            }
                        })}
                    </div>
                </div>
            </Show>

            <Show when=move || has_groups.get()>
                {move || save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <Show when=move || save_ok.get()>
                    <p class="flex items-center gap-1 text-sm text-green-600">
                        <Icon name="CheckCircle2" class="size-4"/>
                        " Gespeichert."
                    </p>
                </Show>
                <Button disabled=saving on_click=move |_| confirm_open.set(true)>
                    {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                </Button>
            </Show>

            <Show when=move || item_picker.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">"Item wählen"</p>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| item_picker.set(false)>
                                "Schließen"
                            </Button>
                        </div>
                        <EntityBrowser kind=EntityKind::Item auto_focus=true on_pick=add_item/>
                    </div>
                </div>
            </Show>

            <Show when=move || creating.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">"Neuen Eintrag anlegen"</p>
                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                            "Gruppenname"
                            <input
                                autofocus
                                prop:value=move || new_name.get()
                                on:input=move |e| new_name.set(event_target_value(&e))
                                class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </label>
                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                            "Mob-VNUM"
                            <input
                                type="number"
                                prop:value=move || new_mob_vnum.get()
                                on:input=move |e| new_mob_vnum.set(event_target_value(&e))
                                class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </label>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| creating.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button
                                disabled=Signal::derive(move || new_name.with(|n| n.trim().is_empty()) || new_mob_vnum.with(|v| v.is_empty()))
                                on_click=move |_| submit_new_group()
                            >
                                "Anlegen"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>

            {move || delete_confirm.get().filter(|_| has_groups.get()).map(|dc| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm">
                            "Eintrag "
                            <strong>{move || with_group(dc).map(|g| g.name).unwrap_or_default()}</strong>
                            " wirklich entfernen? Wird erst beim Speichern in die Datei übernommen."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| confirm_delete_group()>
                                "Entfernen"
                            </Button>
                        </div>
                    </div>
                </div>
            })}

            <Show when=move || confirm_open.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>"common_drop_item.txt wird komplett überschrieben (Backup wird vorher angelegt)."</span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_open.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button on_click=move |_| save()>"Speichern"</Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
