use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_members: Vec<String>,
    pub resolve_nodes: Vec<ResolveNode>,
}

#[derive(Debug, Clone, Default)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: PathBuf,
    pub features: BTreeMap<String, Vec<String>>,
    pub optional_dependencies: BTreeSet<String>,
    pub dependency_features: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResolveNode {
    pub id: String,
    pub features: Vec<String>,
    pub dependencies: Vec<String>,
    pub dependency_names: BTreeMap<String, String>,
}

/// Load Cargo's resolved package graph without compiling the target workspace.
pub fn load_metadata(manifest_path: &Path) -> Result<Metadata, Box<dyn std::error::Error>> {
    let manifest = if manifest_path.is_file() {
        manifest_path.to_path_buf()
    } else {
        manifest_path.join("Cargo.toml")
    };
    load_metadata_manifest(&manifest)
}

pub fn load_metadata_manifest(manifest: &Path) -> Result<Metadata, Box<dyn std::error::Error>> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(manifest)
        .output()?;

    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    parse_metadata(&String::from_utf8(output.stdout)?)
}

fn parse_metadata(raw: &str) -> Result<Metadata, Box<dyn std::error::Error>> {
    let packages = array_objects(raw, "packages")
        .into_iter()
        .map(|object| {
            let mut dependency_features = BTreeMap::<String, Vec<String>>::new();
            for dependency in array_objects(&object, "dependencies") {
                let Some(name) = string_field(&dependency, "rename")
                    .or_else(|| string_field(&dependency, "name"))
                else {
                    continue;
                };
                let features = string_array_field(&dependency, "features");
                if !features.is_empty() {
                    dependency_features
                        .entry(name)
                        .or_default()
                        .extend(features);
                }
            }
            for features in dependency_features.values_mut() {
                features.sort();
                features.dedup();
            }

            let optional_dependencies = array_objects(&object, "dependencies")
                .into_iter()
                .filter(|dependency| bool_field(dependency, "optional"))
                .filter_map(|dependency| {
                    string_field(&dependency, "rename")
                        .or_else(|| string_field(&dependency, "name"))
                })
                .collect::<BTreeSet<_>>();

            Package {
                id: string_field(&object, "id").unwrap_or_default(),
                name: string_field(&object, "name").unwrap_or_default(),
                version: string_field(&object, "version").unwrap_or_default(),
                manifest_path: PathBuf::from(
                    string_field(&object, "manifest_path").unwrap_or_default(),
                ),
                features: string_array_map_field(&object, "features"),
                optional_dependencies,
                dependency_features,
            }
        })
        .filter(|package| !package.id.is_empty())
        .collect::<Vec<_>>();

    let workspace_members = string_array_field(raw, "workspace_members");
    let resolve_raw = object_field(raw, "resolve").unwrap_or_default();
    let resolve_nodes = array_objects(&resolve_raw, "nodes")
        .into_iter()
        .map(|object| ResolveNode {
            id: string_field(&object, "id").unwrap_or_default(),
            features: string_array_field(&object, "features"),
            dependencies: resolve_dependencies(&object),
            dependency_names: resolve_dependency_names(&object),
        })
        .filter(|node| !node.id.is_empty())
        .collect();

    Ok(Metadata {
        packages,
        workspace_members,
        resolve_nodes,
    })
}

fn resolve_dependencies(object: &str) -> Vec<String> {
    let mut deps = string_array_field(object, "dependencies");
    if deps.is_empty() {
        deps = array_objects(object, "deps")
            .into_iter()
            .filter_map(|dep| string_field(&dep, "pkg"))
            .collect();
    }
    deps.sort();
    deps.dedup();
    deps
}

fn resolve_dependency_names(object: &str) -> BTreeMap<String, String> {
    array_objects(object, "deps")
        .into_iter()
        .filter_map(|dep| {
            let pkg = string_field(&dep, "pkg")?;
            let name = string_field(&dep, "name")?;
            Some((name, pkg))
        })
        .collect()
}

fn string_field(object: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{key}\":");
    let start = object.find(&key_pattern)? + key_pattern.len();
    let tail = object[start..].trim_start();
    if tail.starts_with("null") {
        return None;
    }
    parse_json_string(tail).map(|(value, _)| value)
}

fn bool_field(object: &str, key: &str) -> bool {
    let key_pattern = format!("\"{key}\":");
    let Some(start) = object.find(&key_pattern).map(|idx| idx + key_pattern.len()) else {
        return false;
    };
    object[start..].trim_start().starts_with("true")
}

fn string_array_field(object: &str, key: &str) -> Vec<String> {
    let key_pattern = format!("\"{key}\":");
    let Some(start) = object.find(&key_pattern).map(|idx| idx + key_pattern.len()) else {
        return Vec::new();
    };
    let Some((array, _)) = bracketed(&object[start..], '[', ']') else {
        return Vec::new();
    };
    string_items(&array)
}

fn string_array_map_field(object: &str, key: &str) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    let Some(raw_object) = object_field(object, key) else {
        return map;
    };
    let mut rest = raw_object.trim_start_matches('{').trim_end_matches('}');

    while let Some(pos) = rest.find('"') {
        rest = &rest[pos..];
        let Some((key, consumed)) = parse_json_string(rest) else {
            break;
        };
        rest = &rest[consumed..];
        let Some(colon) = rest.find(':') else {
            break;
        };
        rest = &rest[colon + 1..];
        let Some((array, consumed)) = bracketed(rest, '[', ']') else {
            break;
        };
        map.insert(key, string_items(&array));
        rest = &rest[consumed..];
    }

    map
}

fn object_field(object: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{key}\":");
    let start = object.find(&key_pattern)? + key_pattern.len();
    bracketed(&object[start..], '{', '}').map(|(value, _)| value)
}

fn array_objects(object: &str, key: &str) -> Vec<String> {
    let key_pattern = format!("\"{key}\":");
    let Some(start) = object.find(&key_pattern).map(|idx| idx + key_pattern.len()) else {
        return Vec::new();
    };
    let Some((array, _)) = bracketed(&object[start..], '[', ']') else {
        return Vec::new();
    };
    let mut objects = Vec::new();
    let mut rest = array.as_str();
    while let Some(pos) = rest.find('{') {
        rest = &rest[pos..];
        if let Some((value, consumed)) = bracketed(rest, '{', '}') {
            objects.push(value);
            rest = &rest[consumed..];
        } else {
            break;
        }
    }
    objects
}

fn bracketed(input: &str, open: char, close: char) -> Option<(String, usize)> {
    let start = input.find(open)?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in input[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            c if c == open => depth += 1,
            c if c == close => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = start + idx + ch.len_utf8();
                    return Some((input[start..end].to_string(), end));
                }
            }
            _ => {}
        }
    }
    None
}

fn string_items(input: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut rest = input;
    while let Some(pos) = rest.find('"') {
        rest = &rest[pos..];
        if let Some((value, consumed)) = parse_json_string(rest) {
            values.push(value);
            rest = &rest[consumed..];
        } else {
            break;
        }
    }
    values
}

fn parse_json_string(input: &str) -> Option<(String, usize)> {
    let start = input.find('"')?;
    let mut value = String::new();
    let mut escaped = false;
    for (idx, ch) in input[start + 1..].char_indices() {
        if escaped {
            value.push(match ch {
                'n' => '\n',
                't' => '\t',
                'r' => '\r',
                other => other,
            });
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Some((value, start + 1 + idx + 1));
        } else {
            value.push(ch);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_package_feature_map_and_resolve_deps_shape() {
        let metadata = parse_metadata(
            r#"{
              "packages":[{"id":"demo 0.1.0","name":"demo","version":"0.1.0","manifest_path":"/tmp/Cargo.toml","features":{"default":["std"],"std":[]},"dependencies":[{"name":"serde","rename":null,"optional":true,"features":["derive"]}]}],
              "workspace_members":["demo 0.1.0"],
              "resolve":{"nodes":[{"id":"demo 0.1.0","features":["default","std"],"deps":[{"name":"serde","pkg":"serde 1.0.0"}]}]}
            }"#,
        )
        .unwrap();

        assert_eq!(metadata.packages[0].features["default"], vec!["std"]);
        assert!(metadata.packages[0].optional_dependencies.contains("serde"));
        assert_eq!(
            metadata.packages[0].dependency_features["serde"],
            vec!["derive"]
        );
        assert_eq!(metadata.resolve_nodes[0].dependencies, vec!["serde 1.0.0"]);
        assert_eq!(
            metadata.resolve_nodes[0].dependency_names["serde"],
            "serde 1.0.0"
        );
    }
}
