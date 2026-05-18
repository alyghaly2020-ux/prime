//! Metrics Collection System
//!
//! Tracks: counters, gauges, histograms, and rates.
//! All metrics are namespaced by module.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, HistogramSummary>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct HistogramSummary {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub p50: f64,
    pub p95: f64,
    pub p99: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct RateInfo {
    pub name: String,
    pub count: u64,
    pub elapsed_secs: f64,
    pub rate_per_sec: f64,
}

struct CounterWithTime {
    value: AtomicU64,
    created_at: Instant,
    last_reset: RwLock<Instant>,
}

pub struct MetricsCollector {
    counters: RwLock<HashMap<String, CounterWithTime>>,
    gauges: RwLock<HashMap<String, f64>>,
    histograms: RwLock<HashMap<String, Vec<f64>>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    pub fn increment(&self, name: &str, value: u64) {
        let mut counters = self.counters.write();
        let counter = counters.entry(name.to_string()).or_insert_with(|| {
            let now = Instant::now();
            CounterWithTime {
                value: AtomicU64::new(0),
                created_at: now,
                last_reset: RwLock::new(now),
            }
        });
        counter.value.fetch_add(value, Ordering::Relaxed);
    }

    pub fn gauge(&self, name: &str, value: f64) {
        self.gauges.write().insert(name.to_string(), value);
    }

    pub fn observe(&self, name: &str, value: f64) {
        let mut histograms = self.histograms.write();
        histograms.entry(name.to_string()).or_default().push(value);
        // Keep last 1000 samples per metric
        if let Some(samples) = histograms.get(name) {
            if samples.len() > 1000 {
                let trimmed: Vec<f64> = samples.iter().rev().take(1000).cloned().collect();
                histograms.insert(name.to_string(), trimmed);
            }
        }
    }

    /// Compute the rate (events per second) for a counter.
    pub fn compute_rate(&self, name: &str) -> Option<RateInfo> {
        let counters = self.counters.read();
        let counter = counters.get(name)?;
        let count = counter.value.load(Ordering::Relaxed);
        let elapsed = counter.created_at.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 {
            count as f64 / elapsed
        } else {
            0.0
        };
        Some(RateInfo {
            name: name.to_string(),
            count,
            elapsed_secs: elapsed,
            rate_per_sec: rate,
        })
    }

    /// Get the top-k metrics by counter value.
    pub fn top_k(&self, count: usize) -> Vec<(String, u64)> {
        let counters = self.counters.read();
        let mut sorted: Vec<(String, u64)> = counters
            .iter()
            .map(|(k, v)| (k.clone(), v.value.load(Ordering::Relaxed)))
            .collect();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.1));
        sorted.truncate(count);
        sorted
    }

    /// Reset a specific counter metric to zero.
    pub fn reset_metric(&self, name: &str) -> bool {
        let counters = self.counters.write();
        if let Some(counter) = counters.get(name) {
            counter.value.store(0, Ordering::Relaxed);
            *counter.last_reset.write() = Instant::now();
            true
        } else {
            false
        }
    }

    /// Get the current value of a specific metric.
    pub fn get_metric(&self, name: &str) -> Option<u64> {
        let counters = self.counters.read();
        counters.get(name).map(|c| c.value.load(Ordering::Relaxed))
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let counters: HashMap<String, u64> = self
            .counters
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), v.value.load(Ordering::Relaxed)))
            .collect();

        let gauges = self.gauges.read().clone();

        let histograms: HashMap<String, HistogramSummary> = self
            .histograms
            .read()
            .iter()
            .map(|(k, v)| (k.clone(), Self::summarize(v)))
            .collect();

        MetricsSnapshot {
            counters,
            gauges,
            histograms,
        }
    }

    fn summarize(samples: &[f64]) -> HistogramSummary {
        let count = samples.len() as u64;
        if count == 0 {
            return HistogramSummary {
                count: 0,
                min: 0.0,
                max: 0.0,
                avg: 0.0,
                p50: 0.0,
                p95: 0.0,
                p99: 0.0,
            };
        }

        let mut sorted = samples.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let sum: f64 = sorted.iter().sum();
        let avg = sum / count as f64;

        HistogramSummary {
            count,
            min: sorted.first().copied().unwrap_or(0.0),
            max: sorted.last().copied().unwrap_or(0.0),
            avg,
            p50: sorted[sorted.len().saturating_mul(50) / 100],
            p95: sorted[sorted.len().saturating_mul(95) / 100],
            p99: sorted[sorted.len().saturating_mul(99) / 100],
        }
    }
}
