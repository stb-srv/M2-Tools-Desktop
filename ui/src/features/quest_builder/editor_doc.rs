//! Text-Modell des Quest-Editors mit Code-Falten.
//!
//! Der volle Text liegt in [`Doc::text`]. Eine Faltung blendet ganze Zeilen
//! aus: sie wird als Byte-Bereich `(a, b)` gespeichert, wobei `text[a]` das
//! `\n` am Ende der Kopfzeile ist und `text[b]` das `\n` am Ende der letzten
//! ausgeblendeten Zeile (die schließende `end`-Zeile bleibt sichtbar). Die
//! Textarea zeigt die *Ansicht* (voller Text ohne `text[a..b]`); Eingaben in
//! der Ansicht werden hier auf den vollen Text zurückgerechnet.

use super::lua_highlight::{tokenize, TokenKind};

/// Startoffsets aller Zeilen.
pub fn line_starts(text: &str) -> Vec<usize> {
    std::iter::once(0).chain(text.match_indices('\n').map(|(i, _)| i + 1)).collect()
}

/// 0-basierte Zeile, in der `byte` liegt.
pub fn line_of(text: &str, byte: usize) -> usize {
    text.as_bytes()[..byte.min(text.len())].iter().filter(|&&c| c == b'\n').count()
}

/// Faltbare Blöcke als (Kopfzeile, schließende Zeile), 0-basiert, nach
/// Kopfzeile sortiert. Erkannt werden die Lua- und Quest-Blöcke
/// `function`/`if`/`do`/`repeat`/`begin` … `end`/`until` sowie
/// mehrzeilige Tabellen `{ … }`; Strings und Kommentare zählen nicht.
/// Ein Block ist nur faltbar, wenn zwischen Kopf- und Schlusszeile
/// mindestens eine Zeile liegt.
pub fn fold_ranges(text: &str) -> Vec<(usize, usize)> {
    let b = text.as_bytes();
    let mut stack: Vec<usize> = Vec::new(); // Offsets der Öffner
    let mut pairs: Vec<(usize, usize)> = Vec::new(); // (Öffner, Schließer)
    for (s, e, kind) in tokenize(text) {
        if matches!(kind, TokenKind::String | TokenKind::Comment | TokenKind::Number) {
            continue;
        }
        let mut i = s;
        while i < e {
            let c = b[i];
            if c.is_ascii_alphabetic() || c == b'_' {
                let mut j = i + 1;
                while j < e && (b[j].is_ascii_alphanumeric() || b[j] == b'_') {
                    j += 1;
                }
                // Teil eines Punkt-Namens (`pc.end`)? Dann kein Schlüsselwort.
                let dotted = (i > 0 && b[i - 1] == b'.') || b.get(j) == Some(&b'.');
                if !dotted {
                    match &text[i..j] {
                        "function" | "if" | "do" | "repeat" | "begin" => stack.push(i),
                        "end" | "until" => {
                            if let Some(o) = stack.pop() {
                                pairs.push((o, i));
                            }
                        }
                        _ => {}
                    }
                }
                i = j;
            } else {
                if c == b'{' {
                    stack.push(i);
                } else if c == b'}' {
                    if let Some(o) = stack.pop() {
                        pairs.push((o, i));
                    }
                }
                i += 1;
            }
        }
    }
    let mut out: Vec<(usize, usize)> = pairs
        .into_iter()
        .map(|(o, c)| (line_of(text, o), line_of(text, c)))
        .filter(|(s, e)| e >= &(s + 2))
        .collect();
    // Pro Kopfzeile nur den größten Block.
    out.sort_by(|x, y| x.0.cmp(&y.0).then(y.1.cmp(&x.1)));
    out.dedup_by_key(|x| x.0);
    out
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Doc {
    pub text: String,
    /// Gefaltete Bereiche `(a, b)`, sortiert nach `a`. Verschachtelte
    /// Faltungen bleiben erhalten (beim Aufklappen der äußeren wieder sichtbar).
    pub folds: Vec<(usize, usize)>,
}

impl Doc {
    pub fn new(text: String) -> Self {
        Self { text, folds: Vec::new() }
    }

    /// Tatsächlich ausgeblendete Bereiche (verschachtelte zusammengefasst).
    pub fn hidden(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = Vec::new();
        for &(a, b) in &self.folds {
            match out.last_mut() {
                Some(last) if a < last.1 => last.1 = last.1.max(b),
                _ => out.push((a, b)),
            }
        }
        out
    }

    pub fn view(&self) -> String {
        let mut out = String::with_capacity(self.text.len());
        let mut pos = 0;
        for (a, b) in self.hidden() {
            out.push_str(&self.text[pos..a]);
            pos = b;
        }
        out.push_str(&self.text[pos..]);
        out
    }

    /// Voller Offset → Ansicht-Offset; liegt er in einem ausgeblendeten
    /// Bereich, landet er am Ende der Kopfzeile.
    pub fn to_view(&self, full: usize) -> usize {
        let mut shift = 0;
        for (a, b) in self.hidden() {
            if full > b {
                shift += b - a;
            } else if full > a {
                return a - shift;
            } else {
                break;
            }
        }
        full - shift
    }

    /// Ansicht-Offset → voller Offset. Am Ende einer gefalteten Kopfzeile
    /// liegt der Punkt vor (`right = false`) oder hinter (`right = true`)
    /// dem ausgeblendeten Bereich.
    pub fn to_full(&self, view: usize, right: bool) -> usize {
        let mut f = view;
        for (a, b) in self.hidden() {
            if a < f || (right && a == f) {
                f += b - a;
            } else {
                break;
            }
        }
        f
    }

    /// Ersetzt `text[start..end]` durch `ins` und passt die Faltungen an:
    /// Faltungen davor bleiben, dahinter werden verschoben, überlappende
    /// werden aufgelöst. Gibt den Offset hinter dem eingefügten Text zurück.
    pub fn apply(&mut self, start: usize, end: usize, ins: &str) -> usize {
        let delta = ins.len() as isize - (end - start) as isize;
        self.text.replace_range(start..end, ins);
        let text = &self.text;
        let ok = |a: usize, b: usize| {
            a < b && text.as_bytes().get(a) == Some(&b'\n') && text.as_bytes().get(b) == Some(&b'\n')
        };
        // Reine Einfügung genau am Kopfzeilenende landet vor dem Bereich.
        let after = |a: usize| if start == end { a >= start } else { a >= end };
        self.folds = self
            .folds
            .iter()
            .filter_map(|&(a, b)| {
                if b <= start {
                    Some((a, b))
                } else if after(a) {
                    Some(((a as isize + delta) as usize, (b as isize + delta) as usize))
                } else {
                    None
                }
            })
            .filter(|&(a, b)| ok(a, b))
            .collect();
        start + ins.len()
    }

    /// Überträgt eine Eingabe in der Textarea (alte → neue Ansicht) auf den
    /// vollen Text. `caret_before` (Ansicht-Byte-Offset des Auswahlbeginns
    /// vor der Eingabe) löst Mehrdeutigkeiten bei gleichen Zeichen auf.
    /// Gibt den neuen vollen Offset hinter der Änderung zurück.
    pub fn apply_view_edit(&mut self, old_view: &str, new_view: &str, caret_before: Option<usize>) -> usize {
        let (ob, nb) = (old_view.as_bytes(), new_view.as_bytes());
        let mut p = ob.iter().zip(nb).take_while(|(x, y)| x == y).count();
        if let Some(c) = caret_before {
            p = p.min(c);
        }
        while !old_view.is_char_boundary(p) || !new_view.is_char_boundary(p) {
            p -= 1;
        }
        let max_s = (ob.len() - p).min(nb.len() - p);
        let mut s = ob.iter().rev().zip(nb.iter().rev()).take(max_s).take_while(|(x, y)| x == y).count();
        while !old_view.is_char_boundary(ob.len() - s) || !new_view.is_char_boundary(nb.len() - s) {
            s -= 1;
        }
        let q = ob.len() - s;
        let ins = &new_view[p..nb.len() - s];
        let (fs, fe) = if p == q {
            let f = self.to_full(p, false);
            (f, f)
        } else {
            (self.to_full(p, true), self.to_full(q, false))
        };
        self.apply(fs, fe.max(fs), ins)
    }

    /// Bereich `(a, b)` für den Block mit Kopfzeile `start` und
    /// Schlusszeile `close`.
    fn range_for(&self, start: usize, close: usize) -> Option<(usize, usize)> {
        let ls = line_starts(&self.text);
        let a = ls.get(start + 1)?.checked_sub(1)?;
        let b = ls.get(close)?.checked_sub(1)?;
        (a < b).then_some((a, b))
    }

    pub fn is_folded_line(&self, line: usize) -> bool {
        let ls = line_starts(&self.text);
        let Some(a) = ls.get(line + 1).map(|x| x - 1) else { return false };
        self.folds.iter().any(|f| f.0 == a)
    }

    /// Faltet den Block, der in Zeile `line` beginnt. `false`, wenn dort
    /// keiner beginnt.
    pub fn fold_line(&mut self, line: usize, ranges: &[(usize, usize)]) -> bool {
        let Some(&(s, c)) = ranges.iter().find(|r| r.0 == line) else { return false };
        let Some(r) = self.range_for(s, c) else { return false };
        if !self.folds.contains(&r) {
            self.folds.push(r);
            self.folds.sort();
        }
        true
    }

    pub fn unfold_line(&mut self, line: usize) -> bool {
        let ls = line_starts(&self.text);
        let Some(a) = ls.get(line + 1).map(|x| x - 1) else { return false };
        let n = self.folds.len();
        self.folds.retain(|f| f.0 != a);
        n != self.folds.len()
    }

    pub fn toggle_line(&mut self, line: usize, ranges: &[(usize, usize)]) {
        if !self.unfold_line(line) {
            self.fold_line(line, ranges);
        }
    }

    /// Innerster faltbarer Block, der Zeile `line` enthält (Kopfzeile).
    pub fn enclosing_block(line: usize, ranges: &[(usize, usize)]) -> Option<usize> {
        ranges.iter().filter(|r| r.0 <= line && line <= r.1).max_by_key(|r| r.0).map(|r| r.0)
    }

    pub fn fold_all(&mut self, ranges: &[(usize, usize)]) {
        for &(s, c) in ranges {
            if let Some(r) = self.range_for(s, c) {
                if !self.folds.contains(&r) {
                    self.folds.push(r);
                }
            }
        }
        self.folds.sort();
    }

    pub fn unfold_all(&mut self) {
        self.folds.clear();
    }

    /// Klappt alle Faltungen auf, die etwas aus `start..end` verbergen.
    pub fn reveal(&mut self, start: usize, end: usize) -> bool {
        let n = self.folds.len();
        // Ausgeblendet sind die Bytes `a..b`; eine leere Position `x` ist
        // verborgen, wenn `a < x <= b`.
        self.folds.retain(|&(a, b)| if start == end { !(a < start && start <= b) } else { !(start < b && end > a) });
        n != self.folds.len()
    }

    /// Ansicht-Zeilen: für jede sichtbare Zeile ihre Zeile im vollen Text.
    pub fn visible_lines(&self) -> Vec<usize> {
        let hidden: Vec<(usize, usize)> =
            self.hidden().into_iter().map(|(a, b)| (line_of(&self.text, a) + 1, line_of(&self.text, b))).collect();
        let total = line_starts(&self.text).len();
        let mut out = Vec::with_capacity(total);
        let mut h = hidden.iter().peekable();
        let mut l = 0;
        while l < total {
            if let Some(&&(s, e)) = h.peek() {
                if l == s {
                    l = e + 1;
                    h.next();
                    continue;
                }
            }
            out.push(l);
            l += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const Q: &str = "quest t begin\n\tstate start begin\n\t\twhen login begin\n\t\t\tsay(\"end\")\n\t\tend\n\tend\nend";

    #[test]
    fn finds_blocks() {
        assert_eq!(fold_ranges(Q), vec![(0, 6), (1, 5), (2, 4)]);
        // eine Innenzeile reicht
        let t = "when x begin\n\ta()\nend";
        assert_eq!(fold_ranges(t), vec![(0, 2)]);
        // if/elseif teilt sich ein end, for … do zählt einmal
        let t = "if a then\n x\nelseif b then\n y\nend\nfor i=1,2 do\n z\nend";
        assert_eq!(fold_ranges(t), vec![(0, 4), (5, 7)]);
        // Kommentar/String und Punktnamen zählen nicht
        let t = "function f() -- end\n x = \"end\"\n pc.end()\nend";
        assert_eq!(fold_ranges(t), vec![(0, 3)]);
        let t = "local t = {\n 1,\n 2,\n}";
        assert_eq!(fold_ranges(t), vec![(0, 3)]);
    }

    fn folded() -> Doc {
        let mut d = Doc::new(Q.to_string());
        let r = fold_ranges(Q);
        assert!(d.fold_line(1, &r));
        d
    }

    #[test]
    fn view_hides_lines() {
        let d = folded();
        assert_eq!(d.view(), "quest t begin\n\tstate start begin\n\tend\nend");
        assert_eq!(d.visible_lines(), vec![0, 1, 5, 6]);
        assert!(d.is_folded_line(1));
        // Offsets hin und zurück
        let end_hdr = Q.find("begin\n\t\twhen").unwrap() + 5;
        assert_eq!(d.to_view(end_hdr), end_hdr);
        assert_eq!(d.to_full(end_hdr, false), end_hdr);
        let close = Q.rfind("\tend\nend").unwrap();
        assert_eq!(d.to_full(end_hdr, true), close - 1);
        assert_eq!(d.to_view(close), end_hdr + 1);
        assert_eq!(d.to_view(end_hdr + 3), end_hdr);
    }

    fn edit(d: &mut Doc, f: impl Fn(&str) -> String, caret: Option<usize>) {
        let old = d.view();
        let new = f(&old);
        d.apply_view_edit(&old, &new, caret);
    }

    #[test]
    fn typing_at_folded_header_stays_on_header() {
        let mut d = folded();
        let at = "quest t begin\n\tstate start begin".len();
        edit(&mut d, |v| format!("{}X{}", &v[..at], &v[at..]), Some(at));
        assert!(d.text.contains("start beginX\n\t\twhen"));
        assert!(d.is_folded_line(1));
        assert_eq!(d.view(), "quest t begin\n\tstate start beginX\n\tend\nend");
    }

    #[test]
    fn enter_at_folded_header_moves_fold_down() {
        let mut d = folded();
        let at = "quest t begin\n\tstate start begin".len();
        edit(&mut d, |v| format!("{}\n{}", &v[..at], &v[at..]), Some(at));
        assert!(d.text.contains("start begin\n\n\t\twhen"));
        assert_eq!(d.view(), "quest t begin\n\tstate start begin\n\n\tend\nend");
        assert!(d.is_folded_line(2));
    }

    #[test]
    fn backspace_after_fold_joins_last_hidden_line_and_unfolds() {
        let mut d = folded();
        let at = "quest t begin\n\tstate start begin\n".len();
        edit(&mut d, |v| format!("{}{}", &v[..at - 1], &v[at..]), Some(at));
        assert!(d.text.contains("\t\tend\tend\nend"));
        assert!(d.folds.is_empty());
        assert_eq!(d.view(), d.text);
    }

    #[test]
    fn deleting_across_fold_removes_hidden_lines() {
        let mut d = folded();
        // "state…begin\n\tend" komplett markiert und gelöscht
        let v = d.view();
        let s = v.find("\tstate").unwrap();
        let e = v.find("\tend\nend").unwrap() + 4;
        edit(&mut d, |v| format!("{}{}", &v[..s], &v[e..]), Some(s));
        assert_eq!(d.text, "quest t begin\n\nend");
        assert!(d.folds.is_empty());
    }

    #[test]
    fn edits_before_and_after_shift_fold() {
        let mut d = folded();
        edit(&mut d, |v| format!("-- x\n{v}"), Some(0));
        assert_eq!(d.view(), "-- x\nquest t begin\n\tstate start begin\n\tend\nend");
        assert!(d.is_folded_line(2));
        let len = d.view().len();
        edit(&mut d, |v| format!("{v}\n"), Some(len));
        assert!(d.is_folded_line(2));
        assert!(d.text.ends_with("end\n"));
    }

    #[test]
    fn nested_and_reveal() {
        let mut d = Doc::new(Q.to_string());
        let r = fold_ranges(Q);
        d.fold_line(1, &r);
        d.fold_line(0, &r);
        assert_eq!(d.view(), "quest t begin\nend");
        d.unfold_line(0);
        assert_eq!(d.view(), "quest t begin\n\tstate start begin\n\tend\nend");
        let say = Q.find("say").unwrap();
        assert!(d.reveal(say, say + 3));
        assert_eq!(d.view(), Q);
        d.fold_all(&r);
        assert_eq!(d.view(), "quest t begin\nend");
        assert_eq!(Doc::enclosing_block(3, &r), Some(2));
        assert_eq!(Doc::enclosing_block(5, &r), Some(1));
    }

    #[test]
    fn utf8_safe() {
        let mut d = Doc::new("äö\nü".into());
        edit(&mut d, |_| "äöx\nü".into(), None);
        assert_eq!(d.text, "äöx\nü");
        edit(&mut d, |_| "ä\nü".into(), None);
        assert_eq!(d.text, "ä\nü");
    }
}
