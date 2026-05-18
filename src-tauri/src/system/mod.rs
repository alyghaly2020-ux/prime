use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::dev::governance::{Constraints, Governance};

// =============================================================================
// Metrics
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub temperature: TemperatureMetrics,
    pub disk: DiskMetrics,
    pub load: LoadMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f64,
    pub cores: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_kb: u64,
    pub available_kb: u64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemperatureMetrics {
    pub celsius: Option<f64>,
    pub sensors_found: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub total_gb: f64,
    pub used_gb: f64,
    pub used_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadMetrics {
    pub one_min: f64,
    pub five_min: f64,
    pub fifteen_min: f64,
    pub per_core: f64,
}

// =============================================================================
// Threat Levels
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatLevel {
    Nominal,
    Warning,
    Critical,
    Emergency,
}

impl ThreatLevel {
    pub fn is_safe(&self) -> bool {
        matches!(self, ThreatLevel::Nominal)
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            ThreatLevel::Nominal => "nominal",
            ThreatLevel::Warning => "warning",
            ThreatLevel::Critical => "critical",
            ThreatLevel::Emergency => "emergency",
        }
    }
}

// =============================================================================
// Threat Report
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatReport {
    pub level: ThreatLevel,
    pub metrics: SystemMetrics,
    pub factors: Vec<ThreatFactor>,
    pub recommended_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatFactor {
    pub name: String,
    pub severity: ThreatLevel,
    pub detail: String,
}

// =============================================================================
// System Monitor (reads /proc + /sys directly)
// =============================================================================

pub struct SystemMonitor {
    last_metrics: RwLock<SystemMetrics>,
    threat_level: RwLock<ThreatLevel>,
    cpu_cores: u32,
}

impl Default for SystemMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemMonitor {
    pub fn new() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(1);

        Self {
            last_metrics: RwLock::new(SystemMetrics {
                cpu: CpuMetrics { usage_percent: 0.0, cores },
                memory: MemoryMetrics { total_kb: 0, available_kb: 0, used_percent: 0.0 },
                temperature: TemperatureMetrics { celsius: None, sensors_found: 0 },
                disk: DiskMetrics { total_gb: 0.0, used_gb: 0.0, used_percent: 0.0 },
                load: LoadMetrics { one_min: 0.0, five_min: 0.0, fifteen_min: 0.0, per_core: 0.0 },
            }),
            threat_level: RwLock::new(ThreatLevel::Nominal),
            cpu_cores: cores,
        }
    }

    // -------------------------------------------------------------------------
    // Metric Readers
    // -------------------------------------------------------------------------

    fn read_memory() -> MemoryMetrics {
        let raw = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
        let mut total_kb = 0u64;
        let mut available_kb = 0u64;
        for line in raw.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    total_kb = val.parse().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(val) = line.split_whitespace().nth(1) {
                    available_kb = val.parse().unwrap_or(0);
                }
            }
        }
        let used_kb = total_kb.saturating_sub(available_kb);
        let used_percent = if total_kb > 0 { (used_kb as f64 / total_kb as f64) * 100.0 } else { 0.0 };
        MemoryMetrics { total_kb, available_kb, used_percent }
    }

    fn read_load(cores: u32) -> LoadMetrics {
        let raw = std::fs::read_to_string("/proc/loadavg").unwrap_or_default();
        let parts: Vec<f64> = raw.split_whitespace()
            .take(3)
            .filter_map(|s| s.parse().ok())
            .collect();
        let one = parts.first().copied().unwrap_or(0.0);
        let five = parts.get(1).copied().unwrap_or(0.0);
        let fifteen = parts.get(2).copied().unwrap_or(0.0);
        let per_core = if cores > 0 { one / cores as f64 } else { 0.0 };
        LoadMetrics { one_min: one, five_min: five, fifteen_min: fifteen, per_core }
    }

    fn read_temperature() -> TemperatureMetrics {
        let mut temps = Vec::new();
        for entry in std::fs::read_dir("/sys/class/thermal").unwrap_or_else(|_| std::fs::read_dir("/dev/null").unwrap()).flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("thermal_zone") {
                let type_path = entry.path().join("type");
                let temp_path = entry.path().join("temp");
                if let Ok(typ) = std::fs::read_to_string(&type_path) {
                    let typ = typ.trim();
                    if typ.contains("cpu") || typ.contains("x86") || typ.contains("pkg") || typ == "acpitz" {
                        if let Ok(raw) = std::fs::read_to_string(&temp_path) {
                            if let Ok(millicelsius) = raw.trim().parse::<f64>() {
                                temps.push(millicelsius / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        let celsius = if temps.is_empty() { None } else { temps.iter().copied().reduce(f64::max) };
        TemperatureMetrics { celsius, sensors_found: temps.len() }
    }

    fn read_cpu_usage() -> f64 {
        // Simple approach: read /proc/stat idle vs total over a short interval
        fn read_cpu_jiffies() -> Option<(u64, u64)> {
            let line = std::fs::read_to_string("/proc/stat").ok()?
                .lines().next()?.to_string();
            let parts: Vec<u64> = line.split_whitespace()
                .skip(1).take(8)
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() < 5 { return None; }
            let idle = parts[3] + parts[4]; // idle + iowait
            let total: u64 = parts.iter().sum();
            Some((idle, total))
        }

        let before = read_cpu_jiffies();
        std::thread::sleep(std::time::Duration::from_millis(200));
        let after = read_cpu_jiffies();

        match (before, after) {
            (Some((idle_b, total_b)), Some((idle_a, total_a))) => {
                let idle_delta = idle_a.saturating_sub(idle_b);
                let total_delta = total_a.saturating_sub(total_b);
                if total_delta > 0 {
                    (1.0 - idle_delta as f64 / total_delta as f64) * 100.0
                } else { 0.0 }
            }
            _ => 0.0,
        }
    }

    fn read_disk() -> DiskMetrics {
        // Parse `df` output: reliable cross-distro block-level usage
        let output = std::process::Command::new("df")
            .arg("-B1")
            .arg("--output=size,used,avail")
            .arg("/")
            .output();
        match output {
            Ok(out) if out.status.success() => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut lines = stdout.lines();
                lines.next(); // skip header
                if let Some(line) = lines.next() {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let (Ok(total), Ok(used)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                            let used_percent = if total > 0.0 { (used / total) * 100.0 } else { 0.0 };
                            return DiskMetrics {
                                total_gb: total / 1_073_741_824.0,
                                used_gb: used / 1_073_741_824.0,
                                used_percent,
                            };
                        }
                    }
                }
                DiskMetrics { total_gb: 0.0, used_gb: 0.0, used_percent: 0.0 }
            }
            _ => DiskMetrics { total_gb: 0.0, used_gb: 0.0, used_percent: 0.0 },
        }
    }

    // -------------------------------------------------------------------------
    // Collect & Analyze
    // -------------------------------------------------------------------------

    pub async fn collect_metrics(&self) -> SystemMetrics {
        let memory = Self::read_memory();
        let load = Self::read_load(self.cpu_cores);
        let temp = Self::read_temperature();
        let cpu = CpuMetrics { usage_percent: Self::read_cpu_usage(), cores: self.cpu_cores };
        let disk = Self::read_disk();

        let metrics = SystemMetrics { cpu, memory, temperature: temp, disk, load };
        *self.last_metrics.write().await = metrics.clone();
        metrics
    }

    pub async fn analyze_threats(&self, metrics: &SystemMetrics) -> ThreatReport {
        let mut factors = Vec::new();
        let mut recommended = Vec::new();

        // CPU load threat
        if metrics.load.per_core > 3.0 {
            factors.push(ThreatFactor {
                name: "cpu_overload".into(),
                severity: ThreatLevel::Emergency,
                detail: format!("CPU load {:.2} per core — system is overloaded", metrics.load.per_core),
            });
            recommended.push("Reduce concurrent agent count immediately".into());
        } else if metrics.load.per_core > 1.5 {
            factors.push(ThreatFactor {
                name: "cpu_high_load".into(),
                severity: ThreatLevel::Critical,
                detail: format!("CPU load {:.2} per core — high utilization", metrics.load.per_core),
            });
            recommended.push("Throttle background agents".into());
        } else if metrics.load.per_core > 0.8 {
            factors.push(ThreatFactor {
                name: "cpu_elevated".into(),
                severity: ThreatLevel::Warning,
                detail: format!("CPU load {:.2} per core", metrics.load.per_core),
            });
        }

        // Memory threat
        if metrics.memory.used_percent > 95.0 {
            factors.push(ThreatFactor {
                name: "memory_critical".into(),
                severity: ThreatLevel::Emergency,
                detail: format!("RAM {:.1}% used — OOM risk", metrics.memory.used_percent),
            });
            recommended.push("Kill non-essential agents. Clear caches.".into());
        } else if metrics.memory.used_percent > 85.0 {
            factors.push(ThreatFactor {
                name: "memory_high".into(),
                severity: ThreatLevel::Critical,
                detail: format!("RAM {:.1}% used", metrics.memory.used_percent),
            });
            recommended.push("Enable memory compression. Reduce token budgets.".into());
        } else if metrics.memory.used_percent > 70.0 {
            factors.push(ThreatFactor {
                name: "memory_elevated".into(),
                severity: ThreatLevel::Warning,
                detail: format!("RAM {:.1}% used", metrics.memory.used_percent),
            });
        }

        // Temperature threat
        if let Some(temp) = metrics.temperature.celsius {
            if temp > 95.0 {
                factors.push(ThreatFactor {
                    name: "thermal_emergency".into(),
                    severity: ThreatLevel::Emergency,
                    detail: format!("CPU {:.1}°C — hardware damage risk!", temp),
                });
                recommended.push("Emergency throttle: freeze all non-critical work".into());
            } else if temp > 85.0 {
                factors.push(ThreatFactor {
                    name: "thermal_critical".into(),
                    severity: ThreatLevel::Critical,
                    detail: format!("CPU {:.1}°C — throttling recommended", temp),
                });
                recommended.push("Reduce thread count. Lower power mode.".into());
            } else if temp > 75.0 {
                factors.push(ThreatFactor {
                    name: "thermal_warning".into(),
                    severity: ThreatLevel::Warning,
                    detail: format!("CPU {:.1}°C", temp),
                });
            }
        }

        // Disk threat
        if metrics.disk.used_percent > 95.0 {
            factors.push(ThreatFactor {
                name: "disk_full".into(),
                severity: ThreatLevel::Emergency,
                detail: format!("Disk {:.1}% full — system may fail", metrics.disk.used_percent),
            });
            recommended.push("Clear logs, temp files, old sessions".into());
        } else if metrics.disk.used_percent > 85.0 {
            factors.push(ThreatFactor {
                name: "disk_high".into(),
                severity: ThreatLevel::Critical,
                detail: format!("Disk {:.1}% full", metrics.disk.used_percent),
            });
        }

        // Determine overall level
        let level = if factors.iter().any(|f| f.severity == ThreatLevel::Emergency) {
            ThreatLevel::Emergency
        } else if factors.iter().any(|f| f.severity == ThreatLevel::Critical) {
            ThreatLevel::Critical
        } else if factors.iter().any(|f| f.severity == ThreatLevel::Warning) {
            ThreatLevel::Warning
        } else {
            ThreatLevel::Nominal
        };

        *self.threat_level.write().await = level.clone();

        ThreatReport { level, metrics: metrics.clone(), factors, recommended_actions: recommended }
    }

    /// Combined: collect + analyze in one call
    pub async fn assess(&self) -> ThreatReport {
        let metrics = self.collect_metrics().await;
        self.analyze_threats(&metrics).await
    }

    /// Apply threat response through Governance
    pub async fn enforce(&self, report: &ThreatReport, gov: &Governance) {
        match report.level {
            ThreatLevel::Emergency => {
                let mut c = gov.constraints().await;
                c.max_concurrent_agents = 2;
                c.max_workflow_steps = 5;
                c.max_agent_retries = 1;
                c.max_tokens_per_action = 10_000;
                gov.set_constraints(c).await;
                tracing::error!(
                    "[SystemGuardian] EMERGENCY — throttled to 2 agents, 5 steps, 10k tokens"
                );
            }
            ThreatLevel::Critical => {
                let mut c = gov.constraints().await;
                c.max_concurrent_agents = 4;
                c.max_workflow_steps = 10;
                c.max_agent_retries = 2;
                c.max_tokens_per_action = 50_000;
                gov.set_constraints(c).await;
                tracing::warn!(
                    "[SystemGuardian] CRITICAL — throttled to 4 agents, 10 steps, 50k tokens"
                );
            }
            ThreatLevel::Warning => {
                let mut c = gov.constraints().await;
                c.max_concurrent_agents = 6;
                gov.set_constraints(c).await;
                tracing::warn!("[SystemGuardian] WARNING — throttled to 6 agents");
            }
            ThreatLevel::Nominal => {
                // Restore defaults when safe
                let c = gov.constraints().await;
                if c.max_concurrent_agents < 10 {
                    gov.set_constraints(Constraints::default()).await;
                    tracing::info!("[SystemGuardian] NOMINAL — restored default constraints");
                }
            }
        }
    }

    pub async fn current_threat_level(&self) -> ThreatLevel {
        self.threat_level.read().await.clone()
    }

    pub async fn latest_metrics(&self) -> SystemMetrics {
        self.last_metrics.read().await.clone()
    }
}
