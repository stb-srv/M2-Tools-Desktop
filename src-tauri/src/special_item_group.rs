use serde::{Deserialize, Serialize};

// special_item_group.txt format - the loot table a GIFTBOX-type item
// (item_proto.type = 23) draws from when opened (source/game/src/
// item_manager_read_tables.cpp::ReadSpecialDropItemFile,
// char_item.cpp's `case ITEM_GIFTBOX`). Byte-verified against the real
// server file (2026-08-06, `od -c`), which differs from mob_drop_item.txt
// in two ways this parser/serializer must respect exactly:
//   - CRLF line endings throughout (mob_drop_item.txt uses plain LF).
//   - The `Group <name>` header line is SPACE-separated, not tab-separated
//     (every other line - `Vnum`, `Type`, entries - IS tab-separated, same
//     as mob_drop_item.txt).
//
//   Group Waffentruhe
//   {
//   	Vnum	50082
//   	1	19	1	1
//   	2	29	1	1
//   	}
//
//   Group Waffentruhe1
//   {
//   	Vnum	50081
//   	Type	pct
//   	1	27994	100	1
//   	}
//
// Entry columns: <index> <item-ref> <count> <prob> [<rare_pct>]. `item-ref`
// is either a plain item vnum, or one of a fixed set of special keywords
// the server recognizes instead of a real item
// (`ITEM_MANAGER::ReadSpecialDropItemFile`): gold, exp, mob, slow,
// drain_hp, poison, bleeding (ENABLE_WOLFMAN only), group. Kept as a plain
// string end-to-end (not parsed into an enum) - the frontend decides
// whether to show an item picker or a keyword dropdown based on whether it
// parses as a number, and this project's other editors already prefer
// "never guess, preserve exactly what's there" over modeling every case.
//
// `Type <kind>` is optional (defaults to the server's own "normal drop"
// behavior when absent - true for every real group in the user's live
// file). `attr`-type groups are a *completely different* schema
// (apply_type/apply_value pairs + an effect filename, no items/counts at
// all - `CSpecialAttrGroup`, not `CSpecialItemGroup`) and are explicitly
// refused rather than silently mismodeled - none exist in the real file
// this was verified against, and supporting them would need a separate
// editor UI entirely.
const SPECIAL_KEYWORDS: &[&str] = &["gold", "exp", "mob", "slow", "drain_hp", "poison", "bleeding", "group"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecialItemGroupEntry {
    /// Item vnum as a decimal string, or one of `SPECIAL_KEYWORDS`.
    pub item_ref: String,
    pub count: i32,
    pub prob: i32,
    pub rare_pct: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpecialItemGroup {
    pub name: String,
    /// The GIFTBOX item's own vnum - `item_proto.type = 23` for this vnum
    /// is what makes the client/server treat opening it as drawing from
    /// this group in the first place.
    pub vnum: i32,
    /// `None` = normal drop behavior (the overwhelming common case - every
    /// group in the real file omits this). `Some("attr")` is rejected by
    /// `parse` before it ever reaches this struct.
    pub group_type: Option<String>,
    pub entries: Vec<SpecialItemGroupEntry>,
}

pub fn is_special_keyword(s: &str) -> bool {
    SPECIAL_KEYWORDS.contains(&s)
}

type Lines<'a> = std::iter::Peekable<std::iter::Enumerate<std::str::Lines<'a>>>;

fn next_nonblank<'a>(lines: &mut Lines<'a>) -> Option<(usize, &'a str)> {
    loop {
        let (i, l) = lines.next()?;
        if !l.trim().is_empty() {
            return Some((i, l));
        }
    }
}

/// Parses the full file. Deliberately strict, matching this project's
/// other line-oriented server file editors (Mob Drop Editor, Regen-Datei-
/// Editor): any line that doesn't match the expected structure fails the
/// whole parse with a line number rather than silently dropping or
/// guessing - the file gets overwritten wholesale on save, so a
/// partial/wrong parse would destroy data. `str::lines()` already strips a
/// trailing `\r` per line on its own, so CRLF input needs no special
/// handling here (only `serialize` has to actively re-add it).
pub fn parse(content: &str) -> Result<Vec<SpecialItemGroup>, String> {
    let mut groups = Vec::new();
    let mut lines: Lines = content.lines().enumerate().peekable();

    while let Some((line_no, line)) = next_nonblank(&mut lines) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Group" {
            return Err(format!("Zeile {}: 'Group <name>' erwartet, gefunden: {line:?}", line_no + 1));
        }
        let name = tokens[1].to_string();

        let (line_no, line) =
            next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende nach 'Group {name}'"))?;
        if line.trim() != "{" {
            return Err(format!("Zeile {}: '{{' erwartet, gefunden: {line:?}", line_no + 1));
        }

        let (line_no, line) =
            next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende in Group '{name}'"))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Vnum" {
            return Err(format!("Zeile {}: 'Vnum <vnum>' erwartet, gefunden: {line:?}", line_no + 1));
        }
        let vnum: i32 = tokens[1]
            .parse()
            .map_err(|_| format!("Zeile {}: ungültige Vnum '{}'", line_no + 1, tokens[1]))?;

        // Optional 'Type <kind>' line - peek without consuming so a
        // group with no Type line falls straight through to entries.
        let mut group_type = None;
        if let Some(&(_, peeked)) = lines.peek() {
            let peeked_tokens: Vec<&str> = peeked.split_whitespace().collect();
            if peeked_tokens.first() == Some(&"Type") {
                let (line_no, line) = lines.next().unwrap();
                if peeked_tokens.len() != 2 {
                    return Err(format!("Zeile {}: 'Type <art>' erwartet, gefunden: {line:?}", line_no + 1));
                }
                let kind = peeked_tokens[1].to_lowercase();
                if kind == "attr" {
                    return Err(format!(
                        "Zeile {}: Group '{name}' hat Type 'attr' - dieser Gruppentyp (Attribut-Vergabe statt Item-Drops) wird noch nicht unterstützt.",
                        line_no + 1
                    ));
                }
                group_type = Some(kind);
            }
        }

        let mut entries = Vec::new();
        loop {
            let (line_no, line) =
                next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende in Group '{name}'"))?;
            if line.trim() == "}" {
                break;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() != 4 && tokens.len() != 5 {
                return Err(format!(
                    "Zeile {}: Eintrag erwartet ('<Nr> <Item/Schlüsselwort> <Anzahl> <Gewicht> [<Selten-%>]'), gefunden: {line:?}",
                    line_no + 1
                ));
            }
            // tokens[0] is the sequential index - must be numeric (that's
            // what distinguishes a real entry line from garbage) but its
            // value is never trusted; serialize() always recomputes it.
            tokens[0]
                .parse::<u32>()
                .map_err(|_| format!("Zeile {}: ungültige laufende Nummer '{}'", line_no + 1, tokens[0]))?;

            let item_ref = tokens[1].to_string();
            if !is_special_keyword(&item_ref) && item_ref.parse::<u32>().is_err() {
                return Err(format!(
                    "Zeile {}: '{}' ist weder eine Item-VNUM noch ein bekanntes Schlüsselwort ({})",
                    line_no + 1,
                    item_ref,
                    SPECIAL_KEYWORDS.join(", ")
                ));
            }
            let count: i32 = tokens[2]
                .parse()
                .map_err(|_| format!("Zeile {}: ungültige Anzahl '{}'", line_no + 1, tokens[2]))?;
            let prob: i32 = tokens[3]
                .parse()
                .map_err(|_| format!("Zeile {}: ungültiges Gewicht '{}'", line_no + 1, tokens[3]))?;
            let rare_pct = if tokens.len() == 5 {
                Some(
                    tokens[4]
                        .parse::<i32>()
                        .map_err(|_| format!("Zeile {}: ungültiger Selten-Wert '{}'", line_no + 1, tokens[4]))?,
                )
            } else {
                None
            };

            entries.push(SpecialItemGroupEntry { item_ref, count, prob, rare_pct });
        }

        groups.push(SpecialItemGroup { name, vnum, group_type, entries });
    }

    Ok(groups)
}

/// Renders the groups back to the exact CRLF, tab-indented format the
/// real server file uses, always regenerating a contiguous 1..N index per
/// group. The `Group <name>` header stays space-separated (matching the
/// live file byte-for-byte), everything else tab-separated.
pub fn serialize(groups: &[SpecialItemGroup]) -> String {
    let mut out = String::new();
    for (gi, group) in groups.iter().enumerate() {
        if gi > 0 {
            out.push_str("\r\n");
        }
        out.push_str(&format!("Group {}\r\n{{\r\n\tVnum\t{}\r\n", group.name, group.vnum));
        if let Some(kind) = &group.group_type {
            out.push_str(&format!("\tType\t{kind}\r\n"));
        }
        for (i, entry) in group.entries.iter().enumerate() {
            out.push_str(&format!("\t{}\t{}\t{}\t{}", i + 1, entry.item_ref, entry.count, entry.prob));
            if let Some(rare) = entry.rare_pct {
                out.push_str(&format!("\t{rare}"));
            }
            out.push_str("\r\n");
        }
        out.push_str("}\r\n");
    }
    out
}

/// Same sanitization rule as `mobdrop::sanitize_group_name` (no spaces, no
/// umlauts) - kept as a separate copy rather than a shared helper since
/// the two file formats are independent and this project's other editors
/// already follow the same "duplicate a small pure function rather than
/// add a cross-feature dependency" convention (see e.g. `ItemProtoInput`
/// being redeclared per-feature instead of imported).
pub fn sanitize_group_name(input: &str) -> String {
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
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // Real content fetched live from the user's dev server (2026-08-06),
    // byte-verified via `od -c` for exact whitespace/line-ending shape
    // before writing this parser - not a guessed fixture.
    const SAMPLE: &str = "Group Waffentruhe\r\n{\r\n\tVnum\t50082\r\n\t1\t19\t1\t1\r\n\t2\t29\t1\t1\r\n\t3\t39\t1\t1\r\n}\r\n\r\nGroup Waffentruhe1\r\n{\r\n\tVnum\t50081\r\n\t1\t27994\t100\t1\r\n\t2\t27994\t1000\t1\r\n\t3\t27994\t2000\t1\r\n}\r\n\r\nGroup test3\r\n{\r\n\tVnum\t83003\r\n\t1\tgold\t10\t1\r\n\t2\tgold\t100\t1\r\n\t3\tgold\t100\t1\r\n\t4\texp\t10\t1\r\n\t5\texp\t100\t1\r\n}\r\n";

    #[test]
    fn parses_real_sample() {
        let groups = parse(SAMPLE).expect("should parse the real file content");
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "Waffentruhe");
        assert_eq!(groups[0].vnum, 50082);
        assert_eq!(groups[0].group_type, None);
        assert_eq!(groups[0].entries.len(), 3);
        assert_eq!(
            groups[0].entries[0],
            SpecialItemGroupEntry { item_ref: "19".into(), count: 1, prob: 1, rare_pct: None }
        );

        // Group with special keywords instead of item vnums.
        assert_eq!(groups[2].name, "test3");
        assert_eq!(groups[2].entries[0].item_ref, "gold");
        assert_eq!(groups[2].entries[3].item_ref, "exp");
    }

    #[test]
    fn round_trips_exactly() {
        let groups = parse(SAMPLE).unwrap();
        let rendered = serialize(&groups);
        assert_eq!(rendered, SAMPLE, "must reproduce the real file byte-for-byte");
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(groups, reparsed);
    }

    #[test]
    fn parses_optional_type_line_and_rare_pct_column() {
        let content = "Group Pct_Kiste\r\n{\r\n\tVnum\t50090\r\n\tType\tpct\r\n\t1\t100\t5\t1\t50\r\n}\r\n";
        let groups = parse(content).unwrap();
        assert_eq!(groups[0].group_type, Some("pct".to_string()));
        assert_eq!(groups[0].entries[0].rare_pct, Some(50));
        assert_eq!(serialize(&groups), content);
    }

    #[test]
    fn rejects_attr_type_group_instead_of_mismodeling_it() {
        let content = "Group Attr_Gruppe\r\n{\r\n\tVnum\t50090\r\n\tType\tattr\r\n\t1\t5\t100\r\n}\r\n";
        let err = parse(content).unwrap_err();
        assert!(err.contains("attr"), "error should explain the unsupported type: {err}");
    }

    #[test]
    fn rejects_unknown_item_ref_instead_of_guessing() {
        let content = "Group Broken\r\n{\r\n\tVnum\t1\r\n\t1\tnotanumber\t1\t1\r\n}\r\n";
        assert!(parse(content).is_err());
    }

    #[test]
    fn renumbers_after_entry_removed() {
        let mut groups = parse(SAMPLE).unwrap();
        groups[0].entries.remove(1);
        let rendered = serialize(&groups);
        assert!(rendered.contains("\t1\t19\t1\t1\r\n\t2\t39\t1\t1\r\n"));
    }

    #[test]
    fn sanitizes_group_names() {
        assert_eq!(sanitize_group_name("Waffentruhe für Krieger"), "Waffentruhe_fuer_Krieger");
    }
}
