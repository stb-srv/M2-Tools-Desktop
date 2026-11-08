//! Zeilenbasierter Text-Diff (Ersatz für `diffLines` aus dem npm-Paket
//! `diff`) auf Basis von `similar`. Aufeinanderfolgende Zeilen derselben
//! Art werden wie bei `diffLines` zu einem Block zusammengefasst; pro Block
//! werden die Zeilen ohne abschließenden Zeilenumbruch geliefert.

use similar::{ChangeTag, TextDiff};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    Added,
    Removed,
    Unchanged,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiffPart {
    pub kind: PartKind,
    /// Die Zeilen des Blocks (ohne `\n` am Ende).
    pub lines: Vec<String>,
}

impl DiffPart {
    pub fn prefix(&self) -> &'static str {
        match self.kind {
            PartKind::Added => "+",
            PartKind::Removed => "-",
            PartKind::Unchanged => " ",
        }
    }

    pub fn class(&self) -> &'static str {
        match self.kind {
            PartKind::Added => "bg-green-500/15 text-green-700 dark:text-green-400",
            PartKind::Removed => "bg-red-500/15 text-red-700 dark:text-red-400",
            PartKind::Unchanged => "text-muted-foreground",
        }
    }
}

/// `old` = aktuell auf dem Server, `new` = Inhalt der Backup-Datei.
pub fn diff_lines(old: &str, new: &str) -> Vec<DiffPart> {
    let diff = TextDiff::from_lines(old, new);
    let mut parts: Vec<(PartKind, String)> = Vec::new();
    for change in diff.iter_all_changes() {
        let kind = match change.tag() {
            ChangeTag::Insert => PartKind::Added,
            ChangeTag::Delete => PartKind::Removed,
            ChangeTag::Equal => PartKind::Unchanged,
        };
        let value = change.value();
        match parts.last_mut() {
            Some((k, v)) if *k == kind => v.push_str(value),
            _ => parts.push((kind, value.to_string())),
        }
    }
    parts
        .into_iter()
        .map(|(kind, value)| {
            // wie `part.value.replace(/\n$/, "").split("\n")`
            let trimmed = value.strip_suffix('\n').unwrap_or(&value);
            DiffPart { kind, lines: trimmed.split('\n').map(str::to_string).collect() }
        })
        .collect()
}

pub fn has_changes(parts: &[DiffPart]) -> bool {
    parts.iter().any(|p| p.kind != PartKind::Unchanged)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_has_no_changes() {
        let parts = diff_lines("a\nb\n", "a\nb\n");
        assert!(!has_changes(&parts));
        assert_eq!(parts, vec![DiffPart { kind: PartKind::Unchanged, lines: vec!["a".into(), "b".into()] }]);
    }

    #[test]
    fn replaced_line_removed_then_added() {
        let parts = diff_lines("a\nb\nc\n", "a\nx\nc\n");
        let kinds: Vec<PartKind> = parts.iter().map(|p| p.kind).collect();
        assert_eq!(kinds, vec![PartKind::Unchanged, PartKind::Removed, PartKind::Added, PartKind::Unchanged]);
        assert_eq!(parts[1].lines, vec!["b"]);
        assert_eq!(parts[2].lines, vec!["x"]);
        assert!(has_changes(&parts));
    }

    #[test]
    fn missing_current_file_is_all_added() {
        let parts = diff_lines("", "one\ntwo");
        assert_eq!(parts, vec![DiffPart { kind: PartKind::Added, lines: vec!["one".into(), "two".into()] }]);
    }

    #[test]
    fn empty_lines_kept() {
        let parts = diff_lines("a\n\nb\n", "a\n\nb\n");
        assert_eq!(parts[0].lines, vec!["a", "", "b"]);
    }

    #[test]
    fn prefixes() {
        let parts = diff_lines("a\n", "b\n");
        assert_eq!(parts.iter().map(|p| p.prefix()).collect::<Vec<_>>(), vec!["-", "+"]);
    }
}
