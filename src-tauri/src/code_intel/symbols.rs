use std::collections::HashMap;
use tokio::sync::RwLock;

use super::SymbolDef;

#[derive(Debug)]
pub struct SymbolIndex {
    by_workspace: RwLock<HashMap<String, Vec<SymbolDef>>>,
    by_name: RwLock<HashMap<String, Vec<(String, SymbolDef)>>>,
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            by_workspace: RwLock::new(HashMap::new()),
            by_name: RwLock::new(HashMap::new()),
        }
    }

    pub async fn add_file(&self, path: &str, symbols: &[SymbolDef]) {
        let mut workspace_map = self.by_workspace.write().await;
        let mut name_map = self.by_name.write().await;

        for sym in symbols {
            let mut full_sym = sym.clone();
            full_sym.file = path.to_string();

            workspace_map
                .entry(path.to_string())
                .or_default()
                .push(full_sym.clone());

            name_map
                .entry(sym.name.clone())
                .or_default()
                .push((path.to_string(), full_sym));
        }
    }

    pub async fn get_for_workspace(&self, path: &str) -> Option<Vec<SymbolDef>> {
        let map = self.by_workspace.read().await;
        map.get(path).cloned()
    }

    pub async fn lookup(&self, name: &str) -> Vec<(String, SymbolDef)> {
        let map = self.by_name.read().await;
        map.get(name).cloned().unwrap_or_default()
    }

    pub async fn search_by_prefix(&self, prefix: &str) -> Vec<(String, SymbolDef)> {
        let map = self.by_name.read().await;
        let prefix_lower = prefix.to_lowercase();
        map.iter()
            .filter(|(name, _)| name.to_lowercase().starts_with(&prefix_lower))
            .flat_map(|(_, entries)| entries.clone())
            .collect()
    }
}
