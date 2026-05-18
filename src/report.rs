use crate::analysis::{Finding, FindingKind, Severity};
use crate::resolver::{FeatureGraph, FeatureNode};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OutputFormat {
    Terminal,
    Markdown,
    Json,
}

#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub format: OutputFormat,
    pub only_unused: bool,
    pub only_bloat: bool,
    pub crate_filter: Option<String>,
    pub min_severity: Option<Severity>,
}

pub fn render(
    graph: &FeatureGraph,
    findings: &[Finding],
    options: &ReportOptions,
) -> Result<String, Box<dyn std::error::Error>> {
    let findings = filter_findings(findings, options);

    match options.format {
        OutputFormat::Terminal => Ok(render_terminal(
            graph,
            &findings,
            options.crate_filter.as_deref(),
        )),
        OutputFormat::Markdown => Ok(render_markdown(
            graph,
            &findings,
            options.crate_filter.as_deref(),
        )),
        OutputFormat::Json => Ok(render_json(
            graph,
            &findings,
            options.crate_filter.as_deref(),
        )),
    }
}

fn filter_findings<'a>(findings: &'a [Finding], options: &ReportOptions) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|finding| !options.only_unused || finding.kind == FindingKind::Unused)
        .filter(|finding| !options.only_bloat || finding.kind == FindingKind::Bloat)
        .filter(|finding| crate_matches(&finding.crate_name, options.crate_filter.as_deref()))
        .filter(|finding| {
            options
                .min_severity
                .map(|minimum| finding.severity >= minimum)
                .unwrap_or(true)
        })
        .collect()
}

fn render_terminal(
    graph: &FeatureGraph,
    findings: &[&Finding],
    crate_filter: Option<&str>,
) -> String {
    let visible_nodes = visible_nodes(graph, crate_filter);
    let total_features: usize = visible_nodes
        .iter()
        .map(|node| node.active_features.len())
        .sum();
    let mut output = String::new();

    output.push_str("Feature Footprint Report\n");
    output.push_str("────────────────────────\n");
    output.push_str(&format!(
        "✓ {total_features} total features active across {} crates\n",
        visible_nodes.len()
    ));
    output.push_str(&format!("⚠ {} findings detected\n\n", findings.len()));

    for node in visible_nodes {
        output.push_str(&format!("┌─ {} ({})\n", node.name, node.version));
        output.push_str(&format!(
            "│  dependencies: {} direct, {} optional\n",
            node.dependencies.len(),
            node.optional_dependencies.len()
        ));
        if node.active_features.is_empty() {
            output.push_str("│  active features: <none>\n");
        } else {
            output.push_str(&format!(
                "│  active features: {}\n",
                node.active_features
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
        for finding in findings
            .iter()
            .filter(|finding| finding.crate_name == node.name)
        {
            output.push_str(&format!(
                "│  {} {:?}: {}\n",
                finding.severity.icon(),
                finding.kind,
                finding.message
            ));
            if let Some(feature) = &finding.feature {
                if let Some(sources) = node.feature_sources.get(feature) {
                    let requesters = sources
                        .iter()
                        .map(|source| source.requested_by.as_str())
                        .collect::<std::collections::BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                        .join(", ");
                    if !requesters.is_empty() {
                        output.push_str(&format!("│    requested by: {requesters}\n"));
                    }
                }
            }
        }
        output.push_str("│\n");
    }

    output
}

fn render_markdown(
    graph: &FeatureGraph,
    findings: &[&Finding],
    crate_filter: Option<&str>,
) -> String {
    let visible_nodes = visible_nodes(graph, crate_filter);
    let mut output = String::new();

    output.push_str("# Feature Footprint Report\n\n");
    output.push_str(&format!("Analyzed **{}** crates.\n\n", visible_nodes.len()));
    output.push_str("| Crate | Version | Active Features | Findings |\n");
    output.push_str("|---|---:|---|---|\n");

    for node in visible_nodes {
        let features = if node.active_features.is_empty() {
            "_none_".to_string()
        } else {
            node.active_features
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        };
        let crate_findings = findings
            .iter()
            .filter(|finding| finding.crate_name == node.name)
            .map(|finding| {
                format!(
                    "{} {:?}: {}",
                    finding.severity.icon(),
                    finding.kind,
                    finding.message
                )
            })
            .collect::<Vec<_>>()
            .join("<br>");
        let crate_findings = if crate_findings.is_empty() {
            "_none_".to_string()
        } else {
            crate_findings
        };

        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            node.name, node.version, features, crate_findings
        ));
    }

    output
}

fn render_json(graph: &FeatureGraph, findings: &[&Finding], crate_filter: Option<&str>) -> String {
    let visible_nodes = visible_nodes(graph, crate_filter);
    let total_features: usize = visible_nodes
        .iter()
        .map(|node| node.active_features.len())
        .sum();
    let mut output = String::new();

    output.push_str("{\n");
    output.push_str(&format!("  \"crate_count\": {},\n", visible_nodes.len()));
    output.push_str(&format!("  \"total_active_features\": {total_features},\n"));
    output.push_str("  \"crates\": [\n");

    for (index, node) in visible_nodes.iter().enumerate() {
        if index > 0 {
            output.push_str(",\n");
        }
        output.push_str("    {\n");
        output.push_str(&format!(
            "      \"name\": \"{}\",\n",
            json_escape(&node.name)
        ));
        output.push_str(&format!(
            "      \"version\": \"{}\",\n",
            json_escape(&node.version)
        ));
        output.push_str(&format!(
            "      \"package_id\": \"{}\",\n",
            json_escape(&node.package_id)
        ));
        output.push_str(&format!(
            "      \"manifest_path\": \"{}\",\n",
            json_escape(&node.manifest_path)
        ));
        output.push_str("      \"active_features\": [");
        for (feature_index, feature) in node.active_features.iter().enumerate() {
            if feature_index > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("\"{}\"", json_escape(feature)));
        }
        output.push_str("],\n");
        output.push_str("      \"dependencies\": [");
        for (dependency_index, dependency) in node.dependencies.iter().enumerate() {
            if dependency_index > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("\"{}\"", json_escape(dependency)));
        }
        output.push_str("],\n");
        output.push_str("      \"optional_dependencies\": [");
        for (dependency_index, dependency) in node.optional_dependencies.iter().enumerate() {
            if dependency_index > 0 {
                output.push_str(", ");
            }
            output.push_str(&format!("\"{}\"", json_escape(dependency)));
        }
        output.push_str("],\n");
        output.push_str("      \"dependency_features\": {");
        for (dep_index, (dependency, features)) in node.dependency_features.iter().enumerate() {
            if dep_index > 0 {
                output.push(',');
            }
            output.push_str(&format!("\n        \"{}\": [", json_escape(dependency)));
            for (feature_index, feature) in features.iter().enumerate() {
                if feature_index > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!("\"{}\"", json_escape(feature)));
            }
            output.push(']');
        }
        if !node.dependency_features.is_empty() {
            output.push_str("\n      ");
        }
        output.push_str("},\n");
        output.push_str("      \"feature_sources\": {");
        for (feature_index, (feature, sources)) in node.feature_sources.iter().enumerate() {
            if feature_index > 0 {
                output.push(',');
            }
            output.push_str(&format!("\n        \"{}\": [", json_escape(feature)));
            for (source_index, source) in sources.iter().enumerate() {
                if source_index > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!(
                    "{{\"requested_by\": \"{}\", \"path\": [",
                    json_escape(&source.requested_by)
                ));
                for (path_index, item) in source.path.iter().enumerate() {
                    if path_index > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("\"{}\"", json_escape(item)));
                }
                output.push_str("]}");
            }
            output.push(']');
        }
        if !node.feature_sources.is_empty() {
            output.push_str("\n      ");
        }
        output.push_str("},\n");
        output.push_str("      \"findings\": [");
        let node_findings = findings
            .iter()
            .filter(|finding| finding.crate_name == node.name)
            .collect::<Vec<_>>();
        for (finding_index, finding) in node_findings.iter().enumerate() {
            if finding_index > 0 {
                output.push(',');
            }
            output.push_str("\n        {");
            output.push_str(&format!("\n          \"kind\": \"{:?}\",", finding.kind));
            output.push_str(&format!(
                "\n          \"severity\": \"{:?}\",",
                finding.severity
            ));
            output.push_str("\n          \"feature\": ");
            if let Some(feature) = &finding.feature {
                output.push_str(&format!("\"{}\"", json_escape(feature)));
            } else {
                output.push_str("null");
            }
            output.push_str(&format!(
                ",\n          \"message\": \"{}\"\n        }}",
                json_escape(&finding.message)
            ));
        }
        if !node_findings.is_empty() {
            output.push_str("\n      ");
        }
        output.push_str("]\n    }");
    }

    output.push_str("\n  ]\n}\n");
    output
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn visible_nodes<'a>(graph: &'a FeatureGraph, crate_filter: Option<&str>) -> Vec<&'a FeatureNode> {
    graph
        .sorted_nodes()
        .filter(|node| crate_matches(&node.name, crate_filter))
        .collect()
}

fn crate_matches(name: &str, filter: Option<&str>) -> bool {
    filter.map(|filter| name.contains(filter)).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::analysis::{Finding, Severity};
    use crate::resolver::FeatureNode;

    #[test]
    fn renders_json_report() {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "demo 0.1.0".to_string(),
            FeatureNode {
                package_id: "demo 0.1.0".to_string(),
                name: "demo".to_string(),
                version: "0.1.0".to_string(),
                active_features: BTreeSet::from(["default".to_string()]),
                ..FeatureNode::default()
            },
        );
        let rendered = render(
            &graph,
            &[],
            &ReportOptions {
                format: OutputFormat::Json,
                only_unused: false,
                only_bloat: false,
                crate_filter: None,
                min_severity: None,
            },
        )
        .unwrap();

        assert!(rendered.contains("\"crate_count\": 1"));
        assert!(rendered.contains("\"name\": \"demo\""));
    }

    #[test]
    fn renders_markdown_report() {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "demo 0.1.0".to_string(),
            FeatureNode {
                name: "demo".to_string(),
                version: "0.1.0".to_string(),
                active_features: BTreeSet::from(["default".to_string()]),
                ..FeatureNode::default()
            },
        );
        let findings = vec![Finding {
            crate_name: "demo".to_string(),
            feature: Some("default".to_string()),
            kind: FindingKind::Duplicate,
            severity: Severity::Warning,
            message: "duplicated".to_string(),
        }];
        let rendered = render(
            &graph,
            &findings,
            &ReportOptions {
                format: OutputFormat::Markdown,
                only_unused: false,
                only_bloat: false,
                crate_filter: None,
                min_severity: None,
            },
        )
        .unwrap();

        assert!(rendered.contains("# Feature Footprint Report"));
        assert!(rendered.contains("demo"));
    }
}
