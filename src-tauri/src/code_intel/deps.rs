use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

use super::DepEdge;

#[derive(Debug)]
pub struct DepGraph {
    edges: RwLock<Vec<DepEdge>>,
    nodes: RwLock<HashSet<String>>,
    adjacency: RwLock<HashMap<String, Vec<(String, String)>>>,
}

impl Default for DepGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl DepGraph {
    pub fn new() -> Self {
        Self {
            edges: RwLock::new(Vec::new()),
            nodes: RwLock::new(HashSet::new()),
            adjacency: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_edge(&self, source: String, target: String, kind: String) {
        self.nodes.write().await.insert(source.clone());
        self.nodes.write().await.insert(target.clone());

        self.edges.write().await.push(DepEdge {
            source: source.clone(),
            target: target.clone(),
            kind: kind.clone(),
        });

        self.adjacency
            .write()
            .await
            .entry(source)
            .or_default()
            .push((target, kind));
    }

    pub async fn get_dependencies(&self, node: &str) -> Vec<(String, String)> {
        self.adjacency
            .read()
            .await
            .get(node)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn get_dependents(&self, node: &str) -> Vec<(String, String)> {
        self.adjacency
            .read()
            .await
            .iter()
            .filter(|(_, deps)| deps.iter().any(|(t, _)| t == node))
            .map(|(s, _)| (s.clone(), "depends_on".to_string()))
            .collect()
    }

    pub async fn all_edges(&self) -> Vec<DepEdge> {
        self.edges.read().await.clone()
    }

    pub async fn topological_sort(&self) -> Vec<String> {
        let adj = self.adjacency.read().await;
        let nodes = self.nodes.read().await;

        let mut in_degree: HashMap<String, usize> = nodes.iter().map(|n| (n.clone(), 0)).collect();

        for deps in adj.values() {
            for (target, _) in deps {
                *in_degree.entry(target.clone()).or_insert(0) += 1;
            }
        }

        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(n, _)| n.clone())
            .collect();

        let mut sorted = Vec::new();
        while let Some(node) = queue.pop() {
            sorted.push(node.clone());
            if let Some(deps) = adj.get(&node) {
                for (target, _) in deps {
                    if let Some(deg) = in_degree.get_mut(target) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push(target.clone());
                        }
                    }
                }
            }
        }

        sorted
    }
}
