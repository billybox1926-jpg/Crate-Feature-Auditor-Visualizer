/// Returns true when a crate name matches an optional substring filter.
pub fn crate_matches(name: &str, filter: Option<&str>) -> bool {
    filter.map(|filter| name.contains(filter)).unwrap_or(true)
}

/// Formats a path through the dependency graph for display.
pub fn format_path(path: &[String]) -> String {
    if path.is_empty() {
        "<unknown>".to_string()
    } else {
        path.join(" → ")
    }
}

#[cfg(test)]
mod tests {
    use super::format_path;

    #[test]
    fn formats_empty_paths() {
        assert_eq!(format_path(&[]), "<unknown>");
    }
}
