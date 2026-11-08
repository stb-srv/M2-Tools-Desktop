//! Liste der verifizierten Quest-Funktionen vom Server (questFunctions.json).

use std::sync::OnceLock;

static FUNCTIONS: OnceLock<Vec<String>> = OnceLock::new();

pub fn quest_functions() -> &'static [String] {
    FUNCTIONS.get_or_init(|| serde_json::from_str(include_str!("quest_functions.json")).unwrap_or_default())
}

/// Wie `filteredFunctions` im TS: Teilstring ohne Groß-/Kleinschreibung,
/// höchstens 200 Treffer.
pub fn filter_functions(query: &str) -> Vec<&'static str> {
    let q = query.trim().to_lowercase();
    quest_functions()
        .iter()
        .filter(|f| q.is_empty() || f.to_lowercase().contains(&q))
        .take(200)
        .map(|s| s.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn loads_and_filters() {
        assert!(super::quest_functions().len() > 500);
        assert!(super::filter_functions("affect.add").contains(&"affect.add_collect"));
        assert_eq!(super::filter_functions("").len(), 200);
    }
}
