//! `common_drop_item.txt` im klassischen Rang-Format.
//!
//! Jede Datenzeile hat 4 Blöcke zu je 6 TAB-getrennten Feldern:
//!
//! ```text
//! Name  LevelVon  LevelBis  Chance  VNUM  Anzahl   (x4: PAWN, S_PAWN, KNIGHT, S_KNIGHT)
//! ```
//!
//! Der Mob-Rang ergibt sich nur aus der Position des Blocks. So liest es
//! `ITEM_MANAGER::ReadCommonDropItemFile`, und so sieht auch die
//! Beispieldatei vom Server des Nutzers aus (2026-11-12). Eine frühere
//! Version dieses Editors hat die Datei im `mob_drop_item.txt`-Format
//! (`Group`/`Mob`/`Type`) gelesen; das war falsch.
//!
//! Was der Server dabei tut und was dieser Editor deshalb prüft:
//! - Das Feld `Name` liest der Server nicht.
//! - `LevelVon` 0 schaltet den Block aus, `Chance` 0 auch (dropt nie).
//! - `Chance` ist der Datei-Wert, die reale Chance bei gleichem Level ist
//!   Datei-Wert ÷ 4 (400 = 100 %). Größere Werte als 400 verrechnen sich.
//! - Die VNUM wird für **jeden** Block nachgeschlagen, auch für einen
//!   ausgeschalteten. Gibt es das Item nicht, startet der Server nicht.
//!   Der Server versucht das Feld zuerst als internen Item-Namen; die
//!   Nummern 1, 2, 14, 18 und 24 landen dadurch bei den Goldringen.
//! - `Anzahl` wird gelesen, aber nicht benutzt (es dropt immer 1 Stück).
//! - Zeilen ohne TAB und Zeilen, die mit `#` beginnen, ignoriert der Server.
//!   Sie bleiben hier unverändert an ihrer Stelle erhalten.
//! - Kein `/reload` - Änderungen wirken erst nach einem Server-Neustart.

use serde::{Deserialize, Serialize};

pub const RANK_COUNT: usize = 4;
pub const FIELDS_PER_BLOCK: usize = 6;
pub const FIELDS_PER_LINE: usize = RANK_COUNT * FIELDS_PER_BLOCK;
pub const RANK_NAMES: [&str; RANK_COUNT] = ["PAWN", "S_PAWN", "KNIGHT", "S_KNIGHT"];
/// Datei-Wert für 100 %.
pub const MAX_CHANCE: f64 = 400.0;
/// VNUMs, die der Server über die Namenssuche mit anderen Items verwechselt.
pub const FORBIDDEN_VNUMS: [u32; 5] = [1, 2, 14, 18, 24];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonDropBlock {
    pub name: String,
    pub level_from: i64,
    pub level_to: i64,
    pub chance: f64,
    pub vnum: u32,
    pub count: i64,
}

impl CommonDropBlock {
    /// Würfelt der Server diesen Block überhaupt?
    pub fn is_active(&self) -> bool {
        self.level_from != 0 && self.chance > 0.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CommonDropLine {
    /// Kommentar, Leerzeile oder Überschrift - wird unverändert geschrieben.
    Text { text: String },
    Rule { blocks: Vec<CommonDropBlock> },
    /// Datenzeile, die sich nicht lesen ließ. Speichern ist gesperrt,
    /// solange es solche Zeilen gibt.
    Invalid { raw: String, error: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonDropFile {
    pub lines: Vec<CommonDropLine>,
    /// Datei hatte Windows-Zeilenenden - beim Schreiben beibehalten.
    #[serde(default)]
    pub crlf: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommonDropIssue {
    /// Index in `CommonDropFile::lines` (nicht die Zeilennummer).
    pub line_index: usize,
    /// 0..4, wenn das Problem einen bestimmten Block betrifft.
    pub rank: Option<usize>,
    pub level: IssueLevel,
    pub message: String,
}

fn is_text_line(line: &str) -> bool {
    !line.contains('\t') || line.trim_start().starts_with('#')
}

/// Überschrift wie im Original (`PAWN\t\t...\tS_PAWN...`).
fn is_header_line(line: &str) -> bool {
    line.split('\t').next().is_some_and(|f| f.trim().eq_ignore_ascii_case("PAWN"))
}

fn parse_int(field: &str, what: &str) -> Result<i64, String> {
    let t = field.trim();
    t.parse::<i64>().map_err(|_| {
        if t.is_empty() {
            format!("{what} ist leer")
        } else {
            format!("{what} ist keine ganze Zahl ('{t}')")
        }
    })
}

fn parse_chance(field: &str) -> Result<f64, String> {
    let t = field.trim();
    if t.contains(',') {
        return Err(format!("Chance '{t}' hat ein Komma - der Server liest dann 0. Bitte Punkt verwenden"));
    }
    match t.parse::<f64>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ if t.is_empty() => Err("Chance ist leer".into()),
        _ => Err(format!("Chance ist keine Zahl ('{t}')")),
    }
}

fn parse_vnum(field: &str) -> Result<u32, String> {
    let t = field.trim();
    t.parse::<u32>().map_err(|_| {
        if t.is_empty() {
            "VNUM ist leer".to_string()
        } else {
            format!("VNUM '{t}' ist keine Nummer (Item-Namen werden in diesem Editor nicht unterstützt)")
        }
    })
}

/// Liest eine einzelne Datenzeile (mit TABs) in 4 Blöcke.
pub fn parse_rule(line: &str) -> Result<Vec<CommonDropBlock>, String> {
    let mut fields: Vec<&str> = line.split('\t').collect();
    // Ein überzähliger TAB am Zeilenende stört den Server nicht.
    while fields.len() > FIELDS_PER_LINE && fields.last().is_some_and(|f| f.trim().is_empty()) {
        fields.pop();
    }
    if fields.len() != FIELDS_PER_LINE {
        return Err(format!(
            "{} Felder statt {FIELDS_PER_LINE} - ein TAB fehlt oder ist zu viel, dadurch rutschen die Werte in die falschen Felder",
            fields.len()
        ));
    }
    let mut blocks = Vec::with_capacity(RANK_COUNT);
    for (rank, chunk) in fields.chunks(FIELDS_PER_BLOCK).enumerate() {
        let r = RANK_NAMES[rank];
        let block = (|| -> Result<CommonDropBlock, String> {
            Ok(CommonDropBlock {
                name: chunk[0].to_string(),
                level_from: parse_int(chunk[1], "LevelVon")?,
                level_to: parse_int(chunk[2], "LevelBis")?,
                chance: parse_chance(chunk[3])?,
                vnum: parse_vnum(chunk[4])?,
                count: parse_int(chunk[5], "Anzahl")?,
            })
        })()
        .map_err(|e| format!("Block {} ({r}): {e}", rank + 1))?;
        blocks.push(block);
    }
    Ok(blocks)
}

/// Liest die ganze Datei. Schlägt nie fehl: unlesbare Datenzeilen kommen
/// als `Invalid` zurück, damit der Editor sie mit Zeilennummer zeigen kann.
pub fn parse(content: &str) -> CommonDropFile {
    let crlf = content.contains("\r\n");
    let mut lines = Vec::new();
    for raw in content.lines() {
        let raw = raw.strip_suffix('\r').unwrap_or(raw);
        if is_text_line(raw) || is_header_line(raw) {
            lines.push(CommonDropLine::Text { text: raw.to_string() });
            continue;
        }
        match parse_rule(raw) {
            Ok(blocks) => lines.push(CommonDropLine::Rule { blocks }),
            Err(error) => lines.push(CommonDropLine::Invalid { raw: raw.to_string(), error }),
        }
    }
    CommonDropFile { lines, crlf }
}

/// Zahl so kurz wie möglich, ohne Exponent: 4 -> "4", 0.104 -> "0.104".
fn fmt_chance(v: f64) -> String {
    let s = format!("{v}");
    if s.contains('e') || s.contains('E') {
        let fixed = format!("{v:.6}");
        fixed.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

/// Notizen dürfen weder TAB noch Zeilenumbruch enthalten.
fn clean_name(name: &str) -> String {
    name.replace(['\t', '\r', '\n'], " ")
}

pub fn serialize_rule(blocks: &[CommonDropBlock]) -> String {
    blocks
        .iter()
        .flat_map(|b| {
            [
                clean_name(&b.name),
                b.level_from.to_string(),
                b.level_to.to_string(),
                fmt_chance(b.chance),
                b.vnum.to_string(),
                b.count.to_string(),
            ]
        })
        .collect::<Vec<_>>()
        .join("\t")
}

/// Kommentarzeile mit TAB würde der Server als Datenzeile lesen.
fn clean_text_line(text: &str) -> String {
    if text.contains('\t') && text.trim_start().starts_with('#') {
        text.replace('\t', " ")
    } else {
        text.to_string()
    }
}

pub fn serialize(file: &CommonDropFile) -> Result<String, String> {
    let eol = if file.crlf { "\r\n" } else { "\n" };
    let mut out = String::new();
    for (i, line) in file.lines.iter().enumerate() {
        match line {
            CommonDropLine::Text { text } => out.push_str(&clean_text_line(text)),
            CommonDropLine::Rule { blocks } => {
                if blocks.len() != RANK_COUNT {
                    return Err(format!("Regel {}: {} Blöcke statt {RANK_COUNT}.", i + 1, blocks.len()));
                }
                out.push_str(&serialize_rule(blocks));
            }
            CommonDropLine::Invalid { error, .. } => {
                return Err(format!("Zeile {} ist noch fehlerhaft: {error}", i + 1));
            }
        }
        out.push_str(eol);
    }
    Ok(out)
}

fn fmt_level_range(b: &CommonDropBlock) -> String {
    format!("{}–{}", b.level_from, b.level_to)
}

/// Prüfungen, die keine Datenbank brauchen. Ob die VNUM in `item_proto`
/// existiert, prüft `commands::check_common_drop_items` getrennt.
pub fn validate(file: &CommonDropFile) -> Vec<CommonDropIssue> {
    let mut issues = Vec::new();
    let mut push = |line_index: usize, rank: Option<usize>, level: IssueLevel, message: String| {
        issues.push(CommonDropIssue { line_index, rank, level, message });
    };

    for (li, line) in file.lines.iter().enumerate() {
        match line {
            CommonDropLine::Invalid { error, .. } => push(li, None, IssueLevel::Error, error.clone()),
            CommonDropLine::Text { text } => {
                if text.contains('\t') && text.trim_start().starts_with('#') {
                    push(
                        li,
                        None,
                        IssueLevel::Warning,
                        "Kommentarzeile enthält TABs - sie werden beim Speichern durch Leerzeichen ersetzt.".into(),
                    );
                }
            }
            CommonDropLine::Rule { blocks } => {
                if blocks.len() != RANK_COUNT {
                    push(li, None, IssueLevel::Error, format!("{} Blöcke statt {RANK_COUNT}.", blocks.len()));
                    continue;
                }
                for (r, b) in blocks.iter().enumerate() {
                    let rn = RANK_NAMES[r];
                    let mut err = |m: String| push(li, Some(r), IssueLevel::Error, format!("{rn}: {m}"));
                    if b.vnum == 0 {
                        err("VNUM 0 gibt es nicht - der Server startet dann nicht.".into());
                    } else if FORBIDDEN_VNUMS.contains(&b.vnum) {
                        err(format!(
                            "VNUM {} geht nicht - der Server verwechselt sie mit den Goldringen (50028-50030).",
                            b.vnum
                        ));
                    }
                    if b.level_from < 0 || b.level_to < 0 {
                        err("Level darf nicht negativ sein.".into());
                    }
                    if b.chance < 0.0 {
                        err("Chance darf nicht negativ sein.".into());
                    } else if b.chance > MAX_CHANCE {
                        err(format!(
                            "Chance {} ist mehr als 400 (= 100 %). Größere Werte verrechnen sich auf dem Server.",
                            fmt_chance(b.chance)
                        ));
                    }
                    if b.is_active() {
                        if b.level_to == 0 {
                            err("LevelBis ist 0 - der Block dropt dann nie.".into());
                        } else if b.level_to < b.level_from {
                            err(format!("LevelBis {} ist kleiner als LevelVon {}.", b.level_to, b.level_from));
                        }
                    }
                }

                if !blocks.iter().any(CommonDropBlock::is_active) {
                    push(li, None, IssueLevel::Warning, "Alle 4 Blöcke sind aus (Chance 0) - diese Regel dropt nie.".into());
                }

                // Unterschiedliche Level-Bereiche innerhalb einer Regel sind
                // erlaubt, aber oft ein Tippfehler.
                let with_levels: Vec<(usize, &CommonDropBlock)> =
                    blocks.iter().enumerate().filter(|(_, b)| b.level_from != 0).collect();
                if let Some((_, first)) = with_levels.first() {
                    let differs = with_levels
                        .iter()
                        .any(|(_, b)| b.level_from != first.level_from || b.level_to != first.level_to);
                    if differs {
                        let ranges = with_levels
                            .iter()
                            .map(|(r, b)| format!("{} {}", RANK_NAMES[*r], fmt_level_range(b)))
                            .collect::<Vec<_>>()
                            .join(", ");
                        push(li, None, IssueLevel::Warning, format!("Unterschiedliche Level-Bereiche ({ranges}) - Absicht?"));
                    }
                }
            }
        }
    }
    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# commen_drop_item.txt - Beispieldatei\n\
# Chance: gewünschte Prozent x 4 (100 % = 400, 1 % = 4, 0 = aus)\n\
#\n\
Seelenstein\t1\t75\t0\t50513\t1\tSeelenstein\t75\t120\t0\t50513\t1\tSeelenstein\t75\t120\t0\t50513\t1\tSeelenstein\t75\t120\t0\t50513\t1\n\
Gegenstandverzaubern\t1\t75\t400\t39028\t1\tGegenstandverzaubern\t75\t120\t0\t39028\t1\tGegenstandverzaubern\t75\t120\t0\t39028\t1\tGegenstandverzaubern\t75\t120\t0\t39028\t1\n\
#\n\
Schwert\t1\t15\t0.08\t11\t5000\tSchwert\t1\t15\t0.104\t11\t3846\tSchwert\t1\t15\t0.12\t11\t3333\tSchwert\t1\t15\t0.32\t11\t1250\n";

    fn rules(file: &CommonDropFile) -> Vec<&Vec<CommonDropBlock>> {
        file.lines
            .iter()
            .filter_map(|l| match l {
                CommonDropLine::Rule { blocks } => Some(blocks),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reads_the_users_sample_file() {
        let file = parse(SAMPLE);
        assert_eq!(file.lines.len(), 7);
        let rules = rules(&file);
        assert_eq!(rules.len(), 3);
        let sword = rules[2];
        assert_eq!(sword[0].chance, 0.08);
        assert_eq!(sword[1].chance, 0.104);
        assert_eq!(sword[3].count, 1250);
        assert_eq!(sword[2].vnum, 11);
        assert_eq!(sword[0].level_to, 15);
    }

    #[test]
    fn round_trips_byte_for_byte() {
        assert_eq!(serialize(&parse(SAMPLE)).unwrap(), SAMPLE);
    }

    #[test]
    fn keeps_windows_line_endings() {
        let crlf = SAMPLE.replace('\n', "\r\n");
        let file = parse(&crlf);
        assert!(file.crlf);
        assert_eq!(serialize(&file).unwrap(), crlf);
    }

    #[test]
    fn keeps_header_line_as_text() {
        let header = "PAWN\t\t\t\t\t\tS_PAWN\t\t\t\t\t\tKNIGHT\t\t\t\t\t\tS_KNIGHT\t\t\t\t\t";
        let file = parse(&format!("{header}\n"));
        assert_eq!(file.lines, vec![CommonDropLine::Text { text: header.into() }]);
    }

    #[test]
    fn reports_missing_tab_with_field_count() {
        let line = "Seele\t75\t120\t4\t50513\t1\tSeele\t75\t120\t0\t50513\t1\tSeele\t75\t120\t0\t50513\t1\tSeele\t75\t120\t0\t50513 1";
        let file = parse(line);
        match &file.lines[0] {
            CommonDropLine::Invalid { error, .. } => assert!(error.contains("23 Felder"), "{error}"),
            other => panic!("expected invalid, got {other:?}"),
        }
        assert!(serialize(&file).is_err());
    }

    #[test]
    fn rejects_decimal_comma() {
        let line = "S\t1\t15\t0,4\t11\t1\tS\t1\t15\t0\t11\t1\tS\t1\t15\t0\t11\t1\tS\t1\t15\t0\t11\t1";
        match &parse(line).lines[0] {
            CommonDropLine::Invalid { error, .. } => assert!(error.contains("Komma") && error.contains("Block 1"), "{error}"),
            other => panic!("expected invalid, got {other:?}"),
        }
    }

    #[test]
    fn allows_empty_name_and_trailing_tab() {
        let line = "\t1\t15\t4\t11\t1\t\t1\t15\t0\t11\t1\t\t1\t15\t0\t11\t1\t\t1\t15\t0\t11\t1\t";
        let file = parse(line);
        let rules = rules(&file);
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0][0].name, "");
    }

    #[test]
    fn serializer_strips_tabs_from_names_and_comments() {
        let mut file = parse(SAMPLE);
        if let CommonDropLine::Rule { blocks } = &mut file.lines[3] {
            blocks[0].name = "Seelen\tstein".into();
        }
        file.lines.push(CommonDropLine::Text { text: "#\tnotiz".into() });
        let out = serialize(&file).unwrap();
        let reparsed = parse(&out);
        assert_eq!(rules(&reparsed).len(), 3);
        assert!(out.contains("Seelen stein\t1\t75"));
        assert!(out.ends_with("# notiz\n"));
    }

    #[test]
    fn chance_is_written_without_exponent() {
        assert_eq!(fmt_chance(4.0), "4");
        assert_eq!(fmt_chance(0.104), "0.104");
        assert_eq!(fmt_chance(0.0000004), "0.0000004");
        assert_eq!(fmt_chance(0.00004), "0.00004");
    }

    fn block(level_from: i64, level_to: i64, chance: f64, vnum: u32) -> CommonDropBlock {
        CommonDropBlock { name: "x".into(), level_from, level_to, chance, vnum, count: 1 }
    }

    fn rule(blocks: [CommonDropBlock; 4]) -> CommonDropFile {
        CommonDropFile { lines: vec![CommonDropLine::Rule { blocks: blocks.to_vec() }], crlf: false }
    }

    fn errors(file: &CommonDropFile) -> Vec<String> {
        validate(file).into_iter().filter(|i| i.level == IssueLevel::Error).map(|i| i.message).collect()
    }

    #[test]
    fn sample_has_no_errors_but_warns_about_level_ranges() {
        let issues = validate(&parse(SAMPLE));
        assert!(issues.iter().all(|i| i.level == IssueLevel::Warning), "{issues:?}");
        // Seelenstein: alles aus + Level-Bereiche, Verzaubern: Level-Bereiche
        assert!(issues.iter().any(|i| i.line_index == 3 && i.message.contains("dropt nie")));
        assert!(issues.iter().any(|i| i.line_index == 4 && i.message.contains("PAWN 1–75")));
        assert!(!issues.iter().any(|i| i.line_index == 6));
    }

    #[test]
    fn forbidden_vnums_are_errors_even_in_inactive_blocks() {
        let f = rule([block(1, 99, 4.0, 11), block(1, 99, 0.0, 14), block(1, 99, 0.0, 11), block(0, 0, 0.0, 11)]);
        let errs = errors(&f);
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].starts_with("S_PAWN") && errs[0].contains("Goldringe"));
    }

    #[test]
    fn level_and_chance_errors() {
        let f = rule([block(50, 0, 4.0, 11), block(50, 40, 4.0, 11), block(1, 99, 1000.0, 11), block(1, 99, -1.0, 11)]);
        let errs = errors(&f);
        assert!(errs.iter().any(|e| e.starts_with("PAWN") && e.contains("dropt dann nie")), "{errs:?}");
        assert!(errs.iter().any(|e| e.starts_with("S_PAWN") && e.contains("kleiner")), "{errs:?}");
        assert!(errs.iter().any(|e| e.starts_with("KNIGHT") && e.contains("mehr als 400")), "{errs:?}");
        assert!(errs.iter().any(|e| e.starts_with("S_KNIGHT") && e.contains("negativ")), "{errs:?}");
    }

    #[test]
    fn inactive_block_may_have_level_to_zero() {
        let f = rule([block(1, 99, 4.0, 11), block(0, 0, 0.0, 11), block(1, 99, 4.0, 11), block(1, 99, 4.0, 11)]);
        assert!(errors(&f).is_empty());
        assert!(validate(&f).is_empty(), "{:?}", validate(&f));
    }
}
