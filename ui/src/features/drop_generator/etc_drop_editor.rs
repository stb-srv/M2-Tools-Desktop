//! etc_drop_item.txt - Item→Prozent-Tabelle, mob-unabhängig.

use super::join::join_all;
use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::mob_drop_editor::components::shared::{js_number, num_sig, NumInput};
use crate::features::shared::drop_chance::format_real_drop_chance;
use crate::features::shared::{ChangePreview, EntityBrowser, EntityKind, EntityRow, RestartServerButton};
use crate::util::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, Section};
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EtcDropEntry {
    pub item_name: String,
    pub percent: f64,
}

#[derive(Deserialize)]
struct ItemBrief {
    vnum: i64,
    locale_name: String,
}

pub fn can_save(entries: Option<&[EtcDropEntry]>) -> bool {
    entries.is_some_and(|e| e.iter().all(|e| !e.item_name.trim().is_empty() && e.percent > 0.0))
}

#[component]
pub fn EtcDropEditor() -> impl IntoView {
    let entries = RwSignal::new(None::<Vec<EtcDropEntry>>);
    // Fehlt ein Schlüssel = noch nicht aufgelöst, `None` = nicht gefunden.
    let labels = RwSignal::new(HashMap::<String, Option<String>>::new());
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let save_error = RwSignal::new(None::<String>);
    let save_ok = RwSignal::new(false);
    let confirm_open = RwSignal::new(false);
    let picker_for = RwSignal::new(None::<usize>);

    let loaded_snapshot = RwSignal::new(None::<Vec<EtcDropEntry>>);
    let dirty = Memo::new(move |_| entries.with(|e| e.is_some() && loaded_snapshot.with(|s| s != e)));
    Effect::new(move |_| report_section_dirty(Section::DropGenerator, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::DropGenerator, false));

    let resolve_labels = move |list: Vec<EtcDropEntry>| async move {
        let resolved = join_all(list.into_iter().map(|e| async move {
            let label = match invoke::<Option<ItemBrief>>("find_item_by_internal_name", &json!({ "name": e.item_name })).await {
                Ok(Some(b)) => Some(format!("{} ({})", b.locale_name, b.vnum)),
                _ => None,
            };
            (e.item_name, label)
        }))
        .await;
        labels.set(resolved.into_iter().collect());
    };

    let load = move || {
        loading.set(true);
        load_error.set(None);
        save_ok.set(false);
        spawn_local(async move {
            match invoke::<Vec<EtcDropEntry>>("read_etc_drop_file", &json!({})).await {
                Ok(list) => {
                    entries.set(Some(list.clone()));
                    loaded_snapshot.set(Some(list.clone()));
                    resolve_labels(list).await;
                }
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    let update_percent = move |idx: usize, p: f64| {
        if entries.with_untracked(|e| e.is_none()) {
            return;
        }
        entries.update(|e| {
            if let Some(e) = e.as_mut().and_then(|e| e.get_mut(idx)) {
                e.percent = p;
            }
        });
        save_ok.set(false);
    };

    let remove_entry = move |idx: usize| {
        entries.update(|e| {
            if let Some(e) = e.as_mut() {
                if idx < e.len() {
                    e.remove(idx);
                }
            }
        })
    };

    let can_save_m = Memo::new(move |_| entries.with(|e| can_save(e.as_deref())));

    let save = move || {
        let Some(es) = entries.get_untracked() else { return };
        confirm_open.set(false);
        saving.set(true);
        save_error.set(None);
        spawn_local(async move {
            match invoke::<Option<String>>("write_etc_drop_file", &json!({ "entries": es })).await {
                Ok(_) => {
                    save_ok.set(true);
                    let n = es.len();
                    loaded_snapshot.set(Some(es));
                    log_activity(
                        Section::DropGenerator.into(),
                        "save",
                        &format!("etc_drop_item.txt gespeichert ({n} Eintrag/Einträge)"),
                        Some("etc_drop_item.txt"),
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

    let on_pick = move |row: EntityRow| {
        let Some(idx) = picker_for.get_untracked() else { return };
        picker_for.set(None);
        spawn_local(async move {
            match invoke::<String>("get_item_internal_name", &json!({ "vnum": row.vnum })).await {
                Ok(internal_name) => {
                    let n2 = internal_name.clone();
                    entries.update(|e| {
                        let next = e.get_or_insert_with(Vec::new);
                        if idx < next.len() {
                            next[idx].item_name = n2;
                        } else {
                            next.push(EtcDropEntry { item_name: n2, percent: 1.0 });
                        }
                    });
                    labels.update(|l| {
                        l.insert(internal_name, Some(format!("{} ({})", row.name, row.vnum)));
                    });
                }
                Err(e) => load_error.set(Some(e)),
            }
        });
    };

    let has_entries = Memo::new(move |_| entries.with(|e| e.is_some()));
    let count = move || entries.with(|e| e.as_ref().map(|e| e.len()).unwrap_or(0));
    let entry = move |i: usize| entries.with(|e| e.as_ref().and_then(|e| e.get(i)).cloned());

    view! {
        <div class="space-y-4 pb-10">
            <div class="rounded-lg border border-border p-3 text-sm text-muted-foreground">
                <p class="mb-1 font-medium text-foreground">"etc_drop_item.txt — Item→Prozent-Tabelle, mob-unabhängig"</p>
                <p>
                    "Diese Datei verknüpft " <strong>"keinen"</strong>
                    " Mob mit einem Item - sie ist nur eine Nachschlagetabelle \"Item X hat Chance Y\". Welcher Mob welchen Eintrag benutzt, legt ein eigenes Feld im jeweiligen Mob-Proto fest (außerhalb dieser Datei, hier nicht editierbar). Beispiel: ein Eintrag \"Kleiner Ring → 0,5\" bedeutet nur, dass IRGENDEIN Mob, der auf diesen Eintrag verweist, den Ring mit dieser Chance droppt."
                </p>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Wirkt erst nach einem kompletten Server-Neustart (kein " <code>"/reload"</code>
                    " dafür). Ein nicht auflösbarer Eintrag lässt den "
                    <strong>"kompletten Server beim nächsten Start nicht mehr hochfahren"</strong>
                    "."
                </span>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-border bg-muted/30 p-3 text-sm text-muted-foreground">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Diese Datei speichert Items über ihren internen Namen, nicht über die VNUM - dieser Editor löst das für dich automatisch auf, wenn du ein Item über die Suche wählst."
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
                <Show when=move || has_entries.get()>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| picker_for.set(Some(count()))>
                        <Icon name="Plus" class="size-3.5"/>
                        "Eintrag hinzufügen"
                    </Button>
                </Show>
            </div>

            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <Show when=move || has_entries.get()>
                <div class="overflow-x-auto rounded-md border border-border">
                    <table class="w-full text-sm">
                        <thead class="bg-muted/40">
                            <tr>
                                <th class="px-2 py-1 text-left font-medium">"Item"</th>
                                <th class="px-2 py-1 text-left font-medium">"Prozent (Datei-Wert)"</th>
                                <th class="px-2 py-1 text-left font-medium">"≈ real"</th>
                                <th></th>
                            </tr>
                        </thead>
                        <tbody>
                            <For each=move || 0..count() key=|i| *i let:idx>
                                <tr class="border-t border-border">
                                    <td class="px-2 py-1">
                                        <button
                                            type="button"
                                            on:click=move |_| picker_for.set(Some(idx))
                                            class="rounded-md border border-border bg-background px-2 py-1 text-left text-xs hover:bg-muted"
                                        >
                                            {move || {
                                                let name = entry(idx).map(|e| e.item_name).unwrap_or_default();
                                                match labels.with(|l| l.get(&name).cloned()) {
                                                    None => name,
                                                    Some(None) => format!("{name} (nicht in item_proto gefunden!)"),
                                                    Some(Some(label)) => label,
                                                }
                                            }}
                                        </button>
                                    </td>
                                    <td class="px-2 py-1">
                                        <NumInput
                                            step="0.000001"
                                            value=num_sig(move || entry(idx).map(|e| e.percent).unwrap_or(0.0))
                                            on_input=move |v: String| update_percent(idx, js_number(&v))
                                            class="w-24 rounded-md border border-border bg-background px-1.5 py-1 text-xs"
                                        />
                                    </td>
                                    <td class="px-2 py-1 text-xs text-muted-foreground">
                                        {move || format_real_drop_chance(entry(idx).map(|e| e.percent).unwrap_or(0.0))}
                                    </td>
                                    <td class="px-2 py-1 text-right">
                                        <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| remove_entry(idx)>
                                            <Icon name="Trash2" class="size-3.5"/>
                                        </Button>
                                    </td>
                                </tr>
                            </For>
                            <Show when=move || count() == 0>
                                <tr>
                                    <td colspan="4" class="px-2 py-3 text-center text-muted-foreground">"Keine Einträge."</td>
                                </tr>
                            </Show>
                        </tbody>
                    </table>
                </div>
            </Show>

            <Show when=move || has_entries.get()>
                {move || save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <Show when=move || save_ok.get()>
                    <div class="flex flex-wrap items-center gap-3">
                        <p class="flex items-center gap-1 text-sm text-green-600">
                            <Icon name="CheckCircle2" class="size-4"/>
                            " Gespeichert. Wirkt nach einem Server-Neustart."
                        </p>
                        <RestartServerButton/>
                    </div>
                </Show>
                <Button disabled=Signal::derive(move || !can_save_m.get() || saving.get()) on_click=move |_| confirm_open.set(true)>
                    {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                </Button>
                <Show when=move || !can_save_m.get()>
                    <p class="text-xs text-destructive">"Jeder Eintrag braucht ein Item und einen Prozentwert > 0."</p>
                </Show>
            </Show>

            <Show when=move || picker_for.with(|p| p.is_some())>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="max-h-[85vh] w-[32rem] space-y-2 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">"Item wählen"</p>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| picker_for.set(None)>
                                "Schließen"
                            </Button>
                        </div>
                        <EntityBrowser kind=EntityKind::Item auto_focus=true on_pick=on_pick/>
                    </div>
                </div>
            </Show>

            <Show when=move || confirm_open.get()>
                <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
                    <div class="max-h-[85vh] w-[40rem] space-y-3 overflow-y-auto rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                "etc_drop_item.txt wird komplett überschrieben (Backup wird vorher angelegt). Wirkt erst nach einem Server-Neustart."
                            </span>
                        </p>
                        {entries.get_untracked().map(|d| view! {
                            <ChangePreview kind="etc_drop" data=serde_json::to_value(&d).unwrap_or_default()/>
                        })}
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

    #[test]
    fn can_save_rules() {
        let e = |n: &str, p: f64| EtcDropEntry { item_name: n.into(), percent: p };
        assert!(!can_save(None));
        assert!(can_save(Some(&[])));
        assert!(can_save(Some(&[e("ring", 0.5)])));
        assert!(!can_save(Some(&[e(" ", 0.5)])));
        assert!(!can_save(Some(&[e("ring", 0.0)])));
    }
}
