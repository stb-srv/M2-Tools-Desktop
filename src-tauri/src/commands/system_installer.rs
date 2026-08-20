//! System-Installer.
//!
//! Baut fertige Community-"Systeme" (Server-/Client-Erweiterungen wie
//! ResizeWindow oder ein Admin-Panel-Modul) automatisiert ein - siehe
//! system_patch.rs für die verifizierte Paket-Konvention (search/add-
//! Marker) und system_installs.rs für die Zielort-Entscheidung
//! (Server-Quellcode live über SSH wie jedes andere Server-Datei-Werkzeug
//! hier, Client-Quellcode lokal im binary_src_path-Checkout, Client-
//! Installationsdateien im bestehenden client_path). Eigenständiges Modul,
//! leicht wieder entfernbar: nichts Bestehendes wird hier verändert außer
//! diesem einen Bereich.

use russh_sftp::client::SftpSession;
use crate::settings;
use crate::ssh;
use crate::state::AppState;
use crate::system_installs::{self, FileAction, InstalledFile, TargetKind};
use crate::system_patch::{self, InsertionResolution, PatchOp, Placement};
use crate::system_scan::{self, ScannedFile};
use tauri::State;

use super::support::{build_deploy_setting, client_path_setting, stored_ssh_auth};

fn binary_src_path_setting(state: &State<'_, AppState>) -> Result<String, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    settings::get_path(&conn, "binary_src_path")?
        .filter(|v| !v.is_empty())
        .ok_or_else(|| "Kein lokaler Client-Quellcode-Pfad (binary_src_path) konfiguriert.".to_string())
}

async fn read_target_content(
    state: &State<'_, AppState>,
    category: TargetKind,
    path: &str,
    shared_sftp: Option<&SftpSession>,
) -> Result<Option<String>, String> {
    match category {
        TargetKind::LiveServer => {
            if let Some(sftp) = shared_sftp {
                ssh::read_file_if_exists_via(sftp, path).await
            } else {
                let (config, auth) = stored_ssh_auth(state)?;
                ssh::read_remote_file_if_exists(&config, &auth, path).await
            }
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            let p = std::path::Path::new(path);
            if !p.exists() {
                return Ok(None);
            }
            if p.is_dir() {
                return Err(format!(
                    "\"{path}\" ist ein Ordner, keine Datei - bitte den vollständigen Pfad zur Zieldatei angeben (inkl. Dateiname)."
                ));
            }
            // Reale Client-Quellcode-/Installationsdateien sind wie die
            // Quest-Dateien auf dem Server oft nicht UTF-8 sondern
            // Windows-1252 (z.B. deutsche Umlaute in Kommentaren) - `
            // read_to_string` bricht dabei hart ab ("stream did not contain
            // valid UTF-8"), reale Live-Meldung eines Nutzers. Nutzt
            // denselben Fallback wie die SSH-Seite.
            let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
            Ok(Some(ssh::decode_bytes(bytes)))
        }
    }
}

async fn write_target_with_backup(
    state: &State<'_, AppState>,
    category: TargetKind,
    path: &str,
    content: &str,
    shared_sftp: Option<&SftpSession>,
) -> Result<Option<String>, String> {
    match category {
        TargetKind::LiveServer => {
            if let Some(sftp) = shared_sftp {
                ssh::write_file_with_backup_via(sftp, path, content).await
            } else {
                let (config, auth) = stored_ssh_auth(state)?;
                ssh::write_remote_file_with_backup(&config, &auth, path, content).await
            }
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            let p = std::path::Path::new(path);
            // Read before backup_file() renames the file away, same
            // rationale as write_remote_file_with_backup - a local client
            // source/install file can be Windows-1252 (see ssh::encode_matching)
            // just as easily as a remote one.
            let original_bytes = std::fs::read(p).ok();
            let backup = crate::packtools::backup_file(p)?;
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(p, ssh::encode_matching(content, original_bytes.as_deref()))
                .map_err(|e| e.to_string())?;
            Ok(backup.map(|b| b.display().to_string()))
        }
    }
}

async fn delete_target(state: &State<'_, AppState>, category: TargetKind, path: &str) -> Result<(), String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(state)?;
            ssh::delete_remote_file(&config, &auth, path).await
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            let p = std::path::Path::new(path);
            if p.exists() {
                std::fs::remove_file(p).map_err(|e| e.to_string())?;
            }
            Ok(())
        }
    }
}

async fn restore_from_backup(
    state: &State<'_, AppState>,
    category: TargetKind,
    backup_path: &str,
    target_path: &str,
) -> Result<(), String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(state)?;
            // Byte-exact, same reasoning as restore_remote_backup above -
            // this is a straight "put the backup back" operation, no text
            // interpretation needed or wanted.
            let content = ssh::read_remote_file_bytes(&config, &auth, backup_path).await?;
            ssh::write_remote_file_bytes_with_backup(&config, &auth, target_path, &content).await?;
            Ok(())
        }
        TargetKind::LocalClientSource | TargetKind::LocalClientInstall => {
            std::fs::copy(backup_path, target_path).map_err(|e| e.to_string())?;
            Ok(())
        }
    }
}

/// Liest ein lokal ausgewähltes Systempaket ein und klassifiziert/parst
/// jede enthaltene Datei - reines Dateisystem-Lesen, kein SSH nötig.
#[tauri::command]
pub fn scan_system_package(root: String) -> Result<Vec<ScannedFile>, String> {
    system_scan::scan_system_package(std::path::Path::new(&root))
}

/// Sucht die echte Zieldatei zu einem im Systempaket vorkommenden
/// Dateinamen - Server live über SSH, Client-Quellcode/-Installation lokal.
/// Liefert alle Treffer, der Aufrufer entscheidet bei Mehrdeutigkeit.
#[tauri::command]
pub async fn find_system_target(
    state: State<'_, AppState>,
    category: TargetKind,
    filename: String,
) -> Result<Vec<String>, String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(&state)?;
            let root = build_deploy_setting(&state, "build_live_source_root", "/usr/home/source/server")?;
            ssh::find_remote_file_by_name(&config, &auth, &root, &filename).await
        }
        TargetKind::LocalClientSource => {
            let root = binary_src_path_setting(&state)?;
            Ok(system_scan::find_local_file_by_name(std::path::Path::new(&root), &filename))
        }
        TargetKind::LocalClientInstall => {
            let root = client_path_setting(&state)?;
            Ok(system_scan::find_local_file_by_name(std::path::Path::new(&root), &filename))
        }
    }
}

/// Wie `find_system_target`, aber löst viele Dateinamen einer Kategorie in
/// einem Rutsch auf (ein Verzeichnis-Durchlauf bzw. ein `find`-Aufruf statt
/// einer pro Datei) - der eigentliche Grund für den Befehl: `client_path`
/// zeigt real oft auf einen kompletten Client-Ordner mit mehreren
/// zehntausend Dateien, ein Systempaket mit einem Dutzend Client-Dateien
/// hätte diesen Baum sonst ein Dutzend Mal komplett durchsucht. Wird beim
/// initialen Scannen eines Systempakets genutzt, `find_system_target`
/// bleibt für die gezielte Einzelsuche ("Erneut suchen").
#[tauri::command]
pub async fn find_system_targets_batch(
    state: State<'_, AppState>,
    category: TargetKind,
    filenames: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    match category {
        TargetKind::LiveServer => {
            let (config, auth) = stored_ssh_auth(&state)?;
            let root = build_deploy_setting(&state, "build_live_source_root", "/usr/home/source/server")?;
            ssh::find_remote_files_by_names(&config, &auth, &root, &filenames).await
        }
        TargetKind::LocalClientSource => {
            let root = binary_src_path_setting(&state)?;
            let wanted: std::collections::HashSet<String> = filenames.into_iter().collect();
            Ok(system_scan::find_local_files_by_names(std::path::Path::new(&root), &wanted))
        }
        TargetKind::LocalClientInstall => {
            let root = client_path_setting(&state)?;
            let wanted: std::collections::HashSet<String> = filenames.into_iter().collect();
            Ok(system_scan::find_local_files_by_names(std::path::Path::new(&root), &wanted))
        }
    }
}

/// Liest den aktuellen Inhalt eines bestätigten Zielpfads - Grundlage für
/// die Anker-Suche/Vorschau, bevor irgendetwas geschrieben wird.
#[tauri::command]
pub async fn read_system_target_file(
    state: State<'_, AppState>,
    category: TargetKind,
    path: String,
) -> Result<Option<String>, String> {
    read_target_content(&state, category, &path, None).await
}

/// Reine Anker-/Einfüge-Auflösung (keine Datei-I/O) - lässt das Frontend
/// live in der Vorschau anzeigen, ob/wo ein Block automatisch übernommen
/// werden könnte, ohne dafür extra einen SSH-Roundtrip zu brauchen (der
/// Inhalt wurde vorher schon einmal per `read_system_target_file` geholt).
#[tauri::command]
pub fn resolve_system_insertion(
    haystack: String,
    scope: Option<String>,
    anchor: String,
    placement: Placement,
) -> InsertionResolution {
    system_patch::resolve_insertion(&haystack, scope.as_deref(), &anchor, placement)
}

#[derive(serde::Deserialize)]
pub struct PlannedFile {
    pub target_path: String,
    pub category: TargetKind,
    /// Nur bereits vom Nutzer bestätigte `SearchInsert`/`AppendToEnd`-Blöcke
    /// - `FreeformInstruction` oder ein Block mit unsicherer Anker-Auflösung
    /// gehört hier nicht rein, das bleibt in der UI zur manuellen Prüfung.
    pub ops: Vec<PatchOp>,
}

#[derive(serde::Serialize)]
pub struct ApplyInstallResult {
    pub install_id: i64,
    pub warnings: Vec<String>,
}

async fn apply_one_file(
    state: &State<'_, AppState>,
    file: &PlannedFile,
    shared_sftp: Option<&SftpSession>,
) -> Result<(InstalledFile, Option<String>), String> {
    let existing = read_target_content(state, file.category, &file.target_path, shared_sftp).await?;
    let existed = existing.is_some();
    let mut content = existing.unwrap_or_default();

    for op in &file.ops {
        match op {
            PatchOp::AppendToEnd { code } => {
                if !content.is_empty() && !content.ends_with('\n') {
                    content.push('\n');
                }
                content.push_str(code);
                content.push('\n');
            }
            PatchOp::SearchInsert { scope, anchor, placement, code } => {
                match system_patch::resolve_insertion(&content, scope.as_deref(), anchor, *placement) {
                    InsertionResolution::Ready { line, .. } => {
                        content = system_patch::splice_lines(&content, line, code);
                    }
                    InsertionResolution::ReadyReplace { start_line, end_line, .. } => {
                        content = system_patch::replace_lines(&content, start_line, end_line, code);
                    }
                    InsertionResolution::NeedsReview { reason } => {
                        return Err(format!("{}: {reason} - bitte manuell prüfen.", file.target_path));
                    }
                }
            }
            PatchOp::FreeformInstruction { .. } => {
                return Err(format!(
                    "{}: Freitext-Block kann nicht automatisch angewendet werden.",
                    file.target_path
                ));
            }
        }
    }

    let warning = system_patch::check_structural_balance(&file.target_path, &content)
        .map(|w| format!("{}: {w}", file.target_path));

    let backup_path =
        write_target_with_backup(state, file.category, &file.target_path, &content, shared_sftp).await?;
    let installed = InstalledFile {
        target_path: file.target_path.clone(),
        target_kind: file.category,
        backup_path,
        action: if existed { FileAction::Patched } else { FileAction::Created },
    };
    Ok((installed, warning))
}

/// Schreibt alle übergebenen (bereits bestätigten) Änderungen in einem
/// Rutsch, mit Backup vor jedem Schreiben, und legt einen Verlaufs-Eintrag
/// an - die Grundlage für "Rückgängig machen". Bricht bei der ersten Datei
/// ab, die doch nicht automatisch anwendbar ist (sollte praktisch nicht
/// vorkommen, da das Frontend vorher nur geprüfte Blöcke schickt) - alles,
/// was bis dahin schon erfolgreich geschrieben wurde, landet trotzdem im
/// Verlauf, damit dafür kein Rückgängig-machen fehlt.
#[tauri::command]
pub async fn apply_system_install(
    state: State<'_, AppState>,
    system_name: String,
    files: Vec<PlannedFile>,
) -> Result<ApplyInstallResult, String> {
    // One shared SFTP session for every LiveServer file in this run instead
    // of a fresh SSH connect+login per file - real live report (2026-08-11):
    // a 10-server-file package meant 20 separate connect+login round-trips
    // (read+write each), and a single stalled one among those froze the
    // entire "Anwenden" step forever with no error shown (see the SSH
    // timeout added in ssh.rs for the other half of that fix). Only opened
    // if at least one file actually needs it - a purely client-side package
    // shouldn't require a live SSH connection at all.
    let shared_sftp = if files.iter().any(|f| f.category == TargetKind::LiveServer) {
        let (config, auth) = stored_ssh_auth(&state)?;
        Some(ssh::open_sftp(&config, &auth).await?)
    } else {
        None
    };

    let mut installed_files = Vec::new();
    let mut warnings = Vec::new();
    let mut first_error: Option<String> = None;

    for file in &files {
        match apply_one_file(&state, file, shared_sftp.as_ref()).await {
            Ok((installed, warning)) => {
                installed_files.push(installed);
                if let Some(w) = warning {
                    warnings.push(w);
                }
            }
            Err(e) => {
                first_error = Some(e);
                break;
            }
        }
    }

    let install_id = if !installed_files.is_empty() {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        Some(system_installs::record_install(&conn, &system_name, &installed_files)?)
    } else {
        None
    };

    if let Some(err) = first_error {
        return Err(match install_id {
            Some(id) => format!(
                "{err} (bereits geschriebene Dateien wurden unter Verlauf-Eintrag #{id} gesichert - dort ggf. rückgängig machen)"
            ),
            None => err,
        });
    }

    Ok(ApplyInstallResult { install_id: install_id.unwrap_or(0), warnings })
}

#[tauri::command]
pub fn list_system_installs(state: State<'_, AppState>) -> Result<Vec<system_installs::SystemInstall>, String> {
    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    system_installs::list_installs(&conn)
}

/// Stellt jede Datei eines Verlaufs-Eintrags aus ihrem Backup wieder her
/// (bzw. löscht neu angelegte Dateien) und entfernt danach den Eintrag -
/// Vorbild `undo_import_batch`.
#[tauri::command]
pub async fn undo_system_install(state: State<'_, AppState>, id: i64) -> Result<(), String> {
    let install = {
        let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
        system_installs::get_install(&conn, id)?
            .ok_or_else(|| format!("System-Installation {id} nicht gefunden (bereits entfernt?)"))?
    };

    for file in &install.files {
        match file.action {
            FileAction::Created => {
                delete_target(&state, file.target_kind, &file.target_path).await?;
            }
            FileAction::Patched => {
                let backup_path = file.backup_path.as_ref().ok_or_else(|| {
                    format!("Kein Backup für {} hinterlegt - manuelles Wiederherstellen nötig.", file.target_path)
                })?;
                restore_from_backup(&state, file.target_kind, backup_path, &file.target_path).await?;
            }
        }
    }

    let conn = state.settings_db.lock().map_err(|e| e.to_string())?;
    system_installs::delete_install_record(&conn, id)?;
    Ok(())
}
