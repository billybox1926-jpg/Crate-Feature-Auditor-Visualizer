use super::Suggestions;

const BUILTIN_SUGGESTIONS_JSON: &str = include_str!("../../docs/suggestions.json");

/// Load the built-in suggestion database embedded in the released binary.
pub fn load() -> Suggestions {
    super::parse_suggestions(BUILTIN_SUGGESTIONS_JSON)
}

#[cfg(test)]
mod tests {
    use super::load;

    #[test]
    fn embedded_builtin_rules_are_available() {
        let suggestions = load();

        assert!(!suggestions.conflicts.is_empty());
        assert!(!suggestions.bloat.is_empty());
        assert!(suggestions
            .default_features
            .iter()
            .any(|rule| rule.crate_name == "serde"));
    }
}
