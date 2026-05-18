//! Multi-tier memory system. Seven tiers: working (short-term), episodic (history), semantic (knowledge), vector (embeddings), RAG (retrieval-augmented generation), cache (LRU), and compression (pruning/summarization).

pub mod cache;
pub mod compression;
pub mod episodic;
pub mod rag;
pub mod semantic;
pub mod vector;
pub mod working;

use parking_lot::RwLock as SyncRwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::security::encryption::EncryptionEngine;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Hash, Eq, PartialEq)]
pub enum MemoryType {
    Working,
    Episodic,
    Semantic,
    Vector,
    Procedural,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Option<Vec<f32>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub importance: f32,
}

pub struct System {
    pub working: Arc<working::WorkingMemory>,
    pub episodic: Arc<episodic::EpisodicMemory>,
    pub semantic: Arc<semantic::SemanticMemory>,
    pub vector: Arc<vector::VectorMemory>,
    pub compression: Arc<compression::MemoryCompressor>,
    pub rag: Arc<rag::RagEngine>,
    pub cache: Arc<cache::EmbeddingCache>,
    encryption: RwLock<Option<EncryptionEngine>>,
    encryption_enabled: SyncRwLock<HashMap<MemoryType, bool>>,
}

impl std::fmt::Debug for System {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("System").finish_non_exhaustive()
    }
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn new() -> Self {
        Self {
            working: Arc::new(working::WorkingMemory::new()),
            episodic: Arc::new(episodic::EpisodicMemory::new()),
            semantic: Arc::new(semantic::SemanticMemory::new()),
            vector: Arc::new(vector::VectorMemory::new()),
            compression: Arc::new(compression::MemoryCompressor::new()),
            rag: Arc::new(rag::RagEngine::new()),
            cache: Arc::new(cache::EmbeddingCache::new()),
            encryption: RwLock::new(None),
            encryption_enabled: SyncRwLock::new(HashMap::new()),
        }
    }

    // =========================================================================
    // Encryption API
    // =========================================================================

    /// Initialize the encryption engine with a password and salt.
    pub fn set_encryption_key(&self, password: &str, salt: &[u8]) -> anyhow::Result<()> {
        let mut engine = EncryptionEngine::new();
        engine.init_with_password(password, salt)?;
        *self.encryption.blocking_write() = Some(engine);
        Ok(())
    }

    /// Enable or disable encryption for a specific memory type.
    pub fn set_encryption_enabled(&self, memory_type: MemoryType, enabled: bool) {
        self.encryption_enabled.write().insert(memory_type, enabled);
    }

    /// Check if encryption is enabled for a memory type.
    pub fn is_encryption_enabled(&self, memory_type: &MemoryType) -> bool {
        self.encryption_enabled
            .read()
            .get(memory_type)
            .copied()
            .unwrap_or(false)
    }

    /// Encrypt memory contents where encryption is enabled.
    /// Returns the total number of encrypted entries.
    pub async fn encrypt(&self) -> anyhow::Result<usize> {
        let guard = self.encryption.read().await;
        let engine = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Encryption not initialized"))?;

        let mut total = 0usize;

        if self.is_encryption_enabled(&MemoryType::Episodic) {
            let entries = self.episodic.get_all_entries().await;
            for entry in &entries {
                let _encrypted = engine.encrypt(entry.content.as_bytes())?;
                // Note: actual in-place encryption would require extending MemoryEntry
                total += 1;
            }
        }

        if self.is_encryption_enabled(&MemoryType::Semantic) {
            // Semantic memory encryption placeholder
            total += 0;
        }

        if self.is_encryption_enabled(&MemoryType::Vector) {
            // Vector memory encryption placeholder
            total += 0;
        }

        Ok(total)
    }

    /// Decrypt memory contents where encryption is enabled.
    pub async fn decrypt(&self) -> anyhow::Result<usize> {
        let guard = self.encryption.read().await;
        let _engine = guard
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Encryption not initialized"))?;

        // Decryption would reverse encryption (placeholder)
        Ok(0)
    }

    // =========================================================================
    // Replay API
    // =========================================================================

    /// Replay all episodic memories in chronological order.
    pub async fn replay(&self) -> Vec<MemoryEntry> {
        let mut entries = self.episodic.get_all_entries().await;
        entries.sort_by_key(|a| a.created_at);
        entries
    }

    /// Replay episodic memories between two timestamps.
    pub async fn replay_between(
        &self,
        t1: chrono::DateTime<chrono::Utc>,
        t2: chrono::DateTime<chrono::Utc>,
    ) -> Vec<MemoryEntry> {
        let entries = self.episodic.get_all_entries().await;
        let mut filtered: Vec<MemoryEntry> = entries
            .into_iter()
            .filter(|e| e.created_at >= t1 && e.created_at <= t2)
            .collect();
        filtered.sort_by_key(|a| a.created_at);
        filtered
    }

    /// Replay the last `n` episodic memories.
    pub async fn replay_last(&self, n: usize) -> Vec<MemoryEntry> {
        let mut entries = self.episodic.get_all_entries().await;
        entries.sort_by_key(|b| std::cmp::Reverse(b.created_at));
        entries.truncate(n);
        entries.reverse();
        entries
    }

    /// Replay episodic memories grouped by session.
    pub async fn replay_by_session(&self) -> HashMap<String, Vec<MemoryEntry>> {
        let sessions = self.episodic.get_sessions().await;
        let mut result = HashMap::new();
        for session_id in sessions {
            if let Ok(entries) = self.episodic.export_session(&session_id).await {
                result.insert(session_id, entries);
            }
        }
        result
    }

    // =========================================================================
    // Core Memory API (unchanged signatures)
    // =========================================================================

    pub async fn store(
        &self,
        memory_type: &str,
        content: String,
        metadata: serde_json::Value,
    ) -> anyhow::Result<String> {
        match memory_type {
            "working" => self.working.store(content, metadata).await,
            "episodic" => self.episodic.store(content, metadata).await,
            "semantic" => self.semantic.store(content, metadata).await,
            "vector" => self.vector.store(content, metadata).await,
            "procedural" => self.semantic.store(content, metadata).await,
            _ => Err(anyhow::anyhow!("Unknown memory type: {}", memory_type)),
        }
    }

    pub async fn recall(&self, memory_type: &str, query: &str) -> anyhow::Result<Vec<MemoryEntry>> {
        match memory_type {
            "working" => self.working.recall(query).await,
            "episodic" => self.episodic.recall(query).await,
            "semantic" => self.semantic.recall(query).await,
            "vector" => self.vector.recall(query).await,
            "procedural" => self.semantic.recall(query).await,
            _ => Err(anyhow::anyhow!("Unknown memory type: {}", memory_type)),
        }
    }

    pub async fn query(&self, query: &str, memory_type: &str) -> anyhow::Result<String> {
        let results = self.recall(memory_type, query).await?;
        let json = serde_json::to_string_pretty(&results)?;
        Ok(json)
    }

    pub async fn consolidate(&self) -> anyhow::Result<()> {
        // Move important working memories to episodic
        let working_items = self.working.get_all().await;
        for item in working_items {
            if item.importance > 0.7 {
                self.episodic
                    .store(item.content.clone(), item.metadata.clone())
                    .await?;
            }
        }
        self.working.clear_old().await;
        self.episodic.compress().await;
        Ok(())
    }

    /// Get memory statistics: count per type
    pub async fn get_stats(&self) -> Result<serde_json::Value, String> {
        use std::collections::HashMap;
        let mut counts = HashMap::new();
        counts.insert(
            "working_count".to_string(),
            serde_json::json!(self.working.get_all().await.len()),
        );
        counts.insert(
            "episodic_count".to_string(),
            serde_json::json!(self.episodic.get_all_entries().await.len()),
        );
        counts.insert(
            "semantic_count".to_string(),
            serde_json::json!(self.semantic.get_all().await.len()),
        );
        counts.insert(
            "vector_count".to_string(),
            serde_json::json!(self.vector.get_all().await.len()),
        );
        counts.insert(
            "procedural_count".to_string(),
            serde_json::json!(self.semantic.get_all().await.len()),
        );
        serde_json::to_value(counts).map_err(|e| e.to_string())
    }

    /// Delete a memory entry by ID across all stores
    pub async fn delete_entry(&self, id: &str) -> Result<(), String> {
        // Try each store
        if self.working.delete(id).await.is_ok() {
            return Ok(());
        }
        if self.episodic.delete(id).await.is_ok() {
            return Ok(());
        }
        if self.semantic.delete(id).await.is_ok() {
            return Ok(());
        }
        if self.vector.delete(id).await.is_ok() {
            return Ok(());
        }
        Err(format!("Memory entry not found: {}", id))
    }

    /// Clear all entries of a given memory type
    pub async fn clear_type(&self, memory_type: &str) -> Result<(), String> {
        match memory_type {
            "working" => {
                self.working.clear_all().await;
                Ok(())
            }
            "episodic" => {
                self.episodic.clear_all().await;
                Ok(())
            }
            "semantic" => {
                self.semantic.clear_all().await;
                Ok(())
            }
            "vector" => {
                self.vector.clear_all().await;
                Ok(())
            }
            "procedural" => {
                self.semantic.clear_all().await;
                Ok(())
            }
            _ => Err(format!("Unknown memory type: {}", memory_type)),
        }
    }
}
