// Liest einen lokal ausgewählten "System"-Paketordner (siehe
// system_patch.rs für die Dateikonvention) rekursiv ein, klassifiziert jede
// Datei grob (Server-Quellcode-Patch/Client-Quellcode-Patch/Client-
// Installationsdatei) und parst sie. Reines Dateisystem-Lesen + Delegation
// an system_patch - keine SSH/DB-Abhängigkeit, deshalb ohne AppState nutzbar.

use crate::system_installs::TargetKind;
use crate::system_patch::{self, ParsedSystemFile};
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct ScannedFile {
    /// Pfad relativ zum importierten Systemordner, mit `/` als Trenner
    /// (unabhängig vom Betriebssystem) - z.B. "Source/Server/game/cmd_gm.cpp"
    /// oder "ADDONS/EditLine Center Fix/ClientSrc-EterLib-GrpTextInstance.cpp".
    pub relative_path: String,
    /// `Some("EditLine Center Fix")`, wenn diese Datei unter `ADDONS/<name>/`
    /// liegt - wird in der UI als eigene, einzeln zuschaltbare Unterebene
    /// dargestellt statt Teil des Hauptsystems zu sein.
    pub addon_name: Option<String>,
    pub category: TargetKind,
    /// Nur der Dateiname, ohne Pfad - Grundlage für die Zielpfad-Suche
    /// (`ssh::find_remote_file_by_name` bzw. die lokale Entsprechung).
    pub filename: String,
    pub parsed: ParsedSystemFile,
    /// Nur gesetzt, wenn `parsed.ops` leer ist (keine search/add-Marker
    /// gefunden - Ganzdatei-Kandidat, z.B. ein komplett neues Python-UI-
    /// Skript). Das Frontend nutzt das als `AppendToEnd`-Inhalt gegen eine
    /// (noch) nicht existierende Zieldatei - bei bereits vorhandener
    /// Zieldatei wird das bewusst NICHT automatisch übernommen (kein
    /// Überschreiben einer bestehenden Datei ohne klare Herkunft).
    pub raw_content: Option<String>,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn to_relative_slash(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// Manche ADDONS-Unterordner verflachen den echten Pfad mit Bindestrichen
/// statt echter Unterordner zu nutzen (real beobachtet:
/// "ClientSrc-EterLib-GrpTextInstance.cpp" statt "ClientSrc/EterLib/...").
/// Für die Kategorisierung/Anzeige wird das wieder aufgelöst - die Datei
/// selbst bleibt unangetastet, das hier ist nur eine Lese-Hilfe.
fn unflatten_dashed_path(name: &str) -> String {
    const KNOWN_PREFIXES: [&str; 3] = ["ClientSrc-", "GameSource-", "Source-"];
    if !name.contains('/') && KNOWN_PREFIXES.iter().any(|p| name.starts_with(p)) {
        name.replace('-', "/")
    } else {
        name.to_string()
    }
}

/// Grobe Kategorie aus dem ersten (bzw. ersten zwei) Pfadsegment(en) - siehe
/// system_installs.rs::TargetKind für die Zielort-Entscheidung dahinter.
/// Kein Anspruch auf 100% Automatik (die Präfix-Konventionen unterscheiden
/// sich zwischen Systemen), der Nutzer kann die Kategorie/den Zielpfad pro
/// Datei jederzeit überschreiben.
pub fn categorize(relative_path: &str) -> TargetKind {
    let lower = relative_path.to_lowercase();
    let mut segments = lower.split('/');
    let first = segments.next().unwrap_or("");

    if first == "gamesource" {
        return TargetKind::LiveServer;
    }
    if first == "clientsrc" {
        return TargetKind::LocalClientSource;
    }
    if first == "source" {
        match segments.next().unwrap_or("") {
            "server" => return TargetKind::LiveServer,
            "client" => return TargetKind::LocalClientSource,
            _ => {}
        }
    }
    // "Client/..." (ohne "Source" davor) und alles sonst Unbekannte: die
    // häufigste/sicherste Annahme in beiden untersuchten Beispielsystemen.
    TargetKind::LocalClientInstall
}

/// Viele Archive (.rar/.zip) entpacken sich in einen einzelnen Ordner, der
/// nach dem Archiv selbst benannt ist - real beobachtet bei beiden
/// InGame-Admin-Teilsystemen (`v1.2_ap_mod_create_item_aslan/` als einziger
/// Eintrag, `ADDONS`/`Client`/`Source` erst eine Ebene tiefer). Wählt der
/// Nutzer den äußeren, entpackten Ordner statt des inneren Systemordners,
/// würde `categorize()` sonst den Wrapper-Namen als erstes Pfadsegment
/// sehen statt "Source"/"Client"/"ADDONS" und alles falsch einordnen.
/// Steigt deshalb so lange in einen einzelnen Unterordner ab, wie der
/// aktuelle Ordner exakt einen Eintrag enthält und dieser ein Ordner ist.
fn resolve_effective_root(root: &Path) -> PathBuf {
    let mut current = root.to_path_buf();
    loop {
        let Ok(mut entries) = std::fs::read_dir(&current) else {
            break;
        };
        let Some(Ok(only)) = entries.next() else { break };
        if entries.next().is_some() {
            break;
        }
        if !only.path().is_dir() {
            break;
        }
        current = only.path();
    }
    current
}

pub fn scan_system_package(root: &Path) -> Result<Vec<ScannedFile>, String> {
    if !root.is_dir() {
        return Err(format!("Ordner nicht gefunden: {}", root.display()));
    }
    let root = &resolve_effective_root(root);

    let mut paths = Vec::new();
    walk(root, &mut paths);

    let mut result = Vec::new();
    for path in paths {
        let relative_path = to_relative_slash(root, &path);

        let (addon_name, path_for_category) = match relative_path.strip_prefix("ADDONS/") {
            Some(rest) => {
                let mut parts = rest.splitn(2, '/');
                let name = parts.next().unwrap_or("").to_string();
                let inner = unflatten_dashed_path(parts.next().unwrap_or(""));
                (Some(name), inner)
            }
            None => (None, relative_path.clone()),
        };

        // Binäre Dateien (Bilder, .rar/.zip-Reste) sind keine Patch-
        // Kandidaten und werden übersprungen, statt einen Fehler für die
        // ganze Sichtung auszulösen - erkannt über das Vorhandensein von
        // NUL-Bytes (echter Text, auch cp1252, enthält praktisch nie welche;
        // dieselbe Heuristik wie z.B. bei git). Wichtig: NICHT einfach
        // `std::fs::read_to_string` nutzen und den Fehlerfall überspringen -
        // das hätte auch echte, aber nicht-UTF-8 Text-Dateien (z.B. deutsche
        // Kommentare/Locale-Strings in Windows-1252) lautlos aus der
        // Sichtung verschwinden lassen, ohne jeden Hinweis für den Nutzer.
        let Ok(raw) = std::fs::read(&path) else {
            continue;
        };
        if raw.contains(&0u8) {
            continue;
        }
        let content = crate::ssh::decode_bytes(raw);

        let filename = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default()
            .to_string();
        let category = categorize(&path_for_category);
        let parsed = system_patch::parse_system_file(&content);
        let raw_content = parsed.ops.is_empty().then(|| content.clone());

        result.push(ScannedFile { relative_path, addon_name, category, filename, parsed, raw_content });
    }

    Ok(result)
}

/// Sucht lokal (nicht über SSH) rekursiv nach Dateien mit genau diesem
/// Namen - Gegenstück zu `ssh::find_remote_file_by_name`, für
/// `binary_src_path`/`client_path`. Nur für gezielte Einzelsuche (z.B. der
/// "Erneut suchen"-Knopf im Frontend) - für viele Dateinamen auf einmal
/// unbedingt `find_local_files_by_names` nutzen, siehe dort.
pub fn find_local_file_by_name(root: &Path, filename: &str) -> Vec<String> {
    let mut paths = Vec::new();
    walk(root, &mut paths);
    paths
        .into_iter()
        .filter(|p| p.file_name().and_then(|f| f.to_str()) == Some(filename))
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .collect()
}

/// Wie `find_local_file_by_name`, aber sucht viele Dateinamen in einem
/// einzigen Verzeichnis-Durchlauf statt den Baum für jede Datei erneut
/// abzulaufen. Wichtig für `client_path`: reale Client-Ordner haben oft
/// mehrere zehntausend Dateien (Texturen, Sounds, Modelle) - ein System-
/// Paket mit einem Dutzend Dateien hat den kompletten Baum sonst ein
/// Dutzend Mal durchsucht, was real mehrere Minuten dauern kann.
pub fn find_local_files_by_names(
    root: &Path,
    filenames: &std::collections::HashSet<String>,
) -> std::collections::HashMap<String, Vec<String>> {
    let mut paths = Vec::new();
    walk(root, &mut paths);

    let mut result: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    for p in paths {
        let Some(name) = p.file_name().and_then(|f| f.to_str()) else {
            continue;
        };
        if filenames.contains(name) {
            result
                .entry(name.to_string())
                .or_default()
                .push(p.to_string_lossy().replace('\\', "/"));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn scan_reads_windows_1252_text_files_instead_of_silently_dropping_them() {
        // Reale Live-Meldung eines Nutzers: eine cp1252-kodierte Datei
        // (z.B. deutscher Kommentar/Locale-String im Systempaket) brach
        // `read_to_string` bisher hart ab - der `let Ok(..) else { continue }`
        // hat sie dann lautlos aus der Sichtung verschworen, ganz ohne
        // Hinweis für den Nutzer.
        let scratch = std::env::temp_dir().join(format!("m2manager_systemscan_cp1252_test_{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        let bytes = vec![b'/', b'/', b' ', b'K', 0xE4, b'f', b'e', b'r', b'\n']; // "// Käfer" in cp1252
        std::fs::write(scratch.join("comment.cpp"), &bytes).unwrap();

        let files = scan_system_package(&scratch).expect("scan failed");
        assert_eq!(files.len(), 1);
        assert!(files[0].raw_content.as_deref() == Some("// Käfer\n"));

        std::fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn scan_skips_genuine_binary_files_with_nul_bytes() {
        let scratch = std::env::temp_dir().join(format!("m2manager_systemscan_binary_test_{}", std::process::id()));
        std::fs::create_dir_all(&scratch).unwrap();
        std::fs::write(scratch.join("icon.tga"), [0x00u8, 0x01, 0x02, 0x00, 0xFF]).unwrap();

        let files = scan_system_package(&scratch).expect("scan failed");
        assert!(files.is_empty());

        std::fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn find_local_files_by_names_resolves_several_names_in_one_walk() {
        let scratch = std::env::temp_dir().join(format!("m2manager_systemscan_batchfind_test_{}", std::process::id()));
        std::fs::create_dir_all(scratch.join("EterLib")).unwrap();
        std::fs::create_dir_all(scratch.join("UserInterface")).unwrap();
        std::fs::write(scratch.join("EterLib").join("GrpDevice.cpp"), "x").unwrap();
        std::fs::write(scratch.join("UserInterface").join("Defines.h"), "x").unwrap();
        std::fs::write(scratch.join("UserInterface").join("Unrelated.cpp"), "x").unwrap();

        let wanted: HashSet<String> = ["GrpDevice.cpp".to_string(), "Defines.h".to_string(), "Missing.cpp".to_string()]
            .into_iter()
            .collect();
        let found = find_local_files_by_names(&scratch, &wanted);

        assert_eq!(found.len(), 2);
        assert!(found["GrpDevice.cpp"][0].ends_with("EterLib/GrpDevice.cpp"));
        assert!(found["Defines.h"][0].ends_with("UserInterface/Defines.h"));
        assert!(!found.contains_key("Missing.cpp"));
        assert!(!found.contains_key("Unrelated.cpp"));

        std::fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn categorizes_source_server_as_live_server() {
        assert_eq!(categorize("Source/Server/game/cmd_gm.cpp"), TargetKind::LiveServer);
    }

    #[test]
    fn categorizes_gamesource_prefix_as_live_server() {
        assert_eq!(categorize("GameSource/game/cmd_gm.cpp"), TargetKind::LiveServer);
    }

    #[test]
    fn categorizes_source_client_as_local_client_source() {
        assert_eq!(categorize("Source/Client/GameLib/ItemManager.cpp"), TargetKind::LocalClientSource);
    }

    #[test]
    fn categorizes_clientsrc_prefix_as_local_client_source() {
        assert_eq!(categorize("ClientSrc/EterLib/GrpDevice.cpp"), TargetKind::LocalClientSource);
    }

    #[test]
    fn categorizes_bare_client_prefix_as_local_client_install() {
        assert_eq!(categorize("Client/root/ui.py"), TargetKind::LocalClientInstall);
    }

    #[test]
    fn unflattens_dashed_addon_filenames_for_categorization() {
        let unflattened = unflatten_dashed_path("ClientSrc-EterLib-GrpTextInstance.cpp");
        assert_eq!(unflattened, "ClientSrc/EterLib/GrpTextInstance.cpp");
        assert_eq!(categorize(&unflattened), TargetKind::LocalClientSource);
    }

    #[test]
    fn leaves_already_nested_addon_paths_untouched() {
        assert_eq!(unflatten_dashed_path("Client/root/ui.py"), "Client/root/ui.py");
    }

    #[test]
    fn scan_unwraps_a_single_archive_wrapper_folder() {
        // Nachgebaute reale Struktur eines entpackten .rar-Archivs (siehe
        // InGame-Admin/v1.2_ap_mod_create_item_aslan): der gewählte Ordner
        // enthält nur EINEN Eintrag, der wiederum den echten Systembaum
        // enthält. Ohne Auflösung würde "Source/game.cpp" als
        // "<wrapper>/Source/game.cpp" gesehen und falsch kategorisiert.
        let scratch = std::env::temp_dir().join(format!("m2manager_systemscan_wrap_test_{}", std::process::id()));
        let inner = scratch.join("v1.2_wrapper_name");
        std::fs::create_dir_all(inner.join("Source/Server/game")).unwrap();
        std::fs::create_dir_all(inner.join("Client/root")).unwrap();
        std::fs::write(inner.join("Source/Server/game/cmd.cpp"), "// add on the end of this file\nint x;\n").unwrap();
        std::fs::write(inner.join("Client/root/ui.py"), "pass\n").unwrap();

        let mut files = scan_system_package(&scratch).expect("scan failed");
        files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].relative_path, "Client/root/ui.py");
        assert_eq!(files[1].relative_path, "Source/Server/game/cmd.cpp");
        assert_eq!(files[1].category, TargetKind::LiveServer);

        std::fs::remove_dir_all(&scratch).unwrap();
    }

    #[test]
    fn scan_does_not_unwrap_when_root_already_has_multiple_entries() {
        let scratch = std::env::temp_dir().join(format!("m2manager_systemscan_nowrap_test_{}", std::process::id()));
        std::fs::create_dir_all(scratch.join("Source")).unwrap();
        std::fs::create_dir_all(scratch.join("Client")).unwrap();
        std::fs::write(scratch.join("Source").join("a.cpp"), "int x;\n").unwrap();

        let files = scan_system_package(&scratch).expect("scan failed");
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "Source/a.cpp");

        std::fs::remove_dir_all(&scratch).unwrap();
    }
}
