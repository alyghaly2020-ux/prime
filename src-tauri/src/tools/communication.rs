use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub id: String,
    pub name: String,
    pub platform_type: String,
    pub connected: bool,
    pub last_active: String,
}

#[allow(dead_code)]
pub struct CommunicationHub {
    platforms: RwLock<HashMap<String, PlatformConfig>>,
    active_platforms: RwLock<Vec<String>>,
}

#[allow(dead_code)]
impl CommunicationHub {
    pub fn new() -> Self {
        let mut platforms = HashMap::new();
        platforms.insert(
            "telegram".to_string(),
            PlatformConfig {
                id: "telegram".to_string(),
                name: "Telegram".to_string(),
                platform_type: "messaging".to_string(),
                connected: false,
                last_active: String::new(),
            },
        );
        platforms.insert(
            "discord".to_string(),
            PlatformConfig {
                id: "discord".to_string(),
                name: "Discord".to_string(),
                platform_type: "messaging".to_string(),
                connected: false,
                last_active: String::new(),
            },
        );
        platforms.insert(
            "slack".to_string(),
            PlatformConfig {
                id: "slack".to_string(),
                name: "Slack".to_string(),
                platform_type: "team".to_string(),
                connected: false,
                last_active: String::new(),
            },
        );
        platforms.insert(
            "matrix".to_string(),
            PlatformConfig {
                id: "matrix".to_string(),
                name: "Matrix".to_string(),
                platform_type: "federated".to_string(),
                connected: false,
                last_active: String::new(),
            },
        );
        Self {
            platforms: RwLock::new(platforms),
            active_platforms: RwLock::new(Vec::new()),
        }
    }

    pub async fn list(&self) -> Vec<PlatformConfig> {
        self.platforms.read().await.values().cloned().collect()
    }

    pub async fn connect(&self, id: &str) -> Result<(), AppError> {
        let mut platforms = self.platforms.write().await;
        let platform = platforms.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown platform: {id}"))
        })?;
        platform.connected = true;
        let mut active = self.active_platforms.write().await;
        if !active.contains(&id.to_string()) {
            active.push(id.to_string());
        }
        Ok(())
    }

    pub async fn disconnect(&self, id: &str) -> Result<(), AppError> {
        let mut platforms = self.platforms.write().await;
        let platform = platforms.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown platform: {id}"))
        })?;
        platform.connected = false;
        let mut active = self.active_platforms.write().await;
        active.retain(|p| p != id);
        Ok(())
    }

    pub async fn broadcast(&self, message: String) -> Result<Vec<String>, AppError> {
        let active = self.active_platforms.read().await;
        if active.is_empty() {
            return Err(AppError::Workspace("No active platforms to broadcast to".to_string()));
        }
        Ok(active.iter().map(|p| format!("{p}: {message}")).collect())
    }
}

impl Default for CommunicationHub {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_platforms(
    state: tauri::State<'_, CommunicationHub>,
) -> Result<Vec<PlatformConfig>, AppError> {
    Ok(state.list().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn connect_platform(
    state: tauri::State<'_, CommunicationHub>,
    id: String,
) -> Result<(), AppError> {
    state.connect(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn disconnect_platform(
    state: tauri::State<'_, CommunicationHub>,
    id: String,
) -> Result<(), AppError> {
    state.disconnect(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn broadcast_message(
    state: tauri::State<'_, CommunicationHub>,
    message: String,
) -> Result<Vec<String>, AppError> {
    state.broadcast(message).await
}
