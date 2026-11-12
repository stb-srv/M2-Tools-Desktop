//! Datenmodell und reine Logik des Common-Drop-Editors (ohne UI), damit
//! sie per `cargo test` prüfbar ist. Das Format selbst liest und schreibt
//! das Backend (`src-tauri/src/common_drop.rs`).

use serde::{Deserialize, Serialize};

pub const RANK_COUNT: usize = 4;
/// Datei-Wert für 100 %.
pub const MAX_CHANCE: f64 = 400.0;
/// Standard-Bereich für neue Regeln.
pub const DEFAULT_LEVEL_FROM: i64 = 1;
pub const DEFAULT_LEVEL_TO: i64 = 120;

pub struct RankInfo {
    pub key: &'static str,
    pub short: &'static str,
    pub title: &'static str,
    pub examples: &'static str,
}

pub const RANKS: [RankInfo; RANK_COUNT] = [
    RankInfo { key: "PAWN", short: "P", title: "Normale Mobs", examples: "z.B. Wildhund, Wolf, Alpha-Wolf" },
    RankInfo { key: "S_PAWN", short: "SP", title: "Stärkere Mobs", examples: "z.B. Roter Keiler, Dryade" },
    RankInfo { key: "KNIGHT", short: "K", title: "Elite", examples: "z.B. Verfl. Wolf, Goldener Frosch" },
    RankInfo { key: "S_KNIGHT", short: "SK", title: "Starke Elite", examples: "z.B. Affenanführer, Böser Höllengeneral" },
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Block {
    pub name: String,
    pub level_from: i64,
    pub level_to: i64,
    pub chance: f64,
    pub vnum: u32,
    pub count: i64,
}

impl Block {
    pub fn is_active(&self) -> bool {
        self.level_from != 0 && self.chance > 0.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Line {
    Text { text: String },
    Rule { blocks: Vec<Block> },
    Invalid { raw: String, error: String },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DropFile {
    pub lines: Vec<Line>,
    #[serde(default)]
    pub crlf: bool,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Issue {
    pub line_index: usize,
    pub rank: Option<usize>,
    /// "error" | "warning"
    pub level: String,
    pub message: String,
}

impl Issue {
    pub fn is_error(&self) -> bool {
        self.level == "error"
    }
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ItemBrief {
    pub vnum: u32,
    pub locale_name: String,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ItemCheck {
    pub vnum: u32,
    pub exists: bool,
    pub locale_name: Option<String>,
    pub name_collision: Option<ItemBrief>,
}

/// Datei-Wert -> reale Prozent bei gleichem Level.
pub fn chance_to_percent(chance: f64) -> f64 {
    chance / 4.0
}

/// Reale Prozent -> Datei-Wert, auf 6 Nachkommastellen gerundet, damit
/// z.B. 0,026 % sauber als 0.104 in der Datei landet.
pub fn percent_to_chance(percent: f64) -> f64 {
    if !percent.is_finite() || percent <= 0.0 {
        return 0.0;
    }
    ((percent * 4.0) * 1_000_000.0).round() / 1_000_000.0
}

/// "jeder 100. Mob" - im Schnitt, bei gleichem Level.
pub fn every_nth(chance: f64) -> Option<u64> {
    if chance <= 0.0 || !chance.is_finite() {
        return None;
    }
    Some((MAX_CHANCE / chance).round().max(1.0) as u64)
}

pub fn every_nth_label(chance: f64) -> String {
    match every_nth(chance) {
        None => "dropt nie".into(),
        Some(1) => "jedes Mal".into(),
        Some(n) => format!("≈ jeder {}. Mob", de_thousands(n)),
    }
}

fn de_thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    out
}

/// Prozent kurz und auf Deutsch: 1 -> "1 %", 0.026 -> "0,026 %".
pub fn fmt_percent(percent: f64) -> String {
    let fixed = format!("{percent:.4}");
    let trimmed = fixed.trim_end_matches('0').trim_end_matches('.');
    format!("{} %", trimmed.replace('.', ","))
}

/// Zahl wie im Backend geschrieben (4 -> "4", 0.104 -> "0.104").
pub fn fmt_chance(chance: f64) -> String {
    format!("{chance}")
}

/// Alle 4 Blöcke mit demselben Item und derselben Notiz?
pub fn is_shared(blocks: &[Block]) -> bool {
    blocks.windows(2).all(|w| w[0].vnum == w[1].vnum && w[0].name == w[1].name)
}

/// Notiz ohne TAB/Zeilenumbruch (sonst verrutschen die Felder).
pub fn clean_name(name: &str) -> String {
    name.replace(['\t', '\r', '\n'], " ")
}

pub fn new_rule(vnum: u32, name: &str) -> Vec<Block> {
    (0..RANK_COUNT)
        .map(|_| Block {
            name: clean_name(name),
            level_from: DEFAULT_LEVEL_FROM,
            level_to: DEFAULT_LEVEL_TO,
            chance: 4.0,
            vnum,
            count: 1,
        })
        .collect()
}

/// Block ein- oder ausschalten. Aus = Chance 0 (Level bleiben stehen).
pub fn set_active(block: &mut Block, active: bool) {
    if active {
        if block.level_from == 0 {
            block.level_from = DEFAULT_LEVEL_FROM;
        }
        if block.level_to == 0 {
            block.level_to = DEFAULT_LEVEL_TO;
        }
        if block.chance <= 0.0 {
            block.chance = 4.0;
        }
    } else {
        block.chance = 0.0;
    }
}

/// Level und Chance von Block `from` auf alle anderen übertragen.
pub fn copy_values_to_all(blocks: &mut [Block], from: usize) {
    let Some(src) = blocks.get(from).cloned() else { return };
    for b in blocks.iter_mut() {
        b.level_from = src.level_from;
        b.level_to = src.level_to;
        b.chance = src.chance;
    }
}

/// Item und Notiz von Block 0 auf alle übertragen (zurück zu "ein Item").
pub fn share_item(blocks: &mut [Block]) {
    let Some(first) = blocks.first().cloned() else { return };
    for b in blocks.iter_mut() {
        b.vnum = first.vnum;
        b.name = first.name.clone();
    }
}

/// Eine Rohzeile zum Anzeigen/Bearbeiten: TAB als "→".
pub fn raw_for_edit(raw: &str) -> String {
    raw.replace('\t', "→")
}

/// Rückweg von [`raw_for_edit`]: "→" und echte TABs werden zu TAB.
pub fn raw_from_edit(text: &str) -> String {
    text.replace('→', "\t").replace(['\r', '\n'], "")
}

/// Die 4 Blöcke so, wie sie in der Datei stehen (pro Block 6 Felder).
pub fn preview_fields(blocks: &[Block]) -> Vec<Vec<String>> {
    blocks
        .iter()
        .map(|b| {
            vec![
                clean_name(&b.name),
                b.level_from.to_string(),
                b.level_to.to_string(),
                fmt_chance(b.chance),
                b.vnum.to_string(),
                b.count.to_string(),
            ]
        })
        .collect()
}

/// Passt eine Regel zur Suche (Notiz, Item-Name oder VNUM)?
pub fn matches_search(blocks: &[Block], label: impl Fn(u32) -> Option<String>, query: &str) -> bool {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return true;
    }
    blocks.iter().any(|b| {
        b.name.to_lowercase().contains(&q)
            || b.vnum.to_string() == q
            || b.vnum.to_string().starts_with(&q)
            || label(b.vnum).is_some_and(|l| l.to_lowercase().contains(&q))
    })
}

/// Fehler zu Items aus der Datenbankprüfung, pro Regel und VNUM einmal.
pub fn item_issues(file: &DropFile, checks: &std::collections::HashMap<u32, ItemCheck>) -> Vec<Issue> {
    let mut out = Vec::new();
    for (li, line) in file.lines.iter().enumerate() {
        let Line::Rule { blocks } = line else { continue };
        let mut seen = Vec::new();
        for (r, b) in blocks.iter().enumerate() {
            if seen.contains(&b.vnum) {
                continue;
            }
            seen.push(b.vnum);
            let shared_vnum = blocks.iter().all(|o| o.vnum == b.vnum);
            let rank = if shared_vnum { None } else { Some(r) };
            let prefix = if shared_vnum { String::new() } else { format!("{}: ", RANKS[r].key) };
            let Some(check) = checks.get(&b.vnum) else { continue };
            if !check.exists {
                out.push(Issue {
                    line_index: li,
                    rank,
                    level: "error".into(),
                    message: format!(
                        "{prefix}Item {} gibt es nicht in item_proto - der Server startet dann nicht.",
                        b.vnum
                    ),
                });
            }
            if let Some(other) = &check.name_collision {
                out.push(Issue {
                    line_index: li,
                    rank,
                    level: "error".into(),
                    message: format!(
                        "{prefix}Der Server liest \"{}\" als Item-Namen und nimmt dann Item {} ({}) statt {}.",
                        b.vnum, other.vnum, other.locale_name, b.vnum
                    ),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn percent_conversion() {
        assert_eq!(percent_to_chance(100.0), 400.0);
        assert_eq!(percent_to_chance(1.0), 4.0);
        assert_eq!(percent_to_chance(0.1), 0.4);
        assert_eq!(percent_to_chance(0.026), 0.104);
        assert_eq!(percent_to_chance(-3.0), 0.0);
        assert_eq!(chance_to_percent(0.08), 0.02);
    }

    #[test]
    fn every_nth_matches_ymir_notes() {
        assert_eq!(every_nth(0.08), Some(5000));
        assert_eq!(every_nth(0.104), Some(3846));
        assert_eq!(every_nth(0.12), Some(3333));
        assert_eq!(every_nth(0.32), Some(1250));
        assert_eq!(every_nth(400.0), Some(1));
        assert_eq!(every_nth(0.0), None);
        assert_eq!(every_nth_label(0.08), "≈ jeder 5.000. Mob");
        assert_eq!(every_nth_label(4.0), "≈ jeder 100. Mob");
        assert_eq!(every_nth_label(400.0), "jedes Mal");
    }

    #[test]
    fn percent_label() {
        assert_eq!(fmt_percent(1.0), "1 %");
        assert_eq!(fmt_percent(0.026), "0,026 %");
        assert_eq!(fmt_percent(62.5), "62,5 %");
        assert_eq!(fmt_percent(0.0), "0 %");
    }

    #[test]
    fn activation() {
        let mut b = Block { name: "x".into(), level_from: 0, level_to: 0, chance: 0.0, vnum: 11, count: 1 };
        assert!(!b.is_active());
        set_active(&mut b, true);
        assert!(b.is_active());
        assert_eq!((b.level_from, b.level_to, b.chance), (1, 120, 4.0));
        set_active(&mut b, false);
        assert!(!b.is_active());
        assert_eq!(b.level_from, 1);
    }

    #[test]
    fn sharing_and_copying() {
        let mut blocks = new_rule(50513, "See\tle");
        assert!(is_shared(&blocks));
        assert_eq!(blocks[0].name, "See le");
        blocks[2].vnum = 11;
        assert!(!is_shared(&blocks));
        share_item(&mut blocks);
        assert!(is_shared(&blocks));
        blocks[1].chance = 8.0;
        blocks[1].level_from = 75;
        copy_values_to_all(&mut blocks, 1);
        assert!(blocks.iter().all(|b| b.chance == 8.0 && b.level_from == 75));
    }

    #[test]
    fn raw_edit_round_trip() {
        let raw = "a\t1\t2";
        assert_eq!(raw_for_edit(raw), "a→1→2");
        assert_eq!(raw_from_edit("a→1→2\n"), raw);
    }

    #[test]
    fn search() {
        let blocks = new_rule(50513, "Seele");
        let label = |_| Some("Seelenstein".to_string());
        assert!(matches_search(&blocks, label, ""));
        assert!(matches_search(&blocks, label, "seele"));
        assert!(matches_search(&blocks, label, "505"));
        assert!(matches_search(&blocks, |_| Some("Seelenstein".to_string()), "stein"));
        assert!(!matches_search(&blocks, |_| None, "schwert"));
    }

    #[test]
    fn item_issue_per_vnum() {
        let mut blocks = new_rule(99999, "x");
        let file = DropFile { lines: vec![Line::Text { text: "#".into() }, Line::Rule { blocks: blocks.clone() }], crlf: false };
        let mut checks = HashMap::new();
        checks.insert(99999, ItemCheck { vnum: 99999, exists: false, locale_name: None, name_collision: None });
        let issues = item_issues(&file, &checks);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].line_index, 1);
        assert_eq!(issues[0].rank, None);

        blocks[3].vnum = 14;
        checks.insert(
            14,
            ItemCheck {
                vnum: 14,
                exists: true,
                locale_name: Some("Schwert+4".into()),
                name_collision: Some(ItemBrief { vnum: 50029, locale_name: "Goldring".into() }),
            },
        );
        let file = DropFile { lines: vec![Line::Rule { blocks }], crlf: false };
        let issues = item_issues(&file, &checks);
        assert_eq!(issues.len(), 2);
        assert!(issues[1].message.starts_with("S_KNIGHT: ") && issues[1].message.contains("50029"));
        assert_eq!(issues[1].rank, Some(3));
    }
}
