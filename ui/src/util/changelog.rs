//! Liest `CHANGELOG.md` (Format "Keep a Changelog") in einzelne Versionen.
//!
//! Reines `std`, damit dieselbe Datei auch von der Release-Pipeline benutzt
//! wird (`tools/release` bindet sie per `#[path]` ein): die App zeigt genau
//! die Notizen an, die CI prüft und in `latest.json`/das GitHub-Release
//! schreibt. Eine Abschnittsüberschrift sieht so aus:
//!
//! ```text
//! ## [0.20.1] - 2026-11-12
//! ## [Unveröffentlicht]
//! ```

/// Ein Versionsabschnitt aus dem Changelog.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// z. B. `0.20.1`, oder `Unveröffentlicht` für noch nicht veröffentlichte Änderungen.
    pub version: String,
    /// Datum hinter der Version (`2026-11-12`), falls angegeben.
    pub date: Option<String>,
    /// Markdown zwischen dieser und der nächsten `## `-Überschrift, ohne
    /// führende/abschließende Leerzeilen.
    pub body: String,
}

/// Name des Abschnitts für gesammelte, noch nicht veröffentlichte Änderungen.
pub const UNRELEASED: &str = "Unveröffentlicht";

/// Zerlegt den Changelog in Abschnitte (Reihenfolge wie in der Datei,
/// also neueste zuerst). Text vor dem ersten `## [` wird ignoriert.
pub fn parse(markdown: &str) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut current: Option<(String, Option<String>, Vec<&str>)> = None;
    let mut in_fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
        }
        if !in_fence {
            if let Some((version, date)) = parse_heading(line) {
                if let Some(s) = current.take() {
                    sections.push(finish(s));
                }
                current = Some((version, date, Vec::new()));
                continue;
            }
            // Eine andere Überschrift der Ebene 2 beendet den Abschnitt.
            if line.starts_with("## ") {
                if let Some(s) = current.take() {
                    sections.push(finish(s));
                }
                continue;
            }
        }
        if let Some((_, _, lines)) = current.as_mut() {
            lines.push(line);
        }
    }
    if let Some(s) = current.take() {
        sections.push(finish(s));
    }
    sections
}

fn finish((version, date, lines): (String, Option<String>, Vec<&str>)) -> Section {
    Section { version, date, body: lines.join("\n").trim_matches('\n').trim_end().to_string() }
}

/// `## [0.20.1] - 2026-11-12` -> `("0.20.1", Some("2026-11-12"))`.
fn parse_heading(line: &str) -> Option<(String, Option<String>)> {
    let rest = line.strip_prefix("## [")?;
    let (version, after) = rest.split_once(']')?;
    let version = version.trim();
    if version.is_empty() {
        return None;
    }
    let date = after.trim().trim_start_matches(['-', '–']).trim();
    let date = (!date.is_empty()).then(|| date.to_string());
    Some((version.to_string(), date))
}

/// Abschnitt einer bestimmten Version (`v` davor ist erlaubt).
pub fn section<'a>(sections: &'a [Section], version: &str) -> Option<&'a Section> {
    let version = version.trim().trim_start_matches('v');
    sections.iter().find(|s| s.version == version)
}

/// `"0.20.1"` -> `[0, 20, 1]`. `None` für `Unveröffentlicht` o. ä.
pub fn version_key(version: &str) -> Option<Vec<u64>> {
    let core = version.trim().trim_start_matches('v').split(['-', '+']).next()?;
    core.split('.').map(|p| p.parse().ok()).collect()
}

/// Alle veröffentlichten Abschnitte mit `after < Version <= up_to`, neueste
/// zuerst. Damit sieht man nach einem Sprung über mehrere Versionen alle
/// dazwischenliegenden Änderungen. `after = None` liefert nur `up_to`.
pub fn sections_between<'a>(sections: &'a [Section], after: Option<&str>, up_to: &str) -> Vec<&'a Section> {
    let Some(upper) = version_key(up_to) else { return Vec::new() };
    let lower = after.and_then(version_key);
    sections
        .iter()
        .filter(|s| {
            let Some(k) = version_key(&s.version) else { return false };
            match &lower {
                Some(l) => k > *l && k <= upper,
                None => k == upper,
            }
        })
        .collect()
}

/// Macht aus mehreren Abschnitten ein Markdown mit einer Überschrift je
/// Version (bei nur einem Abschnitt reicht dessen Text).
pub fn render(sections: &[&Section]) -> String {
    if let [only] = sections {
        return only.body.clone();
    }
    sections
        .iter()
        .map(|s| match &s.date {
            Some(d) => format!("## Version {} ({d})\n\n{}", s.version, s.body),
            None => format!("## Version {}\n\n{}", s.version, s.body),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\nIntro\n\n## [Unveröffentlicht]\n\n- kommt noch\n\n## [0.3.0] - 2026-11-13\n\n### Neu\n\n- **A**\n\n```\n## [kein] Abschnitt\n```\n\n## [0.2.1] - 2026-11-12\n\n- Fix\n\n## [0.2.0]\n\n- Start\n";

    #[test]
    fn parses_sections() {
        let s = parse(SAMPLE);
        assert_eq!(s.iter().map(|x| x.version.as_str()).collect::<Vec<_>>(), ["Unveröffentlicht", "0.3.0", "0.2.1", "0.2.0"]);
        assert_eq!(s[1].date.as_deref(), Some("2026-11-13"));
        assert!(s[1].body.starts_with("### Neu"));
        assert!(s[1].body.contains("## [kein] Abschnitt"), "Codeblöcke dürfen keinen Abschnitt beenden");
        assert_eq!(s[2].body, "- Fix");
        assert_eq!(s[3].date, None);
    }

    #[test]
    fn finds_version_and_ranges() {
        let s = parse(SAMPLE);
        assert_eq!(section(&s, "v0.2.1").map(|x| x.body.as_str()), Some("- Fix"));
        assert!(section(&s, "9.9.9").is_none());
        let between: Vec<_> = sections_between(&s, Some("0.2.0"), "0.3.0").iter().map(|x| x.version.clone()).collect();
        assert_eq!(between, ["0.3.0", "0.2.1"]);
        assert_eq!(sections_between(&s, None, "0.2.1").len(), 1);
        assert!(sections_between(&s, Some("0.3.0"), "0.3.0").is_empty());
    }

    #[test]
    fn renders_one_or_many() {
        let s = parse(SAMPLE);
        assert_eq!(render(&sections_between(&s, None, "0.2.1")), "- Fix");
        let many = render(&sections_between(&s, Some("0.1.0"), "0.2.1"));
        assert!(many.starts_with("## Version 0.2.1 (2026-11-12)\n\n- Fix"));
        assert!(many.contains("## Version 0.2.0\n\n- Start"));
    }

    #[test]
    fn version_keys() {
        assert!(version_key("0.10.0") > version_key("0.9.9"));
        assert_eq!(version_key("v1.2.3"), Some(vec![1, 2, 3]));
        assert_eq!(version_key(UNRELEASED), None);
    }
}
