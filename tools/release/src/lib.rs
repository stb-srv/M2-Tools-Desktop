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

/// Zerlegt `X.Y.Z` in Zahlen.
fn parse_semver(v: &str) -> Result<[u64; 3], String> {
    let parts: Vec<&str> = v.trim().trim_start_matches('v').split('.').collect();
    if parts.len() != 3 {
        return Err(format!("\"{v}\" ist keine Version im Format X.Y.Z"));
    }
    let mut out = [0u64; 3];
    for (i, p) in parts.iter().enumerate() {
        out[i] = p.parse().map_err(|_| format!("\"{v}\" ist keine Version im Format X.Y.Z"))?;
    }
    Ok(out)
}

/// Neue Version aus der aktuellen und der Auswahl im Release-Knopf:
/// `patch` (0.30.1 -> 0.30.2), `minor` (-> 0.31.0), `major` (-> 1.0.0),
/// `aktuell` (Version bleibt, z. B. um einen abgebrochenen Release zu
/// wiederholen) oder eine ausgeschriebene Version, die größer sein muss.
pub fn next_version(current: &str, bump: &str) -> Result<String, String> {
    let [ma, mi, pa] = parse_semver(current)?;
    let next = match bump.trim() {
        "patch" => [ma, mi, pa + 1],
        "minor" => [ma, mi + 1, 0],
        "major" => [ma + 1, 0, 0],
        "aktuell" | "current" => return Ok(current.to_string()),
        explicit => {
            let v = parse_semver(explicit)?;
            if v <= [ma, mi, pa] {
                return Err(format!("Version {explicit} ist nicht größer als die aktuelle Version {current}."));
            }
            v
        }
    };
    Ok(format!("{}.{}.{}", next[0], next[1], next[2]))
}

/// Benennt `## [Unveröffentlicht]` in `## [version] - date` um. Fehler, wenn
/// der Abschnitt fehlt oder leer ist (ohne Notizen kein Release) oder die
/// Version schon einen Abschnitt hat.
pub fn finalize_changelog(md: &str, version: &str, date: &str) -> Result<String, String> {
    let sections = changelog::parse(md);
    if changelog::section(&sections, version).is_some() {
        return Err(format!("CHANGELOG.md hat schon einen Abschnitt \"## [{version}]\"."));
    }
    match changelog::section(&sections, changelog::UNRELEASED) {
        Some(s) if !s.body.trim().is_empty() => {}
        _ => {
            return Err(format!(
                "In CHANGELOG.md fehlt der Abschnitt \"## [{}]\" mit den Änderungen dieser Version (oder er ist leer). \
                 Ohne Release-Notes kein Release.",
                changelog::UNRELEASED
            ))
        }
    }
    let heading = format!("## [{}]", changelog::UNRELEASED);
    let mut done = false;
    let mut out: Vec<String> = Vec::new();
    for line in md.lines() {
        if !done && line.trim_end().starts_with(&heading) {
            out.push(format!("## [{version}] - {date}"));
            done = true;
        } else {
            out.push(line.to_string());
        }
    }
    let mut text = out.join("\n");
    if md.ends_with('\n') {
        text.push('\n');
    }
    Ok(text)
}

/// Ersetzt in `text` die erste Zeile, die (ohne Einrückung) mit `prefix`
/// beginnt und nach `after` kommt, durch `prefix` + `value`.
fn replace_version_line(text: &str, after: Option<&str>, prefix: &str, value: &str) -> Result<String, String> {
    let mut seen_anchor = after.is_none();
    let mut done = false;
    let mut out = Vec::new();
    for line in text.lines() {
        if !seen_anchor && Some(line.trim()) == after {
            seen_anchor = true;
        } else if seen_anchor && !done && line.trim_start().starts_with(prefix) {
            let indent = &line[..line.len() - line.trim_start().len()];
            out.push(format!("{indent}{prefix}{value}"));
            done = true;
            continue;
        }
        out.push(line.to_string());
    }
    if !done {
        return Err(format!("\"{prefix}\" nicht gefunden"));
    }
    let mut s = out.join("\n");
    if text.ends_with('\n') {
        s.push('\n');
    }
    Ok(s)
}

/// Setzt die App-Version überall, wo sie steht: `src-tauri/tauri.conf.json`,
/// `src-tauri/Cargo.toml`, `ui/Cargo.toml` und die beiden `Cargo.lock`.
pub fn set_app_version(repo_root: &Path, version: &str) -> Result<(), String> {
    let edits: [(&str, Option<&str>, &str, String); 5] = [
        ("src-tauri/tauri.conf.json", None, "\"version\": ", format!("\"{version}\",")),
        ("src-tauri/Cargo.toml", None, "version = ", format!("\"{version}\"")),
        ("ui/Cargo.toml", None, "version = ", format!("\"{version}\"")),
        ("src-tauri/Cargo.lock", Some("name = \"m2manager\""), "version = ", format!("\"{version}\"")),
        ("ui/Cargo.lock", Some("name = \"m2manager-ui\""), "version = ", format!("\"{version}\"")),
    ];
    for (file, anchor, prefix, value) in edits {
        let path = repo_root.join(file);
        if !path.exists() && file.ends_with(".lock") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|e| format!("{file} lesen: {e}"))?;
        let new = replace_version_line(&text, anchor, prefix, &value).map_err(|e| format!("{file}: {e}"))?;
        std::fs::write(&path, new).map_err(|e| format!("{file} schreiben: {e}"))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bumps() {
        assert_eq!(next_version("0.30.1", "patch").unwrap(), "0.30.2");
        assert_eq!(next_version("0.30.1", "minor").unwrap(), "0.31.0");
        assert_eq!(next_version("0.30.1", "major").unwrap(), "1.0.0");
        assert_eq!(next_version("0.30.1", "aktuell").unwrap(), "0.30.1");
        assert_eq!(next_version("0.30.1", "0.40.0").unwrap(), "0.40.0");
        assert!(next_version("0.30.1", "0.30.1").is_err());
        assert!(next_version("0.30.1", "0.29.9").is_err());
        assert!(next_version("0.30.1", "quatsch").is_err());
    }

    const MD: &str = "# Changelog\n\nIntro\n\n## [Unveröffentlicht]\n\n### Behoben\n\n- **X**: y\n\n## [0.30.1] - 2026-11-13\n\n- alt\n";

    #[test]
    fn finalizes_unreleased_section() {
        let out = finalize_changelog(MD, "0.30.2", "2026-11-14").unwrap();
        assert!(out.contains("## [0.30.2] - 2026-11-14\n\n### Behoben"));
        assert!(!out.contains("Unveröffentlicht"));
        assert!(out.ends_with("- alt\n"));
        let s = changelog::parse(&out);
        assert_eq!(changelog::section(&s, "0.30.2").unwrap().body, "### Behoben\n\n- **X**: y");
    }

    #[test]
    fn refuses_without_notes() {
        assert!(finalize_changelog("## [0.30.1] - x\n\n- a\n", "0.30.2", "d").is_err());
        assert!(finalize_changelog("## [Unveröffentlicht]\n\n## [0.30.1] - x\n- a\n", "0.30.2", "d").is_err());
        assert!(finalize_changelog(MD, "0.30.1", "d").is_err());
    }

    #[test]
    fn sets_all_versions() {
        let dir = std::env::temp_dir().join(format!("m2-release-test-{}", std::process::id()));
        let w = |p: &str, s: &str| {
            let f = dir.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, s).unwrap();
        };
        w("src-tauri/tauri.conf.json", "{\n  \"productName\": \"M2Manager\",\n  \"version\": \"0.30.1\",\n  \"x\": 1\n}\n");
        w("src-tauri/Cargo.toml", "[package]\nname = \"m2manager\"\nversion = \"0.30.1\"\n\n[dependencies]\nfoo = { version = \"1\" }\n");
        w("ui/Cargo.toml", "[package]\nname = \"m2manager-ui\"\nversion = \"0.30.1\"\n");
        w(
            "src-tauri/Cargo.lock",
            "[[package]]\nname = \"aaa\"\nversion = \"9.9.9\"\n\n[[package]]\nname = \"m2manager\"\nversion = \"0.30.1\"\n",
        );
        set_app_version(&dir, "0.31.0").unwrap();
        assert_eq!(app_versions(&dir).unwrap(), ["0.31.0", "0.31.0", "0.31.0"].map(String::from));
        let lock = std::fs::read_to_string(dir.join("src-tauri/Cargo.lock")).unwrap();
        assert!(lock.contains("name = \"aaa\"\nversion = \"9.9.9\""));
        assert!(lock.contains("name = \"m2manager\"\nversion = \"0.31.0\""));
        let conf = std::fs::read_to_string(dir.join("src-tauri/tauri.conf.json")).unwrap();
        assert!(conf.contains("  \"version\": \"0.31.0\",\n  \"x\": 1"));
        std::fs::remove_dir_all(&dir).ok();
    }
}
