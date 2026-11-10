//! Suchen und Ersetzen im Quest-Editor (Strg+F), wie das Suchfeld von
//! CodeMirror: Groß-/Kleinschreibung, ganze Wörter, reguläre Ausdrücke.
//! Treffer sind Byte-Bereiche im vollen Text.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SearchOpts {
    pub case: bool,
    pub regex: bool,
    pub word: bool,
}

/// Mehr Treffer werden nicht hervorgehoben (große Dateien bleiben flüssig).
pub const MAX_MATCHES: usize = 5000;

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Liegt `text[s..e]` an Wortgrenzen?
fn at_word_bounds(text: &str, s: usize, e: usize) -> bool {
    let before = text[..s].chars().next_back();
    let after = text[e..].chars().next();
    let first = text[s..e].chars().next();
    let last = text[s..e].chars().next_back();
    !(before.is_some_and(is_word) && first.is_some_and(is_word)) && !(after.is_some_and(is_word) && last.is_some_and(is_word))
}

fn chars_eq(a: char, b: char, case: bool) -> bool {
    a == b || (!case && a.to_lowercase().eq(b.to_lowercase()))
}

fn find_plain(text: &str, query: &str, case: bool) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let q: Vec<char> = query.chars().collect();
    let mut i = 0;
    while i < text.len() && out.len() < MAX_MATCHES {
        let mut it = text[i..].char_indices();
        let mut end = None;
        for (k, &qc) in q.iter().enumerate() {
            match it.next() {
                Some((_, c)) if chars_eq(c, qc, case) => {
                    if k + 1 == q.len() {
                        end = Some(i + it.offset());
                    }
                }
                _ => break,
            }
        }
        if let Some(e) = end {
            out.push((i, e));
            i = e;
        } else {
            i += text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
        }
    }
    out
}

#[cfg(target_arch = "wasm32")]
fn find_regex(text: &str, query: &str, case: bool) -> Result<Vec<(usize, usize)>, String> {
    use wasm_bindgen::JsCast;
    let flags = if case { "gm" } else { "gmi" };
    // Gültigkeit wurde vorher mit `regex_valid` geprüft.
    let re = js_sys::RegExp::new(query, flags);
    let js = js_sys::JsString::from(text);
    // UTF-16 → Byte, monoton fortschreitend
    let mut u16_pos = 0u32;
    let mut byte_pos = 0usize;
    let mut to_byte = |target: u32| -> usize {
        for c in text[byte_pos..].chars() {
            if u16_pos >= target {
                break;
            }
            u16_pos += c.len_utf16() as u32;
            byte_pos += c.len_utf8();
        }
        byte_pos
    };
    let mut out = Vec::new();
    // matchAll überspringt leere Treffer selbst nicht - die werden hier
    // ausgelassen (wie bei CodeMirror).
    for m in js.match_all(&re).into_iter().flatten() {
        if out.len() >= MAX_MATCHES {
            break;
        }
        let m: js_sys::Array = m.unchecked_into();
        let idx = js_sys::Reflect::get(&m, &"index".into()).ok().and_then(|v| v.as_f64()).unwrap_or(0.0) as u32;
        let s: String = m.get(0).as_string().unwrap_or_default();
        let len = s.encode_utf16().count() as u32;
        if len == 0 {
            continue;
        }
        let a = to_byte(idx);
        let b = to_byte(idx + len);
        out.push((a, b));
    }
    Ok(out)
}

#[cfg(target_arch = "wasm32")]
fn regex_valid(query: &str) -> bool {
    // `RegExp`-Konstruktor über `Reflect.construct`, damit ein Syntaxfehler
    // als `Err` statt als Abbruch ankommt.
    let ctor: js_sys::Function = js_sys::RegExp::new("", "").constructor();
    js_sys::Reflect::construct(&ctor, &js_sys::Array::of1(&query.into())).is_ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn regex_valid(_query: &str) -> bool {
    true
}

#[cfg(not(target_arch = "wasm32"))]
fn find_regex(text: &str, query: &str, case: bool) -> Result<Vec<(usize, usize)>, String> {
    // Nativ (Tests) ohne Regex-Engine: wie eine Klartextsuche behandeln.
    Ok(find_plain(text, query, case))
}

/// Alle Treffer von `query` in `text`. Ein ungültiger regulärer Ausdruck
/// liefert `Err`.
pub fn find_all(text: &str, query: &str, o: SearchOpts) -> Result<Vec<(usize, usize)>, String> {
    if query.is_empty() {
        return Ok(Vec::new());
    }
    let found = if o.regex {
        if !regex_valid(query) {
            return Err("Ungültiger regulärer Ausdruck".into());
        }
        find_regex(text, query, o.case)?
    } else {
        find_plain(text, query, o.case)
    };
    Ok(if o.word { found.into_iter().filter(|&(s, e)| at_word_bounds(text, s, e)).collect() } else { found })
}

/// Ersatztext für einen Treffer; bei regulären Ausdrücken mit `$1`, `$&` …
pub fn replacement_for(matched: &str, query: &str, repl: &str, o: SearchOpts) -> String {
    #[cfg(target_arch = "wasm32")]
    if o.regex {
        let re = js_sys::RegExp::new(query, if o.case { "m" } else { "mi" });
        return js_sys::JsString::from(matched).replace_by_pattern(&re, repl).into();
    }
    let _ = (matched, query, o);
    repl.to_string()
}

/// Index des ersten Treffers ab `pos` (bzw. vor `pos` bei `back`), mit
/// Umlauf am Dateiende.
pub fn next_match(matches: &[(usize, usize)], pos: usize, back: bool) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    if back {
        Some(matches.iter().rposition(|m| m.1 <= pos).unwrap_or(matches.len() - 1))
    } else {
        Some(matches.iter().position(|m| m.0 >= pos).unwrap_or(0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const O: SearchOpts = SearchOpts { case: false, regex: false, word: false };

    #[test]
    fn plain_case_and_words() {
        let t = "Say say SAYING äÄ";
        assert_eq!(find_all(t, "say", O).unwrap(), vec![(0, 3), (4, 7), (8, 11)]);
        assert_eq!(find_all(t, "say", SearchOpts { case: true, ..O }).unwrap(), vec![(4, 7)]);
        assert_eq!(find_all(t, "say", SearchOpts { word: true, ..O }).unwrap(), vec![(0, 3), (4, 7)]);
        assert_eq!(find_all(t, "ä", O).unwrap(), vec![(15, 17), (17, 19)]);
        assert!(find_all(t, "", O).unwrap().is_empty());
        assert_eq!(find_all("aaa", "aa", O).unwrap(), vec![(0, 2)]);
    }

    #[test]
    fn navigation_wraps() {
        let m = [(2, 4), (10, 12)];
        assert_eq!(next_match(&m, 0, false), Some(0));
        assert_eq!(next_match(&m, 3, false), Some(1));
        assert_eq!(next_match(&m, 11, false), Some(0));
        assert_eq!(next_match(&m, 10, true), Some(0));
        assert_eq!(next_match(&m, 1, true), Some(1));
        assert_eq!(next_match(&[], 1, true), None);
    }
}
