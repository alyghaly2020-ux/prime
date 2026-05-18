//! Debug Console
//!
//! In-memory debug log ring buffer accessible at runtime.
//! Supports structured querying of recent activity, streaming subscriptions,
//! full-text search, and statistics.

use parking_lot::RwLock;
use std::collections::VecDeque;
use tokio::sync::broadcast;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DebugLogEntry {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub level: DebugLevel,
    pub module: String,
    pub message: String,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DebugLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for DebugLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DebugLevel::Trace => write!(f, "TRACE"),
            DebugLevel::Debug => write!(f, "DEBUG"),
            DebugLevel::Info => write!(f, "INFO"),
            DebugLevel::Warn => write!(f, "WARN"),
            DebugLevel::Error => write!(f, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DebugConsoleStats {
    pub total: usize,
    pub errors: usize,
    pub warnings: usize,
    pub info: usize,
    pub debug: usize,
    pub trace: usize,
}

pub struct DebugConsole {
    buffer: RwLock<VecDeque<DebugLogEntry>>,
    max_entries: usize,
    tx: broadcast::Sender<DebugLogEntry>,
}

impl Default for DebugConsole {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugConsole {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1024);
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(1000)),
            max_entries: 10_000,
            tx,
        }
    }

    pub fn log(&self, level: DebugLevel, module: &str, message: String) {
        let entry = DebugLogEntry {
            timestamp: chrono::Utc::now(),
            level,
            module: module.to_string(),
            message,
            context: None,
        };

        // Broadcast to subscribers
        let _ = self.tx.send(entry.clone());

        // Store in ring buffer
        let mut buffer = self.buffer.write();
        if buffer.len() >= self.max_entries {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }

    pub fn log_with_context(
        &self,
        level: DebugLevel,
        module: &str,
        message: String,
        context: serde_json::Value,
    ) {
        let entry = DebugLogEntry {
            timestamp: chrono::Utc::now(),
            level,
            module: module.to_string(),
            message,
            context: Some(context),
        };

        let _ = self.tx.send(entry.clone());

        let mut buffer = self.buffer.write();
        if buffer.len() >= self.max_entries {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }

    pub fn recent(&self, count: usize, min_level: Option<DebugLevel>) -> Vec<DebugLogEntry> {
        let buffer = self.buffer.read();
        let min = min_level.as_ref();
        buffer
            .iter()
            .rev()
            .filter(|e| {
                if let Some(min) = min {
                    format!("{}", e.level) >= format!("{}", min)
                } else {
                    true
                }
            })
            .take(count)
            .cloned()
            .collect()
    }

    pub fn query(&self, module: &str, level: Option<DebugLevel>) -> Vec<DebugLogEntry> {
        let buffer = self.buffer.read();
        buffer
            .iter()
            .rev()
            .filter(|e| {
                let module_match = e.module.contains(module);
                let level_match = level
                    .as_ref()
                    .is_none_or(|l| format!("{}", e.level) == format!("{}", l));
                module_match && level_match
            })
            .take(100)
            .cloned()
            .collect()
    }

    /// Subscribe to all log entries at or above the given level.
    pub fn subscribe(&self, _level_filter: DebugLevel) -> broadcast::Receiver<DebugLogEntry> {
        // Note: The receiver will get all messages; filtering by level
        // is done on the consumer side for simplicity.
        self.tx.subscribe()
    }

    /// Export all logs as a JSON string.
    pub fn export(&self) -> String {
        let buffer = self.buffer.read();
        serde_json::to_string_pretty(&*buffer)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {}\"}}", e))
    }

    /// Full-text search within logs.
    pub fn search(&self, query: &str) -> Vec<DebugLogEntry> {
        let buffer = self.buffer.read();
        let query_lower = query.to_lowercase();
        buffer
            .iter()
            .rev()
            .filter(|e| {
                e.message.to_lowercase().contains(&query_lower)
                    || e.module.to_lowercase().contains(&query_lower)
                    || e.context.as_ref().is_some_and(|c| {
                        c.to_string().to_lowercase().contains(&query_lower)
                    })
            })
            .take(100)
            .cloned()
            .collect()
    }

    /// Get log statistics by level.
    pub fn stats(&self) -> DebugConsoleStats {
        let buffer = self.buffer.read();
        let mut stats = DebugConsoleStats {
            total: 0,
            errors: 0,
            warnings: 0,
            info: 0,
            debug: 0,
            trace: 0,
        };

        for entry in buffer.iter() {
            stats.total += 1;
            match entry.level {
                DebugLevel::Error => stats.errors += 1,
                DebugLevel::Warn => stats.warnings += 1,
                DebugLevel::Info => stats.info += 1,
                DebugLevel::Debug => stats.debug += 1,
                DebugLevel::Trace => stats.trace += 1,
            }
        }

        stats
    }

    /// Clear all logs.
    pub fn clear(&self) {
        self.buffer.write().clear();
    }
}
