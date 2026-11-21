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
async fn repack_folders(
    app: &AppHandle,
    client_dir: &Path,
    folders: &[String],
    excluded: &std::collections::HashMap<String, Vec<String>>,
) -> Result<Vec<String>, String> {
    let tool = crate::client_unpack::find_eterpack_tool(&client_dir.to_string_lossy())
        .ok_or("EterPackConsoleLz4.exe ist im Client nicht zu finden")?;
    let tool = tool.to_string_lossy().into_owned();
    let mut done = Vec::new();
    for f in folders {
        let none = Vec::new();
        let ex = excluded.get(f).unwrap_or(&none);
        crate::packtools::run_eterpack_pack_excluding(app, &tool, f, ex).await?;
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
    let folders = if options.repack {
        m2pkg::touched_pack_folders(&pkg, &options.vnums)
    } else {
        Vec::new()
    };
    let mut report = m2pkg::import(pkg, client_dir.clone(), options, pool).await?;
    if !folders.is_empty() {
        match repack_folders(&app, &client_dir, &folders, &Default::default()).await {
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

/// Aus dem Journal: je Pack-Ordner die vom Import angelegten Dateien (relativ
/// zum Pack-Ordner). Sie werden beim Rückgängigmachen gelöscht und dürfen
/// beim Neupacken nicht aus dem alten Archiv zurückkommen.
fn created_pack_files(client_dir: &Path, id: &str) -> std::collections::HashMap<String, Vec<String>> {
    let mut out: std::collections::HashMap<String, Vec<String>> = Default::default();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit() || c == '_') {
        return out;
    }
    let path = ["m2manager_backups", "m2pkg", id, "journal.json"]
        .iter()
        .fold(client_dir.to_path_buf(), |p, c| p.join(c));
    let Ok(text) = std::fs::read_to_string(path) else { return out };
    let Ok(journal) = serde_json::from_str::<m2pkg::Journal>(&text) else { return out };
    for rel in &journal.files_created {
        if let Some(rest) = rel.strip_prefix("pack/") {
            if let Some((folder, inner)) = rest.split_once('/') {
                out.entry(folder.to_string()).or_default().push(inner.to_string());
            }
        }
    }
    out
}

#[tauri::command]
pub async fn m2pkg_undo(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<UndoReport, String> {
    let client_dir = PathBuf::from(client_path_setting(&state)?);
    let pool = optional_pool(&state).await;
    let created = {
        let (dir, id) = (client_dir.clone(), id.clone());
        tokio::task::spawn_blocking(move || created_pack_files(&dir, &id))
            .await
            .map_err(|e| e.to_string())?
    };
    let report = m2pkg::undo(client_dir.clone(), id.clone(), pool).await?;
    // Neupacken, damit die entfernten Dateien auch aus den .epk verschwinden.
    // UndoReport hat kein Notizfeld; das Ergebnis steht im Aktivitätsprotokoll.
    let mut repack_note = String::new();
    if !created.is_empty() {
        let mut folders: Vec<String> = created.keys().cloned().collect();
        folders.sort();
        repack_note = match repack_folders(&app, &client_dir, &folders, &created).await {
            Ok(done) => format!(" - Client neu gepackt: {}", done.join(", ")),
            Err(e) => format!(" - Client NICHT neu gepackt ({e}); Ordner pack im Client-Werkzeug neu packen"),
        };
    }
    if let Ok(conn) = state.settings_db.lock() {
        let _ = activity_log::record(
            &conn, "Paket-Import", "Paket-Import rückgängig gemacht", Some("m2pkg"), Some(&id),
            &format!("Import {id} zurückgenommen{repack_note}"),
        );
    }
    Ok(report)
}
