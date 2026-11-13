//! Typen und reine Hilfsfunktionen des Item-Importers (Port von
//! `shared.ts`). `ItemProtoInput` wird aus dem Item Editor übernommen statt
//! dupliziert.

use crate::tauri::invoke;
use serde::Deserialize;
use serde_json::json;

pub use crate::features::item_editor::types::{pad5, ItemProtoInput, StepStatus};

/// Leeres Item mit gegebenem Typ (Größe 1, alles andere 0).
pub fn empty_item(vnum: i64, item_type: i64) -> ItemProtoInput {
    let mut item = ItemProtoInput::empty(vnum);
    item.r#type = item_type;
    item.size = 1;
    item
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ItemSearchResult {
    pub vnum: i64,
    pub name: String,
}

// Spiegelt die Rust-Structs aus modulescan.rs 1:1.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct WeaponVariant {
    pub key: String,
    pub label: String,
    /// -1 = nicht erkannt, manuelle Auswahl nötig.
    pub subtype: i64,
    pub is_sura_model: bool,
    pub model_source_abs: String,
    pub texture_sources_abs: Vec<String>,
    pub icon_source_abs: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ArmorPiece {
    /// warrior | assassin | sura | shaman
    pub race: String,
    #[allow(dead_code)]
    pub label: String,
    pub male_model_abs: Option<String>,
    #[allow(dead_code)]
    pub male_textures_abs: Vec<String>,
    pub female_model_abs: Option<String>,
    pub female_textures_abs: Vec<String>,
    pub icon_source_abs: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ScannedModule {
    pub module_name: String,
    pub weapons: Vec<WeaponVariant>,
    pub armors: Vec<ArmorPiece>,
    pub icon_pool: Vec<String>,
    pub effect_dirs: Vec<String>,
    pub notices: Vec<String>,
}

/// `RACE_LABELS[race] ?? race`.
pub fn race_label(race: &str) -> String {
    match race {
        "warrior" => "Krieger",
        "assassin" => "Ninja",
        "sura" => "Sura",
        "shaman" => "Schamane",
        other => other,
    }
    .to_string()
}

// Deutsche Bezeichnungen für bekannte Suffixe - in genau dieser Reihenfolge
// geprüft (erster Treffer gewinnt), sonst der Roh-Schlüssel.
const VARIANT_LABELS: &[(&str, &str)] = &[
    ("sword", "Schwert"),
    ("blade", "Schwert"),
    ("saber", "Schwert"),
    ("sabre", "Schwert"),
    ("katana", "Schwert"),
    ("1h", "Einhand"),
    ("sura", "Sura-Variante"),
    ("dagger", "Dolch"),
    ("knife", "Dolch"),
    ("bow", "Bogen"),
    ("2hand", "Zweihand"),
    ("twohand", "Zweihand"),
    ("2h", "Zweihand"),
    ("spear", "Zweihand"),
    ("glaive", "Zweihand"),
    ("bell", "Glocke"),
    ("staff", "Glocke (Stab)"),
    ("fan", "Fächer"),
    ("arrow", "Pfeil"),
    ("claw", "Kralle"),
];

pub fn file_name_of(abs_path: &str) -> String {
    abs_path.rsplit(['\\', '/']).next().unwrap_or(abs_path).to_string()
}

pub fn label_for_variant(v: &WeaponVariant) -> String {
    for (needle, label) in VARIANT_LABELS {
        if v.label.contains(needle) {
            return if v.is_sura_model && *needle != "sura" { format!("{label} (Sura)") } else { label.to_string() };
        }
    }
    v.label.clone()
}

/// item_proto.name/.locale_name sind feste varbinary-Spalten
/// (ITEM_NAME_MAX_LEN = 24 im Client) - MySQL lehnt Überlängen ab, daher
/// wird clientseitig auf Bytes (nicht Zeichen) gekürzt.
pub const ITEM_NAME_MAX_BYTES: usize = 24;

pub fn fit_to_byte_limit(s: &str, max_bytes: usize) -> String {
    let mut truncated = s.to_string();
    while !truncated.is_empty() && truncated.len() > max_bytes {
        truncated.pop();
    }
    truncated
}

/// Wie [`fit_to_byte_limit`], kürzt aber garantiert nur `base`, nie
/// `suffix` (z.B. " +16") - sonst kollidierten zweistellige Stufen ("+10"
/// wurde zu "+1").
pub fn fit_to_byte_limit_with_suffix(base: &str, suffix: &str, max_bytes: usize) -> String {
    let budget = max_bytes.saturating_sub(suffix.len());
    fit_to_byte_limit(base, budget) + suffix
}

// Aufwertungs-Kette: echte Stock-Vorlage dieses Servers (Schwert 10-19,
// Mönchsplattenpanzer 11200-11209). Index i = Kosten/Chance für Stufe i→i+1;
// über die 9 Schritte hinaus wiederholen sich die Werte von +9.
const STOCK_REFINE_COST: [i64; 9] = [600, 1200, 2500, 5000, 10000, 20000, 30000, 45000, 75000];
const STOCK_REFINE_PROB: [i64; 9] = [90, 90, 90, 90, 80, 60, 60, 60, 60];

pub fn refine_cost_for_step(step: i64) -> i64 {
    STOCK_REFINE_COST[(step.max(0) as usize).min(STOCK_REFINE_COST.len() - 1)]
}
pub fn refine_prob_for_step(step: i64) -> i64 {
    STOCK_REFINE_PROB[(step.max(0) as usize).min(STOCK_REFINE_PROB.len() - 1)]
}

/// JavaScripts `Math.round` (x.5 rundet Richtung +∞).
fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Lineares Wachstum eines Kampfwerts: Stufe 0 behält den Wert des
/// Referenz-Items, Stufe N ist `base * (1 + growth/100 * N)`. Basis 0 bleibt 0.
pub fn scaled_value(base: i64, level: i64, growth_percent: i64) -> i64 {
    if level == 0 || base == 0 {
        return base;
    }
    js_round(base as f64 * (1.0 + (growth_percent as f64 / 100.0) * level as f64)) as i64
}

/// Sucht `count` *zusammenhängende* freie vnums ab `range_start` - eine
/// ganze Aufwertungs-Kette braucht einen lückenlosen Block.
pub async fn find_consecutive_free_vnums(range_start: i64, count: i64) -> Result<i64, String> {
    let mut candidate = range_start;
    loop {
        let base = invoke::<i64>("next_free_item_vnum", &json!({ "rangeStart": candidate })).await?;
        let mut ok = true;
        for i in 1..count {
            let next = invoke::<i64>("next_free_item_vnum", &json!({ "rangeStart": base + i })).await?;
            if next != base + i {
                candidate = next;
                ok = false;
                break;
            }
        }
        if ok {
            return Ok(base);
        }
    }
}

/// "Waffe"-Bit, siehe item_flags::WEAR_FLAGS.
pub const WEAPON_WEARFLAG: i64 = 1 << 4;

// item_proto.antiflag-Bits, siehe item_flags::ANTI_FLAGS.
const ANTIFLAG_NO_WARRIOR: i64 = 1 << 2;
const ANTIFLAG_NO_NINJA: i64 = 1 << 3;
const ANTIFLAG_NO_SURA: i64 = 1 << 4;
const ANTIFLAG_NO_SHAMAN: i64 = 1 << 5;
const ANTIFLAG_NO_WOLFMAN: i64 = 1 << 18;
pub const CLASS_ANTIFLAG_MASK: i64 =
    ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN;

/// `RACE_ANTIFLAG` in Deklarationsreihenfolge.
pub const RACE_ANTIFLAG: [(&str, i64); 4] = [
    ("warrior", ANTIFLAG_NO_WARRIOR),
    ("assassin", ANTIFLAG_NO_NINJA),
    ("sura", ANTIFLAG_NO_SURA),
    ("shaman", ANTIFLAG_NO_SHAMAN),
];

/// Inventar-Größe einer Waffe hängt nur vom Subtyp ab (live verifiziert).
pub fn size_for_subtype(subtype: i64) -> i64 {
    match subtype {
        0 | 2 => 2, // Schwert, Bogen
        3 | 7 => 3, // Zweihand, Reitspeer
        _ => 1,
    }
}

/// Welche Klassen eine Waffe tragen dürfen - nur vom Subtyp (und bei
/// Schwertern vom Sura-Modell) abhängig, live gegen ~2000 Stock-Waffen geprüft.
pub fn class_antiflag_for_weapon(subtype: i64, is_sura_model: bool) -> i64 {
    match subtype {
        0 => {
            if is_sura_model {
                ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN
            } else {
                ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN
            }
        }
        1 | 2 | 6 => ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN,
        3 => ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN | ANTIFLAG_NO_WOLFMAN,
        4 | 5 => ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_WOLFMAN,
        8 => ANTIFLAG_NO_WARRIOR | ANTIFLAG_NO_NINJA | ANTIFLAG_NO_SURA | ANTIFLAG_NO_SHAMAN,
        _ => 0,
    }
}

pub fn class_label_for(blocked_mask: i64) -> String {
    let classes: Vec<&str> = [
        (ANTIFLAG_NO_WARRIOR, "Krieger"),
        (ANTIFLAG_NO_NINJA, "Ninja"),
        (ANTIFLAG_NO_SURA, "Sura"),
        (ANTIFLAG_NO_SHAMAN, "Schamane"),
    ]
    .iter()
    .filter(|(bit, _)| blocked_mask & bit == 0)
    .map(|(_, l)| *l)
    .collect();
    if classes.is_empty() {
        "alle Klassen".to_string()
    } else {
        classes.join(" + ")
    }
}

/// `folderPath.split(/[\\/]/).pop()`.
pub fn last_path_segment(p: &str) -> String {
    file_name_of(p)
}

/// `vnums.join(", ")`.
pub fn join_vnums(v: &[i64]) -> String {
    v.iter().map(|n| n.to_string()).collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Port von ModuleImporter.test.ts
    #[test]
    fn never_truncates_the_suffix() {
        let long_base = "Sehr Langer Waffenname XX";
        let names: Vec<String> = (0..17)
            .map(|level| {
                fit_to_byte_limit_with_suffix(
                    long_base,
                    &if level > 0 { format!(" +{level}") } else { String::new() },
                    ITEM_NAME_MAX_BYTES,
                )
            })
            .collect();
        let unique: HashSet<&String> = names.iter().collect();
        assert_eq!(unique.len(), names.len());
        for level in 1..=16 {
            assert!(names[level].ends_with(&format!(" +{level}")), "{}", names[level]);
        }
    }

    #[test]
    fn respects_overall_byte_limit() {
        let long_base = "Ein Extrem Langer Basis-Item-Name Der Weit Ueber Das Limit Hinausgeht";
        let result = fit_to_byte_limit_with_suffix(long_base, " +16", ITEM_NAME_MAX_BYTES);
        assert!(result.len() <= ITEM_NAME_MAX_BYTES);
        assert!(result.ends_with(" +16"));
    }

    #[test]
    fn matches_plain_fit_without_suffix() {
        let base = "Kurzer Name";
        assert_eq!(
            fit_to_byte_limit_with_suffix(base, "", ITEM_NAME_MAX_BYTES),
            fit_to_byte_limit(base, ITEM_NAME_MAX_BYTES)
        );
    }

    #[test]
    fn byte_limit_counts_utf8_bytes() {
        // "ä" = 2 Byte - 13 Stück = 26 Byte -> 12 Zeichen bleiben.
        let s = "ä".repeat(13);
        assert_eq!(fit_to_byte_limit(&s, ITEM_NAME_MAX_BYTES).chars().count(), 12);
    }

    #[test]
    fn helpers() {
        assert_eq!(file_name_of("C:\\a\\b/c.tga"), "c.tga");
        assert_eq!(scaled_value(100, 0, 10), 100);
        assert_eq!(scaled_value(100, 3, 10), 130);
        assert_eq!(scaled_value(0, 3, 10), 0);
        assert_eq!(scaled_value(5, 1, 10), 6); // 5.5 -> 6 (wie Math.round)
        assert_eq!(scaled_value(-5, 1, 10), -5); // -5.5 -> -5 (wie Math.round)
        assert_eq!(refine_cost_for_step(0), 600);
        assert_eq!(refine_cost_for_step(15), 75000);
        assert_eq!(refine_prob_for_step(4), 80);
        assert_eq!(size_for_subtype(3), 3);
        assert_eq!(class_label_for(class_antiflag_for_weapon(0, false)), "Krieger + Ninja + Sura");
        assert_eq!(class_label_for(class_antiflag_for_weapon(8, false)), "alle Klassen");
        let v = WeaponVariant {
            key: "k".into(),
            label: "my_sword".into(),
            subtype: 0,
            is_sura_model: true,
            model_source_abs: String::new(),
            texture_sources_abs: vec![],
            icon_source_abs: None,
        };
        assert_eq!(label_for_variant(&v), "Schwert (Sura)");
    }
}
