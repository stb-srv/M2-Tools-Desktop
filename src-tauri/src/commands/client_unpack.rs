//! "Client vorbereiten" (Einrichtungs-Assistent + Einstellungen): gepackte
//! Client-Archive erkennen und mit EterPackConsoleLz4.exe entpacken, damit
//! Item-Icons, Modelle und Texte auch bei einem frisch installierten Client
//! sofort angezeigt werden. Logik siehe `crate::client_unpack`.

use crate::client_unpack::{self, UnpackResult, UnpackStatus};
use crate::settings;
use crate::state::AppState;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, State};

static UNPACK_RUNNING: AtomicBool = AtomicBool::new(false);

fn non_empty_setting(state: &State<'_, AppState>, key: &str) -> Result<Option<String>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    Ok(settings::get_path(&conn, key)?.filter(|v| !v.is_empty()))
}

/// Explizit übergebener Client-Pfad (Assistent - dort ist er noch nicht
/// gespeichert) oder der gespeicherte.
fn resolve_client_path(state: &State<'_, AppState>, client_path: Option<String>) -> Result<String, String> {
    match client_path.filter(|p| !p.is_empty()) {
        Some(p) => Ok(p),
        None => non_empty_setting(state, "client_path")?.ok_or_else(|| "Kein Client-Pfad konfiguriert.".to_string()),
    }
}

#[tauri::command]
pub fn get_client_unpack_status(
    state: State<'_, AppState>,
    client_path: Option<String>,
) -> Result<UnpackStatus, String> {
    let client = resolve_client_path(&state, client_path)?;
    let configured = non_empty_setting(&state, "eterpack_tool_path")?;
    Ok(client_unpack::status(&client, configured))
}

#[tauri::command]
pub async fn unpack_client_archives(
    app: AppHandle,
    state: State<'_, AppState>,
    client_path: Option<String>,
    names: Vec<String>,
) -> Result<Vec<UnpackResult>, String> {
    let client = resolve_client_path(&state, client_path)?;
    let configured = non_empty_setting(&state, "eterpack_tool_path")?;
    let status = client_unpack::status(&client, configured.clone());
    let tool = status.tool_path.ok_or_else(|| {
        "EterPackConsoleLz4.exe wurde nicht gefunden. Lege sie in den pack-Ordner des Clients oder wähle sie unter Einstellungen › Client & Werkzeuge aus.".to_string()
    })?;

    // Automatisch gefundenes Werkzeug merken, damit auch das Neu-Packen im
    // Item Editor ohne weitere Einstellung funktioniert.
    if configured.as_deref() != Some(tool.as_str()) {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        settings::set_path(&conn, "eterpack_tool_path", &tool)?;
    }

    if UNPACK_RUNNING.swap(true, Ordering::SeqCst) {
        return Err("Es wird bereits entpackt - bitte warten, bis der laufende Vorgang fertig ist.".into());
    }
    let results = client_unpack::unpack(&app, &client, &tool, &names).await;
    UNPACK_RUNNING.store(false, Ordering::SeqCst);
    Ok(results)
}
