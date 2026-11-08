//! Datenbank-Backups: `mysqldump` auf dem Server auslösen, auflisten,
//! wiederherstellen, löschen und den automatischen Zeitplan einstellen.

pub mod schedule;

use crate::components::{toast, Button, ButtonSize, ButtonVariant, Icon, PageHeader, Skeleton};
use crate::features::server_control::util::{js_number, js_number_string};
use crate::util::log_activity;
use crate::store::connection_profiles::connection_profiles;
use crate::store::navigation::EmbeddedModule;
use crate::tauri::{invoke, invoke0, invoke_unit};
use leptos::prelude::*;
use leptos::task::spawn_local;
use schedule::{next_due_at, parse_date_ms, SCHEDULE_HOURS_OPTIONS};
use serde::Deserialize;
use serde_json::json;

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct RemoteEntry {
    name: String,
    is_dir: bool,
}

/// Dateinamen sind immer `m2manager_backup_<YYYYMMDD_HHMMSS>.sql` (siehe
/// db_backup.rs::backup_filename) - der Zeitstempel lässt sich also ohne
/// eigenen stat-Aufruf pro Datei für die Anzeige zurücklesen.
pub fn parse_timestamp(filename: &str) -> Option<String> {
    let rest = filename.strip_prefix("m2manager_backup_")?.strip_suffix(".sql")?;
    let (date, time) = rest.split_once('_')?;
    if date.len() != 8 || time.len() != 6 || !date.bytes().chain(time.bytes()).all(|b| b.is_ascii_digit()) {
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

/// Wie `new Date(ms).toLocaleString("de-DE")`: "5.8.2026, 09:03:04".
fn de_locale_string(ms: f64) -> String {
    let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(ms));
    if d.get_time().is_nan() {
        return "Invalid Date".into();
    }
    format!(
        "{}.{}.{}, {:02}:{:02}:{:02}",
        d.get_date(),
        d.get_month() + 1,
        d.get_full_year(),
        d.get_hours(),
        d.get_minutes(),
        d.get_seconds()
    )
}

const DEFAULT_DIR: &str = "/usr/home/game/m2manager_db_backups";

#[component]
pub fn DbBackups() -> impl IntoView {
    let dir = RwSignal::new(DEFAULT_DIR.to_string());
    let entries = RwSignal::new(None::<Vec<RemoteEntry>>);
    let loading = RwSignal::new(false);
    let load_error = RwSignal::new(None::<String>);

    let creating = RwSignal::new(false);
    let create_confirm = RwSignal::new(false);

    let restore_target = RwSignal::new(None::<String>);
    let restoring = RwSignal::new(false);

    let delete_target = RwSignal::new(None::<String>);
    let deleting = RwSignal::new(false);

    let schedule_hours = RwSignal::new(0.0f64);
    let last_auto_at = RwSignal::new(None::<String>);

    let load = move || {
        let path = dir.get_untracked();
        loading.set(true);
        load_error.set(None);
        spawn_local(async move {
            match invoke::<Vec<RemoteEntry>>("list_backup_dir", &json!({ "path": path })).await {
                Ok(e) => entries.set(Some(e)),
                Err(e) => load_error.set(Some(e)),
            }
            loading.set(false);
        });
    };

    // Backup-Ordner/Zeitplan gehören zum Server-Profil -> nach Wechsel neu laden.
    let profiles = connection_profiles();
    Effect::new(move |_| {
        profiles.revision.track();
        spawn_local(async move {
            if let Ok(v) = invoke::<Option<String>>("get_setting", &json!({ "key": "db_backup_dir" })).await {
                dir.set(v.filter(|s| !s.is_empty()).unwrap_or_else(|| DEFAULT_DIR.to_string()));
            }
            load();
        });
        spawn_local(async move {
            if let Ok(v) = invoke::<Option<String>>("get_setting", &json!({ "key": "db_backup_schedule_hours" })).await {
                schedule_hours.set(v.map(|s| js_number(&s)).unwrap_or(0.0));
            }
        });
        spawn_local(async move {
            if let Ok(v) = invoke::<Option<String>>("get_setting", &json!({ "key": "db_backup_last_auto_at" })).await {
                last_auto_at.set(v);
            }
        });
    });

    let change_schedule = move |hours: f64| {
        let previous = schedule_hours.get_untracked();
        schedule_hours.set(hours);
        spawn_local(async move {
            if let Err(e) =
                invoke_unit("set_setting", &json!({ "key": "db_backup_schedule_hours", "value": js_number_string(hours) })).await
            {
                // Auswahl zurücksetzen, damit die Anzeige nicht einen Zeitplan vorgaukelt, der nie gespeichert wurde.
                schedule_hours.set(previous);
                toast::error(&format!("Backup-Zeitplan konnte nicht gespeichert werden: {e}"));
            }
        });
    };

    let run_create = move || {
        create_confirm.set(false);
        creating.set(true);
        spawn_local(async move {
            match invoke0::<String>("create_database_backup").await {
                Ok(path) => {
                    toast::success(&format!("Backup erstellt: {path}"));
                    log_activity(
                        EmbeddedModule::DbBackups.into(),
                        "create",
                        &format!("Datenbank-Backup erstellt: '{path}'"),
                        Some("backup"),
                        Some(&path),
                    );
                    load();
                }
                Err(e) => toast::error(&e),
            }
            creating.set(false);
        });
    };

    let run_restore = move || {
        let Some(path) = restore_target.get_untracked() else { return };
        restore_target.set(None);
        restoring.set(true);
        spawn_local(async move {
            match invoke_unit("restore_database_backup", &json!({ "backupPath": path })).await {
                Ok(()) => {
                    toast::success(&format!("Wiederhergestellt aus: {path}"));
                    log_activity(
                        EmbeddedModule::DbBackups.into(),
                        "restore",
                        &format!("Datenbank-Backup wiederhergestellt: '{path}' (überschreibt Live-DB)"),
                        Some("backup"),
                        Some(&path),
                    );
                }
                Err(e) => toast::error(&e),
            }
            restoring.set(false);
        });
    };

    let run_delete = move || {
        let Some(path) = delete_target.get_untracked() else { return };
        delete_target.set(None);
        deleting.set(true);
        spawn_local(async move {
            match invoke_unit("delete_database_backup", &json!({ "backupPath": path })).await {
                Ok(()) => {
                    log_activity(
                        EmbeddedModule::DbBackups.into(),
                        "delete",
                        &format!("Datenbank-Backup gelöscht: '{path}'"),
                        Some("backup"),
                        Some(&path),
                    );
                    load();
                }
                Err(e) => toast::error(&e),
            }
            deleting.set(false);
        });
    };

    let files = move || entries.with(|e| e.as_ref().map(|e| e.iter().filter(|x| !x.is_dir).cloned().collect::<Vec<_>>()).unwrap_or_default());

    let schedule_note = move || {
        let hours = schedule_hours.get();
        let last = last_auto_at.get();
        let mut out = String::new();
        if hours > 0.0 {
            let due = next_due_at(crate::util::format::now_ms(), last.as_deref(), hours).map(de_locale_string).unwrap_or_default();
            out.push_str(&format!(" Nächstes automatisches Backup fällig: {due}."));
        }
        if let Some(l) = last.filter(|s| !s.is_empty()) {
            let shown = parse_date_ms(&l).map(de_locale_string).unwrap_or_else(|| "Invalid Date".into());
            out.push_str(&format!(" Letztes automatisches Backup: {shown}."));
        }
        out
    };

    view! {
        <div class="space-y-4">
            <PageHeader module=EmbeddedModule::DbBackups title="Datenbank-Backups".to_string()/>
            <p class="max-w-4xl text-sm text-muted-foreground">
                "Löst "<code>"mysqldump"</code>" auf dem Server über die bestehende SSH-Verbindung aus, mit denselben MySQL-Zugangsdaten wie die eigene DB-Verbindung dieser App (setzt voraus, dass diese auch vom Server aus funktionieren - typischerweise der Fall, wenn Datenbank und Server auf derselben Maschine laufen). Andere Editoren sichern nur einzelne Dateien - das hier ist eine vollständige Momentaufnahme der Datenbanken, konfigurierbar unter Einstellungen → Server-Pfade."
            </p>

            <div class="flex items-center gap-2">
                <Button on_click=move |_| create_confirm.set(true) disabled=creating>
                    <Icon name="DatabaseBackup" class="size-4"/>
                    {move || if creating.get() { "Erstelle Backup…" } else { "Backup jetzt erstellen" }}
                </Button>
                <Button variant=ButtonVariant::Outline on_click=move |_| load() disabled=loading>
                    <Icon name="RefreshCw" class=Signal::derive(move || if loading.get() { "size-4 animate-spin".to_string() } else { "size-4 ".to_string() })/>
                    "Neu laden"
                </Button>
            </div>

            <div class="space-y-1.5 rounded-md border border-border p-3">
                <div class="flex items-center gap-2">
                    <label for="db-backup-schedule" class="text-sm font-medium">"Automatisches Backup"</label>
                    <select
                        id="db-backup-schedule"
                        on:change=move |e| change_schedule(js_number(&event_target_value(&e)))
                        class="rounded-md border border-border bg-background px-2 py-1 text-sm"
                    >
                        {SCHEDULE_HOURS_OPTIONS.iter().map(|h| {
                            let h = *h;
                            view! {
                                <option value=h.to_string() selected=move || schedule_hours.get() == h as f64>
                                    {if h == 0 { "Aus".to_string() } else { format!("Alle {h} Stunden") }}
                                </option>
                            }
                        }).collect_view()}
                    </select>
                </div>
                <p class="text-xs text-muted-foreground">
                    "Läuft nur, solange diese App geöffnet ist (kein echter Cron-Job auf dem Server) - wird beim nächsten periodischen Check nachgeholt, sobald die App wieder läuft."
                    {schedule_note}
                </p>
            </div>

            {move || load_error.get().map(|e| view! { <p class="text-sm text-destructive">{e}</p> })}

            <div class="space-y-1 rounded-md border border-border p-1">
                <For each=files key=|f| f.name.clone() let:f>
                    {
                        let timestamp = parse_timestamp(&f.name);
                        let n1 = f.name.clone();
                        let n2 = f.name.clone();
                        view! {
                            <div class="flex items-center gap-2 rounded-md px-2 py-1.5 text-sm hover:bg-muted">
                                <span class="flex-1 truncate font-mono text-xs">{f.name.clone()}</span>
                                {timestamp.map(|t| view! { <span class="shrink-0 text-xs text-muted-foreground">{t}</span> })}
                                <Button
                                    size=ButtonSize::Sm
                                    variant=ButtonVariant::Outline
                                    on_click=move |_| restore_target.set(Some(format!("{}/{}", dir.get_untracked(), n1)))
                                    disabled=restoring
                                >
                                    <Icon name="History" class="size-3.5"/>
                                    "Wiederherstellen"
                                </Button>
                                <Button
                                    size=ButtonSize::Sm
                                    variant=ButtonVariant::Ghost
                                    on_click=move |_| delete_target.set(Some(format!("{}/{}", dir.get_untracked(), n2)))
                                    disabled=deleting
                                >
                                    <Icon name="Trash2" class="size-3.5"/>
                                </Button>
                            </div>
                        }
                    }
                </For>
                <Show when=move || entries.with(|e| e.is_some()) && files().is_empty()>
                    <p class="p-2 text-sm text-muted-foreground">"Noch keine Backups."</p>
                </Show>
                <Show when=move || entries.with(|e| e.is_none()) && load_error.with(|e| e.is_none())>
                    <div class="space-y-1 p-1">
                        <Skeleton class="h-8 w-full"/>
                        <Skeleton class="h-8 w-full"/>
                        <Skeleton class="h-8 w-3/4"/>
                    </div>
                </Show>
            </div>

            <Show when=move || create_confirm.get()>
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                "Erstellt einen vollständigen "<code>"mysqldump"</code>" aller konfigurierten Datenbanken auf dem Server. Kann je nach Datenmenge etwas dauern."
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| create_confirm.set(false)>
                                "Abbrechen"
                            </Button>
                            <Button on_click=move |_| run_create()>"Erstellen"</Button>
                        </div>
                    </div>
                </div>
            </Show>

            {move || restore_target.get().map(|target| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-[28rem] space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                <strong>"Alle Daten in den gesicherten Datenbanken werden durch den Stand dieses Backups ersetzt."</strong>
                                " Das betrifft die komplette Live-Datenbank, nicht nur einzelne Zeilen, und kann nicht rückgängig gemacht werden außer durch ein neueres Backup. Datei: "
                                <code>{target}</code>
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| restore_target.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| run_restore() disabled=restoring>
                                {move || if restoring.get() { "Stelle wieder her…" } else { "Wirklich wiederherstellen" }}
                            </Button>
                        </div>
                    </div>
                </div>
            })}

            {move || delete_target.get().map(|target| view! {
                <div class="fixed inset-0 flex items-center justify-center bg-black/50">
                    <div class="w-96 space-y-3 rounded-lg border border-border bg-card p-4">
                        <p class="flex items-start gap-2 text-sm">
                            <Icon name="AlertTriangle" class="mt-0.5 size-4 shrink-0 text-destructive"/>
                            <span>
                                <code>{target}</code>
                                " wird endgültig gelöscht (kein weiteres Backup davon)."
                            </span>
                        </p>
                        <div class="flex justify-end gap-2">
                            <Button variant=ButtonVariant::Outline on_click=move |_| delete_target.set(None)>
                                "Abbrechen"
                            </Button>
                            <Button variant=ButtonVariant::Destructive on_click=move |_| run_delete() disabled=deleting>
                                "Löschen"
                            </Button>
                        </div>
                    </div>
                </div>
            })}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::parse_timestamp;

    #[test]
    fn timestamps() {
        assert_eq!(parse_timestamp("m2manager_backup_20260825_120304.sql").as_deref(), Some("2026-08-25 12:03:04"));
        assert_eq!(parse_timestamp("m2manager_backup_2026082_120304.sql"), None);
        assert_eq!(parse_timestamp("other.sql"), None);
    }
}
