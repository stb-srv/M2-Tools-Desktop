//! Prüft und liest Release-Notes aus `CHANGELOG.md` (im aktuellen Verzeichnis = Repo-Wurzel).
//!
//! - `changelog check` - die drei Versionsangaben passen zusammen und für
//!   diese Version gibt es einen nicht leeren Changelog-Abschnitt (läuft in CI
//!   bei jedem Pull Request und vor jedem Release).
//! - `changelog notes <version>` - gibt den Abschnitt als Markdown aus
//!   (Release-Text auf GitHub und `notes` in `updater/latest.json`).

use release_tools::{app_versions, release_notes};
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
        _ => fail("Aufruf: changelog check | changelog notes <version>"),
    }
}
