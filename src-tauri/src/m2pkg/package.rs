//! Paket lesen und prüfen: ZIP öffnen, `manifest.json` auswerten, Format-
//! Version prüfen, alle Prüfsummen aus `checksums.sha256` kontrollieren.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const SUPPORTED_FORMAT_VERSION: u64 = 1;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Warning {
    pub vnum: Option<u32>,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Assumption {
    pub field: String,
    pub bit: u64,
    pub name: String,
    #[serde(default)]
    pub basis: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssumedFlag {
    pub name: String,
    pub bit: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestFile {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub guessed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ManifestItem {
    pub vnum: u32,
    pub name: Option<String>,
    #[serde(rename = "type", default)]
    pub item_type: String,
    pub icon_path: Option<String>,
    pub model_path: Option<String>,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default)]
    pub assumed_antiflags: Vec<AssumedFlag>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Collection {
    pub id: Option<i64>,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub format: String,
    pub format_version: u64,
    pub dbtool_version: Option<String>,
    pub created: Option<String>,
    pub collection: Option<Collection>,
    pub client_version: Option<String>,
    pub proto_source: Option<String>,
    #[serde(default)]
    pub assumptions: Vec<Assumption>,
    #[serde(default)]
    pub items: Vec<ManifestItem>,
    #[serde(default)]
    pub warnings: Vec<Warning>,
}

impl Manifest {
    pub fn collection_name(&self) -> String {
        self.collection.as_ref().map(|c| c.name.clone()).unwrap_or_else(|| "Paket".into())
    }
}

/// Geöffnetes, vollständig geprüftes Paket. Das ZIP-Archiv bleibt offen, damit
/// Einzelzugriffe (Icons, Dateien) nicht bei jedem Mal das Zentralverzeichnis neu lesen.
pub struct Package {
    pub path: PathBuf,
    pub manifest: Manifest,
    /// Pfad im ZIP → Größe
    pub entries: BTreeMap<String, u64>,
    /// Pfad im ZIP → geprüfte SHA-256
    pub hashes: BTreeMap<String, String>,
    archive: Mutex<Archive>,
}

/// `..`, absolute Pfade, `\`, `:` und leere Teile sind in einem Paket nie erlaubt.
pub fn is_safe_zip_path(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.contains('\\')
        && !name.contains(':')
        && !name.contains('\0')
        && name.split('/').all(|p| !p.is_empty() && p != "." && p != "..")
}

pub fn sha256_reader(r: &mut impl Read) -> Result<String, String> {
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = r.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let mut f = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    sha256_reader(&mut f)
}

type Archive = zip::ZipArchive<File>;

fn open_archive(path: &Path) -> Result<Archive, String> {
    let file = File::open(path).map_err(|e| format!("ZIP-Datei nicht lesbar: {e}"))?;
    zip::ZipArchive::new(file).map_err(|e| format!("Das ist keine gültige ZIP-Datei: {e}"))
}

fn parse_checksums(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let (hash, rest) = line
            .split_once(' ')
            .ok_or_else(|| format!("checksums.sha256, Zeile {}: Format „Prüfsumme  Pfad“ erwartet", i + 1))?;
        let path = rest.trim_start_matches([' ', '*']);
        if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) || path.is_empty() {
            return Err(format!("checksums.sha256, Zeile {}: ungültige Prüfsumme oder Pfad", i + 1));
        }
        out.push((hash.to_ascii_lowercase(), path.to_string()));
    }
    Ok(out)
}

impl Package {
    /// Öffnet das ZIP und prüft Format, Version und **alle** Prüfsummen.
    /// Jeder Fehler bricht ab, bevor irgendetwas geschrieben wird.
    pub fn open(path: &Path) -> Result<Package, String> {
        let mut ar = open_archive(path)?;
        let mut entries = BTreeMap::new();
        for i in 0..ar.len() {
            let f = ar.by_index(i).map_err(|e| format!("ZIP-Eintrag nicht lesbar: {e}"))?;
            if f.is_dir() {
                continue;
            }
            let name = f.name().to_string();
            if !is_safe_zip_path(&name) {
                return Err(format!("Unsicherer Pfad im Paket: „{name}“. Das Paket wird nicht importiert."));
            }
            entries.insert(name, f.size());
        }

        if !entries.contains_key("manifest.json") {
            return Err("Das ist kein Import-Paket: manifest.json fehlt.".into());
        }
        let manifest_bytes = read_entry(&mut ar, "manifest.json")?;
        let raw: serde_json::Value =
            serde_json::from_slice(&manifest_bytes).map_err(|e| format!("manifest.json ist kein gültiges JSON: {e}"))?;
        if raw.get("format").and_then(|v| v.as_str()) != Some("m2pkg") {
            return Err("Das ist kein Import-Paket (format ist nicht „m2pkg“).".into());
        }
        let version = raw
            .get("format_version")
            .and_then(|v| v.as_u64())
            .ok_or("manifest.json: format_version fehlt oder ist keine Zahl.")?;
        if version > SUPPORTED_FORMAT_VERSION {
            return Err(format!(
                "Das Paket hat die Format-Version {version}, diese App kennt nur Version {SUPPORTED_FORMAT_VERSION}. \
                 Bitte M2 Tools Desktop aktualisieren."
            ));
        }
        if version == 0 {
            return Err("manifest.json: ungültige format_version 0.".into());
        }
        let manifest: Manifest =
            serde_json::from_value(raw).map_err(|e| format!("manifest.json ist unvollständig oder fehlerhaft: {e}"))?;

        let mut pkg = Package {
            path: path.to_path_buf(),
            manifest,
            entries,
            hashes: BTreeMap::new(),
            archive: Mutex::new(ar),
        };
        pkg.verify_checksums()?;
        pkg.verify_manifest_files()?;
        Ok(pkg)
    }

    fn verify_checksums(&mut self) -> Result<(), String> {
        if !self.entries.contains_key("checksums.sha256") {
            return Err("checksums.sha256 fehlt im Paket. Ohne Prüfsummen wird nicht importiert.".into());
        }
        let text = String::from_utf8(self.read("checksums.sha256")?)
            .map_err(|_| "checksums.sha256 ist kein gültiger UTF-8-Text".to_string())?;
        let listed = parse_checksums(&text)?;
        let mut problems: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (hash, name) in &listed {
            seen.insert(name.clone());
            self.hashes.insert(name.clone(), hash.clone());
            if !self.entries.contains_key(name) {
                problems.push(format!("{name}: fehlt im ZIP"));
                continue;
            }
            let actual = {
                let mut ar = self.lock_archive()?;
                let mut f = ar.by_name(name).map_err(|e| format!("{name}: {e}"))?;
                sha256_reader(&mut f)?
            };
            if &actual != hash {
                problems.push(format!("{name}: Prüfsumme stimmt nicht"));
            }
        }
        for name in self.entries.keys() {
            if name != "checksums.sha256" && !seen.contains(name) {
                problems.push(format!("{name}: steht nicht in checksums.sha256"));
            }
        }
        if problems.is_empty() {
            return Ok(());
        }
        let total = problems.len();
        problems.truncate(8);
        let more = if total > 8 { format!("\n… und {} weitere", total - 8) } else { String::new() };
        Err(format!(
            "Prüfsummen-Fehler, das Paket ist beschädigt oder verändert ({total} Problem(e)):\n{}{more}",
            problems.join("\n")
        ))
    }

    /// Die im Manifest genannten Dateien müssen da sein, unter `client/` liegen
    /// und zu Größe und Prüfsumme im Manifest passen.
    fn verify_manifest_files(&self) -> Result<(), String> {
        for item in &self.manifest.items {
            for f in &item.files {
                if !is_safe_zip_path(&f.path) || !f.path.starts_with("client/") {
                    return Err(format!("Item {}: unzulässiger Dateipfad „{}“ im Manifest.", item.vnum, f.path));
                }
                match self.entries.get(&f.path) {
                    None => return Err(format!("Item {}: Datei „{}“ fehlt im ZIP.", item.vnum, f.path)),
                    Some(size) if *size != f.size => {
                        return Err(format!("Item {}: Größe von „{}“ stimmt nicht mit dem Manifest.", item.vnum, f.path))
                    }
                    _ => {}
                }
                if self.hashes.get(&f.path).map(|h| h.as_str()) != Some(f.sha256.to_ascii_lowercase().as_str()) {
                    return Err(format!("Item {}: Prüfsumme von „{}“ im Manifest passt nicht zu checksums.sha256.", item.vnum, f.path));
                }
            }
        }
        Ok(())
    }

    /// Liest einen ZIP-Eintrag komplett.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        let mut ar = self.lock_archive()?;
        read_entry(&mut ar, name)
    }

    fn lock_archive(&self) -> Result<std::sync::MutexGuard<'_, Archive>, String> {
        self.archive.lock().map_err(|_| "ZIP-Archiv ist nicht mehr verfügbar.".to_string())
    }

    pub fn read_optional(&self, name: &str) -> Result<Option<Vec<u8>>, String> {
        if self.entries.contains_key(name) {
            self.read(name).map(Some)
        } else {
            Ok(None)
        }
    }

    /// Schreibt einen ZIP-Eintrag und gibt dessen Prüfsumme zurück.
    pub fn extract_to(&self, name: &str, dest: &Path) -> Result<String, String> {
        let mut ar = self.lock_archive()?;
        let mut f = ar.by_name(name).map_err(|e| format!("{name}: {e}"))?;
        let mut out = File::create(dest).map_err(|e| format!("{}: {e}", dest.display()))?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| format!("{}: {e}", dest.display()))?;
        }
        out.flush().map_err(|e| e.to_string())?;
        Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
    }
}

fn read_entry(ar: &mut Archive, name: &str) -> Result<Vec<u8>, String> {
    let mut f = ar.by_name(name).map_err(|e| format!("{name}: {e}"))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| format!("{name}: {e}"))?;
    Ok(buf)
}

#[cfg(test)]
pub(crate) mod test_pkg {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    pub fn sha(data: &[u8]) -> String {
        sha256_reader(&mut std::io::Cursor::new(data)).unwrap()
    }

    pub fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("m2pkg_test_{tag}_{}_{}", std::process::id(), rand_suffix()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn rand_suffix() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    }

    pub const NAMES_CLIENT: &[u8] = b"VNUM\tLOCALE_NAME\r\n300\tD\xfcrendal\r\n301\tZahnstocher\r\n";
    pub const LIST_CLIENT: &[u8] = b"300\tWEAPON\ticon/item/00300.tga\td:/ymir work/item/weapon/00300.gr2\r\n301\tWEAPON\ticon/item/00300.tga\td:/ymir work/item/weapon/00300.gr2";

    /// Zwei Items teilen sich Icon und Modell; kleines, aber vollständiges Paket.
    pub fn build(dir: &Path, version: u64, corrupt: bool, tamper_list: bool) -> PathBuf {
        let icon = b"ICONDATA".to_vec();
        let model = b"MODELDATA".to_vec();
        let sql_replace = "-- m2pkg\nREPLACE INTO player.item_proto (`vnum`, `name`) VALUES (300, x'41');\nREPLACE INTO player.item_proto (`vnum`, `name`) VALUES (301, x'42');\n";
        let sql_ignore = sql_replace.replace("REPLACE INTO", "INSERT IGNORE INTO");
        let proto_txt = "Vnum\tName\n300\tA\n301\tB\n";

        let mk = |path: &str, data: &[u8]| ManifestFile {
            path: path.into(),
            sha256: sha(data),
            size: data.len() as u64,
            role: "icon".into(),
            guessed: false,
        };
        let item = |vnum: u32| ManifestItem {
            vnum,
            name: Some(format!("Item {vnum}")),
            item_type: "WEAPON".into(),
            icon_path: Some("icon/item/00300.tga".into()),
            model_path: Some("d:/ymir work/item/weapon/00300.gr2".into()),
            files: vec![
                mk("client/icon/item/00300.tga", &icon),
                ManifestFile { role: "model".into(), ..mk("client/d_/ymir work/item/weapon/00300.gr2", &model) },
                ManifestFile {
                    role: "texture".into(),
                    guessed: true,
                    ..mk("client/d_/ymir work/item/weapon/00300.dds", b"TEX")
                },
            ],
            assumed_antiflags: vec![AssumedFlag { name: "ANTI_GIVE".into(), bit: 8192 }],
        };
        let manifest = Manifest {
            format: "m2pkg".into(),
            format_version: version,
            dbtool_version: Some("2.33.0".into()),
            created: None,
            collection: Some(Collection { id: Some(1), name: "Test".into() }),
            client_version: None,
            proto_source: None,
            assumptions: vec![Assumption { field: "AntiFlags".into(), bit: 8192, name: "ANTI_GIVE".into(), basis: "x".into() }],
            items: vec![item(300), item(301)],
            warnings: vec![],
        };
        let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap();

        let mut files: Vec<(String, Vec<u8>)> = vec![
            ("manifest.json".into(), manifest_bytes),
            ("client/icon/item/00300.tga".into(), icon),
            ("client/d_/ymir work/item/weapon/00300.gr2".into(), model),
            ("client/d_/ymir work/item/weapon/00300.dds".into(), b"TEX".to_vec()),
            ("client/item_list.txt".into(), LIST_CLIENT.to_vec()),
            ("client/item_names.txt".into(), NAMES_CLIENT.to_vec()),
            ("server/item_proto_replace.sql".into(), sql_replace.as_bytes().to_vec()),
            ("server/item_proto_insert_ignore.sql".into(), sql_ignore.as_bytes().to_vec()),
            ("server/item_proto.txt".into(), proto_txt.as_bytes().to_vec()),
            ("server/item_names.txt".into(), NAMES_CLIENT.to_vec()),
        ];
        let mut sums = String::new();
        for (n, d) in &files {
            sums.push_str(&format!("{}  {}\n", sha(d), n));
        }
        if corrupt {
            files[1].1 = b"BROKEN!!".to_vec();
        }
        if tamper_list {
            files[4].1.extend_from_slice(b"X");
        }
        files.push(("checksums.sha256".into(), sums.into_bytes()));

        let path = dir.join("paket.zip");
        let mut zw = zip::ZipWriter::new(File::create(&path).unwrap());
        for (n, d) in files {
            zw.start_file(n, SimpleFileOptions::default()).unwrap();
            zw.write_all(&d).unwrap();
        }
        zw.finish().unwrap();
        path
    }
}

#[cfg(test)]
mod tests {
    use super::test_pkg::*;
    use super::*;

    #[test]
    fn opens_valid_package() {
        let dir = temp_dir("open");
        let pkg = Package::open(&build(&dir, 1, false, false)).unwrap();
        assert_eq!(pkg.manifest.items.len(), 2);
        assert_eq!(pkg.manifest.collection_name(), "Test");
    }

    #[test]
    fn rejects_higher_format_version() {
        let dir = temp_dir("ver");
        let err = Package::open(&build(&dir, 2, false, false)).err().unwrap();
        assert!(err.contains("Version 2"), "{err}");
    }

    #[test]
    fn rejects_checksum_mismatch() {
        let dir = temp_dir("sum");
        let err = Package::open(&build(&dir, 1, true, false)).err().unwrap();
        assert!(err.contains("Prüfsumme"), "{err}");
    }

    #[test]
    fn rejects_file_not_listed_in_checksums_or_changed() {
        let dir = temp_dir("tamper");
        let err = Package::open(&build(&dir, 1, false, true)).err().unwrap();
        assert!(err.contains("item_list.txt"), "{err}");
    }

    #[test]
    fn rejects_non_zip_and_wrong_format() {
        let dir = temp_dir("junk");
        let p = dir.join("x.zip");
        std::fs::write(&p, b"nope").unwrap();
        assert!(Package::open(&p).is_err());
    }

    #[test]
    fn safe_paths() {
        assert!(is_safe_zip_path("client/d_/ymir work/a.gr2"));
        assert!(!is_safe_zip_path("../x"));
        assert!(!is_safe_zip_path("client/../x"));
        assert!(!is_safe_zip_path("/abs"));
        assert!(!is_safe_zip_path("client/d:/x"));
        assert!(!is_safe_zip_path("a\\b"));
    }
}
