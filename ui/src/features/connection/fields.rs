//! Gemeinsame Eingabefelder für SSH/MySQL - genutzt vom "Neues Profil"-Dialog,
//! den Einstellungen (Verbindung) und dem Setup-Assistenten.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::lib::connection::{mysql_complete, ssh_complete, test_mysql, test_ssh, MysqlValues, SshAuthMode, SshValues};
use crate::tauri::{dialog_open_one, OpenDialogOptions};
use leptos::html;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn TextField(
    #[prop(into)] label: String,
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(into, optional)] placeholder: Option<String>,
    #[prop(into, optional)] class: Option<String>,
    #[prop(optional)] auto_focus: bool,
) -> impl IntoView {
    let input_ref = NodeRef::<html::Input>::new();
    if auto_focus {
        Effect::new(move |_| {
            if let Some(el) = input_ref.get() {
                let _ = el.focus();
            }
        });
    }
    view! {
        <label class=format!("block space-y-1 {}", class.unwrap_or_default())>
            <span class="text-xs font-medium text-muted-foreground">{label}</span>
            <input
                node_ref=input_ref
                prop:value=move || value.get()
                on:input=move |e| on_change.run(event_target_value(&e))
                placeholder=placeholder
                class="w-full rounded-md border border-border bg-background px-3 py-2 text-sm"
            />
        </label>
    }
}

/// Passwortfeld mit Auge-Knopf (gleiches Markup wie `PasswordInput`, aber mit
/// Wert + Änderungs-Callback statt eigenem Signal).
#[component]
pub fn PasswordField(
    #[prop(into)] value: Signal<String>,
    #[prop(into)] on_change: Callback<String>,
    #[prop(into, optional)] placeholder: Option<String>,
) -> impl IntoView {
    let visible = RwSignal::new(false);
    view! {
        <div class="relative ">
            <input
                type=move || if visible.get() { "text" } else { "password" }
                prop:value=move || value.get()
                on:input=move |e| on_change.run(event_target_value(&e))
                placeholder=placeholder
                class="w-full rounded-md border border-border bg-background px-3 py-2 pr-8 text-sm"
            />
            <button
                type="button"
                tabindex="-1"
                on:click=move |_| visible.update(|v| *v = !*v)
                class="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            >
                <Icon name=Signal::derive(move || if visible.get() { "EyeOff" } else { "Eye" }) class="size-3.5"/>
            </button>
        </div>
    }
}

#[component]
pub fn SshFields(value: RwSignal<SshValues>) -> impl IntoView {
    let pick_key = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some("Privaten SSH-Schlüssel auswählen".into()),
                multiple: false,
                ..Default::default()
            };
            if let Ok(Some(selected)) = dialog_open_one(&opts).await {
                value.update(|v| v.key_path = selected);
            }
        });
    };
    let mode_button = move |mode: SshAuthMode| {
        view! {
            <button
                type="button"
                on:click=move |_| value.update(|v| v.auth_mode = mode)
                class=move || {
                    if value.with(|v| v.auth_mode == mode) {
                        "flex-1 px-3 py-1 bg-primary text-primary-foreground"
                    } else {
                        "flex-1 px-3 py-1"
                    }
                }
            >
                {if mode == SshAuthMode::Password { "Passwort" } else { "SSH-Schlüssel" }}
            </button>
        }
    };
    view! {
        <div class="space-y-2">
            <div class="grid grid-cols-3 gap-2">
                <TextField
                    label="Host / IP"
                    value=Signal::derive(move || value.with(|v| v.host.clone()))
                    on_change=move |s: String| value.update(|v| v.host = s)
                    placeholder="z.B. 192.168.0.10"
                    class="col-span-2"
                />
                <TextField
                    label="Port"
                    value=Signal::derive(move || value.with(|v| v.port.clone()))
                    on_change=move |s: String| value.update(|v| v.port = s)
                    placeholder="22"
                />
            </div>
            <TextField
                label="Benutzername"
                value=Signal::derive(move || value.with(|v| v.username.clone()))
                on_change=move |s: String| value.update(|v| v.username = s)
            />

            <div class="space-y-1">
                <span class="text-xs font-medium text-muted-foreground">"Anmeldung"</span>
                <div class="flex overflow-hidden rounded-md border border-border text-sm">
                    {mode_button(SshAuthMode::Password)}
                    {mode_button(SshAuthMode::Key)}
                </div>
            </div>

            {move || {
                if value.with(|v| v.auth_mode == SshAuthMode::Password) {
                    view! {
                        <PasswordField
                            value=Signal::derive(move || value.with(|v| v.password.clone()))
                            on_change=move |s: String| value.update(|v| v.password = s)
                            placeholder="Passwort"
                        />
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="flex items-center gap-2">
                            <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=pick_key class="shrink-0">
                                <Icon name="FolderOpen" class="size-4"/>
                                "Schlüsseldatei"
                            </Button>
                            <span class="truncate text-sm text-muted-foreground">
                                {move || {
                                    let p = value.with(|v| v.key_path.clone());
                                    if p.is_empty() { "Keine Datei ausgewählt".to_string() } else { p }
                                }}
                            </span>
                        </div>
                        <PasswordField
                            value=Signal::derive(move || value.with(|v| v.key_passphrase.clone()))
                            on_change=move |s: String| value.update(|v| v.key_passphrase = s)
                            placeholder="Passphrase (falls vorhanden)"
                        />
                    }
                    .into_any()
                }
            }}
        </div>
    }
}

#[component]
pub fn MysqlFields(value: RwSignal<MysqlValues>) -> impl IntoView {
    view! {
        <div class="space-y-2">
            <div class="grid grid-cols-3 gap-2">
                <TextField
                    label="Host / IP"
                    value=Signal::derive(move || value.with(|v| v.host.clone()))
                    on_change=move |s: String| value.update(|v| v.host = s)
                    placeholder="meist wie SSH-Host"
                    class="col-span-2"
                />
                <TextField
                    label="Port"
                    value=Signal::derive(move || value.with(|v| v.port.clone()))
                    on_change=move |s: String| value.update(|v| v.port = s)
                    placeholder="3306"
                />
            </div>
            <TextField
                label="Benutzername"
                value=Signal::derive(move || value.with(|v| v.username.clone()))
                on_change=move |s: String| value.update(|v| v.username = s)
            />
            <PasswordField
                value=Signal::derive(move || value.with(|v| v.password.clone()))
                on_change=move |s: String| value.update(|v| v.password = s)
                placeholder="Passwort"
            />
        </div>
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TestState {
    Idle,
    Testing,
    Ok,
    Error,
}

/// Status + Fehlertext eines Verbindungstests. Ein altes "klappt" darf nach
/// einer Änderung an den Feldern nicht stehen bleiben: sobald sich `values`
/// ändert, geht der Zustand zurück auf `Idle`.
#[derive(Clone, Copy)]
pub struct ConnectionTest {
    pub state: RwSignal<TestState>,
    pub error: RwSignal<Option<String>>,
}

impl ConnectionTest {
    pub fn new<T: Clone + PartialEq + Send + Sync + 'static>(values: Signal<T>) -> Self {
        let state = RwSignal::new(TestState::Idle);
        let error = RwSignal::new(None::<String>);
        let fingerprint = Memo::new(move |_| values.get());
        Effect::new(move |_| {
            fingerprint.track();
            state.set(TestState::Idle);
            error.set(None);
        });
        Self { state, error }
    }

    /// Führt `run` aus und setzt Status/Fehler entsprechend.
    pub fn run<F>(&self, fut: F)
    where
        F: std::future::Future<Output = Result<(), String>> + 'static,
    {
        let (state, error) = (self.state, self.error);
        state.set(TestState::Testing);
        error.set(None);
        spawn_local(async move {
            match fut.await {
                Ok(()) => state.set(TestState::Ok),
                Err(e) => {
                    state.set(TestState::Error);
                    error.set(Some(e));
                }
            }
        });
    }
}

#[component]
pub fn TestResult(#[prop(into)] state: Signal<TestState>, #[prop(into)] error: Signal<Option<String>>) -> impl IntoView {
    move || match state.get() {
        TestState::Ok => view! {
            <span class="flex items-center gap-1 text-sm text-green-600">
                <Icon name="CheckCircle2" class="size-4"/>
                "Verbindung klappt"
            </span>
        }
        .into_any(),
        TestState::Error => view! {
            <span class="flex min-w-0 items-start gap-1 text-sm text-destructive">
                <Icon name="XCircle" class="mt-0.5 size-4 shrink-0"/>
                <span class="line-clamp-3">{move || error.get().unwrap_or_default()}</span>
            </span>
        }
        .into_any(),
        _ => ().into_any(),
    }
}

#[component]
pub fn SshTestButton(#[prop(into)] value: Signal<SshValues>) -> impl IntoView {
    let t = ConnectionTest::new(value);
    view! {
        <div class="flex items-start gap-3">
            <Button
                variant=ButtonVariant::Outline
                size=ButtonSize::Sm
                on_click=move |_| {
                    let v = value.get_untracked();
                    t.run(async move { test_ssh(&v).await });
                }
                disabled=Signal::derive(move || !value.with(ssh_complete) || t.state.get() == TestState::Testing)
                class="shrink-0"
            >
                {move || if t.state.get() == TestState::Testing { "Teste…" } else { "Verbindung testen" }}
            </Button>
            <TestResult state=t.state error=t.error/>
        </div>
    }
}

#[component]
pub fn MysqlTestButton(#[prop(into)] value: Signal<MysqlValues>) -> impl IntoView {
    let t = ConnectionTest::new(value);
    view! {
        <div class="flex items-start gap-3">
            <Button
                variant=ButtonVariant::Outline
                size=ButtonSize::Sm
                on_click=move |_| {
                    let v = value.get_untracked();
                    t.run(async move { test_mysql(&v).await });
                }
                disabled=Signal::derive(move || !value.with(mysql_complete) || t.state.get() == TestState::Testing)
                class="shrink-0"
            >
                {move || if t.state.get() == TestState::Testing { "Teste…" } else { "Verbindung testen" }}
            </Button>
            <TestResult state=t.state error=t.error/>
        </div>
    }
}
