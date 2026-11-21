//! Übersetzungen (ersetzt i18next). Die JSON-Dateien sind 1:1 die alten
//! `src/i18n/locales/*.json`; Schlüssel werden wie bisher mit Punkten
//! adressiert (`t("nav.dashboard")`), Platzhalter als `{{name}}`.

use leptos::prelude::*;
use serde_json::Value;
use std::sync::OnceLock;

const STORAGE_KEY: &str = "m2manager-language";

static DE: OnceLock<Value> = OnceLock::new();
static EN: OnceLock<Value> = OnceLock::new();
static LANG: OnceLock<RwSignal<String>> = OnceLock::new();

fn de() -> &'static Value {
    DE.get_or_init(|| serde_json::from_str(include_str!("de.json")).expect("de.json"))
}
fn en() -> &'static Value {
    EN.get_or_init(|| serde_json::from_str(include_str!("en.json")).expect("en.json"))
}

/// Muss einmal vor dem Mounten aufgerufen werden (außerhalb eines Owners,
/// damit das Signal nie entsorgt wird).
pub fn init() {
    let stored = crate::util::storage::get(STORAGE_KEY);
    let _ = LANG.set(RwSignal::new(sanitize_language(stored.as_deref())));
}

/// Englisch ist noch nicht vollständig übersetzt und daher nicht wählbar;
/// eine früher gespeicherte Auswahl "en" fällt auf Deutsch zurück.
fn sanitize_language(stored: Option<&str>) -> String {
    match stored {
        Some("de") | None => "de".into(),
        Some(_) => "de".into(),
    }
}

/// Aktuelle Sprache ("de"/"en"), reaktiv.
pub fn language() -> String {
    LANG.get().map(|s| s.get()).unwrap_or_else(|| "de".into())
}

#[allow(dead_code)]
pub fn set_language(lang: &str) {
    if let Some(s) = LANG.get() {
        s.set(lang.to_string());
    }
    crate::util::storage::set(STORAGE_KEY, lang);
}

fn lookup<'a>(root: &'a Value, key: &str) -> Option<&'a str> {
    let mut cur = root;
    for part in key.split('.') {
        cur = cur.get(part)?;
    }
    cur.as_str()
}

/// Übersetzt `key` in der aktuellen Sprache (Fallback Englisch, dann der
/// Schlüssel selbst). Reaktiv: in einem `view!`/Closure aufgerufen, wird bei
/// Sprachwechsel automatisch neu gerendert.
pub fn t(key: &str) -> String {
    let lang = language();
    let primary = if lang == "en" { en() } else { de() };
    lookup(primary, key)
        .or_else(|| lookup(en(), key))
        .map(str::to_string)
        .unwrap_or_else(|| key.to_string())
}

/// Wie [`t`], ersetzt zusätzlich `{{name}}`-Platzhalter.
pub fn t_with(key: &str, vars: &[(&str, &str)]) -> String {
    let mut s = t(key);
    for (k, v) in vars {
        s = s.replace(&format!("{{{{{k}}}}}"), v);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(v: &Value, prefix: &str, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                for (k, v) in m {
                    keys(v, &format!("{prefix}{k}."), out);
                }
            }
            _ => out.push(prefix.trim_end_matches('.').to_string()),
        }
    }

    #[test]
    fn de_und_en_haben_dieselben_schluessel() {
        let (mut a, mut b) = (vec![], vec![]);
        keys(de(), "", &mut a);
        keys(en(), "", &mut b);
        a.sort();
        b.sort();
        assert_eq!(a, b);
    }

    #[test]
    fn gespeichertes_englisch_faellt_auf_deutsch() {
        assert_eq!(sanitize_language(Some("en")), "de");
        assert_eq!(sanitize_language(None), "de");
    }
}
