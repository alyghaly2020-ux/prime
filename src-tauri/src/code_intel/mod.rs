//! Code intelligence subsystem. Multi-language parsing via tree-sitter, symbol indexing, dependency graph extraction, and full-text search with context-aware results.

pub mod deps;
pub mod parser;
pub mod search;
pub mod symbols;

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use walkdir::WalkDir;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SearchResult {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub content: String,
    pub score: f64,
    pub symbol_type: Option<String>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolDef {
    pub name: String,
    pub kind: String,
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub signature: Option<String>,
    pub doc: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DepEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

// ---- Graph visualization types for D3.js ----

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub file: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub kind: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphData {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

pub struct Engine {
    pub parser: Arc<parser::TreeSitterParser>,
    pub search: Arc<search::SearchEngine>,
    pub symbols: Arc<symbols::SymbolIndex>,
    pub deps: Arc<deps::DepGraph>,
    workspace_cache: RwLock<HashMap<String, Vec<SymbolDef>>>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            parser: Arc::new(parser::TreeSitterParser::new()),
            search: Arc::new(search::SearchEngine::new()),
            symbols: Arc::new(symbols::SymbolIndex::new()),
            deps: Arc::new(deps::DepGraph::new()),
            workspace_cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn search(&self, query: &str, path: &str) -> anyhow::Result<Vec<SearchResult>> {
        self.search.search(query, path).await
    }

    pub async fn parse_file(&self, path: &str) -> anyhow::Result<Vec<SymbolDef>> {
        self.parser.parse(path).await
    }

    pub async fn index_workspace(&self, path: &str) -> anyhow::Result<()> {
        tracing::info!("Indexing workspace: {}", path);
        let mut symbols = Vec::new();

        // Walk filesystem in blocking context to avoid blocking the async runtime
        let walk_path = path.to_string();
        let files: Vec<String> = tokio::task::spawn_blocking(move || {
            WalkDir::new(&walk_path)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.')
                        && name.as_ref() != "node_modules"
                        && name.as_ref() != "target"
                })
                .filter(|e| e.file_type().is_file())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| {
                            parser::SUPPORTED_EXTENSIONS.contains(&ext.to_string_lossy().as_ref())
                        })
                        .unwrap_or(false)
                })
                .map(|e| e.path().to_string_lossy().to_string())
                .collect()
        })
        .await?;

        for file_path in &files {
            if let Ok(file_syms) = self.parser.parse(file_path).await {
                self.symbols.add_file(file_path, &file_syms).await;
                symbols.extend(file_syms);
            }
        }

        self.workspace_cache
            .write()
            .await
            .insert(path.to_string(), symbols);

        tracing::info!("Workspace indexed: {}", path);
        Ok(())
    }

    pub async fn get_symbols(&self, path: &str) -> Vec<SymbolDef> {
        self.symbols
            .get_for_workspace(path)
            .await
            .clone()
            .unwrap_or_default()
    }

    // =========================================================================
    // Graph Visualization API
    // =========================================================================

    /// Return nodes and edges for D3.js visualization of the code graph.
    pub async fn get_graph_data(&self) -> GraphData {
        let edges = self.deps.all_edges().await;
        let mut node_set: HashMap<String, (String, String)> = HashMap::new(); // id -> (label, kind)

        // Collect nodes from edges
        for edge in &edges {
            node_set.entry(edge.source.clone()).or_insert_with(|| {
                let label = edge
                    .source
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&edge.source)
                    .to_string();
                (label, "file".to_string())
            });
            node_set.entry(edge.target.clone()).or_insert_with(|| {
                let label = edge
                    .target
                    .rsplit(['/', '\\'])
                    .next()
                    .unwrap_or(&edge.target)
                    .to_string();
                (label, "file".to_string())
            });
        }

        // Also collect nodes from symbol index
        let symbols = self.symbols.search_by_prefix("").await;
        for (_file, sym) in &symbols {
            node_set
                .entry(sym.name.clone())
                .or_insert_with(|| (sym.name.clone(), sym.kind.clone()));
        }

        let nodes: Vec<GraphNode> = node_set
            .into_iter()
            .map(|(id, (label, kind))| GraphNode {
                id,
                label,
                kind,
                file: String::new(),
            })
            .collect();

        let graph_edges: Vec<GraphEdge> = edges
            .into_iter()
            .map(|e| GraphEdge {
                source: e.source,
                target: e.target,
                kind: e.kind,
            })
            .collect();

        GraphData {
            nodes,
            edges: graph_edges,
        }
    }

    /// Get the full dependency chain for a given file (what this file depends on).
    pub async fn get_dependency_chain(&self, file: &str) -> Vec<String> {
        let mut chain = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![file.to_string()];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            chain.push(current.clone());
            let deps = self.deps.get_dependencies(&current).await;
            for (dep, _kind) in deps {
                stack.push(dep);
            }
        }

        chain
    }

    /// Get the impact analysis for a file: what other files are affected
    /// if this file changes (reverse dependency traversal).
    pub async fn get_impact_analysis(&self, file: &str) -> Vec<String> {
        let mut affected = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = vec![file.to_string()];

        while let Some(current) = queue.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if current != file {
                affected.push(current.clone());
            }
            let dependents = self.deps.get_dependents(&current).await;
            for (dep, _kind) in dependents {
                queue.push(dep);
            }
        }

        affected
    }
}
