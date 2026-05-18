use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProviderStatus {
    Active,
    Inactive,
    Error(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderIntegration {
    pub id: String,
    pub name: String,
    pub backend_type: String,
    pub endpoint: String,
    pub models: Vec<String>,
    pub status: ProviderStatus,
}

#[allow(dead_code)]
pub struct AiIntegration {
    enabled_providers: RwLock<Vec<ProviderIntegration>>,
    active_backend: RwLock<Option<String>>,
}

#[allow(dead_code)]
impl AiIntegration {
    pub fn new() -> Self {
        let providers = vec![
            ProviderIntegration {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                backend_type: "openai".to_string(),
                endpoint: "https://api.openai.com/v1".to_string(),
                models: vec![
                    "gpt-4o".to_string(),
                    "gpt-4o-mini".to_string(),
                    "o3".to_string(),
                ],
                status: ProviderStatus::Inactive,
            },
            ProviderIntegration {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                backend_type: "anthropic".to_string(),
                endpoint: "https://api.anthropic.com/v1".to_string(),
                models: vec![
                    "claude-sonnet-4-20250514".to_string(),
                    "claude-haiku-3-5".to_string(),
                ],
                status: ProviderStatus::Inactive,
            },
            ProviderIntegration {
                id: "ollama".to_string(),
                name: "Ollama".to_string(),
                backend_type: "ollama".to_string(),
                endpoint: "http://localhost:11434".to_string(),
                models: vec![
                    "llama3.2".to_string(),
                    "mistral".to_string(),
                    "codellama".to_string(),
                ],
                status: ProviderStatus::Inactive,
            },
            ProviderIntegration {
                id: "openrouter".to_string(),
                name: "OpenRouter".to_string(),
                backend_type: "openrouter".to_string(),
                endpoint: "https://openrouter.ai/api/v1".to_string(),
                models: vec![
                    "auto".to_string(),
                ],
                status: ProviderStatus::Inactive,
            },
        ];
        Self {
            enabled_providers: RwLock::new(providers),
            active_backend: RwLock::new(None),
        }
    }

    pub async fn list(&self) -> Vec<ProviderIntegration> {
        self.enabled_providers.read().await.clone()
    }

    pub async fn set_active(&self, id: &str) -> Result<(), AppError> {
        let providers = self.enabled_providers.read().await;
        if !providers.iter().any(|p| p.id == id) {
            return Err(AppError::Workspace(format!("Unknown provider: {id}")));
        }
        *self.active_backend.write().await = Some(id.to_string());
        Ok(())
    }

    pub async fn test_connection(&self, id: &str) -> Result<u64, AppError> {
        let providers = self.enabled_providers.read().await;
        let _provider = providers.iter().find(|p| p.id == id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown provider: {id}"))
        })?;
        Ok(0u64)
    }
}

impl Default for AiIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_ai_providers(
    state: tauri::State<'_, AiIntegration>,
) -> Result<Vec<ProviderIntegration>, AppError> {
    Ok(state.list().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn set_active_ai_provider(
    state: tauri::State<'_, AiIntegration>,
    id: String,
) -> Result<(), AppError> {
    state.set_active(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn test_ai_connection(
    state: tauri::State<'_, AiIntegration>,
    id: String,
) -> Result<u64, AppError> {
    state.test_connection(&id).await
}
