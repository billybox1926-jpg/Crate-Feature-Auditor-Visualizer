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
}

#[derive(Debug, Clone, Default)]
pub struct ResolveNode {
    pub id: String,
    pub features: Vec<String>,
    pub dependencies: Vec<String>,
}

/// Load Cargo's resolved package graph without compiling the target workspace.
pub fn load_metadata(manifest_path: &Path) -> Result<Metadata, Box<dyn std::error::Error>> {
    let manifest = if manifest_path.is_file() {
        manifest_path.to_path_buf()
    } else {
        manifest_path.join("Cargo.toml")
    };
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest)
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
        .map(|object| Package {
            id: string_field(&object, "id").unwrap_or_default(),
            name: string_field(&object, "name").unwrap_or_default(),
            version: string_field(&object, "version").unwrap_or_default(),
            manifest_path: PathBuf::from(
                string_field(&object, "manifest_path").unwrap_or_default(),
            ),
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
            dependencies: string_array_field(&object, "dependencies"),
        })
        .filter(|node| !node.id.is_empty())
        .collect();

    Ok(Metadata {
        packages,
        workspace_members,
        resolve_nodes,
    })
}

fn string_field(object: &str, key: &str) -> Option<String> {
    let key_pattern = format!("\"{key}\":");
    let start = object.find(&key_pattern)? + key_pattern.len();
    parse_json_string(&object[start..]).map(|(value, _)| value)
}

fn string_array_field(object: &str, key: &str) -> Vec<String> {
    let key_pattern = format!("\"{key}\":");
    let Some(start) = object.find(&key_pattern).map(|idx| idx + key_pattern.len()) else {
        return Vec::new();
    };
    let Some((array, _)) = bracketed(&object[start..], '[', ']') else {
        return Vec::new();
    };
    let mut values = Vec::new();
    let mut rest = array.as_str();
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
                depth -= 1;
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
    use super::parse_metadata;

    #[test]
    fn parses_minimal_metadata_shape() {
        let metadata = parse_metadata(
            r#"{"packages":[{"id":"demo 0.1.0","name":"demo","version":"0.1.0","manifest_path":"/tmp/Cargo.toml"}],"workspace_members":["demo 0.1.0"],"resolve":{"nodes":[{"id":"demo 0.1.0","dependencies":[],"features":["default"]}]}}"#,
        )
        .unwrap();
        assert_eq!(metadata.packages[0].name, "demo");
        assert_eq!(metadata.resolve_nodes[0].features, vec!["default"]);
    }
}
