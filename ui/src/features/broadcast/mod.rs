//! Broadcast-System (Port von `src/features/broadcast/BroadcastSystem.tsx`).

pub mod broadcast_quest;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::lib::{log_activity, use_save_shortcut};
use crate::store::navigation::{report_section_dirty, ModuleId, Section};
use crate::tauri::{invoke, invoke0, invoke_unit, listen, Unlisten};
use broadcast_quest::{generate_broadcast_quest, BroadcastMessage};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::{json, Value};

const QUEST_RELATIVE_PATH: &str = "Broadcast/Broadcast_System.lua";
const MODULE: ModuleId = ModuleId::Section(Section::BroadcastSystem);

#[derive(Clone, Debug, Deserialize)]
struct ServerCommandResult {
    #[allow(dead_code)]
    output: String,
    exit_status: Option<i64>,
}

/// Wie JavaScripts `Number(s)` (leer = 0, ungültig = NaN).
fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// Zahl als JSON-Argument (ganzzahlig, wenn möglich - wie JS es serialisiert).
fn num_value(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

pub fn can_save_message(text: &str, interval_minutes: &str) -> bool {
    let n = js_number(interval_minutes);
    !text.trim().is_empty() && n.is_finite() && n >= 1.0
}

fn truncate(s: &str) -> String {
    const MAX: usize = 40;
    if s.chars().count() > MAX {
        format!("{}…", s.chars().take(MAX).collect::<String>())
    } else {
        s.to_string()
    }
}

#[component]
pub fn BroadcastSystem() -> impl IntoView {
    let messages = RwSignal::new(None::<Vec<BroadcastMessage>>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let busy_id = RwSignal::new(None::<i64>);

    let editing_id = RwSignal::new(None::<i64>);
    let text = RwSignal::new(String::new());
    let interval_minutes = RwSignal::new("30".to_string());

    let deploying = RwSignal::new(false);
    let deploy_log = RwSignal::new(Vec::<String>::new());
    let deploy_open = RwSignal::new(false);

    let dirty = Memo::new(move |_| editing_id.with(|e| e.is_some()) || !text.with(|t| t.trim().is_empty()));
    Effect::new(move |_| report_section_dirty(Section::BroadcastSystem, dirty.get()));
    on_cleanup(move || report_section_dirty(Section::BroadcastSystem, false));

    let log_ref = NodeRef::<leptos::html::Pre>::new();

    let load = move || {
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke0::<Vec<BroadcastMessage>>("list_broadcast_messages").await {
                Ok(m) => {
                    messages.try_set(Some(m));
                }
                Err(e) => {
                    error.try_set(Some(e));
                }
            }
            loading.try_set(false);
        });
    };
    load();

    // Live-Ausgabe des Server-Befehls kommt als Ereignis-Strom (index.sh ist ein
    // interaktives Menüskript, dessen Ausgabe der Backend-Befehl live
    // weiterreicht, siehe `run_server_command`/`server-output`) - ohne diesen
    // Listener käme alles erst am Ende als ein einziger Textblock an.
    let unlisten = StoredValue::new_local(None::<Unlisten>);
    spawn_local(async move {
        let res = listen::<String>("server-output", move |payload| {
            deploy_log.try_update(|l| l.push(payload));
        })
        .await;
        if let Ok(u) = res {
            // Schon wieder ausgehängt, bevor das Abo stand: sofort aufheben.
            if let Some(u) = unlisten.try_set_value(Some(u)).flatten() {
                u.unlisten();
            }
        }
    });
    on_cleanup(move || {
        if let Some(Some(u)) = unlisten.try_update_value(|u| u.take()) {
            u.unlisten();
        }
    });

    Effect::new(move |_| {
        deploy_log.track();
        if let Some(el) = log_ref.get_untracked() {
            el.set_scroll_top(el.scroll_height());
        }
    });

    let reset_form = move || {
        editing_id.set(None);
        text.set(String::new());
        interval_minutes.set("30".into());
    };

    let start_edit = move |m: &BroadcastMessage| {
        editing_id.set(Some(m.id));
        text.set(m.text.clone());
        interval_minutes.set(m.interval_minutes.to_string());
    };

    let save_message = move || {
        let trimmed = text.get_untracked().trim().to_string();
        let minutes = js_number(&interval_minutes.get_untracked());
        if trimmed.is_empty() || !minutes.is_finite() || minutes < 1.0 {
            return;
        }
        let edit = editing_id.get_untracked();
        error.set(None);
        spawn_local(async move {
            let res = match edit {
                Some(id) => {
                    invoke_unit(
                        "update_broadcast_message",
                        &json!({ "id": id, "text": trimmed, "intervalMinutes": num_value(minutes) }),
                    )
                    .await
                }
                None => {
                    invoke_unit("create_broadcast_message", &json!({ "text": trimmed, "intervalMinutes": num_value(minutes) })).await
                }
            };
            match res {
                Ok(()) => {
                    match edit {
                        Some(id) => log_activity(
                            MODULE,
                            "update",
                            &format!("Broadcast-Nachricht aktualisiert: '{}'", truncate(&trimmed)),
                            Some("broadcast-message"),
                            Some(&id.to_string()),
                        ),
                        None => log_activity(
                            MODULE,
                            "create",
                            &format!("Broadcast-Nachricht angelegt: '{}'", truncate(&trimmed)),
                            Some("broadcast-message"),
                            None,
                        ),
                    }
                    reset_form();
                    load();
                }
                Err(e) => error.set(Some(e)),
            }
        });
    };

    use_save_shortcut(
        Section::BroadcastSystem,
        move || dirty.get_untracked() && can_save_message(&text.get_untracked(), &interval_minutes.get_untracked()),
        save_message,
    );

    let toggle_enabled = move |m: BroadcastMessage| {
        busy_id.set(Some(m.id));
        error.set(None);
        spawn_local(async move {
            match invoke_unit("set_broadcast_message_enabled", &json!({ "id": m.id, "enabled": !m.enabled })).await {
                Ok(()) => {
                    log_activity(
                        MODULE,
                        if m.enabled { "disable" } else { "enable" },
                        &format!(
                            "Broadcast-Nachricht '{}' {}",
                            truncate(&m.text),
                            if m.enabled { "deaktiviert" } else { "aktiviert" }
                        ),
                        Some("broadcast-message"),
                        Some(&m.id.to_string()),
                    );
                    load();
                }
                Err(e) => error.set(Some(e)),
            }
            busy_id.set(None);
        });
    };

    let delete_message = move |id: i64| {
        let target_text = messages.with_untracked(|m| {
            m.as_ref().and_then(|m| m.iter().find(|m| m.id == id).map(|m| m.text.clone())).unwrap_or_default()
        });
        busy_id.set(Some(id));
        error.set(None);
        spawn_local(async move {
            match invoke_unit("delete_broadcast_message", &json!({ "id": id })).await {
                Ok(()) => {
                    log_activity(
                        MODULE,
                        "delete",
                        &format!("Broadcast-Nachricht gelöscht: '{}'", truncate(&target_text)),
                        Some("broadcast-message"),
                        Some(&id.to_string()),
                    );
                    if editing_id.get_untracked() == Some(id) {
                        reset_form();
                    }
                    load();
                }
                Err(e) => error.set(Some(e)),
            }
            busy_id.set(None);
        });
    };

    // Gleicher Mechanismus wie Quest Builders "Kompilieren & Neuladen":
    // dieselbe `server_cmd_reload_quests`-Einstellung + `run_server_command`.
    // Neu ist nur der Schritt davor, der die generierte Datei über die schon
    // vorhandenen Quest-Builder-Commands schreibt (anlegen beim ersten Mal,
    // sonst überschreiben).
    let run_deploy = move || {
        deploy_open.set(true);
        deploying.set(true);
        error.set(None);
        deploy_log.update(|l| l.push("\n[Generiere Quest-Datei…]\n".into()));
        let content = generate_broadcast_quest(&messages.get_untracked().unwrap_or_default());
        spawn_local(async move {
            let res: Result<(), String> = async {
                let file_exists = invoke::<Value>("read_quest_file", &json!({ "relativePath": QUEST_RELATIVE_PATH })).await.is_ok();
                if file_exists {
                    invoke_unit("write_quest_file", &json!({ "relativePath": QUEST_RELATIVE_PATH, "content": content })).await?;
                } else {
                    invoke_unit(
                        "create_quest_file",
                        &json!({ "category": "Broadcast", "name": "Broadcast_System", "content": content, "extension": "lua" }),
                    )
                    .await?;
                }
                deploy_log.try_update(|l| l.push("[Quest-Datei geschrieben. Lade auf dem Server neu…]\n".into()));

                let setting = |key: &'static str| async move {
                    invoke::<Option<String>>("get_setting", &json!({ "key": key })).await.ok().flatten().filter(|s| !s.is_empty())
                };
                let workdir = setting("server_workdir").await.unwrap_or_else(|| "/usr/home/game".into());
                let command = setting("server_cmd_reload_quests").await.unwrap_or_else(|| format!("cd {workdir} && echo 4 | sh index.sh"));

                let result = invoke::<ServerCommandResult>("run_server_command", &json!({ "command": command })).await?;
                match result.exit_status {
                    Some(code) if code != 0 => {
                        deploy_log.try_update(|l| l.push(format!("\n[Beendet mit Code {code}]\n")));
                    }
                    _ => {
                        deploy_log.try_update(|l| l.push("\n[Fertig]\n".into()));
                    }
                }
                Ok(())
            }
            .await;
            if let Err(e) = res {
                error.try_set(Some(e.clone()));
                deploy_log.try_update(|l| l.push(format!("\nFehler: {e}\n")));
            }
            deploying.try_set(false);
        });
    };

    let preview = move || messages.with(|m| generate_broadcast_quest(m.as_deref().unwrap_or_default()));
    let active_count = move || messages.with(|m| m.as_ref().map(|m| m.iter().filter(|m| m.enabled).count()).unwrap_or(0));
    let total_count = move || messages.with(|m| m.as_ref().map(|m| m.len()).unwrap_or(0));

    view! {
        <div class="flex h-full max-w-4xl flex-col gap-4 pb-10">
            <div class="flex items-center justify-between">
                <PageHeader module=Section::BroadcastSystem title="Broadcast-System".to_string()/>
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                    "Neu laden"
                </Button>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Jede Nachricht bekommt ihr eigenes Intervall und wird per " <code>"notice_all()"</code>
                    " an alle Spieler auf allen Cores gesendet (kleine Ansagezeile, kein großes Banner - dafür gibt es auf diesem Server aktuell keinen funktionierenden globalen Befehl). Ein Timer wird erst beim ersten Login nach einem „Deploy & Neuladen\" scharfgeschaltet, nicht sofort beim Speichern hier."
                </span>
            </div>

            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            <div class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">
                    {move || if editing_id.with(|e| e.is_some()) { "Nachricht bearbeiten" } else { "Neue Nachricht" }}
                </h2>
                <textarea
                    prop:value=move || text.get()
                    on:input=move |e| text.set(event_target_value(&e))
                    placeholder="Ansagetext…"
                    rows=2
                    class="w-full rounded-md border border-border bg-background px-2 py-1 text-sm"
                ></textarea>
                <div class="flex items-center gap-2">
                    <label class="flex items-center gap-2 text-sm text-muted-foreground">
                        "Alle"
                        <input
                            type="number"
                            min=1
                            prop:value=move || interval_minutes.get()
                            on:input=move |e| interval_minutes.set(event_target_value(&e))
                            class="w-20 rounded-md border border-border bg-background px-2 py-1 text-sm"
                        />
                        "Minuten"
                    </label>
                    <Button
                        size=ButtonSize::Sm
                        on_click=move |_| save_message()
                        disabled=Signal::derive(move || text.with(|t| t.trim().is_empty()) || interval_minutes.with(|m| js_number(m) < 1.0))
                    >
                        <Icon name=Signal::derive(move || if editing_id.with(|e| e.is_some()) { "Pencil" } else { "Plus" }) class="size-3.5"/>
                        {move || if editing_id.with(|e| e.is_some()) { "Speichern" } else { "Hinzufügen" }}
                    </Button>
                    <Show when=move || editing_id.with(|e| e.is_some())>
                        <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| reset_form()>
                            <Icon name="X" class="size-3.5"/>
                            "Abbrechen"
                        </Button>
                    </Show>
                </div>
            </div>

            <div class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">
                    {move || format!("Nachrichten ({} von {} aktiv)", active_count(), total_count())}
                </h2>
                <Show when=move || total_count() == 0>
                    <p class="text-sm text-muted-foreground">"Noch keine Nachrichten angelegt."</p>
                </Show>
                {move || messages.get().unwrap_or_default().into_iter().map(|m| {
                    let (m_toggle, m_edit, id) = (m.clone(), m.clone(), m.id);
                    view! {
                        <div class="flex items-start gap-2 border-t border-border py-2 first:border-t-0">
                            <button
                                title=if m.enabled { "Aktiv - klicken zum Deaktivieren" } else { "Deaktiviert - klicken zum Aktivieren" }
                                on:click=move |_| toggle_enabled(m_toggle.clone())
                                disabled=move || busy_id.get() == Some(id)
                                class=if m.enabled {
                                    "mt-0.5 size-4 shrink-0 rounded-full border border-green-600 bg-green-600"
                                } else {
                                    "mt-0.5 size-4 shrink-0 rounded-full border border-border bg-transparent"
                                }
                            ></button>
                            <div class="min-w-0 flex-1">
                                <p class=if m.enabled { "whitespace-pre-wrap text-sm " } else { "whitespace-pre-wrap text-sm text-muted-foreground line-through" }>
                                    {m.text.clone()}
                                </p>
                                <p class="text-xs text-muted-foreground">{format!("Alle {} Minuten", m.interval_minutes)}</p>
                            </div>
                            <Button size=ButtonSize::IconSm variant=ButtonVariant::Ghost on_click=move |_| start_edit(&m_edit)>
                                <Icon name="Pencil" class="size-3.5"/>
                            </Button>
                            <Button
                                size=ButtonSize::IconSm
                                variant=ButtonVariant::Ghost
                                disabled=Signal::derive(move || busy_id.get() == Some(id))
                                on_click=move |_| delete_message(id)
                            >
                                <Icon name="Trash2" class="size-3.5"/>
                            </Button>
                        </div>
                    }
                }).collect_view()}
            </div>

            <div class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Vorschau (generierte Quest-Datei)"</h2>
                <pre class="max-h-64 overflow-auto rounded-md border border-border bg-muted/30 p-2 font-mono text-xs">{preview}</pre>
            </div>

            <div class="space-y-1">
                <div class="flex items-center justify-between">
                    <Button on_click=move |_| run_deploy() disabled=deploying>
                        <Icon name="PlayCircle" class="size-4"/>
                        {move || if deploying.get() { "Wird eingespielt…" } else { "Deploy & Neuladen" }}
                    </Button>
                    <Show when=move || deploy_log.with(|l| !l.is_empty())>
                        <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| deploy_open.set(true)>
                            "Ausgabe anzeigen"
                        </Button>
                    </Show>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Schreibt " <code>{QUEST_RELATIVE_PATH}</code>
                    " auf den Server und führt denselben Befehl aus wie „Quests reloaden\" in der Server-Steuerung."
                </p>
            </div>

            <Show when=move || deploy_open.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="flex max-h-[70vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">"Broadcast-System einspielen"</p>
                            <div class="flex items-center gap-1">
                                <Button
                                    variant=ButtonVariant::Ghost
                                    size=ButtonSize::IconSm
                                    on_click=move |_| deploy_log.set(vec![])
                                    disabled=Signal::derive(move || deploy_log.with(|l| l.is_empty()))
                                    title="Ausgabe leeren"
                                >
                                    <Icon name="Eraser" class="size-3.5"/>
                                </Button>
                                <button on:click=move |_| deploy_open.set(false) class="text-muted-foreground hover:text-foreground">
                                    <Icon name="X" class="size-4"/>
                                </button>
                            </div>
                        </div>
                        <pre
                            node_ref=log_ref
                            class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/30 p-2 font-mono text-xs"
                        >
                            {move || deploy_log.with(|l| if l.is_empty() { "…".to_string() } else { l.concat() })}
                        </pre>
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
    fn save_validation_like_number_is_finite() {
        assert!(can_save_message("Hallo", "30"));
        assert!(!can_save_message("  ", "30"));
        assert!(!can_save_message("Hallo", "0"));
        assert!(!can_save_message("Hallo", "abc"));
        assert!(!can_save_message("Hallo", ""));
        assert!(can_save_message("Hallo", "1.5"));
    }

    #[test]
    fn truncates_after_40_chars() {
        assert_eq!(truncate("kurz"), "kurz");
        let long = "a".repeat(41);
        assert_eq!(truncate(&long), format!("{}…", "a".repeat(40)));
    }
}
