//! Paket-Import (Format m2pkg v1): Befehle für Vorschau, Einspielen und
//! Rückgängigmachen. Die Logik steckt in `crate::m2pkg`.

use crate::activity_log;
use crate::m2pkg::{self, ApplyOptions, ApplyReport, Inspect, ImportRecord, Package, Preflight, UndoReport};
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::State;

use super::support::client_path_setting;

async fn open_package(zip_path: String) -> Result<Package, String> {
    tokio::task::spawn_blocking(move || Package::open(Path::new(&zip_path)))
        .await
        .map_err(|e| e.to_string())?
}

async fn optional_pool(state: &State<'_, AppState>) -> Option<sqlx::MySqlPool> {
    state.mysql_pool.lock().await.clone()
}

/// Liest das Paket, prüft Format-Version und alle Prüfsummen und liefert die
/// Vorschau. Schreibt nichts.
#[tauri::command]
pub async fn m2pkg_inspect(zip_path: String) -> Result<Inspect, String> {
    let pkg = open_package(zip_path.clone()).await?;
    tokio::task::spawn_blocking(move || m2pkg::inspect(&pkg, &zip_path))
        .await
        .map_err(|e| e.to_string())?
}

/// Was gibt es schon? (Vnum in der Datenbank/item_list, abweichende Dateien)
#[tauri::command]
pub async fn m2pkg_preflight(state: State<'_, AppState>, options: ApplyOptions) -> Result<Preflight, String> {
    let client_dir = PathBuf::from(client_path_setting(&state)?);
    let pkg = open_package(options.zip_path.clone()).await?;
    let pool = optional_pool(&state).await;
    m2pkg::preflight(pkg, client_dir, options, pool).await
}

#[tauri::command]
pub async fn m2pkg_import(state: State<'_, AppState>, options: ApplyOptions) -> Result<ApplyReport, String> {
    let client_dir = PathBuf::from(client_path_setting(&state)?);
    let pkg = open_package(options.zip_path.clone()).await?;
    let pool = optional_pool(&state).await;
    let collection = pkg.manifest.collection_name();
    let zip_name = Path::new(&options.zip_path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let report = m2pkg::import(pkg, client_dir, options, pool).await?;
    if let Ok(conn) = state.settings_db.lock() {
        let _ = activity_log::record(
            &conn,
            "Paket-Import",
            "Paket eingespielt",
            Some("m2pkg"),
            Some(&report.backup_id),
            &format!("{collection}: {} Item(s) aus Paket „{zip_name}“", report.items_selected),
        );
    }
    Ok(report)
}

#[tauri::command]
pub fn m2pkg_list_imports(state: State<'_, AppState>) -> Result<Vec<ImportRecord>, String> {
    let client_dir = PathBuf::from(client_path_setting(&state)?);
    Ok(m2pkg::list_imports(&client_dir))
}

#[tauri::command]
pub async fn m2pkg_undo(state: State<'_, AppState>, id: String) -> Result<UndoReport, String> {
    let client_dir = PathBuf::from(client_path_setting(&state)?);
    let pool = optional_pool(&state).await;
    let report = m2pkg::undo(client_dir, id.clone(), pool).await?;
    if let Ok(conn) = state.settings_db.lock() {
        let _ = activity_log::record(
            &conn, "Paket-Import", "Paket-Import rückgängig gemacht", Some("m2pkg"), Some(&id),
            &format!("Import {id} zurückgenommen"),
        );
    }
    Ok(report)
}
