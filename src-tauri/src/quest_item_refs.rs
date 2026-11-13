//! Format-bewusste Suche nach einer Item-VNUM in Quest-Dateien für „Wo wird
//! das benutzt?“. Die frühere reine Teilstring-Suche fand für VNUM 10 jede
//! „10“ (100, 2010, `pc.level >= 10`, Texte in `say(...)` …). Hier zählt nur
//! eine Zahl, die
//!
//! - als eigenständige Zahl dasteht (keine Teilzahl, keine Kommazahl),
//! - nicht in einem Kommentar oder einer Zeichenkette steht und
//! - in einem Item-Zusammenhang vorkommt: erstes Argument einer Item-Funktion
//!   (`pc.give_item2(10, 1)`, `pc.count_item(10)`, `pc.remove_item(10)`,
//!   `item_name(10)` …), Vergleich mit `item.vnum`/`item.get_vnum()` oder ein
//!   Item-Trigger wie `when 10.use`.
//!
//! Übrige Stellen, an denen die Zahl zwar eigenständig im Code steht, aber
//! nicht als Item erkennbar ist (z.B. in einer VNUM-Tabelle `{10, 20}`),
//! werden getrennt als „ungeprüft“ zurückgegeben.

use crate::quest::QuestFile;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QuestItemRef {
    pub relative_path: String,
    pub line_number: usize,
    pub text: String,
    /// Kurzbeschreibung, woran die Stelle als Item erkannt wurde; leer bei
    /// ungeprüften Stellen.
    pub kind: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct QuestItemScan {
    pub confirmed: Vec<QuestItemRef>,
    pub unconfirmed: Vec<QuestItemRef>,
}

/// Funktionen, deren erstes Argument eine Item-VNUM ist (letztes Glied des
/// Aufrufs, also `pc.give_item2` und `give_item2` gleichermaßen).
const ITEM_FUNCTIONS: &[&str] = &[
    "give_item",
    "give_item2",
    "give_item2_select",
    "give_or_drop_item",
    "give_or_drop_item_and_select",
    "give_item_from_special_item_group",
    "remove_item",
    "removeitem",
    "count_item",
    "countitem",
    "item_name",
    "get_item_name",
    "drop_item",
    "drop_item_with_ownership",
    "enough_inventory",
];

/// Quest-Trigger, bei denen `<vnum>.<trigger>` ein Item meint.
const ITEM_TRIGGERS: &[&str] = &["use", "pick", "equip", "unequip"];

pub fn scan_files(files: &[QuestFile], contents: &[Result<String, String>], vnum: u32) -> QuestItemScan {
    let mut scan = QuestItemScan::default();
    for (file, content) in files.iter().zip(contents.iter()) {
        if let Ok(content) = content {
            scan_content(&file.relative_path, content, vnum, &mut scan);
        }
    }
    scan
}

fn scan_content(path: &str, content: &str, vnum: u32, scan: &mut QuestItemScan) {
    let needle = vnum.to_string();
    let code = blank_strings_and_comments(content);
    for (i, (code_line, raw_line)) in code.lines().zip(content.lines()).enumerate() {
        let mut kinds: Vec<&'static str> = Vec::new();
        let mut standalone = false;
        for (start, end) in standalone_numbers(code_line, &needle) {
            standalone = true;
            if let Some(kind) = classify(code_line, start, end) {
                if !kinds.contains(&kind) {
                    kinds.push(kind);
                }
            }
        }
        if !standalone {
            continue;
        }
        let entry = QuestItemRef {
            relative_path: path.to_string(),
            line_number: i + 1,
            text: raw_line.trim().to_string(),
            kind: kinds.join(", "),
        };
        if kinds.is_empty() {
            scan.unconfirmed.push(entry);
        } else {
            scan.confirmed.push(entry);
        }
    }
}

/// Ersetzt Inhalte von Zeichenketten (`"…"`, `'…'`, `[[…]]`) und Kommentaren
/// (`-- …`, `--[[ … ]]`) durch Leerzeichen. Zeilenumbrüche und Zeichen-
/// positionen bleiben erhalten, damit Zeilennummern und Spalten weiter zum
/// Original passen.
fn blank_strings_and_comments(src: &str) -> String {
    #[derive(PartialEq)]
    enum Mode {
        Code,
        Str(char),
        LongStr,
        LineComment,
        BlockComment,
    }
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut mode = Mode::Code;
    let mut i = 0;
    let blank = |c: char| if c == '\n' { '\n' } else { ' ' };
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        match mode {
            Mode::Code => {
                if c == '-' && next == Some('-') {
                    if chars.get(i + 2) == Some(&'[') && chars.get(i + 3) == Some(&'[') {
                        mode = Mode::BlockComment;
                        out.push_str("    ");
                        i += 4;
                    } else {
                        mode = Mode::LineComment;
                        out.push_str("  ");
                        i += 2;
                    }
                    continue;
                }
                if c == '[' && next == Some('[') {
                    mode = Mode::LongStr;
                    out.push_str("[[");
                    i += 2;
                    continue;
                }
                if c == '"' || c == '\'' {
                    mode = Mode::Str(c);
                }
                out.push(c);
            }
            Mode::Str(q) => {
                if c == '\\' && next.is_some() {
                    out.push(' ');
                    out.push(blank(next.unwrap()));
                    i += 2;
                    continue;
                }
                if c == q {
                    mode = Mode::Code;
                    out.push(c);
                } else if c == '\n' {
                    // Nicht geschlossene Zeichenkette endet an der Zeile.
                    mode = Mode::Code;
                    out.push('\n');
                } else {
                    out.push(' ');
                }
            }
            Mode::LongStr | Mode::BlockComment => {
                if c == ']' && next == Some(']') {
                    let was_str = mode == Mode::LongStr;
                    mode = Mode::Code;
                    out.push_str(if was_str { "]]" } else { "  " });
                    i += 2;
                    continue;
                }
                out.push(blank(c));
            }
            Mode::LineComment => {
                if c == '\n' {
                    mode = Mode::Code;
                }
                out.push(blank(c));
            }
        }
        i += 1;
    }
    out
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Byte-Bereiche, in denen `needle` als eigenständige ganze Zahl vorkommt.
fn standalone_numbers(line: &str, needle: &str) -> Vec<(usize, usize)> {
    let mut found = Vec::new();
    let bytes = line.as_bytes();
    let mut from = 0;
    while let Some(pos) = line[from..].find(needle) {
        let start = from + pos;
        let end = start + needle.len();
        from = start + 1;
        let prev = start.checked_sub(1).map(|p| bytes[p] as char);
        if prev.is_some_and(|p| is_ident_char(p) || p == '.') {
            continue;
        }
        let next = bytes.get(end).map(|&b| b as char);
        if next.is_some_and(is_ident_char) {
            continue;
        }
        if next == Some('.') && bytes.get(end + 1).is_some_and(|b| b.is_ascii_digit()) {
            continue; // Kommazahl wie 10.5
        }
        found.push((start, end));
    }
    found
}

fn without_spaces(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

fn is_item_vnum_expr_end(s: &str) -> bool {
    let s = without_spaces(s).to_ascii_lowercase();
    s.ends_with("item.vnum") || s.ends_with("item.get_vnum()")
}

fn is_item_vnum_expr_start(s: &str) -> bool {
    let s = without_spaces(s).to_ascii_lowercase();
    s.starts_with("item.vnum") || s.starts_with("item.get_vnum()")
}

fn classify(line: &str, start: usize, end: usize) -> Option<&'static str> {
    let before = line[..start].trim_end();
    let after = &line[end..];

    // when 10.use begin
    if let Some(rest) = after.strip_prefix('.') {
        let trigger: String = rest.chars().take_while(|c| is_ident_char(*c)).collect();
        if ITEM_TRIGGERS.contains(&trigger.to_ascii_lowercase().as_str()) {
            return Some("Item-Trigger");
        }
    }

    // pc.give_item2(10, …) - nur als erstes Argument
    if let Some(call) = before.strip_suffix('(') {
        let call = call.trim_end();
        let name: String = call
            .chars()
            .rev()
            .take_while(|c| is_ident_char(*c))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        if ITEM_FUNCTIONS.contains(&name.to_ascii_lowercase().as_str()) {
            return Some("Item-Funktion");
        }
    }

    // item.vnum == 10  /  10 == item.vnum
    for op in ["==", "~="] {
        if let Some(lhs) = before.strip_suffix(op) {
            if is_item_vnum_expr_end(lhs) {
                return Some("Vergleich mit item.vnum");
            }
        }
        if let Some(rhs) = after.trim_start().strip_prefix(op) {
            if is_item_vnum_expr_start(rhs) {
                return Some("Vergleich mit item.vnum");
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(src: &str, vnum: u32) -> QuestItemScan {
        let mut s = QuestItemScan::default();
        scan_content("Test/test.quest", src, vnum, &mut s);
        s
    }

    fn confirmed_lines(s: &QuestItemScan) -> Vec<usize> {
        s.confirmed.iter().map(|r| r.line_number).collect()
    }

    #[test]
    fn ignores_other_numbers_containing_the_digits() {
        let src = "pc.give_item2(100, 1)\npc.give_item2(2010)\npc.give_item2(27001, 10)\nlocal x = 10.5\n";
        let s = scan(src, 10);
        assert!(s.confirmed.is_empty());
        // `27001, 10` steht eigenständig da, ist aber die Anzahl, kein Item.
        assert_eq!(s.unconfirmed.len(), 1);
        assert_eq!(s.unconfirmed[0].line_number, 3);
    }

    #[test]
    fn finds_item_function_calls() {
        let src = "\
pc.give_item2(10, 1)
if pc.count_item( 10 ) >= 1 then
pc.remove_item(10)
say(item_name(10))
give_item2(10)
";
        let s = scan(src, 10);
        assert_eq!(confirmed_lines(&s), vec![1, 2, 3, 4, 5]);
        assert_eq!(s.confirmed[0].kind, "Item-Funktion");
    }

    #[test]
    fn finds_triggers_and_comparisons() {
        let src = "\
when 10.use begin
if item.vnum == 10 then
if item.get_vnum() ~= 10 then
if 10 == item.vnum then
when 10.chat.\"Hallo\" begin
";
        let s = scan(src, 10);
        assert_eq!(confirmed_lines(&s), vec![1, 2, 3, 4]);
        assert_eq!(s.confirmed[0].kind, "Item-Trigger");
        assert_eq!(s.confirmed[1].kind, "Vergleich mit item.vnum");
        // NPC-Trigger `10.chat` ist kein Item
        assert_eq!(s.unconfirmed.len(), 1);
    }

    #[test]
    fn ignores_level_checks_strings_and_comments() {
        let src = "\
if pc.level >= 10 then
say(\"Bring mir 10 Felle, z.B. pc.give_item2(10)\")
-- pc.give_item2(10) auskommentiert
--[[
pc.give_item2(10)
]]
local t = [[pc.give_item2(10)]]
";
        let s = scan(src, 10);
        assert!(s.confirmed.is_empty());
        assert_eq!(s.unconfirmed.len(), 1);
        assert_eq!(s.unconfirmed[0].line_number, 1);
    }

    #[test]
    fn keeps_line_numbers_after_block_comment() {
        let src = "--[[ a\nb ]]\npc.give_item2(10)\n";
        let s = scan(src, 10);
        assert_eq!(confirmed_lines(&s), vec![3]);
        assert_eq!(s.confirmed[0].text, "pc.give_item2(10)");
    }

    #[test]
    fn table_entries_are_unconfirmed() {
        let s = scan("local items = {10, 11, 12}\n", 10);
        assert!(s.confirmed.is_empty());
        assert_eq!(s.unconfirmed.len(), 1);
    }

    #[test]
    fn scan_files_skips_failed_reads() {
        let files = crate::quest::parse_quest_list("A/a.quest\nB/b.quest\n");
        let contents = vec![Err("weg".to_string()), Ok("pc.give_item2(10)".to_string())];
        let s = scan_files(&files, &contents, 10);
        assert_eq!(s.confirmed.len(), 1);
        assert_eq!(s.confirmed[0].relative_path, "B/b.quest");
    }
}
