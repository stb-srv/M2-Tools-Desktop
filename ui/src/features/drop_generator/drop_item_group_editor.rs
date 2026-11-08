//! drop_item_group.txt - Zufalls-Gruppen pro Mob.

use super::join::join_all;
use super::ItemProtoName;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::mob_drop_editor::components::shared::{fmt_num, js_number, num_sig, NumInput};
use crate::features::shared::drop_chance::format_real_drop_chance;
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DropGroupItem {
    pub item_ref: String,
    pub percent: f64,
    pub count: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DropItemGroup {
    pub name: String,
    pub mob_vnum: f64,
    pub items: Vec<DropGroupItem>,
}

#[derive(Deserialize)]
struct TableRows {
    columns: Vec<String>,
    rows: Vec<Vec<Option<String>>>,
}

fn empty_group() -> DropItemGroup {
    DropItemGroup { name: String::new(), mob_vnum: 0.0, items: vec![] }
}

async fn resolve_mob_label(vnum: f64) -> String {
    let res = invoke::<TableRows>(
        "search_table_rows",
        &json!({ "database": "player", "table": "mob_proto", "column": "vnum", "query": fmt_num(vnum) }),
    )
    .await;
    let not_found = format!("{} (nicht gefunden!)", fmt_num(vnum));
    let Ok(result) = res else { return not_found };
    let idx = result.columns.iter().position(|c| c == "locale_name");
    let vnum_idx = result.columns.iter().position(|c| c == "vnum");
    let row = result.rows.iter().find(|r| {
        let cell = vnum_idx.and_then(|i| r.get(i));
        let n = match cell {
            None => f64::NAN,
            Some(None) => 0.0,
            Some(Some(s)) => js_number(s),
        };
        n == vnum
    });
    match (row, idx) {
        (Some(row), Some(i)) => match row.get(i).cloned().flatten() {
            Some(name) if !name.is_empty() => format!("{name} ({})", fmt_num(vnum)),
            _ => not_found,
        },
        _ => not_found,
    }
}

/// Mob-VNUMs (> 0), die mehr als einmal vorkommen.
pub fn duplicate_mob_vnums(groups: &[DropItemGroup]) -> HashSet<i64> {
    let vnums: Vec<f64> = groups.iter().map(|g| g.mob_vnum).collect();
    vnums
        .iter()
        .enumerate()
        .filter(|(i, v)| **v > 0.0 && vnums.iter().position(|x| x == *v) != Some(*i))
        .map(|(_, v)| *v as i64)
        .collect()
}

pub fn can_save(groups: Option<&[DropItemGroup]>) -> bool {
    let Some(groups) = groups else { return false };
    !groups.is_empty()
        && groups.iter().all(|g| {
            !g.name.trim().is_empty()
                && g.mob_vnum > 0.0
                && !g.items.is_empty()
                && g.items.iter().all(|it| !it.item_ref.trim().is_empty())
        })
        && duplicate_mob_vnums(groups).is_empty()
}

#[component]
pub fn DropItemGroupEditor() -> impl IntoView {
    let groups = RwSignal::new(None::<Vec<DropItemGroup>>);
    let item_labels = RwSignal::new(HashMap::<String, String>::new());
    let mob_labels = RwSignal::new(HashMap::<i64, String>::new());
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let confirm_open = RwSignal::new(false);
    let mob_picker_for = RwSignal::new(None::<usize>);
    let item_picker_for = RwSignal::new(None::<(usize, usize)>);

    let loaded_snapshot = RwSignal::new(None::<Vec<DropItemGroup>>);
    let dirty = Memo::new(move |_| groups.with(|g| g.is_some() && loaded_snapshot.with(|s| s != g)));
    Effect::new(move |_| report_section_dirty(Section::DropGenerator, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::DropGenerator, false));

    let resolve_all_labels = move |list: Vec<DropItemGroup>| async move {
        let mut seen = Vec::<f64>::new();
        for g in &list {
            if !seen.contains(&g.mob_vnum) {
                seen.push(g.mob_vnum);
            }
        }
        let mob_entries = join_all(seen.into_iter().map(|v| async move { (v as i64, resolve_mob_label(v).await) })).await;
        mob_labels.set(mob_entries.into_iter().collect());

        let item_vnums: BTreeSet<String> = list
            .iter()
            .flat_map(|g| g.items.iter().map(|it| it.item_ref.trim().to_string()))
            .filter(|s| !s.is_empty())
            .collect();
        let item_entries = join_all(item_vnums.into_iter().map(|v| async move {
            let label = match invoke::<Option<ItemProtoName>>("get_item_proto", &json!({ "vnum": js_number(&v) })).await {
                Ok(Some(proto)) => format!("{} ({v})", proto.locale_name),
                _ => format!("{v} (nicht gefunden!)"),
            };
            (v, label)
        }))
        .await;
        item_labels.set(item_entries.into_iter().collect());
    };

    let load = move || {
        loading.set(true);
        load_error.set(None);
        save_ok.set(false);
        spawn_local(async move {
            match invoke::<Vec<DropItemGroup>>("read_drop_item_group_file", &json!({})).await {
                Ok(list) => {
                    groups.set(Some(list.clone()));
                    loaded_snapshot.set(Some(list.clone()));
                    resolve_all_labels(list).await;
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let update_group = move |idx: usize, f: Box<dyn FnOnce(&mut DropItemGroup)>| {
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

    let add_group = move || groups.update(|g| g.get_or_insert_with(Vec::new).push(empty_group()));

    let remove_group = move |idx: usize| {
        groups.update(|g| {
            if let Some(g) = g.as_mut() {
                if idx < g.len() {
                    g.remove(idx);
                }
            }
        })
    };

    let add_item = move |gi: usize| {
        if groups.with_untracked(|g| g.is_none()) {
            return;
        }
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(gi)) {
                g.items.push(DropGroupItem { item_ref: String::new(), percent: 1.0, count: 1.0 });
            }
        });
    };

    let update_item = move |gi: usize, ii: usize, f: Box<dyn FnOnce(&mut DropGroupItem)>| {
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

    let duplicates = Memo::new(move |_| groups.with(|g| duplicate_mob_vnums(g.as_deref().unwrap_or_default())));
    let can_save_m = Memo::new(move |_| groups.with(|g| can_save(g.as_deref())));

    let save = move || {
        let Some(gs) = groups.get_untracked() else { return };
        confirm_open.set(false);
        saving.set(true);
        save_error.set(None);
        spawn_local(async move {
            match invoke::<Option<String>>("write_drop_item_group_file", &json!({ "groups": gs })).await {
                Ok(_) => {
                    save_ok.set(true);
                    let n = gs.len();
                    loaded_snapshot.set(Some(gs));
                    log_activity(
                        Section::DropGenerator.into(),
                        "save",
                        &format!("drop_item_group.txt gespeichert ({n} Gruppe(n))"),
                        Some("drop_item_group.txt"),
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
        move || dirty.get_untracked() && can_save_m.get_untracked() && !saving.get_untracked(),
        move || confirm_open.set(true),
    );

    let has_groups = Memo::new(move |_| groups.with(|g| g.is_some()));
    let group_count = move || groups.with(|g| g.as_ref().map(|g| g.len()).unwrap_or(0));
    let gfield = move |gi: usize| groups.with(|g| g.as_ref().and_then(|g| g.get(gi)).map(|g| (g.name.clone(), g.mob_vnum, g.items.len())));
    let ifield = move |gi: usize, ii: usize| {
        groups.with(|g| g.as_ref().and_then(|g| g.get(gi)).and_then(|g| g.items.get(ii)).cloned())
    };

    let on_mob_pick = move |row: EntityRow| {
        let Some(idx) = mob_picker_for.get_untracked() else { return };
        update_group(idx, Box::new(move |g| g.mob_vnum = row.vnum as f64));
        mob_labels.update(|m| {
            m.insert(row.vnum, format!("{} ({})", row.name, row.vnum));
        });
        mob_picker_for.set(None);
    };

    let on_item_pick = move |row: EntityRow| {
        let Some((gi, ii)) = item_picker_for.get_untracked() else { return };
        let vnum_str = row.vnum.to_string();
        let v2 = vnum_str.clone();
        update_item(gi, ii, Box::new(move |it| it.item_ref = v2));
        item_labels.update(|m| {
            m.insert(vnum_str.clone(), format!("{} ({vnum_str})", row.name));
        });
        item_picker_for.set(None);
    };

    view! {
        <div class="space-y-4 pb-10">
            <div class="rounded-lg border border-border p-3 text-sm text-muted-foreground">
                <p class="mb-1 font-medium text-foreground">"drop_item_group.txt — Zufalls-Gruppen pro Mob"</p>
                <p>
                    "Jede Gruppe gehört zu genau einem Mob (über die Mob-VNUM) und enthält beliebig viele Items mit je einer eigenen Chance. Beispiel: Gruppe für Mob 101 mit \"Item A: 0,5%\" und \"Item B: 0,3%\" - bei einem Kill wird"
                    <strong>" jeder Eintrag einzeln gewürfelt"</strong>
                    ": es können also Item A und B gleichzeitig droppen, nur eines der beiden, oder auch keins."
                </p>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-border bg-muted/30 p-3 text-sm text-muted-foreground">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Unterschied zum \"Item-Gruppen\"-Tab (special_item_group.txt, Kisten-Loot): dort wird "
                    <strong>"ein"</strong>
                    " Treffer aus dem Pool gezogen. Hier gibt es keine Pool-Ziehung - jeder Eintrag ist ein eigener, unabhängiger Wurf."
                </span>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Wirkt erst nach einem kompletten Server-Neustart (kein " <code>"/reload"</code>
                    " dafür). Ein nicht auflösbares Item lässt den "
                    <strong>"kompletten Server beim nächsten Start nicht mehr hochfahren"</strong>
                    "."
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
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| add_group()>
                        <Icon name="Plus" class="size-3.5"/>
                        "Gruppe hinzufügen"
                    </Button>
                </Show>
            </div>

            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <Show when=move || has_groups.get()>
                <div class="space-y-3">
                    <For each=move || 0..group_count() key=|i| *i let:gi>
                        {
                            let item_count = move || gfield(gi).map(|g| g.2).unwrap_or(0);
                            let mob_vnum = move || gfield(gi).map(|g| g.1).unwrap_or(0.0);
                            view! {
                                <div class="space-y-2 rounded-lg border border-border p-3">
                                    <div class="flex flex-wrap items-center gap-2">
                                        <input
                                            prop:value=move || gfield(gi).map(|g| g.0).unwrap_or_default()
                                            on:input=move |e| {
                                                let v = event_target_value(&e);
                                                update_group(gi, Box::new(move |g| g.name = v));
                                            }
                                            placeholder="Gruppenname"
                                            class="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                        <button
                                            type="button"
                                            on:click=move |_| mob_picker_for.set(Some(gi))
                                            class="rounded-md border border-border bg-background px-2 py-1 text-left text-xs hover:bg-muted"
                                        >
                                            {move || {
                                                let v = mob_vnum();
                                                if v > 0.0 {
                                                    mob_labels.with(|m| m.get(&(v as i64)).cloned()).unwrap_or_else(|| fmt_num(v))
                                                } else {
                                                    "Mob wählen…".to_string()
                                                }
                                            }}
                                        </button>
                                        <div class="ml-auto">
                                            <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| remove_group(gi)>
                                                <Icon name="Trash2" class="size-3.5"/>
                                            </Button>
                                        </div>
                                    </div>
                                    <Show when=move || { let v = mob_vnum(); v > 0.0 && duplicates.with(|d| d.contains(&(v as i64))) }>
                                        <p class="text-xs text-destructive">
                                            "Diese Mob-VNUM wird bereits von einer anderen Gruppe benutzt - der Server würde beide beim nächsten Laden automatisch zu einer Gruppe zusammenführen. Bitte auf verschiedene Mobs aufteilen oder die Items in einer Gruppe zusammenfassen."
                                        </p>
                                    </Show>

                                    <table class="w-full text-sm">
                                        <thead>
                                            <tr class="text-xs text-muted-foreground">
                                                <th class="px-1 py-1 text-left font-medium">"Item"</th>
                                                <th class="px-1 py-1 text-left font-medium">"Prozent (Datei-Wert)"</th>
                                                <th class="px-1 py-1 text-left font-medium">"≈ real"</th>
                                                <th class="px-1 py-1 text-left font-medium">"Anzahl"</th>
                                                <th></th>
                                            </tr>
                                        </thead>
                                        <tbody>
                                            <For each=move || 0..item_count() key=|i| *i let:ii>
                                                <tr class="border-t border-border">
                                                    <td class="px-1 py-1">
                                                        <button
                                                            type="button"
                                                            on:click=move |_| item_picker_for.set(Some((gi, ii)))
                                                            class="rounded-md border border-border bg-background px-2 py-1 text-left text-xs hover:bg-muted"
                                                        >
                                                            {move || {
                                                                let r = ifield(gi, ii).map(|i| i.item_ref).unwrap_or_default();
                                                                if r.is_empty() {
                                                                    "Item wählen…".to_string()
                                                                } else {
                                                                    item_labels.with(|m| m.get(&r).cloned()).unwrap_or(r)
                                                                }
                                                            }}
                                                        </button>
                                                    </td>
                                                    <td class="px-1 py-1">
                                                        <NumInput
                                                            step="0.000001"
                                                            value=num_sig(move || ifield(gi, ii).map(|i| i.percent).unwrap_or(0.0))
                                                            on_input=move |v: String| update_item(gi, ii, Box::new(move |it| it.percent = js_number(&v)))
                                                            class="w-24 rounded-md border border-border bg-background px-1.5 py-1 text-xs"
                                                        />
                                                    </td>
                                                    <td class="px-1 py-1 text-xs text-muted-foreground">
                                                        {move || format_real_drop_chance(ifield(gi, ii).map(|i| i.percent).unwrap_or(0.0))}
                                                    </td>
                                                    <td class="px-1 py-1">
                                                        <NumInput
                                                            value=num_sig(move || ifield(gi, ii).map(|i| i.count).unwrap_or(0.0))
                                                            on_input=move |v: String| update_item(gi, ii, Box::new(move |it| it.count = js_number(&v)))
                                                            class="w-16 rounded-md border border-border bg-background px-1.5 py-1 text-xs"
                                                        />
                                                    </td>
                                                    <td class="px-1 py-1 text-right">
                                                        <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| remove_item(gi, ii)>
                                                            <Icon name="Trash2" class="size-3.5"/>
                                                        </Button>
                                                    </td>
                                                </tr>
                                            </For>
                                        </tbody>
                                    </table>
                                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| add_item(gi)>
                                        <Icon name="Plus" class="size-3.5"/>
                                        "Item hinzufügen"
                                    </Button>
                                </div>
                            }
                        }
                    </For>

                    {move || save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    <Show when=move || save_ok.get()>
                        <p class="flex items-center gap-1 text-sm text-green-600">
                            <Icon name="CheckCircle2" class="size-4"/>
                            " Gespeichert."
                        </p>
                    </Show>
                    <Button
                        disabled=Signal::derive(move || !can_save_m.get() || saving.get())
                        on_click=move |_| confirm_open.set(true)
                    >
                        {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                    </Button>
                    <Show when=move || !can_save_m.get()>
                        <p class="text-xs text-destructive">
                            "Jede Gruppe braucht Name, Mob, mindestens ein Item und darf keine bereits verwendete Mob-VNUM wiederholen."
                        </p>
                    </Show>
                </div>
            </Show>

            <Show when=move || mob_picker_for.with(|m| m.is_some())>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">"Mob wählen"</p>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| mob_picker_for.set(None)>
                                "Schließen"
                            </Button>
                        </div>
                        <EntityBrowser kind=EntityKind::Mob auto_focus=true on_pick=on_mob_pick/>
                    </div>
                </div>
            </Show>

            <Show when=move || item_picker_for.with(|m| m.is_some())>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">"Item wählen"</p>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| item_picker_for.set(None)>
                                "Schließen"
                            </Button>
                        </div>
                        <EntityBrowser kind=EntityKind::Item auto_focus=true on_pick=on_item_pick/>
                    </div>
                </div>
            </Show>

            <Show when=move || confirm_open.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                "drop_item_group.txt wird komplett überschrieben (Backup wird vorher angelegt). Wirkt erst nach einem Server-Neustart."
                            </span>
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

#[cfg(test)]
mod tests {
    use super::*;

    fn g(name: &str, mob: f64, refs: &[&str]) -> DropItemGroup {
        DropItemGroup {
            name: name.into(),
            mob_vnum: mob,
            items: refs.iter().map(|r| DropGroupItem { item_ref: r.to_string(), percent: 1.0, count: 1.0 }).collect(),
        }
    }

    #[test]
    fn duplicates_and_can_save() {
        assert!(!can_save(None));
        assert!(!can_save(Some(&[])));
        assert!(can_save(Some(&[g("A", 101.0, &["1"]), g("B", 102.0, &["2"])])));
        assert!(!can_save(Some(&[g("A", 101.0, &["1"]), g("B", 101.0, &["2"])])));
        assert_eq!(duplicate_mob_vnums(&[g("A", 101.0, &["1"]), g("B", 101.0, &["2"])]), HashSet::from([101]));
        // 0 zählt nie als Dopplung, macht die Gruppe aber trotzdem ungültig.
        assert!(duplicate_mob_vnums(&[g("A", 0.0, &["1"]), g("B", 0.0, &["2"])]).is_empty());
        assert!(!can_save(Some(&[g("A", 0.0, &["1"])])));
        assert!(!can_save(Some(&[g(" ", 101.0, &["1"])])));
        assert!(!can_save(Some(&[g("A", 101.0, &[])])));
        assert!(!can_save(Some(&[g("A", 101.0, &[" "])])));
    }
}
