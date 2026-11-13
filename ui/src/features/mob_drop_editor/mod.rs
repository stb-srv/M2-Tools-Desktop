//! Mob Drop Editor (mob_drop_item.txt) - portiert aus
//! `src/features/mob-drop-editor`. Lebt als Tab im Drop-Generator.

pub mod components;
pub mod duplicates;
pub mod server_notes;
pub mod types;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, Skeleton};
use crate::features::shared::drop_chance::real_drop_chance_percent;
use crate::features::shared::{ChangePreview, EntityRow};
use crate::util::csv::export_rows_as_csv;
use crate::util::{log_activity, open_manual};
use crate::store::navigation::{nav, report_section_dirty, EmbeddedModule, Section};
use crate::tauri::{dialog_open_one, invoke, DialogFilter, OpenDialogOptions};
use components::shared::{clamp_percent, fmt_num};
use components::{
    BulkEditPanel, BulkEditParams, CreateMobModal, DuplicatesModal, GroupSidebar, ItemPickerModal, MobDetailPanel,
    ReverseLookupModal, ServerInfoPopover, SimulatorModal,
};
use duplicates::{find_duplicate_items_in_mobs, find_duplicate_mobs};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;
use std::collections::{HashMap, HashSet};
use types::{BulkMode, BulkScope, MobDropGroup, MobDropItem, MobDropLoadResult, NumberingIssue};

/// Item-Icons nach vnum: `None` = kein Icon/Fehler, fehlt der Schlüssel =
/// noch nicht geladen.
pub type IconMap = HashMap<i64, Option<String>>;

/// Liest reaktiv aus der Gruppe `index` (falls vorhanden).
pub fn with_group<T>(groups: RwSignal<Option<Vec<MobDropGroup>>>, index: usize, f: impl FnOnce(&MobDropGroup) -> T) -> Option<T> {
    groups.with(|g| g.as_ref().and_then(|g| g.get(index)).map(f))
}

/// Lädt fehlende Item-Icons nach (wie `ensureIcons` im TS-Code).
pub fn ensure_icons(icons: RwSignal<IconMap>, vnums: impl IntoIterator<Item = i64>) {
    let unique: HashSet<i64> = vnums.into_iter().collect();
    let missing: Vec<i64> = icons.with_untracked(|m| unique.into_iter().filter(|v| !m.contains_key(v)).collect());
    for vnum in missing {
        spawn_local(async move {
            let res = invoke::<Option<String>>("get_item_icon", &json!({ "vnum": vnum })).await;
            icons.update(|m| {
                m.insert(vnum, res.ok().flatten());
            });
        });
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Server,
    Local,
}

#[derive(Clone, PartialEq)]
struct RawRecovery {
    content: String,
    error: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Skeleton,
    Error,
    Main,
}

#[component]
pub fn MobDropEditor() -> impl IntoView {
    let source = RwSignal::new(Source::Server);
    let groups = RwSignal::new(None::<Vec<MobDropGroup>>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);

    let local_path = RwSignal::new(None::<String>);
    let raw_recovery = RwSignal::new(None::<RawRecovery>);
    let raw_checking = RwSignal::new(false);

    let group_search = RwSignal::new(String::new());
    let selected_index = RwSignal::new(None::<usize>);

    let icons = RwSignal::new(IconMap::new());

    let item_picker = RwSignal::new(false);
    let creating = RwSignal::new(false);
    let delete_confirm = RwSignal::new(None::<usize>);

    let saving = RwSignal::new(false);
    let save_confirm = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(None::<String>);

    let exporting = RwSignal::new(false);
    let export_error = RwSignal::new(None::<String>);

    let reverse_lookup_open = RwSignal::new(false);
    let duplicates_open = RwSignal::new(false);
    let simulator_open = RwSignal::new(false);

    let dirty = RwSignal::new(false);
    Effect::new(move |_| report_section_dirty(Section::DropGenerator, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::DropGenerator, false));

    // Nur zur Diagnose, einmal beim Laden aus dem Rohinhalt ermittelt (siehe
    // mobdrop::check_numbering) - nicht aus `groups` neu berechnet wie die
    // Duplikat-Prüfungen: Speichern nummeriert ohnehin immer korrekt neu.
    let numbering_issues = RwSignal::new(Vec::<NumberingIssue>::new());

    let duplicate_items = Memo::new(move |_| groups.with(|g| find_duplicate_items_in_mobs(g.as_deref().unwrap_or_default())));
    let duplicate_mobs = Memo::new(move |_| groups.with(|g| find_duplicate_mobs(g.as_deref().unwrap_or_default())));

    let load_from_server = move || {
        loading.set(true);
        load_error.set(None);
        selected_index.set(None);
        save_ok.set(None);
        spawn_local(async move {
            match invoke::<MobDropLoadResult>("read_mob_drop_file", &json!({})).await {
                Ok(result) => {
                    groups.set(Some(result.groups));
                    numbering_issues.set(result.numbering_issues);
                    dirty.set(false);
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    load_from_server();

    let load_local_content = move |path: String| {
        loading.set(true);
        load_error.set(None);
        selected_index.set(None);
        save_ok.set(None);
        save_error.set(None);
        groups.set(None);
        raw_recovery.set(None);
        numbering_issues.set(vec![]);
        spawn_local(async move {
            match invoke::<String>("read_local_text_file", &json!({ "path": path })).await {
                Ok(content) => match invoke::<MobDropLoadResult>("parse_mob_drop_text", &json!({ "content": content })).await {
                    Ok(parsed) => {
                        groups.set(Some(parsed.groups));
                        numbering_issues.set(parsed.numbering_issues);
                        dirty.set(false);
                    }
                    Err(parse_error) => raw_recovery.set(Some(RawRecovery { content, error: parse_error })),
                },
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let pick_local_file = move || {
        spawn_local(async move {
            let selected = dialog_open_one(&OpenDialogOptions {
                multiple: false,
                title: Some("mob_drop_item.txt auswählen".into()),
                filters: vec![DialogFilter::new("Textdatei", &["txt"])],
                ..Default::default()
            })
            .await;
            let Ok(Some(path)) = selected else { return };
            local_path.set(Some(path.clone()));
            load_local_content(path);
        });
    };

    let switch_source = move |next: Source| {
        if next == source.get_untracked() {
            return;
        }
        source.set(next);
        groups.set(None);
        load_error.set(None);
        selected_index.set(None);
        local_path.set(None);
        raw_recovery.set(None);
        save_ok.set(None);
        save_error.set(None);
        numbering_issues.set(vec![]);
        if next == Source::Server {
            load_from_server();
        }
    };

    let retry_parse_raw = move || {
        let Some(rr) = raw_recovery.get_untracked() else { return };
        raw_checking.set(true);
        spawn_local(async move {
            match invoke::<MobDropLoadResult>("parse_mob_drop_text", &json!({ "content": rr.content })).await {
                Ok(parsed) => {
                    groups.set(Some(parsed.groups));
                    numbering_issues.set(parsed.numbering_issues);
                    dirty.set(false);
                    raw_recovery.set(None);
                }
                Err(e) => raw_recovery.set(Some(RawRecovery { content: rr.content, error: e })),
            }
            raw_checking.set(false);
        });
    };

    Effect::new(move |_| {
        let Some(si) = selected_index.get() else { return };
        let vnums: Option<Vec<i64>> = with_group(groups, si, |g| g.items.iter().map(|i| i.item_vnum as i64).collect());
        if let Some(v) = vnums {
            ensure_icons(icons, v);
        }
    });

    let remove_item = move |(gi, ii): (usize, usize)| {
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(gi)) {
                if ii < g.items.len() {
                    g.items.remove(ii);
                }
            }
        });
        dirty.set(true);
    };

    let add_item = move |item: EntityRow| {
        let Some(si) = selected_index.get_untracked() else { return };
        groups.update(|g| {
            if let Some(g) = g.as_mut().and_then(|g| g.get_mut(si)) {
                g.items.push(MobDropItem { item_vnum: item.vnum as f64, count: 1.0, percent: 0.0 });
            }
        });
        dirty.set(true);
        item_picker.set(false);
    };

    let handle_mob_created = move |group: MobDropGroup| {
        let len = groups.with_untracked(|g| g.as_ref().map(|g| g.len()));
        groups.update(|g| g.get_or_insert_with(Vec::new).push(group));
        selected_index.set(Some(len.unwrap_or(0)));
        dirty.set(true);
        creating.set(false);
    };

    let navigate_to_duplicate_group = move |gi: usize| {
        selected_index.set(Some(gi));
        duplicates_open.set(false);
    };

    let delete_duplicate_group = move |gi: usize| {
        duplicates_open.set(false);
        delete_confirm.set(Some(gi));
    };

    let confirm_delete_group = move || {
        let Some(dc) = delete_confirm.get_untracked() else { return };
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
        dirty.set(true);
        delete_confirm.set(None);
    };

    let apply_bulk_edit = move |params: BulkEditParams| {
        dirty.set(true);
        let si = selected_index.get_untracked();
        groups.update(|g| {
            let Some(g) = g.as_mut() else { return };
            for (gi, group) in g.iter_mut().enumerate() {
                if params.scope == BulkScope::Current && Some(gi) != si {
                    continue;
                }
                for item in group.items.iter_mut() {
                    if params.mode == BulkMode::SpecificItem {
                        if !params.specific_vnum.is_finite() || item.item_vnum != params.specific_vnum {
                            continue;
                        }
                        item.percent = clamp_percent(params.specific_percent);
                        continue;
                    }
                    let percent = match params.mode {
                        BulkMode::Delta => item.percent + params.delta,
                        BulkMode::Fixed => params.fixed,
                        BulkMode::Random => params.random_min + js_sys::Math::random() * (params.random_max - params.random_min),
                        BulkMode::SpecificItem => item.percent,
                    };
                    item.percent = clamp_percent(percent);
                }
            }
        });
    };

    let save_file = move || {
        let Some(gs) = groups.get_untracked() else { return };
        saving.set(true);
        save_error.set(None);
        save_ok.set(None);
        save_confirm.set(false);
        let src = source.get_untracked();
        let path = local_path.get_untracked();
        spawn_local(async move {
            let res = if src == Source::Server {
                invoke::<Option<String>>("write_mob_drop_file", &json!({ "groups": gs })).await
            } else {
                invoke::<Option<String>>("write_local_mob_drop_file", &json!({ "path": path, "groups": gs })).await
            };
            match res {
                Ok(backup) => {
                    save_ok.set(Some(match backup {
                        Some(b) if !b.is_empty() => format!("Gespeichert. Backup: {b}"),
                        _ => "Gespeichert.".to_string(),
                    }));
                    dirty.set(false);
                    let summary = if src == Source::Server {
                        format!("mob_drop_item.txt auf dem Server gespeichert ({} Gruppe(n))", gs.len())
                    } else {
                        format!("Lokale mob_drop_item.txt-Kopie gespeichert ({} Gruppe(n))", gs.len())
                    };
                    log_activity(EmbeddedModule::MobDropEditor.into(), "save", &summary, Some("mob_drop_item.txt"), None);
                }
                Err(e) => save_error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let filtered_groups = Memo::new(move |_| {
        let raw = group_search.get();
        let q = raw.to_lowercase();
        groups.with(|g| {
            g.as_deref()
                .unwrap_or_default()
                .iter()
                .enumerate()
                .filter(|(_, g)| g.name.to_lowercase().contains(&q) || fmt_num(g.mob_vnum).contains(&raw))
                .map(|(i, _)| i)
                .collect::<Vec<_>>()
        })
    });

    // Exportiert genau die Mobs, die die aktuelle Suche (Name/VNUM) gerade
    // zeigt - z.B. "Metin" eintippen und alle Metin-Mobs samt Drops als CSV.
    let export_filtered = move || {
        export_error.set(None);
        exporting.set(true);
        let rows: Vec<Vec<String>> = groups.with_untracked(|g| {
            let g = g.as_deref().unwrap_or_default();
            filtered_groups
                .get_untracked()
                .iter()
                .filter_map(|i| g.get(*i))
                .flat_map(|g| {
                    if g.items.is_empty() {
                        vec![vec![g.name.clone(), fmt_num(g.mob_vnum), g.drop_type.clone(), String::new(), String::new(), String::new(), String::new()]]
                    } else {
                        g.items
                            .iter()
                            .map(|item| {
                                vec![
                                    g.name.clone(),
                                    fmt_num(g.mob_vnum),
                                    g.drop_type.clone(),
                                    fmt_num(item.item_vnum),
                                    fmt_num(item.count),
                                    fmt_num(item.percent),
                                    format!("{:.4}", real_drop_chance_percent(item.percent)),
                                ]
                            })
                            .collect()
                    }
                })
                .collect()
        });
        let headers = ["Gruppe", "Mob-VNUM", "Drop-Typ", "Item-VNUM", "Anzahl", "Prozent (Datei)", "Prozent (real)"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        spawn_local(async move {
            if let Err(e) = export_rows_as_csv("mob_drop_export.csv", headers, rows).await {
                export_error.set(Some(e));
            }
            exporting.set(false);
        });
    };

    let reload = move || {
        if source.get_untracked() == Source::Server {
            load_from_server();
        } else if let Some(p) = local_path.get_untracked() {
            load_local_content(p);
        }
    };

    // Bereiche bleiben nach dem ersten Besuch (versteckt) gemountet - der
    // Listener muss daher prüfen, ob der Drop-Generator gerade aktiv ist.
    let key_handle = window_event_listener(leptos::ev::keydown, move |e| {
        if nav().section.get_untracked() != Section::DropGenerator {
            return;
        }
        if (e.ctrl_key() || e.meta_key()) && e.key().to_lowercase() == "s" {
            e.prevent_default();
            if dirty.get_untracked() && !saving.get_untracked() && groups.with_untracked(|g| g.is_some()) {
                save_confirm.set(true);
            }
        }
    });
    on_cleanup(move || key_handle.remove());

    let on_picker_rows = move |rows: Vec<EntityRow>| ensure_icons(icons, rows.iter().map(|r| r.vnum));

    let selected = Memo::new(move |_| {
        selected_index.get().filter(|i| groups.with(|g| g.as_ref().is_some_and(|g| *i < g.len())))
    });
    let has_groups = Memo::new(move |_| groups.with(|g| g.is_some()));
    let has_raw = Memo::new(move |_| raw_recovery.with(|r| r.is_some()));

    let view_mode = Memo::new(move |_| {
        if loading.get() && !has_groups.get() && !has_raw.get() {
            ViewMode::Skeleton
        } else if load_error.with(|e| e.is_some()) && !has_groups.get() && !has_raw.get() {
            ViewMode::Error
        } else {
            ViewMode::Main
        }
    });

    let source_btn = move |s: Source| {
        move || {
            if source.get() == s {
                "px-3 py-1 bg-primary text-primary-foreground"
            } else {
                "px-3 py-1 "
            }
        }
    };

    let main_view = move || {
        view! {
            <div class="flex h-full flex-col gap-3">
                <div class="flex items-center justify-between">
                    <div class="flex items-center gap-2">
                        <h1 class="text-2xl font-semibold">"Mob Drop Editor"</h1>
                        <Button
                            variant=ButtonVariant::Ghost
                            size=ButtonSize::IconSm
                            title="Hilfe zu diesem Modul"
                            on_click=move |_| open_manual(Some(EmbeddedModule::MobDropEditor.into()))
                        >
                            <Icon name="HelpCircle" class="size-4"/>
                        </Button>
                        <ServerInfoPopover/>
                    </div>
                    <div class="flex items-center gap-2">
                        {move || save_ok.get().map(|ok| view! {
                            <span class="flex items-center gap-1 text-sm text-green-600">
                                <Icon name="CheckCircle2" class="size-4"/>
                                " "
                                {ok}
                            </span>
                        })}
                        <Button
                            variant=ButtonVariant::Outline
                            on_click=move |_| reverse_lookup_open.set(true)
                            disabled=Signal::derive(move || !has_groups.get())
                        >
                            <Icon name="Crosshair" class="size-4"/>
                            "Wer droppt…?"
                        </Button>
                        <Button
                            variant=ButtonVariant::Outline
                            on_click=move |_| export_filtered()
                            disabled=Signal::derive(move || !has_groups.get() || exporting.get() || filtered_groups.with(|f| f.is_empty()))
                            title="Exportiert die aktuell gefilterte Mob-Liste (siehe Suchfeld) als CSV"
                        >
                            <Icon name="Download" class="size-4"/>
                            {move || if exporting.get() { "Exportiere…" } else { "Exportieren" }}
                        </Button>
                        <Button
                            variant=ButtonVariant::Outline
                            on_click=move |_| reload()
                            disabled=Signal::derive(move || {
                                loading.get() || (source.get() == Source::Local && local_path.with(|p| p.is_none()))
                            })
                        >
                            <Icon
                                name="RefreshCw"
                                class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })
                            />
                            "Neu laden"
                        </Button>
                        <Button
                            on_click=move |_| save_confirm.set(true)
                            disabled=Signal::derive(move || saving.get() || !has_groups.get() || !dirty.get())
                        >
                            {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                        </Button>
                    </div>
                </div>

                <div class="flex overflow-hidden rounded-md border border-border text-sm w-fit">
                    <button on:click=move |_| switch_source(Source::Server) class=source_btn(Source::Server)>
                        "Server (SFTP)"
                    </button>
                    <button on:click=move |_| switch_source(Source::Local) class=source_btn(Source::Local)>
                        "Lokale Datei prüfen/reparieren"
                    </button>
                </div>

                {move || save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                {move || export_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

                <Show when=move || {
                    duplicate_items.with(|d| !d.is_empty()) || duplicate_mobs.with(|d| !d.is_empty())
                        || numbering_issues.with(|n| !n.is_empty())
                }>
                    <div class="flex items-center justify-between gap-2 rounded-md border border-amber-500/40 bg-amber-500/10 p-2 text-sm text-amber-700 dark:text-amber-400">
                        <span class="flex items-center gap-2">
                            <Icon name="Copy" class="size-4 shrink-0"/>
                            {move || {
                                let mut parts = vec![];
                                let di = duplicate_items.with(|d| d.len());
                                let dm = duplicate_mobs.with(|d| d.len());
                                let ni = numbering_issues.with(|n| n.len());
                                if di > 0 {
                                    parts.push(format!("{di} Mob(s) mit doppelten Items"));
                                }
                                if dm > 0 {
                                    parts.push(format!("{dm} doppelte(r) Mob-Eintrag"));
                                }
                                if ni > 0 {
                                    parts.push(format!("{ni} Mob(s) mit fehlerhafter Nummerierung"));
                                }
                                parts.join(", ")
                            }}
                        </span>
                        <Button
                            size=ButtonSize::Sm
                            variant=ButtonVariant::Outline
                            on_click=move |_| {
                                ensure_icons(icons, duplicate_items.get_untracked().iter().map(|f| f.item_vnum as i64));
                                duplicates_open.set(true);
                            }
                        >
                            "Anzeigen"
                        </Button>
                    </div>
                </Show>

                {move || (source.get() == Source::Local).then(|| local_path.get()).flatten().map(|p| view! {
                    <p class="text-xs text-muted-foreground">"Datei: " <code>{p}</code></p>
                })}

                <Show when=move || source.get() == Source::Local && !has_groups.get() && !has_raw.get()>
                    <div class="flex flex-1 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border">
                        <p class="text-sm text-muted-foreground">
                            "Wähle eine lokale " <code>"mob_drop_item.txt"</code> ", um sie auf korrekte Syntax zu prüfen."
                        </p>
                        <Button on_click=move |_| pick_local_file()>
                            <Icon name="FolderOpen" class="size-4"/>
                            "Datei wählen…"
                        </Button>
                    </div>
                </Show>

                <Show when=move || source.get() == Source::Local && has_raw.get()>
                    <div class="flex flex-1 flex-col gap-2">
                        <div class="flex items-start gap-2 rounded-md border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                            <span class="whitespace-pre-wrap">
                                {move || raw_recovery.with(|r| r.as_ref().map(|r| r.error.clone()).unwrap_or_default())}
                            </span>
                        </div>
                        <p class="text-xs text-muted-foreground">
                            "Die Syntax stimmt nicht - korrigiere die betroffene Stelle unten (Zeilennummer siehe Fehler oben) und prüfe erneut. Erst wenn es fehlerfrei einliest, kannst du im strukturierten Editor weiterbearbeiten und speichern."
                        </p>
                        <textarea
                            prop:value=move || raw_recovery.with(|r| r.as_ref().map(|r| r.content.clone()).unwrap_or_default())
                            on:input=move |e| {
                                let v = event_target_value(&e);
                                raw_recovery.update(|r| {
                                    if let Some(r) = r.as_mut() {
                                        r.content = v;
                                    }
                                });
                            }
                            spellcheck="false"
                            class="min-h-0 flex-1 rounded-md border border-border bg-background p-2 font-mono text-xs"
                        ></textarea>
                        <div class="flex gap-2">
                            <Button on_click=move |_| retry_parse_raw() disabled=raw_checking>
                                {move || if raw_checking.get() { "Prüfe…" } else { "Erneut prüfen" }}
                            </Button>
                            <Button variant=ButtonVariant::Outline on_click=move |_| pick_local_file()>
                                "Andere Datei wählen"
                            </Button>
                        </div>
                    </div>
                </Show>

                <Show when=move || has_groups.get()>
                    <div class="flex min-h-0 flex-1 gap-4">
                        <GroupSidebar
                            groups=groups
                            filtered_groups=filtered_groups
                            group_search=group_search
                            selected_index=selected_index
                            on_create_click=move |_| creating.set(true)
                        />

                        <div class="min-w-0 flex-1 space-y-4 overflow-y-auto">
                            <Show when=move || selected.get().is_none()>
                                <p class="text-sm text-muted-foreground">"Wähle links einen Mob aus."</p>
                            </Show>
                            {move || selected.get().map(|si| view! {
                                <MobDetailPanel
                                    groups=groups
                                    selected_index=si
                                    icons=icons
                                    dirty=dirty
                                    on_remove_item=remove_item
                                    on_delete_click=move |_| delete_confirm.set(Some(si))
                                    on_add_item_click=move |_| item_picker.set(true)
                                    on_simulate_click=move |_| simulator_open.set(true)
                                />
                            })}

                            <BulkEditPanel
                                has_groups=Signal::derive(move || groups.with(|g| g.as_ref().is_some_and(|g| !g.is_empty())))
                                has_selection=Signal::derive(move || selected.get().is_some())
                                selected_name=Signal::derive(move || selected.get().and_then(|si| with_group(groups, si, |g| g.name.clone())))
                                on_apply=apply_bulk_edit
                            />
                        </div>
                    </div>

                    <Show when=move || item_picker.get()>
                        <ItemPickerModal
                            on_close=move |_| item_picker.set(false)
                            on_pick=add_item
                            icons=icons
                            on_rows_change=on_picker_rows
                        />
                    </Show>

                    <Show when=move || reverse_lookup_open.get()>
                        <ReverseLookupModal
                            groups=groups
                            on_select_mob=move |index: usize| {
                                selected_index.set(Some(index));
                                reverse_lookup_open.set(false);
                            }
                            on_close=move |_| reverse_lookup_open.set(false)
                        />
                    </Show>

                    <Show when=move || creating.get()>
                        <CreateMobModal on_close=move |_| creating.set(false) on_create=handle_mob_created/>
                    </Show>

                    {move || (simulator_open.get()).then(|| selected.get()).flatten().map(|si| view! {
                        <SimulatorModal groups=groups group_index=si icons=icons on_close=move |_| simulator_open.set(false)/>
                    })}

                    <Show when=move || duplicates_open.get()>
                        <DuplicatesModal
                            duplicate_items=duplicate_items
                            duplicate_mobs=duplicate_mobs
                            numbering_issues=numbering_issues
                            groups=groups
                            icons=icons
                            on_navigate_to_group=navigate_to_duplicate_group
                            on_delete_item_occurrence=remove_item
                            on_delete_group=delete_duplicate_group
                            on_close=move |_| duplicates_open.set(false)
                        />
                    </Show>

                    {move || delete_confirm.get().map(|dc| view! {
                        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                            <div class="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
                                <p class="text-sm">
                                    "Mob "
                                    <strong>{move || with_group(groups, dc, |g| g.name.clone()).unwrap_or_default()}</strong>
                                    " wirklich aus der Liste entfernen? Wird erst beim Speichern in die Datei übernommen."
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

                    <Show when=move || save_confirm.get()>
                        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                            <div class="max-h-[85vh] w-[40rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
                                <p class="flex items-start gap-2 text-sm">
                                    <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                                    <span>
                                        {move || if source.get() == Source::Server {
                                            "Die Datei auf dem Server wird gesichert und dann komplett mit dem aktuellen Stand überschrieben."
                                        } else {
                                            "Die lokale Datei wird gesichert und dann komplett mit dem aktuellen Stand überschrieben."
                                        }}
                                    </span>
                                </p>
                                {(source.get_untracked() == Source::Server).then(|| groups.get_untracked()).flatten().map(|d| view! {
                                    <ChangePreview kind="mob_drop" data=serde_json::to_value(&d).unwrap_or_default()/>
                                })}
                                <div class="flex justify-end gap-2">
                                    <Button variant=ButtonVariant::Outline on_click=move |_| save_confirm.set(false)>
                                        "Abbrechen"
                                    </Button>
                                    <Button on_click=move |_| save_file()>"Speichern"</Button>
                                </div>
                            </div>
                        </div>
                    </Show>
                </Show>
            </div>
        }
    };

    move || match view_mode.get() {
        ViewMode::Skeleton => view! {
            <div class="flex gap-4">
                <div class="w-64 shrink-0 space-y-1.5">
                    <Skeleton class="h-8 w-full"/>
                    <Skeleton class="h-8 w-full"/>
                    <Skeleton class="h-8 w-full"/>
                    <Skeleton class="h-8 w-3/4"/>
                </div>
                <div class="flex-1 space-y-2">
                    <Skeleton class="h-6 w-1/3"/>
                    <Skeleton class="h-24 w-full"/>
                    <Skeleton class="h-24 w-full"/>
                </div>
            </div>
        }
        .into_any(),
        ViewMode::Error => view! {
            <div class="max-w-md space-y-3">
                <h1 class="text-2xl font-semibold">"Mob Drop Editor"</h1>
                <p class="text-sm text-destructive">{move || load_error.get().unwrap_or_default()}</p>
                <div class="flex gap-2">
                    <Button on_click=move |_| reload()>"Erneut versuchen"</Button>
                    <Show when=move || source.get() == Source::Local>
                        <Button variant=ButtonVariant::Outline on_click=move |_| pick_local_file()>
                            "Andere Datei wählen"
                        </Button>
                    </Show>
                </div>
            </div>
        }
        .into_any(),
        ViewMode::Main => main_view().into_any(),
    }
}
