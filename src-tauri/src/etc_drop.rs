use serde::{Deserialize, Serialize};

// etc_drop_item.txt - a flat item->probability lookup table, keyed by item,
// NOT by mob (source/game/src/item_manager_read_tables.cpp::ReadEtcDropItemFile,
// verified this session against the real server source). Which mob uses
// which entry is decided entirely OUTSIDE this file, by each mob_proto's own
// "etc drop item" field (`GetMobDropItemVnum()` in item_manager.cpp) - this
// file only maps an item to its drop probability, nothing links it to a mob.
//
//   Grand_Necklace	0.010000
//   Small_Ring	0.500000
//
// One `<item_name><TAB><probability>` per line - the LAST tab on the line is
// the split point (`strrchr`, line 458). A line with prob 0.0, or an empty
// name, is silently skipped (matches the real server's own "unset" convention).
//
// CRITICAL, unlike every other drop file in this project: the item MUST be
// referenced by its internal `item_proto.name` (`GetVnumByOriginalName`,
// prefix-matched case-insensitively against `item_proto.name` - NOT
// `locale_name`) - there is NO numeric-vnum fallback here (unlike
// common_drop_item.txt/drop_item_group.txt, which both fall back to
// `str_to_number` if the name lookup fails). A name that doesn't resolve is
// a hard boot failure for the WHOLE server (`sys_err` + `return false`,
// line 471-476) - so this project resolves the real internal name via a
// live DB lookup (`commands::get_item_internal_name`/
// `find_item_by_internal_name`) rather than ever writing a raw vnum here.
//
// Percent encoding identical to the other drop files: stored as
// `probability * 10000`, real chance at baseline ≈ file value ÷ 4.
//
// No `/reload` case in `cmd_gm.cpp`'s `do_reload` - boot-time only.
//
// NOT byte-verified against a real live file (no sample was available this
// session, same caveat as `cube.rs`/`common_drop.rs`).

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EtcDropEntry {
    /// The item's internal `item_proto.name`, NOT its vnum or `locale_name`
    /// - see module doc for why.
    pub item_name: String,
    pub percent: f64,
}

/// Deliberately strict, matching this project's other line-oriented server
/// file editors. Unlike the real server's own parser (which silently skips
/// a line with prob 0.0 or an empty name instead of erroring), this refuses
/// such a line outright - a value that would just vanish on the server
/// should not be presented as "saved" in this editor.
pub fn parse(content: &str) -> Result<Vec<EtcDropEntry>, String> {
    let mut entries = Vec::new();

    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        if raw_line.trim().is_empty() {
            continue;
        }
        let Some(tab_idx) = raw_line.rfind('\t') else {
            return Err(format!("Zeile {line_no}: kein Tab-Zeichen gefunden, erwartet '<Item-Name>\\t<Prozent>'."));
        };
        let item_name = raw_line[..tab_idx].trim().to_string();
        let percent_str = raw_line[tab_idx + 1..].trim();

        if item_name.is_empty() {
            return Err(format!("Zeile {line_no}: leerer Item-Name."));
        }
        let percent: f64 = percent_str
            .parse()
            .map_err(|_| format!("Zeile {line_no}: ungültiger Prozentwert '{percent_str}'."))?;
        if percent == 0.0 {
            return Err(format!(
                "Zeile {line_no}: Prozentwert 0 - der Server würde diese Zeile beim Laden stillschweigend überspringen, wäre also nie wirksam."
            ));
        }

        entries.push(EtcDropEntry { item_name, percent });
    }

    Ok(entries)
}

pub fn serialize(entries: &[EtcDropEntry]) -> String {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&format!("{}\t{:.6}\n", entry.item_name, entry.percent));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let entries = vec![
            EtcDropEntry { item_name: "Grand_Necklace".into(), percent: 0.01 },
            EtcDropEntry { item_name: "Small_Ring".into(), percent: 0.5 },
        ];
        let rendered = serialize(&entries);
        let reparsed = parse(&rendered).unwrap();
        assert_eq!(entries, reparsed);
    }

    #[test]
    fn splits_on_the_last_tab() {
        let entries = parse("Grand_Necklace\t0.010000\n").unwrap();
        assert_eq!(entries[0].item_name, "Grand_Necklace");
        assert_eq!(entries[0].percent, 0.01);
    }

    #[test]
    fn rejects_zero_percent_instead_of_silently_dropping_it() {
        let err = parse("Small_Ring\t0.000000\n").unwrap_err();
        assert!(err.contains("stillschweigend"), "unexpected error: {err}");
    }

    #[test]
    fn rejects_missing_tab() {
        assert!(parse("Small_Ring 0.5\n").is_err());
    }

    #[test]
    fn ignores_blank_lines() {
        let entries = parse("\nGrand_Necklace\t0.010000\n\n").unwrap();
        assert_eq!(entries.len(), 1);
    }
}
