//! Änderungsprotokoll: gemeinsame Zeitleiste aller Änderungen über die App.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::i18n::t;
use crate::lib::format::{de_datetime, de_int};
use crate::store::navigation::{nav, EmbeddedModule, Section};
use crate::tauri::{invoke, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct UndoRef {
    kind: String,
    id: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct ActivityEntry {
    id: String,
    created_at: String,
    module: String,
    #[allow(dead_code)]
    action: String,
    summary: String,
    #[allow(dead_code)]
    source: String,
    undo: Option<UndoRef>,
}

#[derive(Clone, Debug, Deserialize)]
struct ActivityFeedPage {
    entries: Vec<ActivityEntry>,
    total: i64,
}

const PAGE_SIZE: i64 = 30;

/// Module, über die dieses Modul selbst nichts protokolliert (keine echten
/// Änderungsquellen) - aus der Filterliste ausgeschlossen.
const NON_ACTIVITY_SECTIONS: [Section; 3] = [Section::Dashboard, Section::Settings, Section::ActivityLog];

/// (Kennung, Übersetzungsschlüssel) aller Module: Sidebar-Bereiche, dann eingebettete Module.
fn all_modules() -> Vec<(&'static str, &'static str)> {
    Section::ALL
        .iter()
        .map(|s| (s.id(), s.label_key()))
        .chain(EmbeddedModule::ALL.iter().map(|e| (e.id(), e.label_key())))
        .collect()
}

fn module_label(module: &str) -> String {
    all_modules().into_iter().find(|(id, _)| *id == module).map(|(_, key)| t(key)).unwrap_or_else(|| module.to_string())
}

#[component]
pub fn ActivityLog() -> impl IntoView {
    let module_filter = RwSignal::new(String::new());
    let search = RwSignal::new(String::new());
    let page = RwSignal::new(0i64);
    let refresh_key = RwSignal::new(0u32);

    let entries = RwSignal::new(Vec::<ActivityEntry>::new());
    let total = RwSignal::new(0i64);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let undoing_id = RwSignal::new(None::<String>);
    let confirm_undo = RwSignal::new(None::<ActivityEntry>);
    let undo_error = RwSignal::new(None::<String>);

    Effect::new(move |_| {
        let m = module_filter.get();
        let p = page.get();
        refresh_key.track();
        let s = search.get_untracked().trim().to_string();
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<ActivityFeedPage>(
                "list_activity_feed",
                &json!({
                    "module": if m.is_empty() { None } else { Some(m) },
                    "search": if s.is_empty() { None } else { Some(s) },
                    "offset": p * PAGE_SIZE,
                    "limit": PAGE_SIZE,
                }),
            )
            .await
            {
                Ok(r) => {
                    entries.set(r.entries);
                    total.set(r.total);
                }
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    });

    let run_search = move || {
        page.set(0);
        refresh_key.update(|k| *k += 1);
    };

    let confirm_import_undo = move || {
        let Some(entry) = confirm_undo.get_untracked() else { return };
        let Some(undo) = entry.undo.clone() else { return };
        confirm_undo.set(None);
        undoing_id.set(Some(entry.id.clone()));
        undo_error.set(None);
        spawn_local(async move {
            match invoke_unit("undo_import_batch", &json!({ "id": undo.id })).await {
                Ok(()) => refresh_key.update(|k| *k += 1),
                Err(e) => undo_error.set(Some(e)),
            }
            undoing_id.set(None);
        });
    };

    let total_pages = move || ((total.get() + PAGE_SIZE - 1) / PAGE_SIZE).max(1);

    let filter_options = move || {
        Section::ALL
            .iter()
            .filter(|s| !NON_ACTIVITY_SECTIONS.contains(s))
            .map(|s| (s.id(), s.label_key()))
            .chain(EmbeddedModule::ALL.iter().map(|e| (e.id(), e.label_key())))
            .map(|(id, key)| view! { <option value=id selected=move || module_filter.get() == id>{move || t(key)}</option> })
            .collect_view()
    };

    view! {
        <div class="flex h-full flex-col gap-3">
            <div>
                <PageHeader module=Section::ActivityLog title=Signal::derive(|| t("nav.activityLog"))/>
                <p class="max-w-4xl text-sm text-muted-foreground">
                    "Gemeinsame Zeitleiste aller Änderungen über die App - Item-/Shop-/Mob-Drop-/Quest-Edits, Account-/GM-Aktionen, Server-Events und mehr, zusammen mit den bereits bestehenden Verläufen für „Bauen & Einspielen\" und den Modul-Importer."
                </p>
            </div>

            <div class="flex flex-wrap items-center gap-2">
                <select
                    on:change=move |e| {
                        module_filter.set(event_target_value(&e));
                        page.set(0);
                    }
                    class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                >
                    <option value="" selected=move || module_filter.get().is_empty()>"Alle Module"</option>
                    {filter_options}
                </select>
                <input
                    prop:value=move || search.get()
                    on:input=move |e| search.set(event_target_value(&e))
                    on:keydown=move |e| {
                        if e.key() == "Enter" {
                            run_search()
                        }
                    }
                    placeholder="Volltextsuche über die Zusammenfassung…"
                    class="flex-1 min-w-48 rounded-md border border-border bg-background px-2 py-1 text-sm"
                />
                <Button variant=ButtonVariant::Outline on_click=move |_| run_search() disabled=loading>
                    <Icon name="Search" class="size-4"/>
                </Button>
            </div>

            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
            {move || undo_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <div class="flex-1 space-y-1 overflow-y-auto rounded-md border border-border p-1">
                <Show when=move || loading.get() && entries.with(|e| e.is_empty())>
                    <p class="p-2 text-sm text-muted-foreground">"Lade…"</p>
                </Show>
                <Show when=move || !loading.get() && entries.with(|e| e.is_empty()) && error.with(|e| e.is_none())>
                    <p class="p-2 text-sm text-muted-foreground">"Keine Einträge."</p>
                </Show>
                <For each=move || entries.get() key=|e| e.id.clone() let:entry>
                    {
                        let kind = entry.undo.as_ref().map(|u| u.kind.clone()).unwrap_or_default();
                        let id1 = entry.id.clone();
                        let id2 = entry.id.clone();
                        let e2 = entry.clone();
                        let module = entry.module.clone();
                        view! {
                            <div class="flex flex-wrap items-center justify-between gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted">
                                <div class="flex min-w-0 flex-1 items-center gap-2">
                                    <span class="shrink-0 text-xs text-muted-foreground">{de_datetime(&entry.created_at)}</span>
                                    <span class="shrink-0 rounded-full border border-border px-1.5 py-0.5 text-xs text-muted-foreground">
                                        {move || module_label(&module)}
                                    </span>
                                    <span class="truncate">{entry.summary.clone()}</span>
                                </div>
                                {(kind == "undo_import").then(|| view! {
                                    <Button
                                        size=ButtonSize::Sm
                                        variant=ButtonVariant::Outline
                                        disabled=Signal::derive(move || undoing_id.with(|u| u.as_deref() == Some(id1.as_str())))
                                        on_click=move |_| confirm_undo.set(Some(e2.clone()))
                                    >
                                        <Icon name="Undo2" class="size-3.5"/>
                                        {move || if undoing_id.with(|u| u.as_deref() == Some(id2.as_str())) { "Entferne…" } else { "Rückgängig machen" }}
                                    </Button>
                                })}
                                {(kind == "rollback").then(|| {
                                    view! {
                                        <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| nav().set_section(Section::BuildDeploy)>
                                            <Icon name="ExternalLink" class="size-3.5"/>
                                            "In Bauen & Einspielen öffnen"
                                        </Button>
                                    }
                                })}
                            </div>
                        }
                    }
                </For>
            </div>

            <Show when=move || { total.get() > 0 }>
                <div class="flex items-center justify-between text-xs text-muted-foreground">
                    <span>
                        {move || de_int(total.get())}
                        " Treffer"
                        {move || (total_pages() > 1).then(|| format!(" · Seite {}/{}", page.get() + 1, total_pages()))}
                    </span>
                    <Show when=move || { total_pages() > 1 }>
                        <div class="flex gap-1">
                            <Button
                                variant=ButtonVariant::Outline
                                size=ButtonSize::IconSm
                                on_click=move |_| page.update(|p| *p = (*p - 1).max(0))
                                disabled=Signal::derive(move || page.get() == 0 || loading.get())
                            >
                                <Icon name="ChevronLeft" class="size-3.5"/>
                            </Button>
                            <Button
                                variant=ButtonVariant::Outline
                                size=ButtonSize::IconSm
                                on_click=move |_| page.update(|p| *p = (*p + 1).min(total_pages() - 1))
                                disabled=Signal::derive(move || page.get() >= total_pages() - 1 || loading.get())
                            >
                                <Icon name="ChevronRight" class="size-3.5"/>
                            </Button>
                        </div>
                    </Show>
                </div>
            </Show>

            {move || confirm_undo.get().map(|entry| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>{format!("{} - endgültig rückgängig machen?", entry.summary)}</span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| confirm_undo.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| confirm_import_undo()>
                                "Endgültig entfernen"
                            </Button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_ids_known() {
        let ids: Vec<&str> = all_modules().iter().map(|(id, _)| *id).collect();
        assert!(ids.contains(&"build-deploy"));
        assert!(ids.contains(&"db-backups"));
    }
}
