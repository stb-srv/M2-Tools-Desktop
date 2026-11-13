//! Gemeinsame Prüfung aller 5 Drop-Dateien: welche Items und Mobs werden
//! referenziert, und gibt es sie in der Datenbank?
//!
//! Hintergrund (Server-Quellcode, `item_manager_read_tables.cpp`): Für
//! `mob_drop_item.txt`, `common_drop_item.txt`, `drop_item_group.txt` und
//! `special_item_group.txt` versucht der Server das Item-Feld zuerst als
//! internen Item-Namen (`GetVnumByOriginalName`) und erst danach als Nummer.
//! Gibt es das Item nicht, bricht er mit `return false` ab - der ganze
//! Server startet dann nicht. Heißt ein anderes Item intern genau wie die
//! Nummer (z.B. "14"), nimmt der Server dieses Item. `etc_drop_item.txt`
//! kennt nur Namen.
//!
//! Dieses Modul ist reine Logik (testbar ohne Server/DB). Das Einlesen der
//! Dateien und die Datenbankabfragen stehen in `commands/drop_check.rs`.

use crate::common_drop::{CommonDropFile, CommonDropLine, RANK_NAMES};
use crate::drop_item_group::DropItemGroup;
use crate::etc_drop::EtcDropEntry;
use crate::mobdrop::MobDropGroup;
use crate::special_item_group::{is_special_keyword, SpecialItemGroup};
use serde::Serialize;
use std::collections::{BTreeSet, HashMap};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DropFileKind {
    MobDrop,
    CommonDrop,
    EtcDrop,
    SpecialItemGroup,
    DropItemGroup,
}

impl DropFileKind {
    pub const ALL: [DropFileKind; 5] = [
        DropFileKind::MobDrop,
        DropFileKind::CommonDrop,
        DropFileKind::EtcDrop,
        DropFileKind::SpecialItemGroup,
        DropFileKind::DropItemGroup,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            DropFileKind::MobDrop => "mob_drop_item.txt",
            DropFileKind::CommonDrop => "common_drop_item.txt",
            DropFileKind::EtcDrop => "etc_drop_item.txt",
            DropFileKind::SpecialItemGroup => "special_item_group.txt",
            DropFileKind::DropItemGroup => "drop_item_group.txt",
        }
    }

    pub fn settings_key(self) -> &'static str {
        match self {
            DropFileKind::MobDrop => "mob_drop_file_path",
            DropFileKind::CommonDrop => "common_drop_file_path",
            DropFileKind::EtcDrop => "etc_drop_file_path",
            DropFileKind::SpecialItemGroup => "special_item_group_file_path",
            DropFileKind::DropItemGroup => "drop_item_group_file_path",
        }
    }

    pub fn default_path(self) -> String {
        format!("/usr/home/game/share/{}", self.file_name())
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "mob_drop" => Some(DropFileKind::MobDrop),
            "common_drop" => Some(DropFileKind::CommonDrop),
            "etc_drop" => Some(DropFileKind::EtcDrop),
            "special_item_group" => Some(DropFileKind::SpecialItemGroup),
            "drop_item_group" => Some(DropFileKind::DropItemGroup),
            _ => None,
        }
    }
}

/// Ein Item- oder Mob-Verweis in einer Drop-Datei.
#[derive(Debug, Clone, PartialEq)]
pub enum DropRef {
    /// Item über die Nummer (Server prüft zuerst den Namen, dann die Nummer).
    Item { location: String, vnum: u32 },
    /// Item über den internen Namen (nur etc_drop_item.txt).
    ItemName { location: String, name: String },
    /// Kisten-Item einer special_item_group-Gruppe (muss es geben, sonst
    /// lässt sich die Kiste nicht öffnen - startet aber trotzdem).
    BoxItem { location: String, vnum: u32 },
    /// Mob-VNUM (fehlende Mobs stoppen den Server nicht, die Drops wirken
    /// dann nur nie).
    Mob { location: String, vnum: u32 },
}

pub fn refs_mob_drop(groups: &[MobDropGroup]) -> Vec<DropRef> {
    let mut out = Vec::new();
    for g in groups {
        let loc = format!("Gruppe „{}“ (Mob {})", g.name, g.mob_vnum);
        out.push(DropRef::Mob { location: loc.clone(), vnum: g.mob_vnum });
        for it in &g.items {
            out.push(DropRef::Item { location: loc.clone(), vnum: it.item_vnum });
        }
    }
    out
}

pub fn refs_common_drop(file: &CommonDropFile) -> Vec<DropRef> {
    let mut out = Vec::new();
    for (li, line) in file.lines.iter().enumerate() {
        if let CommonDropLine::Rule { blocks } = line {
            let mut seen = BTreeSet::new();
            for (r, b) in blocks.iter().enumerate() {
                if seen.insert(b.vnum) {
                    let rank = RANK_NAMES.get(r).copied().unwrap_or("?");
                    let loc = if blocks.iter().all(|o| o.vnum == b.vnum) {
                        format!("Zeile {}", li + 1)
                    } else {
                        format!("Zeile {}, {rank}", li + 1)
                    };
                    out.push(DropRef::Item { location: loc, vnum: b.vnum });
                }
            }
        }
    }
    out
}

pub fn refs_etc_drop(entries: &[EtcDropEntry]) -> Vec<DropRef> {
    entries
        .iter()
        .enumerate()
        .map(|(i, e)| DropRef::ItemName { location: format!("Zeile {}", i + 1), name: e.item_name.clone() })
        .collect()
}

pub fn refs_special_item_group(groups: &[SpecialItemGroup]) -> Vec<DropRef> {
    let mut out = Vec::new();
    for g in groups {
        let loc = format!("Gruppe „{}“ (Kiste {})", g.name, g.vnum);
        if g.vnum > 0 {
            out.push(DropRef::BoxItem { location: loc.clone(), vnum: g.vnum as u32 });
        }
        for e in &g.entries {
            if is_special_keyword(&e.item_ref) {
                continue;
            }
            if let Ok(vnum) = e.item_ref.trim().parse::<u32>() {
                out.push(DropRef::Item { location: loc.clone(), vnum });
            } else {
                out.push(DropRef::ItemName { location: loc.clone(), name: e.item_ref.clone() });
            }
        }
    }
    out
}

pub fn refs_drop_item_group(groups: &[DropItemGroup]) -> Vec<DropRef> {
    let mut out = Vec::new();
    for g in groups {
        let loc = format!("Gruppe „{}“ (Mob {})", g.name, g.mob_vnum);
        out.push(DropRef::Mob { location: loc.clone(), vnum: g.mob_vnum });
        for it in &g.items {
            if let Ok(vnum) = it.item_ref.trim().parse::<u32>() {
                out.push(DropRef::Item { location: loc.clone(), vnum });
            } else {
                out.push(DropRef::ItemName { location: loc.clone(), name: it.item_ref.clone() });
            }
        }
    }
    out
}

/// Was die Datenbank über eine Item-VNUM weiß.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemFacts {
    pub exists: bool,
    /// Ein *anderes* Item, dessen interner Name genau die Nummer ist.
    pub name_collision: Option<(u32, String)>,
}

/// Abfrageergebnisse aus der Datenbank.
#[derive(Debug, Clone, Default)]
pub struct DbFacts {
    pub items: HashMap<u32, ItemFacts>,
    /// Interner Name -> existiert?
    pub item_names: HashMap<String, bool>,
    pub mobs: HashMap<u32, bool>,
}

pub fn item_vnums(refs: &[DropRef]) -> BTreeSet<u32> {
    refs.iter()
        .filter_map(|r| match r {
            DropRef::Item { vnum, .. } | DropRef::BoxItem { vnum, .. } => Some(*vnum),
            _ => None,
        })
        .collect()
}

pub fn item_names(refs: &[DropRef]) -> BTreeSet<String> {
    refs.iter()
        .filter_map(|r| match r {
            DropRef::ItemName { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect()
}

pub fn mob_vnums(refs: &[DropRef]) -> BTreeSet<u32> {
    refs.iter()
        .filter_map(|r| match r {
            DropRef::Mob { vnum, .. } => Some(*vnum),
            _ => None,
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DropCheckIssue {
    pub file: String,
    pub location: String,
    /// "error" (Server startet nicht / falsches Item) oder "warning".
    pub level: String,
    pub message: String,
}

impl DropCheckIssue {
    pub fn is_error(&self) -> bool {
        self.level == "error"
    }
}

/// Bewertet die Verweise einer Datei. Fehlt ein Wert in `facts`, wurde er
/// nicht geprüft und erzeugt keine Meldung.
pub fn evaluate(kind: DropFileKind, refs: &[DropRef], facts: &DbFacts) -> Vec<DropCheckIssue> {
    let file = kind.file_name().to_string();
    let mut out = Vec::new();
    let mut push = |location: &str, level: &str, message: String| {
        let issue = DropCheckIssue { file: file.clone(), location: location.to_string(), level: level.into(), message };
        if !out.contains(&issue) {
            out.push(issue);
        }
    };
    for r in refs {
        match r {
            DropRef::Item { location, vnum } => {
                let Some(f) = facts.items.get(vnum) else { continue };
                if !f.exists {
                    push(location, "error", format!("Item {vnum} gibt es nicht in item_proto - der Server startet dann nicht."));
                }
                if let Some((other, name)) = &f.name_collision {
                    push(
                        location,
                        "error",
                        format!("Der Server liest „{vnum}“ als Item-Namen und nimmt Item {other} ({name}) statt {vnum}."),
                    );
                }
            }
            DropRef::ItemName { location, name } => {
                if facts.item_names.get(name) == Some(&false) {
                    push(location, "error", format!("Kein Item mit dem internen Namen „{name}“ - der Server startet dann nicht."));
                }
            }
            DropRef::BoxItem { location, vnum } => {
                if facts.items.get(vnum).is_some_and(|f| !f.exists) {
                    push(location, "warning", format!("Das Kisten-Item {vnum} gibt es nicht in item_proto - die Gruppe wird nie benutzt."));
                }
            }
            DropRef::Mob { location, vnum } => {
                if facts.mobs.get(vnum) == Some(&false) {
                    push(location, "warning", format!("Mob {vnum} gibt es nicht in mob_proto - diese Drops wirken nie."));
                }
            }
        }
    }
    out
}

/// Kurzform für die Speichern-Sperre: nur die Fehler, als Text.
pub fn blocking_message(issues: &[DropCheckIssue]) -> Option<String> {
    let errors: Vec<String> =
        issues.iter().filter(|i| i.is_error()).map(|i| format!("{}: {}", i.location, i.message)).collect();
    if errors.is_empty() {
        None
    } else {
        Some(format!("Nicht gespeichert - die Datei verweist auf Items, die der Server nicht findet:\n{}", errors.join("\n")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common_drop;
    use crate::drop_item_group::DropGroupItem;
    use crate::mobdrop::MobDropItem;
    use crate::special_item_group::SpecialItemGroupEntry;

    fn facts() -> DbFacts {
        let mut f = DbFacts::default();
        f.items.insert(11, ItemFacts { exists: true, name_collision: None });
        f.items.insert(99999, ItemFacts { exists: false, name_collision: None });
        f.items.insert(14, ItemFacts { exists: true, name_collision: Some((50029, "Goldring".into())) });
        f.item_names.insert("Small_Ring".into(), true);
        f.item_names.insert("Broken".into(), false);
        f.mobs.insert(101, true);
        f.mobs.insert(7777, false);
        f
    }

    #[test]
    fn mob_drop_refs_and_issues() {
        let groups = vec![MobDropGroup {
            name: "Wolf".into(),
            mob_vnum: 7777,
            drop_type: "drop".into(),
            items: vec![
                MobDropItem { item_vnum: 11, count: 1, percent: 4.0 },
                MobDropItem { item_vnum: 99999, count: 1, percent: 4.0 },
                MobDropItem { item_vnum: 14, count: 1, percent: 4.0 },
            ],
        }];
        let refs = refs_mob_drop(&groups);
        assert_eq!(item_vnums(&refs).into_iter().collect::<Vec<_>>(), vec![11, 14, 99999]);
        assert_eq!(mob_vnums(&refs).into_iter().collect::<Vec<_>>(), vec![7777]);
        let issues = evaluate(DropFileKind::MobDrop, &refs, &facts());
        assert_eq!(issues.iter().filter(|i| i.is_error()).count(), 2, "{issues:?}");
        assert!(issues.iter().any(|i| !i.is_error() && i.message.contains("Mob 7777")));
        assert!(issues.iter().all(|i| i.file == "mob_drop_item.txt" && i.location.contains("Wolf")));
        assert!(blocking_message(&issues).unwrap().contains("99999"));
    }

    #[test]
    fn unchecked_values_produce_no_issue() {
        let groups = vec![MobDropGroup {
            name: "x".into(),
            mob_vnum: 1234,
            drop_type: "drop".into(),
            items: vec![MobDropItem { item_vnum: 4242, count: 1, percent: 1.0 }],
        }];
        assert!(evaluate(DropFileKind::MobDrop, &refs_mob_drop(&groups), &facts()).is_empty());
        assert!(blocking_message(&[]).is_none());
    }

    #[test]
    fn common_drop_dedupes_shared_vnum() {
        let file = common_drop::parse(
            "#x\nS\t1\t15\t4\t99999\t1\tS\t1\t15\t0\t99999\t1\tS\t1\t15\t0\t99999\t1\tS\t1\t15\t0\t11\t1\n",
        );
        let refs = refs_common_drop(&file);
        assert_eq!(refs.len(), 2);
        let issues = evaluate(DropFileKind::CommonDrop, &refs, &facts());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].location, "Zeile 2, PAWN");
    }

    #[test]
    fn etc_drop_names() {
        let entries = vec![
            EtcDropEntry { item_name: "Small_Ring".into(), percent: 0.5 },
            EtcDropEntry { item_name: "Broken".into(), percent: 0.5 },
        ];
        let issues = evaluate(DropFileKind::EtcDrop, &refs_etc_drop(&entries), &facts());
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].location, "Zeile 2");
    }

    #[test]
    fn special_group_skips_keywords_and_warns_for_box() {
        let groups = vec![SpecialItemGroup {
            name: "Truhe".into(),
            vnum: 99999,
            group_type: None,
            entries: vec![
                SpecialItemGroupEntry { item_ref: "gold".into(), count: 1000, prob: 1, rare_pct: None },
                SpecialItemGroupEntry { item_ref: "11".into(), count: 1, prob: 1, rare_pct: None },
            ],
        }];
        let refs = refs_special_item_group(&groups);
        assert_eq!(refs.len(), 2);
        let issues = evaluate(DropFileKind::SpecialItemGroup, &refs, &facts());
        assert_eq!(issues.len(), 1);
        assert!(!issues[0].is_error());
    }

    #[test]
    fn drop_item_group_refs() {
        let groups = vec![DropItemGroup {
            name: "Extra".into(),
            mob_vnum: 101,
            items: vec![DropGroupItem { item_ref: "14".into(), percent: 0.5, count: 1 }],
        }];
        let issues = evaluate(DropFileKind::DropItemGroup, &refs_drop_item_group(&groups), &facts());
        assert_eq!(issues.len(), 1);
        assert!(issues[0].message.contains("50029"));
    }

    #[test]
    fn kind_ids() {
        for k in DropFileKind::ALL {
            assert!(k.default_path().ends_with(k.file_name()));
        }
        assert_eq!(DropFileKind::from_id("common_drop"), Some(DropFileKind::CommonDrop));
        assert_eq!(DropFileKind::from_id("x"), None);
    }
}
