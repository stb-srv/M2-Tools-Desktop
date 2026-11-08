//! Reine Logik des Aufwertungs-Editors.

use super::RefineMaterial;
use crate::features::regen_editor::num_input::{js_number_string, js_to_number};

#[derive(Clone, Debug, PartialEq)]
pub struct MaterialRow {
    pub vnum: f64,
    pub name: String,
    pub count: f64,
}

// Empty materials always pad up to 5 slots in the edit form (matching
// refine_proto's fixed vnum0-4/count0-4 storage) so removing a row just
// means clearing it, no array-length juggling.
impl Default for MaterialRow {
    fn default() -> Self {
        Self { vnum: 0.0, name: String::new(), count: 1.0 }
    }
}

pub fn initial_material_rows(materials: &[RefineMaterial]) -> Vec<MaterialRow> {
    let mut rows: Vec<MaterialRow> = materials
        .iter()
        .map(|m| MaterialRow {
            vnum: m.vnum,
            name: m.locale_name.clone().unwrap_or_else(|| format!("#{}", js_number_string(m.vnum))),
            count: m.count,
        })
        .collect();
    while rows.len() < 5 {
        rows.push(MaterialRow::default());
    }
    rows
}

/// Nur belegte Slots gehen ans Backend.
pub fn active_materials(rows: &[MaterialRow]) -> Vec<MaterialRow> {
    rows.iter().filter(|m| m.vnum > 0.0 && m.count > 0.0).cloned().collect()
}

/// `Number(reuseId)` muss eine ganze Zahl > 0 sein.
pub fn parse_reuse_id(input: &str) -> Option<f64> {
    let id = js_to_number(input);
    (id.is_finite() && id.fract() == 0.0 && id > 0.0).then_some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_to_five_slots() {
        let rows = initial_material_rows(&[RefineMaterial { vnum: 30050.0, count: 2.0, locale_name: None }]);
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].name, "#30050");
        assert_eq!(rows[1], MaterialRow::default());
        assert_eq!(rows[1].count, 1.0);
    }

    #[test]
    fn active_filters_empty_slots() {
        let rows = vec![
            MaterialRow { vnum: 1.0, name: "a".into(), count: 1.0 },
            MaterialRow { vnum: 2.0, name: "b".into(), count: 0.0 },
            MaterialRow::default(),
        ];
        assert_eq!(active_materials(&rows).len(), 1);
    }

    #[test]
    fn reuse_id_validation() {
        assert_eq!(parse_reuse_id("12"), Some(12.0));
        assert_eq!(parse_reuse_id(" 7 "), Some(7.0));
        assert_eq!(parse_reuse_id(""), None);
        assert_eq!(parse_reuse_id("0"), None);
        assert_eq!(parse_reuse_id("-3"), None);
        assert_eq!(parse_reuse_id("1.5"), None);
        assert_eq!(parse_reuse_id("abc"), None);
    }
}
