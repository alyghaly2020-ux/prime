use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::AppError;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpSkill {
    pub id: String,
    pub name: String,
    pub skill_type: String,
    pub endpoint: String,
    pub installed: bool,
    pub enabled: bool,
}

#[allow(dead_code)]
pub struct McpSkillsRegistry {
    skills: RwLock<HashMap<String, McpSkill>>,
    active_skills: RwLock<Vec<String>>,
}

#[allow(dead_code)]
impl McpSkillsRegistry {
    pub fn new() -> Self {
        let mut skills = HashMap::new();
        for (id, name, sk_type, ep) in [
            ("web-search", "Web Search", "search", "mcp://web-search"),
            ("code-exec", "Code Execution", "execution", "mcp://code-exec"),
            ("file-system", "File System", "filesystem", "mcp://file-system"),
            ("browser", "Browser Automation", "browser", "mcp://browser"),
            ("memory", "Memory Store", "memory", "mcp://memory"),
            ("image-gen", "Image Generation", "creative", "mcp://image-gen"),
        ] {
            skills.insert(
                id.to_string(),
                McpSkill {
                    id: id.to_string(),
                    name: name.to_string(),
                    skill_type: sk_type.to_string(),
                    endpoint: ep.to_string(),
                    installed: false,
                    enabled: false,
                },
            );
        }
        Self {
            skills: RwLock::new(skills),
            active_skills: RwLock::new(Vec::new()),
        }
    }

    pub async fn list(&self) -> Vec<McpSkill> {
        self.skills.read().await.values().cloned().collect()
    }

    pub async fn install(&self, id: &str) -> Result<(), AppError> {
        let mut skills = self.skills.write().await;
        let skill = skills.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown skill: {id}"))
        })?;
        skill.installed = true;
        Ok(())
    }

    pub async fn uninstall(&self, id: &str) -> Result<(), AppError> {
        let mut skills = self.skills.write().await;
        let skill = skills.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown skill: {id}"))
        })?;
        skill.installed = false;
        skill.enabled = false;
        let mut active = self.active_skills.write().await;
        active.retain(|s| s != id);
        Ok(())
    }

    pub async fn enable(&self, id: &str) -> Result<(), AppError> {
        let mut skills = self.skills.write().await;
        let skill = skills.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown skill: {id}"))
        })?;
        if !skill.installed {
            return Err(AppError::Workspace(format!("Skill '{id}' not installed")));
        }
        skill.enabled = true;
        let mut active = self.active_skills.write().await;
        if !active.contains(&id.to_string()) {
            active.push(id.to_string());
        }
        Ok(())
    }

    pub async fn disable(&self, id: &str) -> Result<(), AppError> {
        let mut skills = self.skills.write().await;
        let skill = skills.get_mut(id).ok_or_else(|| {
            AppError::Workspace(format!("Unknown skill: {id}"))
        })?;
        skill.enabled = false;
        let mut active = self.active_skills.write().await;
        active.retain(|s| s != id);
        Ok(())
    }
}

impl Default for McpSkillsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[tauri::command]
#[allow(dead_code)]
pub async fn list_mcp_skills(
    state: tauri::State<'_, McpSkillsRegistry>,
) -> Result<Vec<McpSkill>, AppError> {
    Ok(state.list().await)
}

#[tauri::command]
#[allow(dead_code)]
pub async fn install_mcp_skill(
    state: tauri::State<'_, McpSkillsRegistry>,
    id: String,
) -> Result<(), AppError> {
    state.install(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn uninstall_mcp_skill(
    state: tauri::State<'_, McpSkillsRegistry>,
    id: String,
) -> Result<(), AppError> {
    state.uninstall(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn enable_mcp_skill(
    state: tauri::State<'_, McpSkillsRegistry>,
    id: String,
) -> Result<(), AppError> {
    state.enable(&id).await
}

#[tauri::command]
#[allow(dead_code)]
pub async fn disable_mcp_skill(
    state: tauri::State<'_, McpSkillsRegistry>,
    id: String,
) -> Result<(), AppError> {
    state.disable(&id).await
}
