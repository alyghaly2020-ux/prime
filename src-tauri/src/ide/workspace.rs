use std::fs;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use tauri::ipc::InvokeError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub path: String,
    pub last_opened: String,
}

fn workspaces_file() -> PathBuf {
    let mut p = dirs_next::config_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("prime");
    p.push("workspaces.json");
    p
}

fn load_workspaces() -> Vec<Workspace> {
    let path = workspaces_file();
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_workspaces(workspaces: &[Workspace]) {
    let path = workspaces_file();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(s) = serde_json::to_string_pretty(workspaces) {
        let _ = fs::write(&path, &s);
    }
}

#[tauri::command]
pub fn list_workspaces() -> Vec<Workspace> {
    let mut list = load_workspaces();
    
    // Clean up non-existent directories to ensure robust state
    list.retain(|w| std::path::Path::new(&w.path).exists());

    if let Ok(cwd) = std::env::current_dir() {
        let cwd_str = cwd.to_string_lossy().to_string();
        if !list.iter().any(|w| w.path == cwd_str) {
            let name = cwd.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Prime Workspace".to_string());
            let id = "default-workspace".to_string();
            let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
            let ws = Workspace { id, name, path: cwd_str, last_opened: now };
            list.push(ws);
        }
    }
    
    save_workspaces(&list);
    list
}

#[tauri::command]
pub fn add_workspace(name: String, path: String) -> Result<Workspace, InvokeError> {
    let p = PathBuf::from(&path);
    if !p.exists() {
        return Err(InvokeError::from(format!("Path not found: {}", path)));
    }
    let id = uuid::Uuid::new_v4().to_string();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let ws = Workspace { id, name, path, last_opened: now };
    let mut list = load_workspaces();
    list.push(ws.clone());
    save_workspaces(&list);
    Ok(ws)
}

#[tauri::command]
pub fn remove_workspace(id: String) -> Result<(), InvokeError> {
    let mut list = load_workspaces();
    list.retain(|w| w.id != id);
    save_workspaces(&list);
    Ok(())
}

#[tauri::command]
pub fn open_workspace(id: String) -> Result<Workspace, InvokeError> {
    let mut list = load_workspaces();
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
    if let Some(ws) = list.iter_mut().find(|w| w.id == id) {
        ws.last_opened = now.clone();
        let result = ws.clone();
        save_workspaces(&list);
        Ok(result)
    } else {
        Err(InvokeError::from("Workspace not found"))
    }
}
