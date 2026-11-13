//! Release-Notes der App, direkt aus `CHANGELOG.md` beim Bauen eingebettet.
//!
//! Damit kennt jede ausgelieferte Version ihre eigenen Notizen (und die aller
//! älteren Versionen) auch ohne Internet, z. B. für den "Was ist neu"-Dialog
//! nach einem Update und die Anzeige unter Einstellungen › Updates. CI prüft,
//! dass es für die aktuelle Version einen Abschnitt gibt (siehe Test unten und
//! `tools/release`, Befehl `changelog check`).

use super::changelog::{self, Section};
use std::sync::OnceLock;

/// Der komplette Changelog im Repo-Wurzelverzeichnis.
pub const CHANGELOG_MD: &str = include_str!("../../../CHANGELOG.md");

/// Höchstens so viele Versionen fasst "Was ist neu" nach einem großen Sprung zusammen.
const MAX_SECTIONS: usize = 10;

pub fn sections() -> &'static [Section] {
    static SECTIONS: OnceLock<Vec<Section>> = OnceLock::new();
    SECTIONS.get_or_init(|| changelog::parse(CHANGELOG_MD))
}

/// Notizen genau einer Version.
pub fn notes_for(version: &str) -> Option<String> {
    changelog::section(sections(), version).map(|s| s.body.clone()).filter(|b| !b.is_empty())
}

/// Alle Notizen seit `last_seen` (exklusiv) bis `current` (inklusiv), neueste
/// zuerst; ohne `last_seen` nur die von `current`.
pub fn notes_since(last_seen: Option<&str>, current: &str) -> Option<String> {
    let mut found = changelog::sections_between(sections(), last_seen, current);
    found.truncate(MAX_SECTIONS);
    let text = changelog::render(&found);
    (!text.trim().is_empty()).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sicherung, dass kein Release ohne Release-Notes rausgeht: die Version
    /// in `ui/Cargo.toml` muss einen nicht leeren Abschnitt im Changelog haben.
    #[test]
    fn current_version_has_release_notes() {
        let v = env!("CARGO_PKG_VERSION");
        assert!(
            notes_for(v).is_some(),
            "CHANGELOG.md hat keinen (oder einen leeren) Abschnitt \"## [{v}] - JJJJ-MM-TT\". \
             Bitte vor dem Release die Änderungen dieser Version dort eintragen."
        );
    }

    #[test]
    fn notes_since_spans_versions() {
        let both = notes_since(Some("0.19.0"), "0.20.1").unwrap();
        assert!(both.contains("## Version 0.20.1"));
        assert!(both.contains("## Version 0.20.0"));
        assert!(!both.contains("## Version 0.19.0"));
    }
}
