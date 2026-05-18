use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum SourceUsageClass {
    Normal,
    Dev,
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct SourceUsage {
    pub normal_features: BTreeSet<String>,
    pub dev_features: BTreeSet<String>,
}

impl SourceUsage {
    pub fn referenced_features(&self) -> BTreeSet<String> {
        self.normal_features
            .union(&self.dev_features)
            .cloned()
            .collect()
    }
}

pub fn scan_package_sources(manifest_path: &str) -> SourceUsage {
    let manifest = PathBuf::from(manifest_path);
    let Some(package_root) = manifest.parent() else {
        return SourceUsage::default();
    };
    if !package_root.exists() || !package_root.is_dir() {
        return SourceUsage::default();
    }

    let mut usage = SourceUsage::default();
    for (dir, class) in [
        ("src", SourceUsageClass::Normal),
        ("tests", SourceUsageClass::Dev),
        ("examples", SourceUsageClass::Dev),
        ("benches", SourceUsageClass::Dev),
    ] {
        let path = package_root.join(dir);
        for file in sorted_rust_files(&path) {
            if let Ok(raw) = fs::read_to_string(&file) {
                let features = extract_feature_references(&raw);
                match class {
                    SourceUsageClass::Normal => usage.normal_features.extend(features),
                    SourceUsageClass::Dev => usage.dev_features.extend(features),
                }
            }
        }
    }

    usage
}

fn sorted_rust_files(root: &Path) -> Vec<PathBuf> {
    if !root.exists() || !root.is_dir() {
        return Vec::new();
    }

    let mut files = Vec::new();
    visit_rust_files(root, &mut files);
    files.sort();
    files
}

fn visit_rust_files(path: &Path, files: &mut Vec<PathBuf>) {
    let Ok(read_dir) = fs::read_dir(path) else {
        return;
    };
    let mut entries = read_dir.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let entry_path = entry.path();
        if is_hidden(&entry_path) || entry_path.ends_with("target") || entry_path.ends_with(".git")
        {
            continue;
        }
        if entry_path.is_dir() {
            visit_rust_files(&entry_path, files);
            continue;
        }
        if entry_path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(entry_path);
        }
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

pub fn extract_feature_references(raw: &str) -> BTreeSet<String> {
    let mut features = BTreeSet::new();
    let mut remaining = raw;

    while let Some(index) = remaining.find("feature") {
        remaining = &remaining[index + "feature".len()..];
        let trimmed = remaining.trim_start();
        if !trimmed.starts_with('=') {
            continue;
        }
        let after_equals = trimmed[1..].trim_start();
        let Some(stripped) = after_equals.strip_prefix('"') else {
            continue;
        };
        let Some(end_quote) = stripped.find('"') else {
            continue;
        };
        let candidate = &stripped[..end_quote];
        if !candidate.is_empty()
            && candidate
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        {
            features.insert(candidate.to_string());
        }
        remaining = &stripped[end_quote + 1..];
    }

    features
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_simple_cfg_and_cfg_macro_features() {
        let raw = r#"
#[cfg(feature = "foo")]
const FOO: bool = cfg!(feature = "bar");
"#;
        let features = extract_feature_references(raw);
        assert_eq!(
            features,
            BTreeSet::from(["bar".to_string(), "foo".to_string()])
        );
    }

    #[test]
    fn extracts_nested_any_and_all_cfg_features() {
        let raw = r#"
#[cfg(any(feature = "foo", feature = "bar"))]
#[cfg(all(feature = "baz", unix))]
fn item() {}
"#;
        let features = extract_feature_references(raw);
        assert_eq!(
            features,
            BTreeSet::from(["bar".to_string(), "baz".to_string(), "foo".to_string()])
        );
    }

    #[test]
    fn ignores_unrelated_string_literals() {
        let raw = r#"
let text = "feature not cfg";
let cfg = "cfg(feature = string only)";
"#;
        assert!(extract_feature_references(raw).is_empty());
    }
}
