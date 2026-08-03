use serde::{Deserialize, Serialize};

// Format verified against the real server file (share/translate.lua, ~8860
// lines). Its own header comment explains the convention: every quest's
// dialog strings get extracted into this one flat file as
//
//   gameforge.<quest_name> = {}
//   gameforge.<quest_name>.<key> = "<value>"
//
// (`[ENTER]` inside a value is a literal in-game line-break marker, not a
// real escape - same convention used by the Quest Builder's own generated
// say()/say_title() strings, see questTemplates.ts::luaString.)
//
// This module deliberately does NOT parse/reserialize the whole file - it's
// large and largely machine-generated (per its own header, extracted by an
// external localization tool), so round-tripping the parts we don't
// specifically understand risks silently mangling them. Instead,
// `write_namespace` finds one namespace's line range by prefix and replaces
// only those lines, leaving everything else byte-identical.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocaleEntry {
    pub key: String,
    pub value: String,
}

fn unescape_lua_string(raw: &str) -> Option<String> {
    let s = raw.trim();
    if s.len() < 2 || !s.starts_with('"') || !s.ends_with('"') {
        return None;
    }
    let inner = &s[1..s.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('"') => {
                    out.push('"');
                    chars.next();
                }
                Some('\\') => {
                    out.push('\\');
                    chars.next();
                }
                _ => out.push(c),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

fn escape_lua_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn parse_header(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("gameforge.")?;
    let (name, rhs) = rest.split_once('=')?;
    if rhs.trim() == "{}" {
        Some(name.trim())
    } else {
        None
    }
}

fn parse_entry(line: &str, namespace: &str) -> Option<LocaleEntry> {
    let trimmed = line.trim();
    let prefix = format!("gameforge.{namespace}.");
    let rest = trimmed.strip_prefix(&prefix)?;
    let (key, rhs) = rest.split_once('=')?;
    let value = unescape_lua_string(rhs)?;
    Some(LocaleEntry {
        key: key.trim().to_string(),
        value,
    })
}

pub fn list_namespaces(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(parse_header)
        .map(|s| s.to_string())
        .collect()
}

pub fn read_namespace(content: &str, namespace: &str) -> Vec<LocaleEntry> {
    content
        .lines()
        .filter_map(|l| parse_entry(l, namespace))
        .collect()
}

/// Replaces the header-through-last-entry line range for `namespace` with
/// freshly rendered lines from `entries` (order preserved, keys not present
/// anymore are simply dropped, new keys appended) - or errors if the
/// namespace doesn't exist (use `create_namespace` first for a new one).
pub fn write_namespace(content: &str, namespace: &str, entries: &[LocaleEntry]) -> Result<String, String> {
    let header_needle = format!("gameforge.{namespace} =");
    let entry_prefix = format!("gameforge.{namespace}.");

    let lines: Vec<&str> = content.lines().collect();
    let header_idx = lines
        .iter()
        .position(|l| l.trim().starts_with(&header_needle))
        .ok_or_else(|| format!("Namespace 'gameforge.{namespace}' nicht gefunden."))?;

    let mut end_idx = header_idx + 1;
    while end_idx < lines.len() && lines[end_idx].trim_start().starts_with(&entry_prefix) {
        end_idx += 1;
    }

    let mut new_lines: Vec<String> = lines[..=header_idx].iter().map(|s| s.to_string()).collect();
    for entry in entries {
        new_lines.push(format!(
            "gameforge.{namespace}.{} = {}",
            entry.key,
            escape_lua_string(&entry.value)
        ));
    }
    new_lines.extend(lines[end_idx..].iter().map(|s| s.to_string()));
    Ok(new_lines.join("\n") + "\n")
}

/// Appends a brand-new, empty namespace at the end of the file.
pub fn create_namespace(content: &str, namespace: &str) -> Result<String, String> {
    if list_namespaces(content).iter().any(|n| n == namespace) {
        return Err(format!("Namespace 'gameforge.{namespace}' existiert bereits."));
    }
    let mut out = content.trim_end_matches('\n').to_string();
    out.push('\n');
    out.push_str(&format!("gameforge.{namespace} = {{}}\n"));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "-- header comment, left untouched\n\
gameforge.horse_summon = {}\n\
gameforge.horse_summon._10_npcChat = \"Neues Pferdebild \"\n\
gameforge.horse_summon._100_say = \"Line with \\\"quotes\\\" and[ENTER]a break\"\n\
\n\
gameforge.dragon_lair_access = {}\n\
gameforge.dragon_lair_access._010_npcChat = \"Ich will den Drachen toeten! \"\n";

    #[test]
    fn lists_namespaces() {
        assert_eq!(list_namespaces(SAMPLE), vec!["horse_summon", "dragon_lair_access"]);
    }

    #[test]
    fn reads_namespace_entries_only() {
        let entries = read_namespace(SAMPLE, "horse_summon");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].key, "_10_npcChat");
        assert_eq!(entries[0].value, "Neues Pferdebild ");
        assert_eq!(entries[1].value, "Line with \"quotes\" and[ENTER]a break");
    }

    #[test]
    fn write_namespace_leaves_rest_of_file_untouched() {
        let updated = write_namespace(
            SAMPLE,
            "horse_summon",
            &[LocaleEntry { key: "_10_npcChat".to_string(), value: "Neuer Text".to_string() }],
        )
        .unwrap();
        assert!(updated.contains("gameforge.horse_summon._10_npcChat = \"Neuer Text\""));
        assert!(!updated.contains("_100_say"));
        assert!(updated.contains("gameforge.dragon_lair_access._010_npcChat"));
        assert!(updated.starts_with("-- header comment, left untouched"));
    }

    #[test]
    fn create_namespace_rejects_duplicate() {
        assert!(create_namespace(SAMPLE, "horse_summon").is_err());
        let updated = create_namespace(SAMPLE, "new_quest").unwrap();
        assert!(updated.contains("gameforge.new_quest = {}"));
    }
}
