use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed subset of Cargo.toml data needed for feature analysis.
#[derive(Debug, Clone, Default)]
pub struct ParsedManifest {
    pub package_name: Option<String>,
    pub features: BTreeMap<String, Vec<String>>,
    pub optional_dependencies: BTreeSet<String>,
    pub dependency_features: BTreeMap<String, Vec<String>>,
    pub default_features: Vec<String>,
}

#[derive(Debug, Default)]
pub struct ManifestCache {
    manifests: HashMap<PathBuf, ParsedManifest>,
}

impl ManifestCache {
    pub fn get_or_parse(
        &mut self,
        path: &Path,
    ) -> Result<ParsedManifest, Box<dyn std::error::Error>> {
        let path = path.to_path_buf();
        if let Some(manifest) = self.manifests.get(&path) {
            return Ok(manifest.clone());
        }

        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parsed = ParsedManifest::default();
                self.manifests.insert(path, parsed.clone());
                return Ok(parsed);
            }
            Err(error) => return Err(error.into()),
        };
        let parsed = parse_manifest(&raw);
        self.manifests.insert(path, parsed.clone());
        Ok(parsed)
    }
}

fn parse_manifest(raw: &str) -> ParsedManifest {
    let mut parsed = ParsedManifest::default();
    let mut section = String::new();

    for line in raw.lines().map(strip_comment).map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            section = line.trim_matches(['[', ']']).to_string();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim().trim_matches('"').to_string();
        let value = value.trim();

        match section.as_str() {
            "package" if key == "name" => parsed.package_name = parse_string(value),
            "features" => {
                let values = parse_array(value);
                if key == "default" {
                    parsed.default_features = values.clone();
                }
                parsed.features.insert(key, values);
            }
            section if is_dependency_section(section) && value.starts_with('{') => {
                if table_bool(value, "optional") {
                    parsed.optional_dependencies.insert(key.clone());
                }
                let features = table_array(value, "features");
                if !features.is_empty() {
                    parsed.dependency_features.insert(key, features);
                }
            }
            _ => {}
        }
    }

    parsed
}

fn is_dependency_section(section: &str) -> bool {
    matches!(
        section,
        "dependencies" | "dev-dependencies" | "build-dependencies"
    ) || section.ends_with(".dependencies")
        || section.ends_with(".dev-dependencies")
        || section.ends_with(".build-dependencies")
}

fn strip_comment(line: &str) -> &str {
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in line.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
        } else if ch == '"' {
            in_string = true;
        } else if ch == '#' {
            return &line[..idx];
        }
    }

    line
}

fn parse_string(value: &str) -> Option<String> {
    let value = value.trim();
    if value.starts_with('"') && value.ends_with('"') {
        Some(value.trim_matches('"').to_string())
    } else {
        None
    }
}

fn parse_array(value: &str) -> Vec<String> {
    let value = value.trim().trim_start_matches('[').trim_end_matches(']');
    value
        .split(',')
        .filter_map(|item| parse_string(item.trim()))
        .collect()
}

fn table_array(value: &str, key: &str) -> Vec<String> {
    let Some(start) = value.find(key) else {
        return Vec::new();
    };
    let tail = &value[start + key.len()..];
    let Some((_, tail)) = tail.split_once('=') else {
        return Vec::new();
    };
    let tail = tail.trim_start();
    if !tail.starts_with('[') {
        return Vec::new();
    }
    let Some(end) = tail.find(']') else {
        return Vec::new();
    };
    parse_array(&tail[..=end])
}

fn table_bool(value: &str, key: &str) -> bool {
    value.contains(&format!("{key} = true")) || value.contains(&format!("{key}=true"))
}

#[cfg(test)]
mod tests {
    use super::parse_manifest;

    #[test]
    fn parses_optional_dependencies_and_features() {
        let parsed = parse_manifest(
            r#"
            [package]
            name = "demo"

            [features]
            default = ["serde"]
            full = ["dep:serde_json", "serde/derive"]

            [dependencies]
            serde = { version = "1", optional = true, features = ["derive"] }
            serde_json = { version = "1", optional = true }
            "#,
        );

        assert_eq!(parsed.package_name.as_deref(), Some("demo"));
        assert!(parsed.optional_dependencies.contains("serde"));
        assert_eq!(parsed.default_features, vec!["serde"]);
        assert_eq!(parsed.dependency_features["serde"], vec!["derive"]);
    }
}
