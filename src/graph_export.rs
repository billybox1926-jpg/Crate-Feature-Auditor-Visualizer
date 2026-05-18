use std::collections::{BTreeMap, BTreeSet};

use crate::resolver::{FeatureGraph, FeatureNode};

/// Render the resolved dependency graph as Graphviz DOT.
pub fn render_dot(graph: &FeatureGraph, crate_filter: Option<&str>) -> String {
    let visible_nodes = visible_nodes(graph, crate_filter);
    let node_ids = stable_node_ids(&visible_nodes, "crate");
    let edges = dependency_edges(&visible_nodes, &node_ids);
    let mut output = String::new();

    output.push_str("digraph feature_lens {\n");
    for node in &visible_nodes {
        let id = node_ids
            .get(node.package_id.as_str())
            .expect("visible nodes should have stable ids");
        output.push_str(&format!(
            "  \"{}\" [label=\"{}\"];\n",
            dot_escape(id),
            dot_escape(&node_label(node))
        ));
    }
    for edge in edges {
        output.push_str(&format!(
            "  \"{}\" -> \"{}\";\n",
            dot_escape(&edge.from_id),
            dot_escape(&edge.to_id)
        ));
    }
    output.push_str("}\n");
    output
}

/// Render the resolved dependency graph as Markdown-friendly Mermaid syntax.
pub fn render_mermaid(graph: &FeatureGraph, crate_filter: Option<&str>) -> String {
    let visible_nodes = visible_nodes(graph, crate_filter);
    let node_ids = stable_node_ids(&visible_nodes, "crate");
    let edges = dependency_edges(&visible_nodes, &node_ids);
    let mut output = String::new();

    output.push_str("graph TD\n");
    for node in &visible_nodes {
        let id = node_ids
            .get(node.package_id.as_str())
            .expect("visible nodes should have stable ids");
        output.push_str(&format!(
            "  {}[\"{}\"]\n",
            id,
            mermaid_label_escape(&node_label(node))
        ));
    }
    for edge in edges {
        output.push_str(&format!("  {} --> {}\n", edge.from_id, edge.to_id));
    }
    output
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct GraphEdge {
    from_id: String,
    to_id: String,
}

fn visible_nodes<'a>(graph: &'a FeatureGraph, crate_filter: Option<&str>) -> Vec<&'a FeatureNode> {
    graph
        .sorted_nodes()
        .filter(|node| crate_matches(&node.name, crate_filter))
        .collect()
}

fn dependency_edges(
    visible_nodes: &[&FeatureNode],
    node_ids: &BTreeMap<&str, String>,
) -> Vec<GraphEdge> {
    let visible_package_ids = visible_nodes
        .iter()
        .map(|node| node.package_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut edges = BTreeSet::new();

    for node in visible_nodes {
        let Some(from_id) = node_ids.get(node.package_id.as_str()) else {
            continue;
        };
        for dependency in &node.dependencies {
            if !visible_package_ids.contains(dependency.as_str()) {
                continue;
            }
            let Some(to_id) = node_ids.get(dependency.as_str()) else {
                continue;
            };
            edges.insert(GraphEdge {
                from_id: from_id.clone(),
                to_id: to_id.clone(),
            });
        }
    }

    edges.into_iter().collect()
}

fn stable_node_ids<'a>(
    visible_nodes: &[&'a FeatureNode],
    prefix: &str,
) -> BTreeMap<&'a str, String> {
    let mut used = BTreeSet::new();
    let mut node_ids = BTreeMap::new();

    for node in visible_nodes {
        let base = sanitize_id(&node.package_id, prefix);
        let mut candidate = base.clone();
        let mut suffix = 2;
        while !used.insert(candidate.clone()) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }
        node_ids.insert(node.package_id.as_str(), candidate);
    }

    node_ids
}

fn node_label(node: &FeatureNode) -> String {
    let mut label = format!("{} {}", node.name, node.version);
    if !node.active_features.is_empty() {
        label.push('\n');
        label.push_str("features: ");
        label.push_str(
            &node
                .active_features
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    label
}

fn sanitize_id(value: &str, prefix: &str) -> String {
    let mut sanitized = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty()
        || sanitized
            .chars()
            .next()
            .map(|ch| ch.is_ascii_digit())
            .unwrap_or(true)
    {
        sanitized.insert(0, '_');
    }

    format!("{prefix}_{sanitized}")
}

fn dot_escape(value: &str) -> String {
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

fn mermaid_label_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("#quot;"),
            '\n' | '\r' => escaped.push_str("<br/>"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '&' => escaped.push_str("&amp;"),
            ch if ch.is_control() => escaped.push(' '),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn crate_matches(name: &str, filter: Option<&str>) -> bool {
    filter.map(|filter| name.contains(filter)).unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::FeatureGraph;

    #[test]
    fn dot_escapes_labels_and_sanitizes_ids() {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "pkg+demo 0.1.0 (registry+https://example.com?q=\"x\")".to_string(),
            FeatureNode {
                package_id: "pkg+demo 0.1.0 (registry+https://example.com?q=\"x\")".to_string(),
                name: "demo\"crate".to_string(),
                version: "0.1.0".to_string(),
                active_features: BTreeSet::from([
                    "line\\feature".to_string(),
                    "quote\"feature".to_string(),
                ]),
                ..FeatureNode::default()
            },
        );

        let rendered = render_dot(&graph, None);

        assert!(rendered.contains("\"crate_pkg_demo_0_1_0__registry_https___example_com_q__x__\""));
        assert!(
            rendered.contains("demo\\\"crate 0.1.0\\nfeatures: line\\\\feature, quote\\\"feature")
        );
    }

    #[test]
    fn dot_edges_are_sorted_and_ignore_missing_dependencies() {
        let graph = graph_with_unsorted_dependencies();

        let rendered = render_dot(&graph, None);

        let edge_a = rendered
            .find("\"crate_root_0_1_0\" -> \"crate_a_dep_0_1_0\"")
            .unwrap();
        let edge_b = rendered
            .find("\"crate_root_0_1_0\" -> \"crate_b_dep_0_1_0\"")
            .unwrap();
        assert!(edge_a < edge_b);
        assert!(!rendered.contains("missing 0.1.0"));
    }

    #[test]
    fn mermaid_escapes_labels_and_sanitizes_ids() {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "1 weird/package".to_string(),
            FeatureNode {
                package_id: "1 weird/package".to_string(),
                name: "demo <crate> & \"quotes\"".to_string(),
                version: "0.1.0".to_string(),
                active_features: BTreeSet::from(["default".to_string()]),
                ..FeatureNode::default()
            },
        );

        let rendered = render_mermaid(&graph, None);

        assert!(rendered.contains("crate__1_weird_package["));
        assert!(rendered
            .contains("demo &lt;crate&gt; &amp; #quot;quotes#quot; 0.1.0<br/>features: default"));
    }

    #[test]
    fn mermaid_edges_are_sorted_and_ignore_missing_dependencies() {
        let graph = graph_with_unsorted_dependencies();

        let rendered = render_mermaid(&graph, None);

        let edge_a = rendered
            .find("crate_root_0_1_0 --> crate_a_dep_0_1_0")
            .unwrap();
        let edge_b = rendered
            .find("crate_root_0_1_0 --> crate_b_dep_0_1_0")
            .unwrap();
        assert!(edge_a < edge_b);
        assert!(!rendered.contains("missing 0.1.0"));
    }

    fn graph_with_unsorted_dependencies() -> FeatureGraph {
        let mut graph = FeatureGraph::default();
        graph.nodes.insert(
            "root 0.1.0".to_string(),
            FeatureNode {
                package_id: "root 0.1.0".to_string(),
                name: "root".to_string(),
                version: "0.1.0".to_string(),
                dependencies: vec![
                    "b-dep 0.1.0".to_string(),
                    "missing 0.1.0".to_string(),
                    "a-dep 0.1.0".to_string(),
                ],
                ..FeatureNode::default()
            },
        );
        graph.nodes.insert(
            "b-dep 0.1.0".to_string(),
            FeatureNode {
                package_id: "b-dep 0.1.0".to_string(),
                name: "b-dep".to_string(),
                version: "0.1.0".to_string(),
                ..FeatureNode::default()
            },
        );
        graph.nodes.insert(
            "a-dep 0.1.0".to_string(),
            FeatureNode {
                package_id: "a-dep 0.1.0".to_string(),
                name: "a-dep".to_string(),
                version: "0.1.0".to_string(),
                ..FeatureNode::default()
            },
        );
        graph
    }
}
