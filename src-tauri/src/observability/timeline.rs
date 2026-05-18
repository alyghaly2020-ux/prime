//! Event Timeline
//!
//! Ordered sequence of all significant system events.
//! Used for debugging, replay, and audit trails.

use parking_lot::RwLock;
use std::collections::VecDeque;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TimelineEntry {
    pub sequence: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub source: String,
    pub module_path: Option<String>,
}

pub struct EventTimeline {
    entries: RwLock<VecDeque<TimelineEntry>>,
    sequence: std::sync::atomic::AtomicU64,
    max_entries: usize,
}

impl Default for EventTimeline {
    fn default() -> Self {
        Self::new()
    }
}

impl EventTimeline {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(1000)),
            sequence: std::sync::atomic::AtomicU64::new(0),
            max_entries: 50_000,
        }
    }

    pub fn record(&self, event_type: &str, source: &str) {
        let seq = self
            .sequence
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut entries = self.entries.write();
        if entries.len() >= self.max_entries {
            entries.pop_front();
        }
        entries.push_back(TimelineEntry {
            sequence: seq,
            timestamp: chrono::Utc::now(),
            event_type: event_type.to_string(),
            source: source.to_string(),
            module_path: Some(source.to_string()),
        });
    }

    pub fn query(&self, event_type: &str, limit: usize) -> Vec<TimelineEntry> {
        let entries = self.entries.read();
        entries
            .iter()
            .rev()
            .filter(|e| e.event_type.contains(event_type))
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn all(&self) -> Vec<TimelineEntry> {
        self.entries.read().iter().cloned().collect()
    }

    pub fn clear(&self) {
        self.entries.write().clear();
    }
}
