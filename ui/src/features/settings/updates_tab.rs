//! Selbst-Update über das Tauri-Updater-Plugin (GitHub-Releases, öffentlich,
//! kein Token nötig).

use crate::components::{Button, ButtonVariant};
use crate::store::update::update_store;
use crate::tauri::{app_version, relaunch, update_check, DownloadEvent, UpdateInfo};
use leptos::prelude::*;
use leptos::task::spawn_local;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Error,
    Available,
}

/// Fortschrittsanzeige wie im alten Frontend: Prozent, wenn die Gesamtgröße
/// bekannt (und > 0) ist, sonst die bisher geladenen Bytes.
pub fn progress_text(downloaded: u64, total: Option<u64>) -> String {
    match total {
        Some(t) if t > 0 => format!("{}%", ((downloaded as f64 / t as f64) * 100.0).round() as i64),
        _ => format!("{downloaded} Bytes"),
    }
}

#[component]
pub fn UpdatesTab() -> impl IntoView {
    let current_version = RwSignal::new(String::new());
    let status = RwSignal::new(UpdateStatus::Idle);
    let error = RwSignal::new(None::<String>);
    // Version + Notizen fürs Anzeigen, das JS-Objekt fürs Installieren.
    let update = RwSignal::new(None::<(String, Option<String>)>);
    let handle = StoredValue::new_local(None::<UpdateInfo>);
    let downloading = RwSignal::new(false);
    let downloaded_bytes = RwSignal::new(0u64);
    let total_bytes = RwSignal::new(None::<u64>);
    let installed = RwSignal::new(false);
    let restarting = RwSignal::new(false);

    spawn_local(async move {
        if let Ok(v) = app_version().await {
            current_version.set(v);
        }
    });

    let run_check = move |_| {
        status.set(UpdateStatus::Checking);
        error.set(None);
        installed.set(false);
        spawn_local(async move {
            match update_check().await {
                Ok(result) => {
                    update.set(result.as_ref().map(|u| (u.version.clone(), u.body.clone())));
                    handle.set_value(result.clone());
                    status.set(if result.is_some() { UpdateStatus::Available } else { UpdateStatus::UpToDate });
                    update_store().set_available(result);
                }
                Err(e) => {
                    status.set(UpdateStatus::Error);
                    error.set(Some(e));
                }
            }
        });
    };

    let install_now = move |_| {
        let Some(info) = handle.get_value() else { return };
        downloading.set(true);
        downloaded_bytes.set(0);
        total_bytes.set(None);
        error.set(None);
        spawn_local(async move {
            let res = info
                .download_and_install(move |ev| match ev {
                    DownloadEvent::Started { content_length } => total_bytes.set(content_length),
                    DownloadEvent::Progress { chunk_length } => downloaded_bytes.update(|b| *b += chunk_length),
                    DownloadEvent::Finished => installed.set(true),
                })
                .await;
            match res {
                Ok(()) => installed.set(true),
                Err(e) => error.set(Some(e)),
            }
            downloading.set(false);
        });
    };

    let restart = move |_| {
        restarting.set(true);
        spawn_local(async move {
            let _ = relaunch().await;
        });
    };

    view! {
        <div class="space-y-4">
            <section class="space-y-3 rounded-xl border border-border bg-card p-5 shadow-sm">
                <p class="text-sm text-muted-foreground">
                    "Aktuelle Version: "
                    <span class="font-medium text-foreground">
                        {move || {
                            let v = current_version.get();
                            if v.is_empty() { "…".to_string() } else { v }
                        }}
                    </span>
                </p>
                <Button
                    variant=ButtonVariant::Outline
                    on_click=run_check
                    disabled=Signal::derive(move || status.get() == UpdateStatus::Checking || downloading.get())
                >
                    {move || if status.get() == UpdateStatus::Checking { "Suche…" } else { "Nach Updates suchen" }}
                </Button>

                <Show when=move || status.get() == UpdateStatus::UpToDate>
                    <p class="text-sm text-green-600">"Du hast bereits die neueste Version."</p>
                </Show>
                <Show when=move || status.get() == UpdateStatus::Error>
                    <p class="text-sm text-destructive">{move || error.get().unwrap_or_default()}</p>
                </Show>

                {move || {
                    if status.get() != UpdateStatus::Available || installed.get() {
                        return None;
                    }
                    let (version, body) = update.get()?;
                    Some(view! {
                        <div class="space-y-2 rounded-md border border-blue-500/40 bg-blue-500/10 p-3">
                            <p class="text-sm font-medium">{format!("Update verfügbar: v{version}")}</p>
                            {body.filter(|b| !b.is_empty()).map(|b| view! {
                                <p class="whitespace-pre-wrap text-xs text-muted-foreground">{b}</p>
                            })}
                            {move || {
                                if !downloading.get() {
                                    view! { <Button on_click=install_now>"Jetzt installieren"</Button> }.into_any()
                                } else {
                                    view! {
                                        <p class="text-xs text-muted-foreground">
                                            {move || format!("Lade herunter… {}", progress_text(downloaded_bytes.get(), total_bytes.get()))}
                                        </p>
                                    }
                                    .into_any()
                                }
                            }}
                        </div>
                    })
                }}

                <Show when=move || installed.get()>
                    <div class="space-y-2 rounded-md border border-green-500/40 bg-green-500/10 p-3">
                        <p class="text-sm">"Installiert. Neustart nötig, um die neue Version zu verwenden."</p>
                        <Button on_click=restart disabled=restarting>
                            {move || if restarting.get() { "Starte neu…" } else { "Jetzt neu starten" }}
                        </Button>
                    </div>
                </Show>
            </section>
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::progress_text;

    #[test]
    fn progress() {
        assert_eq!(progress_text(50, Some(200)), "25%");
        assert_eq!(progress_text(50, Some(0)), "50 Bytes");
        assert_eq!(progress_text(1234, None), "1234 Bytes");
    }
}
