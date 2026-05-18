use std::collections::{BTreeMap, BTreeSet};

use crate::manifest::ManifestCache;
use crate::metadata::{Metadata, ResolveNode};

/// A best-effort, enriched dependency graph keyed by Cargo package ID.
#[derive(Debug, Clone, Default)]
pub struct FeatureGraph {
    pub workspace_members: BTreeSet<String>,
    pub nodes: BTreeMap<String, FeatureNode>,
}

impl FeatureGraph {
    pub fn sorted_nodes(&self) -> impl Iterator<Item = &FeatureNode> {
        self.nodes.values()
    }

    pub fn get(&self, package_id: &str) -> Option<&FeatureNode> {
        self.nodes.get(package_id)
    }
}

/// Feature information for a single resolved package.
#[derive(Debug, Clone, Default)]
pub struct FeatureNode {
    pub package_id: String,
    pub name: String,
    pub version: String,
    pub manifest_path: String,
    pub active_features: BTreeSet<String>,
    pub available_features: BTreeMap<String, Vec<String>>,
    pub optional_dependencies: BTreeSet<String>,
    pub dependency_features: BTreeMap<String, Vec<String>>,
    pub feature_sources: BTreeMap<String, Vec<FeatureSource>>,
    pub dependencies: Vec<String>,
}

/// A recorded reason that a feature is active.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FeatureSource {
    pub requested_by: String,
    pub path: Vec<String>,
}

/// Build an initial feature graph from `cargo metadata` and package manifests.
pub fn resolve(
    metadata: &Metadata,
    manifests: &mut ManifestCache,
) -> Result<FeatureGraph, Box<dyn std::error::Error>> {
    let mut graph = FeatureGraph {
        workspace_members: metadata.workspace_members.iter().cloned().collect(),
        ..FeatureGraph::default()
    };

    for package in &metadata.packages {
        let manifest = manifests.get_or_parse(&package.manifest_path)?;
        let package_id = package.id.clone();
        let mut available_features = manifest.features;
        merge_feature_map(&mut available_features, package.features.clone());
        sort_feature_map(&mut available_features);
        let mut optional_dependencies = manifest.optional_dependencies;
        optional_dependencies.extend(package.optional_dependencies.clone());
        let mut dependency_features = manifest.dependency_features;
        merge_feature_map(
            &mut dependency_features,
            package.dependency_features.clone(),
        );
        sort_feature_map(&mut dependency_features);

        graph.nodes.insert(
            package_id.clone(),
            FeatureNode {
                package_id,
                name: package.name.clone(),
                version: package.version.clone(),
                manifest_path: package.manifest_path.display().to_string(),
                available_features,
                optional_dependencies,
                dependency_features,
                ..FeatureNode::default()
            },
        );
    }

    for node in &metadata.resolve_nodes {
        let package_id = node.id.clone();
        if let Some(feature_node) = graph.nodes.get_mut(&package_id) {
            feature_node
                .active_features
                .extend(node.features.iter().cloned());
            feature_node.dependencies = sorted_unique(node.dependencies.clone());
            let self_name = feature_node.name.clone();
            let active_features = feature_node.active_features.clone();
            for feature in active_features {
                add_feature_source(
                    feature_node,
                    &feature,
                    FeatureSource {
                        requested_by: self_name.clone(),
                        path: vec![self_name.clone()],
                    },
                );
            }
        }
    }

    record_dependency_sources(&mut graph, &metadata.resolve_nodes);
    record_manifest_feature_sources(&mut graph, &metadata.resolve_nodes);
    Ok(graph)
}

fn record_dependency_sources(graph: &mut FeatureGraph, nodes: &[ResolveNode]) {
    let resolve_nodes = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();

    for parent_id in graph.nodes.keys().cloned().collect::<Vec<_>>() {
        let Some(metadata_node) = resolve_nodes.get(parent_id.as_str()).copied() else {
            continue;
        };
        let Some(parent) = graph.get(&parent_id) else {
            continue;
        };
        let parent_name = parent.name.clone();
        let dependency_features = parent.dependency_features.clone();

        for (dependency_name, child_id) in &metadata_node.dependency_names {
            let child_name = graph
                .nodes
                .get(child_id)
                .map(|child| child.name.clone())
                .unwrap_or_else(|| dependency_name.clone());
            let requested_features = dependency_features
                .get(dependency_name)
                .or_else(|| dependency_features.get(&child_name))
                .cloned()
                .unwrap_or_default();
            if requested_features.is_empty() {
                continue;
            }

            for feature in requested_features {
                let Some(child) = graph.nodes.get_mut(child_id) else {
                    continue;
                };
                if child.active_features.contains(&feature) {
                    add_feature_source(
                        child,
                        &feature,
                        FeatureSource {
                            requested_by: format!("{parent_name}/{child_name}"),
                            path: vec![parent_name.clone(), child_name.clone(), feature.clone()],
                        },
                    );
                }
            }
        }
    }
}

fn record_manifest_feature_sources(graph: &mut FeatureGraph, nodes: &[ResolveNode]) {
    let resolve_nodes = nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<BTreeMap<_, _>>();
    let package_names = graph
        .nodes
        .iter()
        .map(|(id, node)| (node.name.clone(), id.clone()))
        .collect::<BTreeMap<_, _>>();

    for package_id in graph.nodes.keys().cloned().collect::<Vec<_>>() {
        let Some(node_snapshot) = graph.nodes.get(&package_id).cloned() else {
            continue;
        };
        let metadata_node = resolve_nodes.get(package_id.as_str()).copied();

        for feature in &node_snapshot.active_features {
            let Some(expanded) = node_snapshot.available_features.get(feature) else {
                continue;
            };
            for item in expanded {
                if let Some((dependency_name, dependency_feature)) = item.split_once('/') {
                    let child_id = metadata_node
                        .and_then(|node| node.dependency_names.get(dependency_name))
                        .cloned()
                        .or_else(|| package_names.get(dependency_name).cloned());
                    let Some(child_id) = child_id else { continue };
                    let Some(child) = graph.nodes.get_mut(&child_id) else {
                        continue;
                    };
                    if child.active_features.contains(dependency_feature) {
                        add_feature_source(
                            child,
                            dependency_feature,
                            FeatureSource {
                                requested_by: format!("{}/{}", node_snapshot.name, feature),
                                path: vec![
                                    node_snapshot.name.clone(),
                                    feature.clone(),
                                    dependency_name.to_string(),
                                    dependency_feature.to_string(),
                                ],
                            },
                        );
                    }
                    continue;
                }

                let normalized = normalize_feature_reference(item);
                if let Some(node) = graph.nodes.get_mut(&package_id) {
                    if node.active_features.contains(&normalized) {
                        add_feature_source(
                            node,
                            &normalized,
                            FeatureSource {
                                requested_by: format!("{}/{}", node_snapshot.name, feature),
                                path: vec![
                                    node_snapshot.name.clone(),
                                    feature.clone(),
                                    normalized.clone(),
                                ],
                            },
                        );
                    }
                }
            }
        }
    }
}

fn add_feature_source(node: &mut FeatureNode, feature: &str, source: FeatureSource) {
    let sources = node.feature_sources.entry(feature.to_string()).or_default();
    if !sources.contains(&source) {
        sources.push(source);
        sources.sort_by(|left, right| {
            left.requested_by
                .cmp(&right.requested_by)
                .then_with(|| left.path.cmp(&right.path))
        });
    }
}

fn merge_feature_map(
    target: &mut BTreeMap<String, Vec<String>>,
    source: BTreeMap<String, Vec<String>>,
) {
    for (feature, values) in source {
        target.entry(feature).or_default().extend(values);
    }
}

fn sort_feature_map(map: &mut BTreeMap<String, Vec<String>>) {
    for values in map.values_mut() {
        *values = sorted_unique(std::mem::take(values));
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn normalize_feature_reference(feature: &str) -> String {
    feature
        .strip_prefix("dep:")
        .unwrap_or(feature)
        .split('/')
        .next_back()
        .unwrap_or(feature)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_feature_reference;
    use crate::manifest::ManifestCache;
    use crate::{metadata, resolver};
    use std::path::Path;

    fn resolver_v2_graph() -> resolver::FeatureGraph {
        let metadata = metadata::load_metadata(Path::new("tests/fixtures/resolver-v2"))
            .expect("fixture metadata should load");
        resolver::resolve(&metadata, &mut ManifestCache::default())
            .expect("fixture graph should resolve")
    }

    fn node<'a>(graph: &'a resolver::FeatureGraph, name: &str) -> &'a resolver::FeatureNode {
        graph
            .sorted_nodes()
            .find(|node| node.name == name)
            .unwrap_or_else(|| panic!("missing node {name}"))
    }

    #[test]
    fn normalizes_dep_and_namespaced_features() {
        assert_eq!(normalize_feature_reference("dep:serde_json"), "serde_json");
        assert_eq!(normalize_feature_reference("serde/derive"), "derive");
    }

    #[test]
    fn uses_cargo_metadata_resolved_features_for_direct_dependency_features() {
        let graph = resolver_v2_graph();
        let leaf = node(&graph, "leaf");

        assert!(leaf.active_features.contains("direct-leaf"));
        assert!(leaf.feature_sources["direct-leaf"].iter().any(|source| {
            source.requested_by == "resolver-app/leaf"
                && source.path == ["resolver-app", "leaf", "direct-leaf"]
        }));
    }

    #[test]
    fn records_transitive_feature_propagation_from_manifest_expansion() {
        let graph = resolver_v2_graph();
        let leaf = node(&graph, "leaf");

        assert!(leaf.active_features.contains("propagated"));
        assert!(leaf.feature_sources["propagated"].iter().any(|source| {
            source.requested_by == "mid/propagated"
                && source.path == ["mid", "propagated", "leaf", "propagated"]
        }));
        assert!(leaf.feature_sources["mid-forward"].iter().any(|source| {
            source.requested_by == "mid/direct-mid"
                && source.path == ["mid", "direct-mid", "leaf", "mid-forward"]
        }));
    }

    #[test]
    fn includes_cargo_metadata_dev_and_build_dependency_resolution() {
        let graph = resolver_v2_graph();
        let helper = node(&graph, "helper");
        let leaf = node(&graph, "leaf");

        assert!(helper.active_features.contains("build-only"));
        assert!(leaf.active_features.contains("dev-only"));
        assert!(leaf.active_features.contains("build-transitive"));
        assert!(leaf.feature_sources["dev-only"].iter().any(|source| {
            source.requested_by == "resolver-app/leaf"
                && source.path == ["resolver-app", "leaf", "dev-only"]
        }));
        assert!(leaf.feature_sources["build-transitive"]
            .iter()
            .any(|source| {
                source.requested_by == "helper/leaf"
                    && source.path == ["helper", "leaf", "build-transitive"]
            }));
    }

    #[test]
    fn preserves_workspace_member_feature_unification_and_deterministic_edges() {
        let graph = resolver_v2_graph();
        let tool = node(&graph, "resolver-tool");
        let leaf = node(&graph, "leaf");

        assert_eq!(
            tool.dependency_features["leaf"],
            vec!["tool-only", "workspace-base"]
        );
        assert!(leaf.active_features.contains("workspace-base"));
        assert!(leaf.active_features.contains("tool-only"));
        assert_eq!(
            node(&graph, "resolver-app").dependencies,
            vec![
                node(&graph, "helper").package_id.clone(),
                leaf.package_id.clone(),
                node(&graph, "mid").package_id.clone(),
            ]
        );
    }
}
