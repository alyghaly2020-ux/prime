use std::collections::VecDeque;
use tokio::sync::RwLock;

use super::MemoryEntry;

#[derive(Debug)]
pub struct WorkingMemory {
    entries: RwLock<VecDeque<MemoryEntry>>,
    capacity: usize,
}

impl Default for WorkingMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkingMemory {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(1000)),
            capacity: 1000,
        }
    }

    pub async fn store(
        &self,
        content: String,
        metadata: serde_json::Value,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let entry = MemoryEntry {
            id: id.clone(),
            memory_type: super::MemoryType::Working,
            content,
            metadata,
            embedding: None,
            created_at: chrono::Utc::now(),
            access_count: 0,
            importance: 0.5,
        };

        let mut entries = self.entries.write().await;
        if entries.len() >= self.capacity {
            entries.pop_front();
        }
        entries.push_back(entry);
        Ok(id)
    }

    pub async fn recall(&self, query: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        let query_lower = query.to_lowercase();
        let mut results: Vec<MemoryEntry> = entries
            .iter()
            .filter(|e| e.content.to_lowercase().contains(&query_lower)).cloned()
            .collect();
        results.truncate(20);
        Ok(results)
    }

    pub async fn get_all(&self) -> Vec<MemoryEntry> {
        self.entries.read().await.iter().cloned().collect()
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let mut entries = self.entries.write().await;
        let len_before = entries.len();
        entries.retain(|e| e.id != id);
        if entries.len() < len_before {
            Ok(())
        } else {
            Err(format!("Entry not found in working memory: {}", id))
        }
    }

    pub async fn clear_all(&self) {
        self.entries.write().await.clear();
    }

    pub async fn clear_old(&self) {
        let mut entries = self.entries.write().await;
        entries.retain(|e| (chrono::Utc::now() - e.created_at).num_hours() < 24);
    }
}
