//! Waffen-/Rüstungs-Pakete (Icons + 3D-Modelle) scannen und als Items samt
//! Client-Dateien importieren (Port von `PackageImporter.tsx`).

use super::icon_picker::IconPicker;
use super::reference_item_picker::ReferenceItemPicker;
use crate::components::{Button, Icon};
use crate::features::item_editor::components::shared::{Field, FlagSelect, StepRow};
use crate::features::item_editor::item_flags::{subtypes_by_type, WEAR_FLAGS};
use crate::features::item_editor::types::{js_number, num};
use crate::features::module_importer::shared::*;
use crate::util::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::{dialog_open_one, invoke, invoke_unit, listen, OpenDialogOptions, Unlisten};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::{BTreeSet, HashMap};

#[derive(Clone, Debug, PartialEq)]
struct WeaponRowState {
    selected: bool,
    /// -1 = noch nicht gewählt
    subtype: i64,
    is_sura: bool,
    icon: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct ArmorRowState {
    selected: bool,
    icon: Option<String>,
}

/// Eingaben, aus denen die Items gebaut werden (Schnappschuss beim Start).
#[derive(Clone)]
struct BuildInput {
    ref_item: Option<ItemProtoInput>,
    base_name: String,
    base_locale_name: String,
    chain_growth_percent: i64,
    armor_subtype: i64,
    armor_wearflag: i64,
}

fn or_else<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.is_empty() {
        b
    } else {
        a
    }
}

fn build_weapon_item(inp: &BuildInput, vnum: i64, variant: &WeaponVariant, row: &WeaponRowState, level: i64) -> ItemProtoInput {
    let template = inp.ref_item.clone().unwrap_or_else(|| empty_item(vnum, 1));
    // Klassensperre kommt immer aus Subtyp/Modell der Variante, nie vom
    // Referenz-Item; übrige antiflag-Bits des Referenz-Items bleiben.
    let antiflag = (template.antiflag & !CLASS_ANTIFLAG_MASK) | class_antiflag_for_weapon(row.subtype, row.is_sura);
    let suffix = if level > 0 { format!("+{level}") } else { String::new() };
    let grow = |base: i64| scaled_value(base, level, inp.chain_growth_percent);
    ItemProtoInput {
        vnum,
        vnum_range: 0,
        name: fit_to_byte_limit_with_suffix(
            &format!("{}_{}", or_else(&inp.base_name, "item"), variant.key),
            &suffix,
            ITEM_NAME_MAX_BYTES,
        ),
        locale_name: fit_to_byte_limit_with_suffix(
            &format!("{} ({})", or_else(&inp.base_locale_name, &inp.base_name), label_for_variant(variant)),
            &if suffix.is_empty() { String::new() } else { format!(" {suffix}") },
            ITEM_NAME_MAX_BYTES,
        ),
        r#type: 1,
        subtype: row.subtype,
        antiflag,
        size: size_for_subtype(row.subtype),
        wearflag: if template.wearflag != 0 { template.wearflag } else { WEAPON_WEARFLAG },
        // Schaden min/max + Zusatz-Angriffskraft wachsen gemeinsam.
        value3: grow(template.value3),
        value4: grow(template.value4),
        value5: grow(template.value5),
        // Kette wird danach explizit über set_item_refine_link verdrahtet.
        refine_set: 0,
        refined_vnum: 0,
        ..template
    }
}

fn build_armor_item(inp: &BuildInput, vnum: i64, shape_index: i64, selected_race_keys: &[String], level: i64) -> ItemProtoInput {
    let template = inp.ref_item.clone().unwrap_or_else(|| empty_item(vnum, 2));
    let blocked_mask = if selected_race_keys.is_empty() {
        0
    } else {
        RACE_ANTIFLAG
            .iter()
            .filter(|(race, _)| !selected_race_keys.iter().any(|k| k == race))
            .fold(0, |acc, (_, bit)| acc | bit)
    };
    let antiflag = (template.antiflag & !CLASS_ANTIFLAG_MASK) | blocked_mask;
    let suffix = if level > 0 { format!(" +{level}") } else { String::new() };
    let grow = |base: i64| scaled_value(base, level, inp.chain_growth_percent);
    ItemProtoInput {
        vnum,
        vnum_range: 0,
        name: fit_to_byte_limit_with_suffix(
            &format!("{}_armor", or_else(&inp.base_name, "item")),
            &if level > 0 { format!("+{level}") } else { String::new() },
            ITEM_NAME_MAX_BYTES,
        ),
        locale_name: fit_to_byte_limit_with_suffix(
            or_else(&inp.base_locale_name, &inp.base_name),
            &suffix,
            ITEM_NAME_MAX_BYTES,
        ),
        r#type: 2,
        subtype: inp.armor_subtype,
        antiflag,
        // Rüstung ist im Inventar immer 1 Slot groß - nie vom Referenz-Item erben.
        size: 1,
        wearflag: if inp.armor_wearflag != 0 { inp.armor_wearflag } else { template.wearflag },
        // value3 ist bei Rüstung der 3D-Modell-Shape-Index und bleibt konstant.
        value0: grow(template.value0),
        value1: grow(template.value1),
        value5: grow(template.value5),
        value3: shape_index,
        refine_set: 0,
        refined_vnum: 0,
        ..template
    }
}

fn race_list(armors: &[ArmorPiece]) -> String {
    armors.iter().map(|a| race_label(&a.race)).collect::<Vec<_>>().join(", ")
}

async fn wire_refine_chain(level_vnums: &[i64]) -> Result<(), String> {
    for level in 0..level_vnums.len().saturating_sub(1) {
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
    Ok(())
}

const IN_SM: &str = "rounded-md border border-border bg-background px-2 py-1 text-xs";

#[component]
pub fn PackageImporter(#[prop(into)] on_imported: Callback<()>) -> impl IntoView {
    let folder_path = RwSignal::new(String::new());
    let scanning = RwSignal::new(false);
    let scan_result = RwSignal::new(None::<ScannedModule>);
    let scan_error = RwSignal::new(None::<String>);

    let weapon_rows = RwSignal::new(HashMap::<String, WeaponRowState>::new());
    let armor_rows = RwSignal::new(HashMap::<String, ArmorRowState>::new());
    let armor_subtype = RwSignal::new(0i64);
    let armor_wearflag = RwSignal::new(1i64 << 0); // "Rüstung (Körper)"

    let base_name = RwSignal::new(String::new());
    let base_locale_name = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let summary = RwSignal::new(String::new());
    let vnum_range_start = RwSignal::new(500000i64);
    let include_effects = RwSignal::new(true);
    let ref_item = RwSignal::new(None::<ItemProtoInput>);

    let chain_enabled = RwSignal::new(false);
    let chain_max_level = RwSignal::new(9i64);
    let chain_growth_percent = RwSignal::new(10i64);

    let running = RwSignal::new(false);
    let weapon_status = RwSignal::new(HashMap::<String, StepStatus>::new());
    let armor_status = RwSignal::new(None::<StepStatus>);
    let other_steps = RwSignal::new(HashMap::<&'static str, StepStatus>::new());
    let log = RwSignal::new(Vec::<String>::new());
    let batch_error = RwSignal::new(None::<String>);
    let batch_done = RwSignal::new(None::<Vec<i64>>);
    let confirm = RwSignal::new(false);

    // Gescanntes, noch nicht importiertes Paket = ungespeicherte Arbeit.
    let dirty = Memo::new(move |_| scan_result.with(|s| s.is_some()) && batch_done.with(|b| b.is_none()));
    Effect::new(move |_| report_section_dirty(Section::ModuleImporter, dirty.get()));
    on_cleanup(|| report_section_dirty(Section::ModuleImporter, false));

    let log_ref = NodeRef::<html::Pre>::new();

    spawn_local(async move {
        if let Ok(v) = invoke::<Option<String>>("get_setting", &json!({ "key": "item_vnum_range_start" })).await {
            vnum_range_start.set(js_number(v.as_deref().unwrap_or("500000")) as i64);
        }
    });

    let unlisten = StoredValue::new_local(None::<Unlisten>);
    let disposed = StoredValue::new(false);
    spawn_local(async move {
        if let Ok(u) = listen::<String>("item-editor-output", move |line| log.update(|l| l.push(line))).await {
            if disposed.try_get_value().unwrap_or(true) {
                u.unlisten();
            } else {
                unlisten.set_value(Some(u));
            }
        }
    });
    on_cleanup(move || {
        disposed.set_value(true);
        unlisten.with_value(|u| {
            if let Some(u) = u {
                u.unlisten();
            }
        });
    });

    Effect::new(move |_| {
        log.track();
        request_animation_frame(move || {
            if let Some(el) = log_ref.get_untracked() {
                el.set_scroll_top(el.scroll_height());
            }
        });
    });

    let pick_folder = move || {
        spawn_local(async move {
            let res = dialog_open_one(&OpenDialogOptions {
                directory: true,
                title: Some("Paket-Ordner wählen".into()),
                ..Default::default()
            })
            .await;
            if let Ok(Some(p)) = res {
                folder_path.set(p);
                scan_result.set(None);
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
            match invoke::<ScannedModule>("scan_module", &json!({ "path": path })).await {
                Ok(result) => {
                    let w_rows = result
                        .weapons
                        .iter()
                        .map(|v| {
                            (
                                v.key.clone(),
                                WeaponRowState {
                                    selected: v.icon_source_abs.is_some(),
                                    subtype: v.subtype,
                                    is_sura: v.is_sura_model,
                                    icon: v.icon_source_abs.clone(),
                                },
                            )
                        })
                        .collect();
                    weapon_rows.set(w_rows);
                    let a_rows = result
                        .armors
                        .iter()
                        .map(|a| {
                            (
                                a.race.clone(),
                                ArmorRowState {
                                    selected: a.female_model_abs.is_some() || a.male_model_abs.is_some(),
                                    icon: a.icon_source_abs.clone(),
                                },
                            )
                        })
                        .collect();
                    armor_rows.set(a_rows);
                    if base_name.with_untracked(|b| b.is_empty()) {
                        base_name.set(result.module_name.clone());
                    }
                    if base_locale_name.with_untracked(|b| b.is_empty()) {
                        base_locale_name.set(result.module_name.clone());
                    }
                    scan_result.set(Some(result));
                    weapon_status.set(HashMap::new());
                    armor_status.set(None);
                    other_steps.set(HashMap::new());
                    batch_error.set(None);
                    batch_done.set(None);
                }
                Err(e) => {
                    scan_error.set(Some(e));
                    scan_result.set(None);
                }
            }
            scanning.set(false);
        });
    };

    let selected_weapons = move || -> Vec<WeaponVariant> {
        scan_result.with(|s| {
            s.as_ref()
                .map(|s| {
                    s.weapons
                        .iter()
                        .filter(|v| weapon_rows.with(|r| r.get(&v.key).is_some_and(|x| x.selected)))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let selected_armor_races = move || -> Vec<ArmorPiece> {
        scan_result.with(|s| {
            s.as_ref()
                .map(|s| {
                    s.armors
                        .iter()
                        .filter(|a| armor_rows.with(|r| r.get(&a.race).is_some_and(|x| x.selected)))
                        .cloned()
                        .collect()
                })
                .unwrap_or_default()
        })
    };
    let armor_female_races =
        move || selected_armor_races().into_iter().filter(|a| a.female_model_abs.is_some()).collect::<Vec<_>>();
    let armor_male_only_races = move || {
        selected_armor_races()
            .into_iter()
            .filter(|a| a.female_model_abs.is_none() && a.male_model_abs.is_some())
            .collect::<Vec<_>>()
    };

    let can_import = Memo::new(move |_| {
        let sw = selected_weapons();
        scan_result.with(|s| s.is_some())
            && (!sw.is_empty() || !selected_armor_races().is_empty())
            && !base_name.with(|b| b.trim().is_empty())
            && !base_locale_name.with(|b| b.trim().is_empty())
            && sw.iter().all(|v| weapon_rows.with(|r| r.get(&v.key).map(|x| x.subtype) != Some(-1)))
    });

    use_save_shortcut(
        Section::ModuleImporter,
        move || dirty.get_untracked() && can_import.get_untracked() && !running.get_untracked(),
        move || confirm.set(true),
    );

    let item_count = move || {
        (selected_weapons().len() as i64 + if selected_armor_races().is_empty() { 0 } else { 1 })
            * if chain_enabled.get() { chain_max_level.get() + 1 } else { 1 }
    };

    let run_import = move || {
        confirm.set(false);
        running.set(true);
        batch_error.set(None);
        batch_done.set(None);
        log.set(vec![]);

        let Some(scan) = scan_result.get_untracked() else {
            running.set(false);
            return;
        };
        let sel_weapons = selected_weapons();
        let sel_armors = selected_armor_races();
        let female_races = armor_female_races();
        let w_rows = weapon_rows.get_untracked();
        let a_rows = armor_rows.get_untracked();
        let inp = BuildInput {
            ref_item: ref_item.get_untracked(),
            base_name: base_name.get_untracked(),
            base_locale_name: base_locale_name.get_untracked(),
            chain_growth_percent: chain_growth_percent.get_untracked(),
            armor_subtype: armor_subtype.get_untracked(),
            armor_wearflag: armor_wearflag.get_untracked(),
        };
        let desc = description.get_untracked();
        let summ = summary.get_untracked();
        let has_desc = !desc.trim().is_empty() || !summ.trim().is_empty();
        let range_start0 = vnum_range_start.get_untracked();
        let levels = if chain_enabled.get_untracked() { chain_max_level.get_untracked() + 1 } else { 1 };
        let effects = include_effects.get_untracked();

        weapon_status.set(sel_weapons.iter().map(|v| (v.key.clone(), StepStatus::Pending)).collect());
        armor_status.set((!sel_armors.is_empty()).then_some(StepStatus::Pending));
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
            let mut touched_race_packs = BTreeSet::<String>::new();
            let result: Result<(), String> = async {
                set_other("setup", StepStatus::Running);
                invoke_unit("validate_item_editor_setup", &json!({ "requireIconTool": true })).await?;
                set_other("setup", StepStatus::Done);

                let mut range_start = range_start0;

                for variant in &sel_weapons {
                    let Some(row) = w_rows.get(&variant.key).cloned() else { continue };
                    weapon_status.update(|m| {
                        m.insert(variant.key.clone(), StepStatus::Running);
                    });

                    // Eine ganze Kette braucht einen lückenlosen vnum-Block;
                    // Icon/Modell der Basis-vnum teilen sich alle Stufen.
                    let base_vnum = find_consecutive_free_vnums(range_start, levels).await?;
                    range_start = base_vnum + levels;

                    if let Some(icon) = &row.icon {
                        invoke_unit("write_item_icon", &json!({ "vnum": base_vnum, "sourcePath": icon })).await?;
                    }
                    let (_, virtual_model_path) = invoke::<(String, String)>(
                        "import_weapon_model",
                        &json!({
                            "moduleName": scan.module_name,
                            "vnum": base_vnum,
                            "sourceAbs": variant.model_source_abs,
                            "textureSources": variant.texture_sources_abs,
                        }),
                    )
                    .await?;
                    let icon_rel_path = format!("icon/item/{}.tga", pad5(base_vnum));

                    let mut level_vnums = vec![];
                    for level in 0..levels {
                        let vnum = base_vnum + level;
                        level_vnums.push(vnum);
                        let item = build_weapon_item(&inp, vnum, variant, &row, level);
                        invoke_unit("create_item_proto", &json!({ "item": item })).await?;
                        created.push(vnum);
                        if has_desc {
                            invoke_unit("write_item_desc", &json!({ "vnum": vnum, "description": desc, "summary": summ }))
                                .await?;
                        }
                        invoke_unit(
                            "write_item_list_entry",
                            &json!({ "vnum": vnum, "itemType": 1, "iconRelPath": icon_rel_path, "modelRelPath": virtual_model_path }),
                        )
                        .await?;
                    }
                    wire_refine_chain(&level_vnums).await?;

                    weapon_status.update(|m| {
                        m.insert(variant.key.clone(), StepStatus::Done);
                    });
                }

                if !sel_armors.is_empty() {
                    armor_status.set(Some(StepStatus::Running));

                    let base_vnum = find_consecutive_free_vnums(range_start, levels).await?;
                    range_start = base_vnum + levels;
                    let _ = range_start;

                    let shape_index = if !female_races.is_empty() {
                        invoke::<i64>("next_free_shape_index", &json!({})).await?
                    } else {
                        inp.ref_item.as_ref().map(|r| r.value3).unwrap_or(0)
                    };

                    if let Some(icon) = a_rows.get(&sel_armors[0].race).and_then(|r| r.icon.clone()) {
                        invoke_unit("write_item_icon", &json!({ "vnum": base_vnum, "sourcePath": icon })).await?;
                    }
                    let icon_rel_path = format!("icon/item/{}.tga", pad5(base_vnum));
                    let race_keys: Vec<String> = sel_armors.iter().map(|a| a.race.clone()).collect();

                    let mut level_vnums = vec![];
                    for level in 0..levels {
                        let vnum = base_vnum + level;
                        level_vnums.push(vnum);
                        let item = build_armor_item(&inp, vnum, shape_index, &race_keys, level);
                        invoke_unit("create_item_proto", &json!({ "item": item })).await?;
                        created.push(vnum);
                        if has_desc {
                            invoke_unit("write_item_desc", &json!({ "vnum": vnum, "description": desc, "summary": summ }))
                                .await?;
                        }
                        invoke_unit(
                            "write_item_list_entry",
                            &json!({ "vnum": vnum, "itemType": 2, "iconRelPath": icon_rel_path, "modelRelPath": null }),
                        )
                        .await?;
                    }
                    wire_refine_chain(&level_vnums).await?;

                    // Das 3D-Modell hängt nur am Shape-Index der Basis-vnum.
                    for armor in &female_races {
                        invoke_unit(
                            "import_armor_model",
                            &json!({
                                "moduleName": scan.module_name,
                                "race": armor.race,
                                "sourceAbs": armor.female_model_abs,
                                "textureSources": armor.female_textures_abs,
                                "shapeIndex": shape_index,
                            }),
                        )
                        .await?;
                        touched_race_packs.insert(format!("pc_{}", armor.race));
                    }
                    armor_status.set(Some(StepStatus::Done));
                }

                set_other("pack", StepStatus::Running);
                if !sel_weapons.is_empty() {
                    invoke_unit("pack_item_icons", &json!({})).await?;
                    invoke_unit("pack_item_models", &json!({})).await?;
                } else if !sel_armors.is_empty() {
                    invoke_unit("pack_item_icons", &json!({})).await?;
                }
                for folder in &touched_race_packs {
                    invoke_unit("pack_folder", &json!({ "folderName": folder })).await?;
                }
                if effects && !scan.effect_dirs.is_empty() {
                    for dir in &scan.effect_dirs {
                        invoke_unit("import_effect_bundle", &json!({ "moduleName": scan.module_name, "sourceDir": dir }))
                            .await?;
                    }
                    invoke_unit("pack_item_effects", &json!({})).await?;
                }
                set_other("pack", StepStatus::Done);

                set_other("proto", StepStatus::Running);
                let generated = invoke::<String>("regenerate_item_proto", &json!({})).await?;
                invoke_unit("deploy_item_proto", &json!({ "generatedProtoPath": generated })).await?;
                set_other("proto", StepStatus::Done);

                batch_done.set(Some(created.clone()));
                // Verlauf ist Best-Effort - der Import selbst war erfolgreich.
                let recorded = invoke_unit(
                    "record_import_batch",
                    &json!({
                        "moduleName": scan.module_name,
                        "itemType": if !sel_weapons.is_empty() && sel_armors.is_empty() { 1 } else { 2 },
                        "vnums": created,
                        "hadEffects": effects && !scan.effect_dirs.is_empty(),
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
                        format!(
                            "\n\nAlle {} bereits angelegten Einträge (Datenbank, Icon, item_list.txt) wurden automatisch zurückgenommen.",
                            created.len()
                        )
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

    // ---------------------------------------------------------------- Waffen-Zeilen
    let weapon_row_view = move |v: WeaponVariant, pool: Vec<String>| {
        let key = v.key.clone();
        let k = key.clone();
        let row = move || weapon_rows.with(|r| r.get(&k).cloned());
        let update = {
            let key = key.clone();
            move |f: &dyn Fn(&mut WeaponRowState)| {
                weapon_rows.update(|r| {
                    if let Some(x) = r.get_mut(&key) {
                        f(x)
                    }
                })
            }
        };
        let label = label_for_variant(&v);
        let label2 = label.clone();
        let wanted = move || format!("{} ({})", or_else(&base_locale_name.get(), &base_name.get()), label2);
        let final_name = {
            let wanted = wanted.clone();
            move || fit_to_byte_limit(&wanted(), ITEM_NAME_MAX_BYTES)
        };
        let truncated = {
            let (w, f) = (wanted.clone(), final_name.clone());
            move || f() != w()
        };
        let truncated_cls = truncated.clone();
        let row_c = row.clone();
        let subtype = Signal::derive(move || row_c().map(|r| r.subtype).unwrap_or(-1));
        let row_c = row.clone();
        let icon_sig = Signal::derive(move || row_c().and_then(|r| r.icon));
        let (u1, u2, u3, u4) = (update.clone(), update.clone(), update.clone(), update.clone());
        let row_c = row.clone();
        let row_c2 = row.clone();
        let textures = v.texture_sources_abs.len();
        view! {
            <div class="flex flex-wrap items-center gap-3 rounded-md border border-border p-2 text-sm">
                <input
                    type="checkbox"
                    prop:checked=move || row_c().is_some_and(|r| r.selected)
                    on:change=move |e| {
                        let c = event_target_checked(&e);
                        u1(&|x| x.selected = c)
                    }
                />
                <span class="w-40 font-medium">{label}</span>
                <IconPicker
                    pool=pool
                    value=icon_sig
                    on_change=Callback::new(move |icon: Option<String>| u2(&|x| x.icon = icon.clone()))
                />
                {move || icon_sig.with(|i| i.is_none()).then(|| view! {
                    <span class="flex items-center gap-1 text-xs text-destructive">
                        <Icon name="Image" class="size-3.5"/>
                        " Kein Icon gewählt"
                    </span>
                })}
                <Field label="Subtyp">
                    <FlagSelect
                        options=subtypes_by_type(1).unwrap_or(&[])
                        leading=(-1, "— wählen —")
                        plain_labels=true
                        value=subtype
                        on_change=Callback::new(move |s: i64| u3(&|x| x.subtype = s))
                        class=IN_SM
                    />
                </Field>
                {move || (subtype.get() == 0).then(|| {
                    let u4 = u4.clone();
                    let row_c2 = row_c2.clone();
                    view! {
                        <label class="flex items-center gap-1 text-xs text-muted-foreground">
                            <input
                                type="checkbox"
                                prop:checked=move || row_c2().is_some_and(|r| r.is_sura)
                                on:change=move |e| {
                                    let c = event_target_checked(&e);
                                    u4(&|x| x.is_sura = c)
                                }
                            />
                            "Sura-exklusiv"
                        </label>
                    }
                })}
                {move || row().filter(|r| r.subtype != -1).map(|r| view! {
                    <span class="text-xs text-muted-foreground">
                        "Klasse: " <strong>{class_label_for(class_antiflag_for_weapon(r.subtype, r.is_sura))}</strong>
                    </span>
                })}
                <span class="ml-auto truncate text-xs text-muted-foreground" title=v.model_source_abs.clone()>
                    {file_name_of(&v.model_source_abs)}
                    {(textures > 0).then(|| format!(" (+{textures} Textur(en))"))}
                </span>
                <span class={
                    move || if truncated_cls() { "w-full text-xs text-amber-600" } else { "w-full text-xs text-muted-foreground" }
                }>
                    "Anzeigename: " <code>{final_name.clone()}</code>
                    {move || truncated().then(|| format!(" (gekürzt, max. {ITEM_NAME_MAX_BYTES} Zeichen im Client)"))}
                </span>
            </div>
        }
    };

    // ---------------------------------------------------------------- Rüstungs-Zeilen
    let armor_row_view = move |a: ArmorPiece, pool: Vec<String>| {
        let race = a.race.clone();
        let r1 = race.clone();
        let r2 = race.clone();
        let r3 = race.clone();
        let r4 = race.clone();
        let has_female = a.female_model_abs.is_some();
        let has_male = a.male_model_abs.is_some();
        view! {
            <div class="flex flex-wrap items-center gap-3 rounded-md border border-border p-2 text-sm">
                <input
                    type="checkbox"
                    prop:checked=move || armor_rows.with(|r| r.get(&r1).is_some_and(|x| x.selected))
                    on:change=move |e| {
                        let c = event_target_checked(&e);
                        armor_rows.update(|r| {
                            if let Some(x) = r.get_mut(&r2) {
                                x.selected = c
                            }
                        })
                    }
                />
                <span class="w-28 font-medium">{race_label(&race)}</span>
                <IconPicker
                    pool=pool
                    value=Signal::derive(move || armor_rows.with(|r| r.get(&r3).and_then(|x| x.icon.clone())))
                    on_change=Callback::new(move |icon: Option<String>| {
                        armor_rows.update(|r| {
                            if let Some(x) = r.get_mut(&r4) {
                                x.icon = icon
                            }
                        })
                    })
                />
                {if has_female {
                    view! {
                        <span class="flex items-center gap-1 text-xs text-green-600">
                            <Icon name="CheckCircle2" class="size-3.5"/>
                            " 3D-Modell wird automatisch verknüpft"
                        </span>
                    }
                    .into_any()
                } else if has_male {
                    view! {
                        <span
                            class="flex items-center gap-1 text-xs text-amber-600"
                            title="Männliche Körpermodelle sind clientseitig fest kodiert (kein Datei-basiertes Mapping vorhanden) - nur Icon + Item werden angelegt."
                        >
                            <Icon name="AlertTriangle" class="size-3.5"/>
                            " Nur männliches Modell - 3D nicht automatisierbar"
                        </span>
                    }
                    .into_any()
                } else {
                    view! { <span class="text-xs text-muted-foreground">"Kein 3D-Modell gefunden"</span> }.into_any()
                }}
            </div>
        }
    };

    let scanned_view = move |scan: ScannedModule| {
        let has_weapons = !scan.weapons.is_empty();
        let weapons_count = scan.weapons.len();
        let armors_count = scan.armors.len();
        let effect_dirs = scan.effect_dirs.len();
        view! {
            {(!scan.notices.is_empty()).then(|| view! {
                <div class="space-y-1 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                    {scan.notices.iter().map(|n| view! {
                        <p class="flex items-start gap-2"><Icon name="Info" class="mt-0.5 size-4 shrink-0"/> " " {n.clone()}</p>
                    }).collect_view()}
                </div>
            })}

            {(scan.weapons.is_empty() && scan.armors.is_empty()).then(|| view! {
                <p class="text-sm text-muted-foreground">
                    "Keine Waffen-Modelle (" <code>".gr2"</code> " außerhalb von " <code>"pc/pc2/pc3"</code>
                    ") und keine Rüstungs-Körpermodelle gefunden."
                </p>
            })}

            {has_weapons.then(|| view! {
                <section class="space-y-3 rounded-lg border border-border p-4">
                    <h2 class="text-sm font-medium text-muted-foreground">"2. Erkannte Waffen (" {weapons_count} ")"</h2>
                    <div class="space-y-2">
                        {scan.weapons.iter().map(|v| weapon_row_view(v.clone(), scan.icon_pool.clone())).collect_view()}
                    </div>
                </section>
            })}

            {(!scan.armors.is_empty()).then(|| view! {
                <section class="space-y-3 rounded-lg border border-border p-4">
                    <h2 class="text-sm font-medium text-muted-foreground">
                        {if has_weapons { "3" } else { "2" }} ". Erkannte Rüstung (" {armors_count} " Rasse(n))"
                    </h2>
                    <p class="text-xs text-muted-foreground">
                        "Alle ausgewählten Rassen bilden " <strong>"ein gemeinsames Item"</strong>
                        " (eine vnum), das je Rasse mit deren eigenem 3D-Modell erscheint - nicht ausgewählte Klassen können es nicht anlegen."
                    </p>
                    <div class="space-y-2">
                        {scan.armors.iter().map(|a| armor_row_view(a.clone(), scan.icon_pool.clone())).collect_view()}
                    </div>
                    {move || {
                        let male_only = armor_male_only_races();
                        (!male_only.is_empty()).then(|| view! {
                            <p class="flex items-start gap-2 text-xs text-muted-foreground">
                                <Icon name="Info" class="mt-0.5 size-3.5 shrink-0"/>
                                "Männliche Rüstungs-Körpermodelle sind im Client fest verdrahtet (keine Daten-Datei wie bei der weiblichen Seite existiert dafür) - für "
                                {race_list(&male_only)}
                                " wird nur das Item + Icon angelegt, das 3D-Modell bleibt wie bisher manuelle Arbeit."
                            </p>
                        })
                    }}
                    <div class="flex flex-wrap gap-3 pt-1">
                        <Field label="Subtyp">
                            <FlagSelect
                                options=subtypes_by_type(2).unwrap_or(&[])
                                plain_labels=true
                                value=armor_subtype
                                on_change=Callback::new(move |v| armor_subtype.set(v))
                                class=IN_SM
                            />
                        </Field>
                        <Field label="Trageort">
                            <FlagSelect
                                options=WEAR_FLAGS
                                plain_labels=true
                                value=armor_wearflag
                                on_change=Callback::new(move |v| armor_wearflag.set(v))
                                class=IN_SM
                            />
                        </Field>
                    </div>
                </section>
            })}

            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Basisdaten"</h2>
                <div class="flex flex-wrap gap-3">
                    <Field label="Name (intern, ASCII)">
                        <input
                            prop:value=move || base_name.get()
                            on:input=move |e| base_name.set(event_target_value(&e))
                            class="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <Field label="Anzeigename">
                        <input
                            prop:value=move || base_locale_name.get()
                            on:input=move |e| base_locale_name.set(event_target_value(&e))
                            class="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <Field label="VNUM-Bereich ab">
                        <input
                            type="number"
                            prop:value=move || vnum_range_start.get().to_string()
                            on:input=move |e| vnum_range_start.set(num(&event_target_value(&e)))
                            class="w-32 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                </div>
                <div class="flex flex-wrap gap-3">
                    <Field label="Beschreibung (Tooltip-Text, optional)">
                        <textarea
                            prop:value=move || description.get()
                            on:input=move |e| description.set(event_target_value(&e))
                            class="h-16 w-72 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        ></textarea>
                    </Field>
                    <Field label="Kurzbeschreibung (optional)">
                        <textarea
                            prop:value=move || summary.get()
                            on:input=move |e| summary.set(event_target_value(&e))
                            class="h-16 w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        ></textarea>
                    </Field>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Gilt für alle importierten Varianten/Stufen dieses Durchlaufs. Rein clientseitiger Tooltip-Text ("
                    <code>"locale/<lang>/itemdesc.txt"</code>
                    ") - ohne Eintrag bleibt das Item im Client ohne Beschreibung."
                </p>
                {(effect_dirs > 0).then(|| view! {
                    <label class="flex items-center gap-2 text-sm">
                        <input
                            type="checkbox"
                            prop:checked=move || include_effects.get()
                            on:change=move |e| include_effects.set(event_target_checked(&e))
                        />
                        {effect_dirs}
                        " Glow-Effekt-Ordner mitkopieren (Best-Effort - danach im Client visuell prüfen)"
                    </label>
                })}
            </section>

            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Beispielwerte (optional): Referenz-Item übernehmen"</h2>
                <p class="text-xs text-muted-foreground">
                    "Übernimmt Werte (value0-5, Preise, Flags) eines bestehenden Items als Ausgangsbasis für "
                    <strong>"alle"</strong>
                    " ausgewählten Waffen/Rüstung - Typ/Subtyp/VNUM/Name/Klassenbeschränkung werden trotzdem korrekt je Variante gesetzt. Ohne Auswahl bleiben die Werte auf 0 (bewusst kein Erraten von Balance-Zahlen)."
                </p>
                <ReferenceItemPicker on_pick=Callback::new(move |i: ItemProtoInput| ref_item.set(Some(i)))/>
                {move || ref_item.get().map(|r| view! {
                    <p class="text-xs text-green-600">
                        "Übernommen von "
                        <strong>{if r.locale_name.is_empty() { r.name.clone() } else { r.locale_name.clone() }}</strong>
                        " (vnum " {r.vnum} "). "
                        <button class="underline" on:click=move |_| ref_item.set(None)>"Entfernen"</button>
                    </p>
                })}
            </section>

            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Aufwertungs-Kette (optional)"</h2>
                <label class="flex items-center gap-2 text-sm">
                    <input
                        type="checkbox"
                        prop:checked=move || chain_enabled.get()
                        on:change=move |e| chain_enabled.set(event_target_checked(&e))
                    />
                    "Jedes Item bekommt eine eigene Aufwertungs-Kette (+0 bis +N), aufrüstbar wie bei Stock-Items"
                </label>
                <Show when=move || chain_enabled.get()>
                    <div class="flex flex-wrap gap-3">
                        <Field label="Maximale Stufe (+N)">
                            <input
                                type="number"
                                min="1"
                                max="30"
                                prop:value=move || chain_max_level.get().to_string()
                                on:input=move |e| {
                                    let n = num(&event_target_value(&e));
                                    chain_max_level.set(if n == 0 { 1 } else { n }.max(1))
                                }
                                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Field>
                        <Field label="Werte-Wachstum pro Stufe (%)">
                            <input
                                type="number"
                                min="0"
                                prop:value=move || chain_growth_percent.get().to_string()
                                on:input=move |e| chain_growth_percent.set(num(&event_target_value(&e)).max(0))
                                class="w-24 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                        </Field>
                    </div>
                    <p class="text-xs text-muted-foreground">
                        "Waffe: Schaden min/max + Zusatz-Angriffskraft wachsen. Rüstung: Magie-/Verteidigung + Zusatz-Verteidigung wachsen (das 3D-Modell bleibt über die ganze Kette gleich). Ohne gewähltes Referenz-Item bleibt die Basis 0 - dann wächst auch nichts. "
                        {move || ref_item.with(|r| r.is_none()).then(|| view! {
                            <span class="text-amber-600">"Aktuell kein Referenz-Item gewählt."</span>
                        })}
                    </p>
                    <p class="text-xs text-muted-foreground">
                        "Aufwertungs-Kosten/-Chance je Stufe folgen der echten Stock-Vorlage dieses Servers (600 Gold/90% für Stufe 1, steigend bis 75.000 Gold/60% ab Stufe 9 - danach bleiben Kosten/Chance von +9 gleich). Keine Materialien nötig - lässt sich danach jederzeit im Aufwertungs-Editor ergänzen."
                    </p>
                </Show>
            </section>

            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Importieren"</h2>
                <Button
                    disabled=Signal::derive(move || !can_import.get() || running.get())
                    on_click=move |_| confirm.set(true)
                >
                    {move || if running.get() { "Importiere…".to_string() } else { format!("{} Item(s) importieren", item_count()) }}
                </Button>

                <Show when=move || weapon_status.with(|w| !w.is_empty()) || armor_status.with(|a| a.is_some())>
                    <div class="space-y-1 text-sm">
                        <StepRow
                            spacer_class="size-4 shrink-0"
                            label="Voraussetzungen prüfen (Tool-Pfade)"
                            status=Signal::derive(move || other_steps.with(|s| s.get("setup").copied()))
                        />
                        {move || selected_weapons().into_iter().map(|v| {
                            let key = v.key.clone();
                            view! {
                                <StepRow
                                    spacer_class="size-4 shrink-0"
                                    label=format!("Waffe: {}", label_for_variant(&v))
                                    status=Signal::derive(move || weapon_status.with(|s| s.get(&key).copied()))
                                />
                            }
                        }).collect_view()}
                        {move || {
                            let armors = selected_armor_races();
                            (!armors.is_empty()).then(|| view! {
                                <StepRow
                                    spacer_class="size-4 shrink-0"
                                    label=format!("Rüstung: {}", race_list(&armors))
                                    status=armor_status
                                />
                            })
                        }}
                        <StepRow
                            spacer_class="size-4 shrink-0"
                            label="Paket-Dateien neu packen (icon/item/pc_*/effect)"
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
                <Show when=move || log.with(|l| !l.is_empty())>
                    <pre node_ref=log_ref class="max-h-48 overflow-y-auto rounded-md bg-muted/40 p-2 text-xs">
                        {move || log.with(|l| l.join("\n"))}
                    </pre>
                </Show>
            </section>
        }
    };

    let confirm_view = move || {
        let weapons = selected_weapons();
        let armors = selected_armor_races();
        let female = armor_female_races();
        let has_desc = !description.with_untracked(|d| d.trim().is_empty()) || !summary.with_untracked(|s| s.trim().is_empty());
        let r = ref_item.get_untracked();
        let effects = include_effects.get_untracked()
            && scan_result.with_untracked(|s| s.as_ref().is_some_and(|s| !s.effect_dirs.is_empty()));
        view! {
            <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                    <p class="text-sm font-medium">
                        {item_count()} " Item(s) für „" {base_locale_name.get_untracked()} "\" jetzt anlegen?"
                    </p>
                    <ul class="list-disc space-y-1 pl-5 text-sm text-muted-foreground">
                        {(!weapons.is_empty()).then(|| view! {
                            <li>
                                {weapons.len()} " Waffen-Item(s): "
                                {weapons.iter().map(label_for_variant).collect::<Vec<_>>().join(", ")}
                            </li>
                        })}
                        {(!armors.is_empty()).then(|| view! {
                            <li>
                                "1 Rüstungs-Item für " {race_list(&armors)}
                                {(!female.is_empty()).then(|| format!(" (3D-Modell für {} verknüpft)", race_list(&female)))}
                            </li>
                        })}
                        {chain_enabled.get_untracked().then(|| view! {
                            <li>
                                "Jedes Item bekommt eine Aufwertungs-Kette bis +" {chain_max_level.get_untracked()} " ("
                                {chain_growth_percent.get_untracked()}
                                "% Werte-Wachstum pro Stufe) - wirkt erst nach einem Server-Neustart, da "
                                <code>"refine_proto"</code> " nur beim Serverstart aus der DB geladen wird"
                            </li>
                        })}
                        <li class=if has_desc { "" } else { "text-amber-600" }>
                            {if has_desc {
                                "Beschreibung wird als itemdesc.txt-Eintrag geschrieben"
                            } else {
                                "Keine Beschreibung eingetragen - Item bleibt im Client ohne Tooltip-Text"
                            }}
                        </li>
                        <li class=if r.is_some() { "" } else { "text-amber-600" }>
                            {match &r {
                                Some(r) => format!(
                                    "Werte werden von \"{}\" (vnum {}) übernommen",
                                    if r.locale_name.is_empty() { &r.name } else { &r.locale_name },
                                    r.vnum
                                ),
                                None => "Kein Referenz-Item gewählt - alle Werte (Schaden/Verteidigung, Preise, …) bleiben 0".to_string(),
                            }}
                        </li>
                        <li>"Icons werden geschrieben, " <code>"icon.epk"</code> " neu gepackt"</li>
                        {(!weapons.is_empty()).then(|| view! {
                            <li>"Waffen-Modelle werden kopiert, " <code>"item.epk"</code> " neu gepackt"</li>
                        })}
                        {(!female.is_empty()).then(|| view! {
                            <li>"Rüstungs-Modelle werden kopiert, betroffene " <code>"pc_*.epk"</code> " neu gepackt"</li>
                        })}
                        {effects.then(|| view! {
                            <li>"Glow-Effekte werden kopiert, " <code>"effect.epk"</code> " neu gepackt"</li>
                        })}
                        <li><code>"item_list.txt"</code> " wird ergänzt (Backups werden vorher angelegt)"</li>
                        <li>
                            <code>"item_proto"</code>
                            " wird aus der DB neu erzeugt und im Client ersetzt (Backup wird vorher angelegt)"
                        </li>
                    </ul>
                    <div class="flex justify-end gap-2">
                        <Button variant=crate::components::ButtonVariant::Outline on_click=move |_| confirm.set(false)>
                            "Abbrechen"
                        </Button>
                        <Button on_click=move |_| run_import()>"Ausführen"</Button>
                    </div>
                </div>
            </div>
        }
    };

    view! {
        <div class="space-y-6">
            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"1. Paket-Ordner wählen"</h2>
                <p class="text-xs text-muted-foreground">
                    "Beliebiger Ordner mit Waffen- und/oder Rüstungs-Dateien - die Ordnerstruktur darf variieren, wird automatisch durchsucht."
                </p>
                <div class="flex items-center gap-2">
                    <Button variant=crate::components::ButtonVariant::Outline on_click=move |_| pick_folder()>
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

            {move || scan_result.get().map(scanned_view)}

            {move || confirm.get().then(confirm_view)}
        </div>
    }
}
