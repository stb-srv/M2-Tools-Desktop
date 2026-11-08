//! Kleine Helfer, die Server-Steuerung, Bauen & Einspielen und
//! Tag/Nacht & Schnee gemeinsam nutzen (Live-Ausgabe per Tauri-Event,
//! automatisches Mitscrollen, `Number(...)`-Semantik, `sleep`, Zwischenablage).

use leptos::prelude::*;
use leptos::task::spawn_local;
use serde::de::DeserializeOwned;

/// Abonniert ein Tauri-Event, solange die aufrufende Komponente gemountet
/// ist (wie `useEffect(() => { const u = listen(..); return () => u.then(f => f()) }, [])`).
pub fn listen_while_mounted<T: DeserializeOwned + 'static>(event: &'static str, handler: impl Fn(T) + 'static) {
    let slot = StoredValue::new_local(None::<crate::tauri::Unlisten>);
    spawn_local(async move {
        if let Ok(u) = crate::tauri::listen::<T>(event, handler).await {
            if slot.is_disposed() {
                // Komponente wurde schon wieder entfernt, bevor das Abo stand.
                u.unlisten();
            } else {
                slot.set_value(Some(u));
            }
        }
    });
    on_cleanup(move || {
        slot.with_value(|u| {
            if let Some(u) = u {
                u.unlisten();
            }
        });
    });
}

/// Scrollt ein `<pre>` bei jeder Änderung von `track` ganz nach unten.
pub fn auto_scroll(node: NodeRef<leptos::html::Pre>, track: impl Fn() + 'static) {
    Effect::new(move |_| {
        track();
        if let Some(el) = node.get() {
            el.set_scroll_top(el.scroll_height());
        }
    });
}

/// Wartet `ms` Millisekunden (wie `new Promise(r => setTimeout(r, ms))`).
pub async fn sleep_ms(ms: f64) {
    let promise = js_sys::Promise::new(&mut |resolve, _reject| {
        if let Some(w) = web_sys::window() {
            let _ = w.set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms.min(i32::MAX as f64) as i32);
        }
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}

/// Kopiert Text in die Zwischenablage (`navigator.clipboard.writeText`).
pub async fn copy_to_clipboard(text: &str) {
    if let Some(w) = web_sys::window() {
        let _ = wasm_bindgen_futures::JsFuture::from(w.navigator().clipboard().write_text(text)).await;
    }
}

/// JavaScripts `Number(s)` für Texteingaben: getrimmt, leer = 0, sonst die
/// Zahl oder `NaN`.
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
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return u64::from_str_radix(rest, radix).map(|v| v as f64).unwrap_or(f64::NAN);
        }
    }
    if !t.chars().all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | 'e' | 'E')) {
        return f64::NAN;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// Formatiert eine Zahl wie JavaScripts `String(n)` für die üblichen Fälle.
pub fn js_number_string(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n > 0.0 {
            "Infinity".into()
        } else {
            "-Infinity".into()
        }
    } else if n == 0.0 {
        "0".into()
    } else {
        format!("{n}")
    }
}

/// Zahl als JSON-Wert wie `JSON.stringify` (NaN/Infinity -> null, ganze
/// Zahlen ohne Nachkommastellen).
pub fn js_number_json(n: f64) -> serde_json::Value {
    if !n.is_finite() {
        serde_json::Value::Null
    } else if n.fract() == 0.0 && n.abs() < 9.007_199_254_740_992e15 {
        serde_json::Value::from(n as i64)
    } else {
        serde_json::Value::from(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_like_js() {
        assert_eq!(js_number(""), 0.0);
        assert_eq!(js_number("  "), 0.0);
        assert_eq!(js_number(" 5 "), 5.0);
        assert_eq!(js_number("1.5"), 1.5);
        assert_eq!(js_number("0x10"), 16.0);
        assert!(js_number("abc").is_nan());
        assert!(js_number("inf").is_nan());
        assert!(js_number("5s").is_nan());
        assert_eq!(js_number("-3"), -3.0);
    }

    #[test]
    fn number_string_like_js() {
        assert_eq!(js_number_string(5.0), "5");
        assert_eq!(js_number_string(1.5), "1.5");
        assert_eq!(js_number_string(-0.0), "0");
    }

    #[test]
    fn number_json() {
        assert_eq!(js_number_json(200.0), serde_json::json!(200));
        assert_eq!(js_number_json(1.5), serde_json::json!(1.5));
        assert_eq!(js_number_json(f64::NAN), serde_json::Value::Null);
    }
}
