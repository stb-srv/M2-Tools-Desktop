use serde::{Deserialize, Serialize};

// mob_drop_item.txt format (confirmed against the user's real server file):
//
//   Group	Wildhund
//   {
//   	Mob	101
//   	Type	drop
//   	1	4006	1	10
//   	2	46	1	10
//   	}
//
// One `Group <name> { ... }` block per mob, blank line between blocks. The
// leading number on each item line is a sequential index (1..N) - the user
// requires it to always be contiguous, so it is never trusted/stored here;
// `serialize` always regenerates it from array position.

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobDropItem {
    pub item_vnum: u32,
    pub count: u32,
    pub percent: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MobDropGroup {
    pub name: String,
    pub mob_vnum: u32,
    pub drop_type: String,
    pub items: Vec<MobDropItem>,
}

// The real server loader (`ReadMonsterDropItemGroup` /
// `item_manager_read_tables.cpp`) does not read item lines positionally -
// for k in 1..256 it looks up the *literal string key* "1", "2", ... and
// stops at the first missing key, silently discarding every remaining entry
// in that group. So a file where the leading index has a gap, a duplicate,
// or doesn't start at 1 doesn't fail to load - it silently loses drops on
// the live server. `parse()` above never trusts this number for its own
// data model (position always wins, see `serialize`), so this tool itself
// can't produce such a file - but it can load one that already has the
// problem (hand-edited, or from before this tool existed) and needs to
// surface that before the user assumes what they see is what the server
// sees. `group_index` matches the position of the group in `parse()`'s
// output, so the frontend can point back at `groups[group_index]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumberingIssue {
    pub group_index: usize,
    pub found: Vec<u32>,
}

/// Scans already-valid `mob_drop_item.txt` content for the numbering problem
/// described above. Deliberately independent of `parse()`'s own line-by-line
/// state machine (tracks braces instead of the Group/Mob/Type grammar) since
/// it only needs the leading index of each item line, not the fields around
/// it - kept simple on purpose rather than threading a second return value
/// through every step of `parse()`.
pub fn check_numbering(content: &str) -> Vec<NumberingIssue> {
    let mut issues = Vec::new();
    let mut group_index: Option<usize> = None;
    let mut next_group_index: usize = 0;
    let mut current: Vec<u32> = Vec::new();
    let mut in_group = false;

    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() == 2 && tokens[0] == "Group" {
            group_index = Some(next_group_index);
            next_group_index += 1;
            current.clear();
            in_group = false;
            continue;
        }
        if line == "{" {
            in_group = true;
            continue;
        }
        if line == "}" {
            if in_group {
                if let Some(gi) = group_index {
                    let mut sorted = current.clone();
                    sorted.sort_unstable();
                    let expected: Vec<u32> = (1..=current.len() as u32).collect();
                    if sorted != expected {
                        issues.push(NumberingIssue {
                            group_index: gi,
                            found: current.clone(),
                        });
                    }
                }
            }
            in_group = false;
            continue;
        }
        if in_group && tokens.len() == 4 {
            if let Ok(idx) = tokens[0].parse::<u32>() {
                current.push(idx);
            }
        }
    }

    issues
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

/// Parses the full file. Deliberately strict: any line that doesn't match
/// the expected structure fails the whole parse with a line number, rather
/// than silently dropping or guessing at content we don't understand - this
/// file gets overwritten wholesale on save, so a partial/wrong parse would
/// destroy data.
pub fn parse(content: &str) -> Result<Vec<MobDropGroup>, String> {
    let mut groups = Vec::new();
    let mut lines: Lines = content.lines().enumerate().peekable();

    while let Some((line_no, line)) = next_nonblank(&mut lines) {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Group" {
            return Err(format!(
                "Zeile {}: 'Group <name>' erwartet, gefunden: {line:?}",
                line_no + 1
            ));
        }
        let name = tokens[1].to_string();

        let (line_no, line) = next_nonblank(&mut lines)
            .ok_or_else(|| format!("Unerwartetes Dateiende nach 'Group {name}'"))?;
        if line.trim() != "{" {
            return Err(format!("Zeile {}: '{{' erwartet, gefunden: {line:?}", line_no + 1));
        }

        let (line_no, line) = next_nonblank(&mut lines)
            .ok_or_else(|| format!("Unerwartetes Dateiende in Group '{name}'"))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Mob" {
            return Err(format!(
                "Zeile {}: 'Mob <vnum>' erwartet, gefunden: {line:?}",
                line_no + 1
            ));
        }
        let mob_vnum: u32 = tokens[1]
            .parse()
            .map_err(|_| format!("Zeile {}: ungültige Mob-VNUM '{}'", line_no + 1, tokens[1]))?;

        let (line_no, line) = next_nonblank(&mut lines)
            .ok_or_else(|| format!("Unerwartetes Dateiende in Group '{name}'"))?;
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() != 2 || tokens[0] != "Type" {
            return Err(format!(
                "Zeile {}: 'Type <typ>' erwartet, gefunden: {line:?}",
                line_no + 1
            ));
        }
        let drop_type = tokens[1].to_string();

        let mut items = Vec::new();
        loop {
            let (line_no, line) = next_nonblank(&mut lines)
                .ok_or_else(|| format!("Unerwartetes Dateiende in Group '{name}'"))?;
            if line.trim() == "}" {
                break;
            }
            let tokens: Vec<&str> = line.split_whitespace().collect();
            if tokens.len() != 4 {
                return Err(format!(
                    "Zeile {}: Drop-Eintrag erwartet ('<Nr> <Item-VNUM> <Anzahl> <Prozent>'), gefunden: {line:?}",
                    line_no + 1
                ));
            }
            // tokens[0] is the sequential index - must be numeric (that's
            // what distinguishes a real entry line from garbage) but its
            // value is never trusted; serialize() always recomputes it.
            tokens[0].parse::<u32>().map_err(|_| {
                format!("Zeile {}: ungültige laufende Nummer '{}'", line_no + 1, tokens[0])
            })?;
            let item_vnum: u32 = tokens[1]
                .parse()
                .map_err(|_| format!("Zeile {}: ungültige Item-VNUM '{}'", line_no + 1, tokens[1]))?;
            let count: u32 = tokens[2]
                .parse()
                .map_err(|_| format!("Zeile {}: ungültige Anzahl '{}'", line_no + 1, tokens[2]))?;
            let percent: f64 = tokens[3].parse().map_err(|_| {
                format!("Zeile {}: ungültiger Prozentwert '{}'", line_no + 1, tokens[3])
            })?;
            items.push(MobDropItem {
                item_vnum,
                count,
                percent,
            });
        }

        groups.push(MobDropGroup {
            name,
            mob_vnum,
            drop_type,
            items,
        });
    }

    Ok(groups)
}

fn format_percent(value: f64) -> String {
    let rounded = (value * 10_000.0).round() / 10_000.0;
    if rounded.fract() == 0.0 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded:.4}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

/// Renders the groups back to the exact tab-indented format the server
/// expects, always regenerating a contiguous 1..N index per group.
pub fn serialize(groups: &[MobDropGroup]) -> String {
    let mut out = String::new();
    for (gi, group) in groups.iter().enumerate() {
        if gi > 0 {
            out.push('\n');
        }
        out.push_str(&format!(
            "Group\t{}\n{{\n\tMob\t{}\n\tType\t{}\n",
            group.name, group.mob_vnum, group.drop_type
        ));
        for (i, item) in group.items.iter().enumerate() {
            out.push_str(&format!(
                "\t{}\t{}\t{}\t{}\n",
                i + 1,
                item.item_vnum,
                item.count,
                format_percent(item.percent)
            ));
        }
        out.push_str("}\n");
    }
    out
}

/// Turns a free-typed display name into a valid `Group` identifier: no
/// spaces, no umlauts (per the user's explicit rule - "Stein der Willkür"
/// must become "Stein_der_Willkuer").
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

    const SAMPLE: &str = "Group\tHungriger_Wildhund\n{\n\tMob\t171\n\tType\tdrop\n\t1\t4006\t1\t10\n\t2\t2006\t1\t10\n\t3\t56\t1\t10\n\t4\t1006\t1\t10\n\t5\t7016\t1\t10\n}\n\nGroup\tWildhund\n{\n\tMob\t101\n\tType\tdrop\n\t1\t4006\t1\t10\n\t2\t46\t1\t10\n\t3\t2006\t1\t10\n\t4\t56\t1\t10\n\t5\t1006\t1\t10\n\t6\t3026\t1\t10\n\t7\t7016\t1\t10\n\t8\t3006\t1\t10\n}\n\nGroup\tHungriger_Wolf\n{\n\tMob\t172\n\tType\tdrop\n\t1\t4006\t1\t10\n\t2\t2006\t1\t10\n\t3\t1006\t1\t10\n\t4\t3026\t1\t10\n}\n";

    #[test]
    fn parses_real_sample() {
        let groups = parse(SAMPLE).expect("should parse the user's real file");
        assert_eq!(groups.len(), 3);
        assert_eq!(groups[0].name, "Hungriger_Wildhund");
        assert_eq!(groups[0].mob_vnum, 171);
        assert_eq!(groups[0].drop_type, "drop");
        assert_eq!(groups[0].items.len(), 5);
        assert_eq!(
            groups[0].items[0],
            MobDropItem {
                item_vnum: 4006,
                count: 1,
                percent: 10.0
            }
        );
        assert_eq!(groups[1].name, "Wildhund");
        assert_eq!(groups[1].items.len(), 8);
    }

    #[test]
    fn round_trips_exactly() {
        let groups = parse(SAMPLE).unwrap();
        let rendered = serialize(&groups);
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(groups, reparsed);
    }

    #[test]
    fn renumbers_after_item_removed() {
        let mut groups = parse(SAMPLE).unwrap();
        groups[0].items.remove(1); // drop the old "2" entry
        let rendered = serialize(&groups);
        assert!(rendered.contains("\t1\t4006\t1\t10\n\t2\t56\t1\t10\n"));
    }

    #[test]
    fn rejects_malformed_block_instead_of_dropping_it() {
        let broken = "Group\tWildhund\n{\n\tMob\t101\n\tType\tdrop\n\tnot_a_number\t4006\t1\t10\n}\n";
        assert!(parse(broken).is_err());
    }

    #[test]
    fn sanitizes_group_names() {
        assert_eq!(sanitize_group_name("Stein der Willkür"), "Stein_der_Willkuer");
        assert_eq!(sanitize_group_name("Hungriger Wildhund"), "Hungriger_Wildhund");
    }

    #[test]
    fn numbering_check_passes_the_real_sample() {
        assert_eq!(check_numbering(SAMPLE), Vec::new());
    }

    #[test]
    fn numbering_check_finds_a_gap() {
        // "3" is missing - on the real server this silently drops every
        // entry after it (see doc comment on `check_numbering`), even
        // though this parses fine and shows all 4 items in this tool.
        let broken = "Group\tWildhund\n{\n\tMob\t101\n\tType\tdrop\n\t1\t4006\t1\t10\n\t2\t46\t1\t10\n\t4\t56\t1\t10\n\t5\t1006\t1\t10\n}\n";
        assert!(parse(broken).is_ok());
        let issues = check_numbering(broken);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].group_index, 0);
        assert_eq!(issues[0].found, vec![1, 2, 4, 5]);
    }

    #[test]
    fn numbering_check_finds_a_duplicate() {
        let broken = "Group\tWildhund\n{\n\tMob\t101\n\tType\tdrop\n\t1\t4006\t1\t10\n\t1\t46\t1\t10\n}\n";
        let issues = check_numbering(broken);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].found, vec![1, 1]);
    }

    #[test]
    fn numbering_check_finds_a_wrong_start() {
        let broken = "Group\tWildhund\n{\n\tMob\t101\n\tType\tdrop\n\t2\t4006\t1\t10\n\t3\t46\t1\t10\n}\n";
        let issues = check_numbering(broken);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].found, vec![2, 3]);
    }

    #[test]
    fn numbering_check_reports_correct_group_index_among_several() {
        // Only the second group ("Wildhund") is broken.
        let content = "Group\tHungriger_Wildhund\n{\n\tMob\t171\n\tType\tdrop\n\t1\t4006\t1\t10\n}\n\nGroup\tWildhund\n{\n\tMob\t101\n\tType\tdrop\n\t1\t4006\t1\t10\n\t3\t46\t1\t10\n}\n";
        let issues = check_numbering(content);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].group_index, 1);
    }

    #[test]
    fn numbering_check_ignores_empty_groups() {
        let content = "Group\tLeer\n{\n\tMob\t1\n\tType\tdrop\n}\n";
        assert_eq!(check_numbering(content), Vec::new());
    }
}
