//! "Client vorbereiten": a freshly installed Metin2 client only ships packed
//! archives (`pack/<name>.epk` + `.eix`). Everything this app reads from the
//! client - item icons (`pack/icon/icon/item/*.tga`), `item_list.txt`,
//! 3D models, maps - is read as *loose* files, so on a new setup icons and
//! models simply stayed empty. This module finds the archives that are not
//! unpacked yet and unpacks them with the user's own
//! `EterPackConsoleLz4.exe` (the same tool the Item Editor already uses for
//! repacking), without ever overwriting a loose file that already exists.

use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};

pub const OUTPUT_EVENT: &str = "client-unpack-output";
pub const PROGRESS_EVENT: &str = "client-unpack-progress";

#[derive(Debug, Clone, Serialize)]
pub struct ArchiveStatus {
    /// Archive name without extension, e.g. `icon` for `icon.epk/.eix`.
    pub name: String,
    /// Human-readable German description of what's inside.
    pub label: String,
    pub size_bytes: u64,
    /// A non-empty loose folder `pack/<name>` already exists.
    pub unpacked: bool,
    /// Needed by the app's core features (icons, item texts) - preselected.
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnpackStatus {
    pub pack_dir: String,
    pub pack_dir_exists: bool,
    pub tool_path: Option<String>,
    pub archives: Vec<ArchiveStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnpackResult {
    pub name: String,
    pub files_copied: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Progress<'a> {
    current: usize,
    total: usize,
    name: &'a str,
}

pub fn pack_dir(client_path: &str) -> PathBuf {
    Path::new(client_path).join("pack")
}

fn is_eterpack_exe(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.starts_with("eterpack") && lower.ends_with(".exe")
}

/// Looks for `EterPackConsoleLz4.exe` (or another `EterPack*.exe`) in the
/// client's `pack` folder first - where it normally lives and where the
/// repack step expects it - and then in the client folder itself. The exact
/// name `EterPackConsoleLz4.exe` wins over other matches.
pub fn find_eterpack_tool(client_path: &str) -> Option<PathBuf> {
    for dir in [pack_dir(client_path), PathBuf::from(client_path)] {
        let Ok(entries) = std::fs::read_dir(&dir) else { continue };
        let mut candidates: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_file())
            .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(is_eterpack_exe))
            .collect();
        candidates.sort_by_key(|p| {
            let exact = p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.eq_ignore_ascii_case("EterPackConsoleLz4.exe"));
            (!exact, p.clone())
        });
        if let Some(first) = candidates.into_iter().next() {
            return Some(first);
        }
    }
    None
}

fn archive_label(name: &str) -> &'static str {
    let lower = name.to_lowercase();
    if lower == "icon" {
        "Item-Icons"
    } else if lower.starts_with("locale") {
        "Texte, Item-Liste & Beschreibungen"
    } else if lower == "item" {
        "Item-Modelle (Waffen, Rüstungen)"
    } else if lower.starts_with("pc") {
        "Charakter-Modelle"
    } else if lower.starts_with("npc") || lower.starts_with("monster") || lower.starts_with("guild") {
        "NPC- & Monster-Modelle"
    } else if lower.starts_with("map") || lower.starts_with("outdoor") || lower.starts_with("indoor") || lower.starts_with("season") {
        "Karten & Umgebung"
    } else if lower.starts_with("effect") {
        "Effekte"
    } else if lower.starts_with("textureset") || lower.starts_with("terrain") || lower.starts_with("tree") || lower.starts_with("zone") {
        "Texturen & Landschaft"
    } else if lower.starts_with("sound") || lower.starts_with("bgm") {
        "Sounds & Musik"
    } else {
        "Sonstiges"
    }
}

fn is_recommended(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower == "icon" || lower.starts_with("locale")
}

fn dir_has_entries(dir: &Path) -> bool {
    std::fs::read_dir(dir).map(|mut e| e.next().is_some()).unwrap_or(false)
}

/// Every `pack/<name>.eix` with a matching `.epk`, sorted with the
/// recommended archives first.
pub fn archive_status(client_path: &str) -> Vec<ArchiveStatus> {
    let dir = pack_dir(client_path);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<ArchiveStatus> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let ext = path.extension()?.to_str()?;
            if !ext.eq_ignore_ascii_case("eix") {
                return None;
            }
            let name = path.file_stem()?.to_str()?.to_string();
            let epk = dir.join(format!("{name}.epk"));
            let size_bytes = std::fs::metadata(&epk).ok()?.len();
            Some(ArchiveStatus {
                label: archive_label(&name).to_string(),
                unpacked: dir_has_entries(&dir.join(&name)),
                recommended: is_recommended(&name),
                size_bytes,
                name,
            })
        })
        .collect();
    out.sort_by(|a, b| b.recommended.cmp(&a.recommended).then_with(|| a.name.cmp(&b.name)));
    out
}

pub fn status(client_path: &str, configured_tool: Option<String>) -> UnpackStatus {
    let dir = pack_dir(client_path);
    let tool_path = configured_tool
        .filter(|p| !p.is_empty() && Path::new(p).is_file())
        .or_else(|| find_eterpack_tool(client_path).map(|p| p.to_string_lossy().into_owned()));
    UnpackStatus {
        pack_dir_exists: dir.is_dir(),
        pack_dir: dir.to_string_lossy().into_owned(),
        tool_path,
        archives: archive_status(client_path),
    }
}

/// Archive names come from the frontend - only accept plain names of
/// archives that really exist in `pack/`, never paths.
fn validate_name(dir: &Path, name: &str) -> Result<(), String> {
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return Err(format!("Ungültiger Archivname: {name}"));
    }
    if !dir.join(format!("{name}.eix")).is_file() || !dir.join(format!("{name}.epk")).is_file() {
        return Err(format!("{name}.epk/.eix nicht gefunden in {}", dir.display()));
    }
    Ok(())
}

/// Unpacks the given archives one after another. A failing archive doesn't
/// stop the others; each gets its own result.
pub async fn unpack(app: &AppHandle, client_path: &str, tool_path: &str, names: &[String]) -> Vec<UnpackResult> {
    let dir = pack_dir(client_path);
    let tool = Path::new(tool_path);
    let mut results = Vec::with_capacity(names.len());

    for (i, name) in names.iter().enumerate() {
        let _ = app.emit(PROGRESS_EVENT, Progress { current: i + 1, total: names.len(), name });
        let outcome = match validate_name(&dir, name) {
            Err(e) => Err(e),
            Ok(()) => crate::packtools::extract_missing_from_archive(app, OUTPUT_EVENT, tool, &dir, name).await,
        };
        results.push(match outcome {
            Ok(Some(files_copied)) => UnpackResult { name: name.clone(), files_copied, error: None },
            Ok(None) => UnpackResult {
                name: name.clone(),
                files_copied: 0,
                error: Some(format!(
                    "EterPack lief durch, hat aber keinen Ordner \"{name}\" erzeugt - das Archiv ist evtl. anders aufgebaut oder verschlüsselt."
                )),
            },
            Err(e) => UnpackResult { name: name.clone(), files_copied: 0, error: Some(e) },
        });
    }

    crate::icons::invalidate_caches();
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("m2m_client_unpack_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("pack")).unwrap();
        dir
    }

    #[test]
    fn lists_archives_with_unpacked_state_and_recommended_first() {
        let client = scratch("status");
        let pack = client.join("pack");
        for name in ["icon", "item", "locale_de", "zzz"] {
            std::fs::write(pack.join(format!("{name}.eix")), b"x").unwrap();
            std::fs::write(pack.join(format!("{name}.epk")), b"xyz").unwrap();
        }
        // Index without data file -> ignored.
        std::fs::write(pack.join("broken.eix"), b"x").unwrap();
        // "item" is already unpacked, "zzz" only has an empty folder.
        std::fs::create_dir_all(pack.join("item").join("ymir work")).unwrap();
        std::fs::create_dir_all(pack.join("zzz")).unwrap();

        let list = archive_status(&client.to_string_lossy());
        let names: Vec<_> = list.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["icon", "locale_de", "item", "zzz"]);
        assert!(list[0].recommended && !list[0].unpacked);
        assert_eq!(list[0].size_bytes, 3);
        assert!(list[2].unpacked && !list[2].recommended);
        assert!(!list[3].unpacked);
        assert_eq!(list[0].label, "Item-Icons");
        let _ = std::fs::remove_dir_all(&client);
    }

    #[test]
    fn finds_eterpack_in_pack_folder_and_prefers_exact_name() {
        let client = scratch("tool");
        let client_str = client.to_string_lossy().into_owned();
        assert!(find_eterpack_tool(&client_str).is_none());

        std::fs::write(client.join("EterPackOld.exe"), b"x").unwrap();
        assert!(find_eterpack_tool(&client_str).unwrap().ends_with("EterPackOld.exe"));

        std::fs::write(client.join("pack").join("eterpack_custom.exe"), b"x").unwrap();
        std::fs::write(client.join("pack").join("EterPackConsoleLz4.exe"), b"x").unwrap();
        let found = find_eterpack_tool(&client_str).unwrap();
        assert!(found.ends_with("EterPackConsoleLz4.exe"));
        assert_eq!(found.parent().unwrap(), client.join("pack"));
        let _ = std::fs::remove_dir_all(&client);
    }

    #[test]
    fn rejects_path_like_or_missing_archive_names() {
        let client = scratch("validate");
        let pack = client.join("pack");
        std::fs::write(pack.join("icon.eix"), b"x").unwrap();
        std::fs::write(pack.join("icon.epk"), b"x").unwrap();
        assert!(validate_name(&pack, "icon").is_ok());
        assert!(validate_name(&pack, "../icon").is_err());
        assert!(validate_name(&pack, "sub\\icon").is_err());
        assert!(validate_name(&pack, "item").is_err());
        assert!(validate_name(&pack, "").is_err());
        let _ = std::fs::remove_dir_all(&client);
    }

    #[test]
    fn status_falls_back_to_detected_tool_when_configured_one_is_missing() {
        let client = scratch("status_tool");
        let client_str = client.to_string_lossy().into_owned();
        std::fs::write(client.join("pack").join("EterPackConsoleLz4.exe"), b"x").unwrap();
        let s = status(&client_str, Some("C:\\gibt\\es\\nicht.exe".into()));
        assert!(s.pack_dir_exists);
        assert!(s.tool_path.unwrap().ends_with("EterPackConsoleLz4.exe"));
        let _ = std::fs::remove_dir_all(&client);
    }
}
