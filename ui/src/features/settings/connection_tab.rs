use super::shared::SettingsCard;
use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::features::connection::fields::ConnectionTest;
use crate::features::connection::{MysqlFields, NewProfileDialog, SshFields, TestResult, TestState};
use crate::lib::connection::{
    connect_mysql, load_stored_connection, mysql_complete, save_mysql, save_ssh, ssh_complete, test_mysql, test_ssh,
    MysqlValues, SshValues,
};
use crate::store::connection_profiles::{connection_profiles, ConnectionProfile};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[component]
pub fn ConnectionTab() -> impl IntoView {
    let store = connection_profiles();
    let dialog_open = RwSignal::new(false);
    let busy_id = RwSignal::new(None::<i64>);
    let confirm_delete_id = RwSignal::new(None::<i64>);

    let ssh = RwSignal::new(SshValues::default());
    let mysql = RwSignal::new(MysqlValues::default());

    spawn_local(async move {
        let _ = store.reload().await;
    });

    // Nach jedem Server-Wechsel (auch von der Startseite aus) die Felder neu befüllen.
    Effect::new(move |_| {
        store.revision.track();
        spawn_local(async move {
            let (s, m) = load_stored_connection().await;
            ssh.set(s);
            mysql.set(m);
        });
    });

    let on_activate = move |profile: ConnectionProfile| {
        busy_id.set(Some(profile.id));
        spawn_local(async move {
            match store.activate(profile.id).await {
                Ok(()) => toast::success(&format!("Verbunden mit „{}“", profile.name)),
                Err(e) => toast::error(&e),
            }
            busy_id.set(None);
        });
    };

    let on_delete = move |profile: ConnectionProfile| {
        busy_id.set(Some(profile.id));
        spawn_local(async move {
            match store.remove(profile.id).await {
                Ok(()) => toast::success(&format!("Profil „{}“ gelöscht", profile.name)),
                Err(e) => toast::error(&e),
            }
            busy_id.set(None);
            confirm_delete_id.set(None);
        });
    };

    let active_name = move || {
        let active = store.active_id.get();
        store.profiles.with(|p| p.iter().find(|x| Some(x.id) == active).map(|x| x.name.clone()))
    };

    let profile_rows = move || {
        let profiles = store.profiles.get();
        if profiles.is_empty() {
            return view! { <p class="px-5 py-4 text-sm text-muted-foreground">"Noch keine Profile angelegt."</p> }.into_any();
        }
        profiles
            .into_iter()
            .map(|profile| {
                let id = profile.id;
                let is_active = move || store.active_id.get() == Some(id);
                let summary = format!(
                    "SSH {} · MySQL {}",
                    if profile.ssh_host.is_empty() { "-" } else { &profile.ssh_host },
                    if profile.mysql_host.is_empty() { "-" } else { &profile.mysql_host },
                );
                let p_activate = profile.clone();
                let p_delete = profile.clone();
                let any_busy = Signal::derive(move || busy_id.get().is_some());
                view! {
                    <div class=move || {
                        format!(
                            "flex flex-wrap items-center gap-3 border-l-4 px-5 py-3 text-sm {}",
                            if is_active() { "border-l-green-600 bg-green-600/5" } else { "border-l-transparent" }
                        )
                    }>
                        <div class="min-w-0 flex-1">
                            <div class="flex items-center gap-2 text-base font-medium">
                                {profile.name.clone()}
                                <Show when=is_active>
                                    <span class="flex items-center gap-1 rounded-full bg-green-600/15 px-2 py-0.5 text-[11px] font-medium text-green-600">
                                        <Icon name="CheckCircle2" class="size-3"/>
                                        "Aktiv"
                                    </span>
                                </Show>
                            </div>
                            <div class="truncate text-xs text-muted-foreground">{summary}</div>
                        </div>
                        {move || {
                            if confirm_delete_id.get() == Some(id) {
                                let p = p_delete.clone();
                                view! {
                                    <span class="text-xs text-muted-foreground">"Wirklich löschen?"</span>
                                    <Button
                                        size=ButtonSize::Sm
                                        variant=ButtonVariant::Destructive
                                        on_click=move |_| on_delete(p.clone())
                                        disabled=any_busy
                                    >
                                        "Löschen"
                                    </Button>
                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| confirm_delete_id.set(None)>
                                        "Abbrechen"
                                    </Button>
                                }
                                .into_any()
                            } else {
                                let p = p_activate.clone();
                                view! {
                                    <Show when=move || !is_active()>
                                        {
                                            let p = p.clone();
                                            view! {
                                                <Button
                                                    size=ButtonSize::Sm
                                                    variant=ButtonVariant::Outline
                                                    on_click=move |_| on_activate(p.clone())
                                                    disabled=any_busy
                                                >
                                                    {move || if busy_id.get() == Some(id) { "Verbinde…" } else { "Aktivieren" }}
                                                </Button>
                                            }
                                        }
                                    </Show>
                                    <Button
                                        size=ButtonSize::IconSm
                                        variant=ButtonVariant::Ghost
                                        on_click=move |_| confirm_delete_id.set(Some(id))
                                        disabled=any_busy
                                        title="Profil löschen"
                                    >
                                        <Icon name="Trash2" class="size-3.5"/>
                                    </Button>
                                }
                                .into_any()
                            }
                        }}
                    </div>
                }
            })
            .collect_view()
            .into_any()
    };

    view! {
        <div class="space-y-4">
            <SettingsCard
                title="Server-Profile".to_string()
                icon="Server"
                description="Jedes Profil merkt sich SSH- und MySQL-Zugang inkl. Passwörter (Windows Anmeldeinformationsverwaltung).".to_string()
                actions=std::sync::Arc::new(move || view! {
                    <Button size=ButtonSize::Sm on_click=move |_| dialog_open.set(true)>
                        <Icon name="Plus" class="size-3.5"/>
                        "Neues Profil"
                    </Button>
                }.into_any())
            >
                {profile_rows}
            </SettingsCard>

            <div class="grid gap-4 2xl:grid-cols-2">
                <ConnectionEditor
                    title="SSH-Zugang"
                    icon="Terminal"
                    description=Signal::derive(move || match active_name() {
                        Some(n) => format!("Gehört zum aktiven Profil „{n}“. Für Server-Steuerung, Logs, Quests und alle Server-Dateien."),
                        None => "Für Server-Steuerung, Logs, Quests und alle Server-Dateien.".to_string(),
                    })
                    kind=EditorKind::Ssh(ssh)
                />
                <ConnectionEditor
                    title="MySQL-Zugang"
                    icon="Database"
                    description=Signal::derive(move || match active_name() {
                        Some(n) => format!("Gehört zum aktiven Profil „{n}“. Für alle Datenbank-Editoren und die Statistiken."),
                        None => "Für alle Datenbank-Editoren und die Statistiken.".to_string(),
                    })
                    kind=EditorKind::Mysql(mysql)
                />
            </div>

            <Show when=move || dialog_open.get()>
                <NewProfileDialog on_close=move |_| dialog_open.set(false)/>
            </Show>
        </div>
    }
}

#[derive(Clone, Copy)]
enum EditorKind {
    Ssh(RwSignal<SshValues>),
    Mysql(RwSignal<MysqlValues>),
}

#[component]
fn ConnectionEditor(
    title: &'static str,
    icon: &'static str,
    #[prop(into)] description: Signal<String>,
    kind: EditorKind,
) -> impl IntoView {
    // Zustand geht bei jeder Änderung an den Feldern zurück auf "idle".
    let fingerprint = Signal::derive(move || match kind {
        EditorKind::Ssh(s) => format!("{:?}", s.get()),
        EditorKind::Mysql(m) => format!("{:?}", m.get()),
    });
    let test = ConnectionTest::new(fingerprint);
    let can_save = move || match kind {
        EditorKind::Ssh(s) => s.with(ssh_complete),
        EditorKind::Mysql(m) => m.with(mysql_complete),
    };
    let save = move |_| match kind {
        EditorKind::Ssh(s) => {
            let v = s.get_untracked();
            test.run(async move {
                test_ssh(&v).await?;
                save_ssh(&v).await?;
                connection_profiles().sync_active().await
            });
        }
        EditorKind::Mysql(m) => {
            let v = m.get_untracked();
            test.run(async move {
                test_mysql(&v).await?;
                connect_mysql(&v).await?;
                save_mysql(&v).await?;
                connection_profiles().sync_active().await
            });
        }
    };
    let fields = match kind {
        EditorKind::Ssh(s) => view! { <SshFields value=s/> }.into_any(),
        EditorKind::Mysql(m) => view! { <MysqlFields value=m/> }.into_any(),
    };
    view! {
        <SettingsCard title=title.to_string() description=description icon=icon class="h-fit">
            <div class="max-w-3xl space-y-4 px-5 py-4">
                {fields}
                <div class="flex items-start gap-3">
                    <Button
                        on_click=save
                        disabled=Signal::derive(move || !can_save() || test.state.get() == TestState::Testing)
                        class="shrink-0"
                    >
                        {move || if test.state.get() == TestState::Testing { "Teste…" } else { "Testen & speichern" }}
                    </Button>
                    <TestResult state=test.state error=test.error/>
                </div>
            </div>
        </SettingsCard>
    }
}
