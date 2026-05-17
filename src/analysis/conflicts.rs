use crate::analysis::{AnalysisContext, Finding, FindingKind};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        for rule in &context.suggestions.conflicts {
            if rule.crate_name != node.name {
                continue;
            }
            if rule
                .features
                .iter()
                .all(|feature| node.active_features.contains(feature))
            {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(rule.features.join(", ")),
                    kind: FindingKind::Conflict,
                    severity: rule.severity,
                    message: rule.message.clone(),
                });
            }
        }
    }

    findings
}
