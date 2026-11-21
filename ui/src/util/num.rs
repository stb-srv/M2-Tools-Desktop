//! Gemeinsame Zahl-Helfer, die JavaScripts `Number(s)` nachbilden.

/// Wie `Number(s)` ohne Zahlpräfixe: getrimmt, leer = 0, `Infinity` erlaubt,
/// alles andere Ungültige (auch `0x10`) = NaN.
pub fn js_number_basic(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
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

/// Wie `Number(s)` vollständig: zusätzlich `0x`/`0o`/`0b`-Präfixe.
pub fn js_number(s: &str) -> f64 {
    let t = s.trim();
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(rest) = t.strip_prefix(prefix) {
            return u64::from_str_radix(rest, radix).map(|v| v as f64).unwrap_or(f64::NAN);
        }
    }
    js_number_basic(t)
}

/// Wie `Number(x ?? 0)`.
pub fn js_number_opt(s: Option<&str>) -> f64 {
    s.map_or(0.0, js_number_basic)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic() {
        assert_eq!(js_number_basic(""), 0.0);
        assert_eq!(js_number_basic("  12.5 "), 12.5);
        assert_eq!(js_number_basic("1e3"), 1000.0);
        assert!(js_number_basic("abc").is_nan());
        assert!(js_number_basic("inf").is_nan());
        assert!(js_number_basic("0x10").is_nan());
        assert_eq!(js_number_basic("-Infinity"), f64::NEG_INFINITY);
    }

    #[test]
    fn full() {
        assert_eq!(js_number("0x10"), 16.0);
        assert_eq!(js_number("0b101"), 5.0);
        assert_eq!(js_number("0o17"), 15.0);
        assert!(js_number("0xzz").is_nan());
        assert_eq!(js_number("7"), 7.0);
        assert_eq!(js_number_opt(None), 0.0);
        assert_eq!(js_number_opt(Some("3")), 3.0);
    }
}
