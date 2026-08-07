use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// `locale/<lang>/itemdesc.txt` - the client's tooltip text for an item
// (`CItemManager::LoadItemDesc` in the real client source, `GameLib/
// ItemManager.cpp`). Purely cosmetic client-side text, no `item_proto`
// column at all and no server involvement whatsoever - this is why "where
// do I edit an item's description" wasn't findable anywhere else in this
// app. Byte-verified directly against the user's real client install
// (`locale/de/itemdesc.txt`, 2026-08-07, `od -c`/`awk`), not guessed:
//   - Plain LF line endings (unlike `item_list.txt` in the same folder,
//     which is CRLF - don't assume the two share a convention).
//   - Windows-1252 encoding (same as `item_list.txt`).
//   - Tab-separated: `<vnum>\t<description>\t<summary>`, both text fields
//     routinely empty (e.g. every "Eistalisman+170"-style row in the real
//     file has an empty summary). No blank lines, no duplicate vnums, file
//     ends with a trailing `\n` after the last row.
//   - A small number of real rows (24 of 6065) carry an *undocumented 4th
//     tab-separated column* (seen values like "Forschung") that the client
//     loader never reads (`ITEMDESC_COL_NUM` in the client source is 3) -
//     kept as an opaque `extra` field so editing an unrelated row can't
//     silently drop it on save.
//
// Deliberately targets only the *first* `locale/<lang>/` folder that has an
// `itemdesc.txt` under the client path, unlike `packtools::
// upsert_item_list_entries` which intentionally writes the same row to
// every locale (icon/model paths are locale-independent; description text
// is not, and this server only ships one text locale - see
// [[m2manager_item_editor]] for why this wasn't generalized further).

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDescEntry {
    pub vnum: u32,
    pub description: String,
    pub summary: String,
    pub extra: Option<String>,
}

pub fn parse(content: &str) -> Result<Vec<ItemDescEntry>, String> {
    let mut entries = Vec::new();
    for (line_no, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 3 {
            return Err(format!(
                "Zeile {}: '<vnum>\\t<Beschreibung>\\t<Kurzbeschreibung>' erwartet, gefunden: {line:?}",
                line_no + 1
            ));
        }
        let vnum: u32 = parts[0]
            .parse()
            .map_err(|_| format!("Zeile {}: ungültige VNUM '{}'", line_no + 1, parts[0]))?;
        let extra = if parts.len() > 3 { Some(parts[3..].join("\t")) } else { None };
        entries.push(ItemDescEntry {
            vnum,
            description: parts[1].to_string(),
            summary: parts[2].to_string(),
            extra,
        });
    }
    Ok(entries)
}

pub fn serialize(entries: &[ItemDescEntry]) -> String {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&format!("{}\t{}\t{}", entry.vnum, entry.description, entry.summary));
        if let Some(extra) = &entry.extra {
            out.push('\t');
            out.push_str(extra);
        }
        out.push('\n');
    }
    out
}

/// First `locale/<lang>/itemdesc.txt` found under the client path (see
/// module doc for why "first" instead of "every locale").
pub fn find_file(client_path: &str) -> Result<PathBuf, String> {
    let locale_dir = Path::new(client_path).join("locale");
    let mut subdirs: Vec<PathBuf> = std::fs::read_dir(&locale_dir)
        .map_err(|e| format!("Locale-Ordner {locale_dir:?} nicht lesbar: {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    subdirs.sort();

    subdirs
        .into_iter()
        .map(|dir| dir.join("itemdesc.txt"))
        .find(|p| p.exists())
        .ok_or_else(|| format!("Keine itemdesc.txt unter {locale_dir:?}/<lang>/ gefunden."))
}

pub fn read_entry(client_path: &str, vnum: u32) -> Result<Option<ItemDescEntry>, String> {
    let path = find_file(client_path)?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    let entries = parse(&text)?;
    Ok(entries.into_iter().find(|e| e.vnum == vnum))
}

/// Upserts `vnum`'s row (adds it if absent) and writes the file back,
/// preserving every other row's `extra` column exactly.
pub fn write_entry(client_path: &str, vnum: u32, description: &str, summary: &str) -> Result<(), String> {
    let path = find_file(client_path)?;
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let (text, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    let mut entries = parse(&text)?;

    match entries.iter_mut().find(|e| e.vnum == vnum) {
        Some(existing) => {
            existing.description = description.to_string();
            existing.summary = summary.to_string();
        }
        None => entries.push(ItemDescEntry {
            vnum,
            description: description.to_string(),
            summary: summary.to_string(),
            extra: None,
        }),
    }

    crate::packtools::backup_file(&path)?;
    let out = serialize(&entries);
    let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(&out);
    std::fs::write(&path, encoded).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Excerpt fetched live from the user's real client install
    // (locale/de/itemdesc.txt, 2026-08-07) - not a guessed fixture. Mixes a
    // normal 3-column row, an empty-summary row, and one of the real
    // 4-column rows.
    const SAMPLE: &str = "100001\tElixier der Zeit (M)\tFüllt die Laufzeit eines Drachensteins auf.\n10000\tEistalisman+170\t\n50701\tPfirsichblüte\tFrüher wurden Pfirsichblumen verwendet.\tForschung\n";

    #[test]
    fn parses_real_sample() {
        let entries = parse(SAMPLE).unwrap();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].vnum, 100001);
        assert_eq!(entries[0].summary, "Füllt die Laufzeit eines Drachensteins auf.");
        assert_eq!(entries[1].vnum, 10000);
        assert_eq!(entries[1].summary, "");
        assert_eq!(entries[2].extra.as_deref(), Some("Forschung"));
    }

    #[test]
    fn round_trips_exactly() {
        let entries = parse(SAMPLE).unwrap();
        let rendered = serialize(&entries);
        assert_eq!(rendered, SAMPLE, "must reproduce the real file byte-for-byte");
    }

    #[test]
    fn rejects_line_with_no_tabs_instead_of_guessing() {
        assert!(parse("not a valid line at all").is_err());
    }

    #[test]
    fn read_write_entry_round_trip_preserves_other_rows_and_extra_column() {
        let dir = std::env::temp_dir().join(format!("m2m_itemdesc_test_{}", std::process::id()));
        let locale_dir = dir.join("locale").join("de");
        std::fs::create_dir_all(&locale_dir).unwrap();
        let (encoded, _, _) = encoding_rs::WINDOWS_1252.encode(SAMPLE);
        std::fs::write(locale_dir.join("itemdesc.txt"), encoded).unwrap();

        let client_path = dir.to_str().unwrap();
        write_entry(client_path, 10000, "Neuer Text", "Neue Kurzfassung").unwrap();

        let updated = read_entry(client_path, 10000).unwrap().unwrap();
        assert_eq!(updated.description, "Neuer Text");
        assert_eq!(updated.summary, "Neue Kurzfassung");

        // untouched rows, including the 4-column one, must survive as-is
        let untouched = read_entry(client_path, 50701).unwrap().unwrap();
        assert_eq!(untouched.extra.as_deref(), Some("Forschung"));

        write_entry(client_path, 999999, "Ganz neu", "").unwrap();
        let created = read_entry(client_path, 999999).unwrap().unwrap();
        assert_eq!(created.description, "Ganz neu");

        std::fs::remove_dir_all(&dir).ok();
    }
}
