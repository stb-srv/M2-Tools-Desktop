//! Gemeinsamer Code der Release-Hilfsprogramme.

/// Derselbe Changelog-Parser, den die App für "Was ist neu" benutzt - so
/// prüft und veröffentlicht die Pipeline genau das, was die App anzeigt.
#[path = "../../../ui/src/util/changelog.rs"]
pub mod changelog;

use std::path::Path;

/// Byte-Bereich in einem Text.
type Span = std::ops::Range<usize>;

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
    let conf = read_repo_file(repo_root, "src-tauri/tauri.conf.json")?;
    let conf_span = json_top_level_str_span(&conf, "version").map_err(|e| format!("tauri.conf.json: {e}"))?;
    let conf_v = unquote(&conf[conf_span]).to_string();
    let cargo_v = |p: &str| -> Result<String, String> {
        let text = read_repo_file(repo_root, p)?;
        let span = cargo_package_str_span(&text, "version").map_err(|e| format!("{p}: {e}"))?;
        Ok(unquote(&text[span]).to_string())
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
/// Version schon einen Abschnitt hat. Zeilen in Code-Blöcken (```) zählen nicht
/// als Überschrift, genau wie im App-Parser. Nur die Überschrift wird ersetzt;
/// alle anderen Zeilen und deren Zeilenenden (LF oder CRLF) bleiben unverändert.
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
    let mut in_fence = false;
    for (off, line) in lines_with_offsets(md) {
        if toggles_fence(line) {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && line.trim_end().starts_with(&heading) {
            return Ok(splice(md, off..off + line.len(), &format!("## [{version}] - {date}")));
        }
    }
    Err(format!("In CHANGELOG.md fehlt eine Überschrift \"## [{}]\" außerhalb von Code-Blöcken.", changelog::UNRELEASED))
}

/// Setzt die App-Version überall, wo sie steht: `src-tauri/tauri.conf.json`,
/// `src-tauri/Cargo.toml`, `ui/Cargo.toml` und die beiden `Cargo.lock` (falls
/// vorhanden). Alle Änderungen werden vorher berechnet; erst wenn alle
/// Dateien erfolgreich angepasst werden konnten, wird geschrieben.
pub fn set_app_version(repo_root: &Path, version: &str) -> Result<(), String> {
    let quoted = format!("\"{version}\"");
    let mut writes: Vec<(&str, String)> = Vec::new();

    let conf_path = "src-tauri/tauri.conf.json";
    let conf = read_repo_file(repo_root, conf_path)?;
    let span = json_top_level_str_span(&conf, "version").map_err(|e| format!("{conf_path}: {e}"))?;
    let new_conf = splice(&conf, span, &quoted);
    let parsed: serde_json::Value =
        serde_json::from_str(&new_conf).map_err(|e| format!("{conf_path}: ungültiges JSON nach dem Ersetzen: {e}"))?;
    if parsed["version"] != version {
        return Err(format!("{conf_path}: \"version\" wurde nicht korrekt ersetzt"));
    }
    writes.push((conf_path, new_conf));

    for (cargo_path, lock_path) in [
        ("src-tauri/Cargo.toml", "src-tauri/Cargo.lock"),
        ("ui/Cargo.toml", "ui/Cargo.lock"),
    ] {
        let cargo = read_repo_file(repo_root, cargo_path)?;
        let name_span = cargo_package_str_span(&cargo, "name").map_err(|e| format!("{cargo_path}: {e}"))?;
        let name = unquote(&cargo[name_span]).to_string();
        let span = cargo_package_str_span(&cargo, "version").map_err(|e| format!("{cargo_path}: {e}"))?;
        writes.push((cargo_path, splice(&cargo, span, &quoted)));

        if repo_root.join(lock_path).exists() {
            let lock = read_repo_file(repo_root, lock_path)?;
            let span = cargo_lock_version_span(&lock, &name).map_err(|e| format!("{lock_path}: {e}"))?;
            writes.push((lock_path, splice(&lock, span, &quoted)));
        }
    }

    for (file, new) in writes {
        std::fs::write(repo_root.join(file), new).map_err(|e| format!("{file} schreiben: {e}"))?;
    }
    Ok(())
}

fn read_repo_file(repo_root: &Path, file: &str) -> Result<String, String> {
    std::fs::read_to_string(repo_root.join(file)).map_err(|e| format!("{file} lesen: {e}"))
}

/// Ersetzt `span` in `text` durch `new`. Alles andere bleibt byte-genau gleich.
fn splice(text: &str, span: Span, new: &str) -> String {
    format!("{}{}{}", &text[..span.start], new, &text[span.end..])
}

/// Entfernt die Anführungszeichen um einen Wert.
fn unquote(s: &str) -> &str {
    s.trim_matches('"')
}

/// Byte-Abstand von `base` zu `sub` (`sub` muss ein Teil von `base` sein).
fn offset_of(base: &str, sub: &str) -> usize {
    sub.as_ptr() as usize - base.as_ptr() as usize
}

/// Zeilen ohne Zeilenende (LF oder CRLF), jeweils mit Byte-Offset im Text.
fn lines_with_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut off = 0;
    for piece in text.split_inclusive('\n') {
        out.push((off, piece.trim_end_matches(['\r', '\n'])));
        off += piece.len();
    }
    out
}

/// Ob eine Zeile einen Code-Block (```) öffnet oder schließt.
fn toggles_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// Index nach dem schließenden `"` des JSON-Strings, der bei `start` beginnt.
fn json_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

fn json_skip_ws(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\r' | b'\n') {
        i += 1;
    }
    i
}

/// Byte-Bereich (mit Anführungszeichen) des Textwerts des Schlüssels `key` auf
/// oberster Ebene eines JSON-Objekts. Verschachtelte Schlüssel gleichen Namens
/// und Strings, die `key` enthalten, werden übersprungen; die Formatierung
/// bleibt unangetastet.
fn json_top_level_str_span(text: &str, key: &str) -> Result<Span, String> {
    let b = text.as_bytes();
    let mut depth = 0usize;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'{' | b'[' => {
                depth += 1;
                i += 1;
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
                i += 1;
            }
            b'"' => {
                let end = json_string_end(b, i).ok_or("ungeschlossener Text in JSON")?;
                let is_key = depth == 1 && &text[i + 1..end - 1] == key;
                i = end;
                if is_key {
                    let colon = json_skip_ws(b, i);
                    if b.get(colon) == Some(&b':') {
                        let v = json_skip_ws(b, colon + 1);
                        return match b.get(v) {
                            Some(b'"') => Ok(v..json_string_end(b, v).ok_or("ungeschlossener Text in JSON")?),
                            _ => Err(format!("\"{key}\" ist kein Text")),
                        };
                    }
                }
            }
            _ => i += 1,
        }
    }
    Err(format!("\"{key}\" auf oberster Ebene nicht gefunden"))
}

/// Byte-Bereich (mit Anführungszeichen) des Werts `key = "..."` in der
/// `[package]`-Sektion einer Cargo.toml. Tabellen davor oder danach (z. B.
/// `[dependencies]`) werden ignoriert. `key.workspace = true` und
/// `key = { workspace = true }` geben einen Fehler, weil der Wert dann nicht
/// in dieser Datei steht.
fn cargo_package_str_span(text: &str, key: &str) -> Result<Span, String> {
    let mut in_package = false;
    for (_, line) in lines_with_offsets(text) {
        let t = line.trim();
        if t.starts_with('[') {
            // Tabellenkopf, evtl. mit Kommentar; nur exakt `[package]` zählt.
            in_package = t.split('#').next().unwrap_or("").trim() == "[package]";
            continue;
        }
        if !in_package {
            continue;
        }
        let Some(rest) = t.strip_prefix(key) else { continue };
        let rest = rest.trim_start();
        if rest.starts_with('.') {
            return Err(format!(
                "\"{key}\" ist per Workspace vererbt ({key}.workspace). In dieser Datei muss \"{key}\" fest in [package] stehen."
            ));
        }
        let Some(value) = rest.strip_prefix('=') else { continue };
        let value = value.trim_start();
        if value.starts_with('{') {
            return Err(format!(
                "\"{key}\" in [package] ist eine Tabelle (z. B. {{ workspace = true }}), keine feste Angabe."
            ));
        }
        if !value.starts_with('"') {
            return Err(format!("\"{key}\" in [package] ist kein Text."));
        }
        let close = value[1..].find('"').ok_or_else(|| format!("\"{key}\" in [package]: Anführungszeichen fehlen"))?;
        let start = offset_of(text, value);
        return Ok(start..start + close + 2);
    }
    Err(format!("\"{key}\" in [package] nicht gefunden"))
}

/// Byte-Bereich der `version = "..."` des einen `[[package]]`-Eintrags mit dem
/// Namen `name` in einer Cargo.lock. Fehler, wenn es nicht genau einen solchen
/// Eintrag gibt. Andere Abschnitte (z. B. `[metadata]`) werden nicht beachtet.
fn cargo_lock_version_span(text: &str, name: &str) -> Result<Span, String> {
    #[derive(Default)]
    struct Block {
        name: Option<String>,
        version: Option<Span>,
    }
    let mut blocks: Vec<Block> = Vec::new();
    let mut cur: Option<Block> = None;
    for (off, line) in lines_with_offsets(text) {
        if line.starts_with('[') {
            blocks.extend(cur.take());
            if line.trim() == "[[package]]" {
                cur = Some(Block::default());
            }
            continue;
        }
        let Some(b) = cur.as_mut() else { continue };
        if let Some(rest) = line.strip_prefix("name = \"") {
            b.name = rest.split('"').next().map(str::to_string);
        } else if let Some(rest) = line.strip_prefix("version = \"") {
            if b.version.is_none() {
                let quote = off + "version = ".len();
                let close = rest.find('"').ok_or("version in Cargo.lock: Anführungszeichen fehlen")?;
                b.version = Some(quote..quote + close + 2);
            }
        }
    }
    blocks.extend(cur.take());

    let matching: Vec<Block> = blocks.into_iter().filter(|b| b.name.as_deref() == Some(name)).collect();
    if matching.len() != 1 {
        return Err(format!("{} [[package]]-Einträge namens \"{name}\" gefunden, erwartet genau 1", matching.len()));
    }
    matching[0].version.clone().ok_or_else(|| format!("[[package]] \"{name}\" hat keine version"))
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
    fn ignores_headings_in_code_fences() {
        // Eine Beispiel-Überschrift im Code-Block wird nicht ersetzt, die echte schon.
        let md = "```md\n## [Unveröffentlicht]\n```\n\n## [Unveröffentlicht]\n\n- **X**: y\n\n## [0.30.1] - 2026-11-13\n\n- alt\n";
        let out = finalize_changelog(md, "0.30.2", "2026-11-14").unwrap();
        assert_eq!(
            out,
            "```md\n## [Unveröffentlicht]\n```\n\n## [0.30.2] - 2026-11-14\n\n- **X**: y\n\n## [0.30.1] - 2026-11-13\n\n- alt\n"
        );
        // Ein bestehender Abschnitt im Code-Block zählt nicht als "schon vorhanden".
        let md = "```md\n## [0.30.2] - beispiel\n```\n\n## [Unveröffentlicht]\n\n- **X**: y\n";
        let out = finalize_changelog(md, "0.30.2", "2026-11-14").unwrap();
        assert_eq!(out, "```md\n## [0.30.2] - beispiel\n```\n\n## [0.30.2] - 2026-11-14\n\n- **X**: y\n");
        // Nur innerhalb eines Blocks: kein echter Abschnitt -> Fehler.
        assert!(finalize_changelog("```\n## [Unveröffentlicht]\n- a\n```\n\n## [0.30.1] - x\n- a\n", "0.30.2", "d").is_err());
    }

    #[test]
    fn keeps_crlf_line_endings() {
        let md = "# Changelog\r\n\r\n## [Unveröffentlicht]\r\n\r\n- **X**: y\r\n\r\n## [0.30.1] - 2026-11-13\r\n\r\n- alt";
        let out = finalize_changelog(md, "0.30.2", "2026-11-14").unwrap();
        assert_eq!(
            out,
            "# Changelog\r\n\r\n## [0.30.2] - 2026-11-14\r\n\r\n- **X**: y\r\n\r\n## [0.30.1] - 2026-11-13\r\n\r\n- alt"
        );
    }

    #[test]
    fn json_version_top_level_only() {
        // Verschachteltes "version" zuerst, Top-Level-Schlüssel als letzter Eintrag ohne Komma.
        let json = "{\r\n  \"bundle\": { \"version\": \"9.9.9\" },\r\n  \"productName\": \"M2\",\r\n  \"version\": \"0.30.1\"\r\n}\r\n";
        let span = json_top_level_str_span(json, "version").unwrap();
        assert_eq!(&json[span.clone()], "\"0.30.1\"");
        let new = splice(json, span, "\"0.31.0\"");
        assert_eq!(new, "{\r\n  \"bundle\": { \"version\": \"9.9.9\" },\r\n  \"productName\": \"M2\",\r\n  \"version\": \"0.31.0\"\r\n}\r\n");
        let v: serde_json::Value = serde_json::from_str(&new).unwrap();
        assert_eq!(v["version"], "0.31.0");
        assert_eq!(v["bundle"]["version"], "9.9.9");
    }

    #[test]
    fn json_version_nested_only_is_error() {
        assert!(json_top_level_str_span("{\"bundle\": {\"version\": \"1.0.0\"}}", "version").is_err());
        // "version" als Wert, nicht als Schlüssel
        assert!(json_top_level_str_span("{\"name\": \"version\"}", "version").is_err());
    }

    #[test]
    fn cargo_package_after_dependency_tables() {
        let toml = "[dependencies.serde]\nversion = \"1\"\n\n[dependencies]\nfoo = { version = \"2\" }\n\n[package]\nname = \"m2manager\"\nversion = \"0.30.1\"\n\n[[bin]]\nname = \"x\"\nversion = \"7\"\n";
        let span = cargo_package_str_span(toml, "version").unwrap();
        assert_eq!(&toml[span], "\"0.30.1\"");
        let span = cargo_package_str_span(toml, "name").unwrap();
        assert_eq!(&toml[span], "\"m2manager\"");
    }

    #[test]
    fn cargo_package_before_dependency_tables_and_crlf() {
        let toml = "[package]\r\nname = \"m2manager-ui\" # Kommentar\r\nversion = \"0.30.1\"\r\n\r\n[dependencies]\r\nversion = \"3\"\r\n";
        let span = cargo_package_str_span(toml, "version").unwrap();
        let new = splice(toml, span, "\"0.31.0\"");
        assert_eq!(new, "[package]\r\nname = \"m2manager-ui\" # Kommentar\r\nversion = \"0.31.0\"\r\n\r\n[dependencies]\r\nversion = \"3\"\r\n");
    }

    #[test]
    fn cargo_package_workspace_version_is_clear_error() {
        let dotted = "[package]\nname = \"a\"\nversion.workspace = true\n";
        let err = cargo_package_str_span(dotted, "version").unwrap_err();
        assert!(err.contains("Workspace"), "{err}");
        let table = "[package]\nname = \"a\"\nversion = { workspace = true }\n";
        let err = cargo_package_str_span(table, "version").unwrap_err();
        assert!(err.contains("Tabelle"), "{err}");
        // Eine Version nur in einer Dependency-Tabelle gilt nicht als Paketversion.
        assert!(cargo_package_str_span("[dependencies]\nversion = \"1\"\n", "version").is_err());
    }

    #[test]
    fn cargo_lock_only_the_named_package_block() {
        let lock = "[[package]]\nname = \"m2manager\"\nversion = \"0.30.1\"\n\n[[package]]\nname = \"aaa\"\nversion = \"0.30.1\"\n\n[metadata]\nversion = \"1\"\n";
        let span = cargo_lock_version_span(lock, "m2manager").unwrap();
        assert_eq!(&lock[span], "\"0.30.1\"");
        let new = splice(lock, cargo_lock_version_span(lock, "aaa").unwrap(), "\"0.31.0\"");
        assert!(new.contains("name = \"m2manager\"\nversion = \"0.30.1\"\n\n[[package]]\nname = \"aaa\"\nversion = \"0.31.0\""));
    }

    #[test]
    fn cargo_lock_requires_exactly_one_match() {
        let none = "[[package]]\nname = \"aaa\"\nversion = \"1\"\n";
        assert!(cargo_lock_version_span(none, "m2manager").is_err());
        let two = "[[package]]\nname = \"m2manager\"\nversion = \"1\"\n\n[[package]]\nname = \"m2manager\"\nversion = \"1\"\n";
        assert!(cargo_lock_version_span(two, "m2manager").is_err());
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

    #[test]
    fn set_version_keeps_crlf_and_writes_nothing_on_error() {
        let dir = std::env::temp_dir().join(format!("m2-release-crlf-{}", std::process::id()));
        let w = |p: &str, s: &str| {
            let f = dir.join(p);
            std::fs::create_dir_all(f.parent().unwrap()).unwrap();
            std::fs::write(f, s).unwrap();
        };
        let conf = "{\r\n  \"version\": \"0.30.1\"\r\n}\r\n";
        w("src-tauri/tauri.conf.json", conf);
        w("src-tauri/Cargo.toml", "[package]\r\nname = \"m2manager\"\r\nversion = \"0.30.1\"\r\n");
        w("ui/Cargo.toml", "[package]\r\nname = \"m2manager-ui\"\r\nversion.workspace = true\r\n");
        assert!(set_app_version(&dir, "0.31.0").is_err());
        // Fehler in ui/Cargo.toml: keine Datei darf geschrieben worden sein.
        assert_eq!(std::fs::read_to_string(dir.join("src-tauri/tauri.conf.json")).unwrap(), conf);
        w("ui/Cargo.toml", "[package]\r\nname = \"m2manager-ui\"\r\nversion = \"0.30.1\"\r\n");
        set_app_version(&dir, "0.31.0").unwrap();
        assert_eq!(
            std::fs::read_to_string(dir.join("src-tauri/tauri.conf.json")).unwrap(),
            "{\r\n  \"version\": \"0.31.0\"\r\n}\r\n"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("ui/Cargo.toml")).unwrap(),
            "[package]\r\nname = \"m2manager-ui\"\r\nversion = \"0.31.0\"\r\n"
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
