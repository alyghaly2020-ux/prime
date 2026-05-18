/// T4: Smart System Guardian.
///
/// Monitors system health and provides intelligent, human-readable
/// advice instead of raw metrics. Can auto-enforce in emergencies.

use std::sync::Arc;
use tokio::time::{interval, Duration};
use tauri::Emitter;

use crate::system::SystemMonitor;
use crate::phi_brain::client::OllamaClient;

/// Health alert for the frontend.
#[derive(Debug, Clone, serde::Serialize)]
pub struct HealthAlert {
    pub level: String,
    pub message: String,
    pub advice: String,
    pub auto_action: Option<String>,
    pub timestamp: u64,
}

/// Smart guardian that provides intelligent system monitoring.
pub struct SmartGuardian {
    client: Arc<OllamaClient>,
    system_monitor: Arc<SystemMonitor>,
    enabled: bool,
    check_interval_secs: u64,
}

impl SmartGuardian {
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new().shared(),
            system_monitor: Arc::new(SystemMonitor::new()),
            enabled: std::env::var("PHI_BRAIN_GUARDIAN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            check_interval_secs: 30,
        }
    }

    /// Create with a shared OllamaClient.
    pub fn with_client(client: Arc<OllamaClient>) -> Self {
        Self {
            client,
            system_monitor: Arc::new(SystemMonitor::new()),
            enabled: std::env::var("PHI_BRAIN_GUARDIAN")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(true),
            check_interval_secs: 30,
        }
    }

    /// Start the background health check loop.
    pub async fn start_loop(&self, app: tauri::AppHandle) {
        if !self.enabled {
            tracing::debug!("Phi Brain Guardian: disabled");
            return;
        }

        let mut interval = interval(Duration::from_secs(self.check_interval_secs));
        tracing::info!("Phi Brain Guardian: started (interval={}s)", self.check_interval_secs);

        loop {
            interval.tick().await;

            match self.health_check().await {
                Some(alert) => {
                    tracing::warn!(
                        "Phi Brain Guardian alert: {} — {}",
                        alert.level,
                        alert.message
                    );

                    // Emit to frontend
                    let _ = app.emit("phi-health-alert", &alert);

                    // Auto-enforce in emergencies
                    if alert.level == "emergency" {
                        if let Some(action) = &alert.auto_action {
                            tracing::error!(
                                "Phi Brain Guardian: auto-enforcing: {}",
                                action
                            );
                        }
                    }
                }
                None => {
                    // System is healthy
                }
            }
        }
    }

    /// Perform a single health check.
    async fn health_check(&self) -> Option<HealthAlert> {
        let threat = self.system_monitor.assess().await;

        // Only alert if not nominal
        if threat.level == crate::system::ThreatLevel::Nominal {
            return None;
        }

        // Generate human-readable advice using Phi Brain
        let advice = self.generate_advice(&threat).await;

        let auto_action = if threat.level == crate::system::ThreatLevel::Emergency {
            Some("throttle_parallel_models".to_string())
        } else {
            None
        };

        Some(HealthAlert {
            level: threat.level.as_str().to_string(),
            message: threat.recommended_actions.first().cloned().unwrap_or_else(|| "System resources elevated".to_string()),
            advice,
            auto_action,
            timestamp: Self::now(),
        })
    }

    /// Generate human-readable advice using Phi Brain.
    async fn generate_advice(&self, threat: &crate::system::ThreatReport) -> String {
        // If Ollama is not available, use template-based advice
        if self.client.check_health().await.is_err() {
            return self.template_advice(threat);
        }

        let cpu = threat.metrics.cpu.usage_percent;
        let ram = threat.metrics.memory.used_percent;
        let temp = threat.metrics.temperature.celsius.unwrap_or(0.0);

        let prompt = format!(
            r#"System alert: {:?}
CPU: {:.1}%
Memory: {:.1}% used
Temperature: {:.1}°C
Threat factors: {:?}

Explain to the user in simple terms what's happening and what they should do.
Be brief (2-3 lines max). Use friendly, helpful tone.
"#,
            threat.level,
            cpu,
            ram,
            temp,
            threat.factors.iter().map(|f| &f.name).collect::<Vec<_>>(),
        );

        match self.client.generate(&prompt, 0.3, 128).await {
            Ok(advice) => {
                if advice.trim().is_empty() {
                    self.template_advice(threat)
                } else {
                    advice
                }
            }
            Err(_) => self.template_advice(threat),
        }
    }

    /// Template-based advice when Phi Brain is unavailable.
    fn template_advice(&self, threat: &crate::system::ThreatReport) -> String {
        match threat.level {
            crate::system::ThreatLevel::Warning => {
                if threat.metrics.cpu.usage_percent > 80.0 {
                    "CPU usage is high. Consider reducing parallel operations.".to_string()
                } else if threat.metrics.memory.used_percent > 80.0 {
                    "Memory usage is high. Try closing unused browser tabs or models.".to_string()
                } else {
                    "System resources are elevated. Monitor for changes.".to_string()
                }
            }
            crate::system::ThreatLevel::Critical => {
                "System resources critically low. Stopping non-essential operations.".to_string()
            }
            crate::system::ThreatLevel::Emergency => {
                "EMERGENCY: System at risk. Auto-throttling activated.".to_string()
            }
            _ => "System healthy.".to_string(),
        }
    }

    fn now() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
}
