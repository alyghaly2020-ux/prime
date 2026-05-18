use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    pub cpu_limit: f64, // CPU cores fraction
    pub memory_limit_mb: u64,
    pub time_limit_secs: u64,
    pub network_access: bool,
    pub filesystem_access: Vec<String>,
    pub env_vars: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            cpu_limit: 1.0,
            memory_limit_mb: 256,
            time_limit_secs: 30,
            network_access: false,
            filesystem_access: vec![],
            env_vars: vec![],
        }
    }
}

pub struct Sandbox {
    configs: RwLock<std::collections::HashMap<String, SandboxConfig>>,
}

impl Default for Sandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Sandbox {
    pub fn new() -> Self {
        Self {
            configs: RwLock::new(std::collections::HashMap::new()),
        }
    }

    pub async fn create(&self, id: String, config: SandboxConfig) {
        self.configs.write().await.insert(id, config);
    }

    pub async fn get_config(&self, id: &str) -> Option<SandboxConfig> {
        self.configs.read().await.get(id).cloned()
    }

    pub async fn remove(&self, id: &str) {
        self.configs.write().await.remove(id);
    }
}
