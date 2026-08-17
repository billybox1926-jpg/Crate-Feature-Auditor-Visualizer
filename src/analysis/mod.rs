use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::resolver::FeatureGraph;

pub mod bloat;
pub mod conflicts;
pub mod duplication;
pub mod source_usage;
pub mod unused;

/// Shared inputs for independent analysis passes.
pub struct AnalysisContext<'a> {
    pub graph: &'a FeatureGraph,
    pub suggestions: &'a Suggestions,
}

impl<'a> AnalysisContext<'a> {
    pub fn new(graph: &'a FeatureGraph, suggestions: &'a Suggestions) -> Self {
        Self { graph, suggestions }
    }
}

/// A reportable analysis result.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Finding {
    pub crate_name: String,
    pub feature: Option<String>,
    pub kind: FindingKind,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum FindingKind {
    Unused,
    Conflict,
    Duplicate,
    Bloat,
    DefaultFeature,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn icon(self) -> &'static str {
        match self {
            Severity::Info => "ℹ",
            Severity::Warning => "⚠",
            Severity::Error => "✖",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "info" => Some(Severity::Info),
            "warning" | "warn" => Some(Severity::Warning),
            "error" | "deny" => Some(Severity::Error),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

/// Optional suggestion database loaded from `suggestions.json`.
#[derive(Debug, Clone, Default)]
pub struct Suggestions {
    pub conflicts: Vec<ConflictRule>,
    pub bloat: Vec<BloatRule>,
    pub default_features: Vec<DefaultFeatureRule>,
}

impl Suggestions {
    pub fn load_optional(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(parse_suggestions(&raw))
    }

    pub fn load_local_optional(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        parse_local_suggestions(&raw)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()).into())
    }

    pub fn extend(&mut self, other: Suggestions) {
        self.conflicts.extend(other.conflicts);
        self.bloat.extend(other.bloat);
        self.default_features.extend(other.default_features);
    }
}

#[derive(Debug, Clone)]
pub struct ConflictRule {
    pub crate_name: String,
    pub features: Vec<String>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct BloatRule {
    pub crate_name: String,
    pub feature: String,
    pub pulls_in: Vec<String>,
    pub severity: Severity,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DefaultFeatureRule {
    pub crate_name: String,
    pub severity: Severity,
    pub reason: String,
}

pub fn run_all(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(unused::analyze(context));
    findings.extend(conflicts::analyze(context));
    findings.extend(duplication::analyze(context));
    findings.extend(bloat::analyze(context));
    findings.extend(default_features(context));
    findings.sort_by(|left, right| {
        left.crate_name
            .cmp(&right.crate_name)
            .then_with(|| format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)))
            .then_with(|| left.feature.cmp(&right.feature))
    });
    findings
}

fn default_features(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        // Cargo metadata's resolved feature list is the source of truth here.
        // For this first advisory rule, treat a resolved `default` feature as
        // evidence that the crate's default features are active. This stays
        // deterministic and avoids guessing from raw dependency declarations.
        if !node.active_features.contains("default") {
            continue;
        }
        for rule in &context.suggestions.default_features {
            if rule.crate_name == node.name {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some("default".to_string()),
                    kind: FindingKind::DefaultFeature,
                    severity: rule.severity,
                    message: rule.reason.clone(),
                });
            }
        }
    }

    findings
}

pub fn parse_suggestions(raw: &str) -> Suggestions {
    let mut suggestions = Suggestions::default();

    for object in array_objects(raw, "conflicts") {
        let Some(crate_name) = field(&object, "crate") else {
            continue;
        };
        let Some(features) = array_field(&object, "features") else {
            continue;
        };
        let Some(message) = field(&object, "message") else {
            continue;
        };
        suggestions.conflicts.push(ConflictRule {
            crate_name,
            features,
            severity: field(&object, "severity")
                .as_deref()
                .and_then(Severity::parse)
                .unwrap_or(Severity::Warning),
            message,
        });
    }

    for object in array_objects(raw, "bloat") {
        let Some(crate_name) = field(&object, "crate") else {
            continue;
        };
        let Some(feature) = field(&object, "feature") else {
            continue;
        };
        let Some(message) = field(&object, "message") else {
            continue;
        };
        suggestions.bloat.push(BloatRule {
            crate_name,
            feature,
            pulls_in: array_field(&object, "pulls_in").unwrap_or_default(),
            severity: field(&object, "severity")
                .as_deref()
                .and_then(Severity::parse)
                .unwrap_or(Severity::Info),
            message,
        });
    }

    if let Some(default_features) = object_field(raw, "default_features") {
        for object in array_objects(&default_features, "suggest_opt_out") {
            let Some(crate_name) = field(&object, "crate") else {
                continue;
            };
            let Some(reason) = field(&object, "reason") else {
                continue;
            };
            suggestions.default_features.push(DefaultFeatureRule {
                crate_name,
                severity: field(&object, "severity")
                    .as_deref()
                    .and_then(Severity::parse)
                    .unwrap_or(Severity::Info),
                reason,
            });
        }
    }

    suggestions
}

fn parse_local_suggestions(raw: &str) -> Result<Suggestions, String> {
    let mut out = Suggestions::default();
    let mut section: Option<String> = None;
    let mut pending = std::collections::BTreeMap::<String, String>::new();
    let mut line_no = 0usize;

    let push_pending =
        |section: Option<&str>,
         pending: &mut std::collections::BTreeMap<String, String>,
         out: &mut Suggestions|
         -> Result<(), String> {
            let Some(current) = section else {
                return Ok(());
            };
            match current {
                "conflicts" => {
                    let crate_name = pending.remove("crate").ok_or("missing `crate`")?;
                    let features = pending
                        .remove("features")
                        .ok_or("missing `features`")?
                        .split(',')
                        .map(|item| item.trim().to_string())
                        .filter(|item| !item.is_empty())
                        .collect::<Vec<_>>();
                    let message = pending.remove("message").ok_or("missing `message`")?;
                    let severity = match pending.remove("severity") {
                        Some(raw) => Severity::parse(&raw)
                            .ok_or_else(|| format!("invalid `severity`: {raw}"))?,
                        None => Severity::Warning,
                    };
                    out.conflicts.push(ConflictRule {
                        crate_name,
                        features,
                        severity,
                        message,
                    });
                }
                "bloat" => {
                    let crate_name = pending.remove("crate").ok_or("missing `crate`")?;
                    let feature = pending.remove("feature").ok_or("missing `feature`")?;
                    let message = pending.remove("message").ok_or("missing `message`")?;
                    let pulls_in = pending
                        .remove("pulls_in")
                        .map(|value| {
                            value
                                .split(',')
                                .map(|item| item.trim().to_string())
                                .filter(|item| !item.is_empty())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let severity = match pending.remove("severity") {
                        Some(raw) => Severity::parse(&raw)
                            .ok_or_else(|| format!("invalid `severity`: {raw}"))?,
                        None => Severity::Info,
                    };
                    out.bloat.push(BloatRule {
                        crate_name,
                        feature,
                        pulls_in,
                        severity,
                        message,
                    });
                }
                "default_features.suggest_opt_out" => {
                    let crate_name = pending.remove("crate").ok_or("missing `crate`")?;
                    let reason = pending.remove("reason").ok_or("missing `reason`")?;
                    let severity = match pending.remove("severity") {
                        Some(raw) => Severity::parse(&raw)
                            .ok_or_else(|| format!("invalid `severity`: {raw}"))?,
                        None => Severity::Info,
                    };
                    out.default_features.push(DefaultFeatureRule {
                        crate_name,
                        severity,
                        reason,
                    });
                }
                _ => {}
            }
            pending.clear();
            Ok(())
        };

    for raw_line in raw.lines() {
        line_no += 1;
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with("[[") && line.ends_with("]]") {
            push_pending(section.as_deref(), &mut pending, &mut out)
                .map_err(|err| format!("{err} in section before line {line_no}"))?;
            section = Some(line[2..line.len() - 2].to_string());
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {line_no}: expected `key = value`"));
        };
        let key = key.trim().to_string();
        let value = value.trim();
        let parsed = if value.starts_with('\"') {
            parse_json_string(value)
                .ok_or_else(|| format!("line {line_no}: malformed string"))?
                .0
        } else if value.starts_with('[') {
            if !value.ends_with(']') {
                return Err(format!("line {line_no}: malformed array"));
            }
            let arr = string_items(value);
            if arr.is_empty() && value != "[]" {
                return Err(format!(
                    "line {line_no}: array items must be quoted strings"
                ));
            }
            arr.join(",")
        } else {
            return Err(format!("line {line_no}: unsupported value `{value}`"));
        };
        pending.insert(key, parsed);
    }
    push_pending(section.as_deref(), &mut pending, &mut out)?;
    Ok(out)
}

fn object_field(object: &str, key: &str) -> Option<String> {
    bracketed(value_after_key(object, key)?, '{', '}').map(|(value, _)| value)
}

fn array_objects(object: &str, key: &str) -> Vec<String> {
    let Some(array) = raw_array_field(object, key) else {
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

fn field(object: &str, key: &str) -> Option<String> {
    parse_json_string(value_after_key(object, key)?).map(|(value, _)| value)
}

fn array_field(object: &str, key: &str) -> Option<Vec<String>> {
    raw_array_field(object, key).map(|array| string_items(&array))
}

fn raw_array_field(object: &str, key: &str) -> Option<String> {
    bracketed(value_after_key(object, key)?, '[', ']').map(|(value, _)| value)
}

fn value_after_key<'a>(object: &'a str, key: &str) -> Option<&'a str> {
    let key_pattern = format!("\"{key}\"");
    let start = object.find(&key_pattern)? + key_pattern.len();
    let tail = object[start..].trim_start();
    tail.strip_prefix(':')
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
    use std::collections::BTreeSet;

    use super::*;
    use crate::resolver::{FeatureGraph, FeatureNode};

    #[test]
    fn parses_all_suggestion_rule_kinds() {
        let suggestions = parse_suggestions(include_str!("../../docs/suggestions.json"));

        assert!(!suggestions.conflicts.is_empty());
        assert!(!suggestions.bloat.is_empty());
        assert!(suggestions.default_features.len() >= 3);
        assert!(suggestions.default_features.iter().any(|rule| {
            rule.crate_name == "log"
                && rule.severity == Severity::Info
                && rule.reason.contains("no-std logger")
        }));
        assert!(suggestions
            .default_features
            .iter()
            .any(|rule| rule.crate_name == "reqwest"));
        assert!(suggestions
            .default_features
            .iter()
            .any(|rule| rule.crate_name == "serde"));
    }

    #[test]
    fn ignores_default_feature_like_objects_outside_suggest_opt_out() {
        let suggestions = parse_suggestions(
            r#"{
                "conflicts": [],
                "bloat": [
                  {"crate": "demo", "feature": "full", "reason": "not a default rule", "message": "heavy"}
                ],
                "default_features": {"suggest_opt_out": []}
            }"#,
        );

        assert!(suggestions.default_features.is_empty());
    }

    #[test]
    fn emits_default_feature_finding_when_default_is_active() {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "log 0.4.0".to_string(),
            FeatureNode {
                package_id: "log 0.4.0".to_string(),
                name: "log".to_string(),
                version: "0.4.0".to_string(),
                active_features: BTreeSet::from(["default".to_string(), "std".to_string()]),
                ..FeatureNode::default()
            },
        );
        let suggestions = Suggestions {
            default_features: vec![DefaultFeatureRule {
                crate_name: "log".to_string(),
                severity: Severity::Info,
                reason: "review log defaults".to_string(),
            }],
            ..Suggestions::default()
        };

        let findings = default_features(&AnalysisContext::new(&graph, &suggestions));

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::DefaultFeature);
        assert_eq!(findings[0].severity, Severity::Info);
        assert_eq!(findings[0].feature.as_deref(), Some("default"));
        assert_eq!(findings[0].message, "review log defaults");
    }

    #[test]
    fn skips_default_feature_finding_when_default_is_disabled() {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "log 0.4.0".to_string(),
            FeatureNode {
                package_id: "log 0.4.0".to_string(),
                name: "log".to_string(),
                version: "0.4.0".to_string(),
                active_features: BTreeSet::from(["std".to_string()]),
                ..FeatureNode::default()
            },
        );
        let suggestions = Suggestions {
            default_features: vec![DefaultFeatureRule {
                crate_name: "log".to_string(),
                severity: Severity::Info,
                reason: "review log defaults".to_string(),
            }],
            ..Suggestions::default()
        };

        let findings = default_features(&AnalysisContext::new(&graph, &suggestions));

        assert!(findings.is_empty());
    }

    #[test]
    fn parses_local_toml_rules() {
        let suggestions = parse_local_suggestions(
            r#"
[[conflicts]]
crate = "reqwest"
features = ["native-tls", "rustls-tls"]
severity = "error"
message = "exclusive"

[[bloat]]
crate = "serde"
feature = "derive"
pulls_in = ["syn", "quote"]
message = "macro heft"

[[default_features.suggest_opt_out]]
crate = "log"
reason = "trim defaults"
"#,
        )
        .expect("local rules parse");
        assert_eq!(suggestions.conflicts.len(), 1);
        assert_eq!(suggestions.bloat.len(), 1);
        assert_eq!(suggestions.default_features.len(), 1);
    }

    #[test]
    fn merges_suggestions_additively() {
        let mut left = Suggestions {
            conflicts: vec![ConflictRule {
                crate_name: "a".to_string(),
                features: vec!["x".to_string(), "y".to_string()],
                severity: Severity::Warning,
                message: "m".to_string(),
            }],
            ..Suggestions::default()
        };
        let right = Suggestions {
            bloat: vec![BloatRule {
                crate_name: "b".to_string(),
                feature: "f".to_string(),
                pulls_in: vec![],
                severity: Severity::Info,
                message: "bloat".to_string(),
            }],
            ..Suggestions::default()
        };
        left.extend(right);
        assert_eq!(left.conflicts.len(), 1);
        assert_eq!(left.bloat.len(), 1);
    }

    #[test]
    fn rejects_invalid_local_severity() {
        let err = parse_local_suggestions(
            r#"
[[conflicts]]
crate = "reqwest"
features = ["native-tls", "rustls-tls"]
severity = "bogus"
message = "exclusive"
"#,
        )
        .expect_err("should fail");
        assert!(err.contains("invalid `severity`: bogus"));
    }

    #[test]
    fn rejects_malformed_local_toml() {
        let err = parse_local_suggestions(
            r#"
[[conflicts]]
crate = reqwest
"#,
        )
        .expect_err("should fail");
        assert!(err.contains("unsupported value"));
    }
}
