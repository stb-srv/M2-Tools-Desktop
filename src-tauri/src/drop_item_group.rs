use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// drop_item_group.txt - per-mob item pools where EVERY entry rolls
// independently (source/game/src/item_manager_read_tables.cpp::ReadDropItemGroup,
// verified this session against the real server source - this is a
// DIFFERENT function from `ReadMonsterDropItemGroup`, which is what
// mob_drop_item.txt actually uses, despite the similar name).
//
//   Wildhund_Extra
//   {
//   	Vnum	101
//   	Mob	101
//   	1	4006	0.500000	1
//   	2	46	0.300000
//   }
//
// Same brace-delimited `GroupName { ... }` grammar family as
// special_item_group.txt/cube.txt, but with real differences:
//   - `Vnum` and `Mob` are both mandatory header tokens. `Vnum` is read but
//     NEVER used anywhere else in the loader or its consumer (`CreateDropItem`)
//     - a vestigial identifier field, like `refine_proto.src_vnum`. This
//       editor doesn't expose it; `serialize` always writes `Vnum` equal to
//       `mob_vnum`.
//   - `Mob` is the real link to a monster - `ITEM_MANAGER` keys groups by
//     mob vnum directly (`std::map<DWORD, CDropItemGroup*>`), not through
//     mob_proto or any other file.
//   - Entry column order is `<idx> <item> <percent> <count(optional,
//     default 1)>` - PERCENT BEFORE COUNT, the opposite of
//     special_item_group.txt's `<idx> <item> <count> <prob> [<rare_pct>]`.
//   - Item lookup accepts a name OR a numeric vnum (name first via
//     `GetVnumByOriginalName`, falls back to `str_to_number` + `GetTable`) -
//     this editor always writes a plain vnum string, which the fallback
//     always accepts.
//   - THE BIG SEMANTIC DIFFERENCE from special_item_group.txt: this is NOT
//     a mutually-exclusive weighted pick from a pool. Every entry in a group
//     is rolled INDEPENDENTLY (`item_manager.cpp:860-890`) - multiple items
//     from the same group can drop from a single kill, and none dropping at
//     all is equally possible. It behaves like N independent per-item drop
//     chances, not a single pooled roll.
//   - Two blocks with the SAME `Mob` vnum are not an error - the server
//     merges their entries into a single group (`m_map_pkDropItemGroup.find`
//     reuses the existing group object rather than inserting a second one).
//     `parse` mirrors this exactly; the editor UI is expected to prevent
//     creating a second group with an already-used mob vnum in the first
//     place, so this merge path only matters for reading a pre-existing file.
//
// Percent encoding identical to the other drop files: stored as
// `percent * 10000`, real chance at baseline ≈ file value ÷ 4.
//
// No `/reload` case in `cmd_gm.cpp`'s `do_reload` - boot-time only.
//
// NOT byte-verified against a real live file (no sample was available this
// session) - LF line endings assumed as the closer analogue to
// mob_drop_item.txt/cube.txt (flat keyword-per-line-family formats), unlike
// special_item_group.txt's confirmed CRLF. Confirm on first live use.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropGroupItem {
    /// Item vnum as a decimal string (this editor never writes an item
    /// name here, even though the server would also accept one).
    pub item_ref: String,
    pub percent: f64,
    pub count: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropItemGroup {
    pub name: String,
    pub mob_vnum: u32,
    pub items: Vec<DropGroupItem>,
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

/// Deliberately strict, matching this project's other line-oriented server
/// file editors: any line that doesn't match the expected structure fails
/// the whole parse with a line number. Two blocks sharing the same `Mob`
/// vnum are merged (their item lists concatenated in encounter order),
/// mirroring the real server's own `ReadDropItemGroup` behavior exactly
/// rather than treating it as an error.
pub fn parse(content: &str) -> Result<Vec<DropItemGroup>, String> {
    let mut groups: Vec<DropItemGroup> = Vec::new();
    let mut index_by_mob: HashMap<u32, usize> = HashMap::new();
    let mut lines: Lines = content.lines().enumerate().peekable();

    while let Some((line_no, line)) = next_nonblank(&mut lines) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 1 {
            return Err(format!("Zeile {}: '<Gruppenname>' erwartet, gefunden: {line:?}", line_no + 1));
        }
        let name = tokens[0].to_string();

        let (line_no, line) =
            next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende nach '{name}'"))?;
        if line.trim() != "{" {
            return Err(format!("Zeile {}: '{{' erwartet, gefunden: {line:?}", line_no + 1));
        }

        let (line_no, line) =
            next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende in Gruppe '{name}'"))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Vnum" {
            return Err(format!("Zeile {}: 'Vnum <id>' erwartet, gefunden: {line:?}", line_no + 1));
        }
        // Vnum's actual value is never used by the server, see module doc -
        // parsed only to validate the line shape, then discarded.
        tokens[1]
            .parse::<i32>()
            .map_err(|_| format!("Zeile {}: ungültige Vnum '{}'", line_no + 1, tokens[1]))?;

        let (line_no, line) =
            next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende in Gruppe '{name}'"))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Mob" {
            return Err(format!("Zeile {}: 'Mob <vnum>' erwartet, gefunden: {line:?}", line_no + 1));
        }
        let mob_vnum: u32 = tokens[1]
            .parse()
            .map_err(|_| format!("Zeile {}: ungültige Mob-Vnum '{}'", line_no + 1, tokens[1]))?;

        let mut items = Vec::new();
        loop {
            let (line_no, line) =
                next_nonblank(&mut lines).ok_or_else(|| format!("Unerwartetes Dateiende in Gruppe '{name}'"))?;
            if line.trim() == "}" {
                break;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() != 3 && tokens.len() != 4 {
                return Err(format!(
                    "Zeile {}: Eintrag erwartet ('<Nr> <Item> <Prozent> [<Anzahl>]'), gefunden: {line:?}",
                    line_no + 1
                ));
            }
            tokens[0]
                .parse::<u32>()
                .map_err(|_| format!("Zeile {}: ungültige laufende Nummer '{}'", line_no + 1, tokens[0]))?;

            let item_ref = tokens[1].to_string();
            if item_ref.parse::<u32>().is_err() {
                return Err(format!(
                    "Zeile {}: '{}' ist keine Item-VNUM (dieser Editor schreibt/erwartet ausschließlich Zahlen, keine Item-Namen).",
                    line_no + 1,
                    item_ref
                ));
            }
            let percent: f64 = tokens[2]
                .parse()
                .map_err(|_| format!("Zeile {}: ungültiger Prozentwert '{}'.", line_no + 1, tokens[2]))?;
            let count: i32 = if tokens.len() == 4 {
                tokens[3]
                    .parse()
                    .map_err(|_| format!("Zeile {}: ungültige Anzahl '{}'.", line_no + 1, tokens[3]))?
            } else {
                1
            };

            items.push(DropGroupItem { item_ref, percent, count });
        }

        if let Some(&existing_idx) = index_by_mob.get(&mob_vnum) {
            groups[existing_idx].items.extend(items);
        } else {
            index_by_mob.insert(mob_vnum, groups.len());
            groups.push(DropItemGroup { name, mob_vnum, items });
        }
    }

    Ok(groups)
}

/// Renders back to the brace-delimited format, always regenerating a
/// contiguous 1..N index per group and writing `Vnum` equal to `mob_vnum`
/// (see module doc - the server never reads the real value back).
pub fn serialize(groups: &[DropItemGroup]) -> String {
    let mut out = String::new();
    for (gi, group) in groups.iter().enumerate() {
        if gi > 0 {
            out.push('\n');
        }
        out.push_str(&format!("{}\n{{\n\tVnum\t{}\n\tMob\t{}\n", group.name, group.mob_vnum, group.mob_vnum));
        for (i, item) in group.items.iter().enumerate() {
            out.push_str(&format!("\t{}\t{}\t{:.6}\t{}\n", i + 1, item.item_ref, item.percent, item.count));
        }
        out.push_str("}\n");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Wildhund_Extra\n{\n\tVnum\t101\n\tMob\t101\n\t1\t4006\t0.500000\t1\n\t2\t46\t0.300000\t2\n}\n\nBaer_Extra\n{\n\tVnum\t102\n\tMob\t102\n\t1\t46\t1.000000\t1\n}\n";

    #[test]
    fn parses_real_grammar() {
        let groups = parse(SAMPLE).expect("should parse");
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].name, "Wildhund_Extra");
        assert_eq!(groups[0].mob_vnum, 101);
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(groups[0].items[0], DropGroupItem { item_ref: "4006".into(), percent: 0.5, count: 1 });
        assert_eq!(groups[1].mob_vnum, 102);
    }

    #[test]
    fn round_trips_exactly() {
        let groups = parse(SAMPLE).unwrap();
        let rendered = serialize(&groups);
        assert_eq!(rendered, SAMPLE);
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(groups, reparsed);
    }

    #[test]
    fn count_defaults_to_one_when_omitted() {
        let content = "G\n{\n\tVnum\t1\n\tMob\t1\n\t1\t5\t10.000000\n}\n";
        let groups = parse(content).unwrap();
        assert_eq!(groups[0].items[0].count, 1);
    }

    #[test]
    fn merges_duplicate_mob_vnum_blocks_like_the_real_server() {
        let content = "First\n{\n\tVnum\t1\n\tMob\t500\n\t1\t10\t1.000000\t1\n}\n\nSecond\n{\n\tVnum\t2\n\tMob\t500\n\t1\t20\t2.000000\t1\n}\n";
        let groups = parse(content).unwrap();
        assert_eq!(groups.len(), 1, "duplicate mob vnum must merge into one group, not create two");
        assert_eq!(groups[0].name, "First", "the first block's name wins, matching the server keeping the first-created group object");
        assert_eq!(groups[0].items.len(), 2);
        assert_eq!(groups[0].items[1].item_ref, "20");
    }

    #[test]
    fn rejects_item_name_instead_of_vnum() {
        let content = "G\n{\n\tVnum\t1\n\tMob\t1\n\t1\tSomeItemName\t1.000000\t1\n}\n";
        let err = parse(content).unwrap_err();
        assert!(err.contains("keine Item-VNUM"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_missing_mob_line() {
        let content = "G\n{\n\tVnum\t1\n\t1\t5\t1.000000\t1\n}\n";
        assert!(parse(content).is_err());
    }

    #[test]
    fn rejects_unclosed_group() {
        assert!(parse("G\n{\n\tVnum\t1\n\tMob\t1\n").is_err());
    }

    #[test]
    fn renumbers_after_item_removed() {
        let mut groups = parse(SAMPLE).unwrap();
        groups[0].items.remove(0);
        let rendered = serialize(&groups);
        assert!(rendered.contains("\t1\t46\t0.300000\t2\n"));
    }
}
