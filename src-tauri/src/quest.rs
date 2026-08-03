use serde::{Deserialize, Serialize};

// Real quest source layout on the user's dev server (verified by reading it
// directly over SFTP, not guessed from generic Metin2 docs):
//
//   share/quest/quest_list                <- flat list of relative paths,
//                                             one per line - this is what
//                                             `make.py` iterates to compile
//   share/quest/<Category>/<Name>.lua     <- quest DSL source
//   share/quest/Runs/<Name>.quest         <- dungeon "run" scripts (same
//                                             list, different extension)
//
// "Quests reloaden" in Server Control already runs
// `cd share/quest && python make.py`, which recompiles everything in
// `quest_list` via the server's own `qc_x64` tool into `object/` - so this
// module only has to manage the .lua/.quest *source* files and keep
// `quest_list` in sync; the compile step is already solved.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QuestFile {
    pub category: String,
    pub name: String,
    pub extension: String,
    pub relative_path: String,
}

/// Parses `quest_list` into structured entries. Unlike `mobdrop::parse`,
/// this never needs to reject/repair the file - it's just a flat list of
/// paths, so any non-blank line is accepted as-is.
pub fn parse_quest_list(content: &str) -> Vec<QuestFile> {
    content
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|line| {
            let (category, filename) = match line.rfind('/') {
                Some(idx) => (&line[..idx], &line[idx + 1..]),
                None => ("", line),
            };
            let (name, extension) = match filename.rfind('.') {
                Some(idx) => (&filename[..idx], &filename[idx + 1..]),
                None => (filename, ""),
            };
            QuestFile {
                category: category.to_string(),
                name: name.to_string(),
                extension: extension.to_string(),
                relative_path: line.to_string(),
            }
        })
        .collect()
}

/// Appends a path to `quest_list` if it isn't already there. Line-based and
/// minimal-diff on purpose - this file is also touched by the server's own
/// tooling, so it shouldn't be rewritten wholesale.
pub fn quest_list_add(content: &str, relative_path: &str) -> String {
    if content.lines().any(|l| l.trim() == relative_path) {
        return content.to_string();
    }
    let mut out = content.trim_end_matches('\n').to_string();
    if !out.is_empty() {
        out.push('\n');
    }
    out.push_str(relative_path);
    out.push('\n');
    out
}

pub fn quest_list_remove(content: &str, relative_path: &str) -> String {
    let kept: Vec<&str> = content
        .lines()
        .filter(|l| l.trim() != relative_path)
        .collect();
    if kept.is_empty() {
        String::new()
    } else {
        format!("{}\n", kept.join("\n"))
    }
}

/// Turns a free-typed name into a safe identifier: no spaces, no umlauts
/// (same transliteration rule as the Mob Drop Editor's group names), and -
/// unlike group names - never starting with a digit, since this also has to
/// be valid after `quest <name> begin` in the Lua source (a Lua identifier
/// can't start with a digit).
pub fn sanitize_identifier(input: &str) -> String {
    let replaced = input
        .replace('ä', "ae")
        .replace('Ä', "Ae")
        .replace('ö', "oe")
        .replace('Ö', "Oe")
        .replace('ü', "ue")
        .replace('Ü', "Ue")
        .replace('ß', "ss");

    let mut out = String::new();
    for ch in replaced.chars() {
        if ch.is_whitespace() {
            out.push('_');
        } else if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        }
    }

    if out.chars().next().is_some_and(|c| c.is_ascii_digit()) {
        out = format!("q_{out}");
    }
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestSearchLine {
    pub line_number: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestSearchMatch {
    pub relative_path: String,
    pub category: String,
    pub name: String,
    pub lines: Vec<QuestSearchLine>,
}

/// Case-insensitive line search across already-read quest file contents -
/// split out from the Tauri command so it's testable without a live SFTP
/// connection. `contents` must be the same length/order as `files` (a failed
/// read for one file just excludes it, rather than failing the whole search).
pub fn search_contents(
    files: &[QuestFile],
    contents: &[Result<String, String>],
    query: &str,
) -> Vec<QuestSearchMatch> {
    let needle = query.to_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }
    files
        .iter()
        .zip(contents.iter())
        .filter_map(|(file, content)| {
            let content = content.as_ref().ok()?;
            let lines: Vec<QuestSearchLine> = content
                .lines()
                .enumerate()
                .filter(|(_, line)| line.to_lowercase().contains(&needle))
                .map(|(i, line)| QuestSearchLine {
                    line_number: i + 1,
                    text: line.to_string(),
                })
                .collect();
            if lines.is_empty() {
                None
            } else {
                Some(QuestSearchMatch {
                    relative_path: file.relative_path.clone(),
                    category: file.category.clone(),
                    name: file.name.clone(),
                    lines,
                })
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Biologie/Biochecker.lua\nGilde/Gilden_Npcs.lua\nRuns/Beran.quest\n";

    #[test]
    fn parses_quest_list() {
        let files = parse_quest_list(SAMPLE);
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].category, "Biologie");
        assert_eq!(files[0].name, "Biochecker");
        assert_eq!(files[0].extension, "lua");
        assert_eq!(files[0].relative_path, "Biologie/Biochecker.lua");
        assert_eq!(files[2].category, "Runs");
        assert_eq!(files[2].extension, "quest");
    }

    #[test]
    fn adds_new_entry_once() {
        let once = quest_list_add(SAMPLE, "Pferd/Neue_Quest.lua");
        assert!(once.contains("Pferd/Neue_Quest.lua"));
        let twice = quest_list_add(&once, "Pferd/Neue_Quest.lua");
        assert_eq!(once, twice);
    }

    #[test]
    fn removes_entry() {
        let removed = quest_list_remove(SAMPLE, "Gilde/Gilden_Npcs.lua");
        assert!(!removed.contains("Gilden_Npcs"));
        assert!(removed.contains("Biochecker"));
        assert!(removed.contains("Beran.quest"));
    }

    #[test]
    fn sanitizes_names_like_mob_drop_but_never_leads_with_a_digit() {
        assert_eq!(sanitize_identifier("Stein der Willkür"), "Stein_der_Willkuer");
        assert_eq!(sanitize_identifier("3 Wünsche"), "q_3_Wuensche");
    }

    #[test]
    fn searches_contents_case_insensitively_and_skips_failed_reads() {
        let files = parse_quest_list(SAMPLE);
        let contents = vec![
            Ok("when NPC.chat.\"Start\" begin\n  say(\"Hallo Abenteurer\")\nend".to_string()),
            Err("SFTP-Fehler".to_string()),
            Ok("d.regen_file(\"x\")".to_string()),
        ];
        let results = search_contents(&files, &contents, "ABENTEURER");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].relative_path, "Biologie/Biochecker.lua");
        assert_eq!(results[0].lines.len(), 1);
        assert_eq!(results[0].lines[0].line_number, 2);
    }

    #[test]
    fn empty_query_returns_no_matches() {
        let files = parse_quest_list(SAMPLE);
        let contents = vec![Ok("abc".to_string()), Ok("def".to_string()), Ok("ghi".to_string())];
        assert!(search_contents(&files, &contents, "").is_empty());
    }
}
