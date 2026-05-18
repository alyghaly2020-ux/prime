use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Online,
    Offline,
    Degraded,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedServer {
    pub id: String,
    pub name: String,
    pub host: String,
    pub services: Vec<String>,
    pub status: ServerStatus,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerAlert {
    pub id: String,
    pub server_id: String,
    pub message: String,
    pub severity: String,
    pub timestamp: String,
}

#[allow(dead_code)]
pub struct ServerManager {
    servers: RwLock<HashMap<String, ManagedServer>>,
    alerts: RwLock<Vec<ServerAlert>>,
}

#[allow(dead_code)]
impl ServerManager {
    pub fn new() -> Self {
        let mut servers = HashMap::new();
        servers.insert(
            "srv-web-01".to_string(),
            ManagedServer {
                id: "srv-web-01".to_string(),
                name: "Web Server 01".to_string(),
                host: "192.168.1.10".to_string(),
                services: vec!["nginx".to_string(), "node-app".to_string()],
                status: ServerStatus::Online,
            },
        );
        servers.insert(
            "srv-db-01".to_string(),
            ManagedServer {
                id: "srv-db-01".to_string(),
                name: "Database Primary".to_string(),
                host: "192.168.1.20".to_string(),
                services: vec!["postgresql".to_string(), "redis".to_string()],
                status: ServerStatus::Online,
            },
        );
        servers.insert(
            "srv-cache-01".to_string(),
            ManagedServer {
                id: "srv-cache-01".to_string(),
                name: "Cache Cluster".to_string(),
                host: "192.168.1.30".to_string(),
                services: vec!["redis-cluster".to_string()],
                status: ServerStatus::Online,
            },
        );
        Self {
            servers: RwLock::new(servers),
            alerts: RwLock::new(Vec::new()),
        }
    }

    pub async fn list_servers(&self) -> Vec<ManagedServer> {
        self.servers.read().await.values().cloned().collect()
    }

    pub async fn add_server(&self, server: ManagedServer) {
        let mut servers = self.servers.write().await;
        servers.insert(server.id.clone(), server);
    }

    pub async fn remove_server(&self, id: &str) -> Result<(), AppError> {
        let mut servers = self.servers.write().await;
        servers.remove(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown server: {id}"))
        })?;
        Ok(())
    }

    pub async fn get_alerts(&self) -> Vec<ServerAlert> {
        self.alerts.read().await.clone()
    }
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_servers(
    state: tauri::State<'_, ServerManager>,
) -> Result<Vec<ManagedServer>, AppError> {
    Ok(state.list_servers().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn add_server(
    state: tauri::State<'_, ServerManager>,
    server: String,
) -> Result<(), AppError> {
    let parsed: ManagedServer = serde_json::from_str(&server)
        .map_err(|e| AppError::Workspace(format!("Invalid server JSON: {e}")))?;
    state.add_server(parsed).await;
    Ok(())
}

#[tauri::command]
#[allow(dead_code)]
pub async fn remove_server(
    state: tauri::State<'_, ServerManager>,
    id: String,
) -> Result<(), AppError> {
    state.remove_server(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn get_server_alerts(
    state: tauri::State<'_, ServerManager>,
) -> Result<Vec<ServerAlert>, AppError> {
    Ok(state.get_alerts().await)
}
