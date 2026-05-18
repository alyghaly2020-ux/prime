use std::collections::HashMap;
use tokio::sync::RwLock;

use crate::tools::config::{ToolCategory, ToolConfig, all_tool_configs};

pub struct ToolRegistry {
    tools: RwLock<HashMap<String, ToolConfig>>,
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistry {
    pub fn new() -> Self {
        let tools = all_tool_configs()
            .into_iter()
            .map(|t| (t.id.clone(), t))
            .collect();
        Self { tools: RwLock::new(tools) }
    }

    pub async fn get(&self, id: &str) -> Option<ToolConfig> {
        self.tools.read().await.get(id).cloned()
    }

    pub async fn list_by_category(&self, category: &ToolCategory) -> Vec<ToolConfig> {
        self.tools.read().await.values()
            .filter(|t| t.category == *category)
            .cloned().collect()
    }

    pub async fn list_all(&self) -> Vec<ToolConfig> {
        self.tools.read().await.values().cloned().collect()
    }

    pub async fn set_enabled(&self, id: &str, enabled: bool) {
        if let Some(tool) = self.tools.write().await.get_mut(id) {
            tool.enabled = enabled;
        }
    }

    pub async fn set_installed(&self, id: &str, installed: bool) {
        if let Some(tool) = self.tools.write().await.get_mut(id) {
            tool.installed = installed;
        }
    }

    pub async fn count_by_category(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for tool in self.tools.read().await.values() {
            *counts.entry(format!("{:?}", tool.category)).or_insert(0) += 1;
        }
        counts
    }

    pub async fn search(&self, query: &str) -> Vec<ToolConfig> {
        let q = query.to_lowercase();
        self.tools.read().await.values()
            .filter(|t| t.id.to_lowercase().contains(&q) || t.name.to_lowercase().contains(&q))
            .cloned().collect()
    }

    pub async fn enable_category(&self, category: &ToolCategory) {
        let mut tools = self.tools.write().await;
        for tool in tools.values_mut() {
            if tool.category == *category {
                tool.enabled = true;
            }
        }
    }

    pub async fn disable_all(&self) {
        let mut tools = self.tools.write().await;
        for tool in tools.values_mut() {
            tool.enabled = false;
        }
    }
}
