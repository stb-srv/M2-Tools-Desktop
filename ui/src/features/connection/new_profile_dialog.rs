//! Legt ein neues Server-Profil an: erst nur der Name, dann SSH, dann MySQL.
//! Am Ende wird das Profil gespeichert und sofort zur aktiven Verbindung.

use super::fields::{MysqlFields, MysqlTestButton, SshFields, SshTestButton, TextField};
use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::util::connection::{mysql_complete, ssh_complete, MysqlValues, SshValues};
use crate::store::connection_profiles::connection_profiles;
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    Name,
    Ssh,
    Mysql,
}

const STEPS: [(Step, &str); 3] = [(Step::Name, "Name"), (Step::Ssh, "SSH"), (Step::Mysql, "Datenbank")];

/// Prüft, ob es schon ein Profil mit diesem Namen gibt (ohne Groß/Klein).
pub fn name_taken<'a>(mut existing: impl Iterator<Item = &'a str>, trimmed: &str) -> bool {
    let lower = trimmed.to_lowercase();
    existing.any(|n| n.to_lowercase() == lower)
}

#[component]
pub fn NewProfileDialog(#[prop(into)] on_close: Callback<()>) -> impl IntoView {
    let store = connection_profiles();
    let step_index = RwSignal::new(0usize);
    let step = move || STEPS[step_index.get()].0;
    let name = RwSignal::new(String::new());
    let ssh = RwSignal::new(SshValues::default());
    let mysql = RwSignal::new(MysqlValues::default());
    let saving = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let trimmed_name = move || name.with(|n| n.trim().to_string());
    let is_name_taken = move || {
        let t = trimmed_name();
        store.profiles.with(|p| name_taken(p.iter().map(|x| x.name.as_str()), &t))
    };
    let can_continue = move || match step() {
        Step::Name => !trimmed_name().is_empty() && !is_name_taken(),
        Step::Ssh => ssh.with(ssh_complete),
        Step::Mysql => mysql.with(mysql_complete),
    };

    let next = move |_| {
        // Datenbank liegt meistens auf demselben Server - Host als Vorschlag übernehmen.
        if step_index.get_untracked() == 1 && mysql.with_untracked(|m| m.host.is_empty()) {
            let host = ssh.with_untracked(|s| s.host.clone());
            mysql.update(|m| m.host = host);
        }
        step_index.update(|i| *i += 1);
    };

    let finish = move |_| {
        saving.set(true);
        error.set(None);
        let n = trimmed_name();
        let s = ssh.get_untracked();
        let m = mysql.get_untracked();
        spawn_local(async move {
            match store.create(&n, &s, &m).await {
                Ok(()) => {
                    toast::success(&format!("Profil „{n}“ angelegt und aktiviert"));
                    saving.set(false);
                    on_close.run(());
                }
                Err(e) => {
                    error.set(Some(e));
                    saving.set(false);
                }
            }
        });
    };

    let step_header = STEPS
        .iter()
        .enumerate()
        .map(|(i, (_, label))| {
            view! {
                <div class="flex-1 space-y-1">
                    <div class=move || if i <= step_index.get() { "h-1 rounded-full bg-primary" } else { "h-1 rounded-full bg-muted" }></div>
                    <div class=move || {
                        if i == step_index.get() { "text-[11px] font-medium text-foreground" } else { "text-[11px] text-muted-foreground" }
                    }>{format!("{}. {}", i + 1, label)}</div>
                </div>
            }
        })
        .collect_view();

    view! {
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
            <div class="flex max-h-[90vh] w-[30rem] flex-col rounded-lg border border-border bg-card">
                <div class="flex items-center justify-between border-b border-border px-4 py-3">
                    <div class="flex items-center gap-2">
                        <Icon name="Server" class="size-4 text-primary"/>
                        <h2 class="text-sm font-semibold">"Neues Server-Profil"</h2>
                    </div>
                    <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| on_close.run(()) title="Abbrechen">
                        <Icon name="X" class="size-4"/>
                    </Button>
                </div>

                <div class="flex gap-1 px-4 pt-3">{step_header}</div>

                <div class="space-y-3 overflow-y-auto px-4 py-4">
                    <Show when=move || step() == Step::Name>
                        <p class="text-sm text-muted-foreground">
                            "Wie soll der Server heißen? Unter diesem Namen wählst du ihn später auf der Startseite aus."
                        </p>
                        <TextField
                            label="Profilname"
                            value=name
                            on_change=move |v: String| name.set(v)
                            placeholder="z.B. Live-Server oder Testserver"
                            auto_focus=true
                        />
                        <Show when=is_name_taken>
                            <p class="text-sm text-destructive">"Ein Profil mit diesem Namen gibt es schon."</p>
                        </Show>
                    </Show>

                    <Show when=move || step() == Step::Ssh>
                        <p class="text-sm text-muted-foreground">
                            "SSH-Zugang zum Gameserver - für Server-Steuerung, Logs, Quests und alle Dateien auf dem Server."
                        </p>
                        <SshFields value=ssh/>
                        <SshTestButton value=ssh/>
                    </Show>

                    <Show when=move || step() == Step::Mysql>
                        <p class="text-sm text-muted-foreground">
                            "MySQL-Zugang - für Item-, Shop- und Mob-Editor, Accounts und die Statistiken auf der Startseite."
                        </p>
                        <MysqlFields value=mysql/>
                        <MysqlTestButton value=mysql/>
                    </Show>

                    {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                </div>

                <div class="flex justify-between border-t border-border px-4 py-3">
                    <Button
                        variant=ButtonVariant::Ghost
                        on_click=move |_| {
                            if step_index.get_untracked() == 0 {
                                on_close.run(())
                            } else {
                                step_index.update(|i| *i -= 1)
                            }
                        }
                        disabled=saving
                    >
                        {move || if step_index.get() == 0 { "Abbrechen" } else { "Zurück" }}
                    </Button>
                    {move || {
                        if step_index.get() < STEPS.len() - 1 {
                            view! {
                                <Button on_click=next disabled=Signal::derive(move || !can_continue())>
                                    "Weiter"
                                </Button>
                            }
                            .into_any()
                        } else {
                            view! {
                                <Button on_click=finish disabled=Signal::derive(move || !can_continue() || saving.get())>
                                    {move || if saving.get() { "Speichere…" } else { "Profil anlegen & verbinden" }}
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

#[cfg(test)]
mod tests {
    use super::name_taken;

    #[test]
    fn name_taken_ignores_case() {
        let names = ["Live-Server", "Test"];
        assert!(name_taken(names.iter().copied(), "live-server"));
        assert!(!name_taken(names.iter().copied(), "Neu"));
    }
}
