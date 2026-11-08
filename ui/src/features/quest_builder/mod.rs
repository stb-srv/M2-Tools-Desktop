//! Quest Builder (aus src/features/quest-builder): Quest-Dateien auf dem
//! Server durchsuchen, im Code-Editor bearbeiten, neu anlegen (Vorlagen),
//! löschen und kompilieren. Dazu das Quest-Wiki als eigenes Fenster.

pub mod apply_types;
pub mod code_editor;
pub mod components;
pub mod js_num;
pub mod lua_highlight;
pub mod quest_description_parser;
pub mod quest_functions;
pub mod quest_templates;
pub mod wiki;

pub use wiki::QuestWiki;

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::util::log_activity;
use crate::store::navigation::{nav, report_section_dirty, Section};
use crate::tauri::{invoke, invoke0, invoke_unit, listen, Unlisten};
use code_editor::{CodeEditor, CodeEditorHandle};
use components::CreateQuestModal;
use leptos::prelude::*;
use leptos::task::spawn_local;
use quest_functions::{filter_functions, quest_functions};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
struct QuestFile {
    category: String,
    name: String,
    #[allow(dead_code)]
    extension: String,
    relative_path: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QuestSearchLine {
    line_number: usize,
    text: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct QuestSearchMatch {
    relative_path: String,
    category: String,
    name: String,
    lines: Vec<QuestSearchLine>,
}

#[derive(Deserialize)]
struct CommandResult {
    #[allow(dead_code)]
    output: String,
    exit_status: Option<i64>,
}

/// Grobe Nachbildung von `localeCompare` für die Kategorie-Sortierung:
/// ohne Groß-/Kleinschreibung, Umlaute wie ihr Grundbuchstabe.
fn locale_key(s: &str) -> (String, String) {
    let folded: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ä' => 'a',
            'ö' => 'o',
            'ü' => 'u',
            'ß' => 's',
            c => c,
        })
        .collect();
    (folded, s.to_string())
}

fn group_by_category(files: &[QuestFile], search: &str) -> Vec<(String, Vec<QuestFile>)> {
    let q = search.to_lowercase();
    let mut groups: Vec<(String, Vec<QuestFile>)> = vec![];
    for f in files {
        if !search.is_empty() && !f.name.to_lowercase().contains(&q) && !f.category.to_lowercase().contains(&q) {
            continue;
        }
        match groups.iter_mut().find(|(c, _)| *c == f.category) {
            Some((_, list)) => list.push(f.clone()),
            None => groups.push((f.category.clone(), vec![f.clone()])),
        }
    }
    groups.sort_by_key(|(a, _)| locale_key(a));
    groups
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PageMode {
    Loading,
    Error,
    Main,
}

#[component]
pub fn QuestBuilder() -> impl IntoView {
    let files = RwSignal::new(None::<Vec<QuestFile>>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);
    let search = RwSignal::new(String::new());
    let content_results = RwSignal::new(None::<Vec<QuestSearchMatch>>);
    let content_searching = RwSignal::new(false);
    let content_search_error = RwSignal::new(None::<String>);
    let pending_jump_line = RwSignal::new(None::<usize>);

    let selected_path = RwSignal::new(None::<String>);
    let content = RwSignal::new(String::new());
    let content_loading = RwSignal::new(false);
    let content_error = RwSignal::new(None::<String>);
    let dirty = RwSignal::new(false);
    Effect::new(move |_| report_section_dirty(Section::QuestBuilder, dirty.get()));
    on_cleanup(|| report_section_dirty(Section::QuestBuilder, false));

    let saving = RwSignal::new(false);
    let save_ok = RwSignal::new(None::<String>);
    let save_error = RwSignal::new(None::<String>);
    let delete_confirm = RwSignal::new(false);
    let creating = RwSignal::new(false);
    let deploying = RwSignal::new(false);
    let deploy_log = RwSignal::new(Vec::<String>::new());
    let deploy_open = RwSignal::new(false);
    let function_ref_open = RwSignal::new(false);
    let function_query = RwSignal::new(String::new());
    let editor = CodeEditorHandle::new();

    let load_files = move || {
        loading.set(true);
        load_error.set(None);
        spawn_local(async move {
            match invoke0::<Vec<QuestFile>>("list_quest_files").await {
                Ok(r) => {
                    files.try_set(Some(r));
                }
                Err(e) => {
                    load_error.try_set(Some(e));
                }
            }
            loading.try_set(false);
        });
    };

    let open_file = move |path: String| {
        selected_path.set(Some(path.clone()));
        content.set(String::new());
        content_error.set(None);
        dirty.set(false);
        save_ok.set(None);
        save_error.set(None);
        content_loading.set(true);
        spawn_local(async move {
            match invoke::<String>("read_quest_file", &json!({ "relativePath": path })).await {
                Ok(text) => {
                    content.try_set(text);
                }
                Err(e) => {
                    content_error.try_set(Some(e));
                }
            }
            content_loading.try_set(false);
        });
    };

    load_files();
    // Globale Suche (Strg+Umschalt+F) springt hierher mit dem relative_path
    // eines Quest-Treffers.
    if let Some(target) = nav().consume_pending_selection(Section::QuestBuilder) {
        open_file(target);
    }

    // Ausgabe des Kompilier-Befehls mitschreiben.
    let unlisten = StoredValue::new_local(None::<Unlisten>);
    spawn_local(async move {
        if let Ok(u) = listen::<String>("server-output", move |line| {
            deploy_log.try_update(|l| l.push(line));
        })
        .await
        {
            if let Some(u) = unlisten.try_set_value(Some(u)).flatten() {
                u.unlisten();
            }
        }
    });
    on_cleanup(move || {
        unlisten.try_with_value(|u| {
            if let Some(u) = u {
                u.unlisten()
            }
        });
    });

    let run_content_search = move || {
        let q = search.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        content_searching.set(true);
        content_search_error.set(None);
        spawn_local(async move {
            match invoke::<Vec<QuestSearchMatch>>("search_quest_files", &json!({ "query": q })).await {
                Ok(r) => {
                    content_results.try_set(Some(r));
                }
                Err(e) => {
                    content_search_error.try_set(Some(e));
                }
            }
            content_searching.try_set(false);
        });
    };

    let open_file_at_line = move |path: String, line: usize| {
        pending_jump_line.set(Some(line));
        open_file(path);
    };

    let save_file = move || {
        let Some(path) = selected_path.get_untracked() else { return };
        saving.set(true);
        save_error.set(None);
        let text = content.get_untracked();
        spawn_local(async move {
            match invoke::<Option<String>>("write_quest_file", &json!({ "relativePath": path, "content": text })).await {
                Ok(backup) => {
                    save_ok.try_set(Some(match backup {
                        Some(b) if !b.is_empty() => format!("Gespeichert. Backup: {b}"),
                        _ => "Gespeichert.".to_string(),
                    }));
                    dirty.try_set(false);
                    log_activity(Section::QuestBuilder.into(), "update", &format!("Quest '{path}' gespeichert"), Some("quest"), Some(&path));
                }
                Err(e) => {
                    save_error.try_set(Some(e));
                }
            }
            saving.try_set(false);
        });
    };

    let confirm_delete = move || {
        let Some(path) = selected_path.get_untracked() else { return };
        delete_confirm.set(false);
        spawn_local(async move {
            match invoke_unit("delete_quest_file", &json!({ "relativePath": path })).await {
                Ok(()) => {
                    log_activity(Section::QuestBuilder.into(), "delete", &format!("Quest '{path}' gelöscht"), Some("quest"), Some(&path));
                    selected_path.try_set(None);
                    content.try_set(String::new());
                    load_files();
                }
                Err(e) => {
                    save_error.try_set(Some(e));
                }
            }
        });
    };

    let insert_function_at_cursor = move |name: &str| {
        if !editor.is_mounted() {
            return;
        }
        // Löst über das input-Ereignis on_change aus (setzt dirty/save_ok).
        editor.insert_at_cursor(name);
        dirty.set(true);
        save_ok.set(None);
    };

    // Springt zu einer per Volltextsuche gefundenen Zeile, sobald die Datei
    // im Editor geladen ist.
    Effect::new(move |_| {
        let Some(line) = pending_jump_line.get() else { return };
        content.track();
        if !editor.track_mounted() {
            return;
        }
        editor.select_line(line, &content.get_untracked());
        pending_jump_line.set(None);
    });

    let filtered_functions = Memo::new(move |_| function_query.with(|q| filter_functions(q)));

    // Eigenes Fenster statt eines Bereichs im Hauptfenster, damit die Wiki
    // parallel zum Skript-Editor offen bleiben kann.
    let open_wiki = move || {
        spawn_local(async move {
            if let Err(e) = crate::tauri::open_window("wiki", "index.html?view=wiki", "Quest-Wiki - M2Manager", 1100.0, 800.0).await {
                toast::error(&e);
            }
        });
    };

    let run_deploy = move || {
        deploy_open.set(true);
        deploying.set(true);
        deploy_log.update(|l| l.push("\n[Kompiliere Quests auf dem Server…]\n".to_string()));
        spawn_local(async move {
            let setting = |key: &'static str| async move {
                invoke::<Option<String>>("get_setting", &json!({ "key": key })).await.ok().flatten().filter(|s| !s.is_empty())
            };
            let workdir = setting("server_workdir").await.unwrap_or_else(|| "/usr/home/game".to_string());
            let command =
                setting("server_cmd_reload_quests").await.unwrap_or_else(|| format!("cd {workdir} && echo 4 | sh index.sh"));
            match invoke::<CommandResult>("run_server_command", &json!({ "command": command })).await {
                Ok(r) => {
                    if let Some(code) = r.exit_status.filter(|c| *c != 0) {
                        deploy_log.try_update(|l| l.push(format!("\n[Beendet mit Code {code}]\n")));
                    }
                }
                Err(e) => {
                    deploy_log.try_update(|l| l.push(format!("\nFehler: {e}\n")));
                }
            }
            deploying.try_set(false);
        });
    };

    let categories = Memo::new(move |_| files.with(|f| search.with(|s| group_by_category(f.as_deref().unwrap_or(&[]), s))));

    let mode = Memo::new(move |_| {
        let has_files = files.with(|f| f.is_some());
        if loading.get() && !has_files {
            PageMode::Loading
        } else if load_error.with(|e| e.is_some()) && !has_files {
            PageMode::Error
        } else {
            PageMode::Main
        }
    });

    // ------------------------------------------------------------ Teil-Ansichten

    let sidebar = move || {
        view! {
            <div class="flex w-72 shrink-0 flex-col gap-2">
                <div class="flex gap-2">
                    <div class="relative flex-1">
                        <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                        <input
                            prop:value=move || search.get()
                            on:input=move |e| {
                                search.set(event_target_value(&e));
                                content_results.set(None);
                            }
                            on:keydown=move |e| {
                                if e.key() == "Enter" {
                                    run_content_search()
                                }
                            }
                            placeholder="Quest suchen (Name) oder Enter für Volltextsuche…"
                            class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                        />
                    </div>
                    <Button
                        variant=ButtonVariant::Outline
                        size=ButtonSize::Icon
                        on_click=move |_| run_content_search()
                        disabled=Signal::derive(move || content_searching.get() || search.with(|s| s.trim().is_empty()))
                        title="Volltextsuche über alle Quest-Dateien"
                    >
                        <Icon name="FileSearch" class="size-4"/>
                    </Button>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Icon on_click=move |_| creating.set(true)>
                        <Icon name="Plus" class="size-4"/>
                    </Button>
                </div>

                <Show when=move || content_searching.get()>
                    <p class="text-xs text-muted-foreground">"Durchsuche Dateiinhalte…"</p>
                </Show>
                {move || content_search_error.get().map(|e| view! { <p class="text-xs text-destructive">{e}</p> })}

                {move || {
                    content_results.get().map(|results| {
                        let count = results.len();
                        let empty = results.is_empty();
                        view! {
                            <div class="space-y-1 rounded-md border border-border p-2">
                                <div class="flex items-center justify-between">
                                    <span class="text-xs font-medium text-muted-foreground">
                                        {count} " Datei(en) mit Treffer für „" {move || search.get()} "\""
                                    </span>
                                    <button on:click=move |_| content_results.set(None) class="text-muted-foreground hover:text-foreground">
                                        <Icon name="X" class="size-3.5"/>
                                    </button>
                                </div>
                                <div class="max-h-56 space-y-2 overflow-y-auto">
                                    {results
                                        .into_iter()
                                        .map(|m| {
                                            let more = m.lines.len().saturating_sub(5);
                                            let lines = m
                                                .lines
                                                .iter()
                                                .take(5)
                                                .map(|l| {
                                                    let path = m.relative_path.clone();
                                                    let n = l.line_number;
                                                    view! {
                                                        <button
                                                            on:click=move |_| open_file_at_line(path.clone(), n)
                                                            class="block w-full truncate rounded px-1.5 py-0.5 text-left font-mono text-[11px] hover:bg-muted"
                                                            title=l.text.clone()
                                                        >
                                                            <span class="text-muted-foreground">{n} ":"</span>
                                                            " "
                                                            {l.text.trim().to_string()}
                                                        </button>
                                                    }
                                                })
                                                .collect_view();
                                            view! {
                                                <div>
                                                    <div class="text-xs font-medium">{format!("{}/{}", m.category, m.name)}</div>
                                                    {lines}
                                                    {(more > 0).then(|| view! {
                                                        <p class="pl-1.5 text-[10px] text-muted-foreground">{format!("+{more} weitere Treffer")}</p>
                                                    })}
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                    {empty.then(|| view! { <p class="text-xs text-muted-foreground">"Keine Treffer im Dateiinhalt."</p> })}
                                </div>
                            </div>
                        }
                    })
                }}

                <div class="flex-1 space-y-3 overflow-y-auto">
                    <For each=move || categories.get() key=|(c, list)| (c.clone(), list.clone()) let:group>
                        {
                            let (category, list) = group;
                            view! {
                                <div class="space-y-1">
                                    <div class="px-1 text-xs font-medium text-muted-foreground">
                                        {if category.is_empty() { "(ohne Kategorie)".to_string() } else { category }}
                                    </div>
                                    {list
                                        .into_iter()
                                        .map(|f| {
                                            let path = f.relative_path.clone();
                                            let p2 = path.clone();
                                            view! {
                                                <div
                                                    on:click=move |_| open_file(path.clone())
                                                    class=move || {
                                                        if selected_path.with(|s| s.as_deref() == Some(p2.as_str())) {
                                                            "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted bg-muted"
                                                        } else {
                                                            "cursor-pointer rounded-md border border-border p-2 text-sm hover:bg-muted "
                                                        }
                                                    }
                                                >
                                                    <div class="font-medium">{f.name.clone()}</div>
                                                    <div class="text-xs text-muted-foreground">{f.relative_path.clone()}</div>
                                                </div>
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                        }
                    </For>
                    <Show when=move || categories.with(|c| c.is_empty())>
                        <p class="p-2 text-sm text-muted-foreground">"Keine Quests gefunden."</p>
                    </Show>
                </div>
            </div>
        }
    };

    let function_panel = move || {
        view! {
            <div class="flex w-64 shrink-0 flex-col gap-2 rounded-md border border-border p-2">
                <div class="relative">
                    <Icon name="Search" class="absolute left-2 top-1/2 size-3.5 -translate-y-1/2 text-muted-foreground"/>
                    <input
                        autofocus=true
                        prop:value=move || function_query.get()
                        on:input=move |e| function_query.set(event_target_value(&e))
                        placeholder="Funktion suchen…"
                        class="w-full rounded-md border border-border bg-background py-1 pl-7 pr-2 text-sm"
                    />
                </div>
                <p class="text-[10px] text-muted-foreground">
                    {quest_functions().len()}
                    " verifizierte Funktionen vom Server. Klick fügt sie an der Cursor-Position ein."
                </p>
                <div class="flex-1 space-y-0.5 overflow-y-auto">
                    {move || {
                        filtered_functions
                            .get()
                            .into_iter()
                            .map(|f| {
                                view! {
                                    <button
                                        on:click=move |_| insert_function_at_cursor(f)
                                        class="block w-full truncate rounded px-1.5 py-0.5 text-left font-mono text-xs hover:bg-muted"
                                        title=f
                                    >
                                        {f}
                                    </button>
                                }
                            })
                            .collect_view()
                    }}
                    <Show when=move || filtered_functions.with(|f| f.is_empty())>
                        <p class="p-1 text-xs text-muted-foreground">"Keine Treffer."</p>
                    </Show>
                </div>
            </div>
        }
    };

    let editor_area = move || {
        if content_loading.get() {
            return view! { <p class="text-sm text-muted-foreground">"Lade…"</p> }.into_any();
        }
        view! {
            <div class="flex min-h-0 flex-1 gap-2">
                <div class="min-h-0 min-w-0 flex-1 overflow-hidden rounded-md border border-border">
                    <CodeEditor
                        value=Signal::derive(move || content.get())
                        on_change=move |v: String| {
                            content.set(v);
                            dirty.set(true);
                            save_ok.set(None);
                        }
                        handle=editor
                        class="h-full text-xs"
                    />
                </div>
                <Show when=move || function_ref_open.get()>{function_panel}</Show>
            </div>
        }
        .into_any()
    };

    let main_pane = move || {
        view! {
            <div class="flex min-w-0 flex-1 flex-col gap-2">
                {move || match selected_path.get() {
                    None => view! {
                        <div class="flex flex-1 flex-col items-center justify-center gap-3 rounded-lg border border-dashed border-border text-center">
                            <p class="text-sm text-muted-foreground">"Wähle links eine Quest aus oder lege über „+\" eine neue an."</p>
                        </div>
                    }
                    .into_any(),
                    Some(path) => view! {
                        <div class="flex items-center justify-between">
                            <div class="flex items-center gap-2">
                                <Icon name="Code2" class="size-4 text-muted-foreground"/>
                                <code class="text-sm">{path}</code>
                            </div>
                            <div class="flex items-center gap-2">
                                {move || save_ok.get().map(|ok| view! {
                                    <span class="flex items-center gap-1 text-sm text-green-600">
                                        <Icon name="CheckCircle2" class="size-4"/>
                                        " "
                                        {ok}
                                    </span>
                                })}
                                {move || {
                                    let open = function_ref_open.get();
                                    view! {
                                        <Button
                                            variant=if open { ButtonVariant::Default } else { ButtonVariant::Outline }
                                            size=ButtonSize::Sm
                                            on_click=move |_| function_ref_open.update(|v| *v = !*v)
                                        >
                                            <Icon name="BookOpen" class="size-4"/>
                                            "Funktionen"
                                        </Button>
                                    }
                                }}
                                <Button variant=ButtonVariant::Destructive size=ButtonSize::Sm on_click=move |_| delete_confirm.set(true)>
                                    <Icon name="Trash2" class="size-4"/>
                                    "Löschen"
                                </Button>
                                <Button
                                    size=ButtonSize::Sm
                                    on_click=move |_| save_file()
                                    disabled=Signal::derive(move || saving.get() || !dirty.get())
                                >
                                    {move || if saving.get() { "Speichere…" } else { "Speichern" }}
                                </Button>
                            </div>
                        </div>
                        {move || save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        {move || content_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        {editor_area}
                    }
                    .into_any(),
                }}
            </div>
        }
    };

    let main_view = move || {
        view! {
            <div class="flex h-full flex-col gap-3">
                <div class="flex items-center justify-between">
                    <PageHeader module=Section::QuestBuilder title="Quest Builder".to_string()/>
                    <div class="flex items-center gap-2">
                        <Button variant=ButtonVariant::Outline on_click=move |_| open_wiki()>
                            <Icon name="BookOpen" class="size-4"/>
                            "Wiki öffnen"
                        </Button>
                        <Button variant=ButtonVariant::Outline on_click=move |_| load_files() disabled=loading>
                            <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                            "Neu laden"
                        </Button>
                        <Button on_click=move |_| run_deploy() disabled=deploying>
                            <Icon name="PlayCircle" class="size-4"/>
                            {move || if deploying.get() { "Kompiliere…" } else { "Kompilieren & Neuladen" }}
                        </Button>
                    </div>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Führt denselben Befehl aus wie „Quests reloaden\" in der Server-Steuerung ("
                    <code>"make.py"</code>
                    " kompiliert alle in "
                    <code>"quest_list"</code>
                    " eingetragenen Dateien neu). Fehler beim Kompilieren erscheinen unten in der Ausgabe."
                </p>

                <div class="flex min-h-0 flex-1 gap-4">
                    {sidebar}
                    {main_pane}
                </div>

                // Neue Quest
                <Show when=move || creating.get()>
                    <CreateQuestModal
                        on_close=move |_| creating.set(false)
                        on_created=move |path: String| {
                            // wie im TS: erst die Liste neu laden, dann die neue Datei öffnen
                            load_files();
                            open_file(path);
                        }
                    />
                </Show>

                <Show when=move || delete_confirm.get()>
                    <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                        <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                            <p class="flex items-start gap-2 text-sm">
                                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                                <span>
                                    <code>{move || selected_path.get().unwrap_or_default()}</code>
                                    " wird aus "
                                    <code>"quest_list"</code>
                                    " entfernt und die Quelldatei ins Backup-Verzeichnis verschoben (kein endgültiges Löschen)."
                                </span>
                            </p>
                            <div class="flex justify-end gap-2">
                                <Button variant=ButtonVariant::Outline on_click=move |_| delete_confirm.set(false)>
                                    "Abbrechen"
                                </Button>
                                <Button variant=ButtonVariant::Destructive on_click=move |_| confirm_delete()>
                                    "Entfernen"
                                </Button>
                            </div>
                        </div>
                    </div>
                </Show>

                <Show when=move || deploy_open.get()>
                    <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                        <div class="flex max-h-[70vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                            <div class="flex items-center justify-between">
                                <p class="text-sm font-medium">"Quests kompilieren & neu laden"</p>
                                <button on:click=move |_| deploy_open.set(false) class="text-muted-foreground hover:text-foreground">
                                    <Icon name="X" class="size-4"/>
                                </button>
                            </div>
                            <pre class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">
                                {move || deploy_log.with(|l| if l.is_empty() { "…".to_string() } else { l.concat() })}
                            </pre>
                        </div>
                    </div>
                </Show>
            </div>
        }
    };

    move || match mode.get() {
        PageMode::Loading => view! { <p class="text-sm text-muted-foreground">"Lade Quest-Liste…"</p> }.into_any(),
        PageMode::Error => view! {
            <div class="max-w-md space-y-3">
                <h1 class="text-2xl font-semibold">"Quest Builder"</h1>
                <p class="text-sm text-destructive">{move || load_error.get().unwrap_or_default()}</p>
                <Button on_click=move |_| load_files()>"Erneut versuchen"</Button>
            </div>
        }
        .into_any(),
        PageMode::Main => main_view().into_any(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn qf(category: &str, name: &str) -> QuestFile {
        QuestFile { category: category.into(), name: name.into(), extension: "lua".into(), relative_path: format!("{category}/{name}.lua") }
    }

    #[test]
    fn groups_and_filters() {
        let files = vec![qf("Pferd", "rufen"), qf("Biologie", "Orkzahn"), qf("biologie2", "x"), qf("Pferd", "Futter")];
        let g = group_by_category(&files, "");
        assert_eq!(g.iter().map(|(c, _)| c.as_str()).collect::<Vec<_>>(), vec!["Biologie", "biologie2", "Pferd"]);
        assert_eq!(g[2].1.len(), 2);
        let g = group_by_category(&files, "ork");
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].1[0].name, "Orkzahn");
    }
}
