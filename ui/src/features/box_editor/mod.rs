//! Kisten-Editor (special_item_group.txt) - portiert aus
//! `src/features/box-editor/BoxEditor.tsx`. Lebt als Tab "Item-Gruppen" im
//! Drop-Generator.

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::mob_drop_editor::components::shared::{fmt_num, js_number, js_number_or, Field, NumInput};
use crate::features::mob_drop_editor::{ensure_icons, IconMap};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow, RowRenderer};
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{nav, report_section_dirty, EmbeddedModule, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

const GIFTBOX_TYPE: i64 = 23;
const FLAG_STACKABLE: i32 = 1 << 2;
const ANTIFLAG_NOT_STACKABLE: i32 = 1 << 15;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ItemProtoFullLite {
    #[serde(default)]
    locale_name: String,
    #[serde(default)]
    name: String,
    #[serde(default, rename = "type")]
    item_type: f64,
    #[serde(default)]
    flag: f64,
    #[serde(default)]
    antiflag: f64,
}

impl ItemProtoFullLite {
    fn display_name(&self) -> String {
        if self.locale_name.is_empty() {
            self.name.clone()
        } else {
            self.locale_name.clone()
        }
    }
}

/// JS-Bitoperationen rechnen mit 32-Bit-Ganzzahlen.
fn to_i32(x: f64) -> i32 {
    if x.is_finite() {
        x as i64 as i32
    } else {
        0
    }
}

fn box_item_is_ready(proto: &ItemProtoFullLite) -> bool {
    proto.item_type == GIFTBOX_TYPE as f64
        && (to_i32(proto.flag) & FLAG_STACKABLE) != 0
        && (to_i32(proto.antiflag) & ANTIFLAG_NOT_STACKABLE) == 0
}

/// Setzt item_proto.type=GIFTBOX(23), das "Stapelbar"-Flag und löscht das
/// "Nicht stapelbar"-Antiflag - danach wird das Client-item_proto neu
/// gepackt und verteilt, genau wie der "proto"-Schritt des Item-Editors.
/// Kein Server-Neustart nötig (item_proto ist eine normale DB-Tabelle).
async fn setup_box_item(vnum: i64) -> Result<(), String> {
    let full = invoke::<Option<Value>>("get_item_proto", &json!({ "vnum": vnum })).await?;
    let Some(Value::Object(mut full)) = full else {
        return Err(format!("Error: Item #{vnum} wurde nicht gefunden."));
    };
    let num = |v: Option<&Value>| to_i32(v.and_then(|v| v.as_f64()).unwrap_or(0.0));
    let flag = num(full.get("flag")) | FLAG_STACKABLE;
    let antiflag = num(full.get("antiflag")) & !ANTIFLAG_NOT_STACKABLE;
    full.insert("type".into(), json!(GIFTBOX_TYPE));
    full.insert("flag".into(), json!(flag));
    full.insert("antiflag".into(), json!(antiflag));
    invoke::<Value>("update_item_proto", &json!({ "item": Value::Object(full) })).await?;
    let generated = invoke::<String>("regenerate_item_proto", &json!({})).await?;
    invoke::<Value>("deploy_item_proto", &json!({ "generatedProtoPath": generated })).await?;
    Ok(())
}

#[derive(Clone, PartialEq)]
enum ProtoState {
    Unknown,
    NotFound,
    Found(ItemProtoFullLite),
}

/// Prüft eine "Kisten-Item-VNUM" gegen die echte item_proto-Zeile, damit ein
/// Tippfehler sofort auffällt statt erst als stilles "Du hast nichts
/// erhalten" im Spiel.
#[component]
fn BoxVnumHint(#[prop(into)] vnum: Signal<f64>) -> impl IntoView {
    let proto = RwSignal::new(ProtoState::Unknown);
    let fixing = RwSignal::new(false);
    let fix_error = RwSignal::new(None::<String>);

    let reload = move |v: f64| {
        if v == 0.0 || v.is_nan() {
            proto.set(ProtoState::Unknown);
            return;
        }
        spawn_local(async move {
            match invoke::<Option<ItemProtoFullLite>>("get_item_proto", &json!({ "vnum": v })).await {
                Ok(Some(p)) => proto.set(ProtoState::Found(p)),
                _ => proto.set(ProtoState::NotFound),
            }
        });
    };

    Effect::new(move |_| {
        let v = vnum.get();
        fix_error.set(None);
        reload(v);
    });

    let fix = move || {
        let v = vnum.get_untracked();
        fixing.set(true);
        fix_error.set(None);
        spawn_local(async move {
            match setup_box_item(v as i64).await {
                Ok(()) => reload(v),
                Err(e) => fix_error.set(Some(e)),
            }
            fixing.set(false);
        });
    };

    move || {
        let v = vnum.get();
        if v == 0.0 || v.is_nan() {
            return ().into_any();
        }
        match proto.get() {
            ProtoState::Unknown => ().into_any(),
            ProtoState::NotFound => view! {
                <p class="flex items-center gap-1 text-xs text-destructive">
                    <Icon name="AlertTriangle" class="size-3.5 shrink-0"/>
                    " Kein Item mit dieser VNUM gefunden."
                </p>
            }
            .into_any(),
            ProtoState::Found(p) if !box_item_is_ready(&p) => view! {
                <div class="space-y-1">
                    <p class="flex items-center gap-1 text-xs text-destructive">
                        <Icon name="AlertTriangle" class="size-3.5 shrink-0"/>
                        {format!("\"{}\" ist noch nicht als Kisten-Item eingerichtet (Typ GIFTBOX + Stapelbar).", p.display_name())}
                    </p>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| fix() disabled=fixing>
                        {move || if fixing.get() { "Richte ein…" } else { "Jetzt automatisch einrichten" }}
                    </Button>
                    {move || fix_error.get().map(|e| view! { <p class="text-xs text-destructive">{e}</p> })}
                </div>
            }
            .into_any(),
            ProtoState::Found(p) => view! {
                <p class="flex items-center gap-1 text-xs text-muted-foreground">
                    <Icon name="CheckCircle2" class="size-3.5 shrink-0 text-green-600"/>
                    {format!("\"{}\" (GIFTBOX, stapelbar)", p.display_name())}
                </p>
            }
            .into_any(),
        }
    }
}

/// Suchen-und-Einrichten für das Feld "Kisten-Item-VNUM": ein normales
/// (Nicht-GIFTBOX-)Item braucht vorher eine ausdrückliche Bestätigung.
#[component]
fn BoxItemPicker(#[prop(into)] on_pick: Callback<i64>) -> impl IntoView {
    let picking = RwSignal::new(false);
    let confirm_target = RwSignal::new(None::<(i64, ItemProtoFullLite)>);
    let busy_vnum = RwSignal::new(None::<i64>);
    let error = RwSignal::new(None::<String>);
    let statuses = RwSignal::new(HashMap::<i64, Option<ItemProtoFullLite>>::new());

    let ensure_statuses = move |vnums: Vec<i64>| {
        let mut seen = Vec::new();
        for v in vnums {
            if seen.contains(&v) || statuses.with_untracked(|s| s.contains_key(&v)) {
                continue;
            }
            seen.push(v);
            spawn_local(async move {
                let p = invoke::<Option<ItemProtoFullLite>>("get_item_proto", &json!({ "vnum": v })).await.ok().flatten();
                statuses.update(|s| {
                    s.insert(v, p);
                });
            });
        }
    };

    let apply = move |vnum: i64| {
        error.set(None);
        busy_vnum.set(Some(vnum));
        spawn_local(async move {
            match setup_box_item(vnum).await {
                Ok(()) => {
                    on_pick.run(vnum);
                    picking.set(false);
                    confirm_target.set(None);
                }
                Err(e) => error.set(Some(e)),
            }
            busy_vnum.set(None);
        });
    };

    let handle_pick = move |vnum: i64| {
        error.set(None);
        busy_vnum.set(Some(vnum));
        spawn_local(async move {
            match invoke::<Option<ItemProtoFullLite>>("get_item_proto", &json!({ "vnum": vnum })).await {
                Ok(Some(p)) if !box_item_is_ready(&p) => {
                    confirm_target.set(Some((vnum, p)));
                    busy_vnum.set(None);
                }
                Ok(_) => apply(vnum),
                Err(e) => {
                    error.set(Some(e));
                    busy_vnum.set(None);
                }
            }
        });
    };

    let on_rows = move |rows: Vec<EntityRow>| ensure_statuses(rows.iter().map(|r| r.vnum).collect());

    let trailing: RowRenderer = Arc::new(move |r: &EntityRow| {
        let vnum = r.vnum;
        (move || {
            statuses.with(|s| s.get(&vnum).cloned()).map(|status| {
                let ready = status.as_ref().is_some_and(box_item_is_ready);
                if ready {
                    view! { <span class="ml-1 text-green-600">"(bereits eingerichtet)"</span> }.into_any()
                } else {
                    view! { <span class="ml-1 text-amber-600">"(wird umgestellt)"</span> }.into_any()
                }
            })
        })
        .into_any()
    });

    view! {
        <div class="flex flex-col gap-1">
            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| picking.update(|p| *p = !*p)>
                <Icon name="Search" class="size-3.5"/>
                " Item suchen & einrichten…"
            </Button>
            <Show when=move || picking.get()>
                <div class="w-96 rounded-md border border-border bg-card p-2 shadow-md">
                    {move || error.get().map(|e| view! { <p class="mb-1 text-xs text-destructive">{e}</p> })}
                    <EntityBrowser
                        kind=EntityKind::Item
                        pick_label="Verwenden"
                        auto_focus=true
                        max_height_class="max-h-56"
                        on_pick=move |r: EntityRow| handle_pick(r.vnum)
                        on_rows_change=on_rows
                        render_trailing=trailing.clone()
                    />
                </div>
            </Show>
            {move || confirm_target.get().map(|(vnum, proto)| view! {
                <div class="fixed inset-0 z-10 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                {format!(
                                    "\"{}\" hat aktuell nicht den Typ GIFTBOX (Typ {}). Wird jetzt auf GIFTBOX (23) + Stapelbar umgestellt - das ändert, wie sich dieses Item im Spiel verhält. Wirklich fortfahren?",
                                    proto.display_name(),
                                    fmt_num(proto.item_type),
                                )}
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_target.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button
                                variant=ButtonVariant::Destructive
                                on_click=move |_| apply(vnum)
                                disabled=Signal::derive(move || busy_vnum.get() == Some(vnum))
                            >
                                "Umstellen"
                            </Button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecialItemGroupEntry {
    pub item_ref: String,
    pub count: f64,
    pub prob: f64,
    pub rare_pct: Option<f64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SpecialItemGroup {
    pub name: String,
    pub vnum: f64,
    pub group_type: Option<String>,
    pub entries: Vec<SpecialItemGroupEntry>,
}

/// Schlüsselwörter, die der Server statt einer Item-VNUM akzeptiert
/// (item_manager_read_tables.cpp - ReadSpecialDropItemFile); "bleeding" gibt
/// es nur mit ENABLE_WOLFMAN, das dieser Server hat.
const SPECIAL_KEYWORDS: [(&str, &str); 8] = [
    ("gold", "Gold"),
    ("exp", "EXP"),
    ("mob", "Zufälliger Mob (mob)"),
    ("slow", "Verlangsamung (slow)"),
    ("drain_hp", "HP-Entzug (drain_hp)"),
    ("poison", "Gift (poison)"),
    ("bleeding", "Blutung (bleeding)"),
    ("group", "Verschachtelte Gruppe (group)"),
];

fn is_keyword(s: &str) -> bool {
    SPECIAL_KEYWORDS.iter().any(|(k, _)| *k == s)
}

/// Die "count"-Spalte bedeutet je nach item_ref etwas anderes - bei
/// "gold"/"exp" ist es der ausgegebene Betrag, keine Stückzahl.
pub fn count_label_for(item_ref: &str) -> &'static str {
    match item_ref {
        "gold" => "Betrag (Yang)",
        "exp" => "Betrag (EXP)",
        r if is_keyword(r) => "Wert",
        _ => "Anzahl",
    }
}

const GROUP_TYPES: [(&str, &str); 4] = [
    ("", "Normal (Standard-Drop)"),
    ("pct", "pct (Wahrscheinlichkeit anders gewichtet)"),
    ("quest", "quest (von einem NPC per Quest-Skript ausgelöst)"),
    ("special", "special"),
];

#[component]
fn ItemRefEditor(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    icons: RwSignal<IconMap>,
) -> impl IntoView {
    let picking = RwSignal::new(false);
    let is_kw = Memo::new(move |_| value.with(|v| is_keyword(v)));
    let numeric_vnum = Memo::new(move |_| {
        if is_kw.get() {
            return None;
        }
        let n = js_number(&value.get());
        (!n.is_nan() && n != 0.0).then_some(n)
    });
    Effect::new(move |_| {
        if let Some(v) = numeric_vnum.get() {
            ensure_icons(icons, [v as i64]);
        }
    });
    let icon = move || {
        let n = js_number(&value.get());
        if n.is_nan() {
            return None;
        }
        icons.with(|m| m.get(&(n as i64)).cloned().flatten())
    };

    view! {
        <div class="flex flex-col gap-1">
            <div class="flex items-center gap-1">
                <select
                    on:change=move |e| {
                        let v = event_target_value(&e);
                        if v == "__item__" { on_change.run("0".into()) } else { on_change.run(v) }
                    }
                    class="rounded-md border border-border bg-background px-1 py-1 text-xs"
                >
                    <option value="__item__" selected=move || !is_kw.get()>"Item (VNUM)"</option>
                    {SPECIAL_KEYWORDS.iter().map(|(kw, label)| {
                        let kw = *kw;
                        view! { <option value=kw selected=move || value.with(|v| v == kw)>{*label}</option> }
                    }).collect_view()}
                </select>
                <Show when=move || !is_kw.get()>
                    {move || icon().map(|src| view! { <img src=src alt="" class="size-6 rounded border border-border object-contain"/> })}
                    <input
                        type="number"
                        prop:value=move || value.get()
                        on:input=move |e| on_change.run(event_target_value(&e))
                        class="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs"
                    />
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| picking.update(|p| *p = !*p)>
                        <Icon name="Search" class="size-3.5"/>
                    </Button>
                </Show>
            </div>
            <Show when=move || picking.get() && !is_kw.get()>
                <div class="rounded-md border border-border bg-card p-2">
                    <EntityBrowser
                        kind=EntityKind::Item
                        pick_label="Übernehmen"
                        auto_focus=true
                        max_height_class="max-h-56"
                        on_pick=move |r: EntityRow| {
                            on_change.run(r.vnum.to_string());
                            picking.set(false);
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

#[component]
pub fn BoxEditor() -> impl IntoView {
    let groups = RwSignal::new(None::<Vec<SpecialItemGroup>>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let selected_index = RwSignal::new(None::<usize>);
    let icons = RwSignal::new(IconMap::new());

    let creating = RwSignal::new(false);
    let new_name = RwSignal::new(String::new());
    let new_name_preview = RwSignal::new(String::new());
    let new_vnum = RwSignal::new(String::new());

    let delete_confirm = RwSignal::new(None::<usize>);
    let saving = RwSignal::new(false);
    let save_confirm = RwSignal::new(false);

    let undo_confirm = RwSignal::new(false);
    let undoing = RwSignal::new(false);

    let loaded_snapshot = RwSignal::new(None::<Vec<SpecialItemGroup>>);
    let dirty = Memo::new(move |_| groups.with(|g| g.is_some() && loaded_snapshot.with(|s| s != g)));
    Effect::new(move |_| report_section_dirty(Section::DropGenerator, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::DropGenerator, false));

    let load = move || {
        selected_index.set(None);
        loading.set(true);
        load_error.set(None);
        spawn_local(async move {
            match invoke::<Vec<SpecialItemGroup>>("read_special_item_group_file", &json!({})).await {
                Ok(loaded) => {
                    groups.set(Some(loaded.clone()));
                    loaded_snapshot.set(Some(loaded));
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };
    load();

    Effect::new(move |_| {
        if !creating.get() {
            return;
        }
        let n = new_name.get();
        if n.trim().is_empty() {
            new_name_preview.set(String::new());
            return;
        }
        spawn_local(async move {
            match invoke::<String>("sanitize_special_item_group_name", &json!({ "name": n })).await {
                Ok(p) => new_name_preview.set(p),
                Err(_) => new_name_preview.set(String::new()),
            }
        });
    });

    let update_group = move |idx: usize, f: Box<dyn FnOnce(&mut SpecialItemGroup)>| {
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(idx)) {
                f(g)
            }
        });
    };
    let update_entry = move |gi: usize, ei: usize, f: Box<dyn FnOnce(&mut SpecialItemGroupEntry)>| {
        groups.update(|g| {
            if let Some(e) = g.as_mut().and_then(|g| g.get_mut(gi)).and_then(|g| g.entries.get_mut(ei)) {
                f(e)
            }
        });
    };
    let add_entry = move |gi: usize| {
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(gi)) {
                g.entries.push(SpecialItemGroupEntry { item_ref: "0".into(), count: 1.0, prob: 1.0, rare_pct: None });
            }
        })
    };
    let remove_entry = move |gi: usize, ei: usize| {
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(gi)) {
                if ei < g.entries.len() {
                    g.entries.remove(ei);
                }
            }
        })
    };

    let create_group = move || {
        let preview = new_name_preview.get_untracked();
        if new_name.with_untracked(|n| n.trim().is_empty()) || preview.is_empty() {
            return;
        }
        let vnum = js_number_or(&new_vnum.get_untracked(), 0.0);
        let len = groups.with_untracked(|g| g.as_ref().map(|g| g.len()).unwrap_or(0));
        groups.update(|g| {
            g.get_or_insert_with(Vec::new).push(SpecialItemGroup { name: preview, vnum, group_type: None, entries: vec![] })
        });
        selected_index.set(Some(len));
        creating.set(false);
        new_name.set(String::new());
        new_vnum.set(String::new());
    };

    let delete_group = move |idx: usize| {
        groups.update(|g| {
            if let Some(g) = g.as_mut() {
                if idx < g.len() {
                    g.remove(idx);
                }
            }
        });
        selected_index.set(None);
        delete_confirm.set(None);
    };

    let save = move || {
        save_confirm.set(false);
        let gs = groups.get_untracked();
        saving.set(true);
        spawn_local(async move {
            match invoke::<Option<String>>("write_special_item_group_file", &json!({ "groups": gs })).await {
                Ok(backup_path) => {
                    toast::success(&match backup_path {
                        Some(p) if !p.is_empty() => format!("Gespeichert (Backup: {p})"),
                        _ => "Gespeichert.".to_string(),
                    });
                    let n = gs.as_ref().map(|g| g.len()).unwrap_or(0);
                    loaded_snapshot.set(gs);
                    log_activity(
                        EmbeddedModule::BoxEditor.into(),
                        "save",
                        &format!("special_item_group.txt gespeichert ({n} Gruppe(n))"),
                        Some("file"),
                        None,
                    );
                }
                Err(e) => toast::error(&e),
            }
            saving.set(false);
        });
    };

    use_save_shortcut(
        Section::DropGenerator,
        move || dirty.get_untracked() && !saving.get_untracked(),
        move || save_confirm.set(true),
    );

    // "Letzte Änderung rückgängig machen" - jedes Speichern legt serverseitig
    // schon ein Backup an, das undo_special_item_group_write nur restauriert.
    let undo = move || {
        undo_confirm.set(false);
        undoing.set(true);
        spawn_local(async move {
            match invoke::<String>("undo_special_item_group_write", &json!({})).await {
                Ok(backup_path) => {
                    toast::success(&format!("Wiederhergestellt aus: {backup_path}"));
                    log_activity(
                        EmbeddedModule::BoxEditor.into(),
                        "restore",
                        &format!("special_item_group.txt auf vorherigen Stand zurückgesetzt ({backup_path})"),
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

    let has_groups = Memo::new(move |_| groups.with(|g| g.is_some()));
    let group_count = move || groups.with(|g| g.as_ref().map(|g| g.len()).unwrap_or(0));
    let gread = move |i: usize| groups.with(|g| g.as_ref().and_then(|g| g.get(i)).map(|g| (g.name.clone(), g.vnum, g.group_type.clone(), g.entries.len())));
    let eread = move |gi: usize, ei: usize| groups.with(|g| g.as_ref().and_then(|g| g.get(gi)).and_then(|g| g.entries.get(ei)).cloned());
    let selected = Memo::new(move |_| selected_index.get().filter(|i| groups.with(|g| g.as_ref().is_some_and(|g| *i < g.len()))));

    view! {
        <div class="space-y-6 pb-10">
            <PageHeader module=EmbeddedModule::BoxEditor title="Kisten-Editor".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Bearbeitet " <code>"special_item_group.txt"</code>
                " - die Beute-Tabelle, aus der ein Item vom Typ " <strong>"GIFTBOX"</strong>
                " (23) beim Öffnen zieht. Die „verbleibende Anzahl\" bei einer mehrfach öffenbaren Kiste ist einfach die Stapelanzahl des Items selbst (keine gesonderte Einstellung nötig) - siehe "
                <code>"ITEM_GIFTBOX"</code>
                " im Server-Quellcode: jedes Öffnen zieht 1 vom Stapel ab."
            </p>
            <p class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                "Damit eine Kiste diese Tabelle überhaupt nutzt, muss ihr " <code>"item_proto.type"</code> " auf "
                <strong>"GIFTBOX (23)"</strong>
                " stehen und „Stapelbar\" gesetzt sein - über „Item suchen & einrichten…\" bei der Kisten-Item-VNUM lässt sich das jetzt auch direkt hier erledigen, statt umständlich im Item-Editor."
            </p>
            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Änderungen wirken erst nach einem " <strong>"Server-Neustart"</strong> " - "
                    <code>"special_item_group.txt"</code>
                    " wird nur beim Start des Game-Prozesses eingelesen (kein Client-Repack, kein "
                    <code>"/reload"</code>
                    "-Sub-Befehl dafür verfügbar - im echten Quellcode geprüft). Eine neu angelegte oder geänderte Kisten-Gruppe zeigt sich also erst nach einem Neustart; bis dahin meldet das Öffnen der Kiste „Du hast nichts erhalten.\" und die Kiste wird dabei nicht verbraucht. "
                    <button class="underline" on:click=move |_| nav().set_section(Section::ServerControl)>
                        "Zur Server-Steuerung"
                    </button>
                </span>
            </div>

            <div class="flex items-center gap-2">
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon
                        name="RefreshCw"
                        class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })
                    />
                    " Neu laden"
                </Button>
                <Button variant=ButtonVariant::Outline on_click=move |_| creating.set(true)>
                    <Icon name="Plus" class="size-4"/>
                    " Neue Kisten-Gruppe"
                </Button>
                <Button on_click=move |_| save_confirm.set(true) disabled=Signal::derive(move || saving.get() || !has_groups.get())>
                    {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                </Button>
                <Button variant=ButtonVariant::Outline on_click=move |_| undo_confirm.set(true) disabled=Signal::derive(move || undoing.get() || !has_groups.get())>
                    <Icon name="Undo2" class="size-4"/>
                    {move || if undoing.get() { " Setze zurück…" } else { " Letzte Änderung rückgängig machen" }}
                </Button>
            </div>
            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <Show when=move || creating.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">"Neue Kisten-Gruppe"</p>
                        <Field label="Name (wird bereinigt: keine Leerzeichen/Umlaute)">
                            <input
                                autofocus
                                prop:value=move || new_name.get()
                                on:input=move |e| new_name.set(event_target_value(&e))
                                class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Field>
                        <Show when=move || !new_name_preview.with(|p| p.is_empty())>
                            <p class="text-xs text-muted-foreground">"Wird zu: " <code>{move || new_name_preview.get()}</code></p>
                        </Show>
                        <Field label="Kisten-Item-VNUM">
                            <input
                                type="number"
                                prop:value=move || new_vnum.get()
                                on:input=move |e| new_vnum.set(event_target_value(&e))
                                class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Field>
                        <BoxItemPicker on_pick=move |v: i64| new_vnum.set(v.to_string())/>
                        <BoxVnumHint vnum=Signal::derive(move || js_number_or(&new_vnum.get(), 0.0))/>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| creating.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button disabled=Signal::derive(move || new_name_preview.with(|p| p.is_empty())) on_click=move |_| create_group()>
                                "Anlegen"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when=move || has_groups.get()>
                <div class="flex gap-4">
                    <div class="w-64 shrink-0 space-y-1 rounded-lg border border-border p-2">
                        <Show when=move || group_count() == 0>
                            <p class="p-2 text-sm text-muted-foreground">"Keine Gruppen."</p>
                        </Show>
                        <For each=move || 0..group_count() key=|i| *i let:i>
                            <button
                                on:click=move |_| selected_index.set(Some(i))
                                class=move || {
                                    if selected_index.get() == Some(i) {
                                        "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm bg-primary text-primary-foreground"
                                    } else {
                                        "flex w-full items-center gap-2 rounded-md px-2 py-1.5 text-left text-sm hover:bg-muted"
                                    }
                                }
                            >
                                <Icon name="Package" class="size-4 shrink-0"/>
                                <span class="flex-1 truncate">{move || gread(i).map(|g| g.0).unwrap_or_default()}</span>
                                <span class="text-xs opacity-70">{move || format!("#{}", gread(i).map(|g| fmt_num(g.1)).unwrap_or_default())}</span>
                            </button>
                        </For>
                    </div>

                    <div class="flex-1 space-y-3 rounded-lg border border-border p-4">
                        <Show when=move || selected.get().is_none()>
                            <p class="text-sm text-muted-foreground">"Gruppe links auswählen oder neu anlegen."</p>
                        </Show>
                        {move || selected.get().map(|si| {
                            let entry_count = move || gread(si).map(|g| g.3).unwrap_or(0);
                            view! {
                                <div class="flex flex-wrap items-center gap-3">
                                    <Field label="Name">
                                        <input
                                            prop:value=move || gread(si).map(|g| g.0).unwrap_or_default()
                                            on:input=move |e| {
                                                let v = event_target_value(&e);
                                                update_group(si, Box::new(move |g| g.name = v));
                                            }
                                            class="w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                    </Field>
                                    <Field label="Kisten-Item-VNUM">
                                        <NumInput
                                            value=Signal::derive(move || Some(gread(si).map(|g| g.1).unwrap_or(0.0)))
                                            on_input=move |v: String| update_group(si, Box::new(move |g| g.vnum = js_number_or(&v, 0.0)))
                                            class="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        />
                                        <BoxItemPicker on_pick=move |v: i64| update_group(si, Box::new(move |g| g.vnum = v as f64))/>
                                        <BoxVnumHint vnum=Signal::derive(move || gread(si).map(|g| g.1).unwrap_or(0.0))/>
                                    </Field>
                                    <Field label="Typ">
                                        <select
                                            on:change=move |e| {
                                                let v = event_target_value(&e);
                                                update_group(si, Box::new(move |g| g.group_type = if v.is_empty() { None } else { Some(v) }));
                                            }
                                            class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                                        >
                                            {GROUP_TYPES.iter().map(|(value, label)| {
                                                let value = *value;
                                                view! {
                                                    <option
                                                        value=value
                                                        selected=move || gread(si).and_then(|g| g.2).unwrap_or_default() == value
                                                    >
                                                        {*label}
                                                    </option>
                                                }
                                            }).collect_view()}
                                        </select>
                                    </Field>
                                    <Button variant=ButtonVariant::Destructive size=ButtonSize::Sm on_click=move |_| delete_confirm.set(Some(si))>
                                        <Icon name="Trash2" class="size-3.5"/>
                                        " Gruppe löschen"
                                    </Button>
                                </div>

                                <div class="space-y-2">
                                    <h2 class="text-sm font-medium text-muted-foreground">
                                        {move || format!("Beute-Einträge ({})", entry_count())}
                                    </h2>
                                    <For each=move || 0..entry_count() key=|i| *i let:ei>
                                        {
                                            let item_ref = Signal::derive(move || eread(si, ei).map(|e| e.item_ref).unwrap_or_default());
                                            view! {
                                                <div class="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
                                                    <ItemRefEditor
                                                        value=item_ref
                                                        on_change=move |v: String| update_entry(si, ei, Box::new(move |e| e.item_ref = v))
                                                        icons=icons
                                                    />
                                                    <Field label=Signal::derive(move || count_label_for(&item_ref.get()).to_string())>
                                                        <NumInput
                                                            value=Signal::derive(move || Some(eread(si, ei).map(|e| e.count).unwrap_or(0.0)))
                                                            on_input=move |v: String| update_entry(si, ei, Box::new(move |e| e.count = js_number_or(&v, 0.0)))
                                                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                                        />
                                                    </Field>
                                                    <Field label="Gewicht/Chance">
                                                        <NumInput
                                                            value=Signal::derive(move || Some(eread(si, ei).map(|e| e.prob).unwrap_or(0.0)))
                                                            on_input=move |v: String| update_entry(si, ei, Box::new(move |e| e.prob = js_number_or(&v, 0.0)))
                                                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                                        />
                                                    </Field>
                                                    <Field label="Selten-% (optional)">
                                                        <NumInput
                                                            value=Signal::derive(move || eread(si, ei).and_then(|e| e.rare_pct))
                                                            on_input=move |v: String| update_entry(si, ei, Box::new(move |e| {
                                                                e.rare_pct = if v.is_empty() { None } else { Some(js_number_or(&v, 0.0)) }
                                                            }))
                                                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                                        />
                                                    </Field>
                                                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| remove_entry(si, ei)>
                                                        <Icon name="Trash2" class="size-3.5"/>
                                                    </Button>
                                                </div>
                                            }
                                        }
                                    </For>
                                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| add_entry(si)>
                                        <Icon name="Plus" class="size-3.5"/>
                                        " Eintrag hinzufügen"
                                    </Button>
                                    <p class="flex items-start gap-2 text-xs text-muted-foreground">
                                        <Icon name="Info" class="mt-0.5 size-3.5 shrink-0"/>
                                        "„Gewicht/Chance\" ist kein Prozentwert, sondern ein relatives Gewicht gegenüber den anderen Einträgen derselben Gruppe (im echten Server-Code als "
                                        <code>"iProb"</code>
                                        " verwendet) - höher heißt wahrscheinlicher, aber die exakte Chance hängt von allen Einträgen zusammen ab."
                                    </p>
                                </div>
                            }
                        })}
                    </div>
                </div>
            </Show>

            {move || delete_confirm.get().map(|dc| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-center gap-2 text-sm font-medium">
                            <Icon name="AlertTriangle" class="size-4 text-destructive"/>
                            {move || format!(" Gruppe „{}\" wirklich entfernen?", gread(dc).map(|g| g.0).unwrap_or_default())}
                        </p>
                        <p class="text-xs text-muted-foreground">
                            "Wird erst beim „Speichern\" wirklich auf den Server geschrieben - bis dahin rückgängig machbar durch nicht speichern/neu laden."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| delete_group(dc)>
                                "Entfernen"
                            </Button>
                        </div>
                    </div>
                </div>
            })}

            <Show when=move || save_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">"special_item_group.txt jetzt überschreiben?"</p>
                        <p class="text-xs text-muted-foreground">"Ein Backup der aktuellen Datei wird vorher auf dem Server angelegt."</p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| save_confirm.set(false)>
                                "Abbrechen"
                            </Button>
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
                            "Letzte gespeicherte Änderung an special_item_group.txt rückgängig machen?"
                        </p>
                        <p class="text-xs text-muted-foreground">
                            "Stellt die zuletzt gesicherte Vorgängerversion wieder her (der aktuelle Stand wird dabei selbst zuerst gesichert). Betrifft nur, was zuletzt tatsächlich gespeichert wurde - nicht unfertige, noch ungespeicherte Eingaben hier im Editor."
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| undo_confirm.set(false)>
                                "Abbrechen"
                            </Button>
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

#[cfg(test)]
mod tests {
    use super::*;

    fn proto(t: f64, flag: f64, antiflag: f64) -> ItemProtoFullLite {
        ItemProtoFullLite { locale_name: String::new(), name: "x".into(), item_type: t, flag, antiflag }
    }

    #[test]
    fn ready_check() {
        assert!(box_item_is_ready(&proto(23.0, 4.0, 0.0)));
        assert!(!box_item_is_ready(&proto(1.0, 4.0, 0.0)));
        assert!(!box_item_is_ready(&proto(23.0, 0.0, 0.0)));
        assert!(!box_item_is_ready(&proto(23.0, 4.0, 32768.0)));
    }

    #[test]
    fn count_labels() {
        assert_eq!(count_label_for("gold"), "Betrag (Yang)");
        assert_eq!(count_label_for("exp"), "Betrag (EXP)");
        assert_eq!(count_label_for("poison"), "Wert");
        assert_eq!(count_label_for("123"), "Anzahl");
    }
}
