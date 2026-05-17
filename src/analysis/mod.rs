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
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
}

/// Optional suggestion database loaded from `suggestions.json`.
#[derive(Debug, Clone, Default)]
pub struct Suggestions {
    pub conflicts: Vec<ConflictRule>,
    pub bloat: Vec<BloatRule>,
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
    pub message: String,
}

pub fn run_all(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();
    findings.extend(unused::analyze(context));
    findings.extend(conflicts::analyze(context));
    findings.extend(duplication::analyze(context));
    findings.extend(bloat::analyze(context));
    findings.sort_by(|left, right| {
        left.crate_name
            .cmp(&right.crate_name)
            .then_with(|| format!("{:?}", left.kind).cmp(&format!("{:?}", right.kind)))
            .then_with(|| left.feature.cmp(&right.feature))
    });
    findings
}

fn parse_suggestions(raw: &str) -> Suggestions {
    let mut suggestions = Suggestions::default();
    for object in json_like_objects(raw) {
        if let (Some(crate_name), Some(message)) =
            (field(&object, "crate"), field(&object, "message"))
        {
            if let Some(features) = array_field(&object, "features") {
                suggestions.conflicts.push(ConflictRule {
                    crate_name,
                    features,
                    severity: field(&object, "severity")
                        .as_deref()
                        .map(parse_severity)
                        .unwrap_or(Severity::Warning),
                    message,
                });
            } else if let Some(feature) = field(&object, "feature") {
                suggestions.bloat.push(BloatRule {
                    crate_name,
                    feature,
                    pulls_in: array_field(&object, "pulls_in").unwrap_or_default(),
                    message,
                });
            }
        }
    }
    suggestions
}

fn parse_severity(value: &str) -> Severity {
    match value {
        "error" => Severity::Error,
        "info" => Severity::Info,
        _ => Severity::Warning,
    }
}

fn json_like_objects(raw: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut depth = 0usize;
    let mut start = None;
    for (idx, ch) in raw.char_indices() {
        match ch {
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
    let first = tail.find('"')? + 1;
    let rest = &tail[first..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn array_field(object: &str, key: &str) -> Option<Vec<String>> {
    let key = format!("\"{key}\"");
    let start = object.find(&key)? + key.len();
    let tail = object[start..].split_once('[')?.1;
    let end = tail.find(']')?;
    Some(
        tail[..end]
            .split(',')
            .filter_map(|item| {
                let item = item.trim();
                if item.starts_with('"') && item.ends_with('"') {
                    Some(item.trim_matches('"').to_string())
                } else {
                    None
                }
            })
            .collect(),
    )
}
