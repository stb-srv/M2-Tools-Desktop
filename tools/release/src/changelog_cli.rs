//! Prüft und liest Release-Notes aus `CHANGELOG.md` (im aktuellen Verzeichnis = Repo-Wurzel).
//!
//! - `changelog check` - die drei Versionsangaben passen zusammen und für
//!   diese Version gibt es einen nicht leeren Changelog-Abschnitt (läuft in CI
//!   bei jedem Pull Request und vor jedem Release).
//! - `changelog notes <version>` - gibt den Abschnitt als Markdown aus
//!   (Release-Text auf GitHub und `notes` in `updater/latest.json`).
//! - `changelog prepare <patch|minor|major|aktuell|X.Y.Z> <datum>` - bereitet
//!   einen Release vor (Release-Knopf in GitHub Actions): zählt die Version in
//!   allen Dateien hoch, benennt `## [Unveröffentlicht]` in
//!   `## [X.Y.Z] - <datum>` um und gibt die neue Version aus. Mit `aktuell`
//!   bleibt alles, wie es ist; dann muss die Version schon Notizen haben.

use release_tools::{app_versions, finalize_changelog, next_version, release_notes, set_app_version};
use std::path::Path;
use std::process::exit;

fn fail(msg: &str) -> ! {
    // "::error::" macht den Fehler in GitHub Actions direkt sichtbar.
    eprintln!("::error::{msg}");
    exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = Path::new(".");
    match args.first().map(String::as_str) {
        Some("check") => {
            let [conf, cargo, ui] = app_versions(root).unwrap_or_else(|e| fail(&e));
            if conf != cargo || conf != ui {
                fail(&format!(
                    "Versionen passen nicht zusammen: tauri.conf.json={conf}, src-tauri/Cargo.toml={cargo}, ui/Cargo.toml={ui}"
                ));
            }
            let notes = release_notes(root, &conf).unwrap_or_else(|e| fail(&e));
            println!("OK: Version {conf} hat Release-Notes ({} Zeilen).", notes.lines().count());
        }
        Some("notes") => {
            let version = args.get(1).unwrap_or_else(|| fail("Aufruf: changelog notes <version>"));
            println!("{}", release_notes(root, version).unwrap_or_else(|e| fail(&e)));
        }
        Some("prepare") => {
            let (Some(bump), Some(date)) = (args.get(1), args.get(2)) else {
                fail("Aufruf: changelog prepare <patch|minor|major|aktuell|X.Y.Z> <JJJJ-MM-TT>")
            };
            let [conf, cargo, ui] = app_versions(root).unwrap_or_else(|e| fail(&e));
            if conf != cargo || conf != ui {
                fail(&format!(
                    "Versionen passen nicht zusammen: tauri.conf.json={conf}, src-tauri/Cargo.toml={cargo}, ui/Cargo.toml={ui}"
                ));
            }
            let next = next_version(&conf, bump).unwrap_or_else(|e| fail(&e));
            if next != conf {
                let path = root.join("CHANGELOG.md");
                let md = std::fs::read_to_string(&path).unwrap_or_else(|e| fail(&format!("CHANGELOG.md lesen: {e}")));
                let new_md = finalize_changelog(&md, &next, date).unwrap_or_else(|e| fail(&e));
                set_app_version(root, &next).unwrap_or_else(|e| fail(&e));
                std::fs::write(&path, new_md).unwrap_or_else(|e| fail(&format!("CHANGELOG.md schreiben: {e}")));
            }
            // Gegenprobe: genau das, was CI und die App erwarten.
            release_notes(root, &next).unwrap_or_else(|e| fail(&e));
            println!("{next}");
        }
        _ => fail("Aufruf: changelog check | changelog notes <version> | changelog prepare <art> <datum>"),
    }
}
