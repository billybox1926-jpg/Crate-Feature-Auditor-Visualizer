use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
        available_features.extend(package.features.clone());
        let mut optional_dependencies = manifest.optional_dependencies;
        optional_dependencies.extend(package.optional_dependencies.clone());
        let mut dependency_features = manifest.dependency_features;
        dependency_features.extend(package.dependency_features.clone());

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
            feature_node.dependencies = node.dependencies.clone();
            for feature in &node.features {
                feature_node
                    .feature_sources
                    .entry(feature.clone())
                    .or_default()
                    .push(FeatureSource {
                        requested_by: feature_node.name.clone(),
                        path: vec![feature_node.name.clone()],
                    });
            }
        }
    }

    record_dependency_sources(&mut graph, &metadata.resolve_nodes);
    record_manifest_feature_sources(&mut graph);
    Ok(graph)
}

fn record_dependency_sources(graph: &mut FeatureGraph, nodes: &[ResolveNode]) {
    let mut queue: VecDeque<(String, Vec<String>)> = graph
        .workspace_members
        .iter()
        .filter_map(|package_id| {
            graph
                .get(package_id)
                .map(|node| (package_id.clone(), vec![node.name.clone()]))
        })
        .collect();

    while let Some((package_id, path)) = queue.pop_front() {
        let Some(metadata_node) = nodes.iter().find(|node| node.id == package_id) else {
            continue;
        };
        let parent_name = graph
            .get(&package_id)
            .map(|node| node.name.clone())
            .unwrap_or_else(|| package_id.clone());

        for child_id in &metadata_node.dependencies {
            let child_id = child_id.clone();
            let mut child_path = path.clone();
            let child_name = graph
                .get(&child_id)
                .map(|node| node.name.clone())
                .unwrap_or_else(|| child_id.clone());
            child_path.push(child_name);

            if let Some(child) = graph.nodes.get_mut(&child_id) {
                for feature in &child.active_features.clone() {
                    child
                        .feature_sources
                        .entry(feature.clone())
                        .or_default()
                        .push(FeatureSource {
                            requested_by: parent_name.clone(),
                            path: child_path.clone(),
                        });
                }
            }
            queue.push_back((child_id, child_path));
        }
    }
}

fn record_manifest_feature_sources(graph: &mut FeatureGraph) {
    for node in graph.nodes.values_mut() {
        let active = node.active_features.clone();
        for feature in active {
            let Some(expanded) = node.available_features.get(&feature) else {
                continue;
            };
            for item in expanded {
                let normalized = normalize_feature_reference(item);
                if node.active_features.contains(&normalized) {
                    node.feature_sources
                        .entry(normalized.clone())
                        .or_default()
                        .push(FeatureSource {
                            requested_by: format!("{}/{}", node.name, feature),
                            path: vec![node.name.clone(), feature.clone(), normalized],
                        });
                }
            }
        }
    }
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

    #[test]
    fn normalizes_dep_and_namespaced_features() {
        assert_eq!(normalize_feature_reference("dep:serde_json"), "serde_json");
        assert_eq!(normalize_feature_reference("serde/derive"), "derive");
    }
}
