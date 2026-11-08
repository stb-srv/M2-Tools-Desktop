//! Best-effort-Absturzmeldung: Solange die App offen ist, werden alle 60 s
//! die konfigurierten Server-Prozesse abgefragt (gleiches Kommando wie die
//! Ressourcen-Anzeige der Startseite). Wechseln sie von "gefunden" zu "nicht
//! gefunden", geht eine Webhook-Nachricht raus. Kein echter Watchdog - endet
//! mit dem Schließen der App. Ein einzelner fehlgeschlagener Abruf (z.B.
//! wackelige SSH-Verbindung) gilt als "gerade unbekannt", nicht als Absturz.
//!
//! Gleichzeitig der einzige periodische SSH-Abruf der App und damit Quelle
//! für den Ressourcen-Verlauf (`log_resource_snapshot`) - unabhängig davon,
//! ob ein Webhook eingerichtet ist. Außerdem Auslöser für geplante
//! DB-Backups: "geplant" heißt nur "fällig beim nächsten Tick, während die
//! App offen ist", nie ein echter Cron.

use crate::tauri::{invoke, invoke0, invoke_unit};
use crate::features::db_backups::schedule::is_backup_due;
use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

const POLL_INTERVAL_MS: u64 = 60_000;

#[derive(Deserialize)]
struct ProcessUsage {
    cpu_percent: f64,
}

#[derive(Deserialize)]
struct MemoryInfo {
    total_bytes: f64,
    free_bytes: f64,
}

#[derive(Deserialize)]
struct DiskUsage {
    capacity_percent: f64,
}

#[derive(Deserialize)]
struct ServerOverview {
    memory: Option<MemoryInfo>,
    disk: Option<DiskUsage>,
}

/// Wie `Number(x ?? 0)` in JS (leer = 0, ungültig = NaN).
pub fn js_number(s: Option<&str>) -> f64 {
    match s {
        None => 0.0,
        Some(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
    }
}

async fn get_setting(key: &str) -> Result<Option<String>, String> {
    invoke::<Option<String>>("get_setting", &json!({ "key": key })).await
}

async fn tick(was_running: Rc<Cell<Option<bool>>>) {
    // Geplantes DB-Backup.
    if let (Ok(schedule_hours), Ok(last_auto_at)) =
        (get_setting("db_backup_schedule_hours").await, get_setting("db_backup_last_auto_at").await)
    {
        let hours = js_number(schedule_hours.as_deref());
        if is_backup_due(js_sys::Date::now(), last_auto_at.as_deref(), hours) {
            // create_database_backup meldet Fehler selbst per Webhook - hier
            // nicht doppelt melden, einfach beim nächsten Tick neu versuchen.
            if invoke0::<String>("create_database_backup").await.is_ok() {
                let now = String::from(js_sys::Date::new_0().to_iso_string());
                let _ = invoke_unit("set_setting", &json!({ "key": "db_backup_last_auto_at", "value": now })).await;
            }
        }
    }
    // Einstellungen nicht lesbar - geplantes Backup einfach überspringen.

    // SSH nicht erreichbar o.ä. - als "unbekannt" behandeln, nicht als Absturz.
    let Ok(processes) = invoke0::<Vec<ProcessUsage>>("get_server_resource_usage").await else { return };
    let running = !processes.is_empty();
    if let Ok(webhook_url) = get_setting("webhook_url").await {
        if webhook_url.is_some_and(|u| !u.is_empty()) && was_running.get() == Some(true) && !running {
            let _ = invoke_unit(
                "notify_webhook_message",
                &json!({ "message": "M2Manager: Die überwachten Server-Prozesse sind verschwunden (evtl. Absturz)." }),
            )
            .await;
        }
    }
    was_running.set(Some(running));

    let cpu_percent: f64 = processes.iter().map(|p| p.cpu_percent).sum();
    // Übersicht nicht verfügbar - Snapshot einfach auslassen.
    if let Ok(overview) = invoke0::<ServerOverview>("get_server_overview").await {
        let _ = invoke_unit(
            "log_resource_snapshot",
            &json!({
                "cpuPercent": cpu_percent,
                "ramUsedBytes": overview.memory.as_ref().map(|m| m.total_bytes - m.free_bytes),
                "ramTotalBytes": overview.memory.as_ref().map(|m| m.total_bytes),
                "diskCapacityPercent": overview.disk.as_ref().map(|d| d.capacity_percent),
            }),
        )
        .await;
    }
}

#[component]
pub fn CrashWatch() -> impl IntoView {
    let was_running: Rc<Cell<Option<bool>>> = Rc::new(Cell::new(None));
    // `set_interval` verlangt Send-Closures; der Zustand lebt deshalb in
    // einem lokalen StoredValue.
    let state = StoredValue::new_local(was_running);
    let handle = set_interval_with_handle(
        move || {
            let wr = state.get_value();
            spawn_local(tick(wr));
        },
        Duration::from_millis(POLL_INTERVAL_MS),
    )
    .ok();
    on_cleanup(move || {
        if let Some(h) = handle {
            h.clear();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn number_like_js() {
        assert_eq!(js_number(None), 0.0);
        assert_eq!(js_number(Some("")), 0.0);
        assert_eq!(js_number(Some(" 12 ")), 12.0);
        assert!(js_number(Some("abc")).is_nan());
    }
}
