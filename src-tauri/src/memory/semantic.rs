use std::collections::HashMap;
use tokio::sync::RwLock;

use super::MemoryEntry;

#[derive(Debug)]
pub struct SemanticMemory {
    facts: RwLock<HashMap<String, MemoryEntry>>,
    relations: RwLock<Vec<(String, String, String)>>, // (subject, predicate, object)
}

impl Default for SemanticMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticMemory {
    pub fn new() -> Self {
        Self {
            facts: RwLock::new(HashMap::new()),
            relations: RwLock::new(Vec::new()),
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
            memory_type: super::MemoryType::Semantic,
            content: content.clone(),
            metadata,
            embedding: None,
            created_at: chrono::Utc::now(),
            access_count: 0,
            importance: 0.8,
        };

        self.facts.write().await.insert(id.clone(), entry);

        // Extract simple relations (subject-predicate-object)
        if let Some((subj, rest)) = content.split_once(" is ") {
            self.relations.write().await.push((
                subj.trim().to_string(),
                "is".to_string(),
                rest.trim().to_string(),
            ));
        }

        Ok(id)
    }

    pub async fn recall(&self, query: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        let facts = self.facts.read().await;
        let query_lower = query.to_lowercase();
        let mut results: Vec<MemoryEntry> = facts
            .values()
            .filter(|e| e.content.to_lowercase().contains(&query_lower))
            .cloned()
            .collect();

        // Also search by relation
        let relations = self.relations.read().await;
        for (subj, pred, obj) in relations.iter() {
            if subj.to_lowercase().contains(&query_lower)
                || obj.to_lowercase().contains(&query_lower)
            {
                let derived = format!("{} {} {}", subj, pred, obj);
                if let Some(entry) = facts.values().find(|e| e.content == derived) {
                    if !results.iter().any(|r| r.id == entry.id) {
                        results.push(entry.clone());
                    }
                }
            }
        }

        results.sort_by(|a, b| {
            b.importance
                .partial_cmp(&a.importance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(20);
        Ok(results)
    }

    pub async fn get_relations(&self, subject: &str) -> Vec<(String, String)> {
        self.relations
            .read()
            .await
            .iter()
            .filter(|(s, _, _)| s == subject)
            .map(|(_, p, o)| (p.clone(), o.clone()))
            .collect()
    }

    /// Return all stored facts.
    pub async fn get_all(&self) -> Vec<MemoryEntry> {
        self.facts.read().await.values().cloned().collect()
    }

    /// Delete a fact by ID.
    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let mut facts = self.facts.write().await;
        if facts.remove(id).is_some() {
            // Also remove related relations
            let mut relations = self.relations.write().await;
            relations.retain(|(subj, _pred, _obj)| {
                // Keep relations whose subject still exists as a fact
                facts.values().any(|e| e.content.starts_with(subj))
            });
            Ok(())
        } else {
            Err(format!("Entry not found in semantic memory: {}", id))
        }
    }

    /// Clear all facts and relations.
    pub async fn clear_all(&self) {
        self.facts.write().await.clear();
        self.relations.write().await.clear();
    }
}
