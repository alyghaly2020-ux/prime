use lru::LruCache;
use parking_lot::RwLock;
use std::num::NonZeroUsize;

#[derive(Debug)]
pub struct EmbeddingCache {
    cache: RwLock<LruCache<String, Vec<f32>>>,
    hits: std::sync::atomic::AtomicU64,
    misses: std::sync::atomic::AtomicU64,
}

impl Default for EmbeddingCache {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(10_000).expect("10_000 is non-zero"))),
            hits: std::sync::atomic::AtomicU64::new(0),
            misses: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn get(&self, key: &str) -> Option<Vec<f32>> {
        if let Some(embedding) = self.cache.write().get(key) {
            self.hits.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Some(embedding.clone())
        } else {
            self.misses
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            None
        }
    }

    pub fn set(&self, key: String, embedding: Vec<f32>) {
        self.cache.write().put(key, embedding);
    }

    pub fn stats(&self) -> (u64, u64) {
        (
            self.hits.load(std::sync::atomic::Ordering::Relaxed),
            self.misses.load(std::sync::atomic::Ordering::Relaxed),
        )
    }

    pub fn hit_rate(&self) -> f64 {
        let hits = self.hits.load(std::sync::atomic::Ordering::Relaxed);
        let misses = self.misses.load(std::sync::atomic::Ordering::Relaxed);
        let total = hits + misses;
        if total == 0 {
            0.0
        } else {
            hits as f64 / total as f64
        }
    }

    pub fn clear(&self) {
        self.cache.write().clear();
        self.hits.store(0, std::sync::atomic::Ordering::Relaxed);
        self.misses.store(0, std::sync::atomic::Ordering::Relaxed);
    }
}
