use std::collections::{HashMap, VecDeque};
use std::time::Instant;
use tokio::sync::RwLock;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_TRACE_HISTORY: usize = 4096;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub duration: std::time::Duration,
    pub success: bool,
    pub metadata: HashMap<String, String>,
}

// ---------------------------------------------------------------------------
// Ring buffer for recent trace history
// ---------------------------------------------------------------------------

struct TraceRingBuffer {
    entries: VecDeque<TraceEntry>,
    max: usize,
}

impl TraceRingBuffer {
    fn new(max: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max),
            max,
        }
    }

    fn push(&mut self, entry: TraceEntry) {
        if self.entries.len() >= self.max {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    fn recent(&self, n: usize) -> Vec<TraceEntry> {
        self.entries.iter().rev().take(n).cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Tracer
// ---------------------------------------------------------------------------

pub struct Tracer {
    enabled: RwLock<bool>,
    sample_rate: RwLock<f64>,
    history: RwLock<TraceRingBuffer>,
}

impl Tracer {
    pub fn new() -> Self {
        Self {
            enabled: RwLock::new(true),
            sample_rate: RwLock::new(0.1),
            history: RwLock::new(TraceRingBuffer::new(MAX_TRACE_HISTORY)),
        }
    }

    pub async fn is_enabled(&self) -> bool {
        *self.enabled.read().await
    }

    pub async fn set_enabled(&self, enabled: bool) {
        *self.enabled.write().await = enabled;
    }

    pub async fn should_sample(&self) -> bool {
        let rate = *self.sample_rate.read().await;
        rand::random::<f64>() < rate
    }

    // ------------------------------------------------------------------
    // Basic synchronous trace
    // ------------------------------------------------------------------

    /// Run `f` inside a `tracing::info_span!` and record timing.
    pub async fn trace<T, F>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> T,
    {
        let span = tracing::info_span!("trace", name = %name);
        let _guard = span.enter();
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        self.record_entry(name, None, start, duration, true, HashMap::new())
            .await;

        result
    }

    /// Like `trace` but also records caller-supplied key-value pairs.
    pub async fn trace_with_metadata<T, F>(
        &self,
        name: &str,
        metadata: HashMap<String, String>,
        f: F,
    ) -> T
    where
        F: FnOnce() -> T,
    {
        let span = tracing::info_span!("trace", name = %name, ?metadata);
        let _guard = span.enter();
        let start = Instant::now();
        let result = f();
        let duration = start.elapsed();

        self.record_entry(name, None, start, duration, true, metadata)
            .await;

        result
    }

    // ------------------------------------------------------------------
    // Async trace
    // ------------------------------------------------------------------

    /// Wrap an async function `f` in a `tracing::info_span!`, measuring
    /// wall-clock duration.
    pub async fn trace_async<F, Fut, T>(&self, name: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let span = tracing::info_span!("trace_async", name = %name);
        let start = Instant::now();
        let result = {
            let _guard = span.enter();
            f().await
        };
        let duration = start.elapsed();

        self.record_entry(name, None, start, duration, true, HashMap::new())
            .await;

        result
    }

    /// Async trace with explicit parent span (for nesting).
    pub async fn trace_child<F, Fut, T>(&self, name: &str, parent_span: &tracing::Span, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let child_span = tracing::info_span!(parent: parent_span, "trace_child", name = %name);
        let start = Instant::now();
        let result = {
            let _guard = child_span.enter();
            f().await
        };
        let duration = start.elapsed();

        self.record_entry(name, None, start, duration, true, HashMap::new())
            .await;

        result
    }

    // ------------------------------------------------------------------
    // History / introspection
    // ------------------------------------------------------------------

    /// Return the `n` most-recent completed trace entries.
    pub async fn recent_traces(&self, n: usize) -> Vec<TraceEntry> {
        self.history.read().await.recent(n)
    }

    /// Return all stored history (for export / debugging).
    pub async fn all_traces(&self) -> Vec<TraceEntry> {
        self.history.read().await.recent(MAX_TRACE_HISTORY)
    }

    // ------------------------------------------------------------------
    // Internal helpers
    // ------------------------------------------------------------------

    async fn record_entry(
        &self,
        name: &str,
        parent_id: Option<String>,
        _start: Instant,
        duration: std::time::Duration,
        success: bool,
        metadata: HashMap<String, String>,
    ) {
        let entry = TraceEntry {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            parent_id,
            start_time: chrono::Utc::now()
                - chrono::Duration::from_std(duration).unwrap_or_default(),
            duration,
            success,
            metadata,
        };

        tracing::trace!(
            trace_id = %entry.id,
            name = %entry.name,
            duration_us = duration.as_micros(),
            success = %entry.success,
            "trace completed"
        );

        self.history.write().await.push(entry);
    }
}

impl Default for Tracer {
    fn default() -> Self {
        Self::new()
    }
}
