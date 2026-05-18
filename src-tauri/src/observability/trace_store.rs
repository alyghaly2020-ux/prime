//! Trace Store
//!
//! Stores completed trace spans for post-hoc analysis.
//! Provides querying for recent traces, trace details by ID, and aggregate stats.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TraceSpan {
    pub id: String,
    pub parent_id: Option<String>,
    pub trace_id: String,
    pub service: String,
    pub span_name: String,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
    pub status: SpanStatus,
    pub error: Option<String>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SpanStatus {
    Ok,
    Error,
    Cancelled,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TraceStats {
    pub total_traces: u64,
    pub total_spans: u64,
    pub avg_duration_ms: f64,
    pub error_rate: f64,
    pub spans_by_service: HashMap<String, u64>,
}

pub struct TraceStore {
    spans: RwLock<Vec<TraceSpan>>,
    max_spans: usize,
    trace_counter: AtomicU64,
}

impl Default for TraceStore {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceStore {
    pub fn new() -> Self {
        Self {
            spans: RwLock::new(Vec::with_capacity(1000)),
            max_spans: 50_000,
            trace_counter: AtomicU64::new(0),
        }
    }

    /// Record a completed trace span.
    pub fn record_span(&self, span: TraceSpan) {
        let mut spans = self.spans.write();
        if spans.len() >= self.max_spans {
            // Remove oldest entries (keep newest)
            let remove_count = spans.len().saturating_sub(self.max_spans) + 1;
            spans.drain(0..remove_count);
        }
        spans.push(span);
        self.trace_counter.fetch_add(1, Ordering::Relaxed);
    }

    /// Get the most recent n traces (by their root spans).
    pub fn get_recent_traces(&self, n: usize) -> Vec<TraceSpan> {
        let spans = self.spans.read();
        spans
            .iter()
            .rev()
            .filter(|s| s.parent_id.is_none()) // root spans only
            .take(n)
            .cloned()
            .collect()
    }

    /// Get all spans for a specific trace by trace_id.
    pub fn get_trace_by_id(&self, trace_id: &str) -> Vec<TraceSpan> {
        let spans = self.spans.read();
        spans
            .iter()
            .filter(|s| s.trace_id == trace_id)
            .cloned()
            .collect()
    }

    /// Get aggregate trace statistics.
    pub fn get_trace_stats(&self) -> TraceStats {
        let spans = self.spans.read();
        let total_spans = spans.len() as u64;
        let total_traces = self.trace_counter.load(Ordering::Relaxed);

        let mut total_duration: u64 = 0;
        let mut duration_count: u64 = 0;
        let mut error_count: u64 = 0;
        let mut spans_by_service: HashMap<String, u64> = HashMap::new();

        for span in spans.iter() {
            *spans_by_service.entry(span.service.clone()).or_default() += 1;

            if let Some(dur) = span.duration_ms {
                total_duration += dur;
                duration_count += 1;
            }

            if matches!(span.status, SpanStatus::Error) {
                error_count += 1;
            }
        }

        let avg_duration_ms = if duration_count > 0 {
            total_duration as f64 / duration_count as f64
        } else {
            0.0
        };

        let error_rate = if total_spans > 0 {
            error_count as f64 / total_spans as f64
        } else {
            0.0
        };

        TraceStats {
            total_traces,
            total_spans,
            avg_duration_ms,
            error_rate,
            spans_by_service,
        }
    }

    /// Clear all stored traces.
    pub fn clear(&self) {
        self.spans.write().clear();
        self.trace_counter.store(0, Ordering::Relaxed);
    }
}
