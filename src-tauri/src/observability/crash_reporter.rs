//! Crash Analytics & Reporter
//!
//! Captures panic information, stores crash data, and generates
//! crash reports for analysis.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CrashRecord {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub error: String,
    pub backtrace: Option<String>,
    pub module: Option<String>,
    pub severity: CrashSeverity,
    pub context: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum CrashSeverity {
    Low,
    Medium,
    High,
    Critical,
}

impl std::fmt::Display for CrashSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrashSeverity::Low => write!(f, "low"),
            CrashSeverity::Medium => write!(f, "medium"),
            CrashSeverity::High => write!(f, "high"),
            CrashSeverity::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrashStats {
    pub total: u64,
    pub most_common: Vec<(String, u64)>,
    pub by_severity: HashMap<String, u64>,
    pub last_crash: Option<CrashRecord>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CrashReport {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub total_crashes: u64,
    pub crashes: Vec<CrashRecord>,
    pub stats: CrashStats,
}

pub struct CrashReporter {
    records: RwLock<Vec<CrashRecord>>,
    max_records: usize,
    crash_counter: AtomicU64,
    error_counter: RwLock<HashMap<String, u64>>,
}

impl Default for CrashReporter {
    fn default() -> Self {
        Self::new()
    }
}

impl CrashReporter {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(Vec::with_capacity(100)),
            max_records: 10_000,
            crash_counter: AtomicU64::new(0),
            error_counter: RwLock::new(HashMap::new()),
        }
    }

    /// Record a crash with error message and optional backtrace.
    pub fn record_crash(&self, error: &str, backtrace: Option<String>) -> CrashRecord {
        let id = uuid::Uuid::new_v4().to_string();
        let record = CrashRecord {
            id: id.clone(),
            timestamp: chrono::Utc::now(),
            error: error.to_string(),
            backtrace,
            module: None,
            severity: CrashSeverity::High,
            context: HashMap::new(),
        };

        {
            let mut records = self.records.write();
            if records.len() >= self.max_records {
                records.remove(0);
            }
            records.push(record.clone());
        }

        // Track error frequency
        {
            let mut counters = self.error_counter.write();
            *counters.entry(error.to_string()).or_insert(0) += 1;
        }

        self.crash_counter.fetch_add(1, Ordering::Relaxed);

        record
    }

    /// Record a crash with full context.
    pub fn record_crash_with_context(
        &self,
        error: &str,
        backtrace: Option<String>,
        module: &str,
        severity: CrashSeverity,
        context: HashMap<String, String>,
    ) -> CrashRecord {
        let id = uuid::Uuid::new_v4().to_string();
        let record = CrashRecord {
            id: id.clone(),
            timestamp: chrono::Utc::now(),
            error: error.to_string(),
            backtrace,
            module: Some(module.to_string()),
            severity,
            context,
        };

        {
            let mut records = self.records.write();
            if records.len() >= self.max_records {
                records.remove(0);
            }
            records.push(record.clone());
        }

        {
            let mut counters = self.error_counter.write();
            *counters.entry(error.to_string()).or_insert(0) += 1;
        }

        self.crash_counter.fetch_add(1, Ordering::Relaxed);

        record
    }

    /// Set up a panic hook that captures panics via the crash reporter.
    pub fn install_panic_hook(self: &Arc<Self>) {
        let reporter = self.clone();
        std::panic::set_hook(Box::new(move |panic_info| {
            let error = if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                s.to_string()
            } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "Unknown panic".to_string()
            };

            let location = panic_info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()));

            let backtrace = std::backtrace::Backtrace::force_capture();
            let bt_str = format!("{}", backtrace);

            let mut context = HashMap::new();
            if let Some(loc) = location {
                context.insert("location".to_string(), loc);
            }

            reporter.record_crash_with_context(
                &error,
                Some(bt_str),
                "panic_hook",
                CrashSeverity::Critical,
                context,
            );
        }));
    }

    /// Get crash statistics.
    pub fn get_crash_stats(&self) -> CrashStats {
        let records = self.records.read();
        let total = self.crash_counter.load(Ordering::Relaxed);

        let mut by_severity: HashMap<String, u64> = HashMap::new();
        for record in records.iter() {
            *by_severity.entry(record.severity.to_string()).or_default() += 1;
        }

        let mut error_counts: Vec<(String, u64)> = self
            .error_counter
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        error_counts.sort_by_key(|b| std::cmp::Reverse(b.1));
        let most_common = error_counts.into_iter().take(10).collect();

        let last_crash = records.last().cloned();

        CrashStats {
            total,
            most_common,
            by_severity,
            last_crash,
        }
    }

    /// Generate a comprehensive crash report.
    pub fn generate_report(&self) -> CrashReport {
        let records = self.records.read();
        let stats = self.get_crash_stats();

        CrashReport {
            generated_at: chrono::Utc::now(),
            total_crashes: self.crash_counter.load(Ordering::Relaxed),
            crashes: records.clone(),
            stats,
        }
    }

    /// Export crash report as formatted JSON string.
    pub fn export_report_json(&self) -> String {
        let report = self.generate_report();
        serde_json::to_string_pretty(&report)
            .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {}\"}}", e))
    }

    /// Get recent crash records.
    pub fn recent(&self, count: usize) -> Vec<CrashRecord> {
        let records = self.records.read();
        records.iter().rev().take(count).cloned().collect()
    }

    /// Clear all crash records.
    pub fn clear(&self) {
        self.records.write().clear();
        self.error_counter.write().clear();
        self.crash_counter.store(0, Ordering::Relaxed);
    }
}
