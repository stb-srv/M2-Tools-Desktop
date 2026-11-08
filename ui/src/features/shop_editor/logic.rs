//! Reine Logik des Shop Editors (Raster-Platzierung, Spalten/Zeilen-Grenzen,
//! `Number()`-artiges Zahlen-Parsen).

use std::collections::HashSet;

/// Metin2-Shopfenster haben höchstens 40 Plätze - Spalten/Zeilen sind nur
/// Darstellung, ihr Produkt darf diese Spielgrenze aber nie überschreiten.
pub const MAX_SLOTS: i64 = 40;

#[derive(Clone, Debug, PartialEq, serde::Deserialize)]
pub struct ShopItem {
    pub item_vnum: i64,
    pub item_name: String,
    pub count: i64,
    pub size: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlacedItem {
    pub item: ShopItem,
    pub col: i64,
    pub row: i64,
    pub span: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub placed_items: Vec<PlacedItem>,
    pub empty_cells: Vec<(i64, i64)>,
    pub overflow_items: Vec<ShopItem>,
    pub used_slots: usize,
}

/// Items sind 1 Feld breit, aber `size` Felder HOCH (item_proto.size, 1-3) -
/// ein size-3-Item blockiert die zwei Zellen darunter, wie im Spiel.
/// Von oben links platzieren, übrige Zellen bleiben klickbare Leerplätze.
/// `empty_cells` als (row, col), 1-basiert.
pub fn place_items(items: &[ShopItem], columns: i64, rows: i64) -> Placement {
    let mut occupied: HashSet<(i64, i64)> = HashSet::new();
    let mut placed_items = Vec::new();
    let mut overflow_items = Vec::new();

    for item in items {
        let span = item.size.max(1).min(rows);
        let mut placed = false;
        let mut row = 1;
        while row <= rows - span + 1 && !placed {
            let mut col = 1;
            while col <= columns && !placed {
                let cells: Vec<(i64, i64)> = (0..span).map(|i| (row + i, col)).collect();
                if !cells.iter().any(|c| occupied.contains(c)) {
                    occupied.extend(cells);
                    placed_items.push(PlacedItem { item: item.clone(), col, row, span });
                    placed = true;
                }
                col += 1;
            }
            row += 1;
        }
        if !placed {
            overflow_items.push(item.clone());
        }
    }

    let mut empty_cells = Vec::new();
    for row in 1..=rows {
        for col in 1..=columns {
            if !occupied.contains(&(row, col)) {
                empty_cells.push((row, col));
            }
        }
    }

    Placement { placed_items, empty_cells, overflow_items, used_slots: occupied.len() }
}

/// Neue Spaltenzahl -> (Spalten, Zeilen).
pub fn update_columns(next: i64, rows: i64) -> (i64, i64) {
    let clamped = next.clamp(1, MAX_SLOTS);
    let new_rows = if clamped * rows > MAX_SLOTS { (MAX_SLOTS / clamped).max(1) } else { rows };
    (clamped, new_rows)
}

/// Neue Zeilenzahl -> (Spalten, Zeilen).
pub fn update_rows(next: i64, columns: i64) -> (i64, i64) {
    let clamped = next.clamp(1, MAX_SLOTS);
    let new_cols = if columns * clamped > MAX_SLOTS { (MAX_SLOTS / clamped).max(1) } else { columns };
    (new_cols, clamped)
}

/// Verhält sich wie JavaScripts `Number(s)`: Leerraum wird ignoriert, ein
/// leerer String ist 0, `0x`/`0o`/`0b`-Präfixe sind erlaubt, alles andere
/// Ungültige ergibt NaN.
pub fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return u64::from_str_radix(rest, radix).map(|v| v as f64).unwrap_or(f64::NAN);
        }
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // Rust akzeptiert zusätzlich "inf"/"nan" - JS nicht.
    if t.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// JSON-Wert einer JS-Zahl (NaN/Infinity werden wie bei `JSON.stringify` zu null).
pub fn js_number_json(x: f64) -> serde_json::Value {
    if !x.is_finite() {
        serde_json::Value::Null
    } else if x.fract() == 0.0 && x.abs() < 9.007_199_254_740_992e15 {
        serde_json::json!(x as i64)
    } else {
        serde_json::json!(x)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(vnum: i64, size: i64) -> ShopItem {
        ShopItem { item_vnum: vnum, item_name: format!("I{vnum}"), count: 1, size }
    }

    #[test]
    fn places_tall_items_top_left_first() {
        let p = place_items(&[item(1, 3), item(2, 1), item(3, 2)], 2, 3);
        let pos: Vec<_> = p.placed_items.iter().map(|x| (x.item.item_vnum, x.row, x.col, x.span)).collect();
        assert_eq!(pos, vec![(1, 1, 1, 3), (2, 1, 2, 1), (3, 2, 2, 2)]);
        assert!(p.empty_cells.is_empty());
        assert_eq!(p.used_slots, 6);
        assert!(p.overflow_items.is_empty());
    }

    #[test]
    fn overflow_and_empty_cells() {
        let p = place_items(&[item(1, 2), item(2, 2), item(3, 1)], 1, 3);
        assert_eq!(p.placed_items.len(), 2);
        assert_eq!(p.overflow_items.iter().map(|i| i.item_vnum).collect::<Vec<_>>(), vec![2]);
        assert!(p.empty_cells.is_empty());
        let p = place_items(&[item(1, 1)], 2, 2);
        assert_eq!(p.empty_cells, vec![(1, 2), (2, 1), (2, 2)]);
    }

    #[test]
    fn size_is_clamped_to_rows_and_min_one() {
        let p = place_items(&[item(1, 5), item(2, 0)], 2, 2);
        assert_eq!(p.placed_items[0].span, 2);
        assert_eq!(p.placed_items[1].span, 1);
        assert_eq!((p.placed_items[1].row, p.placed_items[1].col), (1, 2));
    }

    #[test]
    fn columns_rows_respect_max_slots() {
        assert_eq!(update_columns(6, 8), (6, 6));
        assert_eq!(update_columns(5, 8), (5, 8));
        assert_eq!(update_columns(0, 8), (1, 8));
        assert_eq!(update_columns(99, 1), (40, 1));
        assert_eq!(update_rows(9, 5), (4, 9));
        assert_eq!(update_rows(41, 1), (1, 40));
    }

    #[test]
    fn js_number_like_javascript() {
        assert_eq!(js_number("3306"), 3306.0);
        assert_eq!(js_number(" 12 "), 12.0);
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number("0x10"), 16.0);
        assert_eq!(js_number("1e3"), 1000.0);
        assert!(js_number("abc").is_nan());
        assert!(js_number("inf").is_nan());
        assert!(js_number("12abc").is_nan());
        assert!(js_number("Infinity").is_infinite());
        assert_eq!(js_number_json(3306.0), serde_json::json!(3306));
        assert_eq!(js_number_json(f64::NAN), serde_json::Value::Null);
        assert_eq!(js_number_json(1.5), serde_json::json!(1.5));
    }
}
