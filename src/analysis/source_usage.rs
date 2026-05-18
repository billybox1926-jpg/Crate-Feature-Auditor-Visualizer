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
    let bytes = raw.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if let Some((next, consumed)) = skip_non_code(raw, i) {
            i = next;
            if consumed {
                continue;
            }
        }

        if raw[i..].starts_with("#[cfg(") {
            if let Some((inside, next)) = read_balanced_group(raw, i + 5, '(', ')') {
                collect_cfg_features(&inside, &mut features);
                i = next;
                continue;
            }
        }

        if raw[i..].starts_with("cfg!(") {
            if let Some((inside, next)) = read_balanced_group(raw, i + 4, '(', ')') {
                collect_cfg_features(&inside, &mut features);
                i = next;
                continue;
            }
        }

        i += 1;
    }

    features
}

fn collect_cfg_features(expr: &str, features: &mut BTreeSet<String>) {
    let mut remaining = expr;
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
}

fn read_balanced_group(
    raw: &str,
    start: usize,
    open: char,
    close: char,
) -> Option<(String, usize)> {
    let mut depth = 0i32;
    let mut started = false;
    let mut inside = String::new();

    for (offset, ch) in raw[start..].char_indices() {
        if ch == open {
            depth += 1;
            started = true;
            if depth > 1 {
                inside.push(ch);
            }
            continue;
        }
        if !started {
            continue;
        }
        if ch == close {
            depth -= 1;
            if depth == 0 {
                return Some((inside, start + offset + ch.len_utf8()));
            }
            inside.push(ch);
            continue;
        }
        inside.push(ch);
    }

    None
}

fn skip_non_code(raw: &str, start: usize) -> Option<(usize, bool)> {
    let rest = &raw[start..];
    if rest.starts_with("//") {
        let next = rest
            .find('\n')
            .map(|idx| start + idx + 1)
            .unwrap_or(raw.len());
        return Some((next, true));
    }
    if rest.starts_with("/*") {
        if let Some(end) = rest.find("*/") {
            return Some((start + end + 2, true));
        }
        return Some((raw.len(), true));
    }
    if let Some(stripped) = rest.strip_prefix('"') {
        let mut escaped = false;
        for (offset, ch) in stripped.char_indices() {
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                return Some((start + offset + 2, true));
            }
        }
        return Some((raw.len(), true));
    }
    None
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
    fn ignores_comments_doc_examples_and_unrelated_string_literals() {
        let raw = r##"
// #[cfg(feature = "fake")]
/// Example: #[cfg(feature = "fake_doc")]
/* cfg!(feature = "fake_block") */
let text = "feature = \"fake\"";
let cfg = "#[cfg(feature = \"also_fake\")]";
"##;
        assert!(extract_feature_references(raw).is_empty());
    }
}
