use crate::analysis::{Finding, FindingKind};
use crate::resolver::{FeatureGraph, FeatureNode};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OutputFormat {
    Terminal,
    Markdown,
}

#[derive(Debug, Clone)]
pub struct ReportOptions {
    pub format: OutputFormat,
    pub only_unused: bool,
    pub only_bloat: bool,
    pub crate_filter: Option<String>,
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
    }
}

fn filter_findings<'a>(findings: &'a [Finding], options: &ReportOptions) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|finding| !options.only_unused || finding.kind == FindingKind::Unused)
        .filter(|finding| !options.only_bloat || finding.kind == FindingKind::Bloat)
        .filter(|finding| crate_matches(&finding.crate_name, options.crate_filter.as_deref()))
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
            },
        )
        .unwrap();

        assert!(rendered.contains("# Feature Footprint Report"));
        assert!(rendered.contains("demo"));
    }
}
