//! Nachbildung von JavaScripts `Number(text)` und der Zahl-zu-Text-Umwandlung
//! (`${n}` / `String(n)`), damit die Lua-Vorlagen exakt denselben Text wie
//! die frühere TS-Fassung erzeugen (z.B. `Number("") === 0`,
//! `Number("abc")` = NaN, `Number(" 12 ") === 12`).

/// JavaScripts Leerraum-Definition für `Number()`/`trim()`.
pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\u{9}' | '\u{a}' | '\u{b}' | '\u{c}' | '\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

/// `Number(text)` - NaN, wenn der Text keine gültige Zahl ist.
pub fn number(text: &str) -> f64 {
    let s = text.trim_matches(is_js_space);
    if s.is_empty() {
        return 0.0;
    }
    let radix = |p: &str, r: u32| -> Option<f64> {
        let digits = s.strip_prefix(p).or_else(|| s.strip_prefix(&p.to_uppercase()))?;
        if digits.is_empty() || !digits.chars().all(|c| c.is_digit(r)) {
            return Some(f64::NAN);
        }
        Some(digits.chars().fold(0.0, |acc, c| acc * r as f64 + c.to_digit(r).unwrap() as f64))
    };
    if let Some(v) = radix("0x", 16).or_else(|| radix("0o", 8)).or_else(|| radix("0b", 2)) {
        return v;
    }
    match s {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    // Rust akzeptiert zusätzlich "inf"/"nan"/"infinity" - JS nicht.
    let body = s.trim_start_matches(['+', '-']);
    if body.len() + 1 < s.len() || !body.chars().all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-')) {
        return f64::NAN;
    }
    s.parse::<f64>().unwrap_or(f64::NAN)
}

/// `Number(text) || 0`
pub fn number_or0(text: &str) -> f64 {
    let n = number(text);
    if n.is_nan() || n == 0.0 {
        0.0
    } else {
        n
    }
}

/// `Number(text) || fallback`
pub fn number_or(text: &str, fallback: f64) -> f64 {
    let n = number(text);
    if n.is_nan() || n == 0.0 {
        fallback
    } else {
        n
    }
}

/// `String(n)` wie in JavaScript.
pub fn num_str(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if n == 0.0 {
        return "0".into();
    }
    let abs = n.abs();
    if abs < 1e21 && n.fract() == 0.0 {
        return format!("{n:.0}");
    }
    if (1e-6..1e21).contains(&abs) {
        return format!("{n}");
    }
    let e = format!("{n:e}");
    match e.split_once('e') {
        Some((m, exp)) if !exp.starts_with('-') => format!("{m}e+{exp}"),
        _ => e,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_like_js() {
        assert_eq!(number(""), 0.0);
        assert_eq!(number("  12 "), 12.0);
        assert!(number("abc").is_nan());
        assert!(number("inf").is_nan());
        assert!(number("1 2").is_nan());
        assert_eq!(number("0x10"), 16.0);
        assert_eq!(number("-3.5"), -3.5);
        assert_eq!(number("1e3"), 1000.0);
        assert_eq!(number_or0("x"), 0.0);
        assert_eq!(number_or("0", 1.0), 1.0);
    }

    #[test]
    fn to_string_like_js() {
        assert_eq!(num_str(3.0), "3");
        assert_eq!(num_str(-0.0), "0");
        assert_eq!(num_str(1.5), "1.5");
        assert_eq!(num_str(1e21), "1e+21");
        assert_eq!(num_str(1e-7), "1e-7");
        assert_eq!(num_str(345600.0), "345600");
    }
}
