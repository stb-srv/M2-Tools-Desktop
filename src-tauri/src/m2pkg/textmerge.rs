//! Zeilenweises Einfügen in `item_list.txt` / `item_names.txt` (cp1252) und
//! das exakte Gegenstück zum Rückgängigmachen. Gearbeitet wird auf Bytes,
//! damit nichts in der Datei außer den betroffenen Zeilen verändert wird.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Eine ersetzte Zeile (für das Rückgängigmachen), cp1252 als Text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReplacedLine {
    pub vnum: u32,
    pub old: String,
}

#[derive(Debug, Default)]
pub struct MergeResult {
    pub bytes: Vec<u8>,
    pub added: Vec<u32>,
    pub replaced: Vec<ReplacedLine>,
    pub kept: Vec<u32>,
}

struct Split {
    lines: Vec<Vec<u8>>,
    eol: &'static [u8],
    trailing_eol: bool,
}

fn split_lines(data: &[u8]) -> Split {
    let eol: &'static [u8] = if data.windows(2).any(|w| w == b"\r\n") {
        b"\r\n"
    } else if data.contains(&b'\n') {
        b"\n"
    } else {
        b"\r\n"
    };
    let trailing_eol = data.is_empty() || data.ends_with(b"\n");
    let mut lines: Vec<Vec<u8>> = data
        .split(|b| *b == b'\n')
        .map(|l| l.strip_suffix(b"\r").unwrap_or(l).to_vec())
        .collect();
    if trailing_eol {
        lines.pop();
    }
    Split { lines, eol, trailing_eol }
}

fn join_lines(s: &Split) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, l) in s.lines.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(s.eol);
        }
        out.extend_from_slice(l);
    }
    if s.trailing_eol && !s.lines.is_empty() {
        out.extend_from_slice(s.eol);
    }
    out
}

/// Vnum am Zeilenanfang (bis zum ersten Tabulator).
pub fn line_vnum(line: &[u8]) -> Option<u32> {
    let end = line.iter().position(|b| *b == b'\t').unwrap_or(line.len());
    std::str::from_utf8(&line[..end]).ok()?.trim().parse().ok()
}

fn cp1252_decode(bytes: &[u8]) -> String {
    encoding_rs::WINDOWS_1252.decode(bytes).0.into_owned()
}

fn cp1252_encode(text: &str) -> Vec<u8> {
    encoding_rs::WINDOWS_1252.encode(text).0.into_owned()
}

/// Zeilen eines Pakets (`client/item_list.txt`, `client/item_names.txt`):
/// ohne Kopfzeile und leere Zeilen, nur die gewählten Vnum.
pub fn package_lines(data: &[u8], wanted: &dyn Fn(u32) -> bool) -> Vec<(u32, Vec<u8>)> {
    split_lines(data)
        .lines
        .into_iter()
        .filter_map(|l| line_vnum(&l).filter(|v| wanted(*v)).map(|v| (v, l)))
        .collect()
}

/// Fügt `new_lines` in `existing` ein. Vorhandene Vnum werden je nach
/// `overwrite` ersetzt oder in Ruhe gelassen, neue hinten angehängt.
pub fn merge_lines(existing: &[u8], new_lines: &[(u32, Vec<u8>)], overwrite: bool) -> MergeResult {
    let mut s = split_lines(existing);
    let mut index: HashMap<u32, usize> = HashMap::new();
    for (i, l) in s.lines.iter().enumerate() {
        if let Some(v) = line_vnum(l) {
            index.entry(v).or_insert(i);
        }
    }
    let mut res = MergeResult::default();
    for (vnum, line) in new_lines {
        match index.get(vnum) {
            Some(&i) => {
                if s.lines[i] == *line {
                    res.kept.push(*vnum);
                } else if overwrite {
                    res.replaced.push(ReplacedLine { vnum: *vnum, old: cp1252_decode(&s.lines[i]) });
                    s.lines[i] = line.clone();
                } else {
                    res.kept.push(*vnum);
                }
            }
            None => {
                index.insert(*vnum, s.lines.len());
                s.lines.push(line.clone());
                res.added.push(*vnum);
            }
        }
    }
    // Hatte die Datei kein Zeilenende am Schluss und es kam nichts dazu, bleibt es so.
    if !res.added.is_empty() && s.lines.len() == res.added.len() {
        s.trailing_eol = true;
    }
    res.bytes = join_lines(&s);
    res
}

/// Macht `merge_lines` rückgängig: hinzugefügte Zeilen entfernen, ersetzte
/// wiederherstellen. Alles andere in der Datei bleibt, wie es jetzt ist.
pub fn unmerge_lines(current: &[u8], added: &[u32], replaced: &[ReplacedLine]) -> Vec<u8> {
    let mut s = split_lines(current);
    let restore: HashMap<u32, &ReplacedLine> = replaced.iter().map(|r| (r.vnum, r)).collect();
    let mut out = Vec::with_capacity(s.lines.len());
    let mut restored = std::collections::HashSet::new();
    for l in s.lines.drain(..) {
        match line_vnum(&l) {
            Some(v) if added.contains(&v) => {}
            Some(v) if restore.contains_key(&v) && restored.insert(v) => out.push(cp1252_encode(&restore[&v].old)),
            _ => out.push(l),
        }
    }
    s.lines = out;
    join_lines(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn append_keeps_crlf_and_trailing_eol() {
        let existing = b"1\tWEAPON\ta.tga\r\n2\tARMOR\tb.tga\r\n";
        let r = merge_lines(existing, &[(3, b"3\tETC\tc.tga".to_vec())], true);
        assert_eq!(r.bytes, b"1\tWEAPON\ta.tga\r\n2\tARMOR\tb.tga\r\n3\tETC\tc.tga\r\n");
        assert_eq!(r.added, vec![3]);
    }

    #[test]
    fn no_trailing_eol_is_preserved() {
        let existing = b"1\ta\r\n2\tb";
        let r = merge_lines(existing, &[(3, b"3\tc".to_vec())], true);
        assert_eq!(r.bytes, b"1\ta\r\n2\tb\r\n3\tc");
    }

    #[test]
    fn existing_vnum_overwrite_or_keep() {
        let existing = b"1\told\r\n2\tb\r\n";
        let new = [(1, b"1\tnew".to_vec())];
        let over = merge_lines(existing, &new, true);
        assert_eq!(over.bytes, b"1\tnew\r\n2\tb\r\n");
        assert_eq!(over.replaced, vec![ReplacedLine { vnum: 1, old: "1\told".into() }]);
        let keep = merge_lines(existing, &new, false);
        assert_eq!(keep.bytes, existing);
        assert_eq!(keep.kept, vec![1]);
    }

    #[test]
    fn identical_line_counts_as_kept_not_replaced() {
        let existing = b"1\tsame\r\n";
        let r = merge_lines(existing, &[(1, b"1\tsame".to_vec())], true);
        assert!(r.replaced.is_empty());
        assert_eq!(r.kept, vec![1]);
    }

    #[test]
    fn cp1252_bytes_survive_untouched() {
        let existing = b"VNUM\tLOCALE_NAME\r\n1\tR\xe4cher\r\n";
        let r = merge_lines(existing, &[(2, b"2\tD\xfcrr".to_vec())], true);
        assert_eq!(r.bytes, b"VNUM\tLOCALE_NAME\r\n1\tR\xe4cher\r\n2\tD\xfcrr\r\n");
    }

    #[test]
    fn unmerge_restores_original_and_keeps_later_edits() {
        let existing = b"1\told\r\n2\tb\r\n";
        let r = merge_lines(existing, &[(1, b"1\tnew".to_vec()), (3, b"3\tc".to_vec())], true);
        let mut later = r.bytes.clone();
        later.extend_from_slice(b"9\tmanual\r\n");
        let back = unmerge_lines(&later, &r.added, &r.replaced);
        assert_eq!(back, b"1\told\r\n2\tb\r\n9\tmanual\r\n");
        assert_eq!(unmerge_lines(&r.bytes, &r.added, &r.replaced), existing);
    }

    #[test]
    fn package_lines_skip_header_and_unwanted() {
        let data = b"VNUM\tLOCALE_NAME\r\n1\ta\r\n2\tb\r\n";
        let got = package_lines(data, &|v| v == 2);
        assert_eq!(got, vec![(2, b"2\tb".to_vec())]);
    }

    #[test]
    fn empty_file_gets_lines_with_eol() {
        let r = merge_lines(b"", &[(1, b"1\ta".to_vec())], true);
        assert_eq!(r.bytes, b"1\ta\r\n");
    }
}
