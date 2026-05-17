use crate::analysis::{AnalysisContext, Finding, FindingKind, Severity};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
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

            if !referenced_by_feature
                && !known_dependency_feature
                && !node.optional_dependencies.contains(feature)
            {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(feature.clone()),
                    kind: FindingKind::Unused,
                    severity: Severity::Warning,
                    message: format!(
                        "feature `{feature}` is active but not referenced by manifest feature expansion"
                    ),
                });
            }
        }
    }

    findings
}
