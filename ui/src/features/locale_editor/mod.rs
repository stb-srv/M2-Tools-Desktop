//! Locale-String-Verwaltung: Namespaces aus `share/translate.lua`
//! bearbeiten (Portierung von `src/features/locale-editor`).

pub mod logic;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::lib::log_activity;
use crate::store::navigation::{nav, report_section_dirty, Section};
use crate::tauri::{invoke, invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use logic::{filter_namespaces, namespace_of_orphan, to_display, to_stored};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct LocaleEntry {
    key: String,
    value: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct LocaleReference {
    namespace: String,
    key: String,
    file: String,
    line: i64,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct LocaleCompletenessReport {
    missing: Vec<LocaleReference>,
    orphaned: Vec<String>,
}

#[derive(Clone, Copy)]
struct State {
    namespaces: RwSignal<Option<Vec<String>>>,
    ns_loading: RwSignal<bool>,
    ns_error: RwSignal<Option<String>>,
    selected: RwSignal<Option<String>>,
    entries: RwSignal<Option<Vec<LocaleEntry>>>,
    entries_loading: RwSignal<bool>,
    entries_error: RwSignal<Option<String>>,
    dirty: RwSignal<bool>,
    saving: RwSignal<bool>,
    save_ok: RwSignal<Option<String>>,
    save_error: RwSignal<Option<String>>,
    save_confirm: RwSignal<bool>,
    new_key: RwSignal<String>,
    creating: RwSignal<bool>,
    new_ns_name: RwSignal<String>,
    new_ns_preview: RwSignal<String>,
    create_error: RwSignal<Option<String>>,
    creating_busy: RwSignal<bool>,
    completeness_open: RwSignal<bool>,
    completeness_loading: RwSignal<bool>,
    completeness_error: RwSignal<Option<String>>,
    completeness: RwSignal<Option<LocaleCompletenessReport>>,
}

impl State {
    async fn load_namespaces(self) {
        self.ns_loading.set(true);
        self.ns_error.set(None);
        match invoke0::<Vec<String>>("list_locale_namespaces").await {
            Ok(n) => self.namespaces.set(Some(n)),
            Err(e) => self.ns_error.set(Some(e)),
        }
        self.ns_loading.set(false);
    }

    async fn open_namespace(self, name: String) {
        self.selected.set(Some(name.clone()));
        self.entries.set(None);
        self.entries_error.set(None);
        self.dirty.set(false);
        self.save_ok.set(None);
        self.save_error.set(None);
        self.entries_loading.set(true);
        match invoke::<Vec<LocaleEntry>>("read_locale_namespace", &json!({ "namespace": name })).await {
            Ok(e) => self.entries.set(Some(e)),
            Err(e) => self.entries_error.set(Some(e)),
        }
        self.entries_loading.set(false);
    }

    fn update_value(self, index: usize, value: String) {
        self.entries.update(|e| {
            if let Some(entry) = e.as_mut().and_then(|v| v.get_mut(index)) {
                entry.value = value;
            }
        });
        self.dirty.set(true);
        self.save_ok.set(None);
    }

    fn remove_entry(self, index: usize) {
        self.entries.update(|e| {
            if let Some(v) = e {
                if index < v.len() {
                    v.remove(index);
                }
            }
        });
        self.dirty.set(true);
        self.save_ok.set(None);
    }

    fn add_entry(self) {
        let key = self.new_key.get_untracked().trim().to_string();
        if key.is_empty() {
            return;
        }
        self.entries.update(|e| e.get_or_insert_with(Vec::new).push(LocaleEntry { key, value: String::new() }));
        self.new_key.set(String::new());
        self.dirty.set(true);
        self.save_ok.set(None);
    }

    async fn save_namespace(self) {
        let (Some(selected), Some(entries)) = (self.selected.get_untracked(), self.entries.get_untracked()) else {
            return;
        };
        self.save_confirm.set(false);
        self.saving.set(true);
        self.save_error.set(None);
        match invoke::<Option<String>>("write_locale_namespace", &json!({ "namespace": selected, "entries": entries }))
            .await
        {
            Ok(backup) => {
                self.save_ok.set(Some(match backup {
                    Some(b) if !b.is_empty() => format!("Gespeichert. Backup: {b}"),
                    _ => "Gespeichert.".to_string(),
                }));
                self.dirty.set(false);
                log_activity(
                    Section::LocaleEditor.into(),
                    "update",
                    &format!("Locale-Namespace '{selected}' gespeichert"),
                    Some("locale-namespace"),
                    Some(&selected),
                );
            }
            Err(e) => self.save_error.set(Some(e)),
        }
        self.saving.set(false);
    }

    async fn submit_create(self) {
        let preview = self.new_ns_preview.get_untracked();
        if preview.is_empty() {
            return;
        }
        self.creating_busy.set(true);
        self.create_error.set(None);
        match invoke_unit("create_locale_namespace", &json!({ "namespace": preview })).await {
            Ok(()) => {
                log_activity(
                    Section::LocaleEditor.into(),
                    "create",
                    &format!("Locale-Namespace '{preview}' angelegt"),
                    Some("locale-namespace"),
                    Some(&preview),
                );
                self.creating.set(false);
                self.new_ns_name.set(String::new());
                self.load_namespaces().await;
                self.open_namespace(preview).await;
            }
            Err(e) => self.create_error.set(Some(e)),
        }
        self.creating_busy.set(false);
    }

    async fn run_completeness_check(self) {
        self.completeness_open.set(true);
        self.completeness_loading.set(true);
        self.completeness_error.set(None);
        match invoke0::<LocaleCompletenessReport>("check_locale_completeness").await {
            Ok(r) => self.completeness.set(Some(r)),
            Err(e) => self.completeness_error.set(Some(e)),
        }
        self.completeness_loading.set(false);
    }
}

#[component]
fn Modal(#[prop(into)] on_close: Callback<()>, children: Children) -> impl IntoView {
    view! {
        <div class="fixed inset-0 flex items-center justify-center bg-black/50">
            <div class="w-96 rounded-lg border border-border bg-card p-4">
                <div class="mb-2 flex justify-end">
                    <button on:click=move |_| on_close.run(()) class="text-muted-foreground hover:text-foreground">
                        <Icon name="X" class="size-4"/>
                    </button>
                </div>
                {children()}
            </div>
        </div>
    }
}

#[component]
pub fn LocaleEditor() -> impl IntoView {
    let s = State {
        namespaces: RwSignal::new(None),
        ns_loading: RwSignal::new(false),
        ns_error: RwSignal::new(None),
        selected: RwSignal::new(None),
        entries: RwSignal::new(None),
        entries_loading: RwSignal::new(false),
        entries_error: RwSignal::new(None),
        dirty: RwSignal::new(false),
        saving: RwSignal::new(false),
        save_ok: RwSignal::new(None),
        save_error: RwSignal::new(None),
        save_confirm: RwSignal::new(false),
        new_key: RwSignal::new(String::new()),
        creating: RwSignal::new(false),
        new_ns_name: RwSignal::new(String::new()),
        new_ns_preview: RwSignal::new(String::new()),
        create_error: RwSignal::new(None),
        creating_busy: RwSignal::new(false),
        completeness_open: RwSignal::new(false),
        completeness_loading: RwSignal::new(false),
        completeness_error: RwSignal::new(None),
        completeness: RwSignal::new(None),
    };
    let search = RwSignal::new(String::new());

    Effect::new(move |_| report_section_dirty(Section::LocaleEditor, s.dirty.get()));
    on_cleanup(|| report_section_dirty(Section::LocaleEditor, false));

    spawn_local(s.load_namespaces());

    Effect::new(move |_| {
        let name = s.new_ns_name.get();
        let creating = s.creating.get();
        if !creating || name.trim().is_empty() {
            s.new_ns_preview.set(String::new());
            return;
        }
        spawn_local(async move {
            match invoke::<String>("sanitize_locale_namespace", &json!({ "name": name })).await {
                Ok(p) => s.new_ns_preview.set(p),
                Err(_) => s.new_ns_preview.set(String::new()),
            }
        });
    });

    let filtered = Memo::new(move |_| {
        let q = search.get();
        s.namespaces.with(|n| filter_namespaces(n.as_deref().unwrap_or(&[]), &q))
    });
    let entry_count = Memo::new(move |_| s.entries.with(|e| e.as_ref().map(|v| v.len())));

    let header_actions: ChildrenFn = Arc::new(move || {
        view! {
            <Button
                variant=ButtonVariant::Outline
                size=ButtonSize::Sm
                class="gap-1.5"
                on_click=move |_| spawn_local(s.run_completeness_check())
            >
                <Icon name="ListChecks" class="size-3.5"/>
                "Vollständigkeit prüfen"
            </Button>
        }
        .into_any()
    });

    view! {
        <div class="flex h-full flex-col gap-3">
            <div>
                <PageHeader module=Section::LocaleEditor title="Locale-String-Verwaltung" actions=header_actions/>
                <p class="max-w-4xl text-sm text-muted-foreground">
                    "Bearbeitet "
                    <code>"share/translate.lua"</code>
                    " - die Textbausteine, die Quests über "
                    <code>"gameforge.questname._key"</code>
                    " referenzieren. Speichern ändert nur den ausgewählten Namespace, der Rest der ~8800-Zeilen-Datei bleibt unangetastet."
                </p>
            </div>

            <div class="flex min-h-0 flex-1 gap-4">
                <div class="flex w-72 shrink-0 flex-col gap-2">
                    <div class="flex gap-2">
                        <div class="relative flex-1">
                            <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                            <input
                                prop:value=move || search.get()
                                on:input=move |e| search.set(event_target_value(&e))
                                placeholder="Namespace suchen…"
                                class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                            />
                        </div>
                        <Button variant=ButtonVariant::Outline size=ButtonSize::Icon on_click=move |_| s.creating.set(true)>
                            <Icon name="Plus" class="size-4"/>
                        </Button>
                        <Button
                            variant=ButtonVariant::Outline
                            size=ButtonSize::Icon
                            on_click=move |_| spawn_local(s.load_namespaces())
                            disabled=s.ns_loading
                        >
                            <Icon
                                name="RefreshCw"
                                class=Signal::derive(move || format!("size-4 {}", if s.ns_loading.get() { "animate-spin" } else { "" }))
                            />
                        </Button>
                    </div>
                    {move || s.ns_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                    <div class="flex-1 space-y-1 overflow-y-auto">
                        <For each=move || filtered.get() key=|n| n.clone() let:name>
                            {
                                let label = format!("gameforge.{name}");
                                let n_cls = name.clone();
                                view! {
                                    <button
                                        on:click=move |_| spawn_local(s.open_namespace(name.clone()))
                                        class=move || format!(
                                            "block w-full truncate rounded-md px-2 py-1 text-left text-sm hover:bg-muted {}",
                                            if s.selected.with(|sel| sel.as_deref() == Some(n_cls.as_str())) { "bg-muted" } else { "" },
                                        )
                                    >
                                        {label}
                                    </button>
                                }
                            }
                        </For>
                        <Show when=move || s.namespaces.with(|n| n.is_some()) && filtered.with(|f| f.is_empty())>
                            <p class="p-2 text-sm text-muted-foreground">"Keine Namespaces gefunden."</p>
                        </Show>
                    </div>
                </div>

                <div class="flex min-w-0 flex-1 flex-col gap-2">
                    <Show
                        when=move || s.selected.with(|x| x.is_some())
                        fallback=|| view! {
                            <div class="flex flex-1 flex-col items-center justify-center rounded-lg border border-dashed border-border">
                                <p class="text-sm text-muted-foreground">
                                    "Wähle links einen Namespace aus oder lege über „+\" einen neuen an."
                                </p>
                            </div>
                        }
                    >
                        <div class="flex items-center justify-between">
                            <code class="text-sm">"gameforge." {move || s.selected.get().unwrap_or_default()}</code>
                            <div class="flex items-center gap-2">
                                {move || s.save_ok.get().map(|m| view! {
                                    <span class="flex items-center gap-1 text-sm text-green-600">
                                        <Icon name="CheckCircle2" class="size-4"/>
                                        " "
                                        {m}
                                    </span>
                                })}
                                <Button
                                    size=ButtonSize::Sm
                                    on_click=move |_| s.save_confirm.set(true)
                                    disabled=Signal::derive(move || s.saving.get() || !s.dirty.get())
                                >
                                    {move || if s.saving.get() { "Speichere…" } else { "Speichern" }}
                                </Button>
                            </div>
                        </div>
                        {move || s.save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        {move || s.entries_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <Show when=move || s.entries_loading.get()>
                            <p class="text-sm text-muted-foreground">"Lade…"</p>
                        </Show>

                        <Show when=move || entry_count.get().is_some()>
                            <div class="flex-1 space-y-2 overflow-y-auto">
                                <For each=move || 0..entry_count.get().unwrap_or(0) key=|i| *i let:index>
                                    {
                                        let entry = Memo::new(move |_| {
                                            s.entries.with(|e| e.as_ref().and_then(|v| v.get(index).cloned()))
                                        });
                                        let value = move || entry.with(|e| e.as_ref().map(|e| e.value.clone()).unwrap_or_default());
                                        view! {
                                            <div class="space-y-1 rounded-md border border-border p-2">
                                                <div class="flex items-center justify-between">
                                                    <code class="text-xs text-muted-foreground">
                                                        "gameforge."
                                                        {move || s.selected.get().unwrap_or_default()}
                                                        "."
                                                        {move || entry.with(|e| e.as_ref().map(|e| e.key.clone()).unwrap_or_default())}
                                                    </code>
                                                    <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| s.remove_entry(index)>
                                                        <Icon name="Trash2" class="size-3.5"/>
                                                    </Button>
                                                </div>
                                                <textarea
                                                    prop:value=move || to_display(&value())
                                                    on:input=move |e| s.update_value(index, to_stored(&event_target_value(&e)))
                                                    rows=move || { if value().encode_utf16().count() > 60 { 3 } else { 1 } }
                                                    class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                                                ></textarea>
                                            </div>
                                        }
                                    }
                                </For>
                                <Show when=move || entry_count.get() == Some(0)>
                                    <p class="p-2 text-sm text-muted-foreground">"Noch keine Einträge."</p>
                                </Show>

                                <div class="flex gap-2 rounded-md border border-dashed border-border p-2">
                                    <input
                                        prop:value=move || s.new_key.get()
                                        on:input=move |e| s.new_key.set(event_target_value(&e))
                                        on:keydown=move |e| {
                                            if e.key() == "Enter" {
                                                s.add_entry();
                                            }
                                        }
                                        placeholder="Neuer Key (z.B. _010_npcChat)"
                                        class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                                    />
                                    <Button
                                        variant=ButtonVariant::Outline
                                        on_click=move |_| s.add_entry()
                                        disabled=Signal::derive(move || s.new_key.with(|k| k.trim().is_empty()))
                                    >
                                        <Icon name="Plus" class="size-4"/>
                                        "Key hinzufügen"
                                    </Button>
                                </div>
                            </div>
                        </Show>
                    </Show>
                </div>
            </div>

            <Show when=move || s.creating.get()>
                <Modal on_close=move |_| s.creating.set(false)>
                    <div class="space-y-3">
                        <p class="text-sm font-medium">"Neuen Namespace anlegen"</p>
                        <input
                            autofocus
                            prop:value=move || s.new_ns_name.get()
                            on:input=move |e| s.new_ns_name.set(event_target_value(&e))
                            placeholder="z.B. meine_quest"
                            class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                        <Show when=move || !s.new_ns_name.with(|n| n.trim().is_empty())>
                            <p class="text-xs text-muted-foreground">
                                "Wird gespeichert als: "
                                <code>
                                    "gameforge."
                                    {move || {
                                        let p = s.new_ns_preview.get();
                                        if p.is_empty() { "…".to_string() } else { p }
                                    }}
                                </code>
                            </p>
                        </Show>
                        {move || s.create_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        <Button
                            on_click=move |_| spawn_local(s.submit_create())
                            disabled=Signal::derive(move || s.new_ns_preview.with(|p| p.is_empty()) || s.creating_busy.get())
                        >
                            {move || if s.creating_busy.get() { "Lege an…" } else { "Anlegen" }}
                        </Button>
                    </div>
                </Modal>
            </Show>

            <Show when=move || s.completeness_open.get()>
                <Modal on_close=move |_| s.completeness_open.set(false)>
                    <div class="max-h-[70vh] w-[32rem] max-w-full space-y-4 overflow-y-auto">
                        <p class="text-sm font-medium">"Vollständigkeit von translate.lua"</p>
                        <Show when=move || s.completeness_loading.get()>
                            <p class="text-sm text-muted-foreground">
                                "Lese alle Quest-Dateien und translate.lua - kann bei vielen Quests einen Moment dauern…"
                            </p>
                        </Show>
                        {move || s.completeness_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        {move || s.completeness.get().map(|report| completeness_view(s, report))}
                    </div>
                </Modal>
            </Show>

            <Show when=move || s.save_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                <code>"translate.lua"</code>
                                " wird gesichert, danach werden nur die Zeilen von "
                                <code>"gameforge." {move || s.selected.get().unwrap_or_default()}</code>
                                " überschrieben."
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| s.save_confirm.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button on_click=move |_| spawn_local(s.save_namespace())>"Speichern"</Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}

fn completeness_view(s: State, report: LocaleCompletenessReport) -> impl IntoView {
    let missing_len = report.missing.len();
    let orphaned_len = report.orphaned.len();
    let missing = if report.missing.is_empty() {
        view! { <p class="text-xs text-muted-foreground">"Keine gefunden."</p> }.into_any()
    } else {
        view! {
            <div class="space-y-1">
                {report
                    .missing
                    .into_iter()
                    .map(|r| {
                        let title = format!("{}:{}", r.file, r.line);
                        let file = r.file.clone();
                        view! {
                            <button
                                on:click=move |_| {
                                    nav().go_to_with_selection(Section::QuestBuilder, file.clone());
                                    s.completeness_open.set(false);
                                }
                                class="block w-full truncate rounded-md border border-border px-2 py-1 text-left text-xs hover:bg-muted"
                                title=title
                            >
                                <code>{format!("gameforge.{}.{}", r.namespace, r.key)}</code>
                                <span class="text-muted-foreground">{format!(" — {}:{}", r.file, r.line)}</span>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        }
        .into_any()
    };
    let orphaned = if report.orphaned.is_empty() {
        view! { <p class="text-xs text-muted-foreground">"Keine gefunden."</p> }.into_any()
    } else {
        view! {
            <div class="space-y-1">
                {report
                    .orphaned
                    .into_iter()
                    .map(|entry| {
                        let ns = namespace_of_orphan(&entry);
                        view! {
                            <button
                                on:click=move |_| {
                                    s.completeness_open.set(false);
                                    spawn_local(s.open_namespace(ns.clone()));
                                }
                                class="block w-full truncate rounded-md border border-border px-2 py-1 text-left text-xs hover:bg-muted"
                            >
                                <code>{format!("gameforge.{entry}")}</code>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
        }
        .into_any()
    };
    view! {
        <div class="space-y-1">
            <p class="flex items-center gap-1.5 text-sm font-medium">
                <Icon name="AlertTriangle" class="size-3.5 text-destructive"/>
                {format!("Fehlende Keys ({missing_len})")}
            </p>
            <p class="text-xs text-muted-foreground">
                "Von einer Quest referenziert, aber in translate.lua nicht definiert - zeigt im Client vermutlich einen leeren/Fallback-Text."
            </p>
            {missing}
        </div>

        <div class="space-y-1">
            <p class="text-sm font-medium">{format!("Verwaiste Keys ({orphaned_len})")}</p>
            <p class="text-xs text-muted-foreground">
                "In translate.lua definiert, aber von keiner gescannten Quest-Datei referenziert - nicht zwingend falsch (könnte anderswo z.B. clientseitig genutzt werden), aber ein Aufräum-Kandidat."
            </p>
            {orphaned}
        </div>
    }
}
