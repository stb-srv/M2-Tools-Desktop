//! Item Editor: neue Items anlegen oder bestehende bearbeiten - inklusive
//! Icon, optional kopiertem Waffen-3D-Modell, itemdesc.txt-Tooltip und dem
//! Neu-Erzeugen/Einspielen der Client-item_proto (Port von
//! `src/features/item-editor`).

pub mod components;
pub mod item_flags;
pub mod types;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow, ItemUsageModal};
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{nav, report_section_dirty, Section};
use crate::tauri::{dialog_open_one, invoke, invoke_unit, listen, DialogFilter, OpenDialogOptions, Unlisten};
use components::confirm_pipeline_dialog::ConfirmPipelineDialog;
use components::icon_browser_modal::IconBrowserModal;
use components::item_flags_section::ItemFlagsSection;
use components::item_presets_section::ItemPresetsSection;
use components::shared::{Field, FlagSelect, NumInput, StepRow};
use item_flags::{subtypes_by_type, ITEM_TYPES};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::HashMap;
use std::future::Future;
use types::{empty_item, js_number, num, pad5, ItemDescEntry, ItemProtoInput, Mode, StepStatus};

/// `Number((await get_setting(...).catch(() => null)) ?? "500000")`.
async fn vnum_range_start() -> i64 {
    let v = invoke::<Option<String>>("get_setting", &json!({ "key": "item_vnum_range_start" }))
        .await
        .ok()
        .flatten();
    js_number(v.as_deref().unwrap_or("500000")) as i64
}

async fn next_free_item_vnum(start: i64) -> Result<i64, String> {
    invoke::<i64>("next_free_item_vnum", &json!({ "rangeStart": start })).await
}

async fn get_item_icon(vnum: i64) -> Option<String> {
    invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await.ok().flatten()
}

type Steps = RwSignal<HashMap<&'static str, StepStatus>>;

async fn run_step(steps: Steps, key: &'static str, fut: impl Future<Output = Result<(), String>>) -> Result<(), String> {
    steps.update(|s| {
        s.insert(key, StepStatus::Running);
    });
    match fut.await {
        Ok(()) => {
            steps.update(|s| {
                s.insert(key, StepStatus::Done);
            });
            Ok(())
        }
        Err(e) => {
            steps.update(|s| {
                s.insert(key, StepStatus::Error);
            });
            Err(e)
        }
    }
}

const INPUT_CLS: &str = "rounded-md border border-border bg-background px-2 py-1 text-sm";

#[component]
pub fn ItemEditor() -> impl IntoView {
    let mode = RwSignal::new(Mode::Create);
    let item = RwSignal::new(empty_item(0));
    let original_item = RwSignal::new(None::<ItemProtoInput>);
    let vnum_taken = RwSignal::new(None::<bool>);
    let checking_vnum = RwSignal::new(false);
    let usage_vnum = RwSignal::new(None::<i64>);

    let icon_source_path = RwSignal::new(String::new());
    let icon_preview = RwSignal::new(None::<String>);

    let ref_model_vnum = RwSignal::new(None::<i64>);
    let copy_model = RwSignal::new(false);

    let icon_browser_open = RwSignal::new(false);

    // locale/<lang>/itemdesc.txt (Client-Tooltip) - keine item_proto-Spalte,
    // daher getrennt von `item` geführt und in einem eigenen Schritt
    // geschrieben (siehe src-tauri/src/itemdesc.rs).
    let description = RwSignal::new(String::new());
    let summary = RwSignal::new(String::new());
    let orig_description = RwSignal::new(String::new());
    let orig_summary = RwSignal::new(String::new());

    let confirm_pipeline = RwSignal::new(false);
    let steps: Steps = RwSignal::new(HashMap::new());
    let log = RwSignal::new(Vec::<String>::new());
    let pipeline_error = RwSignal::new(None::<String>);
    let created_in_db = RwSignal::new(false);
    let updated_in_db = RwSignal::new(false);
    let done = RwSignal::new(false);

    let dirty = RwSignal::new(false);
    Effect::new(move |_| report_section_dirty(Section::ItemEditor, dirty.get()));
    on_cleanup(|| report_section_dirty(Section::ItemEditor, false));

    let log_ref = NodeRef::<html::Pre>::new();

    // Nur einmal beim ersten Mounten im Anlege-Modus: vorgeschlagene vnum.
    spawn_local(async move {
        if mode.get_untracked() != Mode::Create {
            return;
        }
        let Ok(v) = invoke::<Option<String>>("get_setting", &json!({ "key": "item_vnum_range_start" })).await else {
            return;
        };
        let start = js_number(v.as_deref().unwrap_or("500000")) as i64;
        if let Ok(vnum) = next_free_item_vnum(start).await {
            item.update(|i| i.vnum = vnum);
        }
    });

    let set = move |key: &'static str, value: i64| {
        item.update(|i| i.set_num(key, value));
        dirty.set(true);
    };

    let check_vnum = move |vnum: i64| {
        checking_vnum.set(true);
        spawn_local(async move {
            match invoke::<bool>("item_vnum_exists", &json!({ "vnum": vnum })).await {
                Ok(exists) => vnum_taken.set(Some(exists)),
                Err(_) => vnum_taken.set(None),
            }
            checking_vnum.set(false);
        });
    };

    let suggest_next_vnum = move || {
        spawn_local(async move {
            let start = vnum_range_start().await;
            if let Ok(vnum) = next_free_item_vnum(start).await {
                set("vnum", vnum);
                vnum_taken.set(Some(false));
            }
        });
    };

    let start_create_mode = move || {
        mode.set(Mode::Create);
        original_item.set(None);
        icon_source_path.set(String::new());
        icon_preview.set(None);
        ref_model_vnum.set(None);
        copy_model.set(false);
        done.set(false);
        pipeline_error.set(None);
        steps.set(HashMap::new());
        spawn_local(async move {
            let start = vnum_range_start().await;
            let vnum = next_free_item_vnum(start).await.unwrap_or(0);
            item.set(empty_item(vnum));
            vnum_taken.set(if vnum > 0 { Some(false) } else { None });
            description.set(String::new());
            summary.set(String::new());
            orig_description.set(String::new());
            orig_summary.set(String::new());
            dirty.set(false);
        });
    };

    let load_for_edit = move |vnum: i64| {
        spawn_local(async move {
            let Ok(Some(full)) = invoke::<Option<ItemProtoInput>>("get_item_proto", &json!({ "vnum": vnum })).await else {
                return;
            };
            mode.set(Mode::Edit);
            item.set(full.clone());
            original_item.set(Some(full));
            vnum_taken.set(None);
            icon_source_path.set(String::new());
            done.set(false);
            pipeline_error.set(None);
            steps.set(HashMap::new());
            icon_preview.set(get_item_icon(vnum).await);
            let desc = invoke::<Option<ItemDescEntry>>("get_item_desc", &json!({ "vnum": vnum })).await.ok().flatten();
            let (d, s) = desc.map(|d| (d.description, d.summary)).unwrap_or_default();
            description.set(d.clone());
            summary.set(s.clone());
            orig_description.set(d);
            orig_summary.set(s);
            dirty.set(false);
        });
    };

    // Globale Suche (Strg+Umschalt+F) springt mit einer vnum hierher -
    // Konsumieren-beim-Lesen verhindert erneutes Auslösen.
    if let Some(target) = nav().consume_pending_selection(Section::ItemEditor) {
        load_for_edit(js_number(&target) as i64);
    }

    // Ausgabe der Pipeline-Werkzeuge mitschreiben.
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

    let pick_icon = move || {
        spawn_local(async move {
            let selected = dialog_open_one(&OpenDialogOptions {
                title: Some("Icon auswählen".into()),
                filters: vec![DialogFilter::new("Bild", &["tga", "png", "jpg", "jpeg", "bmp"])],
                multiple: false,
                ..Default::default()
            })
            .await;
            if let Ok(Some(path)) = selected {
                icon_source_path.set(path);
                icon_preview.set(None);
                dirty.set(true);
            }
        });
    };

    let duplicate_as_new = move || {
        spawn_local(async move {
            let start = vnum_range_start().await;
            let vnum = next_free_item_vnum(start).await.unwrap_or(0);
            item.update(|i| i.vnum = vnum);
            mode.set(Mode::Create);
            original_item.set(None);
            icon_source_path.set(String::new());
            icon_preview.set(None);
            ref_model_vnum.set(None);
            copy_model.set(false);
            done.set(false);
            pipeline_error.set(None);
            steps.set(HashMap::new());
            vnum_taken.set(if vnum > 0 { Some(false) } else { None });
            // Beschreibung bleibt als Entwurf, aber die "Original"-Basis ist
            // leer - die neue vnum hat noch keine eigene itemdesc.txt-Zeile.
            orig_description.set(String::new());
            orig_summary.set(String::new());
            // Vorbefüllter, nie gespeicherter Entwurf = ungespeicherte Arbeit.
            dirty.set(true);
        });
    };

    let load_reference = move |vnum: i64| {
        spawn_local(async move {
            let Ok(full) = invoke::<Option<ItemProtoInput>>("get_item_proto", &json!({ "vnum": vnum })).await else {
                return;
            };
            if let Some(f) = &full {
                let keep = item.get_untracked().vnum;
                item.set(ItemProtoInput { vnum: keep, ..f.clone() });
            }
            // Waffen-3D-Modelle sind clientseitig reine vnum-Dateien - nur bei
            // Typ 1 (Waffe) lässt sich das Modell wiederverwenden.
            ref_model_vnum.set(full.filter(|f| f.r#type == 1).map(|_| vnum));
            copy_model.set(false);
            dirty.set(true);
        });
    };

    // Eine Vorlage füllt nur Feldwerte vor - die aktuelle vnum bleibt.
    let load_preset = move |preset: ItemProtoInput| {
        let keep = item.get_untracked().vnum;
        item.set(ItemProtoInput { vnum: keep, ..preset });
        ref_model_vnum.set(None);
        copy_model.set(false);
        dirty.set(true);
    };

    let toggle_flag = move |key: &'static str, bit: i64| {
        let cur = item.with_untracked(|i| i.num(key));
        set(key, if cur & bit != 0 { cur & !bit } else { cur | bit });
    };

    let has_new_icon = Memo::new(move |_| !icon_source_path.with(|p| p.is_empty()));
    let has_model_copy =
        Memo::new(move |_| copy_model.get() && ref_model_vnum.get().is_some() && item.with(|i| i.r#type == 1));
    let has_desc_change = Memo::new(move |_| description.get() != orig_description.get() || summary.get() != orig_summary.get());
    let can_create = Memo::new(move |_| {
        item.with(|i| i.vnum > 0 && !i.name.trim().is_empty() && !i.locale_name.trim().is_empty())
            && (mode.get() == Mode::Edit || (has_new_icon.get() && vnum_taken.get() != Some(true)))
    });

    use_save_shortcut(
        Section::ItemEditor,
        move || dirty.get_untracked() && can_create.get_untracked() && steps.with_untracked(|s| s.is_empty()),
        move || confirm_pipeline.set(true),
    );

    let run_pipeline = move || {
        confirm_pipeline.set(false);
        pipeline_error.set(None);
        log.set(vec![]);
        steps.set(HashMap::new());
        done.set(false);

        let it = item.get_untracked();
        let md = mode.get_untracked();
        let new_icon = has_new_icon.get_untracked();
        let model_copy = has_model_copy.get_untracked();
        let desc_change = has_desc_change.get_untracked();
        let desc = description.get_untracked();
        let summ = summary.get_untracked();
        let icon_path = icon_source_path.get_untracked();
        let ref_model = ref_model_vnum.get_untracked();
        let original = original_item.get_untracked();

        spawn_local(async move {
            let mut db_created = false;
            let mut db_updated = false;
            let vnum = it.vnum;
            let result: Result<(), String> = async {
                // Zuerst, vor allem Zerstörerischen (auch dem DB-Schritt): ein
                // falscher Werkzeug-Pfad muss hier laut scheitern.
                run_step(steps, "setup", async {
                    invoke_unit("validate_item_editor_setup", &json!({ "requireIconTool": new_icon || model_copy })).await
                })
                .await?;
                run_step(steps, "db", async {
                    if md == Mode::Create {
                        invoke_unit("create_item_proto", &json!({ "item": it })).await?;
                        db_created = true;
                        created_in_db.set(true);
                    } else {
                        invoke_unit("update_item_proto", &json!({ "item": it })).await?;
                        db_updated = true;
                        updated_in_db.set(true);
                    }
                    Ok(())
                })
                .await?;
                if desc_change {
                    run_step(steps, "desc", async {
                        invoke_unit("write_item_desc", &json!({ "vnum": vnum, "description": desc, "summary": summ })).await?;
                        orig_description.set(desc.clone());
                        orig_summary.set(summ.clone());
                        Ok(())
                    })
                    .await?;
                }
                if new_icon {
                    run_step(steps, "icon", async {
                        invoke_unit("write_item_icon", &json!({ "vnum": vnum, "sourcePath": icon_path })).await?;
                        icon_preview.set(get_item_icon(vnum).await);
                        Ok(())
                    })
                    .await?;
                    run_step(steps, "pack", async { invoke_unit("pack_item_icons", &json!({})).await }).await?;
                }
                if model_copy {
                    run_step(steps, "model", async {
                        invoke_unit("write_item_model", &json!({ "vnum": vnum, "sourceVnum": ref_model })).await
                    })
                    .await?;
                    run_step(steps, "packModel", async { invoke_unit("pack_item_models", &json!({})).await }).await?;
                }
                if new_icon {
                    // Der Client braucht eine explizite Zeile in jeder
                    // locale/<lang>/item_list.txt, sonst bleiben Icon/Modell
                    // unsichtbar, obwohl die Dateien stimmen.
                    run_step(steps, "itemList", async {
                        invoke_unit(
                            "write_item_list_entry",
                            &json!({
                                "vnum": vnum,
                                "itemType": it.r#type,
                                "iconRelPath": format!("icon/item/{}.tga", pad5(vnum)),
                                "modelRelPath": if model_copy {
                                    Some(format!("d:/ymir work/item/weapon/{}.gr2", pad5(vnum)))
                                } else {
                                    None
                                },
                            }),
                        )
                        .await
                    })
                    .await?;
                }
                run_step(steps, "proto", async {
                    let generated = invoke::<String>("regenerate_item_proto", &json!({})).await?;
                    invoke_unit("deploy_item_proto", &json!({ "generatedProtoPath": generated })).await
                })
                .await?;
                Ok(())
            }
            .await;

            match result {
                Ok(()) => {
                    done.set(true);
                    dirty.set(false);
                    if md == Mode::Edit {
                        original_item.set(Some(it.clone()));
                    }
                    let name = if it.locale_name.is_empty() { &it.name } else { &it.locale_name };
                    log_activity(
                        Section::ItemEditor.into(),
                        if md == Mode::Create { "create" } else { "update" },
                        &format!(
                            "Item {} ('{}') {}",
                            vnum,
                            name,
                            if md == Mode::Create { "angelegt" } else { "aktualisiert" }
                        ),
                        Some("item"),
                        Some(&vnum.to_string()),
                    );
                }
                Err(e) => {
                    let mut message = e;
                    // Ein Fehler nach dem DB-Schritt würde sonst ein halbfertiges
                    // Item hinterlassen - automatisch zurücknehmen.
                    if db_created {
                        match invoke_unit("delete_item_proto", &json!({ "vnum": vnum })).await {
                            Ok(()) => {
                                created_in_db.set(false);
                                message.push_str("\n\nDatenbankeintrag wurde automatisch zurückgenommen.");
                            }
                            Err(re) => message.push_str(&format!(
                                "\n\nDatenbankeintrag konnte NICHT automatisch zurückgenommen werden: {re}. Bitte manuell prüfen (vnum {vnum})."
                            )),
                        }
                    } else if db_updated {
                        if let Some(orig) = original {
                            match invoke_unit("update_item_proto", &json!({ "item": orig })).await {
                                Ok(()) => {
                                    updated_in_db.set(false);
                                    message.push_str(
                                        "\n\nÄnderungen wurden automatisch zurückgenommen (alter Stand wiederhergestellt).",
                                    );
                                }
                                Err(re) => message.push_str(&format!(
                                    "\n\nÄnderungen konnten NICHT automatisch zurückgenommen werden: {re}. Bitte vnum {vnum} manuell im DB Explorer prüfen."
                                )),
                            }
                        }
                    }
                    pipeline_error.set(Some(message));
                }
            }
        });
    };

    let rollback_db = move || {
        let md = mode.get_untracked();
        let vnum = item.get_untracked().vnum;
        let original = original_item.get_untracked();
        spawn_local(async move {
            let res: Result<(), String> = async {
                if md == Mode::Create {
                    invoke_unit("delete_item_proto", &json!({ "vnum": vnum })).await?;
                    created_in_db.set(false);
                } else if let Some(orig) = original {
                    invoke_unit("update_item_proto", &json!({ "item": orig })).await?;
                    updated_in_db.set(false);
                }
                steps.update(|s| {
                    s.insert("db", StepStatus::Pending);
                });
                Ok(())
            }
            .await;
            if let Err(e) = res {
                pipeline_error.set(Some(e));
            }
        });
    };

    let is_create = move || mode.get() == Mode::Create;
    let item_type = Memo::new(move |_| item.with(|i| i.r#type));
    let step = move |key: &'static str| Signal::derive(move || steps.with(|s| s.get(key).copied()));
    let has_steps = move || steps.with(|s| !s.is_empty());

    // ---------------------------------------------------------------- Modus
    let mode_section = view! {
        <section class="space-y-2 rounded-lg border border-border p-4">
            <div class="flex overflow-hidden rounded-md border border-border text-sm w-fit">
                <button
                    on:click=move |_| start_create_mode()
                    class=move || if is_create() { "px-3 py-1 bg-primary text-primary-foreground" } else { "px-3 py-1 " }
                >
                    "Neues Item"
                </button>
                <button
                    on:click=move |_| mode.set(Mode::Edit)
                    class=move || if !is_create() { "px-3 py-1 bg-primary text-primary-foreground" } else { "px-3 py-1 " }
                >
                    "Bestehendes Item bearbeiten"
                </button>
            </div>

            <Show when=move || !is_create() && original_item.with(|o| o.is_none())>
                <EntityBrowser kind=EntityKind::Item pick_label="Laden" on_pick=Callback::new(move |r: EntityRow| load_for_edit(r.vnum))/>
            </Show>

            <Show when=move || !is_create() && original_item.with(|o| o.is_some())>
                <div class="flex items-center justify-between gap-2">
                    <p class="text-sm text-muted-foreground">
                        "Bearbeite "
                        <strong>
                            {move || item.with(|i| if i.locale_name.is_empty() { i.name.clone() } else { i.locale_name.clone() })}
                        </strong>
                        " (vnum " {move || item.with(|i| i.vnum)} "). "
                        <button class="underline" on:click=move |_| original_item.set(None)>"Anderes Item wählen"</button>
                    </p>
                    <div class="flex gap-2">
                        <Button
                            variant=ButtonVariant::Outline
                            size=ButtonSize::Sm
                            on_click=move |_| usage_vnum.set(Some(item.with_untracked(|i| i.vnum)))
                        >
                            <Icon name="Link2" class="size-3.5"/>
                            "Verwendung prüfen"
                        </Button>
                        <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| duplicate_as_new()>
                            <Icon name="Copy" class="size-3.5"/>
                            "Als neues Item duplizieren"
                        </Button>
                    </div>
                </div>
            </Show>
            {move || usage_vnum.get().map(|v| view! { <ItemUsageModal vnum=v on_close=Callback::new(move |_| usage_vnum.set(None))/> })}
        </section>
    }
    .into_any();

    // ---------------------------------------------------------------- Basisdaten
    let base_section = move || {
        view! {
            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Basisdaten"</h2>

                <div class="flex flex-wrap gap-3">
                    <Field label="VNUM">
                        <div class="flex items-center gap-1">
                            <input
                                type="number"
                                prop:value=move || item.with(|i| if i.vnum == 0 { String::new() } else { i.vnum.to_string() })
                                disabled=move || !is_create()
                                on:input=move |e| {
                                    set("vnum", num(&event_target_value(&e)));
                                    vnum_taken.set(None);
                                }
                                on:blur=move |_| {
                                    let v = item.with_untracked(|i| i.vnum);
                                    if v > 0 {
                                        check_vnum(v);
                                    }
                                }
                                class="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm disabled:opacity-60"
                            />
                            <Show when=is_create>
                                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| suggest_next_vnum()>
                                    "Nächste freie"
                                </Button>
                            </Show>
                        </div>
                        <Show when=move || is_create() && checking_vnum.get()>
                            <p class="text-xs text-muted-foreground">"Prüfe…"</p>
                        </Show>
                        <Show when=move || is_create() && vnum_taken.get() == Some(true)>
                            <p class="flex items-center gap-1 text-xs text-destructive">
                                <Icon name="AlertTriangle" class="size-3.5"/>
                                " VNUM bereits vergeben"
                            </p>
                        </Show>
                        <Show when=move || is_create() && vnum_taken.get() == Some(false)>
                            <p class="flex items-center gap-1 text-xs text-green-600">
                                <Icon name="CheckCircle2" class="size-3.5"/>
                                " frei"
                            </p>
                        </Show>
                    </Field>

                    <Field label="Typ">
                        <FlagSelect
                            options=ITEM_TYPES
                            value=Signal::derive(move || item_type.get())
                            on_change=Callback::new(move |v| set("type", v))
                            class=INPUT_CLS
                        />
                    </Field>

                    <Field label="Subtyp">
                        {move || match subtypes_by_type(item_type.get()) {
                            Some(opts) => view! {
                                <FlagSelect
                                    options=opts
                                    value=Signal::derive(move || item.with(|i| i.subtype))
                                    on_change=Callback::new(move |v| set("subtype", v))
                                    class="w-40 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                />
                            }
                            .into_any(),
                            None => view! {
                                <NumInput
                                    value=Signal::derive(move || item.with(|i| i.subtype))
                                    on_change=Callback::new(move |v| set("subtype", v))
                                    class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                />
                            }
                            .into_any(),
                        }}
                    </Field>

                    <Field label="Größe (1-3)">
                        <NumInput
                            min="1"
                            max="3"
                            value=Signal::derive(move || item.with(|i| i.size))
                            on_change=Callback::new(move |v| set("size", v))
                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>

                    <Field label="Gewicht">
                        <NumInput
                            value=Signal::derive(move || item.with(|i| i.weight))
                            on_change=Callback::new(move |v| set("weight", v))
                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                </div>

                <div class="flex flex-wrap gap-3">
                    <Field label="Name (intern, ASCII)">
                        <input
                            prop:value=move || item.with(|i| i.name.clone())
                            on:input=move |e| {
                                let v = event_target_value(&e);
                                item.update(|i| i.name = v);
                                dirty.set(true);
                            }
                            class="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <Field label="Anzeigename">
                        <input
                            prop:value=move || item.with(|i| i.locale_name.clone())
                            on:input=move |e| {
                                let v = event_target_value(&e);
                                item.update(|i| i.locale_name = v);
                                dirty.set(true);
                            }
                            class="w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                </div>

                <div class="flex flex-wrap gap-3">
                    <Field label="Beschreibung (Tooltip-Text)">
                        <textarea
                            prop:value=move || description.get()
                            on:input=move |e| {
                                description.set(event_target_value(&e));
                                dirty.set(true);
                            }
                            class="h-16 w-72 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        ></textarea>
                    </Field>
                    <Field label="Kurzbeschreibung">
                        <textarea
                            prop:value=move || summary.get()
                            on:input=move |e| {
                                summary.set(event_target_value(&e));
                                dirty.set(true);
                            }
                            class="h-16 w-56 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        ></textarea>
                    </Field>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Rein clientseitiger Tooltip-Text (" <code>"locale/<lang>/itemdesc.txt"</code>
                    ") - keine Datenbank-Spalte, keine Server-Beteiligung. Wird beim Speichern lokal in die Client-Datei geschrieben (mit Backup), nicht über SFTP - ein Client-Neustart/-Relog reicht, um die neue Beschreibung zu sehen, kein Server-Neustart nötig."
                </p>

                <div class="flex flex-wrap gap-3">
                    <Field label="Kaufpreis (NPC → Spieler)">
                        <NumInput
                            value=Signal::derive(move || item.with(|i| i.gold))
                            on_change=Callback::new(move |v| set("gold", v))
                            class="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                    <Field label="Verkaufspreis (Spieler → NPC)">
                        <NumInput
                            value=Signal::derive(move || item.with(|i| i.shop_buy_price))
                            on_change=Callback::new(move |v| set("shop_buy_price", v))
                            class="w-28 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                    </Field>
                </div>
            </section>
        }
    };

    // ---------------------------------------------------------------- Icon
    let icon_section = move || {
        view! {
            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Icon"</h2>
                <div class="flex items-center gap-3">
                    <div class="flex size-12 items-center justify-center rounded-md border border-border bg-muted/40">
                        {move || match icon_preview.get() {
                            Some(src) => view! { <img src=src alt="" class="size-10 object-contain [image-rendering:pixelated]"/> }.into_any(),
                            None => view! { <Icon name="Image" class="size-5 text-muted-foreground"/> }.into_any(),
                        }}
                    </div>
                    <Button variant=ButtonVariant::Outline on_click=move |_| pick_icon()>"Bilddatei wählen…"</Button>
                    <Button variant=ButtonVariant::Outline on_click=move |_| icon_browser_open.set(true)>
                        <Icon name="Images" class="size-4"/>
                        "Aus Icon-Browser wählen…"
                    </Button>
                    <span class="truncate text-sm text-muted-foreground">
                        {move || {
                            let p = icon_source_path.get();
                            if !p.is_empty() {
                                p
                            } else if is_create() {
                                "Keine Datei gewählt".to_string()
                            } else {
                                "Unverändert".to_string()
                            }
                        }}
                    </span>
                </div>
                <Show when=move || !is_create() && !has_new_icon.get()>
                    <p class="text-xs text-green-600">
                        {move || if icon_preview.with(|p| p.is_some()) {
                            "Vorhandenes Icon wird beibehalten - nichts wird beim Speichern neu geschrieben oder gepackt."
                        } else {
                            "Kein Icon gefunden. Nur auswählen, falls du wirklich ein neues setzen willst."
                        }}
                    </p>
                </Show>
                <Show when=move || is_create() || has_new_icon.get()>
                    <p class="text-xs text-muted-foreground">
                        "Wird als " <code>{move || format!("{}.tga", pad5(item.with(|i| i.vnum)))}</code> " in "
                        <code>"pack/icon/icon/item"</code> " gespeichert und " <code>"icon.epk"</code> " neu gepackt."
                    </p>
                </Show>
            </section>
        }
    };

    // ---------------------------------------------------------------- Referenz-Item
    let reference_section = move || {
        view! {
            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Referenz-Item übernehmen (optional)"</h2>
                <p class="text-xs text-muted-foreground">
                    "Übernimmt alle Werte (Typ, Flags, Werte) eines bestehenden Items als Ausgangsbasis — die VNUM bleibt unverändert."
                </p>
                <EntityBrowser kind=EntityKind::Item pick_label="Übernehmen" on_pick=Callback::new(move |r: EntityRow| load_reference(r.vnum))/>
                {move || {
                    let rv = ref_model_vnum.get()?;
                    (item_type.get() == 1).then(|| view! {
                        <label class="flex items-center gap-2 text-sm">
                            <input
                                type="checkbox"
                                prop:checked=move || copy_model.get()
                                on:change=move |e| {
                                    copy_model.set(event_target_checked(&e));
                                    dirty.set(true);
                                }
                            />
                            "3D-Modell von vnum " {rv} " übernehmen (Waffe sieht dann identisch aus)"
                        </label>
                    })
                }}
            </section>
        }
    };

    // ---------------------------------------------------------------- Ausführen
    let run_section = move || {
        view! {
            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">{move || if is_create() { "Anlegen" } else { "Speichern" }}</h2>
                <Button
                    disabled=Signal::derive(move || !can_create.get() || has_steps())
                    on_click=move |_| confirm_pipeline.set(true)
                >
                    {move || if is_create() { "Item anlegen" } else { "Änderungen speichern" }}
                </Button>

                <Show when=has_steps>
                    <div class="space-y-1 text-sm">
                        <StepRow label="Voraussetzungen prüfen (Tool-Pfade)" status=step("setup")/>
                        <StepRow label="Datenbankeintrag" status=step("db")/>
                        <Show when=move || has_desc_change.get()>
                            <StepRow label="Beschreibung schreiben (itemdesc.txt)" status=step("desc")/>
                        </Show>
                        <Show when=move || has_new_icon.get()>
                            <StepRow label="Icon schreiben" status=step("icon")/>
                            <StepRow label="icon.epk neu packen" status=step("pack")/>
                        </Show>
                        <Show when=move || has_model_copy.get()>
                            <StepRow label="3D-Modell kopieren" status=step("model")/>
                            <StepRow label="item.epk neu packen" status=step("packModel")/>
                        </Show>
                        <Show when=move || has_new_icon.get()>
                            <StepRow label="item_list.txt aktualisieren (Icon-/Modell-Zuordnung)" status=step("itemList")/>
                        </Show>
                        <StepRow label="item_proto aus DB erzeugen & in Client einspielen" status=step("proto")/>
                    </div>
                </Show>

                {move || pipeline_error.get().map(|err| view! {
                    <div class="space-y-2">
                        <p class="whitespace-pre-wrap text-sm text-destructive">{err}</p>
                        <Show when=move || {
                            ((is_create() && created_in_db.get()) || (!is_create() && updated_in_db.get())) && !done.get()
                        }>
                            <Button variant=ButtonVariant::Destructive size=ButtonSize::Sm on_click=move |_| rollback_db()>
                                {move || if is_create() { "Datenbankeintrag zurücknehmen" } else { "Änderungen zurücknehmen" }}
                            </Button>
                        </Show>
                    </div>
                })}

                <Show when=move || done.get()>
                    <p class="flex items-center gap-1 text-sm text-green-600">
                        <Icon name="CheckCircle2" class="size-4"/>
                        " "
                        {move || {
                            let v = item.with(|i| i.vnum);
                            if is_create() {
                                format!("Item {v} wurde vollständig angelegt.")
                            } else {
                                format!("Item {v} wurde gespeichert.")
                            }
                        }}
                    </p>
                </Show>

                <Show when=move || log.with(|l| !l.is_empty())>
                    <pre node_ref=log_ref class="max-h-48 overflow-y-auto rounded-md bg-muted/40 p-2 text-xs">
                        {move || log.with(|l| l.join("\n"))}
                    </pre>
                </Show>
            </section>
        }
    };

    let body = move || {
        view! {
            {base_section()}
            {icon_section()}
            <Show when=is_create>
                <ItemPresetsSection
                    current_item=Signal::derive(move || item.get())
                    on_load=Callback::new(move |p: ItemProtoInput| load_preset(p))
                />
            </Show>
            <Show when=is_create>{reference_section()}</Show>
            <ItemFlagsSection
                item=Signal::derive(move || item.get())
                set=Callback::new(move |(k, v): (&'static str, i64)| set(k, v))
                toggle_flag=Callback::new(move |(k, bit): (&'static str, i64)| toggle_flag(k, bit))
            />
            {run_section()}
            {move || confirm_pipeline.get().then(|| view! {
                <ConfirmPipelineDialog
                    mode=mode.get_untracked()
                    item_vnum=item.with_untracked(|i| i.vnum)
                    has_new_icon=has_new_icon.get_untracked()
                    has_model_copy=has_model_copy.get_untracked()
                    ref_model_vnum=ref_model_vnum.get_untracked()
                    on_cancel=Callback::new(move |_| confirm_pipeline.set(false))
                    on_confirm=Callback::new(move |_| run_pipeline())
                />
            })}
        }
    };

    view! {
        <div class="space-y-6 pb-10">
            <PageHeader module=Section::ItemEditor title="Item Editor"/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Legt neue Items an oder bearbeitet bestehende. Waffen können beim Anlegen das 3D-Modell eines bestehenden Waffen-Items übernehmen (Referenz-Item übernehmen unten). Komplett neue, selbst erstellte 3D-Modelle sowie Rüstungen mit übernommenem Modell werden noch nicht unterstützt."
            </p>

            {mode_section}

            <Show when=move || is_create() || original_item.with(|o| o.is_some())>{body()}</Show>

            {move || icon_browser_open.get().then(|| view! {
                <IconBrowserModal
                    on_pick=Callback::new(move |path: String| {
                        icon_source_path.set(path);
                        icon_preview.set(None);
                        dirty.set(true);
                    })
                    on_close=Callback::new(move |_| icon_browser_open.set(false))
                />
            })}
        </div>
    }
}
