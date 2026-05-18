use std::fs;
use std::path::Path;

use crate::resolver::FeatureGraph;

pub mod bloat;
pub mod conflicts;
pub mod duplication;
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
    pub fn load_optional(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)?;
        Ok(parse_suggestions(&raw))
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

fn parse_suggestions(raw: &str) -> Suggestions {
    let mut suggestions = Suggestions::default();
    for object in json_like_objects(raw) {
        if let Some(crate_name) = field(&object, "crate") {
            if let Some(features) = array_field(&object, "features") {
                if let Some(message) = field(&object, "message") {
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
            } else if let Some(feature) = field(&object, "feature") {
                if let Some(message) = field(&object, "message") {
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
            } else if let Some(reason) = field(&object, "reason") {
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
    }
    suggestions
}

fn json_like_objects(raw: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in raw.char_indices() {
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
            '{' => {
                if depth == 1 {
                    start = Some(idx);
                }
                depth += 1;
            }
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 1 {
                    if let Some(start) = start.take() {
                        objects.push(raw[start..=idx].to_string());
                    }
                }
            }
            _ => {}
        }
    }
    objects
}

fn field(object: &str, key: &str) -> Option<String> {
    let key = format!("\"{key}\"");
    let start = object.find(&key)? + key.len();
    let tail = object[start..].split_once(':')?.1.trim();
    parse_json_string(tail).map(|(value, _)| value)
}

fn array_field(object: &str, key: &str) -> Option<Vec<String>> {
    let key = format!("\"{key}\"");
    let start = object.find(&key)? + key.len();
    let tail = object[start..].split_once('[')?.1;
    let end = find_array_end(tail)?;
    Some(string_items(&tail[..end]))
}

fn find_array_end(input: &str) -> Option<usize> {
    let mut in_string = false;
    let mut escaped = false;
    for (idx, ch) in input.char_indices() {
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
        } else if ch == ']' {
            return Some(idx);
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
    fn parses_all_suggestion_rule_kinds() {
        let suggestions = parse_suggestions(include_str!("../../docs/suggestions.json"));

        assert!(!suggestions.conflicts.is_empty());
        assert!(!suggestions.bloat.is_empty());
        assert!(!suggestions.default_features.is_empty());
    }
}
