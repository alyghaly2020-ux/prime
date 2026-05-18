use std::fs;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use tauri::ipc::InvokeError;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileContent {
    pub content: String,
    pub language: String,
    pub modified: String,
}

fn detect_language(path: &str) -> String {
    let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
    match ext {
        "rs" => "rust".into(),
        "ts" | "tsx" => "typescript".into(),
        "js" | "jsx" => "javascript".into(),
        "py" => "python".into(),
        "go" => "go".into(),
        "java" => "java".into(),
        "cpp" | "cc" | "cxx" => "cpp".into(),
        "c" | "h" => "c".into(),
        "json" => "json".into(),
        "yaml" | "yml" => "yaml".into(),
        "toml" => "toml".into(),
        "md" | "markdown" => "markdown".into(),
        "html" => "html".into(),
        "css" => "css".into(),
        "scss" | "sass" => "scss".into(),
        "sh" | "bash" => "shell".into(),
        "sql" => "sql".into(),
        _ => "plaintext".into(),
    }
}

#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<FileEntry>, InvokeError> {
    let dir = Path::new(&path);
    if !dir.exists() {
        return Err(InvokeError::from(format!("Directory not found: {}", path)));
    }
    if !dir.is_dir() {
        return Err(InvokeError::from(format!("Not a directory: {}", path)));
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    let mut dirs: Vec<FileEntry> = Vec::new();

    for entry in fs::read_dir(dir).map_err(|e| InvokeError::from(e.to_string()))? {
        let entry = entry.map_err(|e| InvokeError::from(e.to_string()))?;
        let ft = entry.file_type().map_err(|e| InvokeError::from(e.to_string()))?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') { continue; }
        let full_path = entry.path().to_string_lossy().to_string();
        let md = entry.metadata().map_err(|e| InvokeError::from(e.to_string()))?;
        let modified = md.modified()
            .map(|t| {
                let d: chrono::DateTime<chrono::Local> = t.into();
                d.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_default();

        let fe = FileEntry {
            name: name.clone(),
            path: full_path.clone(),
            is_dir: ft.is_dir(),
            size: if ft.is_file() { md.len() } else { 0 },
            modified,
        };
        if ft.is_dir() {
            dirs.push(fe);
        } else {
            entries.push(fe);
        }
    }
    dirs.sort_by(|a, b| a.name.cmp(&b.name));
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    dirs.extend(entries);
    Ok(dirs)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<FileContent, InvokeError> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err(InvokeError::from(format!("File not found: {}", path)));
    }
    let content = fs::read_to_string(p).map_err(|e| InvokeError::from(e.to_string()))?;
    let md = p.metadata().map_err(|e| InvokeError::from(e.to_string()))?;
    let modified = md.modified()
        .map(|t| {
            let d: chrono::DateTime<chrono::Local> = t.into();
            d.format("%Y-%m-%d %H:%M:%S").to_string()
        })
        .unwrap_or_default();

    Ok(FileContent {
        content,
        language: detect_language(&path),
        modified,
    })
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), InvokeError> {
    let p = Path::new(&path);
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| InvokeError::from(e.to_string()))?;
    }
    fs::write(p, &content).map_err(|e| InvokeError::from(e.to_string()))
}

#[tauri::command]
pub fn create_dir(path: String) -> Result<(), InvokeError> {
    fs::create_dir_all(&path).map_err(|e| InvokeError::from(e.to_string()))
}

#[tauri::command]
pub fn delete_file(path: String) -> Result<(), InvokeError> {
    let p = Path::new(&path);
    if p.is_dir() {
        fs::remove_dir_all(p).map_err(|e| InvokeError::from(e.to_string()))
    } else {
        fs::remove_file(p).map_err(|e| InvokeError::from(e.to_string()))
    }
}

#[tauri::command]
pub fn rename_file(old_path: String, new_path: String) -> Result<(), InvokeError> {
    fs::rename(&old_path, &new_path).map_err(|e| InvokeError::from(e.to_string()))
}

#[tauri::command]
pub fn search_files(path: String, query: String) -> Result<Vec<FileEntry>, InvokeError> {
    let root = PathBuf::from(&path);
    if !root.exists() {
        return Err(InvokeError::from(format!("Path not found: {}", path)));
    }

    let mut results = Vec::new();
    let query_lower = query.to_lowercase();

    fn walk(dir: &Path, query: &str, results: &mut Vec<FileEntry>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') { continue; }
            let full_path = entry.path();
            let ft = entry.file_type().map_err(|e| e.to_string())?;

            if name.to_lowercase().contains(query) {
                let md = entry.metadata().map_err(|e| e.to_string())?;
                let modified = md.modified()
                    .map(|t| {
                        let d: chrono::DateTime<chrono::Local> = t.into();
                        d.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_default();
                results.push(FileEntry {
                    name,
                    path: full_path.to_string_lossy().to_string(),
                    is_dir: ft.is_dir(),
                    size: if ft.is_file() { md.len() } else { 0 },
                    modified,
                });
            }

            if ft.is_dir() {
                walk(&full_path, query, results)?;
            }
        }
        Ok(())
    }

    walk(&root, &query_lower, &mut results).map_err(InvokeError::from)?;
    results.truncate(200);
    Ok(results)
}
