//! Zahlen-Eingabefeld mit demselben Verhalten wie ein kontrolliertes
//! `<input type="number">` in React plus `Number(e.target.value) || 0`.
//!
//! Wird von den Welt-Editoren (Regen, Refine, Cube) gemeinsam genutzt.
//! Werte bleiben bewusst `f64` (wie JS-`number`), damit auch z.B. eine
//! eingetippte Kommazahl unverändert ans Backend geht und dort genauso
//! abgelehnt wird wie bisher.

use leptos::html::Input;
use leptos::prelude::*;

/// JS `Number(string)` (ToNumber) für die Fälle, die in Eingabefeldern
/// vorkommen: Leerraum wird getrimmt, leer = 0, ungültig = NaN.
pub fn js_to_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return u64::from_str_radix(rest, radix).map(|v| v as f64).unwrap_or(f64::NAN);
        }
    }
    let lower = t.to_ascii_lowercase();
    if lower.contains("inf") || lower.contains("nan") {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(s) || 0` - NaN und -0 werden zu 0.
pub fn number_or_zero(s: &str) -> f64 {
    let v = js_to_number(s);
    if v.is_nan() || v == 0.0 {
        0.0
    } else {
        v
    }
}

/// JS `String(number)` für die üblichen Werte (ganze Zahlen ohne `.0`).
pub fn js_number_string(v: f64) -> String {
    if v.is_nan() {
        return "NaN".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if v == 0.0 {
        return "0".into();
    }
    if v.fract() == 0.0 && v.abs() < 1e21 {
        return format!("{}", v as i128);
    }
    format!("{v}")
}

/// Reacts Abgleich für `type="number"`: nur schreiben, wenn sich der
/// angezeigte Text numerisch vom Wert unterscheidet (sonst würde z.B. "1."
/// beim Tippen sofort zu "1"), und ein geleertes Feld zeigt wieder "0".
pub fn should_write_number(dom_value: &str, value: f64) -> bool {
    (value == 0.0 && dom_value.is_empty()) || js_to_number(dom_value) != value
}

/// `<input type="number" value={value} onChange={e => onChange(Number(e.target.value) || 0)}/>`
#[component]
pub fn NumInput(
    #[prop(into)] value: Signal<f64>,
    #[prop(into)] on_change: Callback<f64>,
    #[prop(into)] class: String,
    #[prop(optional)] min: Option<&'static str>,
    #[prop(optional)] max: Option<&'static str>,
) -> impl IntoView {
    let node: NodeRef<Input> = NodeRef::new();
    Effect::new(move |_| {
        let v = value.get();
        if let Some(el) = node.get() {
            if should_write_number(&el.value(), v) {
                el.set_value(&js_number_string(v));
            }
        }
    });
    view! {
        <input
            node_ref=node
            type="number"
            min=min
            max=max
            on:input=move |e| on_change.run(number_or_zero(&event_target_value(&e)))
            class=class
        />
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn to_number_like_js() {
        assert_eq!(js_to_number(""), 0.0);
        assert_eq!(js_to_number("  42 "), 42.0);
        assert_eq!(js_to_number("1.5"), 1.5);
        assert_eq!(js_to_number("1."), 1.0);
        assert_eq!(js_to_number(".5"), 0.5);
        assert_eq!(js_to_number("1e3"), 1000.0);
        assert_eq!(js_to_number("0x10"), 16.0);
        assert_eq!(js_to_number("-7"), -7.0);
        assert!(js_to_number("abc").is_nan());
        assert!(js_to_number("inf").is_nan());
        assert!(js_to_number("nan").is_nan());
        assert!(js_to_number("1,5").is_nan());
        assert_eq!(js_to_number("Infinity"), f64::INFINITY);
    }

    #[test]
    fn or_zero() {
        assert_eq!(number_or_zero("abc"), 0.0);
        assert_eq!(number_or_zero(""), 0.0);
        assert_eq!(number_or_zero("-0"), 0.0);
        assert!(number_or_zero("-0").is_sign_positive());
        assert_eq!(number_or_zero("12"), 12.0);
    }

    #[test]
    fn number_string_like_js() {
        assert_eq!(js_number_string(10.0), "10");
        assert_eq!(js_number_string(-3.0), "-3");
        assert_eq!(js_number_string(1.5), "1.5");
        assert_eq!(js_number_string(0.1), "0.1");
        assert_eq!(js_number_string(-0.0), "0");
        assert_eq!(js_number_string(2147483647.0), "2147483647");
    }

    #[test]
    fn react_number_sync() {
        assert!(!should_write_number("1.", 1.0));
        assert!(!should_write_number("5", 5.0));
        assert!(should_write_number("", 0.0));
        assert!(should_write_number("5", 6.0));
        assert!(should_write_number("abc", 0.0));
    }
}
