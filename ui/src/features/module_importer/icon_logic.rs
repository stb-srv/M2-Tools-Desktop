//! Reine Logik des Icon-Item-Importers (Zeilenzustand, Namensvorschlag,
//! Item-Aufbau je Stufe) - ohne WASM testbar.

use super::shared::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct IconItemRow {
    pub name: String,
    pub locale_name: String,
    pub description: String,
    pub summary: String,
    pub item_type: i64,
    pub subtype: i64,
    pub wearflag: i64,
    pub values: [i64; 6],
    /// Welche value0-5 pro Aufwertungs-Stufe wachsen - bewusst pro Feld,
    /// da die Bedeutung je Typ/Subtyp variiert.
    pub grow_flags: [bool; 6],
    pub ref_item: Option<ItemProtoInput>,
    pub chain_enabled: bool,
    pub chain_max_level: i64,
    pub chain_growth_percent: i64,
}

pub fn default_icon_item_row(suggested_name: &str) -> IconItemRow {
    IconItemRow {
        name: suggested_name.to_string(),
        locale_name: suggested_name.to_string(),
        description: String::new(),
        summary: String::new(),
        item_type: 2,
        subtype: 0,
        wearflag: 0,
        values: [0; 6],
        grow_flags: [false; 6],
        ref_item: None,
        chain_enabled: false,
        chain_max_level: 9,
        chain_growth_percent: 10,
    }
}

/// Vorschlag aus dem Dateinamen: Endung weg, `_`/`-`-Folgen zu Leerzeichen.
pub fn suggested_name(path: &str) -> String {
    let file = file_name_of(path);
    let stem = match file.rfind('.') {
        Some(i) if i + 1 < file.len() => file[..i].to_string(),
        _ => file.clone(),
    };
    let mut out = String::new();
    let mut in_sep = false;
    for c in stem.chars() {
        if c == '_' || c == '-' {
            if !in_sep {
                out.push(' ');
            }
            in_sep = true;
        } else {
            out.push(c);
            in_sep = false;
        }
    }
    let trimmed = out.trim().to_string();
    if trimmed.is_empty() {
        stem
    } else {
        trimmed
    }
}

/// Ausgewählte Icon-Pfade in Scan-Reihenfolge.
pub fn selected_paths(icon_paths: Option<&[String]>, selected: &HashMap<String, bool>) -> Vec<String> {
    icon_paths
        .map(|p| p.iter().filter(|x| selected.get(*x).copied().unwrap_or(false)).cloned().collect())
        .unwrap_or_default()
}

/// Import erlaubt: mindestens ein Icon gewählt und überall Name + Anzeigename gesetzt.
pub fn can_import(selected: &[String], rows: &HashMap<String, IconItemRow>) -> bool {
    !selected.is_empty()
        && selected
            .iter()
            .all(|p| rows.get(p).is_some_and(|x| !x.name.trim().is_empty() && !x.locale_name.trim().is_empty()))
}

/// Anzahl Stufen (Basis + Aufwertungen) der Zeile.
pub fn levels_of(row: &IconItemRow) -> i64 {
    if row.chain_enabled {
        row.chain_max_level + 1
    } else {
        1
    }
}

/// Soll `write_item_desc` laufen?
pub fn has_description(row: &IconItemRow) -> bool {
    !row.description.trim().is_empty() || !row.summary.trim().is_empty()
}

/// Baut das Item für eine Stufe der Zeile.
pub fn build_icon_item(row: &IconItemRow, vnum: i64, level: i64) -> ItemProtoInput {
    let template = row.ref_item.clone().unwrap_or_else(|| empty_item(vnum, row.item_type));
    let suffix = if level > 0 { format!(" +{level}") } else { String::new() };
    let values: Vec<i64> = row
        .values
        .iter()
        .enumerate()
        .map(|(i, v)| if row.grow_flags[i] { scaled_value(*v, level, row.chain_growth_percent) } else { *v })
        .collect();
    ItemProtoInput {
        vnum,
        vnum_range: 0,
        name: fit_to_byte_limit_with_suffix(
            &row.name,
            &if level > 0 { format!("_{level}") } else { String::new() },
            ITEM_NAME_MAX_BYTES,
        ),
        locale_name: fit_to_byte_limit_with_suffix(&row.locale_name, &suffix, ITEM_NAME_MAX_BYTES),
        r#type: row.item_type,
        subtype: row.subtype,
        // Zubehör ohne 3D-Modell ist immer 1 Slot groß.
        size: 1,
        wearflag: row.wearflag,
        value0: values[0],
        value1: values[1],
        value2: values[2],
        value3: values[3],
        value4: values[4],
        value5: values[5],
        refine_set: 0,
        refined_vnum: 0,
        ..template
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggests_names_from_file() {
        assert_eq!(suggested_name("C:/x/my_cool--ring.tga"), "my cool ring");
        assert_eq!(suggested_name("/x/___.png"), "___");
        assert_eq!(suggested_name("/x/noext"), "noext");
    }

    #[test]
    fn selection_and_can_import() {
        let paths = vec!["a.tga".to_string(), "b.tga".to_string()];
        let sel = HashMap::from([("a.tga".to_string(), true), ("b.tga".to_string(), false)]);
        let chosen = selected_paths(Some(&paths), &sel);
        assert_eq!(chosen, vec!["a.tga".to_string()]);
        assert!(selected_paths(None, &sel).is_empty());
        let mut rows = HashMap::from([("a.tga".to_string(), default_icon_item_row("ring"))]);
        assert!(can_import(&chosen, &rows));
        assert!(!can_import(&[], &rows));
        rows.get_mut("a.tga").unwrap().locale_name = " ".into();
        assert!(!can_import(&chosen, &rows));
        assert!(!can_import(&chosen, &HashMap::new()));
    }

    #[test]
    fn levels_and_description() {
        let mut row = default_icon_item_row("x");
        assert_eq!(levels_of(&row), 1);
        row.chain_enabled = true;
        assert_eq!(levels_of(&row), 10);
        assert!(!has_description(&row));
        row.summary = "s".into();
        assert!(has_description(&row));
    }

    #[test]
    fn item_per_level_grows_flagged_values_only() {
        let mut row = default_icon_item_row("ring");
        row.values = [100, 100, 0, 0, 0, 0];
        row.grow_flags = [true, false, false, false, false, false];
        row.chain_growth_percent = 10;
        let base = build_icon_item(&row, 10, 0);
        assert_eq!(base.name, "ring");
        assert_eq!(base.value0, 100);
        assert_eq!(base.size, 1);
        let plus = build_icon_item(&row, 11, 2);
        assert_eq!(plus.name, "ring_2");
        assert_eq!(plus.locale_name, "ring +2");
        assert_eq!(plus.value0, scaled_value(100, 2, 10));
        assert_eq!(plus.value1, 100);
    }
}
