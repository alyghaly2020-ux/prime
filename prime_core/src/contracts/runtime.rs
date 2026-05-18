//! Core Runtime Contract
//! Defines the runtime lifecycle and capabilities interface.

use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeCapabilities {
    pub has_wasm: bool,
    pub has_gpu: bool,
    pub has_network: bool,
    pub max_memory_mb: u64,
    pub max_cpu_cores: f64,
    pub supported_languages: Vec<String>,
}

#[async_trait]
pub trait RuntimeLifecycle: Send + Sync {
    async fn init(&self) -> anyhow::Result<()>;
    async fn shutdown(&self) -> anyhow::Result<()>;
    async fn health_check(&self) -> RuntimeHealth;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeHealth {
    pub status: String,
    pub uptime_secs: u64,
    pub memory_used_mb: u64,
    pub cpu_usage_pct: f64,
    pub active_tasks: usize,
    pub active_connections: usize,
}
