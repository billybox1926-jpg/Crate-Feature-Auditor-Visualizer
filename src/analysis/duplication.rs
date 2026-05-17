use crate::analysis::{AnalysisContext, Finding, FindingKind, Severity};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        for (feature, sources) in &node.feature_sources {
            let unique_requesters = sources
                .iter()
                .map(|source| source.requested_by.as_str())
                .collect::<std::collections::BTreeSet<_>>();
            if unique_requesters.len() > 1 {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(feature.clone()),
                    kind: FindingKind::Duplicate,
                    severity: Severity::Warning,
                    message: format!(
                        "feature `{feature}` is requested by multiple ancestors: {}",
                        unique_requesters.into_iter().collect::<Vec<_>>().join(", ")
                    ),
                });
            }
        }
    }

    findings
}
