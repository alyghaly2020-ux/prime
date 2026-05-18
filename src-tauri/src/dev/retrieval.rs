use parking_lot::RwLock;
use std::sync::Arc;

/// Semantic retrieval engine that uses vector memory and code intelligence
/// to search code semantically, by symbol, by reference, and by type.
#[derive(Debug)]
pub struct SemanticRetrieval {
    vector_memory: RwLock<Option<Arc<crate::memory::vector::VectorMemory>>>,
    symbol_index: RwLock<Option<Arc<crate::code_intel::symbols::SymbolIndex>>>,
    dep_graph: RwLock<Option<Arc<crate::code_intel::deps::DepGraph>>>,
}

impl Default for SemanticRetrieval {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticRetrieval {
    pub fn new() -> Self {
        Self {
            vector_memory: RwLock::new(None),
            symbol_index: RwLock::new(None),
            dep_graph: RwLock::new(None),
        }
    }

    /// Initialize with the required subsystems. Called once during startup.
    pub fn init(
        &self,
        vector_memory: Arc<crate::memory::vector::VectorMemory>,
        symbol_index: Arc<crate::code_intel::symbols::SymbolIndex>,
        dep_graph: Arc<crate::code_intel::deps::DepGraph>,
    ) {
        *self.vector_memory.write() = Some(vector_memory);
        *self.symbol_index.write() = Some(symbol_index);
        *self.dep_graph.write() = Some(dep_graph);
    }

    /// Search for code semantically similar to the query using vector memory.
    pub async fn search_similar(&self, query: &str, _path: &str) -> Vec<String> {
        let vm_arc = {
            let guard = self.vector_memory.read();
            match guard.as_ref() {
                Some(v) => v.clone(),
                None => return vec![],
            }
        };

        // Use vector memory's recall to find similar content
        let results = vm_arc.recall(query).await.unwrap_or_default();
        results.into_iter().map(|e| e.content).collect()
    }

    /// Search for a symbol by name using the symbol index.
    pub async fn search_symbol(&self, name: &str, _path: &str) -> Option<String> {
        let si_arc = {
            let guard = self.symbol_index.read();
            match guard.as_ref() {
                Some(s) => s.clone(),
                None => return Some(name.to_string()),
            }
        };

        let matches = si_arc.lookup(name).await;
        matches.first().map(|(_file, sym)| {
            format!(
                "{} | {}:{} | {}",
                sym.name,
                sym.file,
                sym.line,
                sym.signature.as_deref().unwrap_or("")
            )
        })
    }

    /// Search for all references to a symbol using the dependency graph.
    pub async fn search_references(&self, symbol: &str, _path: &str) -> Vec<(String, usize)> {
        let dg_arc = {
            let guard = self.dep_graph.read();
            match guard.as_ref() {
                Some(d) => d.clone(),
                None => return vec![],
            }
        };

        // Search both directions: dependents (files that use this symbol)
        // and direct lookups in the symbol index
        let dependents = dg_arc.get_dependents(symbol).await;

        // Map to (file, line) — use the symbol name to find impacted files
        let mut results: Vec<(String, usize)> = dependents
            .into_iter()
            .map(|(file, _kind)| (file, 1))
            .collect();

        // Also get dependencies
        let deps = dg_arc.get_dependencies(symbol).await;
        for (file, _kind) in deps {
            if !results.iter().any(|(f, _)| f == &file) {
                results.push((file, 1));
            }
        }

        results
    }

    /// Search for symbols of a specific type (function, struct, etc.).
    pub async fn search_by_type(
        &self,
        type_name: &str,
        path: &str,
    ) -> Vec<(String, String, usize)> {
        let si_arc = {
            let guard = self.symbol_index.read();
            match guard.as_ref() {
                Some(s) => s.clone(),
                None => return vec![],
            }
        };

        let type_lower = type_name.to_lowercase();

        // Get all symbols for the workspace
        let workspace_syms = si_arc.get_for_workspace(path).await.unwrap_or_default();

        // Filter by type kind
        let filtered: Vec<(String, String, usize)> = workspace_syms
            .into_iter()
            .filter(|sym| {
                let kind_lower = sym.kind.to_lowercase();
                kind_lower.contains(&type_lower)
                    || matches!(
                        (type_lower.as_str(), kind_lower.as_str()),
                        ("function", "function_item")
                            | ("function", "function_definition")
                            | ("struct", "struct_item")
                            | ("class", "class_definition")
                            | ("trait", "trait_item")
                            | ("enum", "enum_item")
                            | ("method", "method_definition")
                            | ("method", "method_signature")
                            | ("interface", "interface_declaration")
                            | ("type", "type_alias")
                            | ("const", "const_item")
                            | ("macro", "macro_definition")
                            | ("module", "module")
                    )
            })
            .map(|sym| (sym.name, sym.file, sym.line))
            .collect();

        filtered
    }
}
