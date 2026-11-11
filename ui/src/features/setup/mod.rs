//! Erststart-Assistent: Server-Profil, SSH, MySQL, Client-Ordner, Client vorbereiten.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::features::connection::{MysqlFields, MysqlTestButton, SshFields, SshTestButton, TestResult, TestState, TextField};
use crate::features::shared::client_unpack_panel::{load_unpack_status, ClientUnpackPanel};
use crate::util::connection::{get_setting, set_setting, ssh_complete, MysqlValues, SshValues};
use crate::store::connection_profiles::connection_profiles;
use crate::tauri::{dialog_open_one, invoke, OpenDialogOptions};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    Profile,
    Ssh,
    Mysql,
    Client,
    Prepare,
}

const STEPS: [Step; 5] = [Step::Profile, Step::Ssh, Step::Mysql, Step::Client, Step::Prepare];

fn step_label(s: Step) -> &'static str {
    match s {
        Step::Profile => "Server-Profil",
        Step::Ssh => "SSH-Verbindung",
        Step::Mysql => "MySQL-Verbindung",
        Step::Client => "Client-Ordner",
        Step::Prepare => "Client vorbereiten",
    }
}

/// Erststart-Assistent. `on_complete` wird nach erfolgreicher Einrichtung aufgerufen.
#[component]
pub fn SetupWizard(#[prop(into)] on_complete: Callback<()>) -> impl IntoView {
    let step_index = RwSignal::new(0usize);
    let step = move || STEPS[step_index.get()];

    let profile_name = RwSignal::new(String::new());
    let ssh = RwSignal::new(SshValues::default());
    let mysql = RwSignal::new(MysqlValues::default());

    let client_path = RwSignal::new(String::new());
    let client_test = RwSignal::new(TestState::Idle);
    let client_error = RwSignal::new(None::<String>);

    let finishing = RwSignal::new(false);
    let finish_error = RwSignal::new(None::<String>);

    let pick_client_path = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                directory: true,
                title: Some("Metin2-Client-Ordner auswählen".into()),
                ..Default::default()
            };
            if let Ok(Some(selected)) = dialog_open_one(&opts).await {
                client_path.set(selected);
                client_test.set(TestState::Idle);
            }
        });
    };

    let test_client_path = move |_| {
        client_test.set(TestState::Testing);
        client_error.set(None);
        let path = client_path.get_untracked();
        spawn_local(async move {
            match invoke::<bool>("check_client_path", &json!({ "path": path })).await {
                Ok(true) => client_test.set(TestState::Ok),
                Ok(false) => {
                    client_test.set(TestState::Error);
                    client_error.set(Some("granny2.dll wurde in diesem Ordner nicht gefunden.".into()));
                }
                Err(e) => {
                    client_test.set(TestState::Error);
                    client_error.set(Some(e));
                }
            }
        });
    };

    let next = move |_| {
        if STEPS[step_index.get_untracked()] == Step::Ssh && mysql.with_untracked(|m| m.host.is_empty()) {
            let host = ssh.with_untracked(|s| s.host.clone());
            mysql.update(|m| m.host = host);
        }
        step_index.update(|i| *i += 1);
    };

    let finish = move |_| {
        finishing.set(true);
        finish_error.set(None);
        let name = profile_name.get_untracked().trim().to_string();
        let s = ssh.get_untracked();
        let m = mysql.get_untracked();
        let client = client_path.get_untracked();
        spawn_local(async move {
            let res: Result<(), String> = async {
                // Ohne SSH-Daten kein Profil - das lässt sich später auf der Startseite anlegen.
                if !name.is_empty() && ssh_complete(&s) {
                    connection_profiles().create(&name, &s, &m).await?;
                }
                if !client.is_empty() {
                    set_setting("client_path", &client).await?;
                    // EterPack aus dem pack-Ordner übernehmen, falls noch keins eingestellt ist.
                    if get_setting("eterpack_tool_path").await.unwrap_or_default().is_empty() {
                        if let Ok(Some(tool)) = load_unpack_status(&client).await.map(|s| s.tool_path) {
                            set_setting("eterpack_tool_path", &tool).await?;
                        }
                    }
                }
                set_setting("setup_completed", "true").await
            }
            .await;
            finishing.set(false);
            match res {
                Ok(()) => on_complete.run(()),
                Err(e) => finish_error.set(Some(e)),
            }
        });
    };

    let progress = STEPS
        .iter()
        .enumerate()
        .map(|(i, _)| {
            view! {
                <div class=move || if i <= step_index.get() { "h-1 flex-1 rounded-full bg-primary" } else { "h-1 flex-1 rounded-full bg-muted" }></div>
            }
        })
        .collect_view();

    view! {
        <div class="flex h-screen items-center justify-center bg-background p-6">
            <div class="w-full max-w-lg space-y-6 rounded-lg border border-border bg-card p-6">
                <div>
                    <h1 class="text-xl font-semibold">"Willkommen bei M2Manager"</h1>
                    <p class="text-sm text-muted-foreground">
                        {move || format!(
                            "Ein paar Angaben, bevor es losgeht ({}/{}: {})",
                            step_index.get() + 1,
                            STEPS.len(),
                            step_label(step())
                        )}
                    </p>
                </div>

                <div class="flex gap-1">{progress}</div>

                <Show when=move || step() == Step::Profile>
                    <div class="space-y-3">
                        <p class="text-sm text-muted-foreground">
                            "Gib deinem Server einen Namen. Unter diesem Profil speichert M2Manager die Zugangsdaten - weitere Server legst du später direkt auf der Startseite an."
                        </p>
                        <TextField
                            label="Profilname"
                            value=profile_name
                            on_change=move |v: String| profile_name.set(v)
                            placeholder="z.B. Live-Server oder Testserver"
                            auto_focus=true
                        />
                    </div>
                </Show>

                <Show when=move || step() == Step::Ssh>
                    <div class="space-y-3">
                        <p class="text-sm text-muted-foreground">
                            "SSH-Zugang zum Gameserver (Server starten/stoppen, Logs, Quests, Server-Dateien)."
                        </p>
                        <SshFields value=ssh/>
                        <SshTestButton value=ssh/>
                    </div>
                </Show>

                <Show when=move || step() == Step::Mysql>
                    <div class="space-y-3">
                        <p class="text-sm text-muted-foreground">"MySQL-Zugang (Item-, Shop- und Mob-Editor, Accounts, Statistiken)."</p>
                        <MysqlFields value=mysql/>
                        <MysqlTestButton value=mysql/>
                    </div>
                </Show>

                <Show when=move || step() == Step::Client>
                    <div class="space-y-3">
                        <p class="text-sm text-muted-foreground">
                            "Der Ordner deines Metin2-Clients (dort liegt "
                            <code>"granny2.dll"</code>
                            ") - für 3D-Modelle und Item-Icons. Kannst du auch später in den Einstellungen setzen."
                        </p>
                        <div class="flex items-center gap-2">
                            <Button variant=ButtonVariant::Outline on_click=pick_client_path>
                                <Icon name="FolderOpen" class="size-4"/>
                                "Ordner auswählen"
                            </Button>
                            <span class="truncate text-sm text-muted-foreground">
                                {move || {
                                    let p = client_path.get();
                                    if p.is_empty() { "Kein Ordner ausgewählt".to_string() } else { p }
                                }}
                            </span>
                        </div>
                        <div class="flex items-start gap-3">
                            <Button
                                variant=ButtonVariant::Outline
                                size=ButtonSize::Sm
                                on_click=test_client_path
                                disabled=Signal::derive(move || client_path.with(|p| p.is_empty()) || client_test.get() == TestState::Testing)
                            >
                                {move || if client_test.get() == TestState::Testing { "Prüfe…" } else { "Prüfen" }}
                            </Button>
                            {move || {
                                if client_test.get() == TestState::Ok {
                                    view! { <span class="text-sm text-green-600">"granny2.dll gefunden"</span> }.into_any()
                                } else {
                                    view! { <TestResult state=client_test error=client_error/> }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </Show>

                <Show when=move || step() == Step::Prepare>
                    <div class="space-y-3">
                        <p class="text-sm text-muted-foreground">
                            "Ein frisch installierter Client enthält Icons, Modelle und Texte nur gepackt. M2Manager kann sie hier mit "
                            <code>"EterPackConsoleLz4.exe"</code>
                            " entpacken, damit Item-Icons und Modelle sofort angezeigt werden. Geht auch später unter Einstellungen › Client & Werkzeuge."
                        </p>
                        {move || {
                            if client_path.with(|p| p.is_empty()) {
                                view! { <p class="text-sm text-muted-foreground">"Kein Client-Ordner gewählt - dieser Schritt wird übersprungen."</p> }.into_any()
                            } else {
                                view! { <ClientUnpackPanel client_path=Signal::derive(move || client_path.get())/> }.into_any()
                            }
                        }}
                    </div>
                </Show>

                {move || finish_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

                <div class="flex justify-between pt-2">
                    <Button
                        variant=ButtonVariant::Ghost
                        on_click=move |_| step_index.update(|i| *i = i.saturating_sub(1))
                        disabled=Signal::derive(move || step_index.get() == 0 || finishing.get())
                    >
                        "Zurück"
                    </Button>
                    {move || {
                        if step_index.get() < STEPS.len() - 1 {
                            view! {
                                <Button
                                    on_click=next
                                    disabled=Signal::derive(move || step() == Step::Profile && profile_name.with(|n| n.trim().is_empty()))
                                >
                                    "Weiter"
                                </Button>
                            }
                            .into_any()
                        } else {
                            view! {
                                <Button on_click=finish disabled=finishing>
                                    {move || if finishing.get() { "Speichere…" } else { "Fertig" }}
                                </Button>
                            }
                            .into_any()
                        }
                    }}
                </div>
            </div>
        </div>
    }
}
