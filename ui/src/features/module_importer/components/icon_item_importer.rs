//! Icon-Item-Importer: Zubehör ohne 3D-Modell (Schuhe, Ketten, Schilder,
//! Ohrringe, Armbänder, Verbrauchsgegenstände, ...) - Port von
//! `IconItemImporter.tsx`.

use super::reference_item_picker::ReferenceItemPicker;
use crate::components::{Button, ButtonVariant, Icon};
use crate::features::item_editor::components::shared::{Field, FlagSelect, StepRow};
use crate::features::item_editor::item_flags::{
    item_type_label, subtypes_by_type, value_hints, value_labels_by_type, ITEM_TYPES, WEAR_FLAGS,
};
use crate::features::item_editor::types::{js_number, num};
use crate::features::module_importer::shared::*;
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::{dialog_open_one, invoke, invoke_unit, OpenDialogOptions};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
struct IconItemRow {
    name: String,
    locale_name: String,
    description: String,
    summary: String,
    item_type: i64,
    subtype: i64,
    wearflag: i64,
    values: [i64; 6],
    /// Welche value0-5 pro Aufwertungs-Stufe wachsen - bewusst pro Feld,
    /// da die Bedeutung je Typ/Subtyp variiert.
    grow_flags: [bool; 6],
    ref_item: Option<ItemProtoInput>,
    chain_enabled: bool,
    chain_max_level: i64,
    chain_growth_percent: i64,
}

fn default_icon_item_row(suggested_name: &str) -> IconItemRow {
    IconItemRow {
        name: suggested_name.to_string(),
        locale_name: suggested_name.to_string(),
        description: String::new(),
        summary: String::new(),
        item_type: 2,
        subtype: 0,
        wearflag: 0,
        values: [0; 6],
        grow_flags: [false; 6],
        ref_item: None,
        chain_enabled: false,
        chain_max_level: 9,
        chain_growth_percent: 10,
    }
}

/// Vorschlag aus dem Dateinamen: Endung weg, `_`/`-`-Folgen zu Leerzeichen.
fn suggested_name(path: &str) -> String {
    let file = file_name_of(path);
    let stem = match file.rfind('.') {
        Some(i) if i + 1 < file.len() => file[..i].to_string(),
        _ => file.clone(),
    };
    let mut out = String::new();
    let mut in_sep = false;
    for c in stem.chars() {
        if c == '_' || c == '-' {
            if !in_sep {
                out.push(' ');
            }
            in_sep = true;
        } else {
            out.push(c);
            in_sep = false;
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        stem
    } else {
        trimmed
    }
}

const VALUE_KEYS: [&str; 6] = ["value0", "value1", "value2", "value3", "value4", "value5"];
const IN: &str = "rounded-md border border-border bg-background px-2 py-1 text-sm";

#[component]
pub fn IconItemImporter(#[prop(into)] on_imported: Callback<()>) -> impl IntoView {
    let folder_path = RwSignal::new(String::new());
    let scanning = RwSignal::new(false);
    let scan_error = RwSignal::new(None::<String>);
    let icon_paths = RwSignal::new(None::<Vec<String>>);
    let selected = RwSignal::new(HashMap::<String, bool>::new());
    let rows = RwSignal::new(HashMap::<String, IconItemRow>::new());
    let previews = RwSignal::new(HashMap::<String, Option<String>>::new());
    let vnum_range_start = RwSignal::new(500000i64);

    let running = RwSignal::new(false);
    let item_status = RwSignal::new(HashMap::<String, StepStatus>::new());
    let other_steps = RwSignal::new(HashMap::<&'static str, StepStatus>::new());
    let batch_error = RwSignal::new(None::<String>);
    let batch_done = RwSignal::new(None::<Vec<i64>>);
    let confirm = RwSignal::new(false);

    let dirty = Memo::new(move |_| icon_paths.with(|p| p.is_some()) && batch_done.with(|b| b.is_none()));
    Effect::new(move |_| report_section_dirty(Section::ModuleImporter, dirty.get()));
    on_cleanup(|| report_section_dirty(Section::ModuleImporter, false));

    spawn_local(async move {
        if let Ok(v) = invoke::<Option<String>>("get_setting", &json!({ "key": "item_vnum_range_start" })).await {
            vnum_range_start.set(js_number(v.as_deref().unwrap_or("500000")) as i64);
        }
    });

    let pick_folder = move || {
        spawn_local(async move {
            let res = dialog_open_one(&OpenDialogOptions {
                directory: true,
                title: Some("Icon-Ordner wählen".into()),
                ..Default::default()
            })
            .await;
            if let Ok(Some(p)) = res {
                folder_path.set(p);
                icon_paths.set(None);
                scan_error.set(None);
            }
        });
    };

    let scan = move || {
        let path = folder_path.get_untracked();
        if path.is_empty() {
            return;
        }
        scanning.set(true);
        scan_error.set(None);
        spawn_local(async move {
            match invoke::<Vec<String>>("scan_icon_folder", &json!({ "path": path })).await {
                Ok(found) => {
                    selected.set(found.iter().map(|p| (p.clone(), true)).collect());
                    rows.set(found.iter().map(|p| (p.clone(), default_icon_item_row(&suggested_name(p)))).collect());
                    icon_paths.set(Some(found.clone()));
                    item_status.set(HashMap::new());
                    batch_error.set(None);
                    batch_done.set(None);
                    for p in found {
                        spawn_local(async move {
                            let url = invoke::<String>("preview_image_file", &json!({ "path": p })).await.ok();
                            previews.update(|m| {
                                m.insert(p, url);
                            });
                        });
                    }
                }
                Err(e) => {
                    scan_error.set(Some(e));
                    icon_paths.set(None);
                }
            }
            scanning.set(false);
        });
    };

    let selected_paths = move || -> Vec<String> {
        icon_paths.with(|p| {
            p.as_ref()
                .map(|p| {
                    p.iter()
                        .filter(|x| selected.with(|s| s.get(*x).copied().unwrap_or(false)))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let can_import = Memo::new(move |_| {
        let sel = selected_paths();
        !sel.is_empty()
            && sel.iter().all(|p| {
                rows.with(|r| {
                    r.get(p).is_some_and(|x| !x.name.trim().is_empty() && !x.locale_name.trim().is_empty())
                })
            })
    });

    use_save_shortcut(
        Section::ModuleImporter,
        move || dirty.get_untracked() && can_import.get_untracked() && !running.get_untracked(),
        move || confirm.set(true),
    );

    let run_import = move || {
        confirm.set(false);
        running.set(true);
        batch_error.set(None);
        batch_done.set(None);
        let sel = selected_paths();
        let all_rows = rows.get_untracked();
        let range_start0 = vnum_range_start.get_untracked();
        let folder = folder_path.get_untracked();
        item_status.set(sel.iter().map(|p| (p.clone(), StepStatus::Pending)).collect());
        other_steps.set(HashMap::from([
            ("setup", StepStatus::Pending),
            ("pack", StepStatus::Pending),
            ("proto", StepStatus::Pending),
        ]));
        let set_other = move |k: &'static str, s: StepStatus| {
            other_steps.update(|m| {
                m.insert(k, s);
            })
        };

        spawn_local(async move {
            let mut created: Vec<i64> = vec![];
            let result: Result<(), String> = async {
                set_other("setup", StepStatus::Running);
                invoke_unit("validate_item_editor_setup", &json!({ "requireIconTool": true })).await?;
                set_other("setup", StepStatus::Done);

                let mut range_start = range_start0;

                for path in &sel {
                    let Some(row) = all_rows.get(path).cloned() else { continue };
                    item_status.update(|m| {
                        m.insert(path.clone(), StepStatus::Running);
                    });

                    let levels = if row.chain_enabled { row.chain_max_level + 1 } else { 1 };
                    let base_vnum = find_consecutive_free_vnums(range_start, levels).await?;
                    range_start = base_vnum + levels;

                    invoke_unit("write_item_icon", &json!({ "vnum": base_vnum, "sourcePath": path })).await?;
                    let icon_rel_path = format!("icon/item/{}.tga", pad5(base_vnum));

                    let mut level_vnums = vec![];
                    for level in 0..levels {
                        let vnum = base_vnum + level;
                        level_vnums.push(vnum);
                        let template = row.ref_item.clone().unwrap_or_else(|| empty_item(vnum, row.item_type));
                        let suffix = if level > 0 { format!(" +{level}") } else { String::new() };
                        let values: Vec<i64> = row
                            .values
                            .iter()
                            .enumerate()
                            .map(|(i, v)| if row.grow_flags[i] { scaled_value(*v, level, row.chain_growth_percent) } else { *v })
                            .collect();
                        let item = ItemProtoInput {
                            vnum,
                            vnum_range: 0,
                            name: fit_to_byte_limit_with_suffix(
                                &row.name,
                                &if level > 0 { format!("_{level}") } else { String::new() },
                                ITEM_NAME_MAX_BYTES,
                            ),
                            locale_name: fit_to_byte_limit_with_suffix(&row.locale_name, &suffix, ITEM_NAME_MAX_BYTES),
                            r#type: row.item_type,
                            subtype: row.subtype,
                            // Zubehör ohne 3D-Modell ist immer 1 Slot groß.
                            size: 1,
                            wearflag: row.wearflag,
                            value0: values[0],
                            value1: values[1],
                            value2: values[2],
                            value3: values[3],
                            value4: values[4],
                            value5: values[5],
                            refine_set: 0,
                            refined_vnum: 0,
                            ..template
                        };
                        invoke_unit("create_item_proto", &json!({ "item": item })).await?;
                        created.push(vnum);
                        if !row.description.trim().is_empty() || !row.summary.trim().is_empty() {
                            invoke_unit(
                                "write_item_desc",
                                &json!({ "vnum": vnum, "description": row.description, "summary": row.summary }),
                            )
                            .await?;
                        }
                        invoke_unit(
                            "write_item_list_entry",
                            &json!({ "vnum": vnum, "itemType": row.item_type, "iconRelPath": icon_rel_path, "modelRelPath": null }),
                        )
                        .await?;
                    }

                    for level in 0..(levels - 1).max(0) as usize {
                        let recipe_id = invoke::<i64>(
                            "save_refine_recipe",
                            &json!({
                                "id": null,
                                "cost": refine_cost_for_step(level as i64),
                                "prob": refine_prob_for_step(level as i64),
                                "materials": [],
                            }),
                        )
                        .await?;
                        invoke_unit(
                            "set_item_refine_link",
                            &json!({ "vnum": level_vnums[level], "refineSet": recipe_id, "refinedVnum": level_vnums[level + 1] }),
                        )
                        .await?;
                    }

                    item_status.update(|m| {
                        m.insert(path.clone(), StepStatus::Done);
                    });
                }

                set_other("pack", StepStatus::Running);
                invoke_unit("pack_item_icons", &json!({})).await?;
                set_other("pack", StepStatus::Done);

                set_other("proto", StepStatus::Running);
                let generated = invoke::<String>("regenerate_item_proto", &json!({})).await?;
                invoke_unit("deploy_item_proto", &json!({ "generatedProtoPath": generated })).await?;
                set_other("proto", StepStatus::Done);

                batch_done.set(Some(created.clone()));
                let recorded = invoke_unit(
                    "record_import_batch",
                    &json!({
                        "moduleName": last_path_segment(&folder),
                        "itemType": 2,
                        "vnums": created,
                        "hadEffects": false,
                    }),
                )
                .await;
                if recorded.is_ok() {
                    on_imported.run(());
                }
                Ok(())
            }
            .await;

            if let Err(e) = result {
                let mut message = e;
                if !created.is_empty() {
                    let mut failed = vec![];
                    for &vnum in &created {
                        match invoke_unit("rollback_created_item", &json!({ "vnum": vnum })).await {
                            Ok(()) => log_activity(
                                Section::ModuleImporter.into(),
                                "rollback-item",
                                &format!(
                                    "Neu angelegtes Item zurückgenommen (vnum {vnum}, Rollback nach fehlgeschlagenem Import)"
                                ),
                                Some("item"),
                                Some(&vnum.to_string()),
                            ),
                            Err(_) => failed.push(vnum),
                        }
                    }
                    message.push_str(&if failed.is_empty() {
                        format!("\n\nAlle {} bereits angelegten Einträge wurden automatisch zurückgenommen.", created.len())
                    } else {
                        format!(
                            "\n\nRollback teilweise fehlgeschlagen - bitte vnums {} manuell im DB Explorer prüfen.",
                            join_vnums(&failed)
                        )
                    });
                }
                batch_error.set(Some(message));
            }
            running.set(false);
        });
    };

    // ---------------------------------------------------------------- eine Icon-Zeile
    let row_view = move |path: String| {
        let p = path.clone();
        let get = move |f: fn(&IconItemRow) -> i64| {
            let p = p.clone();
            Signal::derive(move || rows.with(|r| r.get(&p).map(f).unwrap_or(0)))
        };
        let p = path.clone();
        let update = move |f: Box<dyn FnOnce(&mut IconItemRow)>| {
            rows.update(|r| {
                if let Some(x) = r.get_mut(&p) {
                    f(x)
                }
            })
        };
        let p = path.clone();
        let row = move || rows.with(|r| r.get(&p).cloned());
        let item_type = get(|r| r.item_type);
        let subtype = get(|r| r.subtype);
        let wearflag = get(|r| r.wearflag);
        let (p1, p2, p3) = (path.clone(), path.clone(), path.clone());
        let u = update.clone();
        let text_input = {
            let row = row.clone();
            let update = update.clone();
            move |getter: fn(&IconItemRow) -> String, setter: fn(&mut IconItemRow, String), class: &'static str, textarea: bool| {
                let row = row.clone();
                let update = update.clone();
                let value = move || row().map(|r| getter(&r)).unwrap_or_default();
                let on_input = move |e: leptos::ev::Event| {
                    let v = event_target_value(&e);
                    update(Box::new(move |x| setter(x, v)))
                };
                if textarea {
                    view! { <textarea prop:value=value on:input=on_input class=class></textarea> }.into_any()
                } else {
                    view! { <input prop:value=value on:input=on_input class=class/> }.into_any()
                }
            }
        };

        let name_input = text_input(|r| r.name.clone(), |x, v| x.name = v, "w-48 rounded-md border border-border bg-background px-2 py-1 text-sm", false);
        let desc_input = text_input(|r| r.description.clone(), |x, v| x.description = v, "h-16 w-64 rounded-md border border-border bg-background px-2 py-1 text-sm", true);
        let summary_input = text_input(|r| r.summary.clone(), |x, v| x.summary = v, "h-16 w-48 rounded-md border border-border bg-background px-2 py-1 text-sm", true);
        let row_c = row.clone();
        let row_hint = row.clone();
        let row_ref = row.clone();
        let row_vals = row.clone();
        let row_chain = row.clone();
        let (u_sel, u_type, u_sub, u_sub2, u_wear, u_chain, u_ref) =
            (u.clone(), u.clone(), u.clone(), u.clone(), u.clone(), u.clone(), u.clone());
        let u_vals = u.clone();
        let u_chain2 = u.clone();
        view! {
            <div class="rounded-md border border-border p-2">
                <div class="flex items-center gap-3">
                    <input
                        type="checkbox"
                        prop:checked=move || selected.with(|s| s.get(&p1).copied().unwrap_or(false))
                        on:change=move |e| {
                            let c = event_target_checked(&e);
                            let _ = &u_sel;
                            selected.update(|s| {
                                s.insert(p2.clone(), c);
                            })
                        }
                    />
                    {move || match previews.with(|m| m.get(&p3).cloned().flatten()) {
                        Some(src) => view! { <img src=src alt="" class="size-10 rounded border border-border object-contain"/> }.into_any(),
                        None => view! {
                            <div class="flex size-10 items-center justify-center rounded border border-border bg-muted/40">
                                <Icon name="Image" class="size-4 text-muted-foreground"/>
                            </div>
                        }
                        .into_any(),
                    }}
                    <span class="w-40 truncate text-sm" title=path.clone()>{file_name_of(&path)}</span>
                    {
                        let row_c = row_c.clone();
                        let u = u.clone();
                        view! {
                            <input
                                prop:value=move || row_c().map(|r| r.locale_name).unwrap_or_default()
                                on:input=move |e| {
                                    let v = event_target_value(&e);
                                    u(Box::new(move |x| {
                                        x.locale_name = v.clone();
                                        x.name = v;
                                    }))
                                }
                                placeholder="Anzeigename"
                                class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        }
                    }
                    <span class="text-xs text-muted-foreground">
                        {move || {
                            let t = item_type.get();
                            let s = subtype.get();
                            let tl = item_type_label(t).map(str::to_string).unwrap_or_else(|| t.to_string());
                            let sl = subtypes_by_type(t)
                                .and_then(|o| o.iter().find(|x| x.value == s))
                                .map(|x| x.label.to_string())
                                .unwrap_or_else(|| format!("Subtyp {s}"));
                            format!("{tl} ({t}) · {sl}")
                        }}
                    </span>
                </div>

                <details class="mt-2">
                    <summary class="cursor-pointer text-xs text-muted-foreground">
                        "Typ, Werte, Referenz-Item, Aufwertungs-Kette bearbeiten…"
                    </summary>
                    <div class="mt-2 space-y-3 border-t border-border pt-2">
                        <div class="flex flex-wrap gap-3">
                            <Field label="Interner Name (ASCII)">
                                {name_input}
                            </Field>
                            <Field label="Item-Typ">
                                <FlagSelect
                                    options=ITEM_TYPES
                                    value=item_type
                                    on_change=Callback::new(move |v: i64| u_type(Box::new(move |x| {
                                        x.item_type = v;
                                        x.subtype = 0;
                                    })))
                                    class=IN
                                />
                            </Field>
                            {move || {
                                let u_sub = u_sub.clone();
                                let u_sub2 = u_sub2.clone();
                                match subtypes_by_type(item_type.get()) {
                                    Some(opts) => view! {
                                        <Field label="Subtyp">
                                            <FlagSelect
                                                options=opts
                                                value=subtype
                                                on_change=Callback::new(move |v: i64| u_sub(Box::new(move |x| x.subtype = v)))
                                                class=IN
                                            />
                                        </Field>
                                    }
                                    .into_any(),
                                    None => view! {
                                        <Field label="Subtyp (Rohwert, für diesen Typ nicht abgebildet)">
                                            <input
                                                type="number"
                                                prop:value=move || subtype.get().to_string()
                                                on:input=move |e| {
                                                    let v = num(&event_target_value(&e));
                                                    u_sub2(Box::new(move |x| x.subtype = v))
                                                }
                                                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                            />
                                        </Field>
                                    }
                                    .into_any(),
                                }
                            }}
                            <Field label="Trageort (falls zutreffend)">
                                <FlagSelect
                                    options=WEAR_FLAGS
                                    plain_labels=true
                                    leading=(0, "— keiner —")
                                    value=wearflag
                                    on_change=Callback::new(move |v: i64| u_wear(Box::new(move |x| x.wearflag = v)))
                                    class=IN
                                />
                            </Field>
                        </div>

                        <div class="flex flex-wrap gap-3">
                            <Field label="Beschreibung (Tooltip-Text, optional)">
                                {desc_input}
                            </Field>
                            <Field label="Kurzbeschreibung (optional)">
                                {summary_input}
                            </Field>
                        </div>
                        {move || row_hint().filter(|r| r.description.trim().is_empty() && r.summary.trim().is_empty()).map(|_| view! {
                            <p class="flex items-start gap-2 text-xs text-amber-600">
                                <Icon name="Info" class="mt-0.5 size-3.5 shrink-0"/>
                                "Ohne Beschreibung bleibt das Item im Client ohne Tooltip-Text."
                            </p>
                        })}

                        <div class="space-y-1">
                            <p class="text-xs text-muted-foreground">
                                "Beispielwerte (optional): Referenz-Item übernehmen - Bedeutung von value0-5 hängt vom Typ/Subtyp ab, daher hier bewusst pro Item einzeln."
                            </p>
                            <ReferenceItemPicker on_pick=Callback::new(move |item: ItemProtoInput| u_ref(Box::new(move |x| {
                                x.item_type = item.r#type;
                                x.subtype = item.subtype;
                                x.wearflag = item.wearflag;
                                x.values = [item.value0, item.value1, item.value2, item.value3, item.value4, item.value5];
                                x.ref_item = Some(item);
                            })))/>
                            {move || row_ref().and_then(|r| r.ref_item).map(|ri| view! {
                                <p class="text-xs text-green-600">
                                    "Übernommen von "
                                    <strong>{if ri.locale_name.is_empty() { ri.name.clone() } else { ri.locale_name.clone() }}</strong>
                                    " (vnum " {ri.vnum} ")."
                                </p>
                            })}
                        </div>

                        <div class="space-y-1">
                            <p class="text-xs text-muted-foreground">
                                <strong>"Wichtig:"</strong>
                                " Ein Wert ändert sich pro Aufwertungs-Stufe nur, wenn „wächst mit Stufe\" angehakt ist - ungehakte Werte bleiben für die komplette Kette exakt so, wie hier eingetragen (auch wenn unten eine Kette aktiviert ist)."
                            </p>
                            {move || match value_hints(item_type.get()) {
                                Some(h) => view! {
                                    <p class="flex items-start gap-2 rounded-md bg-muted/40 p-2 text-xs text-muted-foreground">
                                        <Icon name="Info" class="mt-0.5 size-3.5 shrink-0"/>
                                        {h}
                                    </p>
                                }
                                .into_any(),
                                None => view! {
                                    <p class="flex items-start gap-2 rounded-md bg-amber-500/10 p-2 text-xs text-amber-700 dark:text-amber-400">
                                        <Icon name="AlertTriangle" class="mt-0.5 size-3.5 shrink-0"/>
                                        "Für diesen Item-Typ ist die Bedeutung von value0-5 (noch) nicht verifiziert - hängt vom Typ und Subtyp ab. Am zuverlässigsten: Werte über „Referenz-Item übernehmen\" oben von einem existierenden Item desselben Typs holen, statt zu raten."
                                    </p>
                                }
                                .into_any(),
                            }}
                            <div class="flex flex-wrap gap-3">
                                {VALUE_KEYS.iter().enumerate().map(|(i, key)| {
                                    let key = *key;
                                    let (rv, rg) = (row_vals.clone(), row_vals.clone());
                                    let (uv, ug) = (u_vals.clone(), u_vals.clone());
                                    view! {
                                        <div class="flex flex-col gap-1">
                                            <label class="text-xs text-muted-foreground" title=key>
                                                {move || value_labels_by_type(item_type.get()).map(|l| l[i]).unwrap_or(key)}
                                            </label>
                                            <div class="flex items-center gap-1">
                                                <input
                                                    type="number"
                                                    prop:value=move || rv().map(|r| r.values[i]).unwrap_or(0).to_string()
                                                    on:input=move |e| {
                                                        let v = num(&event_target_value(&e));
                                                        uv(Box::new(move |x| x.values[i] = v))
                                                    }
                                                    class="w-20 rounded-md border border-border bg-background px-2 py-1 text-xs"
                                                />
                                                <label class="flex items-center gap-1 text-xs text-muted-foreground">
                                                    <input
                                                        type="checkbox"
                                                        prop:checked=move || rg().is_some_and(|r| r.grow_flags[i])
                                                        on:change=move |e| {
                                                            let c = event_target_checked(&e);
                                                            ug(Box::new(move |x| x.grow_flags[i] = c))
                                                        }
                                                    />
                                                    "wächst mit Stufe"
                                                </label>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        </div>

                        <div class="space-y-1">
                            {
                                let rc = row_chain.clone();
                                view! {
                                    <label class="flex items-center gap-2 text-sm">
                                        <input
                                            type="checkbox"
                                            prop:checked=move || rc().is_some_and(|r| r.chain_enabled)
                                            on:change=move |e| {
                                                let c = event_target_checked(&e);
                                                u_chain(Box::new(move |x| x.chain_enabled = c))
                                            }
                                        />
                                        "Eigene Aufwertungs-Kette für dieses Item"
                                    </label>
                                }
                            }
                            {
                                let rc = row_chain.clone();
                                let enabled = Signal::derive(move || rc().is_some_and(|r| r.chain_enabled));
                                let (r1, r2) = (row_chain.clone(), row_chain.clone());
                                let (ua, ub) = (u_chain2.clone(), u_chain2.clone());
                                view! {
                                    <Show when=move || enabled.get()>
                                        {
                                            let (r1, r2) = (r1.clone(), r2.clone());
                                            let (ua, ub) = (ua.clone(), ub.clone());
                                            view! {
                                                <div class="flex flex-wrap gap-3">
                                                    <Field label="Maximale Stufe (+N)">
                                                        <input
                                                            type="number"
                                                            min="1"
                                                            max="30"
                                                            prop:value=move || r1().map(|r| r.chain_max_level).unwrap_or(9).to_string()
                                                            on:input=move |e| {
                                                                let n = num(&event_target_value(&e));
                                                                let n = if n == 0 { 1 } else { n }.max(1);
                                                                ua(Box::new(move |x| x.chain_max_level = n))
                                                            }
                                                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                                        />
                                                    </Field>
                                                    <Field label="Wachstum pro Stufe (%)">
                                                        <input
                                                            type="number"
                                                            min="0"
                                                            prop:value=move || r2().map(|r| r.chain_growth_percent).unwrap_or(10).to_string()
                                                            on:input=move |e| {
                                                                let n = num(&event_target_value(&e)).max(0);
                                                                ub(Box::new(move |x| x.chain_growth_percent = n))
                                                            }
                                                            class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                                        />
                                                    </Field>
                                                </div>
                                            }
                                        }
                                    </Show>
                                }
                            }
                        </div>
                    </div>
                </details>
            </div>
        }
    };

    let scanned_view = move |paths: Vec<String>| {
        let count = paths.len();
        view! {
            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"2. Gefundene Icons (" {count} ")"</h2>
                <Field label="VNUM-Bereich ab">
                    <input
                        type="number"
                        prop:value=move || vnum_range_start.get().to_string()
                        on:input=move |e| vnum_range_start.set(num(&event_target_value(&e)))
                        class="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                    />
                </Field>
                <div class="space-y-2">{paths.into_iter().map(row_view).collect_view()}</div>
            </section>

            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"3. Importieren"</h2>
                <Button
                    disabled=Signal::derive(move || !can_import.get() || running.get())
                    on_click=move |_| confirm.set(true)
                >
                    {move || if running.get() {
                        "Importiere…".to_string()
                    } else {
                        format!("{} Item(s) importieren", selected_paths().len())
                    }}
                </Button>

                <Show when=move || item_status.with(|s| !s.is_empty())>
                    <div class="space-y-1 text-sm">
                        <StepRow
                            spacer_class="size-4 shrink-0"
                            label="Voraussetzungen prüfen (Tool-Pfade)"
                            status=Signal::derive(move || other_steps.with(|s| s.get("setup").copied()))
                        />
                        {move || selected_paths().into_iter().map(|p| {
                            let p2 = p.clone();
                            let p3 = p.clone();
                            view! {
                                <StepRow
                                    spacer_class="size-4 shrink-0"
                                    label=Signal::derive(move || {
                                        rows.with(|r| r.get(&p2).map(|x| x.locale_name.clone()).filter(|n| !n.is_empty()))
                                            .unwrap_or_else(|| file_name_of(&p2))
                                    })
                                    status=Signal::derive(move || item_status.with(|s| s.get(&p3).copied()))
                                />
                            }
                        }).collect_view()}
                        <StepRow
                            spacer_class="size-4 shrink-0"
                            label="icon.epk neu packen"
                            status=Signal::derive(move || other_steps.with(|s| s.get("pack").copied()))
                        />
                        <StepRow
                            spacer_class="size-4 shrink-0"
                            label="item_proto aus DB erzeugen & in Client einspielen"
                            status=Signal::derive(move || other_steps.with(|s| s.get("proto").copied()))
                        />
                    </div>
                </Show>

                {move || batch_error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}
                {move || batch_done.get().map(|d| view! {
                    <p class="flex items-center gap-1 text-sm text-green-600">
                        <Icon name="CheckCircle2" class="size-4"/>
                        " " {d.len()} " Item(s) angelegt: vnums " {join_vnums(&d)}
                    </p>
                })}
            </section>
        }
    };

    view! {
        <div class="space-y-6">
            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"1. Icon-Ordner wählen"</h2>
                <p class="text-xs text-muted-foreground">
                    "Für Items ohne eigenes 3D-Modell: Schuhe, Ketten, Schilder, Ohrringe, Armbänder, Verbrauchsgegenstände usw. - jede Bilddatei im gewählten Ordner (auch in Unterordnern) wird als eigenes Item angeboten."
                </p>
                <div class="flex items-center gap-2">
                    <Button variant=ButtonVariant::Outline on_click=move |_| pick_folder()>
                        <Icon name="FolderOpen" class="size-4"/>
                        "Ordner wählen…"
                    </Button>
                    <span class="truncate text-sm text-muted-foreground">
                        {move || {
                            let p = folder_path.get();
                            if p.is_empty() { "Kein Ordner gewählt".to_string() } else { p }
                        }}
                    </span>
                    <Button
                        on_click=move |_| scan()
                        disabled=Signal::derive(move || folder_path.with(|p| p.is_empty()) || scanning.get())
                    >
                        {move || if scanning.get() { "Scanne…" } else { "Scannen" }}
                    </Button>
                </div>
                {move || scan_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
            </section>

            {move || icon_paths.get().map(scanned_view)}

            {move || confirm.get().then(|| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">{selected_paths().len()} " Item(s) jetzt anlegen?"</p>
                        <ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                            <li>"Ein Datenbankeintrag + Icon pro gewähltem Bild, kein 3D-Modell"</li>
                            <li>"Items mit aktivierter Aufwertungs-Kette bekommen zusätzliche Stufen (eigenes Icon geteilt)"</li>
                            <li>
                                <code>"icon.epk"</code> " neu gepackt, " <code>"item_list.txt"</code> " ergänzt (Backups vorher)"
                            </li>
                            <li>
                                <code>"item_proto"</code> " wird aus der DB neu erzeugt und im Client ersetzt (Backup vorher)"
                            </li>
                        </ul>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm.set(false)>"Abbrechen"</Button>
                            <Button on_click=move |_| run_import()>"Ausführen"</Button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::suggested_name;

    #[test]
    fn suggests_names_from_file_names() {
        assert_eq!(suggested_name("C:/x/my_cool--ring.tga"), "my cool ring");
        assert_eq!(suggested_name("/x/___.png"), "___");
        assert_eq!(suggested_name("/x/noext"), "noext");
    }
}
