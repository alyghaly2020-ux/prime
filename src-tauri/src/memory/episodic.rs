use std::collections::HashMap;
use tokio::sync::RwLock;

use super::vector;
use super::MemoryEntry;

#[derive(Debug)]
pub struct EpisodicMemory {
    sessions: RwLock<HashMap<String, Vec<MemoryEntry>>>,
    current_session: RwLock<String>,
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl EpisodicMemory {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            current_session: RwLock::new(uuid::Uuid::new_v4().to_string()),
        }
    }

    pub async fn store(
        &self,
        content: String,
        metadata: serde_json::Value,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let session = self.current_session.read().await.clone();

        let entry = MemoryEntry {
            id: id.clone(),
            memory_type: super::MemoryType::Episodic,
            content,
            metadata,
            embedding: None,
            created_at: chrono::Utc::now(),
            access_count: 0,
            importance: 0.6,
        };

        self.sessions
            .write()
            .await
            .entry(session)
            .or_default()
            .push(entry);

        Ok(id)
    }

    pub async fn recall(&self, query: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        let sessions = self.sessions.read().await;
        let query_lower = query.to_lowercase();
        let mut results: Vec<MemoryEntry> = sessions
            .values()
            .flatten()
            .filter(|e| {
                e.content.to_lowercase().contains(&query_lower)
                    || e.metadata.to_string().to_lowercase().contains(&query_lower)
            })
            .cloned()
            .collect();

        results.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        results.truncate(50);
        Ok(results)
    }

    /// Actually compress episodic memory by merging similar entries (cosine > 0.8)
    /// within the same session.
    pub async fn compress(&self) {
        let mut sessions = self.sessions.write().await;
        let dim = 384;

        for (_session_id, entries) in sessions.iter_mut() {
            if entries.len() <= 3 {
                continue;
            }

            let n = entries.len();
            let embeddings: Vec<Vec<f32>> = (0..n)
                .map(|i| vector::compute_text_embedding(&entries[i].content, dim))
                .collect();

            let mut merged: Vec<Option<usize>> = vec![None; n];
            let mut group_map: HashMap<usize, Vec<usize>> = HashMap::new();

            for i in 0..n {
                if merged[i].is_some() {
                    continue;
                }
                let mut group = vec![i];
                for j in (i + 1)..n {
                    if merged[j].is_some() {
                        continue;
                    }
                    let sim = vector::cosine_similarity(&embeddings[i], &embeddings[j]);
                    if sim > 0.8 {
                        group.push(j);
                        merged[j] = Some(i);
                    }
                }
                group_map.insert(i, group);
            }

            let mut compressed: Vec<MemoryEntry> = Vec::new();
            for (&leader_idx, group) in &group_map {
                if group.len() > 1 {
                    let merged_content: Vec<&str> = group
                        .iter()
                        .map(|&idx| entries[idx].content.as_str())
                        .collect();
                    let first = &entries[leader_idx];
                    compressed.push(MemoryEntry {
                        id: uuid::Uuid::new_v4().to_string(),
                        memory_type: super::MemoryType::Episodic,
                        content: merged_content.join("\n---\n"),
                        metadata: first.metadata.clone(),
                        embedding: None,
                        created_at: first.created_at,
                        access_count: group.iter().map(|&idx| entries[idx].access_count).sum(),
                        importance: group
                            .iter()
                            .map(|&idx| entries[idx].importance)
                            .sum::<f32>()
                            / group.len() as f32,
                    });
                } else {
                    compressed.push(entries[leader_idx].clone());
                }
            }

            *entries = compressed;
        }
    }

    /// Persist all sessions to a JSON file.
    pub async fn persist(&self, path: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        let json = serde_json::to_string_pretty(&*sessions)?;
        tokio::fs::write(path, json).await?;
        Ok(())
    }

    /// Load sessions from a JSON file.
    pub async fn load(&self, path: &str) -> anyhow::Result<()> {
        let json = tokio::fs::read_to_string(path).await?;
        let sessions: HashMap<String, Vec<MemoryEntry>> = serde_json::from_str(&json)?;
        *self.sessions.write().await = sessions;
        Ok(())
    }

    /// Export a specific session as a vector of entries.
    pub async fn export_session(&self, session_id: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        let sessions = self.sessions.read().await;
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Session not found: {}", session_id))
    }

    pub async fn new_session(&self) {
        *self.current_session.write().await = uuid::Uuid::new_v4().to_string();
    }

    /// Return all entries across all sessions.
    pub async fn get_all_entries(&self) -> Vec<MemoryEntry> {
        let sessions = self.sessions.read().await;
        sessions.values().flatten().cloned().collect()
    }

    /// Return all session IDs.
    pub async fn get_sessions(&self) -> Vec<String> {
        self.sessions.read().await.keys().cloned().collect()
    }

    /// Delete an entry by ID across all sessions.
    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let mut sessions = self.sessions.write().await;
        for entries in sessions.values_mut() {
            let len_before = entries.len();
            entries.retain(|e| e.id != id);
            if entries.len() < len_before {
                return Ok(());
            }
        }
        Err(format!("Entry not found in episodic memory: {}", id))
    }

    /// Clear all sessions and entries.
    pub async fn clear_all(&self) {
        self.sessions.write().await.clear();
    }
}
