//! Observability Stack
//!
//! Central monitoring, metrics, tracing, and telemetry.
//! All modules report to this system - NO module reports elsewhere.

pub mod crash_reporter;
pub mod debug_console;
pub mod metrics;
pub mod task_monitor;
pub mod telemetry;
pub mod timeline;
pub mod trace_store;

use chrono::{DateTime, Utc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize)]
pub struct SystemTelemetry {
    pub app_version: String,
    pub uptime_secs: u64,
    pub metrics: metrics::MetricsSnapshot,
    pub active_spans: usize,
    pub events_processed: u64,
    pub memory_used_mb: u64,
    pub cpu_usage_pct: f64,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub active_connections: usize,
    pub active_skills: usize,
    pub active_agents: usize,
}

pub struct ObservabilitySystem {
    pub metrics: Arc<metrics::MetricsCollector>,
    pub telemetry: Arc<telemetry::TelemetryExporter>,
    pub timeline: Arc<timeline::EventTimeline>,
    pub debug: Arc<debug_console::DebugConsole>,
    pub trace_store: Arc<trace_store::TraceStore>,
    pub task_monitor: Arc<task_monitor::TaskMonitor>,
    pub crash_reporter: Arc<crash_reporter::CrashReporter>,
    started_at: tokio::time::Instant,
    events_processed: AtomicU64,
    last_error: RwLock<Option<(String, DateTime<Utc>)>>,
}

impl Default for ObservabilitySystem {
    fn default() -> Self {
        Self::new()
    }
}

impl ObservabilitySystem {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(metrics::MetricsCollector::new()),
            telemetry: Arc::new(telemetry::TelemetryExporter::new()),
            timeline: Arc::new(timeline::EventTimeline::new()),
            debug: Arc::new(debug_console::DebugConsole::new()),
            trace_store: Arc::new(trace_store::TraceStore::new()),
            task_monitor: Arc::new(task_monitor::TaskMonitor::new()),
            crash_reporter: Arc::new(crash_reporter::CrashReporter::new()),
            started_at: tokio::time::Instant::now(),
            events_processed: AtomicU64::new(0),
            last_error: RwLock::new(None),
        }
    }

    pub fn snapshot(&self) -> SystemTelemetry {
        SystemTelemetry {
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_secs: self.started_at.elapsed().as_secs(),
            metrics: self.metrics.snapshot(),
            active_spans: 0,
            events_processed: self.events_processed.load(Ordering::Relaxed),
            memory_used_mb: 0,
            cpu_usage_pct: 0.0,
            last_error: None,
            last_error_at: None,
            active_connections: 0,
            active_skills: 0,
            active_agents: 0,
        }
    }

    pub fn record_event(&self, event_type: &str, source: &str) {
        self.events_processed.fetch_add(1, Ordering::Relaxed);
        self.timeline.record(event_type, source);
    }

    pub fn record_error(&self, error: String) {
        let now = Utc::now();
        let mut last = self.last_error.blocking_write();
        *last = Some((error, now));
    }
}
