//! Gemeinsamer Code der Release-Hilfsprogramme.

/// Derselbe Changelog-Parser, den die App für "Was ist neu" benutzt - so
/// prüft und veröffentlicht die Pipeline genau das, was die App anzeigt.
#[path = "../../../ui/src/util/changelog.rs"]
pub mod changelog;

use std::path::Path;

/// Liest den Abschnitt `version` aus `CHANGELOG.md` im Repo-Wurzelverzeichnis.
/// Fehler, wenn er fehlt oder leer ist.
pub fn release_notes(repo_root: &Path, version: &str) -> Result<String, String> {
    let path = repo_root.join("CHANGELOG.md");
    let md = std::fs::read_to_string(&path).map_err(|e| format!("{} lesen: {e}", path.display()))?;
    let sections = changelog::parse(&md);
    let version = version.trim().trim_start_matches('v');
    match changelog::section(&sections, version) {
        Some(s) if !s.body.trim().is_empty() => Ok(s.body.clone()),
        Some(_) => Err(format!("Der Abschnitt \"## [{version}]\" in CHANGELOG.md ist leer.")),
        None => Err(format!(
            "CHANGELOG.md hat keinen Abschnitt \"## [{version}] - JJJJ-MM-TT\". Bitte die Änderungen dieser Version eintragen \
             (oder den Abschnitt \"## [{}]\" in \"## [{version}] - <Datum>\" umbenennen).",
            changelog::UNRELEASED
        )),
    }
}

/// Die App-Version aus `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`
/// und `ui/Cargo.toml` (in dieser Reihenfolge).
pub fn app_versions(repo_root: &Path) -> Result<[String; 3], String> {
    let read = |p: &str| std::fs::read_to_string(repo_root.join(p)).map_err(|e| format!("{p} lesen: {e}"));
    let conf = read("src-tauri/tauri.conf.json")?;
    let conf_v = conf
        .lines()
        .find_map(|l| l.trim().strip_prefix("\"version\":"))
        .map(|v| v.trim().trim_end_matches(',').trim_matches('"').to_string())
        .ok_or("\"version\" in tauri.conf.json nicht gefunden")?;
    let cargo_v = |p: &str| -> Result<String, String> {
        read(p)?
            .lines()
            .find_map(|l| l.strip_prefix("version = "))
            .map(|v| v.trim().trim_matches('"').to_string())
            .ok_or_else(|| format!("version in {p} nicht gefunden"))
    };
    Ok([conf_v, cargo_v("src-tauri/Cargo.toml")?, cargo_v("ui/Cargo.toml")?])
}
