//! Paket-Import (Format m2pkg v1): Befehle für Vorschau, Einspielen und
//! Rückgängigmachen. Die Logik steckt in `crate::m2pkg`.

use crate::activity_log;
use crate::m2pkg::{self, ApplyOptions, ApplyReport, Inspect, ImportRecord, Package, Preflight, UndoReport};
use crate::state::AppState;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, State};

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

/// Vorschaubilder (kleine PNG-Data-URLs) für höchstens 300 Icon-Pfade aus dem Paket.
#[tauri::command]
pub async fn m2pkg_icons(zip_path: String, paths: Vec<String>) -> Result<std::collections::HashMap<String, String>, String> {
    if paths.len() > 300 {
        return Err("Höchstens 300 Icons pro Anfrage.".into());
    }
    tokio::task::spawn_blocking(move || m2pkg::icon_thumbnails(Path::new(&zip_path), &paths))
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

/// Packt die angegebenen `pack/<Ordner>` neu (`.epk`), mit dem Packer aus dem Client.
async fn repack_folders(app: &AppHandle, client_dir: &Path, folders: &[String]) -> Result<Vec<String>, String> {
    let tool = crate::client_unpack::find_eterpack_tool(&client_dir.to_string_lossy())
        .ok_or("EterPackConsoleLz4.exe ist im Client nicht zu finden")?;
    let tool = tool.to_string_lossy().into_owned();
    let mut done = Vec::new();
    for f in folders {
        crate::packtools::run_eterpack_pack(app, &tool, f).await?;
        done.push(f.clone());
    }
    Ok(done)
}

#[tauri::command]
pub async fn m2pkg_import(app: AppHandle, state: State<'_, AppState>, options: ApplyOptions) -> Result<ApplyReport, String> {
    let client_dir = PathBuf::from(client_path_setting(&state)?);
    let pkg = open_package(options.zip_path.clone()).await?;
    let pool = optional_pool(&state).await;
    let collection = pkg.manifest.collection_name();
    let zip_name = Path::new(&options.zip_path).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    let folders = if options.repack && options.write_files {
        m2pkg::touched_pack_folders(&pkg, &options.vnums)
    } else {
        Vec::new()
    };
    let mut report = m2pkg::import(pkg, client_dir.clone(), options, pool).await?;
    if !folders.is_empty() {
        match repack_folders(&app, &client_dir, &folders).await {
            Ok(done) => report.notes.push(format!("Client neu gepackt: {}. Das Spiel muss neu gestartet werden.", done.join(", "))),
            Err(e) => report.notes.push(format!(
                "Client NICHT neu gepackt ({e}). Neue Dateien sieht der Client erst nach dem Packen. Bitte den Ordner pack im Client-Werkzeug packen."
            )),
        }
    }
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
