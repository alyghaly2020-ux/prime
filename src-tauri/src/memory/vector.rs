use std::collections::HashMap;
use tokio::sync::RwLock;

use super::MemoryEntry;

#[derive(Debug)]
pub struct VectorMemory {
    entries: RwLock<Vec<MemoryEntry>>,
    dimension: usize,
}

impl Default for VectorMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl VectorMemory {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            dimension: 384, // all-MiniLM-L6-v2 dimension
        }
    }

    pub async fn store(
        &self,
        content: String,
        metadata: serde_json::Value,
    ) -> anyhow::Result<String> {
        let id = uuid::Uuid::new_v4().to_string();
        let embedding = compute_text_embedding(&content, self.dimension);

        let entry = MemoryEntry {
            id: id.clone(),
            memory_type: super::MemoryType::Vector,
            content,
            metadata,
            embedding: Some(embedding),
            created_at: chrono::Utc::now(),
            access_count: 0,
            importance: 0.5,
        };

        self.entries.write().await.push(entry);
        Ok(id)
    }

    pub async fn store_batch(
        &self,
        items: Vec<(String, serde_json::Value)>,
    ) -> anyhow::Result<Vec<String>> {
        let mut ids = Vec::with_capacity(items.len());
        let mut entries = self.entries.write().await;

        for (content, metadata) in items {
            let id = uuid::Uuid::new_v4().to_string();
            let embedding = compute_text_embedding(&content, self.dimension);
            entries.push(MemoryEntry {
                id: id.clone(),
                memory_type: super::MemoryType::Vector,
                content,
                metadata,
                embedding: Some(embedding),
                created_at: chrono::Utc::now(),
                access_count: 0,
                importance: 0.5,
            });
            ids.push(id);
        }

        Ok(ids)
    }

    pub async fn recall(&self, query: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        let query_embed = compute_text_embedding(query, self.dimension);
        let entries = self.entries.read().await;

        let mut scored: Vec<(f32, MemoryEntry)> = Vec::new();
        for e in entries.iter() {
            if let Some(emb) = e.embedding.as_ref() {
                let sim = cosine_similarity(&query_embed, emb);
                scored.push((sim, e.clone()));
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(10);

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    pub async fn recall_batch(&self, queries: &[&str]) -> anyhow::Result<Vec<Vec<MemoryEntry>>> {
        let entries = self.entries.read().await;
        let mut results = Vec::with_capacity(queries.len());

        for query in queries {
            let query_embed = compute_text_embedding(query, self.dimension);
            let mut scored: Vec<(f32, MemoryEntry)> = Vec::new();
            for e in entries.iter() {
                if let Some(emb) = e.embedding.as_ref() {
                    let sim = cosine_similarity(&query_embed, emb);
                    scored.push((sim, e.clone()));
                }
            }
            scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
            scored.truncate(10);
            results.push(scored.into_iter().map(|(_, e)| e).collect());
        }

        Ok(results)
    }

    pub async fn recall_filtered<F>(
        &self,
        query: &str,
        filter: F,
    ) -> anyhow::Result<Vec<MemoryEntry>>
    where
        F: Fn(&MemoryEntry) -> bool,
    {
        let query_embed = compute_text_embedding(query, self.dimension);
        let entries = self.entries.read().await;

        let mut scored: Vec<(f32, MemoryEntry)> = Vec::new();
        for e in entries.iter() {
            if !filter(e) {
                continue;
            }
            if let Some(emb) = e.embedding.as_ref() {
                let sim = cosine_similarity(&query_embed, emb);
                scored.push((sim, e.clone()));
            }
        }

        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(10);

        Ok(scored.into_iter().map(|(_, e)| e).collect())
    }

    pub async fn get_all(&self) -> Vec<MemoryEntry> {
        self.entries.read().await.clone()
    }

    pub async fn delete(&self, id: &str) -> Result<(), String> {
        let mut entries = self.entries.write().await;
        let len_before = entries.len();
        entries.retain(|e| e.id != id);
        if entries.len() < len_before {
            Ok(())
        } else {
            Err(format!("Entry not found in vector memory: {}", id))
        }
    }

    pub async fn clear_all(&self) {
        self.entries.write().await.clear();
    }

    pub async fn remove(&self, id: &str) -> anyhow::Result<bool> {
        let mut entries = self.entries.write().await;
        let len_before = entries.len();
        entries.retain(|e| e.id != id);
        Ok(entries.len() < len_before)
    }

    pub async fn count(&self) -> usize {
        self.entries.read().await.len()
    }

    pub async fn stats(&self) -> HashMap<String, usize> {
        let entries = self.entries.read().await;
        let mut map = HashMap::new();
        map.insert("dimension".to_string(), self.dimension);
        map.insert("total_vectors".to_string(), entries.len());
        let memory_estimate: usize = entries
            .iter()
            .map(|e| {
                let emb_size = e.embedding.as_ref().map(|v| v.len() * 4).unwrap_or(0);
                let content_size = e.content.len();
                emb_size + content_size + 256 // overhead per entry
            })
            .sum();
        map.insert("memory_estimate_bytes".to_string(), memory_estimate);
        map
    }

    pub fn dimension(&self) -> usize {
        self.dimension
    }
}

/// Compute a bag-of-words TF-IDF-like embedding for the given text.
pub fn compute_text_embedding(text: &str, dimension: usize) -> Vec<f32> {
    let mut embedding = vec![0.0f32; dimension];
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        return embedding;
    }
    let total = words.len() as f32;

    for word in &words {
        let hash = fxhash::hash64(word);
        let idx = (hash as usize) % dimension;
        embedding[idx] += 1.0 / total;
    }

    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in embedding.iter_mut() {
            *v /= norm;
        }
    }

    embedding
}

/// Compute cosine similarity between two vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a * norm_b)
    }
}
