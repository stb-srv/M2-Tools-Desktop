//! Gemeinsame Typen und Konstanten der Account-/Spieler-Verwaltung
//! (Port von `src/features/account-manager/shared.ts`).

use leptos::prelude::*;
use serde::Deserialize;
use serde_json::Value;

pub use crate::features::shared::generic_row_editor::{ColumnInfo, TableRows};
use crate::store::navigation::{ModuleId, Section};

/// Kennung fürs Änderungsprotokoll.
pub const MODULE: ModuleId = ModuleId::Section(Section::AccountManager);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TableTarget {
    pub database: &'static str,
    pub table: &'static str,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct AccountSummary {
    pub id: i64,
    pub login: String,
    pub email: String,
    pub status: String,
    pub empire: i64,
    pub create_time: String,
    pub last_play: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct BanRecord {
    pub id: i64,
    pub account_id: i64,
    pub login: String,
    pub reason: String,
    pub banned_at: String,
    pub unban_at: Option<String>,
    pub active: bool,
}

// Verified table locations from earlier live-DB introspection - account/player
// data lives in these two tables on this core. The search column and primary
// key are NOT assumed beyond that: search column is user-editable (schemas
// vary in which column holds the login/character name), and the primary key
// is auto-detected at runtime by GenericRowEditor via `is_primary_key`, never
// guessed as "id".
pub const PLAYER_TABLE: TableTarget = TableTarget { database: "player", table: "player" };

/// Entspricht JavaScripts `Number(s)`: Leerraum wird ignoriert, leer = 0,
/// ungültig = NaN.
pub fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    if t.chars().any(|c| c.is_ascii_alphabetic() && c != 'e' && c != 'E') {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(s)` als Kommando-Argument - wie `JSON.stringify` wird NaN/Infinity
/// zu `null`, ganze Zahlen bleiben ganzzahlig.
pub fn num_arg(s: &str) -> Value {
    let n = js_number(s);
    if !n.is_finite() {
        Value::Null
    } else if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
        Value::from(n as i64)
    } else {
        Value::from(n)
    }
}

/// `String(new Error(msg))` - so zeigte das alte Frontend selbst erzeugte Fehler an.
pub fn js_error(msg: &str) -> String {
    format!("Error: {msg}")
}

/// Ersatz für `autoFocus` bei nachträglich eingeblendeten Feldern: fokussiert
/// das Element, sobald es im DOM hängt.
pub fn focus_on_mount<E>(node: NodeRef<E>)
where
    E: leptos::html::ElementType,
    E::Output: wasm_bindgen::JsCast + Clone + 'static,
    NodeRef<E>: Get<Value = Option<E::Output>>,
{
    Effect::new(move |_| {
        if let Some(el) = node.get() {
            use wasm_bindgen::JsCast;
            if let Some(h) = el.dyn_ref::<web_sys::HtmlElement>() {
                let _ = h.focus();
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn js_number_like_js() {
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number(" 12 "), 12.0);
        assert_eq!(js_number("-5"), -5.0);
        assert_eq!(js_number("1e3"), 1000.0);
        assert!(js_number("abc").is_nan());
        assert_eq!(num_arg("42"), Value::from(42));
        assert_eq!(num_arg("x"), Value::Null);
        assert_eq!(num_arg("1.5"), Value::from(1.5));
    }
}
