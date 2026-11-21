//! Gemeinsame Dateihelfer für Client-Dateien (und lokale Textdateien):
//! atomares Schreiben, eindeutige Sicherungen, Locale-Ordner-Suche,
//! Zeilenenden und die einstellbaren Codepages.
//!
//! Die Codepages stehen in den Einstellungen `client_codepage` (Client-Dateien
//! wie `item_list.txt`/`itemdesc.txt`) und `server_codepage` (Serverdateien,
//! die per SFTP gelesen werden). Sie sind global im Prozess hinterlegt und
//! werden von `settings::set_path`/`init_db` nachgeführt, damit die vielen
//! Dateifunktionen keinen zusätzlichen Parameter brauchen.

use encoding_rs::Encoding;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

pub const CLIENT_CODEPAGE_KEY: &str = "client_codepage";
pub const SERVER_CODEPAGE_KEY: &str = "server_codepage";

/// (Einstellungswert, Anzeigename) - Windows-1252 ist der Standard.
pub const CODEPAGES: &[(&str, &str)] = &[
    ("windows-1252", "Windows-1252 (Westeuropa, Standard)"),
    ("windows-1250", "Windows-1250 (Mitteleuropa, PL/CZ)"),
    ("windows-1254", "Windows-1254 (Türkisch)"),
    ("windows-1251", "Windows-1251 (Kyrillisch)"),
    ("euc-kr", "EUC-KR / CP949 (Koreanisch)"),
    ("windows-1253", "Windows-1253 (Griechisch)"),
    ("utf-8", "UTF-8"),
];

static CLIENT_CP: AtomicUsize = AtomicUsize::new(0);
static SERVER_CP: AtomicUsize = AtomicUsize::new(0);

fn index_of(value: &str) -> usize {
    let v = value.trim().to_ascii_lowercase();
    CODEPAGES.iter().position(|(k, _)| *k == v).unwrap_or(0)
}

/// Encoding zu einem Einstellungswert; unbekannt/leer = Windows-1252.
pub fn encoding_for(value: &str) -> &'static Encoding {
    Encoding::for_label(CODEPAGES[index_of(value)].0.as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Aktuell eingestellte Codepage der Client-Dateien.
pub fn client_encoding() -> &'static Encoding {
    encoding_for(CODEPAGES[CLIENT_CP.load(Ordering::Relaxed)].0)
}

/// Aktuell eingestellte Codepage der Serverdateien.
pub fn server_encoding() -> &'static Encoding {
    encoding_for(CODEPAGES[SERVER_CP.load(Ordering::Relaxed)].0)
}

/// Wird bei jeder geänderten Einstellung aufgerufen (leerer Wert = Standard).
pub fn on_setting_changed(key: &str, value: &str) {
    match key {
        CLIENT_CODEPAGE_KEY => CLIENT_CP.store(index_of(value), Ordering::Relaxed),
        SERVER_CODEPAGE_KEY => SERVER_CP.store(index_of(value), Ordering::Relaxed),
        _ => {}
    }
}

/// Bytes -> Text in der Codepage. Ein BOM wird nicht ausgewertet, damit
/// Rundreisen byte-genau bleiben.
pub fn decode(bytes: &[u8], enc: &'static Encoding) -> String {
    enc.decode_without_bom_handling(bytes).0.into_owned()
}

/// Text -> Bytes. Zeichen, die die Codepage nicht darstellen kann, sind ein
/// Fehler (statt `&#...;`-Ersatz); die Meldung nennt die Zeichen.
pub fn encode(text: &str, enc: &'static Encoding) -> Result<Vec<u8>, String> {
    let (bytes, _, had_errors) = enc.encode(text);
    if !had_errors {
        return Ok(bytes.into_owned());
    }
    let mut bad: Vec<char> = Vec::new();
    for c in text.chars() {
        let mut buf = [0u8; 4];
        if enc.encode(c.encode_utf8(&mut buf)).2 && !bad.contains(&c) {
            bad.push(c);
            if bad.len() >= 5 {
                break;
            }
        }
    }
    let list: Vec<String> = bad.iter().map(|c| format!("„{c}“")).collect();
    Err(format!(
        "Der Text enthält Zeichen, die in der Codepage {} nicht darstellbar sind ({}). Bitte entfernen/ersetzen oder unter Einstellungen eine passende Codepage wählen.",
        enc.name(),
        list.join(", ")
    ))
}

/// Zeilenende des Textes: CRLF, wenn es irgendwo vorkommt, sonst LF; ohne
/// Zeilenumbruch `default`.
pub fn detect_eol_or(text: &str, default: &'static str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else if text.contains('\n') {
        "\n"
    } else {
        default
    }
}

/// Wie `detect_eol_or` mit CRLF (Windows-Client-Standard) als Vorgabe.
pub fn detect_eol(text: &str) -> &'static str {
    detect_eol_or(text, "\r\n")
}

/// Wandelt Text ins gewünschte Zeilenende um.
pub fn with_eol(text: &str, eol: &str) -> String {
    if eol == "\r\n" {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text.to_string()
    }
}

/// Schreibt atomar: Temp-Datei im selben Ordner, `sync_all`, dann `rename`.
/// Bei jedem Fehler wird die Temp-Datei entfernt, das Ziel bleibt unverändert.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let mut name = path.file_name().map(|f| f.to_os_string()).unwrap_or_default();
    name.push(".m2pkg-tmp");
    let tmp = path.with_file_name(name);
    let res = (|| -> std::io::Result<()> {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.flush()?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, path)
    })();
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("{}: {e}", path.display()));
    }
    Ok(())
}

/// Kopiert `path` vor dem Überschreiben nach `m2manager_backups` neben der
/// Datei, mit Zeitstempel (Millisekunden) und Zähler bei Kollisionen - eine
/// vorhandene Sicherung wird nie überschrieben. `None`, wenn es nichts zu
/// sichern gibt.
pub fn backup_file(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let parent = path.parent().ok_or("Datei hat kein übergeordnetes Verzeichnis")?;
    let backup_dir = parent.join("m2manager_backups");
    std::fs::create_dir_all(&backup_dir).map_err(|e| e.to_string())?;

    let filename = path.file_name().and_then(|f| f.to_str()).ok_or("Ungültiger Dateiname")?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S_%3f").to_string();

    // Den Namen per create_new reservieren, damit nie etwas überschrieben wird.
    let mut counter = 0u32;
    let dest = loop {
        let name = if counter == 0 { format!("{filename}.{timestamp}.bak") } else { format!("{filename}.{timestamp}-{counter}.bak") };
        let candidate = backup_dir.join(name);
        match std::fs::OpenOptions::new().write(true).create_new(true).open(&candidate) {
            Ok(_) => break candidate,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => counter += 1,
            Err(e) => return Err(e.to_string()),
        }
    };

    if let Err(e) = std::fs::copy(path, &dest) {
        let _ = std::fs::remove_file(&dest);
        return Err(e.to_string());
    }
    Ok(Some(dest))
}

/// Alle Unterordner von `<client>/locale`, sortiert.
pub fn locale_dirs(client_path: &str) -> Result<Vec<PathBuf>, String> {
    let locale_dir = Path::new(client_path).join("locale");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&locale_dir)
        .map_err(|e| format!("Locale-Ordner {locale_dir:?} nicht lesbar: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    Ok(dirs)
}

/// `locale/<Sprache>/<file_name>` für jede Sprache, in der die Datei existiert.
pub fn locale_lang_files(client_path: &str, file_name: &str) -> Result<Vec<PathBuf>, String> {
    Ok(locale_dirs(client_path)?.into_iter().map(|d| d.join(file_name)).filter(|p| p.is_file()).collect())
}

/// Findet eine Textdatei des Clients automatisch: in jedem `locale/<Sprache>/`,
/// direkt in `locale/` und im Client-Ordner selbst.
pub fn locale_files(client_dir: &Path, file_name: &str) -> Vec<PathBuf> {
    let mut found = locale_lang_files(&client_dir.to_string_lossy(), file_name).unwrap_or_default();
    let locale = client_dir.join("locale");
    for p in [locale.join(file_name), client_dir.join(file_name)] {
        if p.is_file() {
            found.push(p);
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("m2m_clientfs_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn polish_l_stroke_roundtrips_in_1250() {
        let enc = encoding_for("windows-1250");
        let bytes = encode("Miecz Łukasza ł", enc).unwrap();
        assert_eq!(decode(&bytes, enc), "Miecz Łukasza ł");
        assert!(bytes.contains(&0xB3), "ł ist 0xB3 in Windows-1250");
    }

    #[test]
    fn turkish_s_cedilla_roundtrips_in_1254() {
        let enc = encoding_for("windows-1254");
        let bytes = encode("Kılıç şah", enc).unwrap();
        assert_eq!(decode(&bytes, enc), "Kılıç şah");
    }

    #[test]
    fn l_stroke_fails_in_1252_and_names_the_character() {
        let err = encode("Miecz ł", encoding_for("windows-1252")).unwrap_err();
        assert!(err.contains("„ł“"), "{err}");
        assert!(!err.contains("&#"));
    }

    #[test]
    fn korean_cyrillic_greek_and_utf8_work() {
        for (label, text) in [("euc-kr", "검"), ("windows-1251", "Меч"), ("windows-1253", "Ξίφος"), ("utf-8", "龍 ł")] {
            let enc = encoding_for(label);
            assert_eq!(decode(&encode(text, enc).unwrap(), enc), text, "{label}");
        }
    }

    #[test]
    fn unknown_value_falls_back_to_1252() {
        assert_eq!(encoding_for("").name(), "windows-1252");
        assert_eq!(encoding_for("quatsch").name(), "windows-1252");
        assert_eq!(encoding_for("UTF-8").name(), "UTF-8");
    }

    #[test]
    fn eol_detection_and_conversion() {
        assert_eq!(detect_eol("a\r\nb\r\n"), "\r\n");
        assert_eq!(detect_eol("a\nb\n"), "\n");
        assert_eq!(detect_eol("x"), "\r\n");
        assert_eq!(detect_eol_or("x", "\n"), "\n");
        assert_eq!(with_eol("a\nb\n", "\r\n"), "a\r\nb\r\n");
        assert_eq!(with_eol("a\r\nb\n", "\r\n"), "a\r\nb\r\n");
        assert_eq!(with_eol("a\nb\n", "\n"), "a\nb\n");
    }

    #[test]
    fn write_atomic_replaces_and_leaves_no_temp() {
        let dir = temp_dir("atomic");
        let f = dir.join("a.txt");
        std::fs::write(&f, b"alt").unwrap();
        write_atomic(&f, b"neu").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"neu");
        assert!(!dir.join("a.txt.m2pkg-tmp").exists());
        let d = dir.join("ordner");
        std::fs::create_dir_all(&d).unwrap();
        assert!(write_atomic(&d, b"x").is_err());
        assert!(!dir.join("ordner.m2pkg-tmp").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn backups_never_overwrite_each_other() {
        let dir = temp_dir("backup");
        let f = dir.join("item_list.txt");
        assert_eq!(backup_file(&f).unwrap(), None);
        std::fs::write(&f, b"x").unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..20 {
            let p = backup_file(&f).unwrap().unwrap();
            assert!(seen.insert(p), "doppelter Backup-Name");
        }
        assert_eq!(std::fs::read_dir(dir.join("m2manager_backups")).unwrap().count(), 20);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn locale_search_finds_languages_root_and_client_dir() {
        let dir = temp_dir("locale");
        for l in ["de", "en", "tr"] {
            std::fs::create_dir_all(dir.join("locale").join(l)).unwrap();
        }
        std::fs::write(dir.join("locale/de/itemdesc.txt"), b"").unwrap();
        std::fs::write(dir.join("locale/tr/itemdesc.txt"), b"").unwrap();
        std::fs::write(dir.join("locale/itemdesc.txt"), b"").unwrap();
        std::fs::write(dir.join("itemdesc.txt"), b"").unwrap();
        let cp = dir.to_str().unwrap();

        assert_eq!(locale_dirs(cp).unwrap().len(), 3);
        let lang = locale_lang_files(cp, "itemdesc.txt").unwrap();
        assert_eq!(lang, vec![dir.join("locale/de/itemdesc.txt"), dir.join("locale/tr/itemdesc.txt")]);
        let all = locale_files(&dir, "itemdesc.txt");
        assert_eq!(all.len(), 4);
        assert_eq!(all[2], dir.join("locale/itemdesc.txt"));
        assert!(locale_dirs(dir.join("nix").to_str().unwrap()).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }
}
