use crate::analysis::{AnalysisContext, Finding, FindingKind};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        for rule in &context.suggestions.bloat {
            if rule.crate_name == node.name && node.active_features.contains(&rule.feature) {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(rule.feature.clone()),
                    kind: FindingKind::Bloat,
                    severity: rule.severity,
                    message: if rule.pulls_in.is_empty() {
                        rule.message.clone()
                    } else {
                        format!("{} Pulls in: {}.", rule.message, rule.pulls_in.join(", "))
                    },
                });
            }
        }
    }

    findings
}
