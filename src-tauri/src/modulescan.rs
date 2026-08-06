use serde::Serialize;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Weapon-subtype detection by substring aliases against the normalized
/// filename stem. Order matters - first hit wins, so the more specific
/// aliases (sura, two-handed) come before the generic "sword". Values are
/// `EWeaponSubTypes` from `source/common/item_length.h` (see
/// [[m2manager_item_editor]]). "staff" maps to Bell (4): asset packs that
/// ship a staff (e.g. FireDragon) use it as the Shaman bell-slot weapon and
/// ship no separate bell; the UI still allows overriding the subtype.
const WEAPON_ALIASES: &[(&[&str], i8, bool)] = &[
    (&["sura"], 0, true),
    (&["2hand", "twohand", "2h", "spear", "glaive"], 3, false),
    (&["dagger", "knife"], 1, false),
    (&["bow"], 2, false),
    (&["bell", "staff"], 4, false),
    (&["fan"], 5, false),
    (&["arrow"], 6, false),
    (&["claw"], 8, false),
    (&["sword", "blade", "saber", "sabre", "katana", "1h"], 0, false),
];

const IMAGE_EXTS: &[&str] = &["png", "tga", "jpg", "jpeg", "bmp"];
const RACES: &[&str] = &["warrior", "assassin", "sura", "shaman"];

#[derive(Debug, Clone, Serialize)]
pub struct WeaponVariant {
    /// Unique within the scan result (model filename stem, deduped).
    pub key: String,
    pub label: String,
    /// -1 when no alias matched - the frontend renders a manual subtype
    /// picker in that case instead of silently guessing.
    pub subtype: i8,
    pub is_sura_model: bool,
    pub model_source_abs: String,
    /// Sibling `.dds` textures in the model's own folder - GR2 files
    /// reference their skins by bare filename relative to the model, so
    /// these must be copied alongside or the weapon renders untextured.
    pub texture_sources_abs: Vec<String>,
    /// Best fuzzy icon guess - the UI lets the user override from the pool.
    pub icon_source_abs: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArmorPiece {
    /// warrior | assassin | sura | shaman
    pub race: String,
    pub label: String,
    pub male_model_abs: Option<String>,
    pub male_textures_abs: Vec<String>,
    pub female_model_abs: Option<String>,
    pub female_textures_abs: Vec<String>,
    pub icon_source_abs: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScannedModule {
    pub module_name: String,
    pub weapons: Vec<WeaponVariant>,
    pub armors: Vec<ArmorPiece>,
    /// Every image file found (except tooltip/preview shots) - offered in
    /// the UI as manual icon choices.
    pub icon_pool: Vec<String>,
    /// Directories containing `.mse`/`.mde` effect scripts (glow bundles).
    pub effect_dirs: Vec<String>,
    /// Things that were recognized but can't be auto-imported yet
    /// (hair models, wolfman armor, ...), surfaced verbatim in the UI.
    pub notices: Vec<String>,
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn ext_lower(path: &Path) -> String {
    path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

fn stem_lower(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_lowercase()
}

/// Normalized comparison form: lowercase, non-alphanumeric stripped, and a
/// trailing version marker (`_v2`, `2`, `_01`) removed so `fan_v2` and
/// `fan` compare equal.
fn normalize(stem: &str) -> String {
    let lower = stem.to_lowercase();
    let trimmed = lower
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .trim_end_matches(['v', '_', '-', ' ']);
    trimmed.chars().filter(|c| c.is_ascii_alphanumeric()).collect()
}

fn match_weapon_alias(stem: &str) -> Option<(i8, bool)> {
    let norm = normalize(stem);
    // "sura" always marks the Sura reskin, even combined with another alias
    // ("1h_sura", "sura_sword") - and those are subtype 0 swords.
    if norm.contains("sura") {
        return Some((0, true));
    }
    for (aliases, subtype, is_sura) in WEAPON_ALIASES {
        if aliases.iter().any(|a| norm.contains(a)) {
            return Some((*subtype, *is_sura));
        }
    }
    None
}

/// Detects `pc/<race>/`, `pc2/<race>/` or `pc3/wolfman` path segments.
/// Returns (race, is_female) - wolfman reports as its own race name.
fn race_from_path(path: &Path) -> Option<(String, bool)> {
    let norm = path.to_string_lossy().replace('\\', "/").to_lowercase();
    let segments: Vec<&str> = norm.split('/').collect();
    for (i, seg) in segments.iter().enumerate() {
        let gender_female = match *seg {
            "pc" => false,
            "pc2" => true,
            "pc3" => false,
            _ => continue,
        };
        if let Some(next) = segments.get(i + 1) {
            if RACES.contains(next) || *next == "wolfman" {
                return Some((next.to_string(), gender_female));
            }
        }
    }
    None
}

fn is_under_dir_named(path: &Path, name: &str) -> bool {
    path.ancestors().skip(1).any(|a| {
        a.file_name()
            .and_then(|f| f.to_str())
            .map(|f| f.to_lowercase() == name)
            .unwrap_or(false)
    })
}

fn sibling_textures(model_path: &Path) -> Vec<String> {
    let Some(dir) = model_path.parent() else {
        return Vec::new();
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && ext_lower(p) == "dds")
        .map(|p| p.display().to_string())
        .collect()
}

/// Scores how well an icon filename fits a weapon variant; higher is
/// better, `None` means no plausible relation.
fn weapon_icon_score(icon_stem: &str, variant_stem: &str, subtype: i8, is_sura: bool) -> Option<u32> {
    // An icon named exactly after a race ("sura.tga", "warrior.tga") is an
    // armor icon by convention - never let a weapon claim it, even though
    // "sura" also reads as a weapon alias.
    if RACES.contains(&icon_stem) || icon_stem == "wolfman" {
        return None;
    }
    if icon_stem == variant_stem {
        return Some(100);
    }
    if normalize(icon_stem) == normalize(variant_stem) {
        return Some(90);
    }
    match match_weapon_alias(icon_stem) {
        Some((icon_subtype, icon_sura)) if icon_subtype == subtype && icon_sura == is_sura => Some(70),
        _ => None,
    }
}

pub fn scan_module(root: &Path) -> Result<ScannedModule, String> {
    if !root.is_dir() {
        return Err(format!("Ordner nicht gefunden: {}", root.display()));
    }
    let module_name = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Modul")
        .to_string();

    let mut all_files = Vec::new();
    walk(root, &mut all_files);

    let mut weapon_models: Vec<PathBuf> = Vec::new();
    // (race, is_female, model path)
    let mut body_models: Vec<(String, bool, PathBuf)> = Vec::new();
    let mut icon_pool: Vec<PathBuf> = Vec::new();
    let mut effect_dirs: Vec<PathBuf> = Vec::new();
    let mut notices: Vec<String> = Vec::new();
    let mut hair_count = 0usize;
    let mut wolfman_count = 0usize;

    for path in &all_files {
        let ext = ext_lower(path);
        if ext == "gr2" {
            if let Some((race, is_female)) = race_from_path(path) {
                if is_under_dir_named(path, "hair") {
                    hair_count += 1;
                } else if race == "wolfman" {
                    wolfman_count += 1;
                } else {
                    body_models.push((race, is_female, path.clone()));
                }
            } else {
                weapon_models.push(path.clone());
            }
        } else if IMAGE_EXTS.contains(&ext.as_str()) {
            // Tooltip shots and preview renders are not usable as inventory
            // icons - keep them out of both auto-matching and the pool.
            let stem = stem_lower(path);
            if !is_under_dir_named(path, "tooltip") && !stem.starts_with("preview") && !stem.starts_with("prev") {
                icon_pool.push(path.clone());
            }
        } else if matches!(ext.as_str(), "mse" | "mde") {
            if let Some(parent) = path.parent() {
                if !effect_dirs.iter().any(|d| d == parent) {
                    effect_dirs.push(parent.to_path_buf());
                }
            }
        }
    }

    if hair_count > 0 {
        notices.push(format!(
            "{hair_count} Haar-Modell(e) gefunden - Haar-Import (HairData) wird noch nicht unterstützt."
        ));
    }
    if wolfman_count > 0 {
        notices.push(format!(
            "{wolfman_count} Wolfman-Modell(e) gefunden - Wolfman nutzt keine .msm-ShapeData, Import wird noch nicht unterstützt."
        ));
    }

    // ---- weapons ----
    let mut weapons = Vec::new();
    let mut used_keys: HashSet<String> = HashSet::new();
    let mut used_icons: HashSet<PathBuf> = HashSet::new();

    for model_path in &weapon_models {
        let stem = stem_lower(model_path);
        let (subtype, is_sura) = match_weapon_alias(&stem).unwrap_or((-1, false));

        let mut key = stem.clone();
        let mut n = 2;
        while used_keys.contains(&key) {
            key = format!("{stem}_{n}");
            n += 1;
        }
        used_keys.insert(key.clone());

        let icon = icon_pool
            .iter()
            .filter(|p| !used_icons.contains(*p))
            .filter_map(|p| weapon_icon_score(&stem_lower(p), &stem, subtype, is_sura).map(|s| (s, p)))
            .max_by_key(|(score, _)| *score)
            .map(|(_, p)| p.clone());
        if let Some(icon_path) = &icon {
            used_icons.insert(icon_path.clone());
        }

        weapons.push(WeaponVariant {
            key,
            label: stem,
            subtype,
            is_sura_model: is_sura,
            model_source_abs: model_path.display().to_string(),
            texture_sources_abs: sibling_textures(model_path),
            icon_source_abs: icon.map(|p| p.display().to_string()),
        });
    }

    // ---- armor (one piece per race, pairing male/female trees) ----
    let mut armors: Vec<ArmorPiece> = Vec::new();
    for race in RACES {
        let male: Vec<&PathBuf> = body_models
            .iter()
            .filter(|(r, f, _)| r == race && !*f)
            .map(|(_, _, p)| p)
            .collect();
        let female: Vec<&PathBuf> = body_models
            .iter()
            .filter(|(r, f, _)| r == race && *f)
            .map(|(_, _, p)| p)
            .collect();
        if male.is_empty() && female.is_empty() {
            continue;
        }
        if male.len() > 1 || female.len() > 1 {
            notices.push(format!(
                "Mehrere Körper-Modelle für {race} gefunden - nur das jeweils erste wird angeboten."
            ));
        }

        let icon = icon_pool
            .iter()
            .filter(|p| !used_icons.contains(*p))
            .find(|p| stem_lower(p).contains(race));
        if let Some(icon_path) = icon {
            used_icons.insert((*icon_path).clone());
        }

        let male_model = male.first().map(|p| (*p).clone());
        let female_model = female.first().map(|p| (*p).clone());
        armors.push(ArmorPiece {
            race: race.to_string(),
            label: male_model
                .as_ref()
                .or(female_model.as_ref())
                .map(|p| stem_lower(p))
                .unwrap_or_else(|| race.to_string()),
            male_textures_abs: male_model.as_deref().map(sibling_textures).unwrap_or_default(),
            female_textures_abs: female_model.as_deref().map(sibling_textures).unwrap_or_default(),
            male_model_abs: male_model.map(|p| p.display().to_string()),
            female_model_abs: female_model.map(|p| p.display().to_string()),
            icon_source_abs: icon.map(|p| p.display().to_string()),
        });
    }

    Ok(ScannedModule {
        module_name,
        weapons,
        armors,
        icon_pool: icon_pool.iter().map(|p| p.display().to_string()).collect(),
        effect_dirs: effect_dirs.iter().map(|p| p.display().to_string()).collect(),
        notices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weapon_alias_matching_covers_real_pack_names() {
        // FrostEdge
        assert_eq!(match_weapon_alias("fe_sword"), Some((0, false)));
        assert_eq!(match_weapon_alias("fe_sura_sword"), Some((0, true)));
        assert_eq!(match_weapon_alias("fe_2hand"), Some((3, false)));
        assert_eq!(match_weapon_alias("fe_bell"), Some((4, false)));
        // Jotunthrym
        assert_eq!(match_weapon_alias("jotun_thrym_2h_sword"), Some((3, false)));
        assert_eq!(match_weapon_alias("jotun_thrym_daggers_v2"), Some((1, false)));
        assert_eq!(match_weapon_alias("jotun_thrym_claws"), Some((8, false)));
        assert_eq!(match_weapon_alias("jotun_thrym_sura_sword2"), Some((0, true)));
        assert_eq!(match_weapon_alias("jotun_thrym_fan_v2"), Some((5, false)));
        // FireDragon
        assert_eq!(match_weapon_alias("1h"), Some((0, false)));
        assert_eq!(match_weapon_alias("1h_sura"), Some((0, true)));
        assert_eq!(match_weapon_alias("2h"), Some((3, false)));
        assert_eq!(match_weapon_alias("staff"), Some((4, false)));
        // icon-side names
        assert_eq!(match_weapon_alias("surasword"), Some((0, true)));
        assert_eq!(match_weapon_alias("sword2hand"), Some((3, false)));
        assert_eq!(match_weapon_alias("unrecognized_thing"), None);
    }

    // Real fixture shipped in the repo - the module the feature was
    // originally built for. Still must scan correctly through the generic
    // scanner.
    #[test]
    fn scans_real_frostedge_module() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("beispiel_neues_modul")
            .join("FrostEdge");
        if !root.is_dir() {
            eprintln!("skipping: fixture not present at {root:?}");
            return;
        }
        let result = scan_module(&root).expect("scan failed");
        assert_eq!(result.weapons.len(), 7, "FrostEdge ships 7 weapon models");
        assert!(result.armors.is_empty());
        for variant in &result.weapons {
            assert!(
                variant.icon_source_abs.is_some(),
                "every FrostEdge variant has a matching icon, missing for {}",
                variant.key
            );
            assert!(
                !variant.texture_sources_abs.is_empty(),
                "FrostEdge models ship sibling .dds textures, none found for {}",
                variant.key
            );
            assert_ne!(variant.subtype, -1, "all FrostEdge suffixes are known: {}", variant.key);
        }
        assert!(!result.effect_dirs.is_empty(), "FrostEdge ships glow effect dirs");
    }

    // The two real-world packages this rewrite was built against. They live
    // in the user's Downloads folder, so skip silently when absent (e.g. CI).
    #[test]
    fn scans_real_jotunthrym_package() {
        let root = Path::new(r"C:\Users\DevSteven\Downloads\neue_rüstung+waffe");
        if !root.is_dir() {
            eprintln!("skipping: package not present");
            return;
        }
        let result = scan_module(root).expect("scan failed");
        // 9 weapon gr2 under ymir work/item/weapon + 1 under "other fan"
        assert_eq!(result.weapons.len(), 10, "found: {:?}", result.weapons.iter().map(|w| &w.key).collect::<Vec<_>>());
        let claws = result.weapons.iter().find(|w| w.key.contains("claws")).unwrap();
        assert_eq!(claws.subtype, 8);
        let two_hand = result.weapons.iter().find(|w| w.key.contains("2h_sword")).unwrap();
        assert_eq!(two_hand.subtype, 3);
        assert!(two_hand.icon_source_abs.as_deref().unwrap_or("").to_lowercase().contains("sword2hand"));

        // 4 races with male+female body models each
        assert_eq!(result.armors.len(), 4, "found: {:?}", result.armors.iter().map(|a| &a.race).collect::<Vec<_>>());
        for armor in &result.armors {
            assert!(armor.male_model_abs.is_some(), "male model missing for {}", armor.race);
            assert!(armor.female_model_abs.is_some(), "female model missing for {}", armor.race);
            assert!(armor.icon_source_abs.is_some(), "icon missing for {}", armor.race);
            assert!(!armor.male_textures_abs.is_empty(), "male textures missing for {}", armor.race);
        }
        // hair + wolfman notices
        assert!(result.notices.iter().any(|n| n.contains("Haar")));
        assert!(result.notices.iter().any(|n| n.contains("Wolfman")));
    }

    #[test]
    fn scans_real_firedragon_package() {
        let root = Path::new(r"C:\Users\DevSteven\Downloads\newitemswaffen");
        if !root.is_dir() {
            eprintln!("skipping: package not present");
            return;
        }
        let result = scan_module(root).expect("scan failed");
        // 7 FireDragon weapons + 1 lost_sword
        assert_eq!(result.weapons.len(), 8, "found: {:?}", result.weapons.iter().map(|w| &w.key).collect::<Vec<_>>());
        let staff = result.weapons.iter().find(|w| w.key == "staff").unwrap();
        assert_eq!(staff.subtype, 4, "staff maps to bell slot");
        let sura = result.weapons.iter().find(|w| w.key == "1h_sura").unwrap();
        assert!(sura.is_sura_model);
        assert!(sura.icon_source_abs.as_deref().unwrap_or("").to_lowercase().ends_with("1h_sura.tga"));
        // FireDragon weapons ship one shared dds next to the models
        let one_hand = result.weapons.iter().find(|w| w.key == "1h").unwrap();
        assert!(!one_hand.texture_sources_abs.is_empty());
        // weapon_effect dir found even though it's not under "ymir work/effect"
        assert!(!result.effect_dirs.is_empty());
        assert!(result.armors.is_empty());
    }
}
