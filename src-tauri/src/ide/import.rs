use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::ipc::InvokeError;

#[derive(Debug, Serialize, Deserialize)]
pub struct IdeHistory {
    pub source: String,
    pub projects: Vec<IdeProject>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdeProject {
    pub name: String,
    pub path: String,
    pub last_opened: String,
}

fn find_vscode_projects() -> Vec<IdeProject> {
    let mut projects = Vec::new();
    for home in [dirs_next::home_dir().unwrap_or_default()] {
        let storage_paths = vec![
            home.join(".config/Code/storage.json"),
            home.join(".config/Code - OSS/storage.json"),
            home.join("AppData/Roaming/Code/storage.json"),
        ];
        for sp in &storage_paths {
            if let Ok(content) = fs::read_to_string(sp) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(entries) = json.get("openedPathsList").and_then(|o| o.get("entries")) {
                        if let Some(arr) = entries.as_array() {
                            for entry in arr {
                                if let Some(p) = entry.as_str() {
                                    let p = p.trim_start_matches("file://");
                                    let path = PathBuf::from(p);
                                    if path.exists() {
                                        projects.push(IdeProject {
                                            name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                            path: p.to_string(),
                                            last_opened: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        let recent_paths = vec![
            home.join(".config/Code/User/workspaceStorage"),
            home.join(".config/Code - OSS/User/workspaceStorage"),
        ];
        for rp in &recent_paths {
            if rp.exists() {
                if let Ok(entries) = fs::read_dir(rp) {
                    for entry in entries.flatten() {
                        let ws_path = entry.path().join("workspace.json");
                        if let Ok(content) = fs::read_to_string(&ws_path) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                                if let Some(folder) = json.get("folder").and_then(|f| f.as_str()) {
                                    let path = PathBuf::from(folder.trim_start_matches("file://"));
                                    if path.exists() {
                                        projects.push(IdeProject {
                                            name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                            path: folder.to_string(),
                                            last_opened: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    projects.sort_by(|a, b| a.name.cmp(&b.name));
    projects.dedup_by(|a, b| a.path == b.path);
    projects
}

fn find_jetbrains_projects() -> Vec<IdeProject> {
    let mut projects = Vec::new();
    let home = dirs_next::home_dir().unwrap_or_default();
    let config_dirs = vec![
        home.join(".config/JetBrains"),
        home.join("AppData/Roaming/JetBrains"),
    ];
    for config_dir in &config_dirs {
        if !config_dir.exists() { continue; }
        if let Ok(entries) = fs::read_dir(config_dir) {
            for entry in entries.flatten() {
                let recent_path = entry.path().join("options/recentProjects.xml");
                if recent_path.exists() {
                    if let Ok(content) = fs::read_to_string(recent_path) {
                        for line in content.lines() {
                            if line.contains("<entry") && line.contains("recentPath") {
                                if let Some(start) = line.find("value=\"") {
                                    let rest = &line[start + 7..];
                                    if let Some(end) = rest.find('\"') {
                                        let p = &rest[..end];
                                        let path = PathBuf::from(p);
                                        if path.exists() {
                                            projects.push(IdeProject {
                                                name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
                                                path: p.to_string(),
                                                last_opened: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    projects
}

#[tauri::command]
pub fn import_ide_history() -> Result<Vec<IdeHistory>, InvokeError> {
    let mut results = Vec::new();
    let vscode = find_vscode_projects();
    if !vscode.is_empty() {
        results.push(IdeHistory { source: "VS Code".into(), projects: vscode });
    }
    let jetbrains = find_jetbrains_projects();
    if !jetbrains.is_empty() {
        results.push(IdeHistory { source: "JetBrains".into(), projects: jetbrains });
    }
    Ok(results)
}
