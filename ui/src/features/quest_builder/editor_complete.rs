//! Autovervollständigung im Quest-Editor: Lua-Schlüsselwörter und
//! -Funktionen, Quest-Schlüsselwörter, die verifizierten Quest-Funktionen
//! des Servers und Wörter aus der aktuellen Datei.

use super::lua_highlight::{BUILTINS, KEYWORDS};
use super::quest_functions::quest_functions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompKind {
    Keyword,
    Function,
    QuestFunction,
    Word,
}

impl CompKind {
    pub fn label(self) -> &'static str {
        match self {
            CompKind::Keyword => "Schlüsselwort",
            CompKind::Function => "Lua",
            CompKind::QuestFunction => "Quest-Funktion",
            CompKind::Word => "Wort",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub kind: CompKind,
}

/// Höchstzahl angezeigter Vorschläge.
pub const MAX_ITEMS: usize = 50;

const QUEST_KEYWORDS: &[&str] = &["quest", "state", "when", "begin", "with"];

fn ident_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'.'
}

/// Beginn des (Punkt-)Bezeichners direkt vor `caret`.
pub fn word_start(text: &str, caret: usize) -> usize {
    let b = text.as_bytes();
    let mut s = caret.min(b.len());
    while s > 0 && ident_byte(b[s - 1]) {
        s -= 1;
    }
    s
}

/// Bezeichner der Datei (mind. 3 Zeichen, ohne Zahlen am Anfang),
/// außer dem gerade getippten bei `skip`.
pub fn doc_words(text: &str, skip: (usize, usize)) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = std::collections::BTreeSet::new();
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_alphabetic() || b[i] == b'_' {
            let s = i;
            while i < b.len() && ident_byte(b[i]) {
                i += 1;
            }
            let w = text[s..i].trim_end_matches('.');
            if w.len() >= 3 && (s, i) != skip && !(s <= skip.0 && skip.1 <= i && skip.0 < skip.1) {
                out.insert(w.to_string());
            }
        } else if b[i].is_ascii_digit() {
            while i < b.len() && ident_byte(b[i]) {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    out.into_iter().collect()
}

/// Vorschläge für `prefix`: zuerst Präfix-Treffer (ohne Groß-/Klein-
/// schreibung), dann Teilstring-Treffer, jeweils kurze zuerst.
pub fn complete(prefix: &str, words: &[String]) -> Vec<Completion> {
    let p = prefix.to_lowercase();
    let mut seen = std::collections::HashSet::new();
    let mut prefixed = Vec::new();
    let mut contained = Vec::new();
    let sources = KEYWORDS
        .iter()
        .chain(QUEST_KEYWORDS)
        .map(|k| (*k, CompKind::Keyword))
        .chain(quest_functions().iter().map(|f| (f.as_str(), CompKind::QuestFunction)))
        .chain(BUILTINS.iter().map(|f| (*f, CompKind::Function)))
        .chain(words.iter().map(|w| (w.as_str(), CompKind::Word)));
    for (label, kind) in sources {
        if label == prefix || !seen.insert(label) {
            continue;
        }
        let l = label.to_lowercase();
        if l.starts_with(&p) {
            prefixed.push(Completion { label: label.to_string(), kind });
        } else if !p.is_empty() && l.contains(&p) {
            contained.push(Completion { label: label.to_string(), kind });
        }
    }
    let by_len = |a: &Completion, b: &Completion| a.label.len().cmp(&b.label.len()).then(a.label.cmp(&b.label));
    prefixed.sort_by(by_len);
    contained.sort_by(by_len);
    prefixed.into_iter().chain(contained).take(MAX_ITEMS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_start_includes_dots() {
        let t = "if pc.get_le";
        assert_eq!(word_start(t, t.len()), 3);
        assert_eq!(word_start("x(", 2), 2);
    }

    #[test]
    fn completes_quest_functions_and_words() {
        let c = complete("pc.get_lev", &[]);
        assert_eq!(c[0].label, "pc.get_level");
        assert_eq!(c[0].kind, CompKind::QuestFunction);
        let c = complete("beg", &[]);
        assert_eq!(c[0], Completion { label: "begin".into(), kind: CompKind::Keyword });
        let words = doc_words("local my_counter = 1\nmy_c", (21, 25));
        assert_eq!(words, vec!["local".to_string(), "my_counter".to_string()]);
        let c = complete("my_c", &words);
        assert_eq!(c[0].label, "my_counter");
        assert_eq!(c[0].kind, CompKind::Word);
        // Teilstring nach Präfix
        let c = complete("get_level", &[]);
        assert!(c.iter().any(|x| x.label == "pc.get_level"));
        assert!(complete("zzzzqqq", &[]).is_empty());
    }
}
