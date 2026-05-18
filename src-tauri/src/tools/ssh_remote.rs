use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnection {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub user: String,
    pub status: ConnectionStatus,
}

#[allow(dead_code)]
pub struct SshRemoteManager {
    connections: RwLock<HashMap<String, SshConnection>>,
    active: RwLock<Option<String>>,
}

#[allow(dead_code)]
impl SshRemoteManager {
    pub fn new() -> Self {
        let mut connections = HashMap::new();
        connections.insert(
            "ssh-dev-1".to_string(),
            SshConnection {
                id: "ssh-dev-1".to_string(),
                host: "192.168.1.100".to_string(),
                port: 22,
                user: "admin".to_string(),
                status: ConnectionStatus::Disconnected,
            },
        );
        connections.insert(
            "ssh-prod-1".to_string(),
            SshConnection {
                id: "ssh-prod-1".to_string(),
                host: "10.0.0.50".to_string(),
                port: 2222,
                user: "deploy".to_string(),
                status: ConnectionStatus::Disconnected,
            },
        );
        Self {
            connections: RwLock::new(connections),
            active: RwLock::new(None),
        }
    }

    pub async fn list(&self) -> Vec<SshConnection> {
        self.connections.read().await.values().cloned().collect()
    }

    pub async fn connect(&self, id: &str) -> Result<(), AppError> {
        let mut conns = self.connections.write().await;
        let conn = conns.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown connection: {id}"))
        })?;
        conn.status = ConnectionStatus::Connected;
        *self.active.write().await = Some(id.to_string());
        Ok(())
    }

    pub async fn disconnect(&self, id: &str) -> Result<(), AppError> {
        let mut conns = self.connections.write().await;
        let conn = conns.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown connection: {id}"))
        })?;
        conn.status = ConnectionStatus::Disconnected;
        let mut active = self.active.write().await;
        if active.as_deref() == Some(id) {
            *active = None;
        }
        Ok(())
    }

    pub async fn execute_command(&self, id: &str, command: String) -> Result<String, AppError> {
        let conns = self.connections.read().await;
        let conn = conns.get(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown connection: {id}"))
        })?;
        if conn.status != ConnectionStatus::Connected {
            return Err(AppError::Workspace(format!("Connection '{id}' not active")));
        }
        Ok(format!("[ssh://{}@{}:{}] $ {}", conn.user, conn.host, conn.port, command))
    }

    pub async fn transfer_file(
        &self,
        id: &str,
        source: String,
        dest: String,
    ) -> Result<String, AppError> {
        let conns = self.connections.read().await;
        let conn = conns.get(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown connection: {id}"))
        })?;
        if conn.status != ConnectionStatus::Connected {
            return Err(AppError::Workspace(format!("Connection '{id}' not active")));
        }
        Ok(format!("scp {} {}@{}:{}", source, conn.user, conn.host, dest))
    }
}

impl Default for SshRemoteManager {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_ssh_connections(
    state: tauri::State<'_, SshRemoteManager>,
) -> Result<Vec<SshConnection>, AppError> {
    Ok(state.list().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn ssh_connect(
    state: tauri::State<'_, SshRemoteManager>,
    id: String,
) -> Result<(), AppError> {
    state.connect(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn ssh_disconnect(
    state: tauri::State<'_, SshRemoteManager>,
    id: String,
) -> Result<(), AppError> {
    state.disconnect(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn ssh_execute_command(
    state: tauri::State<'_, SshRemoteManager>,
    id: String,
    command: String,
) -> Result<String, AppError> {
    state.execute_command(&id, command).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn ssh_transfer_file(
    state: tauri::State<'_, SshRemoteManager>,
    id: String,
    source: String,
    dest: String,
) -> Result<String, AppError> {
    state.transfer_file(&id, source, dest).await
}
