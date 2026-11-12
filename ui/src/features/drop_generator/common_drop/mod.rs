//! Common-Drops (`common_drop_item.txt`) im klassischen Rang-Format:
//! eine Zeile = eine Regel mit 4 Blöcken (PAWN, S_PAWN, KNIGHT, S_KNIGHT).
//! Format, Prüfungen und Hintergründe: `src-tauri/src/common_drop.rs`.
//!
//! Die Chance wird hier in echten Prozent eingegeben; in die Datei kommt
//! der Wert x 4 (400 = 100 %). Kommentarzeilen bleiben unverändert an ihrer
//! Stelle, neue Regeln kommen ans Dateiende.

pub mod logic;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::mob_drop_editor::components::shared::{js_number, num_sig, NumInput};
use crate::features::shared::{EntityBrowser, EntityKind, EntityRow};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::invoke;
use crate::util::{log_activity, use_save_shortcut};
use leptos::prelude::*;
use leptos::task::spawn_local;
use logic::*;
use serde_json::json;
use std::collections::{BTreeSet, HashMap, HashSet};

type IconCache = RwSignal<HashMap<u32, Option<String>>>;

/// Farbe pro Rang (Liste, Karten, Vorschau).
const RANK_TINT: [&str; RANK_COUNT] = [
    "text-sky-600 dark:text-sky-400",
    "text-emerald-600 dark:text-emerald-400",
    "text-amber-600 dark:text-amber-400",
    "text-rose-600 dark:text-rose-400",
];

const INPUT: &str = "w-full rounded-md border border-border bg-background px-2 py-1 text-sm";

#[derive(Clone, Copy, PartialEq)]
enum PickTarget {
    NewRule,
    /// Item für alle 4 Blöcke der Regel.
    Shared(usize),
    /// Item nur für einen Block.
    Rank(usize, usize),
}

#[derive(Clone, Copy, PartialEq)]
enum LineKind {
    Rule,
    Invalid,
}

#[component]
fn ItemIcon(#[prop(into)] vnum: Signal<u32>, cache: IconCache, #[prop(optional)] large: bool) -> impl IntoView {
    Effect::new(move |_| {
        let v = vnum.get();
        if v == 0 || cache.with_untracked(|c| c.contains_key(&v)) {
            return;
        }
        cache.update(|c| {
            c.insert(v, None);
        });
        spawn_local(async move {
            let url = invoke::<Option<String>>("get_item_icon", &json!({ "vnum": v })).await.ok().flatten();
            cache.update(|c| {
                c.insert(v, url);
            });
        });
    });
    let size = if large { "size-12" } else { "size-9" };
    view! {
        <div class=format!("flex {size} shrink-0 items-center justify-center overflow-hidden rounded-md border border-border bg-muted/40")>
            {move || {
                let v = vnum.get();
                match cache.with(|c| c.get(&v).cloned().flatten()).filter(|u| !u.is_empty()) {
                    Some(url) => view! { <img src=url alt="" class="size-full object-contain"/> }.into_any(),
                    None => view! { <Icon name="Package" class="size-4 text-muted-foreground"/> }.into_any(),
                }
            }}
        </div>
    }
}

#[component]
pub fn CommonDropEditor() -> impl IntoView {
    let file = RwSignal::new(None::<DropFile>);
    let snapshot = RwSignal::new(None::<DropFile>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let confirm_save = RwSignal::new(false);
    let confirm_reload = RwSignal::new(false);
    let delete_confirm = RwSignal::new(None::<usize>);
    let picker = RwSignal::new(None::<PickTarget>);
    let selected = RwSignal::new(None::<usize>);
    let split = RwSignal::new(false);
    let search = RwSignal::new(String::new());
    let rank_filter = RwSignal::new(None::<usize>);
    let raw_edit = RwSignal::new(String::new());
    let raw_error = RwSignal::new(None::<String>);
    let issues = RwSignal::new(Vec::<Issue>::new());
    let checks = RwSignal::new(HashMap::<u32, ItemCheck>::new());
    let pending_checks = StoredValue::new(HashSet::<u32>::new());
    let db_error = RwSignal::new(None::<String>);
    let icons: IconCache = RwSignal::new(HashMap::new());
    let validate_seq = StoredValue::new(0u64);

    let dirty = Memo::new(move |_| file.with(|f| f.is_some() && snapshot.with(|s| s != f)));
    Effect::new(move |_| report_section_dirty(Section::DropGenerator, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::DropGenerator, false));

    // ---- Prüfungen ----
    Effect::new(move |_| {
        let Some(f) = file.get() else {
            issues.set(vec![]);
            return;
        };
        let seq = validate_seq.get_value() + 1;
        validate_seq.set_value(seq);
        spawn_local(async move {
            if let Ok(list) = invoke::<Vec<Issue>>("validate_common_drop", &json!({ "file": f })).await {
                if validate_seq.get_value() == seq {
                    issues.set(list);
                }
            }
        });
    });

    Effect::new(move |_| {
        let vnums: BTreeSet<u32> = file.with(|f| {
            f.as_ref()
                .map(|f| {
                    f.lines
                        .iter()
                        .flat_map(|l| match l {
                            Line::Rule { blocks } => blocks.iter().map(|b| b.vnum).collect::<Vec<_>>(),
                            _ => vec![],
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        let missing: Vec<u32> = vnums
            .into_iter()
            .filter(|v| !checks.with_untracked(|c| c.contains_key(v)) && !pending_checks.with_value(|p| p.contains(v)))
            .collect();
        if missing.is_empty() {
            return;
        }
        pending_checks.update_value(|p| p.extend(missing.iter().copied()));
        spawn_local(async move {
            match invoke::<Vec<ItemCheck>>("check_common_drop_items", &json!({ "vnums": missing })).await {
                Ok(list) => {
                    checks.update(|c| {
                        for check in list {
                            c.insert(check.vnum, check);
                        }
                    });
                    db_error.set(None);
                }
                Err(e) => db_error.set(Some(e)),
            }
        });
    });

    let all_issues = Memo::new(move |_| {
        let mut list = issues.get();
        file.with(|f| {
            if let Some(f) = f {
                checks.with(|c| list.extend(item_issues(f, c)));
            }
        });
        list
    });
    let error_count = Memo::new(move |_| all_issues.with(|l| l.iter().filter(|i| i.is_error()).count()));
    let warning_count = Memo::new(move |_| all_issues.with(|l| l.iter().filter(|i| !i.is_error()).count()));
    let line_has_error = move |li: usize| all_issues.with(|l| l.iter().any(|i| i.line_index == li && i.is_error()));
    let line_has_warning = move |li: usize| all_issues.with(|l| l.iter().any(|i| i.line_index == li && !i.is_error()));

    let label = move |vnum: u32| checks.with(|c| c.get(&vnum).and_then(|c| c.locale_name.clone()));

    // ---- Datenzugriff ----
    let rule_blocks = move |li: usize| -> Option<Vec<Block>> {
        file.with(|f| match f.as_ref()?.lines.get(li)? {
            Line::Rule { blocks } => Some(blocks.clone()),
            _ => None,
        })
    };
    let block = move |li: usize, r: usize| rule_blocks(li).and_then(|b| b.get(r).cloned());
    let update_blocks = move |li: usize, f: Box<dyn FnOnce(&mut Vec<Block>)>| {
        file.update(|file| {
            if let Some(Line::Rule { blocks }) = file.as_mut().and_then(|file| file.lines.get_mut(li)) {
                f(blocks);
            }
        });
        save_ok.set(false);
    };
    let update_block = move |li: usize, r: usize, f: Box<dyn FnOnce(&mut Block)>| {
        update_blocks(
            li,
            Box::new(move |blocks| {
                if let Some(b) = blocks.get_mut(r) {
                    f(b);
                }
            }),
        );
    };

    let select = move |li: usize| {
        selected.set(Some(li));
        split.set(rule_blocks(li).is_some_and(|b| !is_shared(&b)));
        let raw = file.with_untracked(|f| match f.as_ref().and_then(|f| f.lines.get(li)) {
            Some(Line::Invalid { raw, .. }) => Some(raw.clone()),
            _ => None,
        });
        raw_edit.set(raw.map(|r| raw_for_edit(&r)).unwrap_or_default());
        raw_error.set(None);
    };

    // ---- Laden / Speichern ----
    let load = move || {
        confirm_reload.set(false);
        loading.set(true);
        load_error.set(None);
        save_ok.set(false);
        save_error.set(None);
        selected.set(None);
        spawn_local(async move {
            match invoke::<DropFile>("read_common_drop_file", &json!({})).await {
                Ok(f) => {
                    pending_checks.update_value(|p| p.clear());
                    checks.set(HashMap::new());
                    snapshot.set(Some(f.clone()));
                    file.set(Some(f));
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let save = move || {
        let Some(f) = file.get_untracked() else { return };
        confirm_save.set(false);
        saving.set(true);
        save_error.set(None);
        spawn_local(async move {
            match invoke::<Option<String>>("write_common_drop_file", &json!({ "file": f })).await {
                Ok(_) => {
                    let n = f.lines.iter().filter(|l| matches!(l, Line::Rule { .. })).count();
                    snapshot.set(Some(f));
                    save_ok.set(true);
                    log_activity(
                        Section::DropGenerator.into(),
                        "save",
                        &format!("common_drop_item.txt gespeichert ({n} Regel(n))"),
                        Some("common_drop_item.txt"),
                        None,
                    );
                }
                Err(e) => save_error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let can_save = Memo::new(move |_| dirty.get() && error_count.get() == 0 && !saving.get());
    use_save_shortcut(Section::DropGenerator, move || can_save.get_untracked(), move || confirm_save.set(true));

    // ---- Regeln anlegen / löschen ----
    let on_pick = move |row: EntityRow| {
        let Some(target) = picker.get_untracked() else { return };
        let vnum = row.vnum.max(0) as u32;
        checks.update(|c| {
            c.entry(vnum).or_insert(ItemCheck { vnum, exists: true, locale_name: Some(row.name.clone()), name_collision: None });
        });
        // Vollständige Prüfung (auch Namenskollision) nachziehen.
        if !pending_checks.with_value(|p| p.contains(&vnum)) {
            pending_checks.update_value(|p| {
                p.insert(vnum);
            });
            spawn_local(async move {
                if let Ok(list) = invoke::<Vec<ItemCheck>>("check_common_drop_items", &json!({ "vnums": [vnum] })).await {
                    checks.update(|c| {
                        for check in list {
                            c.insert(check.vnum, check);
                        }
                    });
                }
            });
        }
        match target {
            PickTarget::NewRule => {
                let mut new_index = 0;
                file.update(|f| {
                    if let Some(f) = f.as_mut() {
                        f.lines.push(Line::Rule { blocks: new_rule(vnum, &row.name) });
                        new_index = f.lines.len() - 1;
                    }
                });
                search.set(String::new());
                rank_filter.set(None);
                select(new_index);
            }
            PickTarget::Shared(li) => {
                let name = clean_name(&row.name);
                update_blocks(
                    li,
                    Box::new(move |blocks| {
                        for b in blocks.iter_mut() {
                            b.vnum = vnum;
                            b.name = name.clone();
                        }
                    }),
                );
            }
            PickTarget::Rank(li, r) => {
                let name = clean_name(&row.name);
                update_block(
                    li,
                    r,
                    Box::new(move |b| {
                        b.vnum = vnum;
                        b.name = name;
                    }),
                );
            }
        }
        save_ok.set(false);
        picker.set(None);
    };

    let duplicate = move |li: usize| {
        let Some(blocks) = rule_blocks(li) else { return };
        file.update(|f| {
            if let Some(f) = f.as_mut() {
                f.lines.insert(li + 1, Line::Rule { blocks });
            }
        });
        select(li + 1);
    };

    let delete_line = move |li: usize| {
        file.update(|f| {
            if let Some(f) = f.as_mut() {
                if li < f.lines.len() {
                    f.lines.remove(li);
                }
            }
        });
        selected.set(None);
        delete_confirm.set(None);
        save_ok.set(false);
    };

    let apply_raw = move |li: usize| {
        let line = raw_from_edit(&raw_edit.get_untracked());
        spawn_local(async move {
            match invoke::<Line>("parse_common_drop_line", &json!({ "line": line })).await {
                Ok(Line::Invalid { error, .. }) => raw_error.set(Some(error)),
                Ok(parsed) => {
                    file.update(|f| {
                        if let Some(slot) = f.as_mut().and_then(|f| f.lines.get_mut(li)) {
                            *slot = parsed;
                        }
                    });
                    select(li);
                }
                Err(e) => raw_error.set(Some(e)),
            }
        });
    };

    // ---- Liste ----
    let visible = Memo::new(move |_| {
        let q = search.get();
        let rank = rank_filter.get();
        file.with(|f| {
            let Some(f) = f else { return vec![] };
            f.lines
                .iter()
                .enumerate()
                .filter_map(|(li, l)| match l {
                    Line::Invalid { .. } => Some(li),
                    Line::Rule { blocks } => {
                        let rank_ok = rank.is_none_or(|r| blocks.get(r).is_some_and(Block::is_active));
                        (rank_ok && matches_search(blocks, label, &q)).then_some(li)
                    }
                    Line::Text { .. } => None,
                })
                .collect::<Vec<_>>()
        })
    });
    let rule_count = Memo::new(move |_| {
        file.with(|f| f.as_ref().map(|f| f.lines.iter().filter(|l| matches!(l, Line::Rule { .. })).count()).unwrap_or(0))
    });
    let text_count = Memo::new(move |_| {
        file.with(|f| f.as_ref().map(|f| f.lines.iter().filter(|l| matches!(l, Line::Text { .. })).count()).unwrap_or(0))
    });
    let has_file = Memo::new(move |_| file.with(|f| f.is_some()));

    let selected_kind = Memo::new(move |_| {
        let li = selected.get()?;
        file.with(|f| match f.as_ref()?.lines.get(li)? {
            Line::Rule { .. } => Some((li, LineKind::Rule)),
            Line::Invalid { .. } => Some((li, LineKind::Invalid)),
            Line::Text { .. } => None,
        })
    });

    let list_entry = move |li: usize| {
        let is_invalid = move || file.with(|f| matches!(f.as_ref().and_then(|f| f.lines.get(li)), Some(Line::Invalid { .. })));
        let first_vnum = Signal::derive(move || block(li, 0).map(|b| b.vnum).unwrap_or(0));
        let title = move || {
            let Some(blocks) = rule_blocks(li) else { return String::new() };
            let b = &blocks[0];
            if is_shared(&blocks) {
                label(b.vnum).filter(|l| !l.is_empty()).unwrap_or_else(|| if b.name.is_empty() { format!("Item {}", b.vnum) } else { b.name.clone() })
            } else {
                format!("{} (+ weitere Items)", label(b.vnum).unwrap_or_else(|| format!("Item {}", b.vnum)))
            }
        };
        let subtitle = move || {
            let Some(blocks) = rule_blocks(li) else { return String::new() };
            let active: Vec<&Block> = blocks.iter().filter(|b| b.is_active()).collect();
            let levels = match active.first() {
                Some(b) if active.iter().all(|o| o.level_from == b.level_from && o.level_to == b.level_to) => {
                    format!("Lv {}–{}", b.level_from, b.level_to)
                }
                Some(_) => "Lv gemischt".to_string(),
                None => "aus".to_string(),
            };
            format!("#{} · {levels}", blocks[0].vnum)
        };
        view! {
            <button
                type="button"
                on:click=move |_| select(li)
                class=move || {
                    format!(
                        "w-full rounded-md border p-2 text-left text-sm transition-colors hover:bg-muted {} {}",
                        if selected.get() == Some(li) { "border-primary bg-muted" } else { "border-border" },
                        if is_invalid() { "border-destructive/60" } else { "" },
                    )
                }
            >
                {move || if is_invalid() {
                    view! {
                        <div class="flex items-center gap-2 text-destructive">
                            <Icon name="XCircle" class="size-4 shrink-0"/>
                            <span class="font-medium">{format!("Zeile {} ist fehlerhaft", li + 1)}</span>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="flex items-center gap-2">
                            <ItemIcon vnum=first_vnum cache=icons/>
                            <div class="min-w-0 flex-1">
                                <div class="flex items-center gap-1">
                                    <span class="truncate font-medium">{title}</span>
                                    <Show when=move || line_has_error(li)>
                                        <Icon name="XCircle" class="size-3.5 shrink-0 text-destructive"/>
                                    </Show>
                                    <Show when=move || !line_has_error(li) && line_has_warning(li)>
                                        <Icon name="AlertTriangle" class="size-3.5 shrink-0 text-amber-500"/>
                                    </Show>
                                </div>
                                <div class="truncate text-xs text-muted-foreground">{subtitle}</div>
                            </div>
                        </div>
                        <div class="mt-1.5 flex gap-1">
                            {(0..RANK_COUNT).map(|r| {
                                let chip = move || {
                                    let b = block(li, r)?;
                                    Some((b.is_active(), fmt_percent(chance_to_percent(b.chance))))
                                };
                                view! {
                                    <span
                                        title=RANKS[r].key
                                        class=move || {
                                            if chip().is_some_and(|c| c.0) {
                                                format!("rounded border border-current/30 px-1.5 py-0.5 text-[10px] font-medium {}", RANK_TINT[r])
                                            } else {
                                                "rounded border border-border px-1.5 py-0.5 text-[10px] text-muted-foreground/60 line-through".to_string()
                                            }
                                        }
                                    >
                                        {RANKS[r].short} " " {move || chip().map(|c| if c.0 { c.1 } else { "aus".into() }).unwrap_or_default()}
                                    </span>
                                }
                            }).collect_view()}
                        </div>
                    }.into_any()
                }}
            </button>
        }
    };

    // ---- Detail: Rang-Karte ----
    let rank_card = move |li: usize, r: usize| {
        let b = move || block(li, r);
        let active = move || b().is_some_and(|b| b.is_active());
        let vnum = Signal::derive(move || b().map(|b| b.vnum).unwrap_or(0));
        let rank_issues = move || all_issues.with(|l| l.iter().filter(|i| i.line_index == li && i.rank == Some(r)).cloned().collect::<Vec<_>>());
        view! {
            <div class=move || format!(
                "space-y-3 rounded-lg border p-3 transition-opacity {}",
                if rank_issues().iter().any(|i| i.is_error()) { "border-destructive/60" } else { "border-border" },
            )>
                <div class="flex items-start justify-between gap-2">
                    <div>
                        <div class=format!("text-sm font-semibold {}", RANK_TINT[r])>{format!("{} · {}", r + 1, RANKS[r].key)}</div>
                        <div class="text-xs text-muted-foreground">{RANKS[r].title} ", " {RANKS[r].examples}</div>
                    </div>
                    <label class="flex cursor-pointer items-center gap-1.5 text-xs">
                        <input
                            type="checkbox"
                            class="size-4"
                            prop:checked=active
                            on:change=move |e| {
                                let on = event_target_checked(&e);
                                update_block(li, r, Box::new(move |b| set_active(b, on)));
                            }
                        />
                        {move || if active() { "aktiv" } else { "aus" }}
                    </label>
                </div>

                <Show when=move || split.get()>
                    <div class="flex items-center gap-2 rounded-md bg-muted/40 p-2">
                        <ItemIcon vnum=vnum cache=icons/>
                        <div class="min-w-0 flex-1">
                            <div class="truncate text-sm">{move || label(vnum.get()).unwrap_or_else(|| format!("Item {}", vnum.get()))}</div>
                            <div class="text-xs text-muted-foreground">{move || format!("VNUM {}", vnum.get())}</div>
                        </div>
                        <Button size=ButtonSize::Xs variant=ButtonVariant::Outline on_click=move |_| picker.set(Some(PickTarget::Rank(li, r)))>
                            "Ändern"
                        </Button>
                    </div>
                    <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                        "Notiz (Name-Feld)"
                        <input
                            class=INPUT
                            prop:value=move || b().map(|b| b.name).unwrap_or_default()
                            on:input=move |e| {
                                let v = clean_name(&event_target_value(&e));
                                update_block(li, r, Box::new(move |b| b.name = v));
                            }
                        />
                    </label>
                </Show>

                <div class=move || if active() { "space-y-3" } else { "space-y-3 opacity-50" }>
                    <div class="grid grid-cols-2 gap-2">
                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                            "Spieler-Level von"
                            <NumInput
                                min="0"
                                class=INPUT
                                value=num_sig(move || b().map(|b| b.level_from as f64).unwrap_or(0.0))
                                on_input=move |v: String| {
                                    let n = js_number(&v);
                                    let n = if n.is_finite() { n.trunc() as i64 } else { 0 };
                                    update_block(li, r, Box::new(move |b| b.level_from = n));
                                }
                            />
                        </label>
                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                            "bis"
                            <NumInput
                                min="0"
                                class=INPUT
                                value=num_sig(move || b().map(|b| b.level_to as f64).unwrap_or(0.0))
                                on_input=move |v: String| {
                                    let n = js_number(&v);
                                    let n = if n.is_finite() { n.trunc() as i64 } else { 0 };
                                    update_block(li, r, Box::new(move |b| b.level_to = n));
                                }
                            />
                        </label>
                    </div>
                    <div class="grid grid-cols-[1fr_auto] items-end gap-2">
                        <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                            "Chance in %"
                            <div class="relative">
                                <NumInput
                                    min="0"
                                    max="100"
                                    step="0.01"
                                    class=format!("{INPUT} pr-7")
                                    value=num_sig(move || b().map(|b| chance_to_percent(b.chance)).unwrap_or(0.0))
                                    on_input=move |v: String| {
                                        let c = percent_to_chance(js_number(&v));
                                        update_block(li, r, Box::new(move |b| b.chance = c));
                                    }
                                />
                                <span class="pointer-events-none absolute right-2 top-1/2 -translate-y-1/2 text-xs text-muted-foreground">"%"</span>
                            </div>
                        </label>
                        <div class="pb-1 text-right text-xs leading-tight text-muted-foreground">
                            <div>{move || b().map(|b| format!("Datei: {}", fmt_chance(b.chance))).unwrap_or_default()}</div>
                            <div>{move || b().map(|b| every_nth_label(b.chance)).unwrap_or_default()}</div>
                        </div>
                    </div>
                    <label class="flex items-center gap-2 text-xs text-muted-foreground" title="Der Server liest das Feld, benutzt es aber nicht - es dropt immer 1 Stück.">
                        <span class="shrink-0">"Anzahl (vom Server ignoriert)"</span>
                        <NumInput
                            class="w-24 rounded-md border border-border bg-background px-2 py-0.5 text-xs"
                            value=num_sig(move || b().map(|b| b.count as f64).unwrap_or(0.0))
                            on_input=move |v: String| {
                                let n = js_number(&v);
                                let n = if n.is_finite() { n.trunc() as i64 } else { 0 };
                                update_block(li, r, Box::new(move |b| b.count = n));
                            }
                        />
                    </label>
                </div>

                {move || rank_issues().into_iter().map(|i| { let err = i.is_error(); view! {
                    <p class=if err { "text-xs text-destructive" } else { "text-xs text-amber-600 dark:text-amber-400" }>
                        {i.message}
                    </p>
                }}).collect_view()}
            </div>
        }
    };

    // ---- Detail: ganze Regel ----
    let rule_detail = move |li: usize| {
        let first_vnum = Signal::derive(move || block(li, 0).map(|b| b.vnum).unwrap_or(0));
        let rule_issues = move || all_issues.with(|l| l.iter().filter(|i| i.line_index == li && i.rank.is_none()).cloned().collect::<Vec<_>>());
        let copy_from = RwSignal::new(0usize);
        view! {
            <div class="space-y-3">
                <div class="flex flex-wrap items-start gap-3 rounded-lg border border-border p-3">
                    <Show
                        when=move || !split.get()
                        fallback=move || view! {
                            <div class="min-w-0 flex-1 text-sm text-muted-foreground">
                                "Jeder Rang hat sein eigenes Item - Auswahl in den Karten unten."
                            </div>
                        }
                    >
                        <ItemIcon vnum=first_vnum cache=icons large=true/>
                        <div class="min-w-0 flex-1 space-y-2">
                            <div>
                                <div class="truncate font-medium">
                                    {move || label(first_vnum.get()).unwrap_or_else(|| format!("Item {}", first_vnum.get()))}
                                </div>
                                <div class="text-xs text-muted-foreground">{move || format!("VNUM {}", first_vnum.get())}</div>
                            </div>
                            <label class="flex max-w-sm flex-col gap-1 text-xs text-muted-foreground">
                                "Notiz (Name-Feld, liest der Server nicht)"
                                <input
                                    class=INPUT
                                    prop:value=move || block(li, 0).map(|b| b.name).unwrap_or_default()
                                    on:input=move |e| {
                                        let v = clean_name(&event_target_value(&e));
                                        update_blocks(li, Box::new(move |blocks| for b in blocks.iter_mut() { b.name = v.clone(); }));
                                    }
                                />
                            </label>
                        </div>
                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| picker.set(Some(PickTarget::Shared(li)))>
                            <Icon name="PackageSearch" class="size-4"/>
                            "Item ändern"
                        </Button>
                    </Show>
                    <div class="flex gap-2">
                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline title="Regel duplizieren" on_click=move |_| duplicate(li)>
                            <Icon name="Copy" class="size-4"/>
                        </Button>
                        <Button size=ButtonSize::Sm variant=ButtonVariant::Destructive title="Regel löschen" on_click=move |_| delete_confirm.set(Some(li))>
                            <Icon name="Trash2" class="size-4"/>
                        </Button>
                    </div>
                </div>

                <div class="flex flex-wrap items-center gap-x-4 gap-y-2 text-sm">
                    <label class="flex cursor-pointer items-center gap-2">
                        <input
                            type="checkbox"
                            class="size-4"
                            prop:checked=move || split.get()
                            on:change=move |e| {
                                let on = event_target_checked(&e);
                                split.set(on);
                                if !on {
                                    update_blocks(li, Box::new(|blocks| share_item(blocks)));
                                }
                            }
                        />
                        "Pro Rang ein eigenes Item"
                    </label>
                    <div class="flex items-center gap-2">
                        <span class="text-muted-foreground">"Level und Chance von"</span>
                        <select
                            class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                            on:change=move |e| copy_from.set(event_target_value(&e).parse().unwrap_or(0))
                        >
                            {(0..RANK_COUNT).map(|r| view! { <option value=r.to_string()>{RANKS[r].key}</option> }).collect_view()}
                        </select>
                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| {
                            let from = copy_from.get_untracked();
                            update_blocks(li, Box::new(move |blocks| copy_values_to_all(blocks, from)));
                        }>
                            "auf alle Ränge übertragen"
                        </Button>
                    </div>
                </div>

                {move || {
                    let list = rule_issues();
                    (!list.is_empty()).then(|| view! {
                        <div class="space-y-1 rounded-lg border border-border p-3">
                            {list.into_iter().map(|i| { let err = i.is_error(); view! {
                                <p class=if err { "flex items-start gap-2 text-sm text-destructive" } else { "flex items-start gap-2 text-sm text-amber-600 dark:text-amber-400" }>
                                    <Icon name=if err { "XCircle" } else { "AlertTriangle" } class="mt-0.5 size-4 shrink-0"/>
                                    <span>{i.message}</span>
                                </p>
                            }}).collect_view()}
                        </div>
                    })
                }}

                <div class="grid gap-3 xl:grid-cols-2">
                    {(0..RANK_COUNT).map(|r| rank_card(li, r)).collect_view()}
                </div>

                <div class="space-y-1 rounded-lg border border-border bg-muted/30 p-3">
                    <p class="text-xs font-medium text-muted-foreground">"So steht die Zeile in der Datei (→ = TAB)"</p>
                    <p class="break-all font-mono text-xs leading-relaxed">
                        {move || rule_blocks(li).map(|blocks| {
                            preview_fields(&blocks).into_iter().enumerate().map(|(r, fields)| {
                                let lead = if r == 0 { "" } else { "→" };
                                view! {
                                    <span class="text-muted-foreground">{lead}</span>
                                    <span class=RANK_TINT[r]>{fields.join("→")}</span>
                                }
                            }).collect_view()
                        })}
                    </p>
                </div>
            </div>
        }
    };

    // ---- Detail: fehlerhafte Zeile ----
    let invalid_detail = move |li: usize| {
        let error = move || file.with(|f| match f.as_ref().and_then(|f| f.lines.get(li)) {
            Some(Line::Invalid { error, .. }) => error.clone(),
            _ => String::new(),
        });
        view! {
            <div class="space-y-3 rounded-lg border border-destructive/60 p-3">
                <p class="flex items-start gap-2 text-sm text-destructive">
                    <Icon name="XCircle" class="mt-0.5 size-4 shrink-0"/>
                    <span>{move || format!("Zeile {}: {}", li + 1, error())}</span>
                </p>
                <p class="text-xs text-muted-foreground">
                    "Der Server würde diese Zeile falsch lesen oder gar nicht starten. Korrigiere sie hier (→ steht für einen TAB) oder lösche sie. Solange sie fehlerhaft ist, lässt sich die Datei nicht speichern."
                </p>
                <textarea
                    rows="4"
                    class="w-full rounded-md border border-border bg-background p-2 font-mono text-xs"
                    prop:value=move || raw_edit.get()
                    on:input=move |e| raw_edit.set(event_target_value(&e))
                ></textarea>
                {move || raw_error.get().map(|e| view! { <p class="text-xs text-destructive">{e}</p> })}
                <div class="flex gap-2">
                    <Button size=ButtonSize::Sm on_click=move |_| apply_raw(li)>"Übernehmen"</Button>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Destructive on_click=move |_| delete_confirm.set(Some(li))>
                        <Icon name="Trash2" class="size-4"/>
                        "Zeile löschen"
                    </Button>
                </div>
            </div>
        }
    };

    view! {
        <div class="space-y-4 pb-10">
            <div class="space-y-2 rounded-lg border border-border p-3 text-sm text-muted-foreground">
                <p class="font-medium text-foreground">"common_drop_item.txt - Zusatz-Drops nach Mob-Rang"</p>
                <p>
                    "Jede Regel gilt für alle Mobs eines Rangs (Spalte RANK in mob_proto) und nur für Spieler im eingestellten Level-Bereich. Bei mehreren Spielern zählt der mit dem meisten Schaden. Jede Regel wird bei jedem Kill einzeln gewürfelt, es können also mehrere Items gleichzeitig droppen. Bosse (BOSS, KING) stehen nicht hier, sondern in den Mob-Drops."
                </p>
                <p>
                    "Die Chance gibst du in echten Prozent ein. In der Datei steht der Wert mal 4 (100 % = 400). Sie gilt, wenn Spieler und Mob dasselbe Level haben. Ist der Spieler höher, sinkt sie (10 Level drüber etwa die Hälfte)."
                </p>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                <span>"Kein Live-Neuladen: Änderungen wirken erst nach einem Server-Neustart. Ein unbekanntes Item in dieser Datei verhindert den Server-Start - der Editor prüft deshalb jede VNUM gegen item_proto."</span>
            </div>

            <div class="flex flex-wrap items-center gap-2">
                <Button
                    variant=ButtonVariant::Outline
                    disabled=loading
                    on_click=move |_| if dirty.get_untracked() { confirm_reload.set(true) } else { load() }
                >
                    <Icon
                        name="RefreshCw"
                        class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4".to_string() })
                    />
                    {move || if has_file.get() { "Neu laden" } else { "Laden" }}
                </Button>
                <Show when=move || has_file.get()>
                    <Button variant=ButtonVariant::Outline on_click=move |_| picker.set(Some(PickTarget::NewRule))>
                        <Icon name="Plus" class="size-4"/>
                        "Neue Regel"
                    </Button>
                    <div class="ml-auto flex items-center gap-3 text-sm">
                        <Show when=move || error_count.get() != 0>
                            <span class="flex items-center gap-1 text-destructive">
                                <Icon name="XCircle" class="size-4"/>
                                {move || format!("{} Fehler", error_count.get())}
                            </span>
                        </Show>
                        <Show when=move || warning_count.get() != 0>
                            <span class="flex items-center gap-1 text-amber-600 dark:text-amber-400">
                                <Icon name="AlertTriangle" class="size-4"/>
                                {move || format!("{} Hinweis(e)", warning_count.get())}
                            </span>
                        </Show>
                        <Show when=move || error_count.get() == 0 && warning_count.get() == 0>
                            <span class="flex items-center gap-1 text-green-600">
                                <Icon name="CheckCircle2" class="size-4"/>
                                "Keine Probleme"
                            </span>
                        </Show>
                        <Button disabled=Signal::derive(move || !can_save.get()) on_click=move |_| confirm_save.set(true)>
                            <Icon name="Save" class="size-4"/>
                            {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                        </Button>
                    </div>
                </Show>
            </div>

            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
            {move || save_error.get().map(|e| view! { <p class="whitespace-pre-line text-sm text-destructive">{e}</p> })}
            <Show when=move || save_ok.get()>
                <p class="flex items-center gap-1 text-sm text-green-600">
                    <Icon name="CheckCircle2" class="size-4"/>
                    "Gespeichert (Backup angelegt). Server neu starten, damit die Änderungen wirken."
                </p>
            </Show>
            {move || db_error.get().map(|e| view! {
                <p class="flex items-start gap-2 text-sm text-amber-600 dark:text-amber-400">
                    <Icon name="Database" class="mt-0.5 size-4 shrink-0"/>
                    <span>"Items konnten nicht gegen item_proto geprüft werden: " {e}</span>
                </p>
            })}

            <Show when=move || has_file.get()>
                <div class="flex gap-4">
                    <div class="flex w-80 shrink-0 flex-col gap-2">
                        <div class="relative">
                            <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                            <input
                                prop:value=move || search.get()
                                on:input=move |e| search.set(event_target_value(&e))
                                placeholder="Item, Notiz oder VNUM…"
                                class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                            />
                        </div>
                        <div class="flex flex-wrap gap-1">
                            <button
                                type="button"
                                on:click=move |_| rank_filter.set(None)
                                class=move || format!(
                                    "rounded-md border px-2 py-0.5 text-xs {}",
                                    if rank_filter.get().is_none() { "border-primary bg-primary text-primary-foreground" } else { "border-border hover:bg-muted" },
                                )
                            >
                                "Alle"
                            </button>
                            {(0..RANK_COUNT).map(|r| view! {
                                <button
                                    type="button"
                                    title=format!("Nur Regeln, die bei {} aktiv sind", RANKS[r].key)
                                    on:click=move |_| rank_filter.set(if rank_filter.get_untracked() == Some(r) { None } else { Some(r) })
                                    class=move || format!(
                                        "rounded-md border px-2 py-0.5 text-xs {}",
                                        if rank_filter.get() == Some(r) { "border-primary bg-primary text-primary-foreground" } else { "border-border hover:bg-muted" },
                                    )
                                >
                                    {RANKS[r].key}
                                </button>
                            }).collect_view()}
                        </div>
                        <div class="max-h-[65vh] space-y-1 overflow-y-auto pr-1">
                            <For each=move || visible.get() key=|li| *li let:li>
                                {list_entry(li)}
                            </For>
                            <Show when=move || visible.with(|v| v.is_empty())>
                                <p class="p-2 text-sm text-muted-foreground">
                                    {move || if rule_count.get() == 0 { "Noch keine Regeln in der Datei." } else { "Keine Regel passt zur Suche." }}
                                </p>
                            </Show>
                        </div>
                        <p class="text-xs text-muted-foreground">
                            {move || format!("{} Regel(n). {} Kommentar-/Leerzeile(n) bleiben unverändert erhalten.", rule_count.get(), text_count.get())}
                        </p>
                    </div>

                    <div class="min-w-0 flex-1">
                        {move || match selected_kind.get() {
                            None => view! {
                                <div class="flex h-40 items-center justify-center rounded-lg border border-dashed border-border text-sm text-muted-foreground">
                                    "Wähle links eine Regel aus oder lege eine neue an."
                                </div>
                            }.into_any(),
                            Some((li, LineKind::Rule)) => rule_detail(li).into_any(),
                            Some((li, LineKind::Invalid)) => invalid_detail(li).into_any(),
                        }}
                    </div>
                </div>
            </Show>

            <Show when=move || picker.get().is_some()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">
                                {move || match picker.get() {
                                    Some(PickTarget::NewRule) => "Item für die neue Regel wählen".to_string(),
                                    Some(PickTarget::Rank(_, r)) => format!("Item für {} wählen", RANKS[r].key),
                                    _ => "Item wählen".to_string(),
                                }}
                            </p>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| picker.set(None)>
                                "Schließen"
                            </Button>
                        </div>
                        <EntityBrowser kind=EntityKind::Item auto_focus=true on_pick=on_pick/>
                    </div>
                </div>
            </Show>

            {move || delete_confirm.get().map(|li| view! {
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-80 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm">"Diese Zeile wirklich entfernen? Wird erst beim Speichern in die Datei übernommen."</p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(None)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| delete_line(li)>"Entfernen"</Button>
                        </div>
                    </div>
                </div>
            })}

            <Show when=move || confirm_reload.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm">"Ungespeicherte Änderungen verwerfen und die Datei neu vom Server laden?"</p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_reload.set(false)>"Abbrechen"</Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| load()>"Verwerfen und laden"</Button>
                        </div>
                    </div>
                </div>
            </Show>

            <Show when=move || confirm_save.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="w-[26rem] space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-amber-500"/>
                            <span>
                                "common_drop_item.txt wird auf dem Server überschrieben (vorher wird ein Backup angelegt). Die Änderungen wirken erst nach einem Server-Neustart."
                            </span>
                        </p>
                        <Show when=move || warning_count.get() != 0>
                            <p class="text-xs text-amber-600 dark:text-amber-400">
                                {move || format!("Es gibt noch {} Hinweis(e) - sie blockieren das Speichern nicht.", warning_count.get())}
                            </p>
                        </Show>
                        <Show when=move || db_error.get().is_some()>
                            <p class="text-xs text-amber-600 dark:text-amber-400">
                                "Achtung: die Items wurden nicht gegen item_proto geprüft (keine Datenbank-Verbindung)."
                            </p>
                        </Show>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_save.set(false)>"Abbrechen"</Button>
                            <Button on_click=move |_| save()>"Speichern"</Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
