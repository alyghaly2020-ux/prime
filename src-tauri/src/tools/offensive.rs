use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OffensiveTool {
    pub id: String,
    pub name: String,
    pub category: String,
    pub installed: bool,
    pub running: bool,
}

#[allow(dead_code)]
pub struct OffensiveTools {
    enabled: bool,
    active_tool: RwLock<Option<String>>,
    available_tools: RwLock<HashMap<String, OffensiveTool>>,
}

#[allow(dead_code)]
impl OffensiveTools {
    pub fn new() -> Self {
        let mut tools = HashMap::new();
        for (id, name, cat) in [
            ("hydra", "Hydra", "Credential Stuffing"),
            ("nmap", "Nmap", "Network Scanning"),
            ("metasploit", "Metasploit", "Exploitation"),
            ("sqlmap", "SQLMap", "SQL Injection"),
            ("burpsuite", "Burp Suite", "Web Testing"),
        ] {
            tools.insert(
                id.to_string(),
                OffensiveTool {
                    id: id.to_string(),
                    name: name.to_string(),
                    category: cat.to_string(),
                    installed: false,
                    running: false,
                },
            );
        }
        Self {
            enabled: false,
            active_tool: RwLock::new(None),
            available_tools: RwLock::new(tools),
        }
    }

    pub async fn list(&self) -> Vec<OffensiveTool> {
        self.available_tools.read().await.values().cloned().collect()
    }

    pub async fn launch_tool(&self, id: &str) -> Result<(), AppError> {
        let mut tools = self.available_tools.write().await;
        let tool = tools.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown tool: {id}"))
        })?;
        if !tool.installed {
            return Err(AppError::Workspace(format!("Tool '{id}' not installed")));
        }
        tool.running = true;
        *self.active_tool.write().await = Some(id.to_string());
        Ok(())
    }

    pub async fn shutdown_tool(&self, id: &str) -> Result<(), AppError> {
        let mut tools = self.available_tools.write().await;
        let tool = tools.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown tool: {id}"))
        })?;
        tool.running = false;
        let mut active = self.active_tool.write().await;
        if active.as_deref() == Some(id) {
            *active = None;
        }
        Ok(())
    }
}

impl Default for OffensiveTools {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_offensive_tools(
    state: tauri::State<'_, OffensiveTools>,
) -> Result<Vec<OffensiveTool>, AppError> {
    Ok(state.list().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn launch_offensive_tool(
    state: tauri::State<'_, OffensiveTools>,
    id: String,
) -> Result<(), AppError> {
    state.launch_tool(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn shutdown_offensive_tool(
    state: tauri::State<'_, OffensiveTools>,
    id: String,
) -> Result<(), AppError> {
    state.shutdown_tool(&id).await
}
