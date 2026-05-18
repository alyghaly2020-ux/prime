use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct ResourceBudget {
    pub cpu_limit: f64,
    pub memory_limit_mb: u64,
    pub time_limit_secs: u64,
    pub network_egress_mb: u64,
    pub file_descriptors: u32,
}

/// Tracks current resource usage alongside the limits.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ResourceUsage {
    pub current_cpu: f64,
    pub max_cpu: f64,
    pub current_memory_mb: u64,
    pub max_memory_mb: u64,
    pub current_time_secs: u64,
    pub max_time_secs: u64,
}

impl ResourceUsage {
    pub fn cpu_pct(&self) -> f64 {
        if self.max_cpu <= 0.0 {
            0.0
        } else {
            (self.current_cpu / self.max_cpu) * 100.0
        }
    }

    pub fn memory_pct(&self) -> f64 {
        if self.max_memory_mb == 0 {
            0.0
        } else {
            (self.current_memory_mb as f64 / self.max_memory_mb as f64) * 100.0
        }
    }

    pub fn time_pct(&self) -> f64 {
        if self.max_time_secs == 0 {
            0.0
        } else {
            (self.current_time_secs as f64 / self.max_time_secs as f64) * 100.0
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub enum Alert {
    MemoryWarning {
        current_mb: u64,
        limit_mb: u64,
        pct: f64,
    },
    CpuWarning {
        current: f64,
        limit: f64,
        pct: f64,
    },
    TimeWarning {
        current_secs: u64,
        limit_secs: u64,
        pct: f64,
    },
    MemoryExceeded {
        current_mb: u64,
        limit_mb: u64,
    },
    CpuExceeded {
        current: f64,
        limit: f64,
    },
    TimeExceeded {
        current_secs: u64,
        limit_secs: u64,
    },
}

pub struct ResourceLimiter {
    usage: RwLock<ResourceBudget>,
    limits: RwLock<ResourceBudget>,
    alerts: RwLock<Vec<Alert>>,
    max_observed_memory_mb: AtomicU64,
    max_observed_time_secs: AtomicU64,
}

impl Default for ResourceLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceLimiter {
    pub fn new() -> Self {
        Self {
            usage: RwLock::new(ResourceBudget {
                cpu_limit: 0.0,
                memory_limit_mb: 0,
                time_limit_secs: 0,
                network_egress_mb: 0,
                file_descriptors: 0,
            }),
            limits: RwLock::new(ResourceBudget {
                cpu_limit: 4.0,
                memory_limit_mb: 1024,
                time_limit_secs: 60,
                network_egress_mb: 100,
                file_descriptors: 256,
            }),
            alerts: RwLock::new(Vec::new()),
            max_observed_memory_mb: AtomicU64::new(0),
            max_observed_time_secs: AtomicU64::new(0),
        }
    }

    pub async fn check_limits(&self) -> Result<(), String> {
        let usage = self.usage.read().await;
        let limits = self.limits.read().await;

        if usage.cpu_limit > limits.cpu_limit {
            return Err("CPU limit exceeded".to_string());
        }
        if usage.memory_limit_mb > limits.memory_limit_mb {
            return Err("Memory limit exceeded".to_string());
        }
        if usage.time_limit_secs > limits.time_limit_secs {
            return Err("Time limit exceeded".to_string());
        }

        Ok(())
    }

    pub async fn track_usage(&self, cpu: f64, memory: u64, time: u64) {
        let mut usage = self.usage.write().await;
        usage.cpu_limit += cpu;
        usage.memory_limit_mb += memory;
        usage.time_limit_secs += time;

        // Track maximums
        if memory > self.max_observed_memory_mb.load(Ordering::Relaxed) {
            self.max_observed_memory_mb.store(memory, Ordering::Relaxed);
        }
        if time > self.max_observed_time_secs.load(Ordering::Relaxed) {
            self.max_observed_time_secs.store(time, Ordering::Relaxed);
        }
    }

    /// Enforce memory limits by checking current usage against limits.
    /// Returns an alert if limits are approached or exceeded.
    #[allow(clippy::if_same_then_else)]
    pub async fn enforce_memory_limit(&self) -> Option<Alert> {
        let usage = self.usage.read().await;
        let limits = self.limits.read().await;

        if usage.memory_limit_mb >= limits.memory_limit_mb {
            let alert = Alert::MemoryExceeded {
                current_mb: usage.memory_limit_mb,
                limit_mb: limits.memory_limit_mb,
            };
            self.alerts.write().await.push(alert.clone());
            Some(alert)
        } else if usage.memory_limit_mb >= (limits.memory_limit_mb as f64 * 0.9) as u64 {
            let alert = Alert::MemoryWarning {
                current_mb: usage.memory_limit_mb,
                limit_mb: limits.memory_limit_mb,
                pct: usage.memory_limit_mb as f64 / limits.memory_limit_mb as f64 * 100.0,
            };
            self.alerts.write().await.push(alert.clone());
            Some(alert)
        } else if usage.memory_limit_mb >= (limits.memory_limit_mb as f64 * 0.8) as u64 {
            let alert = Alert::MemoryWarning {
                current_mb: usage.memory_limit_mb,
                limit_mb: limits.memory_limit_mb,
                pct: usage.memory_limit_mb as f64 / limits.memory_limit_mb as f64 * 100.0,
            };
            self.alerts.write().await.push(alert.clone());
            Some(alert)
        } else {
            None
        }
    }

    /// Enforce CPU limits by checking current usage against limits.
    /// Returns an alert if limits are approached or exceeded.
    #[allow(clippy::if_same_then_else)]
    pub async fn enforce_cpu_limit(&self) -> Option<Alert> {
        let usage = self.usage.read().await;
        let limits = self.limits.read().await;

        if usage.cpu_limit >= limits.cpu_limit {
            let alert = Alert::CpuExceeded {
                current: usage.cpu_limit,
                limit: limits.cpu_limit,
            };
            self.alerts.write().await.push(alert.clone());
            Some(alert)
        } else if usage.cpu_limit >= limits.cpu_limit * 0.9 {
            let alert = Alert::CpuWarning {
                current: usage.cpu_limit,
                limit: limits.cpu_limit,
                pct: (usage.cpu_limit / limits.cpu_limit) * 100.0,
            };
            self.alerts.write().await.push(alert.clone());
            Some(alert)
        } else if usage.cpu_limit >= limits.cpu_limit * 0.8 {
            let alert = Alert::CpuWarning {
                current: usage.cpu_limit,
                limit: limits.cpu_limit,
                pct: (usage.cpu_limit / limits.cpu_limit) * 100.0,
            };
            self.alerts.write().await.push(alert.clone());
            Some(alert)
        } else {
            None
        }
    }

    /// Check limits AND take action (return error to kill task) if exceeded.
    pub async fn check_and_enforce(&self) -> Result<ResourceUsage, String> {
        let usage = self.usage.read().await;
        let limits = self.limits.read().await;

        let resource_usage = ResourceUsage {
            current_cpu: usage.cpu_limit,
            max_cpu: limits.cpu_limit,
            current_memory_mb: usage.memory_limit_mb,
            max_memory_mb: limits.memory_limit_mb,
            current_time_secs: usage.time_limit_secs,
            max_time_secs: limits.time_limit_secs,
        };

        if usage.cpu_limit > limits.cpu_limit {
            return Err(format!(
                "CPU limit exceeded: {:.2} / {:.2} cores",
                usage.cpu_limit, limits.cpu_limit
            ));
        }
        if usage.memory_limit_mb > limits.memory_limit_mb {
            return Err(format!(
                "Memory limit exceeded: {} / {} MB",
                usage.memory_limit_mb, limits.memory_limit_mb
            ));
        }
        if usage.time_limit_secs > limits.time_limit_secs {
            return Err(format!(
                "Time limit exceeded: {} / {} seconds",
                usage.time_limit_secs, limits.time_limit_secs
            ));
        }

        Ok(resource_usage)
    }

    /// Get current resource usage summary.
    pub async fn usage(&self) -> ResourceUsage {
        let usage = self.usage.read().await;
        let limits = self.limits.read().await;
        ResourceUsage {
            current_cpu: usage.cpu_limit,
            max_cpu: limits.cpu_limit,
            current_memory_mb: usage.memory_limit_mb,
            max_memory_mb: limits.memory_limit_mb,
            current_time_secs: usage.time_limit_secs,
            max_time_secs: limits.time_limit_secs,
        }
    }

    /// Drain all pending alerts.
    pub async fn drain_alerts(&self) -> Vec<Alert> {
        self.alerts.write().await.drain(..).collect()
    }

    pub async fn reset(&self) {
        *self.usage.write().await = ResourceBudget {
            cpu_limit: 0.0,
            memory_limit_mb: 0,
            time_limit_secs: 0,
            network_egress_mb: 0,
            file_descriptors: 0,
        };
        self.max_observed_memory_mb.store(0, Ordering::Relaxed);
        self.max_observed_time_secs.store(0, Ordering::Relaxed);
    }
}
