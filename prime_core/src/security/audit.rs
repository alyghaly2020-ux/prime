//! Audit Logging System
//!
//! Records security-relevant access events in an in-memory ring buffer
//! with automatic flush to disk. Supports querying and exporting.

use parking_lot::RwLock;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditLogEntry {
    pub id: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub action: String,
    pub subject: String,
    pub resource: String,
    pub result: AuditResult,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AuditResult {
    Allow,
    Deny,
}

impl std::fmt::Display for AuditResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditResult::Allow => write!(f, "ALLOW"),
            AuditResult::Deny => write!(f, "DENY"),
        }
    }
}

pub struct AuditLog {
    buffer: RwLock<VecDeque<AuditLogEntry>>,
    capacity: usize,
    sequence: AtomicU64,
    flush_counter: AtomicU64,
    flush_path: RwLock<Option<PathBuf>>,
    auto_flush_interval: u64,
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(10_000)),
            capacity: 10_000,
            sequence: AtomicU64::new(0),
            flush_counter: AtomicU64::new(0),
            flush_path: RwLock::new(None),
            auto_flush_interval: 100,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(capacity)),
            capacity,
            sequence: AtomicU64::new(0),
            flush_counter: AtomicU64::new(0),
            flush_path: RwLock::new(None),
            auto_flush_interval: 100,
        }
    }

    /// Set the path for automatic audit log flushing.
    pub fn set_flush_path(&self, path: PathBuf) {
        *self.flush_path.write() = Some(path);
    }

    /// Record a security-relevant access event.
    pub fn record_access(
        &self,
        action: &str,
        subject: &str,
        resource: &str,
        result: AuditResult,
        reason: Option<String>,
    ) -> AuditLogEntry {
        let id = self.sequence.fetch_add(1, Ordering::Relaxed);
        let entry = AuditLogEntry {
            id,
            timestamp: chrono::Utc::now(),
            action: action.to_string(),
            subject: subject.to_string(),
            resource: resource.to_string(),
            result,
            reason,
        };

        {
            let mut buffer = self.buffer.write();
            if buffer.len() >= self.capacity {
                buffer.pop_front();
            }
            buffer.push_back(entry.clone());
        }

        // Auto-flush check
        let count = self.flush_counter.fetch_add(1, Ordering::Relaxed) + 1;
        if count.is_multiple_of(self.auto_flush_interval) {
            let _ = self.try_flush();
        }

        entry
    }

    /// Query the audit log by action, subject, and/or time range.
    pub fn query_log(
        &self,
        action: Option<&str>,
        subject: Option<&str>,
        since: Option<chrono::DateTime<chrono::Utc>>,
        until: Option<chrono::DateTime<chrono::Utc>>,
        limit: usize,
    ) -> Vec<AuditLogEntry> {
        let buffer = self.buffer.read();
        buffer
            .iter()
            .rev()
            .filter(|e| {
                action.is_none_or(|a| e.action == a)
                    && subject.is_none_or(|s| e.subject == s)
                    && since.is_none_or(|t| e.timestamp >= t)
                    && until.is_none_or(|t| e.timestamp <= t)
            })
            .take(limit)
            .cloned()
            .collect()
    }

    /// Export all audit log entries as a JSON string.
    pub fn export_json(&self) -> String {
        let buffer = self.buffer.read();
        serde_json::to_string_pretty(&*buffer)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {}\"}}", e))
    }

    /// Export all audit log entries as a CSV string.
    pub fn export_csv(&self) -> String {
        let buffer = self.buffer.read();
        let mut csv = String::from("id,timestamp,action,subject,resource,result,reason\n");
        for entry in buffer.iter() {
            let reason = entry.reason.as_deref().unwrap_or("");
            // Escape commas in fields
            let safe_reason = reason.replace(',', " ");
            csv.push_str(&format!(
                "{},{},{},{},{},{},{}\n",
                entry.id,
                entry.timestamp.to_rfc3339(),
                entry.action,
                entry.subject,
                entry.resource,
                entry.result,
                safe_reason,
            ));
        }
        csv
    }

    /// Try to flush the audit log to disk.
    fn try_flush(&self) -> Result<(), String> {
        let path = self.flush_path.read();
        let path = match path.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        drop(path);

        // Append mode: write all entries since last flush
        // For simplicity, we export JSON to the file
        let json = self.export_json();
        let path = self.flush_path.read();
        let path_buf = match path.as_ref() {
            Some(p) => p.clone(),
            None => return Ok(()),
        };
        drop(path);
        std::fs::write(&path_buf, &json).map_err(|e| format!("Failed to flush audit log: {}", e))
    }

    /// Manually flush the audit log to disk.
    pub fn flush(&self) -> Result<(), String> {
        self.try_flush()
    }

    /// Get total number of entries recorded.
    pub fn total_entries(&self) -> u64 {
        self.sequence.load(Ordering::Relaxed)
    }

    /// Get current buffer size.
    pub fn buffer_len(&self) -> usize {
        self.buffer.read().len()
    }

    /// Clear the audit log buffer.
    pub fn clear(&self) {
        self.buffer.write().clear();
    }
}
