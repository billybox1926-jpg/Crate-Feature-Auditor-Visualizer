use crate::analysis::{AnalysisContext, Finding, FindingKind, Severity};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        for rule in &context.suggestions.bloat {
            if rule.crate_name == node.name && node.active_features.contains(&rule.feature) {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(rule.feature.clone()),
                    kind: FindingKind::Bloat,
                    severity: Severity::Info,
                    message: rule.message.clone(),
                });
            }
        }
    }

    findings
}
