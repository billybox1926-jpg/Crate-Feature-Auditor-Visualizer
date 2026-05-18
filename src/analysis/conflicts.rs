use crate::analysis::{AnalysisContext, Finding, FindingKind};

pub fn analyze(context: &AnalysisContext<'_>) -> Vec<Finding> {
    let mut findings = Vec::new();

    for node in context.graph.sorted_nodes() {
        for rule in &context.suggestions.conflicts {
            if rule.crate_name != node.name {
                continue;
            }

            let mut conflict_features = rule.features.clone();
            conflict_features.sort();
            conflict_features.dedup();
            if conflict_features.len() < 2 {
                continue;
            }

            if conflict_features
                .iter()
                .all(|feature| node.active_features.contains(feature))
            {
                findings.push(Finding {
                    crate_name: node.name.clone(),
                    feature: Some(conflict_features.join(", ")),
                    kind: FindingKind::Conflict,
                    severity: rule.severity,
                    message: rule.message.clone(),
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
    use crate::analysis::{ConflictRule, Severity, Suggestions};
    use crate::resolver::{FeatureGraph, FeatureNode};

    fn context_with(
        active_features: &[&str],
        rules: Vec<ConflictRule>,
    ) -> (FeatureGraph, Suggestions) {
        let mut nodes = BTreeMap::new();
        nodes.insert(
            "reqwest 0.0.0".to_string(),
            FeatureNode {
                package_id: "reqwest 0.0.0".to_string(),
                name: "reqwest".to_string(),
                version: "0.0.0".to_string(),
                active_features: active_features
                    .iter()
                    .map(|feature| feature.to_string())
                    .collect::<BTreeSet<_>>(),
                ..FeatureNode::default()
            },
        );

        (
            FeatureGraph {
                nodes,
                ..FeatureGraph::default()
            },
            Suggestions {
                conflicts: rules,
                ..Suggestions::default()
            },
        )
    }

    fn reqwest_tls_rule(severity: Severity) -> ConflictRule {
        ConflictRule {
            crate_name: "reqwest".to_string(),
            features: vec!["rustls-tls".to_string(), "native-tls".to_string()],
            severity,
            message: "TLS backends are mutually exclusive.".to_string(),
        }
    }

    #[test]
    fn emits_conflict_when_all_rule_features_are_active() {
        let (graph, suggestions) = context_with(
            &["native-tls", "rustls-tls"],
            vec![reqwest_tls_rule(Severity::Error)],
        );
        let context = AnalysisContext::new(&graph, &suggestions);

        let findings = analyze(&context);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].kind, FindingKind::Conflict);
        assert_eq!(findings[0].severity, Severity::Error);
        assert_eq!(
            findings[0].feature.as_deref(),
            Some("native-tls, rustls-tls")
        );
        assert_eq!(findings[0].message, "TLS backends are mutually exclusive.");
    }

    #[test]
    fn skips_conflict_when_only_one_rule_feature_is_active() {
        let (graph, suggestions) =
            context_with(&["native-tls"], vec![reqwest_tls_rule(Severity::Error)]);
        let context = AnalysisContext::new(&graph, &suggestions);

        assert!(analyze(&context).is_empty());
    }

    #[test]
    fn preserves_lower_severity_conflict_rules() {
        let (graph, suggestions) = context_with(
            &["native-tls", "rustls-tls"],
            vec![reqwest_tls_rule(Severity::Warning)],
        );
        let context = AnalysisContext::new(&graph, &suggestions);

        let findings = analyze(&context);

        assert_eq!(findings[0].severity, Severity::Warning);
    }

    #[test]
    fn ignores_malformed_rules_with_fewer_than_two_distinct_features() {
        let (graph, suggestions) = context_with(
            &["native-tls"],
            vec![ConflictRule {
                crate_name: "reqwest".to_string(),
                features: vec!["native-tls".to_string(), "native-tls".to_string()],
                severity: Severity::Error,
                message: "not enough distinct features".to_string(),
            }],
        );
        let context = AnalysisContext::new(&graph, &suggestions);

        assert!(analyze(&context).is_empty());
    }
}
