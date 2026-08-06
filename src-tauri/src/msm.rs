use std::path::Path;

/// Client "RaceDataScript" (`.msm`) files map an armor item's `value3`
/// (called `ShapeIndex` here) to a body `Model`/`SourceSkin`/`TargetSkin`
/// triple. This exists **only** for the female race trees
/// (`pc_<race>/ymir work/pc2/<race>/<race>.msm`) - verified 2026-08-06 by
/// grepping the whole client pack for any text file referencing a known
/// male body model filename (`warrior_nahan.gr2`) and finding only the
/// female `.msm` and its own `.gr2`/`.dds` siblings. The male race
/// (`pc/<race>/`) has no equivalent data file anywhere in the client, so
/// its body-armor selection is hardcoded client-side - **not** moddable by
/// dropping in new files, unlike everything else this importer handles.
/// This is why armor import only wires a 3D model for the female variant;
/// male stays icon+item-only, same as before this rewrite, just now with
/// an accurate explanation instead of "not yet investigated".
const RACES: &[&str] = &["warrior", "assassin", "sura", "shaman"];

fn msm_path(client_path: &str, race: &str) -> std::path::PathBuf {
    Path::new(client_path)
        .join("pack")
        .join(format!("pc_{race}"))
        .join("ymir work")
        .join("pc2")
        .join(race)
        .join(format!("{race}.msm"))
}

/// Byte offset range `[start, end)` of the outer `Group ShapeData { ... }`
/// block (the brace-matched body, `start` right after `Group ShapeData\n`,
/// `end` at the closing `}` inclusive) - the file has other unrelated
/// `Group ...` blocks (HairData, AttachingData, ...) after it, so this
/// can't just scan to EOF.
fn find_shapedata_block(text: &str) -> Result<(usize, usize), String> {
    let header = text
        .find("Group ShapeData\n")
        .or_else(|| text.find("Group ShapeData\r\n"))
        .ok_or("Kein 'Group ShapeData'-Block in dieser .msm-Datei gefunden.")?;
    let open = text[header..]
        .find('{')
        .map(|i| header + i)
        .ok_or("'Group ShapeData' ohne öffnende Klammer - Datei beschädigt?")?;
    let bytes = text.as_bytes();
    let mut depth = 0i32;
    let mut i = open;
    loop {
        if i >= bytes.len() {
            return Err("Kein passendes '}' zum ShapeData-Block gefunden - Datei beschädigt?".into());
        }
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((open, i + 1));
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// Highest `ShapeIndex` used anywhere in the file's `ShapeData` block -
/// used together with `max_value3_in_db` (see `commands.rs`) to pick a
/// collision-free new index, since the same `value3` must be free across
/// every race's own `.msm` *and* every existing `item_proto` row.
fn max_shape_index(client_path: &str) -> u32 {
    let mut max = 0u32;
    for race in RACES {
        let Ok(text) = std::fs::read_to_string(msm_path(client_path, race)) else {
            continue;
        };
        for cap in text.match_indices("ShapeIndex") {
            let rest = &text[cap.0 + "ShapeIndex".len()..];
            if let Some(n) = rest.trim_start().split_whitespace().next() {
                if let Ok(v) = n.parse::<u32>() {
                    max = max.max(v);
                }
            }
        }
    }
    max
}

pub fn next_free_shape_index(client_path: &str, max_db_value3: u32) -> u32 {
    max_shape_index(client_path).max(max_db_value3) + 1
}

/// Appends one `ShapeData` entry (new body model for `shape_index`) to
/// `race`'s female `.msm`, bumping `ShapeDataCount`. Backs up the file
/// first. `model_rel`/`skin_rel` are paths relative to the block's own
/// `PathName` (`d:/ymir work/pc2/<race>/`), matching where
/// `packtools::import_custom_armor_model` physically copies the files.
pub fn add_shape_data(
    client_path: &str,
    race: &str,
    shape_index: u32,
    model_rel: &str,
    skin_rel: &str,
) -> Result<(), String> {
    let path = msm_path(client_path, race);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{} nicht lesbar: {e}", path.display()))?;

    let (block_start, block_end) = find_shapedata_block(&text)?;
    let block = &text[block_start..block_end];

    // Locate the exact byte span of the count's digits (relative to
    // `block_start`) so it can be spliced precisely, rather than
    // string-replacing a reconstructed "ShapeDataCount<n>" token that could
    // subtly mismatch on whitespace.
    let count_label_rel = block
        .find("ShapeDataCount")
        .ok_or("ShapeDataCount nicht im ShapeData-Block gefunden.")?;
    let after_label = &block[count_label_rel + "ShapeDataCount".len()..];
    let ws_len = after_label.len() - after_label.trim_start().len();
    let digits = after_label
        .trim_start()
        .split_whitespace()
        .next()
        .ok_or("ShapeDataCount-Wert nicht lesbar.")?;
    let existing_count: u32 = digits.parse().map_err(|_| "ShapeDataCount-Wert nicht lesbar.")?;
    let digits_start = block_start + count_label_rel + "ShapeDataCount".len() + ws_len;
    let digits_end = digits_start + digits.len();

    let next_group_num = (0..)
        .find(|n| !block.contains(&format!("ShapeData{n:02}")))
        .unwrap_or(existing_count);

    let new_entry = format!(
        "\tGroup ShapeData{next_group_num:02}\n\t{{\n\t\tShapeIndex\t\t\t{shape_index}\n\n\t\tModel\t\t\t\t\"{model_rel}\"\n\t\tSourceSkin\t\t\t\"{skin_rel}\"\n\t\tTargetSkin\t\t\t\"{skin_rel}\"\n\t}}\n"
    );

    // Splice back-to-front (entry insertion point comes after the count
    // digits) so earlier offsets stay valid across both edits.
    let mut new_text = String::with_capacity(text.len() + new_entry.len() + 8);
    new_text.push_str(&text[..block_end - 1]);
    new_text.push_str(&new_entry);
    new_text.push_str(&text[block_end - 1..]);
    new_text.replace_range(digits_start..digits_end, &(existing_count + 1).to_string());

    crate::packtools::backup_file(&path)?;
    std::fs::write(&path, new_text).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "Header stuff\n\nGroup ShapeData\n{\n\tPathName\t\"d:/ymir Work/pc2/warrior/\"\n\n\tShapeDataCount\t\t\t2\n\tGroup ShapeData00\n\t{\n\t\tShapeIndex\t\t\t0\n\n\t\tModel\t\t\t\t\"warrior_novice.gr2\"\n\t\tSourceSkin\t\t\t\"warrior_novice_red.dds\"\n\t\tTargetSkin\t\t\t\"warrior_novice_red.dds\"\n\t}\n\tGroup ShapeData01\n\t{\n\t\tShapeIndex\t\t\t1\n\t\tModel\t\t\t\t\"warrior_novice.gr2\"\n\t\tSourceSkin\t\t\t\"warrior_novice_red.dds\"\n\t\tTargetSkin\t\t\t\"warrior_novice_blue.dds\"\n\t}\n}\n\nGroup HairData\n{\n\tPathName \"d:/ymir Work/pc2/warrior/\"\n\tHairDataCount 1\n}\n";

    #[test]
    fn finds_shapedata_block_not_the_whole_file() {
        let (start, end) = find_shapedata_block(SAMPLE).unwrap();
        let block = &SAMPLE[start..end];
        assert!(block.contains("ShapeDataCount"));
        assert!(!block.contains("HairData"), "must stop at the block's own closing brace");
    }

    #[test]
    fn appends_entry_and_bumps_count() {
        let dir = std::env::temp_dir().join(format!("m2manager_msm_test_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("pack/pc_warrior/ymir work/pc2/warrior")).unwrap();
        std::fs::write(
            dir.join("pack/pc_warrior/ymir work/pc2/warrior/warrior.msm"),
            SAMPLE,
        )
        .unwrap();

        add_shape_data(
            dir.to_str().unwrap(),
            "warrior",
            250,
            "custom/TestMod/body.gr2",
            "custom/TestMod/body.dds",
        )
        .unwrap();

        let updated = std::fs::read_to_string(dir.join("pack/pc_warrior/ymir work/pc2/warrior/warrior.msm")).unwrap();
        assert!(updated.contains("ShapeDataCount\t\t\t3"));
        assert!(updated.contains("Group ShapeData02"));
        assert!(updated.contains("ShapeIndex\t\t\t250"));
        assert!(updated.contains("custom/TestMod/body.gr2"));
        assert!(updated.contains("Group HairData"), "unrelated blocks must survive untouched");

        // Re-parses cleanly (brace-balanced) after the edit.
        let (s, e) = find_shapedata_block(&updated).unwrap();
        assert!(updated[s..e].contains("Group ShapeData02"));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn next_free_index_beats_stock_specials() {
        let dir = std::env::temp_dir().join(format!("m2manager_msm_test2_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("pack/pc_warrior/ymir work/pc2/warrior")).unwrap();
        std::fs::write(
            dir.join("pack/pc_warrior/ymir work/pc2/warrior/warrior.msm"),
            SAMPLE.replace("ShapeIndex\t\t\t1\n", "ShapeIndex\t\t\t201\n"),
        )
        .unwrap();
        assert_eq!(next_free_shape_index(dir.to_str().unwrap(), 0), 202);
        assert_eq!(next_free_shape_index(dir.to_str().unwrap(), 500), 501);
        std::fs::remove_dir_all(&dir).ok();
    }
}
