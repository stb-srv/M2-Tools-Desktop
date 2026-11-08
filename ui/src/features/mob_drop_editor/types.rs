//! Datentypen der mob_drop_item.txt (wie `types.ts`).
//!
//! Zahlen bleiben `f64`, weil das TS-Original Eingaben ungeprüft per
//! `Number(..)` übernimmt und genau so an das Backend schickt.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MobDropItem {
    pub item_vnum: f64,
    pub count: f64,
    pub percent: f64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MobDropGroup {
    pub name: String,
    pub mob_vnum: f64,
    pub drop_type: String,
    pub items: Vec<MobDropItem>,
}

/// Siehe mobdrop::check_numbering (Rust-Backend) - die laufende Nummer vor
/// jeder Item-Zeile ist nicht kosmetisch: der Server sucht die Einträge über
/// genau diesen Schlüssel ("1", "2", ...) und bricht an der ersten Lücke ab,
/// alle Einträge danach gehen still verloren. `found` sind die Nummern so,
/// wie sie in der Datei stehen (in Dateireihenfolge).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NumberingIssue {
    pub group_index: usize,
    pub found: Vec<i64>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MobDropLoadResult {
    pub groups: Vec<MobDropGroup>,
    pub numbering_issues: Vec<NumberingIssue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BulkMode {
    Delta,
    Fixed,
    Random,
    SpecificItem,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BulkScope {
    Global,
    Current,
}
