//! Tag/Nacht & Schnee: setzt die Event-Flags `day` und `xmas_snow` über eine
//! generierte Quest-Datei (wirkt beim nächsten Login, ohne Server-Neustart).

pub mod weather_quest;

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::server_control::util::{auto_scroll, copy_to_clipboard, listen_while_mounted};
use crate::util::log_activity;
use crate::store::navigation::EmbeddedModule;
use crate::tauri::{invoke, invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use weather_quest::{generate_weather_quest, gm_commands_for, WeatherState};

const QUEST_RELATIVE_PATH: &str = "Weather/Weather_System.lua";

#[derive(Clone, Debug, Deserialize)]
struct ServerCommandResult {
    #[allow(dead_code)]
    output: String,
    exit_status: Option<i64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Night,
    Snow,
}

#[component]
pub fn WeatherControl() -> impl IntoView {
    let state = RwSignal::new(None::<WeatherState>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);
    let busy = RwSignal::new(false);

    let deploying = RwSignal::new(false);
    let deploy_log = RwSignal::new(Vec::<String>::new());
    let deploy_open = RwSignal::new(false);
    let log_ref = NodeRef::<leptos::html::Pre>::new();

    let load = move || {
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke0::<WeatherState>("get_weather_state").await {
                Ok(s) => state.set(Some(s)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };
    load();

    // Live-Ausgabe als Ereignis-Strom statt am Ende als ein Textblock.
    listen_while_mounted::<String>("server-output", move |payload| deploy_log.update(|l| l.push(payload)));
    auto_scroll(log_ref, move || deploy_log.track());

    let toggle = move |field: Field| {
        let Some(cur) = state.get_untracked() else { return };
        let mut next = cur.clone();
        match field {
            Field::Night => next.night_enabled = !cur.night_enabled,
            Field::Snow => next.snow_enabled = !cur.snow_enabled,
        }
        busy.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<WeatherState>(
                "set_weather_state",
                &json!({ "nightEnabled": next.night_enabled, "snowEnabled": next.snow_enabled }),
            )
            .await
            {
                Ok(result) => {
                    let summary = format!("Wetter gesetzt: Nacht={}, Schnee={}", result.night_enabled, result.snow_enabled);
                    state.set(Some(result));
                    log_activity(EmbeddedModule::WeatherControl.into(), "set", &summary, None, None);
                }
                Err(e) => error.set(Some(e)),
            }
            busy.set(false);
        });
    };

    // Gleicher Mechanismus wie "Deploy & Neuladen" im Broadcast-System:
    // dieselbe `server_cmd_reload_quests`-Einstellung + `run_server_command`.
    let run_deploy = move || {
        let Some(st) = state.get_untracked() else { return };
        deploy_open.set(true);
        deploying.set(true);
        error.set(None);
        deploy_log.update(|l| l.push("\n[Generiere Quest-Datei…]\n".into()));
        spawn_local(async move {
            let res: Result<(), String> = async {
                let content = generate_weather_quest(&st);

                let file_exists =
                    invoke::<serde_json::Value>("read_quest_file", &json!({ "relativePath": QUEST_RELATIVE_PATH })).await.is_ok();

                if file_exists {
                    invoke_unit("write_quest_file", &json!({ "relativePath": QUEST_RELATIVE_PATH, "content": content })).await?;
                } else {
                    invoke_unit(
                        "create_quest_file",
                        &json!({ "category": "Weather", "name": "Weather_System", "content": content, "extension": "lua" }),
                    )
                    .await?;
                }
                deploy_log.update(|l| l.push("[Quest-Datei geschrieben. Lade auf dem Server neu…]\n".into()));

                let workdir = invoke::<Option<String>>("get_setting", &json!({ "key": "server_workdir" }))
                    .await
                    .ok()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| "/usr/home/game".into());
                let command = invoke::<Option<String>>("get_setting", &json!({ "key": "server_cmd_reload_quests" }))
                    .await
                    .ok()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("cd {workdir} && echo 4 | sh index.sh"));

                let result = invoke::<ServerCommandResult>("run_server_command", &json!({ "command": command })).await?;
                match result.exit_status {
                    Some(code) if code != 0 => deploy_log.update(|l| l.push(format!("\n[Beendet mit Code {code}]\n"))),
                    _ => deploy_log.update(|l| l.push("\n[Fertig]\n".into())),
                }
                Ok(())
            }
            .await;
            if let Err(e) = res {
                error.set(Some(e.clone()));
                deploy_log.update(|l| l.push(format!("\nFehler: {e}\n")));
            }
            deploying.set(false);
        });
    };

    let preview = move || state.with(|s| s.as_ref().map(generate_weather_quest).unwrap_or_default());
    let gm_commands = move || state.with(|s| s.as_ref().map(|s| gm_commands_for(s.night_enabled, s.snow_enabled)).unwrap_or_default());

    view! {
        <div class="flex h-full max-w-4xl flex-col gap-4 pb-10">
            <div class="flex items-center justify-between">
                <PageHeader module=EmbeddedModule::WeatherControl title="Tag/Nacht & Schnee".to_string()/>
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                    "Neu laden"
                </Button>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="Info" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Regen gibt es auf diesem Server-Code nicht (kein Feature im Quellcode, weder Client noch Server) - nur Tag/Nacht und Schnee sind über echte Server-Flags steuerbar. Die Änderung greift server-weit sofort für alle Online-Spieler, sobald nach „Deploy & Neuladen\" irgendein Spieler das nächste Mal einloggt (kein Server-Neustart nötig). Für sofortige Wirkung ohne auf einen Login zu warten, kannst du stattdessen die GM-Befehle unten direkt im Spiel eingeben."
                </span>
            </div>

            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            {move || state.get().map(|s| {
                let dot = |on: bool| {
                    if on {
                        "size-4 shrink-0 rounded-full border border-green-600 bg-green-600"
                    } else {
                        "size-4 shrink-0 rounded-full border border-border bg-transparent"
                    }
                };
                view! {
                    <div class="space-y-3 rounded-lg border border-border p-4">
                        <button
                            on:click=move |_| toggle(Field::Night)
                            disabled=move || busy.get()
                            class="flex w-full items-center justify-between gap-2 rounded-md border border-border px-3 py-2 text-left hover:bg-muted/50"
                        >
                            <span class="flex items-center gap-2 text-sm font-medium">
                                <Icon name=if s.night_enabled { "Moon" } else { "Sun" } class="size-4"/>
                                {if s.night_enabled { "Nacht erzwungen" } else { "Tag (normal)" }}
                            </span>
                            <span class=dot(s.night_enabled)></span>
                        </button>
                        <button
                            on:click=move |_| toggle(Field::Snow)
                            disabled=move || busy.get()
                            class="flex w-full items-center justify-between gap-2 rounded-md border border-border px-3 py-2 text-left hover:bg-muted/50"
                        >
                            <span class="flex items-center gap-2 text-sm font-medium">
                                <Icon name="Snowflake" class="size-4"/>
                                "Schnee "
                                {if s.snow_enabled { "aktiv" } else { "aus" }}
                            </span>
                            <span class=dot(s.snow_enabled)></span>
                        </button>
                    </div>
                }
            })}

            <div class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"GM-Befehle (sofortige Wirkung, manuell im Spiel)"</h2>
                {move || gm_commands().into_iter().map(|cmd| {
                    let c = cmd.clone();
                    view! {
                        <div class="flex items-center gap-2">
                            <code class="flex-1 rounded-md border border-border bg-muted/30 px-2 py-1 font-mono text-xs">{cmd}</code>
                            <Button
                                size=ButtonSize::IconSm
                                variant=ButtonVariant::Ghost
                                title="Kopieren"
                                on_click=move |_| {
                                    let c = c.clone();
                                    spawn_local(async move { copy_to_clipboard(&c).await })
                                }
                            >
                                <Icon name="Copy" class="size-3.5"/>
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
                    <Button on_click=move |_| run_deploy() disabled=Signal::derive(move || deploying.get() || state.with(|s| s.is_none()))>
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
                    "Schreibt "<code>{QUEST_RELATIVE_PATH}</code>" auf den Server und führt denselben Befehl aus wie „Quests reloaden\" in der Server-Steuerung."
                </p>
            </div>

            <div class="fixed inset-0 flex items-center justify-center bg-black/50" class:hidden=move || !deploy_open.get()>
                <div class="flex max-h-[70vh] w-[36rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                    <div class="flex items-center justify-between">
                        <p class="text-sm font-medium">"Tag/Nacht & Schnee einspielen"</p>
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
        </div>
    }
}
