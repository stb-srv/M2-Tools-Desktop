//! Server-Auswahl ganz oben auf der Startseite: aktives Profil wechseln oder
//! direkt ein neues anlegen, ohne erst in die Einstellungen zu müssen.

use super::new_profile_dialog::NewProfileDialog;
use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::lib::connection::get_setting;
use crate::store::connection_profiles::{connection_profiles, ConnectionProfile};
use crate::store::settings_section::{open_settings, SettingsSection};
use crate::tauri::invoke_unit;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde_json::json;

/// Zeile unter der Auswahl: Zugangsdaten des aktiven Profils.
pub fn profile_summary(active: Option<&ConnectionProfile>) -> String {
    match active {
        Some(a) => format!(
            "SSH {}@{}:{} · MySQL {}@{}:{}",
            a.ssh_username, a.ssh_host, a.ssh_port, a.mysql_username, a.mysql_host, a.mysql_port
        ),
        None => "Kein Profil aktiv - wähle einen Server aus der Liste.".to_string(),
    }
}

#[component]
pub fn ServerSwitcher() -> impl IntoView {
    let store = connection_profiles();
    let dialog_open = RwSignal::new(false);
    let switching = RwSignal::new(false);

    spawn_local(async move {
        let _ = store.reload().await;
        adopt_legacy_connection().await;
    });

    let on_select = move |id: i64| {
        if Some(id) == store.active_id.get_untracked() {
            return;
        }
        let name = store
            .profiles
            .with_untracked(|p| p.iter().find(|x| x.id == id).map(|x| x.name.clone()))
            .unwrap_or_else(|| id.to_string());
        switching.set(true);
        spawn_local(async move {
            match store.activate(id).await {
                Ok(()) => toast::success(&format!("Verbunden mit „{name}“")),
                Err(e) => toast::error(&e),
            }
            switching.set(false);
        });
    };

    let empty = move || store.loaded.get() && store.profiles.with(|p| p.is_empty());

    view! {
        <section class="rounded-lg border border-border bg-card p-4">
            {move || {
                if empty() {
                    view! {
                        <div class="flex flex-wrap items-center justify-between gap-3">
                            <div class="flex items-center gap-3">
                                <Icon name="Server" class="size-5 text-muted-foreground"/>
                                <div>
                                    <div class="font-medium">"Noch kein Server eingerichtet"</div>
                                    <div class="text-sm text-muted-foreground">
                                        "Leg ein Profil an: erst den Namen, danach die Zugangsdaten."
                                    </div>
                                </div>
                            </div>
                            <Button on_click=move |_| dialog_open.set(true)>
                                <Icon name="Plus" class="size-4"/>
                                "Ersten Server anlegen"
                            </Button>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="flex flex-wrap items-center gap-3">
                            <Icon name="Server" class="size-5 shrink-0 text-primary"/>
                            <div class="min-w-0 flex-1 space-y-1">
                                <div class="flex items-center gap-2">
                                    <select
                                        on:change=move |e| {
                                            if let Ok(id) = event_target_value(&e).parse::<i64>() {
                                                on_select(id);
                                            }
                                        }
                                        disabled=move || switching.get() || !store.loaded.get()
                                        aria-label="Aktiver Server"
                                        class="min-w-48 rounded-md border border-border bg-background px-2 py-1.5 text-sm font-medium"
                                    >
                                        {move || {
                                            // Nach einem (auch fehlgeschlagenen) Wechsel neu
                                            // aufbauen, damit die Auswahl dem aktiven Profil folgt.
                                            switching.track();
                                            let active = store.active_id.get();
                                            let placeholder = active.is_none().then(|| view! {
                                                <option value="" disabled=true selected=true>"Server auswählen…"</option>
                                            });
                                            let options = store
                                                .profiles
                                                .get()
                                                .into_iter()
                                                .map(|p| {
                                                    let selected = Some(p.id) == active;
                                                    view! { <option value=p.id.to_string() selected=selected>{p.name}</option> }
                                                })
                                                .collect_view();
                                            (placeholder, options)
                                        }}
                                    </select>
                                    <Show when=move || switching.get()>
                                        <Icon name="Loader2" class="size-4 animate-spin text-muted-foreground"/>
                                    </Show>
                                </div>
                                <div class="truncate text-xs text-muted-foreground">
                                    {move || {
                                        let active = store.active_id.get();
                                        store.profiles.with(|p| profile_summary(p.iter().find(|x| Some(x.id) == active)))
                                    }}
                                </div>
                            </div>
                            <div class="flex gap-2">
                                <Button
                                    variant=ButtonVariant::Outline
                                    size=ButtonSize::Sm
                                    on_click=move |_| open_settings(SettingsSection::Connection)
                                >
                                    <Icon name="Pencil" class="size-3.5"/>
                                    "Bearbeiten"
                                </Button>
                                <Button size=ButtonSize::Sm on_click=move |_| dialog_open.set(true)>
                                    <Icon name="Plus" class="size-3.5"/>
                                    "Neues Profil"
                                </Button>
                            </div>
                        </div>
                    }
                    .into_any()
                }
            }}

            <Show when=move || dialog_open.get()>
                <NewProfileDialog on_close=move |_| dialog_open.set(false)/>
            </Show>
        </section>
    }
}

/// Wer M2Manager schon vor den Profilen benutzt hat, hat eine eingerichtete
/// Verbindung, aber kein Profil. Die wird einmalig als erstes Profil
/// übernommen, damit die Startseite sofort den gewohnten Server zeigt.
async fn adopt_legacy_connection() {
    let store = connection_profiles();
    if !store.loaded.get_untracked() || !store.profiles.with_untracked(|p| p.is_empty()) {
        return;
    }
    let Some(host) = get_setting("ssh_host").await.filter(|h| !h.is_empty()) else { return };
    let _ = invoke_unit("save_connection_profile", &json!({ "name": host })).await;
    let _ = store.reload().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary() {
        let p = ConnectionProfile {
            id: 1,
            name: "Live".into(),
            ssh_host: "h".into(),
            ssh_port: "22".into(),
            ssh_username: "root".into(),
            ssh_auth_mode: "password".into(),
            ssh_key_path: None,
            mysql_host: "db".into(),
            mysql_port: "3306".into(),
            mysql_username: "m2".into(),
        };
        assert_eq!(profile_summary(Some(&p)), "SSH root@h:22 · MySQL m2@db:3306");
        assert_eq!(profile_summary(None), "Kein Profil aktiv - wähle einen Server aus der Liste.");
    }
}
