//! Server-Steuerung: Start/Stopp/Logs löschen/Quests neu laden über frei
//! einstellbare Shell-Befehle, Live-Ausgabe per `server-output`-Event und
//! eine Volltextsuche über die echten Log-Dateien.

pub mod util;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::i18n::t;
use crate::store::connection_profiles::connection_profiles;
use crate::store::navigation::Section;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use util::{auto_scroll, js_number, js_number_string, listen_while_mounted, sleep_ms};

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct LogSearchHit {
    file: String,
    line: i64,
    text: String,
}

#[derive(Clone, Debug, Deserialize)]
struct ServerCommandResult {
    #[allow(dead_code)]
    output: String,
    exit_status: Option<i64>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum ActionId {
    Start,
    Stop,
    ClearLogs,
    ReloadQuests,
}

struct ServerAction {
    id: ActionId,
    label_key: &'static str,
    icon: &'static str,
    variant: ButtonVariant,
    /// Destruktive Aktionen fragen vorher nach - einen laufenden Server
    /// stoppen oder Logs löschen lässt sich nicht rückgängig machen.
    confirm: Option<&'static str>,
}

const ACTIONS: [ServerAction; 4] = [
    ServerAction { id: ActionId::Start, label_key: "serverControl.startServer", icon: "Play", variant: ButtonVariant::Default, confirm: None },
    ServerAction {
        id: ActionId::Stop,
        label_key: "serverControl.stopServer",
        icon: "Square",
        variant: ButtonVariant::Destructive,
        confirm: Some("Der Server wird heruntergefahren und alle Spieler werden getrennt."),
    },
    ServerAction {
        id: ActionId::ClearLogs,
        label_key: "serverControl.clearLogs",
        icon: "Trash2",
        variant: ButtonVariant::Destructive,
        confirm: Some("Alle Logdateien werden unwiderruflich gelöscht."),
    },
    ServerAction { id: ActionId::ReloadQuests, label_key: "serverControl.reloadQuests", icon: "RefreshCw", variant: ButtonVariant::Outline, confirm: None },
];

impl ActionId {
    fn idx(self) -> usize {
        match self {
            ActionId::Start => 0,
            ActionId::Stop => 1,
            ActionId::ClearLogs => 2,
            ActionId::ReloadQuests => 3,
        }
    }
    fn setting_key(self) -> &'static str {
        match self {
            ActionId::Start => "server_cmd_start",
            ActionId::Stop => "server_cmd_stop",
            ActionId::ClearLogs => "server_cmd_clear_logs",
            ActionId::ReloadQuests => "server_cmd_reload_quests",
        }
    }
    fn default_choice(self) -> u32 {
        match self {
            ActionId::Start => 1,
            ActionId::Stop => 2,
            ActionId::ClearLogs => 3,
            ActionId::ReloadQuests => 4,
        }
    }
    fn action(self) -> &'static ServerAction {
        &ACTIONS[self.idx()]
    }
}

const DEFAULT_WORKDIR: &str = "/usr/home/game";

/// Das typische Metin2-Steuerskript zeigt ein Menü und liest eine Zahl von
/// stdin - jede Aktion ist also dieses Skript mit der passenden Menüauswahl.
fn default_command(workdir: &str, choice: u32) -> String {
    format!("cd {workdir} && echo {choice} | sh index.sh")
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Running {
    Action(ActionId),
    Restart,
}

async fn get_setting(key: &str) -> Option<String> {
    invoke::<Option<String>>("get_setting", &json!({ "key": key })).await.ok().flatten()
}

fn is_digits(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_digit())
}

const INPUT_MONO: &str = "w-full rounded-md border border-border bg-background px-2 py-1 font-mono text-sm";

#[component]
pub fn ServerControl() -> impl IntoView {
    let commands = RwSignal::new([String::new(), String::new(), String::new(), String::new()]);
    let workdir = RwSignal::new(DEFAULT_WORKDIR.to_string());
    let editing = RwSignal::new(false);
    let running = RwSignal::new(None::<Running>);
    let log = RwSignal::new(Vec::<String>::new());
    let error = RwSignal::new(None::<String>);
    let confirm_action = RwSignal::new(None::<ActionId>);
    let restart_confirm = RwSignal::new(false);
    let restart_delay = RwSignal::new("5".to_string());
    let setting_save_error = RwSignal::new(None::<String>);

    let log_search_open = RwSignal::new(false);
    let log_search_query = RwSignal::new(String::new());
    let log_search_hits = RwSignal::new(None::<Vec<LogSearchHit>>);
    let log_search_loading = RwSignal::new(false);
    let log_search_error = RwSignal::new(None::<String>);

    let log_ref = NodeRef::<leptos::html::Pre>::new();

    // Arbeitsordner/Befehle gehören zum Server-Profil -> nach Wechsel neu laden.
    let profiles = connection_profiles();
    Effect::new(move |_| {
        profiles.revision.track();
        spawn_local(async move {
            let saved_workdir = get_setting("server_workdir").await.filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_WORKDIR.to_string());
            workdir.set(saved_workdir.clone());

            let mut loaded: [String; 4] = Default::default();
            for action in ACTIONS.iter() {
                let saved = get_setting(action.id.setting_key()).await.filter(|s| !s.is_empty());
                loaded[action.id.idx()] = saved.unwrap_or_else(|| default_command(&saved_workdir, action.id.default_choice()));
            }
            commands.set(loaded);

            if let Some(d) = get_setting("server_restart_delay_seconds").await.filter(|s| !s.is_empty()) {
                restart_delay.set(d);
            }
        });
    });

    listen_while_mounted::<String>("server-output", move |payload| log.update(|l| l.push(payload)));
    auto_scroll(log_ref, move || log.track());

    let cmd = move |id: ActionId| commands.with(|c| c[id.idx()].clone());

    async fn run_command(log: RwSignal<Vec<String>>, command: String) -> Result<(), String> {
        log.update(|l| l.push(format!("\n$ {command}\n")));
        let result = invoke::<ServerCommandResult>("run_server_command", &json!({ "command": command })).await?;
        if let Some(code) = result.exit_status {
            if code != 0 {
                log.update(|l| l.push(format!("\n[Beendet mit Code {code}]\n")));
            }
        }
        Ok(())
    }

    let execute = move |id: ActionId| {
        confirm_action.set(None);
        running.set(Some(Running::Action(id)));
        error.set(None);
        let command = cmd(id);
        spawn_local(async move {
            if let Err(e) = run_command(log, command).await {
                error.set(Some(e));
            }
            running.set(None);
        });
    };

    let request_action = move |id: ActionId| {
        if id.action().confirm.is_some() {
            confirm_action.set(Some(id));
        } else {
            execute(id);
        }
    };

    let execute_restart = move || {
        restart_confirm.set(false);
        running.set(Some(Running::Restart));
        error.set(None);
        let stop = cmd(ActionId::Stop);
        let start = cmd(ActionId::Start);
        let delay = restart_delay.get_untracked();
        spawn_local(async move {
            let res: Result<(), String> = async {
                run_command(log, stop).await?;
                let n = js_number(&delay);
                let delay_seconds = if n.is_nan() { 0.0 } else { n };
                if delay_seconds > 0.0 {
                    log.update(|l| l.push(format!("\n[Warte {}s vor dem Start…]\n", js_number_string(delay_seconds))));
                    sleep_ms(delay_seconds * 1000.0).await;
                }
                run_command(log, start).await
            }
            .await;
            if let Err(e) = res {
                error.set(Some(e));
            }
            running.set(None);
        });
    };

    // Speichert bei jedem Tastendruck - ein Fehler wird deshalb inline unter
    // den Feldern angezeigt (und beim nächsten erfolgreichen Speichern wieder
    // ausgeblendet) statt als Toast pro Tastendruck.
    let persist_setting = move |key: &'static str, value: String| {
        spawn_local(async move {
            match invoke::<serde_json::Value>("set_setting", &json!({ "key": key, "value": value })).await {
                Ok(_) => setting_save_error.set(None),
                Err(e) => setting_save_error.set(Some(format!("Einstellung konnte nicht gespeichert werden: {e}"))),
            }
        });
    };

    let save_command = move |id: ActionId, value: String| {
        commands.update(|c| c[id.idx()] = value.clone());
        persist_setting(id.setting_key(), value);
    };

    let save_workdir = move |value: String| {
        workdir.set(value.clone());
        persist_setting("server_workdir", value);
    };

    let save_restart_delay = move |value: String| {
        restart_delay.set(value.clone());
        if is_digits(&value) {
            persist_setting("server_restart_delay_seconds", value);
        }
    };

    let run_log_search = move || {
        let q = log_search_query.get_untracked().trim().to_string();
        if q.is_empty() {
            return;
        }
        log_search_loading.set(true);
        log_search_error.set(None);
        spawn_local(async move {
            match invoke::<Vec<LogSearchHit>>("search_server_logs", &json!({ "pattern": q })).await {
                Ok(hits) => log_search_hits.set(Some(hits)),
                Err(e) => log_search_error.set(Some(e)),
            }
            log_search_loading.set(false);
        });
    };

    let reset_to_defaults = move || {
        let wd = workdir.get_untracked();
        for action in ACTIONS.iter() {
            save_command(action.id, default_command(&wd, action.id.default_choice()));
        }
    };

    let action_buttons = ACTIONS
        .iter()
        .map(|action| {
            let id = action.id;
            let label_key = action.label_key;
            let icon = action.icon;
            view! {
                <Button
                    variant=action.variant
                    disabled=Signal::derive(move || running.get().is_some() || commands.with(|c| c[id.idx()].is_empty()))
                    on_click=move |_| request_action(id)
                >
                    <Icon name=icon class="size-4"/>
                    {move || if running.get() == Some(Running::Action(id)) { "Läuft…".to_string() } else { t(label_key) }}
                </Button>
            }
        })
        .collect_view();

    let command_inputs = ACTIONS
        .iter()
        .map(|action| {
            let id = action.id;
            let label_key = action.label_key;
            view! {
                <div class="space-y-1">
                    <label class="text-xs font-medium text-muted-foreground">{move || t(label_key)}</label>
                    <input
                        prop:value=move || commands.with(|c| c[id.idx()].clone())
                        on:input=move |e| save_command(id, event_target_value(&e))
                        class=INPUT_MONO
                    />
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="flex h-full flex-col gap-4">
            <div class="flex items-center justify-between">
                <PageHeader module=Section::ServerControl title=Signal::derive(|| t("serverControl.title"))/>
                <Button variant=ButtonVariant::Ghost on_click=move |_| editing.update(|v| *v = !*v)>
                    <Icon name="Settings2" class="size-4"/>
                    "Befehle "
                    {move || if editing.get() { "ausblenden" } else { "anpassen" }}
                </Button>
            </div>

            <div class="flex flex-wrap gap-2">
                {action_buttons}
                <Button
                    variant=ButtonVariant::Destructive
                    disabled=Signal::derive(move || {
                        running.get().is_some() || commands.with(|c| c[ActionId::Start.idx()].is_empty() || c[ActionId::Stop.idx()].is_empty())
                    })
                    on_click=move |_| restart_confirm.set(true)
                >
                    <Icon name="RotateCcw" class="size-4"/>
                    {move || if running.get() == Some(Running::Restart) { "Startet neu…" } else { "Neustarten" }}
                </Button>
            </div>

            <div class="space-y-3 rounded-lg border border-border bg-card p-3" class:hidden=move || !editing.get()>
                {move || setting_save_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                <div class="space-y-1">
                    <label class="text-xs font-medium text-muted-foreground">"Arbeitsverzeichnis"</label>
                    <input prop:value=move || workdir.get() on:input=move |e| save_workdir(event_target_value(&e)) class=INPUT_MONO/>
                </div>
                {command_inputs}
                <div class="space-y-1">
                    <label class="text-xs font-medium text-muted-foreground">"Wartezeit vor Neustart (Sekunden)"</label>
                    <input
                        prop:value=move || restart_delay.get()
                        on:input=move |e| save_restart_delay(event_target_value(&e))
                        class="w-20 rounded-md border border-border bg-background px-2 py-1 font-mono text-sm"
                    />
                    <p class="text-xs text-muted-foreground">
                        "Zeit zwischen \"Server/Channel schließen\" und \"Server/Channel starten\" beim Neustarten, damit die Prozesse sauber beendet sind."
                    </p>
                </div>
                <div class="flex items-center justify-between">
                    <p class="text-xs text-muted-foreground">"Voreingestellt für ein Menü-Skript, das die Auswahl von stdin liest."</p>
                    <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=move |_| reset_to_defaults()>
                        "Zurücksetzen"
                    </Button>
                </div>
            </div>

            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <div class="flex min-h-0 flex-1 flex-col gap-1">
                <div class="flex items-center justify-between">
                    <span class="text-xs font-medium text-muted-foreground">"Ausgabe"</span>
                    <Button
                        variant=ButtonVariant::Ghost
                        size=ButtonSize::Sm
                        on_click=move |_| log.set(vec![])
                        disabled=Signal::derive(move || log.with(|l| l.is_empty()))
                    >
                        <Icon name="Eraser" class="size-3.5"/>
                        "Leeren"
                    </Button>
                </div>
                <pre
                    node_ref=log_ref
                    class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-lg border border-border bg-muted/30 p-3 font-mono text-xs"
                >
                    {move || log.with(|l| if l.is_empty() { "Noch keine Ausgabe.".to_string() } else { l.concat() })}
                </pre>
            </div>

            <div class="space-y-2 rounded-lg border border-border bg-card p-3">
                <button
                    on:click=move |_| log_search_open.update(|v| *v = !*v)
                    class="flex w-full items-center justify-between text-sm font-medium"
                >
                    <span class="flex items-center gap-2">
                        <Icon name="Search" class="size-4"/>
                        "Log-Archiv durchsuchen"
                    </span>
                    <span class="text-xs text-muted-foreground">{move || if log_search_open.get() { "Ausblenden" } else { "Anzeigen" }}</span>
                </button>
                <Show when=move || log_search_open.get()>
                    <div class="space-y-2">
                        <p class="text-xs text-muted-foreground">
                            "Durchsucht die echten Server-Log-Dateien ("<code>"syserr"</code>"/"<code>"syslog"</code>"/"
                            <code>"stdout"</code>", über alle Channels und archivierten Ordner hinweg) im konfigurierten Arbeitsverzeichnis - nicht die obige Live-Ausgabe. Reiner Text, keine Regex nötig."
                        </p>
                        <div class="flex gap-2">
                            <input
                                prop:value=move || log_search_query.get()
                                on:input=move |e| log_search_query.set(event_target_value(&e))
                                on:keydown=move |e| {
                                    if e.key() == "Enter" {
                                        run_log_search()
                                    }
                                }
                                placeholder="z.B. SPEEDHACK, ein Spielername, eine Fehlermeldung…"
                                class="flex-1 rounded-md border border-border bg-background px-2 py-1 text-sm"
                            />
                            <Button
                                size=ButtonSize::Sm
                                on_click=move |_| run_log_search()
                                disabled=Signal::derive(move || log_search_loading.get() || log_search_query.with(|q| q.trim().is_empty()))
                            >
                                <Icon name="Search" class="size-3.5"/>
                                {move || if log_search_loading.get() { "Suche…" } else { "Suchen" }}
                            </Button>
                        </div>
                        {move || log_search_error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}
                        {move || log_search_hits.get().map(|hits| {
                            let n = hits.len();
                            view! {
                                <div class="max-h-64 space-y-1 overflow-auto rounded-md border border-border p-1">
                                    {(n == 0).then(|| view! { <p class="p-2 text-sm text-muted-foreground">"Keine Treffer."</p> })}
                                    {(n >= 500).then(|| view! {
                                        <p class="p-1 text-xs text-amber-600">"500 Treffer erreicht - Ergebnis wurde begrenzt, ggf. genauer eingrenzen."</p>
                                    })}
                                    {hits.into_iter().map(|hit| view! {
                                        <div class="rounded-md px-2 py-1 text-xs hover:bg-muted">
                                            <div class="font-mono text-muted-foreground">{hit.file.clone()}":"{hit.line}</div>
                                            <div class="whitespace-pre-wrap break-all font-mono">{hit.text.clone()}</div>
                                        </div>
                                    }).collect_view()}
                                </div>
                            }
                        })}
                    </div>
                </Show>
            </div>

            {move || confirm_action.get().map(|id| {
                let action = id.action();
                let label_key = action.label_key;
                view! {
                    <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                        <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                            <p class="text-sm font-medium">{move || t(label_key)}</p>
                            <p class="text-sm text-muted-foreground">{action.confirm.unwrap_or_default()}</p>
                            <pre class="overflow-x-auto rounded-md bg-muted/50 p-2 font-mono text-xs">{move || cmd(id)}</pre>
                            <div class="flex justify-end gap-2">
                                <Button variant=ButtonVariant::Outline on_click=move |_| confirm_action.set(None)>
                                    {move || t("common.cancel")}
                                </Button>
                                <Button variant=ButtonVariant::Destructive on_click=move |_| execute(id)>
                                    "Ausführen"
                                </Button>
                            </div>
                        </div>
                    </div>
                }
            })}

            <Show when=move || restart_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="text-sm font-medium">"Neustarten"</p>
                        <p class="text-sm text-muted-foreground">
                            "Der Server wird heruntergefahren, alle Spieler werden getrennt, und nach "
                            {move || {
                                let d = restart_delay.get();
                                if d.is_empty() { "0".to_string() } else { d }
                            }}
                            "s wieder gestartet."
                        </p>
                        <pre class="overflow-x-auto rounded-md bg-muted/50 p-2 font-mono text-xs">
                            {move || format!("{}\n{}", cmd(ActionId::Stop), cmd(ActionId::Start))}
                        </pre>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| restart_confirm.set(false)>
                                {move || t("common.cancel")}
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| execute_restart()>
                                "Ausführen"
                            </Button>
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
    fn default_commands() {
        assert_eq!(default_command("/usr/home/game", 1), "cd /usr/home/game && echo 1 | sh index.sh");
        assert_eq!(ActionId::ReloadQuests.default_choice(), 4);
    }

    #[test]
    fn delay_only_saved_when_digits() {
        assert!(is_digits("10"));
        assert!(!is_digits(""));
        assert!(!is_digits("1.5"));
        assert!(!is_digits("-1"));
    }
}
