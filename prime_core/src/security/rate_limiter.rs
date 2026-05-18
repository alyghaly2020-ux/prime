//! Rate Limiting System
//!
//! Uses a token bucket algorithm to enforce per-subject and global rate limits.
//! Configurable limits per action type.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u64,
    pub window_secs: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RateLimiterStats {
    pub total_requests: u64,
    pub total_denied: u64,
    pub active_buckets: usize,
}

struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(max_requests: u64, window_secs: u64) -> Self {
        let max_tokens = max_requests as f64;
        let refill_rate = if window_secs > 0 {
            max_tokens / window_secs as f64
        } else {
            max_tokens
        };
        Self {
            tokens: max_tokens,
            max_tokens,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        if elapsed > 0.0 {
            self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
            self.last_refill = now;
        }
    }

    fn try_consume(&mut self, count: f64) -> bool {
        self.refill();
        if self.tokens >= count {
            self.tokens -= count;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub enum RateLimitScope {
    Subject(String),
    Action(String),
    Global,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for {scope}: {limit} requests per {window_secs}s")]
    RateLimitExceeded {
        scope: String,
        limit: u64,
        window_secs: u64,
    },
}

pub struct RateLimiter {
    buckets: RwLock<HashMap<String, TokenBucket>>,
    config: RwLock<HashMap<String, RateLimitConfig>>,
    defaults: RateLimitConfig,
    total_requests: AtomicU64,
    total_denied: AtomicU64,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            buckets: RwLock::new(HashMap::new()),
            config: RwLock::new(HashMap::new()),
            defaults: RateLimitConfig {
                max_requests: 100,
                window_secs: 60,
            },
            total_requests: AtomicU64::new(0),
            total_denied: AtomicU64::new(0),
        }
    }

    /// Configure a specific rate limit for a scope key.
    pub fn configure(&self, key: &str, max_requests: u64, window_secs: u64) {
        self.config.write().insert(
            key.to_string(),
            RateLimitConfig {
                max_requests,
                window_secs,
            },
        );
    }

    /// Check if a subject/action/global scope has exceeded its rate limit.
    pub fn check_rate(&self, scope: RateLimitScope) -> Result<(), RateLimitError> {
        let key = match &scope {
            RateLimitScope::Subject(s) => format!("subject:{}", s),
            RateLimitScope::Action(a) => format!("action:{}", a),
            RateLimitScope::Global => "global".to_string(),
        };

        self.total_requests.fetch_add(1, Ordering::Relaxed);

        let config = {
            let configs = self.config.read();
            configs
                .get(&key)
                .or_else(|| {
                    let key = match &scope {
                        RateLimitScope::Subject(_) => return None,
                        RateLimitScope::Action(a) => a.clone(),
                        RateLimitScope::Global => return None,
                    };
                    configs.get(&key)
                })
                .cloned()
                .unwrap_or_else(|| self.defaults.clone())
        };

        let mut buckets = self.buckets.write();
        let bucket = buckets
            .entry(key.clone())
            .or_insert_with(|| TokenBucket::new(config.max_requests, config.window_secs));

        if !bucket.try_consume(1.0) {
            self.total_denied.fetch_add(1, Ordering::Relaxed);
            return Err(RateLimitError::RateLimitExceeded {
                scope: key,
                limit: config.max_requests,
                window_secs: config.window_secs,
            });
        }

        Ok(())
    }

    /// Check rate limit by subject (e.g., a plugin name).
    pub fn check_subject(&self, subject: &str) -> Result<(), RateLimitError> {
        self.check_rate(RateLimitScope::Subject(subject.to_string()))
    }

    /// Check rate limit by action type.
    pub fn check_action(&self, action: &str) -> Result<(), RateLimitError> {
        self.check_rate(RateLimitScope::Action(action.to_string()))
    }

    /// Check global rate limit.
    pub fn check_global(&self) -> Result<(), RateLimitError> {
        self.check_rate(RateLimitScope::Global)
    }

    /// Get rate limiter statistics.
    pub fn stats(&self) -> RateLimiterStats {
        RateLimiterStats {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_denied: self.total_denied.load(Ordering::Relaxed),
            active_buckets: self.buckets.read().len(),
        }
    }

    /// Reset all rate limit buckets.
    pub fn reset(&self) {
        self.buckets.write().clear();
        self.total_requests.store(0, Ordering::Relaxed);
        self.total_denied.store(0, Ordering::Relaxed);
    }
}
