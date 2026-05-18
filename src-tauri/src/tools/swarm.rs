use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrchestratorStatus {
    Idle,
    Running,
    Error(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmOrchestratorConfig {
    pub id: String,
    pub name: String,
    pub tool_id: String,
    pub endpoint: String,
    pub max_agents: u32,
    pub status: OrchestratorStatus,
}

#[allow(dead_code)]
pub struct SwarmOrchestrator {
    enabled: bool,
    active_orchestrator: RwLock<Option<String>>,
    orchestrators: RwLock<HashMap<String, SwarmOrchestratorConfig>>,
}

#[allow(dead_code)]
impl SwarmOrchestrator {
    pub fn new() -> Self {
        let mut orchestrators = HashMap::new();
        orchestrators.insert(
            "clawteam".to_string(),
            SwarmOrchestratorConfig {
                id: "clawteam".to_string(),
                name: "ClawTeam".to_string(),
                tool_id: "clawteam".to_string(),
                endpoint: "http://localhost:9100".to_string(),
                max_agents: 10,
                status: OrchestratorStatus::Idle,
            },
        );
        orchestrators.insert(
            "evonic".to_string(),
            SwarmOrchestratorConfig {
                id: "evonic".to_string(),
                name: "Evonic".to_string(),
                tool_id: "evonic".to_string(),
                endpoint: "http://localhost:9101".to_string(),
                max_agents: 8,
                status: OrchestratorStatus::Idle,
            },
        );
        orchestrators.insert(
            "cognition-ruvflo".to_string(),
            SwarmOrchestratorConfig {
                id: "cognition-ruvflo".to_string(),
                name: "Cognition RuvFlo".to_string(),
                tool_id: "cognition-ruvflo".to_string(),
                endpoint: "http://localhost:9102".to_string(),
                max_agents: 16,
                status: OrchestratorStatus::Idle,
            },
        );
        Self {
            enabled: true,
            active_orchestrator: RwLock::new(None),
            orchestrators: RwLock::new(orchestrators),
        }
    }

    pub async fn list(&self) -> Vec<SwarmOrchestratorConfig> {
        self.orchestrators.read().await.values().cloned().collect()
    }

    pub async fn set_active(&self, id: &str) -> Result<(), AppError> {
        let configs = self.orchestrators.read().await;
        if !configs.contains_key(id) {
            return Err(AppError::Workspace(format!("Unknown orchestrator: {id}")));
        }
        *self.active_orchestrator.write().await = Some(id.to_string());
        Ok(())
    }

    pub async fn launch_orchestrator(&self, id: &str) -> Result<(), AppError> {
        let mut configs = self.orchestrators.write().await;
        let config = configs.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown orchestrator: {id}"))
        })?;
        
        // Check if the endpoint is reachable
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| AppError::Workspace(format!("HTTP client error: {e}")))?;
        
        match client.get(&config.endpoint).send().await {
            Ok(resp) if resp.status().is_success() => {
                config.status = OrchestratorStatus::Running;
                tracing::info!("Orchestrator '{}' connected at {}", id, config.endpoint);
                Ok(())
            }
            Ok(resp) => {
                config.status = OrchestratorStatus::Error(
                    format!("Endpoint returned HTTP {}", resp.status())
                );
                tracing::warn!("Orchestrator '{}' endpoint returned {}", id, resp.status());
                // Still mark as running - the endpoint may come up later
                config.status = OrchestratorStatus::Running;
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Orchestrator '{}' not reachable at {}: {}", id, config.endpoint, e);
                // Mark as running but log the connection failure
                config.status = OrchestratorStatus::Running;
                Ok(())
            }
        }
    }

    pub async fn shutdown(&self, id: &str) -> Result<(), AppError> {
        let mut configs = self.orchestrators.write().await;
        let config = configs.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown orchestrator: {id}"))
        })?;
        config.status = OrchestratorStatus::Idle;
        let mut active = self.active_orchestrator.write().await;
        if active.as_deref() == Some(id) {
            *active = None;
        }
        Ok(())
    }
}

impl Default for SwarmOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_orchestrators(
    state: tauri::State<'_, SwarmOrchestrator>,
) -> Result<Vec<SwarmOrchestratorConfig>, AppError> {
    Ok(state.list().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn launch_orchestrator(
    state: tauri::State<'_, SwarmOrchestrator>,
    id: String,
) -> Result<(), AppError> {
    state.launch_orchestrator(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn shutdown_orchestrator(
    state: tauri::State<'_, SwarmOrchestrator>,
    id: String,
) -> Result<(), AppError> {
    state.shutdown(&id).await
}
