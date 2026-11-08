//! Duplikat-Prüfungen für mob_drop_item.txt (wie `duplicates.ts`).

use super::types::MobDropGroup;

#[derive(Clone, Debug, PartialEq)]
pub struct DuplicateItemFinding {
    pub group_index: usize,
    pub mob_name: String,
    pub mob_vnum: f64,
    pub item_vnum: f64,
    pub item_indices: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuplicateMobFinding {
    pub mob_vnum: f64,
    pub group_indices: Vec<usize>,
}

/// Gruppiert Positionen nach Schlüssel in Reihenfolge des ersten Auftretens
/// (wie eine JS-`Map`).
fn group_by_first_seen(keys: impl Iterator<Item = f64>) -> Vec<(f64, Vec<usize>)> {
    let mut out: Vec<(f64, Vec<usize>)> = Vec::new();
    for (i, k) in keys.enumerate() {
        match out.iter_mut().find(|(key, _)| *key == k) {
            Some((_, list)) => list.push(i),
            None => out.push((k, vec![i])),
        }
    }
    out
}

/// Innerhalb eines Mobs dieselbe Item-VNUM mehrfach in der Drop-Liste -
/// passiert leicht beim manuellen Bearbeiten der mob_drop_item.txt.
/// `item_indices` enthält jede Position, damit eine einzelne Dopplung
/// gezielt entfernt werden kann statt alle auf einmal.
pub fn find_duplicate_items_in_mobs(groups: &[MobDropGroup]) -> Vec<DuplicateItemFinding> {
    let mut findings = Vec::new();
    for (group_index, g) in groups.iter().enumerate() {
        for (item_vnum, item_indices) in group_by_first_seen(g.items.iter().map(|i| i.item_vnum)) {
            if item_indices.len() > 1 {
                findings.push(DuplicateItemFinding {
                    group_index,
                    mob_name: g.name.clone(),
                    mob_vnum: g.mob_vnum,
                    item_vnum,
                    item_indices,
                });
            }
        }
    }
    findings
}

/// Derselbe Mob (gleiche mob_vnum) kommt mehrfach als eigener Eintrag in der
/// Datei vor - unabhängig von der obigen Prüfung, ein Mob kann in beiden
/// Listen auftauchen.
pub fn find_duplicate_mobs(groups: &[MobDropGroup]) -> Vec<DuplicateMobFinding> {
    group_by_first_seen(groups.iter().map(|g| g.mob_vnum))
        .into_iter()
        .filter(|(_, idx)| idx.len() > 1)
        .map(|(mob_vnum, group_indices)| DuplicateMobFinding { mob_vnum, group_indices })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::mob_drop_editor::types::MobDropItem;

    fn group(name: &str, mob_vnum: f64, items: Vec<MobDropItem>) -> MobDropGroup {
        MobDropGroup { name: name.into(), mob_vnum, drop_type: "drop".into(), items }
    }

    fn item(item_vnum: f64, percent: f64) -> MobDropItem {
        MobDropItem { item_vnum, count: 1.0, percent }
    }

    // findDuplicateItemsInMobs
    #[test]
    fn finds_nothing_when_every_mobs_items_are_unique() {
        let groups = vec![group("Mob", 101.0, vec![item(1.0, 10.0), item(2.0, 5.0)])];
        assert_eq!(find_duplicate_items_in_mobs(&groups), vec![]);
    }

    #[test]
    fn finds_an_item_vnum_entered_twice_within_one_mob() {
        let groups = vec![group("Wolf", 101.0, vec![item(42.0, 10.0), item(99.0, 5.0), item(42.0, 20.0)])];
        assert_eq!(
            find_duplicate_items_in_mobs(&groups),
            vec![DuplicateItemFinding {
                group_index: 0,
                mob_name: "Wolf".into(),
                mob_vnum: 101.0,
                item_vnum: 42.0,
                item_indices: vec![0, 2],
            }]
        );
    }

    #[test]
    fn collects_all_positions_when_same_vnum_appears_three_times() {
        let groups = vec![group("Mob", 101.0, vec![item(5.0, 1.0), item(5.0, 2.0), item(5.0, 3.0)])];
        assert_eq!(find_duplicate_items_in_mobs(&groups)[0].item_indices, vec![0, 1, 2]);
    }

    #[test]
    fn keeps_duplicates_scoped_per_mob() {
        let groups = vec![group("Mob", 101.0, vec![item(7.0, 1.0)]), group("Mob", 102.0, vec![item(7.0, 1.0)])];
        assert_eq!(find_duplicate_items_in_mobs(&groups), vec![]);
    }

    // findDuplicateMobs
    #[test]
    fn finds_nothing_when_every_mob_vnum_is_unique() {
        let groups = vec![group("Mob", 101.0, vec![]), group("Mob", 102.0, vec![])];
        assert_eq!(find_duplicate_mobs(&groups), vec![]);
    }

    #[test]
    fn finds_a_mob_vnum_that_appears_as_more_than_one_group() {
        let groups = vec![group("Mob", 101.0, vec![]), group("Mob", 102.0, vec![]), group("Mob", 101.0, vec![])];
        assert_eq!(find_duplicate_mobs(&groups), vec![DuplicateMobFinding { mob_vnum: 101.0, group_indices: vec![0, 2] }]);
    }

    // both checks together
    #[test]
    fn a_single_mob_shows_up_in_both_findings_independently() {
        let groups = vec![group("Mob", 101.0, vec![item(1.0, 1.0), item(1.0, 1.0)]), group("Mob", 101.0, vec![])];
        assert_eq!(
            find_duplicate_items_in_mobs(&groups),
            vec![DuplicateItemFinding {
                group_index: 0,
                mob_name: "Mob".into(),
                mob_vnum: 101.0,
                item_vnum: 1.0,
                item_indices: vec![0, 1],
            }]
        );
        assert_eq!(find_duplicate_mobs(&groups), vec![DuplicateMobFinding { mob_vnum: 101.0, group_indices: vec![0, 1] }]);
    }
}
