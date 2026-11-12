//! „Client vorbereiten“: zeigt die gepackten Archive des Clients
//! (`pack/*.epk/.eix`) und entpackt die ausgewählten mit
//! EterPackConsoleLz4.exe, damit Item-Icons, Modelle und Texte im Tool
//! auftauchen. Bereits vorhandene lose Dateien werden nie überschrieben.
//! Genutzt im Einrichtungs-Assistenten, in den Einstellungen und auf dem
//! Dashboard. Der Vorgang selbst läuft im Hintergrund (siehe
//! [`crate::store::client_unpack`]) und überlebt das Verlassen der Seite.

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon};
use crate::store::client_unpack::client_unpack_store;
use crate::tauri::{dialog_open_one, invoke, invoke_unit, DialogFilter, OpenDialogOptions};
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ArchiveStatus {
    pub name: String,
    pub label: String,
    pub size_bytes: u64,
    pub unpacked: bool,
    pub recommended: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct UnpackStatus {
    pub pack_dir: String,
    pub pack_dir_exists: bool,
    pub tool_path: Option<String>,
    pub archives: Vec<ArchiveStatus>,
}

impl UnpackStatus {
    /// Icons/Texte sind noch gepackt - die Kernfunktionen zeigen dann nichts an.
    pub fn needs_unpack(&self) -> bool {
        self.archives.iter().any(|a| a.recommended && !a.unpacked)
    }
}

pub fn format_size(bytes: u64) -> String {
    let mb = bytes as f64 / (1024.0 * 1024.0);
    if mb >= 1024.0 {
        format!("{:.1} GB", mb / 1024.0)
    } else if mb >= 1.0 {
        format!("{mb:.0} MB")
    } else {
        format!("{} KB", (bytes / 1024).max(1))
    }
}

fn client_arg(path: &str) -> serde_json::Value {
    if path.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(path.to_string())
    }
}

/// Lädt den Entpack-Status. Leerer `client_path` = gespeicherter Client-Ordner.
pub async fn load_unpack_status(client_path: &str) -> Result<UnpackStatus, String> {
    invoke::<UnpackStatus>("get_client_unpack_status", &json!({ "clientPath": client_arg(client_path) })).await
}

/// `client_path`: leer = gespeicherter Client-Ordner (Einstellungen/Dashboard),
/// sonst der im Assistenten gewählte, noch nicht gespeicherte Ordner.
#[component]
pub fn ClientUnpackPanel(
    #[prop(into)] client_path: Signal<String>,
) -> impl IntoView {
    let job = client_unpack_store();
    let status = RwSignal::new(None::<Result<UnpackStatus, String>>);
    let selected = RwSignal::new(HashSet::<String>::new());
    let running = job.running;
    let progress = job.progress;
    let last_line = job.last_line;
    let results = job.results;
    let show_all = RwSignal::new(false);

    let reload = move || {
        let path = client_path.get_untracked();
        spawn_local(async move {
            let res = load_unpack_status(&path).await;
            if let Ok(s) = &res {
                selected.set(
                    s.archives
                        .iter()
                        .filter(|a| a.recommended && !a.unpacked)
                        .map(|a| a.name.clone())
                        .collect(),
                );
            }
            status.set(Some(res));
        });
    };

    Effect::new(move |prev: Option<String>| {
        let path = client_path.get();
        if prev.is_some_and(|p| p != path) && !running.get_untracked() {
            results.set(None);
        }
        reload();
        path
    });

    // Nach einem (evtl. im Hintergrund) abgeschlossenen Lauf neu einlesen.
    Effect::new(move |prev: Option<u32>| {
        let n = job.finished.get();
        if prev.is_some_and(|p| p != n) {
            reload();
        }
        n
    });

    let pick_tool = move |_| {
        spawn_local(async move {
            let opts = OpenDialogOptions {
                title: Some("EterPackConsoleLz4.exe auswählen".into()),
                filters: vec![DialogFilter::new("Programm", &["exe"])],
                ..Default::default()
            };
            if let Ok(Some(p)) = dialog_open_one(&opts).await {
                match invoke_unit("set_setting", &json!({ "key": "eterpack_tool_path", "value": p })).await {
                    Ok(()) => reload(),
                    Err(e) => toast::error(&format!("Pfad konnte nicht gespeichert werden: {e}")),
                }
            }
        });
    };

    let start = move |_| {
        let names: Vec<String> = status.with_untracked(|s| match s {
            Some(Ok(s)) => s
                .archives
                .iter()
                .map(|a| a.name.clone())
                .filter(|n| selected.with_untracked(|sel| sel.contains(n)))
                .collect(),
            _ => Vec::new(),
        });
        let path = client_path.get_untracked();
        job.start((!path.is_empty()).then_some(path), names);
    };

    let toggle = move |name: String| {
        selected.update(|s| {
            if !s.remove(&name) {
                s.insert(name);
            }
        });
    };

    move || match status.get() {
        None => view! { <p class="text-sm text-muted-foreground">"Prüfe Client-Ordner…"</p> }.into_any(),
        Some(Err(e)) => view! { <p class="text-sm text-destructive">{e}</p> }.into_any(),
        Some(Ok(s)) if !s.pack_dir_exists => view! {
            <p class="text-sm text-muted-foreground">
                "Im Client-Ordner gibt es keinen "<code>"pack"</code>"-Ordner - hier ist nichts zu entpacken."
            </p>
        }
        .into_any(),
        Some(Ok(s)) => {
            let sizes: Vec<(String, u64)> = s.archives.iter().map(|a| (a.name.clone(), a.size_bytes)).collect();
            let total_selected = move || -> u64 {
                selected.with(|sel| sizes.iter().filter(|(n, _)| sel.contains(n)).map(|(_, b)| b).sum())
            };
            let pending = s.needs_unpack();
            let has_optional = s.archives.iter().any(|a| !a.recommended);
            let tool = s.tool_path.clone();
            let archives = s.archives.clone();
            view! {
                <div class="space-y-3">
                    <p class="text-sm">
                        {if pending {
                            view! {
                                <span class="flex items-center gap-1.5 text-amber-600">
                                    <Icon name="AlertTriangle" class="size-4"/>
                                    "Icons/Texte sind noch gepackt - ohne Entpacken bleiben Item-Icons leer."
                                </span>
                            }
                            .into_any()
                        } else {
                            view! {
                                <span class="flex items-center gap-1.5 text-green-600">
                                    <Icon name="CheckCircle2" class="size-4"/>
                                    "Icons und Texte liegen entpackt vor."
                                </span>
                            }
                            .into_any()
                        }}
                    </p>

                    <div class="flex flex-wrap items-center gap-2 text-sm">
                        {match tool {
                            Some(t) => view! {
                                <span class="text-muted-foreground">"EterPack: "</span>
                                <code class="truncate text-xs">{t}</code>
                            }
                            .into_any(),
                            None => view! {
                                <span class="text-destructive">
                                    "EterPackConsoleLz4.exe nicht im Client gefunden."
                                </span>
                                <Button variant=ButtonVariant::Outline size=ButtonSize::Sm on_click=pick_tool>
                                    <Icon name="FolderOpen" class="size-3.5"/>
                                    "Auswählen"
                                </Button>
                            }
                            .into_any(),
                        }}
                    </div>

                    <div class="max-h-64 divide-y divide-border overflow-y-auto rounded-lg border border-border">
                        {archives
                            .into_iter()
                            .map(|a| {
                                let name = a.name.clone();
                                let name_check = a.name.clone();
                                let recommended = a.recommended;
                                view! {
                                    <Show when=move || recommended || show_all.get()>
                                        <label class="flex cursor-pointer items-center gap-3 px-3 py-1.5 text-sm">
                                            <input
                                                type="checkbox"
                                                prop:checked={
                                                    let n = name_check.clone();
                                                    move || selected.with(|s| s.contains(&n))
                                                }
                                                disabled=move || running.get()
                                                on:change={
                                                    let n = name.clone();
                                                    move |_| toggle(n.clone())
                                                }
                                            />
                                            <span class="min-w-0 flex-1">
                                                <code>{a.name.clone()}</code>
                                                <span class="ml-2 text-xs text-muted-foreground">{a.label.clone()}</span>
                                            </span>
                                            <span class="text-xs text-muted-foreground">{format_size(a.size_bytes)}</span>
                                            {if a.unpacked {
                                                view! { <span class="w-20 text-right text-xs text-green-600">"entpackt"</span> }.into_any()
                                            } else {
                                                view! { <span class="w-20 text-right text-xs text-muted-foreground">"gepackt"</span> }.into_any()
                                            }}
                                        </label>
                                    </Show>
                                }
                            })
                            .collect_view()}
                    </div>

                    {has_optional.then(|| view! {
                        <button
                            type="button"
                            class="text-xs text-primary hover:underline"
                            on:click=move |_| show_all.update(|v| *v = !*v)
                        >
                            {move || if show_all.get() {
                                "Nur empfohlene Archive zeigen"
                            } else {
                                "Weitere Archive zeigen (Modelle, Karten, Effekte …)"
                            }}
                        </button>
                    })}

                    <div class="flex flex-wrap items-center gap-3">
                        <Button
                            on_click=start
                            disabled=Signal::derive(move || {
                                running.get()
                                    || selected.with(|s| s.is_empty())
                                    || status.with(|s| !matches!(s, Some(Ok(s)) if s.tool_path.is_some()))
                            })
                        >
                            <Icon name="PackageOpen" class="size-4"/>
                            {move || if running.get() { "Entpacke…".to_string() } else { "Ausgewählte entpacken".to_string() }}
                        </Button>
                        <span class="text-xs text-muted-foreground">
                            {move || if running.get() {
                                "Läuft im Hintergrund - du kannst die Seite verlassen und weiterarbeiten.".to_string()
                            } else {
                                format!("ca. {} - vorhandene Dateien werden nicht überschrieben.", format_size(total_selected()))
                            }}
                        </span>
                    </div>

                    {move || running.get().then(|| view! {
                        <div class="space-y-1 text-xs text-muted-foreground">
                            {move || progress.get().map(|p| format!("Archiv {}/{}: {}", p.current, p.total, p.name))}
                            <div class="truncate font-mono">{move || last_line.get()}</div>
                        </div>
                    })}

                    {move || results.get().map(|r| view! {
                        <ul class="space-y-0.5 text-xs">
                            {r.into_iter()
                                .map(|x| match x.error {
                                    None => view! {
                                        <li class="text-green-600">{format!("{}: {} Dateien entpackt", x.name, x.files_copied)}</li>
                                    }
                                    .into_any(),
                                    Some(e) => view! { <li class="text-destructive">{format!("{}: {e}", x.name)}</li> }.into_any(),
                                })
                                .collect_view()}
                        </ul>
                    })}
                </div>
            }
            .into_any()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_sizes() {
        assert_eq!(format_size(500), "1 KB");
        assert_eq!(format_size(20 * 1024 * 1024), "20 MB");
        assert_eq!(format_size(3 * 1024 * 1024 * 1024 / 2), "1.5 GB");
    }

    #[test]
    fn needs_unpack_only_for_recommended_archives() {
        let a = |name: &str, recommended, unpacked| ArchiveStatus {
            name: name.into(),
            label: String::new(),
            size_bytes: 0,
            unpacked,
            recommended,
        };
        let mut s = UnpackStatus {
            pack_dir: String::new(),
            pack_dir_exists: true,
            tool_path: None,
            archives: vec![a("icon", true, true), a("map_a1", false, false)],
        };
        assert!(!s.needs_unpack());
        s.archives.push(a("locale_de", true, false));
        assert!(s.needs_unpack());
    }
}
