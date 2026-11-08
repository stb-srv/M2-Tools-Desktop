//! Server-Quellcode bauen (in einer separaten Arbeitskopie) und die
//! Programmdateien live einspielen bzw. zurückrollen.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::features::server_control::util::{auto_scroll, listen_while_mounted};
use crate::lib::format::de_datetime;
use crate::store::navigation::Section;
use crate::tauri::{invoke, invoke0};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DeployRecord {
    id: i64,
    kind: String,
    targets: Vec<String>,
    created_at: String,
    game_backup_path: Option<String>,
    db_backup_path: Option<String>,
    note: Option<String>,
    success: Option<bool>,
    #[allow(dead_code)]
    rolled_back_from: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct BuildTargets {
    libs: Vec<String>,
    bins: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct ServerCommandResult {
    #[allow(dead_code)]
    output: String,
    exit_status: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
enum RollbackTarget {
    #[allow(dead_code)]
    Latest,
    Record(DeployRecord),
}

const DEPLOY_PHRASE: &str = "SERVER ERSETZEN";
const ROLLBACK_PHRASE: &str = "ROLLBACK";

const SETTING_DEFAULTS: [(&str, &str); 6] = [
    ("build_live_source_root", "/usr/home/source/server"),
    ("build_scratch_source_root", "/usr/home/m2manager_build/server"),
    ("build_live_game_binary", "/usr/home/source/server/game/game"),
    ("build_live_db_binary", "/usr/home/source/server/db/db"),
    ("deploy_liveness_wait_seconds", "8"),
    ("deploy_liveness_recheck_seconds", "5"),
];

fn defaults() -> BTreeMap<String, String> {
    SETTING_DEFAULTS.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

fn format_date(iso: &str) -> String {
    de_datetime(iso)
}

const MODAL_INPUT: &str = "w-full rounded-md border border-border bg-background px-2 py-1 text-sm";

#[component]
pub fn BuildDeploy() -> impl IntoView {
    let settings = RwSignal::new(defaults());
    let editing_settings = RwSignal::new(false);
    let setting_save_error = RwSignal::new(None::<String>);

    let targets = RwSignal::new(None::<BuildTargets>);
    let selected = RwSignal::new(HashSet::<String>::new());
    let clean_build = RwSignal::new(true);

    let log = RwSignal::new(Vec::<String>::new());
    let log_open = RwSignal::new(false);
    let log_ref = NodeRef::<leptos::html::Pre>::new();

    let syncing = RwSignal::new(false);
    let building = RwSignal::new(false);
    let deploying = RwSignal::new(false);
    let rolling_back = RwSignal::new(false);

    let build_ok = RwSignal::new(None::<bool>);
    let note = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);

    let deploy_confirming = RwSignal::new(false);
    let deploy_confirm_text = RwSignal::new(String::new());
    let rollback_confirming = RwSignal::new(None::<RollbackTarget>);
    let rollback_confirm_text = RwSignal::new(String::new());

    let history = RwSignal::new(Vec::<DeployRecord>::new());
    let last_result = RwSignal::new(None::<DeployRecord>);

    async fn refresh_history(history: RwSignal<Vec<DeployRecord>>) {
        let rows = invoke0::<Vec<DeployRecord>>("list_deploy_history").await.unwrap_or_default();
        history.set(rows);
    }

    spawn_local(async move {
        let mut loaded = defaults();
        for (key, _) in SETTING_DEFAULTS.iter() {
            let saved = invoke::<Option<String>>("get_setting", &json!({ "key": key })).await.ok().flatten();
            if let Some(s) = saved.filter(|s| !s.is_empty()) {
                loaded.insert(key.to_string(), s);
            }
        }
        settings.set(loaded);

        if let Ok(t) = invoke0::<BuildTargets>("list_build_targets").await {
            selected.set(t.libs.iter().chain(t.bins.iter()).cloned().collect());
            targets.set(Some(t));
        }

        refresh_history(history).await;
    });

    listen_while_mounted::<String>("build-deploy-output", move |payload| log.update(|l| l.push(payload)));
    auto_scroll(log_ref, move || log.track());

    let setting = move |key: &str| settings.with(|s| s.get(key).cloned().unwrap_or_default());

    // Speichert bei jedem Tastendruck - Fehler daher inline unter den Feldern
    // statt als Toast pro Tastendruck.
    let save_setting = move |key: &'static str, value: String| {
        settings.update(|s| {
            s.insert(key.to_string(), value.clone());
        });
        spawn_local(async move {
            match invoke::<serde_json::Value>("set_setting", &json!({ "key": key, "value": value })).await {
                Ok(_) => setting_save_error.set(None),
                Err(e) => setting_save_error.set(Some(format!("Einstellung konnte nicht gespeichert werden: {e}"))),
            }
        });
    };

    let toggle_target = move |name: String| {
        selected.update(|s| {
            if !s.remove(&name) {
                s.insert(name);
            }
        });
    };

    let push_log = move |s: String| log.update(|l| l.push(s));

    let run_sync = move || {
        syncing.set(true);
        error.set(None);
        log_open.set(true);
        push_log("\n[Synchronisiere Quellcode-Kopie…]\n".into());
        spawn_local(async move {
            match invoke0::<ServerCommandResult>("sync_build_source").await {
                Ok(result) => match result.exit_status {
                    Some(code) if code != 0 => push_log(format!("\n[Beendet mit Code {code}]\n")),
                    _ => push_log("\n[Synchronisierung abgeschlossen.]\n".into()),
                },
                Err(e) => {
                    error.set(Some(e.clone()));
                    push_log(format!("\nFehler: {e}\n"));
                }
            }
            syncing.set(false);
        });
    };

    let run_build = move || {
        let Some(t) = targets.get_untracked() else { return };
        building.set(true);
        build_ok.set(None);
        error.set(None);
        log_open.set(true);
        push_log("\n[Baue Server-Quellcode (Arbeitskopie — Live-Server bleibt unberührt)…]\n".into());
        // Bibliotheken immer vor den Programmdateien, die sie brauchen -
        // Reihenfolge unabhängig von der Klick-Reihenfolge des Nutzers.
        let ordered: Vec<String> = selected.with_untracked(|sel| t.libs.iter().chain(t.bins.iter()).filter(|x| sel.contains(*x)).cloned().collect());
        let source_root = setting("build_scratch_source_root");
        let clean = clean_build.get_untracked();
        spawn_local(async move {
            match invoke::<ServerCommandResult>(
                "run_source_build",
                &json!({ "options": { "source_root": source_root, "targets": ordered, "clean": clean } }),
            )
            .await
            {
                Ok(result) => {
                    let ok = result.exit_status.is_none_or(|c| c == 0);
                    build_ok.set(Some(ok));
                    push_log(if ok {
                        "\n[Bauen erfolgreich.]\n".to_string()
                    } else {
                        format!("\n[Bauen fehlgeschlagen, Code {}.]\n", result.exit_status.unwrap_or_default())
                    });
                }
                Err(e) => {
                    build_ok.set(Some(false));
                    error.set(Some(e.clone()));
                    push_log(format!("\nFehler: {e}\n"));
                }
            }
            building.set(false);
        });
    };

    let deploy_targets = move || -> Vec<String> {
        targets.with(|t| match t {
            Some(t) => selected.with(|sel| t.bins.iter().filter(|x| sel.contains(*x)).cloned().collect()),
            None => vec![],
        })
    };

    let run_deploy = move || {
        deploy_confirming.set(false);
        deploy_confirm_text.set(String::new());
        deploying.set(true);
        error.set(None);
        log_open.set(true);
        let tg = deploy_targets();
        let n = note.get_untracked();
        spawn_local(async move {
            match invoke::<DeployRecord>("run_deploy", &json!({ "targets": tg, "note": n })).await {
                Ok(record) => {
                    last_result.set(Some(record));
                    refresh_history(history).await;
                }
                Err(e) => {
                    error.set(Some(e.clone()));
                    push_log(format!("\nFehler: {e}\n"));
                }
            }
            deploying.set(false);
        });
    };

    let run_rollback = move || {
        let target = rollback_confirming.get_untracked();
        rollback_confirming.set(None);
        rollback_confirm_text.set(String::new());
        rolling_back.set(true);
        error.set(None);
        log_open.set(true);
        let deploy_id = match target {
            Some(RollbackTarget::Record(r)) => Some(r.id),
            _ => None,
        };
        spawn_local(async move {
            let args = match deploy_id {
                Some(id) => json!({ "deployId": id }),
                None => json!({}),
            };
            match invoke::<DeployRecord>("run_rollback", &args).await {
                Ok(record) => {
                    last_result.set(Some(record));
                    refresh_history(history).await;
                }
                Err(e) => {
                    error.set(Some(e.clone()));
                    push_log(format!("\nFehler: {e}\n"));
                }
            }
            rolling_back.set(false);
        });
    };

    // autoFocus der Bestätigungsfelder beim Öffnen des Dialogs.
    let deploy_input = NodeRef::<leptos::html::Input>::new();
    let rollback_input = NodeRef::<leptos::html::Input>::new();
    Effect::new(move |_| {
        if let Some(el) = deploy_input.get() {
            let _ = el.focus();
        }
    });
    Effect::new(move |_| {
        if let Some(el) = rollback_input.get() {
            let _ = el.focus();
        }
    });

    let busy = Signal::derive(move || syncing.get() || building.get() || deploying.get() || rolling_back.get());
    let rollback_target_label = move || match rollback_confirming.get() {
        Some(RollbackTarget::Latest) => "letzter Einspiel-Vorgang".to_string(),
        Some(RollbackTarget::Record(r)) => format!("Eintrag #{} ({})", r.id, format_date(&r.created_at)),
        None => String::new(),
    };

    let settings_fields = SETTING_DEFAULTS
        .iter()
        .map(|(key, _)| {
            let key: &'static str = key;
            view! {
                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                    {key}
                    <input
                        prop:value=move || setting(key)
                        on:input=move |e| save_setting(key, event_target_value(&e))
                        class="w-full rounded-md border border-border bg-background px-2 py-1 font-mono text-xs"
                    />
                </label>
            }
        })
        .collect_view();

    view! {
        <div class="space-y-6 pb-10">
            <div class="flex items-center justify-between">
                <PageHeader module=Section::BuildDeploy title="Server-Quellcode Bauen & Einspielen".to_string()/>
                <Button variant=ButtonVariant::Ghost on_click=move |_| editing_settings.update(|v| *v = !*v)>
                    <Icon name="Settings2" class="size-4"/>
                    "Einstellungen "
                    {move || if editing_settings.get() { "ausblenden" } else { "anzeigen" }}
                </Button>
            </div>

            <div class="flex items-start gap-2 rounded-lg border border-amber-500/40 bg-amber-500/10 p-3 text-sm text-amber-700 dark:text-amber-400">
                <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0"/>
                <span>
                    "Diese Aktion betrifft "<strong>"ALLE Channels und den Login-Server gleichzeitig"</strong>
                    " — sie teilen sich per Symlink eine einzige Programmdatei (live geprüft). Es gibt keinen Testserver und keine Möglichkeit, einen einzelnen Channel isoliert zu testen. Bauen (unten) ist gefahrlos — es läuft ausschließlich in einer separaten Arbeitskopie und rührt den Live-Server nicht an. Nur der \"Jetzt live einspielen\"-Schritt ersetzt echte Programmdateien."
                </span>
            </div>

            <div class="grid grid-cols-1 gap-3 rounded-lg border border-border bg-card p-3 sm:grid-cols-2" class:hidden=move || !editing_settings.get()>
                {settings_fields}
                {move || setting_save_error.get().map(|e| view! { <p class="text-sm text-destructive sm:col-span-2">{e}</p> })}
            </div>

            {move || error.get().map(|e| view! { <p class="whitespace-pre-wrap text-sm text-destructive">{e}</p> })}

            // Schritt 1: Synchronisieren
            <section class="space-y-2 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Schritt 1: Quellcode-Kopie aktualisieren"</h2>
                <p class="text-xs text-muted-foreground">
                    "Ungefährlich — liest nur vom Live-Quellbaum ("{move || setting("build_live_source_root")}"), schreibt nur in die separate Arbeitskopie ("
                    {move || setting("build_scratch_source_root")}")."
                </p>
                <Button variant=ButtonVariant::Outline on_click=move |_| run_sync() disabled=busy>
                    <Icon name="RefreshCw" class=Signal::derive(move || if syncing.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                    {move || if syncing.get() { "Synchronisiere…" } else { "Quellcode-Kopie aktualisieren" }}
                </Button>
            </section>

            // Schritt 2: Bauen
            <section class="space-y-3 rounded-lg border border-border p-4">
                <h2 class="text-sm font-medium text-muted-foreground">"Schritt 2: Bauen (Arbeitskopie — Live-Server bleibt unberührt)"</h2>
                {move || targets.get().map(|t| view! {
                    <div class="flex flex-wrap gap-3">
                        {t.libs.iter().chain(t.bins.iter()).cloned().map(|name| {
                            let n1 = name.clone();
                            let n2 = name.clone();
                            view! {
                                <label class="flex items-center gap-1.5 text-sm">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || selected.with(|s| s.contains(&n1))
                                        on:change=move |_| toggle_target(n2.clone())
                                    />
                                    {name}
                                </label>
                            }
                        }).collect_view()}
                    </div>
                })}
                <label class="flex items-center gap-1.5 text-sm">
                    <input type="checkbox" prop:checked=move || clean_build.get() on:change=move |e| clean_build.set(event_target_checked(&e))/>
                    "Sauber neu bauen (gmake clean vor jedem Ziel)"
                </label>
                <Show when=move || targets.with(|t| t.as_ref().is_some_and(|t| selected.with(|s| s.len() < t.libs.len() + t.bins.len())))>
                    <p class="flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400">
                        <Icon name="AlertTriangle" class="size-3.5 shrink-0"/>
                        "Bei Teil-Builds können veraltete .a-Bibliotheken verlinkt werden — im Zweifel alles auswählen."
                    </p>
                </Show>
                <Button on_click=move |_| run_build() disabled=Signal::derive(move || busy.get() || selected.with(|s| s.is_empty()))>
                    <Icon name="Hammer" class="size-4"/>
                    {move || if building.get() { "Baue…" } else { "Bauen (Arbeitskopie — Live-Server bleibt unberührt)" }}
                </Button>

                <Show when=move || build_ok.get() == Some(true)>
                    <div class="space-y-2 rounded-md border border-green-500/40 bg-green-500/10 p-3">
                        <p class="flex items-center gap-1 text-sm text-green-700 dark:text-green-400">
                            <Icon name="CheckCircle2" class="size-4"/>
                            " Bauen erfolgreich."
                        </p>
                        {move || if deploy_targets().is_empty() {
                            view! {
                                <p class="text-xs text-muted-foreground">
                                    "Keine Programmdatei (\"game\"/\"db\") ausgewählt — nur Bibliotheken gebaut, nichts zum Einspielen."
                                </p>
                            }.into_any()
                        } else {
                            view! {
                                <label class="flex flex-col gap-1 text-xs text-muted-foreground">
                                    "Was wurde geändert? (wird im Verlauf gespeichert)"
                                    <input
                                        prop:value=move || note.get()
                                        on:input=move |e| note.set(event_target_value(&e))
                                        class=MODAL_INPUT
                                    />
                                </label>
                                <Button variant=ButtonVariant::Destructive on_click=move |_| deploy_confirming.set(true) disabled=busy>
                                    <Icon name="UploadCloud" class="size-4"/>
                                    "Jetzt live einspielen…"
                                </Button>
                            }.into_any()
                        }}
                    </div>
                </Show>
                <Show when=move || build_ok.get() == Some(false)>
                    <p class="flex items-center gap-1 text-sm text-destructive">
                        <Icon name="XCircle" class="size-4"/>
                        " Bauen fehlgeschlagen — Log prüfen, Live-Server wurde nicht angefasst."
                    </p>
                </Show>
            </section>

            // Ergebnis + Rollback
            {move || last_result.get().map(|r| {
                let rr = r.clone();
                view! {
                    <section class="space-y-2 rounded-lg border border-border p-4">
                        <h2 class="text-sm font-medium text-muted-foreground">"Letztes Ergebnis"</h2>
                        {match r.success {
                            Some(true) => view! {
                                <p class="flex items-center gap-1 text-sm text-green-700 dark:text-green-400">
                                    <Icon name="CheckCircle2" class="size-4"/>
                                    {format!(" Live-Prüfung erfolgreich ({}, #{}).", r.kind, r.id)}
                                </p>
                            }.into_any(),
                            Some(false) => view! {
                                <p class="flex items-center gap-1 text-sm text-destructive">
                                    <Icon name="XCircle" class="size-4"/>
                                    {format!(" Live-Prüfung fehlgeschlagen — Server evtl. nicht sauber gestartet (#{}).", r.id)}
                                </p>
                            }.into_any(),
                            None => view! { <p class="text-sm text-muted-foreground">"Live-Prüfung läuft noch oder unbekannt."</p> }.into_any(),
                        }}
                        <Button
                            variant=ButtonVariant::Destructive
                            on_click=move |_| rollback_confirming.set(Some(RollbackTarget::Record(rr.clone())))
                            disabled=busy
                        >
                            <Icon name="RotateCcw" class="size-4"/>
                            {move || if rolling_back.get() { "Rolle zurück…" } else { "Rückgängig machen" }}
                        </Button>
                    </section>
                }
            })}

            // Verlauf
            <section class="space-y-2 rounded-lg border border-border p-4">
                <div class="flex items-center justify-between">
                    <h2 class="text-sm font-medium text-muted-foreground">"Verlauf"</h2>
                    <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| spawn_local(refresh_history(history))>
                        <Icon name="RefreshCw" class="size-3.5"/>
                        " Neu laden"
                    </Button>
                </div>
                <Show when=move || history.with(|h| h.is_empty())>
                    <p class="text-sm text-muted-foreground">"Noch nichts eingespielt."</p>
                </Show>
                <div class="space-y-1">
                    <For each=move || history.get() key=|h| h.id let:h>
                        {
                            let can_rollback = h.kind == "deploy" && (h.game_backup_path.is_some() || h.db_backup_path.is_some());
                            let rec = h.clone();
                            view! {
                                <div class="flex items-center justify-between gap-2 rounded-md border border-border p-2 text-sm">
                                    <div class="min-w-0 flex-1">
                                        <div class="flex items-center gap-2">
                                            <span class="font-medium">
                                                {format!("#{} {}", h.id, if h.kind == "deploy" { "Einspielen" } else { "Rückgängig-machen" })}
                                            </span>
                                            <span class="text-xs text-muted-foreground">{format_date(&h.created_at)}</span>
                                            <span class="text-xs text-muted-foreground">{h.targets.join(", ")}</span>
                                            {(h.success == Some(true)).then(|| view! { <Icon name="CheckCircle2" class="size-3.5 text-green-600"/> })}
                                            {(h.success == Some(false)).then(|| view! { <Icon name="XCircle" class="size-3.5 text-destructive"/> })}
                                        </div>
                                        {h.note.clone().filter(|n| !n.is_empty()).map(|n| view! { <p class="truncate text-xs text-muted-foreground">{n}</p> })}
                                    </div>
                                    {can_rollback.then(|| view! {
                                        <Button
                                            variant=ButtonVariant::Outline
                                            size=ButtonSize::Sm
                                            on_click=move |_| rollback_confirming.set(Some(RollbackTarget::Record(rec.clone())))
                                            disabled=busy
                                        >
                                            "Auf diesen Stand zurücksetzen"
                                        </Button>
                                    })}
                                </div>
                            }
                        }
                    </For>
                </div>
            </section>

            // Log-Modal
            <div class="fixed inset-0 flex items-center justify-center bg-black/50" class:hidden=move || !log_open.get()>
                <div class="flex max-h-[70vh] w-[42rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                    <div class="flex items-center justify-between">
                        <p class="text-sm font-medium">"Ausgabe"</p>
                        <div class="flex items-center gap-2">
                            <Button
                                variant=ButtonVariant::Ghost
                                size=ButtonSize::Sm
                                on_click=move |_| log.set(vec![])
                                disabled=Signal::derive(move || log.with(|l| l.is_empty()))
                            >
                                <Icon name="Eraser" class="size-3.5"/>
                                " Leeren"
                            </Button>
                            <Button variant=ButtonVariant::Ghost size=ButtonSize::Sm on_click=move |_| log_open.set(false) disabled=busy>
                                "Schließen"
                            </Button>
                        </div>
                    </div>
                    <pre
                        node_ref=log_ref
                        class="min-h-0 flex-1 overflow-auto whitespace-pre-wrap rounded-md border border-border bg-muted/30 p-2 font-mono text-xs"
                    >
                        {move || log.with(|l| if l.is_empty() { "…".to_string() } else { l.concat() })}
                    </pre>
                </div>
            </div>

            // Einspiel-Bestätigung
            <Show when=move || deploy_confirming.get()>
                <div class="fixed inset-0 z-10 flex items-center justify-center bg-black/50">
                    <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm font-medium">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            "Live-Server ersetzen"
                        </p>
                        <p class="text-sm text-muted-foreground">
                            "Dies stoppt Server/Channel + Login-Server, sichert die aktuelle(n) Programmdatei(en) ("
                            {move || deploy_targets().join(", ")}
                            ") und ersetzt sie live. Danach wird automatisch neu gestartet."
                        </p>
                        <p class="text-sm">
                            "Tippe zum Bestätigen exakt: "<code class="rounded bg-muted px-1">{DEPLOY_PHRASE}</code>
                        </p>
                        <input
                            node_ref=deploy_input
                            prop:value=move || deploy_confirm_text.get()
                            on:input=move |e| deploy_confirm_text.set(event_target_value(&e))
                            class=MODAL_INPUT
                        />
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| deploy_confirming.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button
                                variant=ButtonVariant::Destructive
                                disabled=Signal::derive(move || deploy_confirm_text.get() != DEPLOY_PHRASE)
                                on_click=move |_| run_deploy()
                            >
                                "Einspielen"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>

            // Rückgängig-machen-Bestätigung
            <Show when=move || rollback_confirming.with(|r| r.is_some())>
                <div class="fixed inset-0 z-10 flex items-center justify-center bg-black/50">
                    <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm font-medium">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            "Rückgängig machen"
                        </p>
                        <p class="text-sm text-muted-foreground">
                            "Stellt die gesicherte(n) Programmdatei(en) von "
                            {rollback_target_label}
                            " wieder her und startet den Server neu."
                        </p>
                        <p class="text-sm">
                            "Tippe zum Bestätigen exakt: "<code class="rounded bg-muted px-1">{ROLLBACK_PHRASE}</code>
                        </p>
                        <input
                            node_ref=rollback_input
                            prop:value=move || rollback_confirm_text.get()
                            on:input=move |e| rollback_confirm_text.set(event_target_value(&e))
                            class=MODAL_INPUT
                        />
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| rollback_confirming.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button
                                variant=ButtonVariant::Destructive
                                disabled=Signal::derive(move || rollback_confirm_text.get() != ROLLBACK_PHRASE)
                                on_click=move |_| run_rollback()
                            >
                                "Zurückrollen"
                            </Button>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    }
}
