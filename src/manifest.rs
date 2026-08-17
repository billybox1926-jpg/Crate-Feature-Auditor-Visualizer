use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::Result;

/// Parsed subset of Cargo.toml data needed for feature analysis.
#[derive(Debug, Clone, Default)]
pub struct ParsedManifest {
    pub package_name: Option<String>,
    pub features: BTreeMap<String, Vec<String>>,
    pub optional_dependencies: BTreeSet<String>,
    pub dependency_features: BTreeMap<String, Vec<String>>,
    pub default_features: Vec<String>,
    pub workspace_dependencies: BTreeMap<String, DependencySpec>,
    pub inherited_workspace_dependencies: BTreeSet<String>,
}

/// Feature-relevant fields from a dependency declaration.
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct DependencySpec {
    pub optional: bool,
    pub features: Vec<String>,
    pub workspace: bool,
}

#[derive(Debug, Default)]
pub struct ManifestCache {
    manifests: HashMap<PathBuf, ParsedManifest>,
}

impl ManifestCache {
    pub fn get_or_parse(
        &mut self,
        path: &Path,
    ) -> Result<ParsedManifest> {
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
        let mut parsed = parse_manifest(&raw);
        hydrate_workspace_dependencies(&path, &mut parsed)?;
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
            "workspace.dependencies" => {
                parsed
                    .workspace_dependencies
                    .insert(key, parse_dependency_spec(value));
            }
            section if is_dependency_section(section) => {
                let spec = parse_dependency_spec(value);
                if spec.optional {
                    parsed.optional_dependencies.insert(key.clone());
                }
                if !spec.features.is_empty() {
                    parsed
                        .dependency_features
                        .insert(key.clone(), spec.features.clone());
                }
                if spec.workspace {
                    parsed.inherited_workspace_dependencies.insert(key);
                }
            }
            _ => {}
        }
    }

    parsed
}

fn hydrate_workspace_dependencies(
    path: &Path,
    parsed: &mut ParsedManifest,
) -> Result<()> {
    if parsed.inherited_workspace_dependencies.is_empty() {
        return Ok(());
    }

    let workspace_dependencies = find_workspace_dependencies(path)?;
    for dependency in &parsed.inherited_workspace_dependencies {
        let Some(workspace_spec) = workspace_dependencies.get(dependency) else {
            continue;
        };

        if workspace_spec.optional {
            parsed.optional_dependencies.insert(dependency.clone());
        }

        let mut features = BTreeSet::new();
        if let Some(existing) = parsed.dependency_features.get(dependency) {
            features.extend(existing.iter().cloned());
        }
        features.extend(workspace_spec.features.iter().cloned());
        if !features.is_empty() {
            parsed
                .dependency_features
                .insert(dependency.clone(), features.into_iter().collect());
        }
    }

    Ok(())
}

fn find_workspace_dependencies(
    path: &Path,
) -> Result<BTreeMap<String, DependencySpec>, Box<dyn std::error::Error>> {
    let Some(manifest_dir) = path.parent() else {
        return Ok(BTreeMap::new());
    };

    for ancestor in manifest_dir.ancestors().skip(1) {
        let candidate = ancestor.join("Cargo.toml");
        if candidate == path || !candidate.is_file() {
            continue;
        }
        let raw = fs::read_to_string(candidate)?;
        let workspace_dependencies = parse_manifest(&raw).workspace_dependencies;
        if !workspace_dependencies.is_empty() {
            return Ok(workspace_dependencies);
        }
    }

    Ok(BTreeMap::new())
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

fn parse_dependency_spec(value: &str) -> DependencySpec {
    if !value.trim_start().starts_with('{') {
        return DependencySpec::default();
    }

    DependencySpec {
        optional: table_bool(value, "optional"),
        features: table_array(value, "features"),
        workspace: table_bool(value, "workspace"),
    }
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
    use super::{parse_manifest, ManifestCache};
    use std::path::Path;

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

    #[test]
    fn parses_target_specific_dependency_sections() {
        let parsed = parse_manifest(include_str!(
            "../tests/fixtures/target-dependencies/Cargo.toml"
        ));

        assert!(parsed.optional_dependencies.contains("nix"));
        assert!(parsed.optional_dependencies.contains("windows-sys"));
        assert_eq!(parsed.dependency_features["nix"], vec!["process"]);
        assert_eq!(
            parsed.dependency_features["windows-sys"],
            vec!["Win32_System_Threading"]
        );
    }

    #[test]
    fn parses_workspace_dependency_inheritance() {
        let mut cache = ManifestCache::default();
        let parsed = cache
            .get_or_parse(Path::new(
                "tests/fixtures/workspace-inheritance/member-a/Cargo.toml",
            ))
            .unwrap();

        assert!(parsed.inherited_workspace_dependencies.contains("serde"));
        assert!(parsed.inherited_workspace_dependencies.contains("tracing"));
        assert!(parsed.optional_dependencies.contains("serde"));
        assert_eq!(parsed.dependency_features["serde"], vec!["derive", "rc"]);
        assert_eq!(parsed.dependency_features["tracing"], vec!["attributes"]);
    }

    #[test]
    fn missing_manifest_still_returns_empty_fallback() {
        let mut cache = ManifestCache::default();
        let parsed = cache
            .get_or_parse(Path::new("tests/fixtures/missing/Cargo.toml"))
            .unwrap();

        assert!(parsed.package_name.is_none());
        assert!(parsed.features.is_empty());
        assert!(parsed.optional_dependencies.is_empty());
        assert!(parsed.dependency_features.is_empty());
    }
}
