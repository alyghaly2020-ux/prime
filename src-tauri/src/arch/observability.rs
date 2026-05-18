use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize)]
pub struct MetricsSnapshot {
    pub events_processed: u64,
    pub active_agents: usize,
    pub active_tasks: usize,
    pub memory_usage_mb: u64,
    pub cpu_usage_pct: f64,
    pub uptime_secs: u64,
}

pub struct Observability {
    metrics: RwLock<MetricsSnapshot>,
}

impl Default for Observability {
    fn default() -> Self {
        Self::new()
    }
}

impl Observability {
    pub fn new() -> Self {
        Self {
            metrics: RwLock::new(MetricsSnapshot {
                events_processed: 0,
                active_agents: 0,
                active_tasks: 0,
                memory_usage_mb: 0,
                cpu_usage_pct: 0.0,
                uptime_secs: 0,
            }),
        }
    }

    pub async fn snapshot(&self) -> MetricsSnapshot {
        self.metrics.read().await.clone()
    }

    pub async fn increment_events(&self) {
        let mut m = self.metrics.write().await;
        m.events_processed += 1;
    }

    pub async fn update_agents(&self, count: usize) {
        self.metrics.write().await.active_agents = count;
    }
}
