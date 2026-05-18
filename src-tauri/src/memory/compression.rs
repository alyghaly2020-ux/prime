use std::collections::HashMap;
use tokio::sync::RwLock;

use super::vector;
use super::MemoryEntry;

#[derive(Debug)]
pub struct MemoryCompressor {
    compression_stats: RwLock<CompressionStats>,
}

#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct CompressionStats {
    pub total_entries_before: usize,
    pub total_entries_after: usize,
    pub compression_ratio: f64,
    pub last_compressed: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Clone, serde::Serialize)]
pub struct SavingsEstimate {
    pub original_size: usize,
    pub compressed_size: usize,
    pub ratio: f64,
}

impl Default for MemoryCompressor {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryCompressor {
    pub fn new() -> Self {
        Self {
            compression_stats: RwLock::new(CompressionStats::default()),
        }
    }

    pub async fn compress_entries(&self, entries: &[MemoryEntry]) -> Vec<MemoryEntry> {
        let mut stats = self.compression_stats.write().await;
        stats.total_entries_before = entries.len();

        // Group by semantic similarity (simple approach: same prefix)
        let mut groups: HashMap<String, Vec<&MemoryEntry>> = HashMap::new();
        for entry in entries {
            let key = entry.content.chars().take(50).collect::<String>();
            groups.entry(key).or_default().push(entry);
        }

        let mut compressed = Vec::new();
        for (_key, group) in groups {
            if group.len() <= 3 {
                compressed.extend(group.into_iter().cloned());
            } else {
                // Merge similar entries
                let merged_content = group
                    .iter()
                    .map(|e| e.content.as_str())
                    .collect::<Vec<_>>()
                    .join("\n---\n");

                if let Some(first) = group.first() {
                    compressed.push(MemoryEntry {
                        id: uuid::Uuid::new_v4().to_string(),
                        memory_type: super::MemoryType::Semantic,
                        content: merged_content,
                        metadata: first.metadata.clone(),
                        embedding: None,
                        created_at: first.created_at,
                        access_count: group.iter().map(|e| e.access_count).sum(),
                        importance: group.iter().map(|e| e.importance).sum::<f32>()
                            / group.len() as f32,
                    });
                }
            }
        }

        stats.total_entries_after = compressed.len();
        stats.compression_ratio = if stats.total_entries_before > 0 {
            (stats.total_entries_before - stats.total_entries_after) as f64
                / stats.total_entries_before as f64
        } else {
            0.0
        };
        stats.last_compressed = Some(chrono::Utc::now());

        compressed
    }

    /// Group semantically similar entries using cosine similarity.
    pub async fn compress_by_semantics(
        &self,
        entries: &[MemoryEntry],
        threshold: f32,
    ) -> Vec<MemoryEntry> {
        if entries.is_empty() {
            return entries.to_vec();
        }

        let dim = 384;
        let n = entries.len();
        let embeddings: Vec<Vec<f32>> = (0..n)
            .map(|i| vector::compute_text_embedding(&entries[i].content, dim))
            .collect();

        let mut merged = vec![None; n];
        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();

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
                if sim > threshold {
                    group.push(j);
                    merged[j] = Some(i);
                }
            }
            groups.insert(i, group);
        }

        let mut compressed = Vec::new();
        for (&leader_idx, group) in &groups {
            if group.len() > 1 {
                let merged_content: Vec<&str> = group
                    .iter()
                    .map(|&idx| entries[idx].content.as_str())
                    .collect();
                let first = &entries[leader_idx];
                compressed.push(MemoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    memory_type: super::MemoryType::Semantic,
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

        compressed
    }

    /// Compress by grouping entries within a time window.
    pub async fn compress_by_time(
        &self,
        entries: &[MemoryEntry],
        window_minutes: i64,
    ) -> Vec<MemoryEntry> {
        if entries.is_empty() {
            return entries.to_vec();
        }

        let mut sorted = entries.to_vec();
        sorted.sort_by_key(|a| a.created_at);

        let mut windows: Vec<Vec<MemoryEntry>> = Vec::new();
        let mut current_window: Vec<MemoryEntry> = Vec::new();
        let mut window_start = sorted[0].created_at;

        for entry in sorted {
            if (entry.created_at - window_start).num_minutes() < window_minutes {
                current_window.push(entry);
            } else {
                window_start = entry.created_at;
                current_window = vec![entry];
            }
        }
        if !current_window.is_empty() {
            windows.push(current_window);
        }

        let mut compressed = Vec::new();
        for window in windows {
            if window.len() <= 3 {
                compressed.extend(window);
            } else {
                let merged_content: Vec<&str> = window.iter().map(|e| e.content.as_str()).collect();
                let first = &window[0];
                compressed.push(MemoryEntry {
                    id: uuid::Uuid::new_v4().to_string(),
                    memory_type: super::MemoryType::Semantic,
                    content: merged_content.join("\n---\n"),
                    metadata: first.metadata.clone(),
                    embedding: None,
                    created_at: first.created_at,
                    access_count: window.iter().map(|e| e.access_count).sum(),
                    importance: window.iter().map(|e| e.importance).sum::<f32>()
                        / window.len() as f32,
                });
            }
        }

        compressed
    }

    /// Estimate potential compression savings without actually compressing.
    pub async fn estimate_savings(&self, entries: &[MemoryEntry]) -> SavingsEstimate {
        let original_size: usize = entries.iter().map(|e| e.content.len() + 256).sum();

        // Quick estimate: group by first 50 chars prefix, count duplicates
        let mut groups: HashMap<String, usize> = HashMap::new();
        for entry in entries {
            let key = entry.content.chars().take(50).collect::<String>();
            *groups.entry(key).or_default() += 1;
        }

        let unique_count = groups.len();
        let compressed_size = if !entries.is_empty() && unique_count > 0 {
            (original_size as f64 * (unique_count as f64 / entries.len() as f64)) as usize
        } else {
            original_size
        };

        SavingsEstimate {
            original_size,
            compressed_size,
            ratio: if original_size > 0 {
                (original_size - compressed_size) as f64 / original_size as f64
            } else {
                0.0
            },
        }
    }

    pub async fn stats(&self) -> CompressionStats {
        self.compression_stats.read().await.clone()
    }
}
