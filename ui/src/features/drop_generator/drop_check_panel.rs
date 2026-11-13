//! Drop-Check: prüft alle 5 Drop-Dateien auf einmal gegen die Datenbank
//! (`run_drop_check`). Ein unbekanntes Item in einer dieser Dateien
//! verhindert den Serverstart - hier sieht man das vorher.

use crate::components::{Button, ButtonSize, ButtonVariant, Icon};
use crate::tauri::invoke0;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
struct DropFileStatus {
    file: String,
    state: String,
    detail: Option<String>,
    entries: usize,
}

#[derive(Clone, Debug, Deserialize)]
struct DropCheckIssue {
    file: String,
    location: String,
    level: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize)]
struct DropCheckReport {
    files: Vec<DropFileStatus>,
    issues: Vec<DropCheckIssue>,
    db_error: Option<String>,
}

#[component]
pub fn DropCheckPanel() -> impl IntoView {
    let running = RwSignal::new(false);
    let report = RwSignal::new(None::<Result<DropCheckReport, String>>);
    let open = RwSignal::new(true);

    let run = move || {
        running.set(true);
        open.set(true);
        spawn_local(async move {
            report.set(Some(invoke0::<DropCheckReport>("run_drop_check").await));
            running.set(false);
        });
    };

    let file_view = move |r: &DropCheckReport, f: DropFileStatus| {
        let errors = r.issues.iter().filter(|i| i.file == f.file && i.level == "error").count();
        let warnings = r.issues.iter().filter(|i| i.file == f.file && i.level != "error").count();
        let (icon, cls, text) = match f.state.as_str() {
            "missing" => ("MinusCircle", "text-muted-foreground", "fehlt".to_string()),
            "unreadable" => ("XCircle", "text-destructive", "nicht lesbar".to_string()),
            _ if errors > 0 => ("XCircle", "text-destructive", format!("{errors} Fehler")),
            _ if warnings > 0 => ("AlertTriangle", "text-amber-600 dark:text-amber-400", format!("{warnings} Hinweis(e)")),
            _ => ("CheckCircle2", "text-green-600", "ok".to_string()),
        };
        let issues: Vec<DropCheckIssue> = r.issues.iter().filter(|i| i.file == f.file).cloned().collect();
        view! {
            <div class="space-y-1">
                <div class="flex items-center gap-2 text-sm" title=f.detail.clone().unwrap_or_default()>
                    <Icon name=icon class=format!("size-4 shrink-0 {cls}")/>
                    <code class="text-xs">{f.file.clone()}</code>
                    <span class="text-xs text-muted-foreground">{if f.state == "ok" { format!("{} Einträge", f.entries) } else { String::new() }}</span>
                    <span class=format!("ml-auto text-xs {cls}")>{text}</span>
                </div>
                {(!issues.is_empty()).then(|| view! {
                    <ul class="ml-6 space-y-0.5">
                        {issues.into_iter().map(|i| view! {
                            <li class=if i.level == "error" { "text-xs text-destructive" } else { "text-xs text-amber-600 dark:text-amber-400" }>
                                <span class="font-medium">{i.location}</span> ": " {i.message}
                            </li>
                        }).collect_view()}
                    </ul>
                })}
                {(f.state == "unreadable").then(|| f.detail.clone().map(|d| view! { <p class="ml-6 text-xs text-destructive">{d}</p> })).flatten()}
            </div>
        }
    };

    view! {
        <div class="space-y-3 rounded-lg border border-border p-3">
            <div class="flex flex-wrap items-center gap-2">
                <Icon name="Stethoscope" class="size-4 text-muted-foreground"/>
                <span class="text-sm font-medium">"Drop-Check"</span>
                <span class="text-sm text-muted-foreground">"Prüft alle 5 Dateien: gibt es jedes Item und jeden Mob?"</span>
                <div class="ml-auto flex gap-2">
                    <Show when=move || report.with(|r| r.is_some())>
                        <Button size=ButtonSize::Sm variant=ButtonVariant::Ghost on_click=move |_| open.update(|o| *o = !*o)>
                            {move || if open.get() { "Einklappen" } else { "Anzeigen" }}
                        </Button>
                    </Show>
                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline disabled=running on_click=move |_| run()>
                        <Icon name=Signal::derive(move || if running.get() { "Loader2" } else { "Search" })
                              class=Signal::derive(move || if running.get() { "size-3.5 animate-spin".to_string() } else { "size-3.5".to_string() })/>
                        {move || if running.get() { "Prüfe…" } else { "Jetzt prüfen" }}
                    </Button>
                </div>
            </div>
            <Show when=move || open.get()>
                {move || report.get().map(|r| match r {
                    Err(e) => view! { <p class="text-sm text-destructive">{e}</p> }.into_any(),
                    Ok(r) => {
                        let files = r.files.clone();
                        let errors = r.issues.iter().filter(|i| i.level == "error").count();
                        view! {
                            <div class="space-y-2">
                                {files.into_iter().map(|f| file_view(&r, f)).collect_view()}
                                {r.db_error.clone().map(|e| view! {
                                    <p class="text-xs text-amber-600 dark:text-amber-400">
                                        "Items und Mobs wurden nicht gegen die Datenbank geprüft: " {e}
                                    </p>
                                })}
                                {(errors > 0).then(|| view! {
                                    <p class="text-xs text-muted-foreground">
                                        "Fehler bedeuten: der Server startet mit dieser Datei nicht oder nimmt ein falsches Item. Die Editoren speichern solche Dateien nicht mehr."
                                    </p>
                                })}
                            </div>
                        }.into_any()
                    }
                })}
            </Show>
        </div>
    }
}
