//! Datentypen des Item Editors (Port von `types.ts`). `ItemProtoInput`
//! entspricht 1:1 dem gleichnamigen Struct in `src-tauri/src/db/item.rs`.

use serde::{Deserialize, Serialize};

macro_rules! item_proto {
    ($($field:ident),* $(,)?) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        pub struct ItemProtoInput {
            pub vnum: i64,
            pub vnum_range: i64,
            pub name: String,
            pub locale_name: String,
            #[serde(rename = "type")]
            pub r#type: i64,
            $(pub $field: i64,)*
        }

        impl ItemProtoInput {
            /// Zahlenfeld nach Spaltenname lesen (wie `item[key]` im TS).
            pub fn num(&self, key: &str) -> i64 {
                match key {
                    "vnum" => self.vnum,
                    "vnum_range" => self.vnum_range,
                    "type" => self.r#type,
                    $(stringify!($field) => self.$field,)*
                    _ => 0,
                }
            }

            /// Zahlenfeld nach Spaltenname setzen.
            pub fn set_num(&mut self, key: &str, value: i64) {
                match key {
                    "vnum" => self.vnum = value,
                    "vnum_range" => self.vnum_range = value,
                    "type" => self.r#type = value,
                    $(stringify!($field) => self.$field = value,)*
                    _ => {}
                }
            }

            pub fn empty(vnum: i64) -> Self {
                Self {
                    vnum,
                    vnum_range: 0,
                    name: String::new(),
                    locale_name: String::new(),
                    r#type: 0,
                    $($field: 0,)*
                }
            }
        }
    };
}

item_proto!(
    subtype, weight, size, antiflag, flag, wearflag, immuneflag, gold, shop_buy_price, refined_vnum, refine_set,
    magic_pct, limittype0, limitvalue0, limittype1, limitvalue1, applytype0, applyvalue0, applytype1, applyvalue1,
    applytype2, applyvalue2, applytype3, applyvalue3, value0, value1, value2, value3, value4, value5, socket0,
    socket1, socket2, socket3, socket4, socket5, specular, socket_pct, addon_type,
);

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ItemDescEntry {
    #[allow(dead_code)]
    pub vnum: i64,
    pub description: String,
    pub summary: String,
    #[allow(dead_code)]
    pub extra: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Done,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Create,
    Edit,
}

/// Neues, leeres Item (Typ 3 = USE, Größe 1) - wie `emptyItem` im TS.
pub fn empty_item(vnum: i64) -> ItemProtoInput {
    let mut item = ItemProtoInput::empty(vnum);
    item.r#type = 3;
    item.size = 1;
    item
}

/// Wie JavaScripts `Number(value)`, mit nicht-endlichen Werten als 0
/// (`Number.isFinite(n) ? n : 0` bzw. `Number(v) || 0`). Leer/Leerzeichen = 0.
pub fn js_number(value: &str) -> f64 {
    let s = value.trim();
    if s.is_empty() {
        return 0.0;
    }
    let lower = s.to_ascii_lowercase();
    let parsed = if let Some(hex) = lower.strip_prefix("0x") {
        i64::from_str_radix(hex, 16).ok().map(|v| v as f64)
    } else if lower.contains("inf") || lower.contains("nan") {
        None
    } else {
        s.parse::<f64>().ok()
    };
    match parsed {
        Some(n) if n.is_finite() => n,
        _ => 0.0,
    }
}

/// `js_number` als Ganzzahl (die Item-Spalten sind alle ganzzahlig).
pub fn num(value: &str) -> i64 {
    js_number(value).trunc() as i64
}

/// `String(n).padStart(5, "0")`.
pub fn pad5(n: i64) -> String {
    let s = n.to_string();
    if s.len() >= 5 {
        s
    } else {
        format!("{}{}", "0".repeat(5 - s.len()), s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_item_defaults() {
        let i = empty_item(42);
        assert_eq!(i.vnum, 42);
        assert_eq!(i.r#type, 3);
        assert_eq!(i.size, 1);
        assert_eq!(i.value5, 0);
    }

    #[test]
    fn serializes_type_key() {
        let v = serde_json::to_value(empty_item(1)).unwrap();
        assert_eq!(v["type"], 3);
        assert_eq!(v["socket_pct"], 0);
        assert_eq!(v.as_object().unwrap().len(), 44);
    }

    #[test]
    fn field_access_by_name() {
        let mut i = empty_item(1);
        i.set_num("applyvalue2", 7);
        assert_eq!(i.applyvalue2, 7);
        assert_eq!(i.num("applyvalue2"), 7);
        i.set_num("type", 1);
        assert_eq!(i.r#type, 1);
    }

    #[test]
    fn js_number_semantics() {
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number("  12 "), 12.0);
        assert_eq!(js_number("abc"), 0.0);
        assert_eq!(js_number("1e3"), 1000.0);
        assert_eq!(js_number("0x10"), 16.0);
        assert_eq!(js_number("Infinity"), 0.0);
        assert_eq!(num("-5"), -5);
    }

    #[test]
    fn pad5_works() {
        assert_eq!(pad5(12), "00012");
        assert_eq!(pad5(500000), "500000");
    }
}
