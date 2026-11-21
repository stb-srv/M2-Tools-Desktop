//! Gespeicherte SSH-Server-Schlüssel (Trust on first use): Fingerprint
//! anzeigen und einen Schlüssel „vergessen“, z.B. nach einer Server-Neuinstallation.

use super::shared::SettingsCard;
use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::tauri::{invoke, invoke0, listen};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize, PartialEq)]
struct KnownHost {
    host: String,
    port: u16,
    fingerprint: String,
    first_seen: String,
}

#[component]
pub fn KnownHostsCard() -> impl IntoView {
    let hosts = RwSignal::new(Vec::<KnownHost>::new());
    let confirm = RwSignal::new(None::<(String, u16)>);

    let reload = move || {
        spawn_local(async move {
            if let Ok(list) = invoke0::<Vec<KnownHost>>("list_known_hosts").await {
                hosts.set(list);
            }
        });
    };
    reload();

    // Beim ersten Kontakt mit einem Server meldet das Backend den neu gemerkten Schlüssel.
    spawn_local(async move {
        let _ = listen::<KnownHost>("ssh-host-key-new", move |h| {
            toast::success(&format!("SSH-Schlüssel von {}:{} gespeichert ({})", h.host, h.port, h.fingerprint));
            reload();
        })
        .await;
    });

    let forget = move |host: String, port: u16| {
        spawn_local(async move {
            match invoke::<bool>("forget_known_host", &json!({ "host": host, "port": port })).await {
                Ok(_) => toast::success("Schlüssel vergessen - beim nächsten Verbinden wird er neu gespeichert"),
                Err(e) => toast::error(&e),
            }
            confirm.set(None);
            reload();
        });
    };

    view! {
        <SettingsCard
            title="SSH-Server-Schlüssel".to_string()
            icon="KeyRound"
            description="Beim ersten Verbinden wird der Fingerprint des Servers gemerkt. Ändert er sich später, wird die Verbindung aus Sicherheitsgründen abgebrochen.".to_string()
        >
            {move || {
                let list = hosts.get();
                if list.is_empty() {
                    return view! { <p class="px-5 py-4 text-sm text-muted-foreground">"Noch keine Server-Schlüssel gespeichert."</p> }.into_any();
                }
                list.into_iter()
                    .map(|h| {
                        let (host, port) = (h.host.clone(), h.port);
                        let (host_c, host_f) = (host.clone(), host.clone());
                        view! {
                            <div class="flex flex-wrap items-center gap-3 border-b border-border px-5 py-3 text-sm last:border-b-0">
                                <div class="min-w-0 flex-1">
                                    <div class="font-medium">{format!("{}:{}", h.host, h.port)}</div>
                                    <div class="break-all font-mono text-xs text-muted-foreground">{h.fingerprint.clone()}</div>
                                    <div class="text-xs text-muted-foreground">{format!("Gespeichert am {}", h.first_seen)}</div>
                                </div>
                                {move || {
                                    if confirm.get() == Some((host.clone(), port)) {
                                        let host_f = host_f.clone();
                                        view! {
                                            <span class="text-xs text-muted-foreground">"Schlüssel wirklich vergessen?"</span>
                                            <Button size=ButtonSize::Sm variant=ButtonVariant::Destructive on_click=move |_| forget(host_f.clone(), port)>
                                                "Vergessen"
                                            </Button>
                                            <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| confirm.set(None)>
                                                "Abbrechen"
                                            </Button>
                                        }
                                        .into_any()
                                    } else {
                                        let host_c = host_c.clone();
                                        view! {
                                            <Button size=ButtonSize::Sm variant=ButtonVariant::Outline on_click=move |_| confirm.set(Some((host_c.clone(), port)))>
                                                <Icon name="Trash2" class="size-3.5"/>
                                                "Schlüssel vergessen"
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
            }}
        </SettingsCard>
    }
}
