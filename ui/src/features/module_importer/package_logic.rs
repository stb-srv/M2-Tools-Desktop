//! Reine Logik des Paket-Importers (Auswahl, Item-Aufbau, Parameter und
//! Meldungen der Import-Pipeline) - ohne WASM testbar. Die Komponente in
//! `components/package_importer.rs` ruft nur noch auf.

use super::shared::*;
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct WeaponRowState {
    pub selected: bool,
    /// -1 = noch nicht gewählt
    pub subtype: i64,
    pub is_sura: bool,
    pub icon: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArmorRowState {
    pub selected: bool,
    pub icon: Option<String>,
}

/// Eingaben, aus denen die Items gebaut werden (Schnappschuss beim Start).
#[derive(Clone)]
pub struct BuildInput {
    pub ref_item: Option<ItemProtoInput>,
    pub base_name: String,
    pub base_locale_name: String,
    pub chain_growth_percent: i64,
    pub armor_subtype: i64,
    pub armor_wearflag: i64,
}

pub fn or_else<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.is_empty() {
        b
    } else {
        a
    }
}

pub fn build_weapon_item(inp: &BuildInput, vnum: i64, variant: &WeaponVariant, row: &WeaponRowState, level: i64) -> ItemProtoInput {
    let template = inp.ref_item.clone().unwrap_or_else(|| empty_item(vnum, 1));
    // Klassensperre kommt immer aus Subtyp/Modell der Variante, nie vom
    // Referenz-Item; übrige antiflag-Bits des Referenz-Items bleiben.
    let antiflag = (template.antiflag & !CLASS_ANTIFLAG_MASK) | class_antiflag_for_weapon(row.subtype, row.is_sura);
    let suffix = if level > 0 { format!("+{level}") } else { String::new() };
    let grow = |base: i64| scaled_value(base, level, inp.chain_growth_percent);
    ItemProtoInput {
        vnum,
        vnum_range: 0,
        name: fit_to_byte_limit_with_suffix(
            &format!("{}_{}", or_else(&inp.base_name, "item"), variant.key),
            &suffix,
            ITEM_NAME_MAX_BYTES,
        ),
        locale_name: fit_to_byte_limit_with_suffix(
            &format!("{} ({})", or_else(&inp.base_locale_name, &inp.base_name), label_for_variant(variant)),
            &if suffix.is_empty() { String::new() } else { format!(" {suffix}") },
            ITEM_NAME_MAX_BYTES,
        ),
        r#type: 1,
        subtype: row.subtype,
        antiflag,
        size: size_for_subtype(row.subtype),
        wearflag: if template.wearflag != 0 { template.wearflag } else { WEAPON_WEARFLAG },
        // Schaden min/max + Zusatz-Angriffskraft wachsen gemeinsam.
        value3: grow(template.value3),
        value4: grow(template.value4),
        value5: grow(template.value5),
        // Kette wird danach explizit über set_item_refine_link verdrahtet.
        refine_set: 0,
        refined_vnum: 0,
        ..template
    }
}

pub fn build_armor_item(inp: &BuildInput, vnum: i64, shape_index: i64, selected_race_keys: &[String], level: i64) -> ItemProtoInput {
    let template = inp.ref_item.clone().unwrap_or_else(|| empty_item(vnum, 2));
    let blocked_mask = if selected_race_keys.is_empty() {
        0
    } else {
        RACE_ANTIFLAG
            .iter()
            .filter(|(race, _)| !selected_race_keys.iter().any(|k| k == race))
            .fold(0, |acc, (_, bit)| acc | bit)
    };
    let antiflag = (template.antiflag & !CLASS_ANTIFLAG_MASK) | blocked_mask;
    let suffix = if level > 0 { format!(" +{level}") } else { String::new() };
    let grow = |base: i64| scaled_value(base, level, inp.chain_growth_percent);
    ItemProtoInput {
        vnum,
        vnum_range: 0,
        name: fit_to_byte_limit_with_suffix(
            &format!("{}_armor", or_else(&inp.base_name, "item")),
            &if level > 0 { format!("+{level}") } else { String::new() },
            ITEM_NAME_MAX_BYTES,
        ),
        locale_name: fit_to_byte_limit_with_suffix(
            or_else(&inp.base_locale_name, &inp.base_name),
            &suffix,
            ITEM_NAME_MAX_BYTES,
        ),
        r#type: 2,
        subtype: inp.armor_subtype,
        antiflag,
        // Rüstung ist im Inventar immer 1 Slot groß - nie vom Referenz-Item erben.
        size: 1,
        wearflag: if inp.armor_wearflag != 0 { inp.armor_wearflag } else { template.wearflag },
        // value3 ist bei Rüstung der 3D-Modell-Shape-Index und bleibt konstant.
        value0: grow(template.value0),
        value1: grow(template.value1),
        value5: grow(template.value5),
        value3: shape_index,
        refine_set: 0,
        refined_vnum: 0,
        ..template
    }
}

pub fn race_list(armors: &[ArmorPiece]) -> String {
    armors.iter().map(|a| race_label(&a.race)).collect::<Vec<_>>().join(", ")
}

/// Ausgewählte Waffen-Varianten in Scan-Reihenfolge.
pub fn selected_weapons(scan: Option<&ScannedModule>, rows: &HashMap<String, WeaponRowState>) -> Vec<WeaponVariant> {
    scan.map(|s| s.weapons.iter().filter(|v| rows.get(&v.key).is_some_and(|x| x.selected)).cloned().collect())
        .unwrap_or_default()
}

/// Ausgewählte Rüstungs-Rassen in Scan-Reihenfolge.
pub fn selected_armor_races(scan: Option<&ScannedModule>, rows: &HashMap<String, ArmorRowState>) -> Vec<ArmorPiece> {
    scan.map(|s| s.armors.iter().filter(|a| rows.get(&a.race).is_some_and(|x| x.selected)).cloned().collect())
        .unwrap_or_default()
}

/// Rassen mit weiblichem Modell (nur diese bekommen ein 3D-Modell verknüpft).
pub fn female_races(armors: &[ArmorPiece]) -> Vec<ArmorPiece> {
    armors.iter().filter(|a| a.female_model_abs.is_some()).cloned().collect()
}

/// Rassen, die nur ein männliches Modell haben.
pub fn male_only_races(armors: &[ArmorPiece]) -> Vec<ArmorPiece> {
    armors.iter().filter(|a| a.female_model_abs.is_none() && a.male_model_abs.is_some()).cloned().collect()
}

/// Darf der Import starten? Es braucht ein Scan-Ergebnis, mindestens eine
/// Auswahl, beide Namen und für jede gewählte Waffe einen Subtyp.
pub fn can_import(
    scanned: bool,
    weapons: &[WeaponVariant],
    weapon_rows: &HashMap<String, WeaponRowState>,
    armors: &[ArmorPiece],
    base_name: &str,
    base_locale_name: &str,
) -> bool {
    scanned
        && (!weapons.is_empty() || !armors.is_empty())
        && !base_name.trim().is_empty()
        && !base_locale_name.trim().is_empty()
        && weapons.iter().all(|v| weapon_rows.get(&v.key).map(|x| x.subtype) != Some(-1))
}

/// Anzahl Stufen (Basis + Aufwertungen) je Item.
pub fn levels_for(chain_enabled: bool, chain_max_level: i64) -> i64 {
    if chain_enabled {
        chain_max_level + 1
    } else {
        1
    }
}

/// Anzahl der anzulegenden Items: je Waffe eins, Rüstung zusammen eins,
/// jeweils mal Kettenstufen.
pub fn item_count(weapons: usize, has_armor: bool, chain_enabled: bool, chain_max_level: i64) -> i64 {
    (weapons as i64 + if has_armor { 1 } else { 0 }) * levels_for(chain_enabled, chain_max_level)
}

/// Soll `write_item_desc` laufen?
pub fn has_description(description: &str, summary: &str) -> bool {
    !description.trim().is_empty() || !summary.trim().is_empty()
}

/// Welche Pack-Schritte laufen: (Icons, Modelle).
pub fn pack_steps(has_weapons: bool, has_armors: bool) -> (bool, bool) {
    if has_weapons {
        (true, true)
    } else if has_armors {
        (true, false)
    } else {
        (false, false)
    }
}

/// Typ für den Verlaufseintrag: 1 nur bei reinen Waffen-Imports.
pub fn batch_item_type(has_weapons: bool, has_armors: bool) -> i64 {
    if has_weapons && !has_armors {
        1
    } else {
        2
    }
}

/// Shape-Index der Rüstung ohne weibliches Modell: vom Referenz-Item.
pub fn fallback_shape_index(ref_item: Option<&ItemProtoInput>) -> i64 {
    ref_item.map(|r| r.value3).unwrap_or(0)
}

/// Relativer Icon-Pfad in `item_list.txt`.
pub fn icon_rel_path(base_vnum: i64) -> String {
    format!("icon/item/{}.tga", pad5(base_vnum))
}

/// Hinweis, der nach einem Rollback an die Fehlermeldung angehängt wird.
/// `scope` benennt, was zurückgenommen wurde (z. B. " (Datenbank, Icon, item_list.txt)").
pub fn rollback_note(created: usize, failed: &[i64], scope: &str) -> String {
    if failed.is_empty() {
        format!("\n\nAlle {created} bereits angelegten Einträge{scope} wurden automatisch zurückgenommen.")
    } else {
        format!(
            "\n\nRollback teilweise fehlgeschlagen - bitte vnums {} manuell im DB Explorer prüfen.",
            join_vnums(failed)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn weapon(key: &str) -> WeaponVariant {
        WeaponVariant {
            key: key.into(),
            label: key.into(),
            subtype: 0,
            is_sura_model: false,
            model_source_abs: "a.gr2".into(),
            texture_sources_abs: vec![],
            icon_source_abs: None,
        }
    }
    fn armor(race: &str, female: bool, male: bool) -> ArmorPiece {
        ArmorPiece {
            race: race.into(),
            label: race.into(),
            male_model_abs: male.then(|| "m".into()),
            male_textures_abs: vec![],
            female_model_abs: female.then(|| "f".into()),
            female_textures_abs: vec![],
            icon_source_abs: None,
        }
    }
    fn scan(weapons: Vec<WeaponVariant>, armors: Vec<ArmorPiece>) -> ScannedModule {
        ScannedModule {
            module_name: "m".into(),
            weapons,
            armors,
            icon_pool: vec![],
            effect_dirs: vec![],
            notices: vec![],
        }
    }
    fn wrow(selected: bool, subtype: i64) -> WeaponRowState {
        WeaponRowState { selected, subtype, is_sura: false, icon: None }
    }
    fn input() -> BuildInput {
        BuildInput {
            ref_item: None,
            base_name: "bow".into(),
            base_locale_name: "Bogen".into(),
            chain_growth_percent: 10,
            armor_subtype: 0,
            armor_wearflag: 1,
        }
    }

    #[test]
    fn selection_filters_by_row_flag() {
        let s = scan(vec![weapon("a"), weapon("b")], vec![armor("warrior", true, true)]);
        let rows = HashMap::from([("a".to_string(), wrow(true, 0)), ("b".to_string(), wrow(false, 0))]);
        let w = selected_weapons(Some(&s), &rows);
        assert_eq!(w.len(), 1);
        assert_eq!(w[0].key, "a");
        assert!(selected_weapons(None, &rows).is_empty());
        let arows = HashMap::from([("warrior".to_string(), ArmorRowState { selected: true, icon: None })]);
        assert_eq!(selected_armor_races(Some(&s), &arows).len(), 1);
    }

    #[test]
    fn female_and_male_only_split() {
        let a = vec![armor("warrior", true, true), armor("sura", false, true), armor("shaman", false, false)];
        assert_eq!(female_races(&a).len(), 1);
        let m = male_only_races(&a);
        assert_eq!(m.len(), 1);
        assert_eq!(m[0].race, "sura");
    }

    #[test]
    fn can_import_rules() {
        let w = vec![weapon("a")];
        let ok = HashMap::from([("a".to_string(), wrow(true, 2))]);
        assert!(can_import(true, &w, &ok, &[], "n", "N"));
        assert!(!can_import(false, &w, &ok, &[], "n", "N"));
        assert!(!can_import(true, &[], &ok, &[], "n", "N"));
        assert!(!can_import(true, &w, &ok, &[], "  ", "N"));
        assert!(!can_import(true, &w, &ok, &[], "n", ""));
        let unset = HashMap::from([("a".to_string(), wrow(true, -1))]);
        assert!(!can_import(true, &w, &unset, &[], "n", "N"));
        assert!(can_import(true, &[], &ok, &[armor("warrior", true, true)], "n", "N"));
    }

    #[test]
    fn counts_and_levels() {
        assert_eq!(item_count(3, true, false, 9), 4);
        assert_eq!(item_count(2, false, true, 9), 20);
        assert_eq!(item_count(0, false, true, 9), 0);
        assert_eq!(levels_for(true, 4), 5);
        assert_eq!(levels_for(false, 4), 1);
    }

    #[test]
    fn pack_and_batch_type() {
        assert_eq!(pack_steps(true, true), (true, true));
        assert_eq!(pack_steps(false, true), (true, false));
        assert_eq!(pack_steps(false, false), (false, false));
        assert_eq!(batch_item_type(true, false), 1);
        assert_eq!(batch_item_type(true, true), 2);
        assert_eq!(batch_item_type(false, true), 2);
    }

    #[test]
    fn description_and_paths() {
        assert!(!has_description(" ", ""));
        assert!(has_description("x", ""));
        assert!(has_description("", "y"));
        assert_eq!(icon_rel_path(42), "icon/item/00042.tga");
        let it = ItemProtoInput { value3: 7, ..ItemProtoInput::empty(1) };
        assert_eq!(fallback_shape_index(Some(&it)), 7);
        assert_eq!(fallback_shape_index(None), 0);
    }

    #[test]
    fn rollback_notes() {
        assert!(rollback_note(3, &[], " (X)").contains("Alle 3 bereits angelegten Einträge (X) wurden"));
        assert!(rollback_note(3, &[5, 6], "").contains("vnums"));
    }

    #[test]
    fn weapon_item_levels() {
        let v = weapon("a");
        let row = wrow(true, 0);
        let base = build_weapon_item(&input(), 100, &v, &row, 0);
        assert_eq!(base.r#type, 1);
        assert_eq!(base.refine_set, 0);
        assert!(base.name.starts_with("bow_"));
        let plus = build_weapon_item(&input(), 101, &v, &row, 2);
        assert!(plus.name.ends_with("+2"));
        assert!(plus.locale_name.ends_with(" +2"));
    }

    #[test]
    fn armor_item_fields() {
        let item = build_armor_item(&input(), 200, 55, &["warrior".to_string()], 0);
        assert_eq!(item.r#type, 2);
        assert_eq!(item.value3, 55);
        assert_eq!(item.size, 1);
        assert_ne!(item.antiflag, 0);
        let all = build_armor_item(&input(), 200, 55, &[], 0);
        assert_eq!(all.antiflag, 0);
    }

    #[test]
    fn race_list_joins_labels() {
        assert_eq!(race_list(&[armor("warrior", true, true), armor("sura", true, true)]).matches(", ").count(), 1);
    }
}
