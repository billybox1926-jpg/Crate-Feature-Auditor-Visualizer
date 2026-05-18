use std::collections::BTreeSet;

use crate::analysis::{AnalysisContext, Finding, FindingKind, Severity};
use crate::resolver::{FeatureNode, FeatureSource};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        for (feature, sources) in &node.feature_sources {
            let lineages = meaningful_lineages(node, sources);
            if lineages.len() > 1 {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(feature.clone()),
                    kind: FindingKind::Duplicate,
                    severity: Severity::Warning,
                    message: format!(
                        "feature `{feature}` is requested through multiple lineages: {}",
                        lineages.into_iter().collect::<Vec<_>>().join("; ")
                    ),
                });
            }
        }
    }

    findings
}

fn meaningful_lineages(node: &FeatureNode, sources: &[FeatureSource]) -> BTreeSet<String> {
    sources
        .iter()
        .filter(|source| !is_self_activation(node, source))
        .map(format_lineage)
        .collect()
}

fn is_self_activation(node: &FeatureNode, source: &FeatureSource) -> bool {
    source.requested_by == node.name
        && (source.path.is_empty()
            || (source.path.len() == 1 && source.path.first() == Some(&node.name)))
}

fn format_lineage(source: &FeatureSource) -> String {
    if source.path.is_empty() {
        source.requested_by.clone()
    } else {
        source.path.join(" -> ")
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::analyze;
    use crate::analysis::{AnalysisContext, FindingKind, Severity, Suggestions};
    use crate::resolver::{FeatureGraph, FeatureNode, FeatureSource};

    #[test]
    fn emits_one_duplicate_for_two_distinct_requesters() {
        let graph = graph_with_sources(vec![
            source("fixture-leaf", &["fixture-leaf"]),
            source("mid-a/fixture-leaf", &["mid-a", "fixture-leaf", "shared"]),
            source("mid-b/fixture-leaf", &["mid-b", "fixture-leaf", "shared"]),
        ]);
        let suggestions = Suggestions::default();
        let context = AnalysisContext::new(&graph, &suggestions);

        let findings = analyze(&context);

        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.crate_name, "fixture-leaf");
        assert_eq!(finding.feature.as_deref(), Some("shared"));
        assert_eq!(finding.kind, FindingKind::Duplicate);
        assert_eq!(finding.severity, Severity::Warning);
        assert_eq!(
            finding.message,
            "feature `shared` is requested through multiple lineages: mid-a -> fixture-leaf -> shared; mid-b -> fixture-leaf -> shared"
        );
    }

    #[test]
    fn does_not_emit_duplicate_for_one_requester() {
        let graph = graph_with_sources(vec![
            source("fixture-leaf", &["fixture-leaf"]),
            source("mid-a/fixture-leaf", &["mid-a", "fixture-leaf", "shared"]),
        ]);
        let suggestions = Suggestions::default();
        let context = AnalysisContext::new(&graph, &suggestions);

        assert!(analyze(&context).is_empty());
    }

    #[test]
    fn does_not_emit_duplicate_for_repeated_identical_sources() {
        let graph = graph_with_sources(vec![
            source("fixture-leaf", &["fixture-leaf"]),
            source("mid-a/fixture-leaf", &["mid-a", "fixture-leaf", "shared"]),
            source("mid-a/fixture-leaf", &["mid-a", "fixture-leaf", "shared"]),
        ]);
        let suggestions = Suggestions::default();
        let context = AnalysisContext::new(&graph, &suggestions);

        assert!(analyze(&context).is_empty());
    }

    fn graph_with_sources(sources: Vec<FeatureSource>) -> FeatureGraph {
        let mut nodes = BTreeMap::new();
        let mut feature_sources = BTreeMap::new();
        feature_sources.insert("shared".to_string(), sources);
        nodes.insert(
            "path+file:///fixture-leaf#0.1.0".to_string(),
            FeatureNode {
                package_id: "path+file:///fixture-leaf#0.1.0".to_string(),
                name: "fixture-leaf".to_string(),
                version: "0.1.0".to_string(),
                active_features: BTreeSet::from(["shared".to_string()]),
                feature_sources,
                ..FeatureNode::default()
            },
        );

        FeatureGraph {
            nodes,
            ..FeatureGraph::default()
        }
    }

    fn source(requested_by: &str, path: &[&str]) -> FeatureSource {
        FeatureSource {
            requested_by: requested_by.to_string(),
            path: path.iter().map(|item| item.to_string()).collect(),
        }
    }
}
