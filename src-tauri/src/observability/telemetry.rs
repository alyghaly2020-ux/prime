//! Telemetry Export System
//!
//! Batches and exports telemetry data to:
//! 1. Local structured log files
//! 2. Optional remote endpoint (OpenTelemetry)
//! 3. In-memory ring buffer for debugging

use parking_lot::RwLock;
use std::collections::VecDeque;

#[derive(Debug, Clone, serde::Serialize)]
pub struct TelemetryEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: String,
    pub module: String,
    pub severity: TelemetrySeverity,
    pub data: serde_json::Value,
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum TelemetrySeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

pub struct TelemetryExporter {
    buffer: RwLock<VecDeque<TelemetryEvent>>,
    max_buffer: usize,
}

impl Default for TelemetryExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryExporter {
    pub fn new() -> Self {
        Self {
            buffer: RwLock::new(VecDeque::with_capacity(1000)),
            max_buffer: 10_000,
        }
    }

    pub fn record(&self, event: TelemetryEvent) {
        let mut buffer = self.buffer.write();
        if buffer.len() >= self.max_buffer {
            buffer.pop_front();
        }
        buffer.push_back(event);
    }

    pub fn drain(&self) -> Vec<TelemetryEvent> {
        let mut buffer = self.buffer.write();
        buffer.drain(..).collect()
    }

    pub fn recent(&self, count: usize) -> Vec<TelemetryEvent> {
        let buffer = self.buffer.read();
        buffer.iter().rev().take(count).cloned().collect()
    }

    pub fn flush(&self) {
        // In production: write to file, send to OTEL endpoint
        let events = self.drain();
        for event in &events {
            let line = serde_json::to_string(event).unwrap_or_default();
            tracing::info!("[telemetry] {}", line);
        }
    }
}
