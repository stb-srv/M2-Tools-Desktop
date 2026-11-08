//! Reine Logik des Locale-Editors.

// "[ENTER]" is the game's own literal in-game line-break marker (same
// convention as questTemplates.ts::luaString) - shown as real newlines in
// the textarea and converted back on save.
pub fn to_display(value: &str) -> String {
    value.replace("[ENTER]", "\n")
}

pub fn to_stored(display: &str) -> String {
    display.replace("\r\n", "[ENTER]").replace('\n', "[ENTER]")
}

/// `n.toLowerCase().includes(search.toLowerCase())`
pub fn filter_namespaces(namespaces: &[String], search: &str) -> Vec<String> {
    let q = search.to_lowercase();
    namespaces.iter().filter(|n| n.to_lowercase().contains(&q)).cloned().collect()
}

/// `entry.split(".")[0]`
pub fn namespace_of_orphan(entry: &str) -> String {
    entry.split('.').next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enter_roundtrip() {
        assert_eq!(to_display("a[ENTER]b[ENTER]"), "a\nb\n");
        assert_eq!(to_stored("a\nb\r\nc"), "a[ENTER]b[ENTER]c");
        assert_eq!(to_stored(&to_display("x[ENTER]y")), "x[ENTER]y");
        assert_eq!(to_stored("\r"), "\r");
    }

    #[test]
    fn filter() {
        let ns = vec!["Quest_A".to_string(), "other".to_string()];
        assert_eq!(filter_namespaces(&ns, "quest"), vec!["Quest_A".to_string()]);
        assert_eq!(filter_namespaces(&ns, "").len(), 2);
    }

    #[test]
    fn orphan_namespace() {
        assert_eq!(namespace_of_orphan("ns._key"), "ns");
        assert_eq!(namespace_of_orphan("plain"), "plain");
    }
}
