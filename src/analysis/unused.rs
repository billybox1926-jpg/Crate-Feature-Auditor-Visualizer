use crate::analysis::source_usage::scan_package_sources;
use crate::analysis::{AnalysisContext, Finding, FindingKind, Severity};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        let source_usage = scan_package_sources(&node.manifest_path);
        let source_referenced_features = source_usage.referenced_features();

        for feature in &node.active_features {
            if feature == "default" {
                continue;
            }
            let referenced_by_feature = node
                .available_features
                .values()
                .flatten()
                .any(|item| item == feature || item.ends_with(&format!("/{feature}")));
            let known_dependency_feature = node
                .dependency_features
                .values()
                .flatten()
                .any(|dependency_feature| dependency_feature == feature);

            let source_referenced = source_referenced_features.contains(feature);

            if !referenced_by_feature
                && !known_dependency_feature
                && !node.optional_dependencies.contains(feature)
                && !source_referenced
            {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(feature.clone()),
                    kind: FindingKind::Unused,
                    severity: Severity::Warning,
                    message: format!(
                        "feature `{feature}` is active but not referenced by manifest feature expansion or scanned Rust source cfg usage"
                    ),
                });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use super::*;
    use crate::analysis::Suggestions;
    use crate::resolver::{FeatureGraph, FeatureNode};

    #[test]
    fn suppresses_unused_when_feature_is_used_in_src_cfg() {
        let node = FeatureNode {
            package_id: "fixture 0.1.0".to_string(),
            name: "fixture".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: "tests/fixtures/unused-source-used/Cargo.toml".to_string(),
            active_features: BTreeSet::from(["lint_only".to_string()]),
            ..FeatureNode::default()
        };

        let graph = FeatureGraph {
            nodes: BTreeMap::from([(node.package_id.clone(), node)]),
            ..FeatureGraph::default()
        };

        let suggestions = Suggestions::default();
        let context = AnalysisContext::new(&graph, &suggestions);
        assert!(analyze(&context).is_empty());
    }

    #[test]
    fn keeps_unused_when_feature_not_referenced_in_source() {
        let node = FeatureNode {
            package_id: "fixture 0.1.0".to_string(),
            name: "fixture".to_string(),
            version: "0.1.0".to_string(),
            manifest_path: "tests/fixtures/unused-source-unused/Cargo.toml".to_string(),
            active_features: BTreeSet::from(["lint_only".to_string()]),
            ..FeatureNode::default()
        };

        let graph = FeatureGraph {
            nodes: BTreeMap::from([(node.package_id.clone(), node)]),
            ..FeatureGraph::default()
        };

        let suggestions = Suggestions::default();
        let context = AnalysisContext::new(&graph, &suggestions);
        let findings = analyze(&context);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::Unused);
    }
}
