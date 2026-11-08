use super::diff::{diff_lines, has_changes};
use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon, PageHeader};
use crate::util::log_activity;
use crate::store::navigation::Section;
use crate::tauri::invoke;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RemoteEntry {
    name: String,
    is_dir: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct BackupDiff {
    target_path: String,
    backup_content: String,
    current_content: Option<String>,
    is_binary: bool,
}

const DEFAULT_ROOT: &str = "/usr/home/game";
const BACKUP_FOLDER_NAME: &str = "m2manager_backups";

/// Backup-Dateinamen kodieren ihren Zeitpunkt:
/// `<name>.<YYYYMMDD_HHMMSS>.bak` / `.deleted` (siehe backups.rs).
pub fn parse_timestamp(filename: &str) -> Option<String> {
    let rest = filename.strip_suffix(".bak").or_else(|| filename.strip_suffix(".deleted"))?;
    // rest endet auf ".<8 Ziffern>_<6 Ziffern>"
    if rest.len() < 16 {
        return None;
    }
    let (head, stamp) = rest.split_at(rest.len() - 16);
    let _ = head;
    let b = stamp.as_bytes();
    if b[0] != b'.' || b[9] != b'_' {
        return None;
    }
    let date = &stamp[1..9];
    let time = &stamp[10..16];
    if !date.bytes().chain(time.bytes()).all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(format!(
        "{}-{}-{} {}:{}:{}",
        &date[0..4],
        &date[4..6],
        &date[6..8],
        &time[0..2],
        &time[2..4],
        &time[4..6]
    ))
}

fn last_segment(path: &str) -> String {
    path.rsplit('/').next().unwrap_or_default().to_string()
}

fn breadcrumbs(dir: Option<&str>, root: Option<&str>) -> Vec<String> {
    match (dir, root) {
        (Some(dir), Some(root)) if !dir.is_empty() && !root.is_empty() && dir != root => {
            let rest = dir.get(root.len()..).unwrap_or_default();
            let rest = rest.strip_prefix('/').unwrap_or(rest);
            rest.split('/').map(str::to_string).collect()
        }
        _ => vec![],
    }
}

fn go_up(prev: Option<String>, root: Option<&str>) -> Option<String> {
    let (Some(p), Some(root)) = (prev.as_deref(), root) else { return prev };
    if p.is_empty() || root.is_empty() || p.len() <= root.len() {
        return prev;
    }
    match p.rfind('/') {
        None => Some(root.to_string()),
        Some(idx) => Some(p[..idx].to_string()),
    }
}

#[component]
pub fn BackupBrowser() -> impl IntoView {
    let root = RwSignal::new(None::<String>);
    let dir = RwSignal::new(None::<String>);
    let entries = RwSignal::new(None::<Vec<RemoteEntry>>);
    let loading = RwSignal::new(false);
    let error = RwSignal::new(None::<String>);

    let restore_confirm = RwSignal::new(None::<String>);
    let restoring = RwSignal::new(false);

    let diff_path = RwSignal::new(None::<String>);
    let diff_data = RwSignal::new(None::<BackupDiff>);
    let diff_loading = RwSignal::new(false);
    let diff_error = RwSignal::new(None::<String>);

    // Wie ein React-State: gleicher Wert -> kein erneutes Laden.
    let set_dir = move |next: Option<String>| {
        if dir.get_untracked() != next {
            dir.set(next);
        }
    };

    spawn_local(async move {
        let workdir = invoke::<Option<String>>("get_setting", &json!({ "key": "server_workdir" }))
            .await
            .ok()
            .flatten()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| DEFAULT_ROOT.to_string());
        root.set(Some(workdir.clone()));
        set_dir(Some(workdir));
    });

    let load_dir = move |path: String| {
        loading.set(true);
        error.set(None);
        spawn_local(async move {
            match invoke::<Vec<RemoteEntry>>("list_remote_dir", &json!({ "path": path })).await {
                Ok(e) => entries.set(Some(e)),
                Err(e) => error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    Effect::new(move |_| {
        if let Some(d) = dir.get() {
            load_dir(d);
        }
    });

    let open_folder = move |name: String| {
        set_dir(dir.get_untracked().map(|p| format!("{p}/{name}")).or(Some(format!("null/{name}"))));
    };

    let open_diff = move |backup_path: String| {
        diff_path.set(Some(backup_path.clone()));
        diff_data.set(None);
        diff_loading.set(true);
        diff_error.set(None);
        spawn_local(async move {
            match invoke::<BackupDiff>("diff_remote_backup", &json!({ "backupPath": backup_path })).await {
                Ok(d) => diff_data.set(Some(d)),
                Err(e) => diff_error.set(Some(e)),
            }
            diff_loading.set(false);
        });
    };

    let confirm_restore = move || {
        let Some(backup_path) = restore_confirm.get_untracked() else { return };
        restore_confirm.set(None);
        restoring.set(true);
        spawn_local(async move {
            match invoke::<String>("restore_remote_backup", &json!({ "backupPath": backup_path })).await {
                Ok(target) => {
                    toast::success(&format!("Wiederhergestellt nach: {target}"));
                    log_activity(
                        Section::BackupBrowser.into(),
                        "restore",
                        &format!("Backup wiederhergestellt: '{backup_path}' → '{target}'"),
                        Some("file"),
                        Some(&target),
                    );
                }
                Err(e) => toast::error(&e),
            }
            restoring.set(false);
        });
    };

    let is_in_backup_folder = move || dir.with(|d| d.as_deref().map(last_segment).as_deref() == Some(BACKUP_FOLDER_NAME));
    let crumbs = Memo::new(move |_| breadcrumbs(dir.get().as_deref(), root.get().as_deref()));
    let not_at_root = move || matches!((dir.get(), root.get()), (Some(d), Some(r)) if !d.is_empty() && !r.is_empty() && d != r);

    view! {
        <div class="flex h-full flex-col gap-3">
            <div>
                <PageHeader module=Section::BackupBrowser title="Datei-Backups".to_string()/>
                <p class="max-w-4xl text-sm text-muted-foreground">
                    "Jeder Editor (Item Editor, Mob Drop Editor, Quest Builder, Regen-Datei-Editor, Locale-Verwaltung) legt vor jedem Überschreiben/Löschen ein Backup direkt neben der Originaldatei an, in einem "
                    <code>{BACKUP_FOLDER_NAME}</code>
                    "-Ordner - hier durchsuchbar und wiederherstellbar, ohne manuell per SFTP nachzuschauen."
                </p>
            </div>

            <div class="flex items-center gap-1 text-sm">
                <Button variant=ButtonVariant::Ghost size=ButtonSize::IconSm on_click=move |_| set_dir(root.get_untracked())>
                    <Icon name="Home" class="size-3.5"/>
                </Button>
                {move || {
                    let parts = crumbs.get();
                    parts.iter().enumerate().map(|(i, part)| {
                        let is_backup = part == BACKUP_FOLDER_NAME;
                        let target_parts = parts[..=i].join("/");
                        view! {
                            <span class="flex items-center gap-1 text-muted-foreground">
                                <Icon name="ChevronRight" class="size-3.5"/>
                                <button
                                    class=if is_backup { "hover:text-foreground font-medium text-foreground" } else { "hover:text-foreground " }
                                    on:click=move |_| {
                                        let r = root.get_untracked().unwrap_or_else(|| "null".into());
                                        set_dir(Some(format!("{r}/{target_parts}")));
                                    }
                                >
                                    {part.clone()}
                                </button>
                            </span>
                        }
                    }).collect_view()
                }}
                <Button
                    variant=ButtonVariant::Ghost
                    size=ButtonSize::IconSm
                    class="ml-auto"
                    on_click=move |_| {
                        if let Some(d) = dir.get_untracked().filter(|d| !d.is_empty()) {
                            load_dir(d);
                        }
                    }
                >
                    <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-3.5 animate-spin".to_string() } else { "size-3.5 ".to_string() })/>
                </Button>
            </div>

            {move || error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <div class="flex-1 space-y-1 overflow-y-auto rounded-md border border-border p-1">
                <Show when=not_at_root>
                    <button
                        on:click=move |_| set_dir(go_up(dir.get_untracked(), root.get_untracked().as_deref()))
                        class="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
                    >
                        <Icon name="Folder" class="size-4 text-muted-foreground"/>
                        ".."
                    </button>
                </Show>
                {move || {
                    let in_backup = is_in_backup_folder();
                    let cur_dir = dir.get().filter(|d| !d.is_empty());
                    entries.get().unwrap_or_default().into_iter().map(|entry| {
                        if entry.is_dir {
                            let is_backup_folder = entry.name == BACKUP_FOLDER_NAME;
                            let name = entry.name.clone();
                            return view! {
                                <button
                                    on:click=move |_| open_folder(name.clone())
                                    class="flex w-full items-center gap-2 rounded-md px-2 py-1 text-left text-sm hover:bg-muted"
                                >
                                    {if is_backup_folder {
                                        view! { <Icon name="FolderArchive" class="size-4 text-amber-500"/> }.into_any()
                                    } else {
                                        view! { <Icon name="Folder" class="size-4 text-muted-foreground"/> }.into_any()
                                    }}
                                    <span class=if is_backup_folder { "font-medium" } else { "" }>{entry.name.clone()}</span>
                                </button>
                            }.into_any();
                        }

                        let timestamp = if in_backup { parse_timestamp(&entry.name) } else { None };
                        let actions = match (&timestamp, &cur_dir) {
                            (Some(_), Some(d)) if in_backup => {
                                let full = format!("{d}/{}", entry.name);
                                let full2 = full.clone();
                                Some(view! {
                                    <Button size=ButtonSize::Sm variant=ButtonVariant::Outline class="shrink-0" on_click=move |_| open_diff(full.clone())>
                                        <Icon name="Eye" class="size-3.5"/>
                                        "Diff anzeigen"
                                    </Button>
                                    <Button
                                        size=ButtonSize::Sm
                                        variant=ButtonVariant::Outline
                                        class="shrink-0"
                                        on_click=move |_| restore_confirm.set(Some(full2.clone()))
                                    >
                                        <Icon name="History" class="size-3.5"/>
                                        "Wiederherstellen"
                                    </Button>
                                })
                            }
                            _ => None,
                        };
                        view! {
                            <div class="flex items-center gap-2 rounded-md px-2 py-1 text-sm hover:bg-muted">
                                <Icon name="FileText" class="size-4 shrink-0 text-muted-foreground"/>
                                <span class="flex-1 truncate">{entry.name.clone()}</span>
                                {timestamp.clone().map(|t| view! { <span class="shrink-0 text-xs text-muted-foreground">{t}</span> })}
                                {actions}
                            </div>
                        }.into_any()
                    }).collect_view()
                }}
                <Show when=move || entries.with(|e| e.as_ref().is_some_and(|e| e.is_empty()))>
                    <p class="p-2 text-sm text-muted-foreground">"Leerer Ordner."</p>
                </Show>
            </div>

            {move || diff_path.get().map(|path| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="flex max-h-[80vh] w-[52rem] flex-col gap-3 rounded-lg border border-border bg-card p-4">
                        <div class="flex items-center justify-between">
                            <p class="text-sm font-medium">"Diff: "{last_segment(&path)}</p>
                            <button on:click=move |_| diff_path.set(None) class="text-muted-foreground hover:text-foreground">
                                <Icon name="X" class="size-4"/>
                            </button>
                        </div>
                        <Show when=move || diff_loading.get()>
                            <p class="text-sm text-muted-foreground">"Lade…"</p>
                        </Show>
                        {move || diff_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}
                        {move || diff_data.get().map(|d| view! {
                            <p class="text-xs text-muted-foreground">
                                "Zielort: "<code>{d.target_path.clone()}</code>
                                {d.current_content.is_none().then_some(" - existiert dort aktuell nicht (würde neu angelegt).")}
                            </p>
                            {if d.is_binary {
                                view! {
                                    <p class="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground">
                                        "Binärdatei - kein Text-Diff möglich. Wiederherstellen kopiert den Inhalt trotzdem unverändert (Byte für Byte)."
                                    </p>
                                }.into_any()
                            } else {
                                view! { <DiffView old_text=d.current_content.clone().unwrap_or_default() new_text=d.backup_content.clone()/> }.into_any()
                            }}
                        })}
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| diff_path.set(None)>
                                "Schließen"
                            </Button>
                            <Show when=move || diff_data.with(|d| d.is_some())>
                                <Button on_click=move |_| {
                                    restore_confirm.set(diff_path.get_untracked());
                                    diff_path.set(None);
                                }>
                                    "Wiederherstellen…"
                                </Button>
                            </Show>
                        </div>
                    </div>
                </div>
            })}

            {move || restore_confirm.get().map(|path| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                <code>{last_segment(&path)}</code>
                                " wird an ihren ursprünglichen Ort zurückgeschrieben. Die aktuell dort liegende Datei wird dabei selbst zuerst gesichert, nicht einfach überschrieben."
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| restore_confirm.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button on_click=move |_| confirm_restore() disabled=restoring>
                                {move || if restoring.get() { "Stelle wieder her…" } else { "Wiederherstellen" }}
                            </Button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

/// "old" = aktuell auf dem Server, "new" = Inhalt der Backup-Datei (die
/// Richtung, in der ein Restore die Datei verändern würde).
#[component]
fn DiffView(old_text: String, new_text: String) -> impl IntoView {
    let parts = diff_lines(&old_text, &new_text);
    if !has_changes(&parts) {
        return view! {
            <p class="rounded-md border border-dashed border-border p-3 text-sm text-muted-foreground">
                "Kein Unterschied - Backup und aktuelle Datei sind inhaltlich identisch."
            </p>
        }
        .into_any();
    }
    view! {
        <pre class="min-h-0 flex-1 overflow-auto rounded-md border border-border bg-muted/20 p-2 font-mono text-xs">
            {parts.into_iter().map(|part| {
                let prefix = part.prefix();
                let cls = part.class();
                view! {
                    <span>
                        {part.lines.into_iter().map(|line| view! { <div class=cls>{prefix}" "{line}</div> }).collect_view()}
                    </span>
                }
            }).collect_view()}
        </pre>
    }
    .into_any()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timestamps() {
        assert_eq!(parse_timestamp("item_proto.20260825_120304.bak").as_deref(), Some("2026-08-25 12:03:04"));
        assert_eq!(parse_timestamp("a.txt.20260101_000000.deleted").as_deref(), Some("2026-01-01 00:00:00"));
        assert_eq!(parse_timestamp("a.txt.2026010_000000.bak"), None);
        assert_eq!(parse_timestamp("a.txt"), None);
    }

    #[test]
    fn crumbs_and_up() {
        assert!(breadcrumbs(Some("/r"), Some("/r")).is_empty());
        assert_eq!(breadcrumbs(Some("/r/a/b"), Some("/r")), vec!["a", "b"]);
        assert_eq!(go_up(Some("/r/a/b".into()), Some("/r")), Some("/r/a".into()));
        assert_eq!(go_up(Some("/r".into()), Some("/r")), Some("/r".into()));
    }
}
