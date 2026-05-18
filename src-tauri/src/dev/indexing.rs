use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use walkdir::WalkDir;

use super::live_reload::LiveReload;

#[derive(Debug)]
pub struct RepoIndexer {
    index_state: RwLock<HashMap<String, IndexSnapshot>>,
    live_reload: Option<Arc<LiveReload>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct IndexSnapshot {
    path: String,
    file_count: usize,
    total_files: usize,
    last_indexed: chrono::DateTime<chrono::Utc>,
    files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexingProgress {
    pub total_files: usize,
    pub indexed: usize,
    pub percentage: f64,
}

impl Default for RepoIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl RepoIndexer {
    pub fn new() -> Self {
        Self {
            index_state: RwLock::new(HashMap::new()),
            live_reload: None,
        }
    }

    /// Set the LiveReload instance for watch functionality.
    pub fn set_live_reload(&mut self, live_reload: Arc<LiveReload>) {
        self.live_reload = Some(live_reload);
    }

    pub async fn index(&self, path: &str) -> anyhow::Result<()> {
        tracing::info!("Indexing repository: {}", path);
        let mut files = Vec::new();

        for entry in WalkDir::new(path)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let ext = entry
                .path()
                .extension()
                .map(|e| e.to_string_lossy().to_string());
            if let Some(ref ext) = ext {
                if matches!(
                    ext.as_str(),
                    "rs" | "py"
                        | "js"
                        | "ts"
                        | "tsx"
                        | "go"
                        | "java"
                        | "c"
                        | "cpp"
                        | "h"
                        | "hpp"
                        | "toml"
                        | "json"
                        | "yaml"
                        | "yml"
                        | "md"
                ) {
                    files.push(entry.path().to_string_lossy().to_string());
                }
            }
        }

        let total_count = files.len();

        let snapshot = IndexSnapshot {
            path: path.to_string(),
            file_count: total_count,
            total_files: total_count,
            last_indexed: chrono::Utc::now(),
            files,
        };

        let file_count = snapshot.file_count;
        self.index_state
            .write()
            .await
            .insert(path.to_string(), snapshot);
        tracing::info!("Indexed {} files in {}", file_count, path);
        Ok(())
    }

    pub async fn get_files(&self, path: &str) -> Option<Vec<String>> {
        self.index_state
            .read()
            .await
            .get(path)
            .map(|s| s.files.clone())
    }

    pub async fn incremental_index(
        &self,
        path: &str,
        changed_files: &[String],
    ) -> anyhow::Result<()> {
        let mut state = self.index_state.write().await;
        if let Some(snapshot) = state.get_mut(path) {
            for file in changed_files {
                if !snapshot.files.contains(file) {
                    snapshot.files.push(file.clone());
                }
            }
            snapshot.file_count = snapshot.files.len();
            snapshot.last_indexed = chrono::Utc::now();
        }
        Ok(())
    }

    /// Watch a directory for changes and auto-reindex changed files.
    pub async fn watch_and_index(&self, path: &str) -> anyhow::Result<()> {
        let watch_path = path.to_string();

        // First, do a full index
        self.index(&watch_path).await?;

        // Set up file watching using the live_reload mechanism
        if let Some(ref lr) = self.live_reload {
            lr.watch_directory(&watch_path).await?;
            tracing::info!("Watching directory for changes: {}", watch_path);
        } else {
            tracing::warn!(
                "LiveReload not set on RepoIndexer — watch_and_index will not auto-reindex"
            );
        }

        Ok(())
    }

    /// Return the current indexing progress for a path.
    pub async fn get_indexing_progress(&self, path: &str) -> IndexingProgress {
        let state = self.index_state.read().await;
        if let Some(snapshot) = state.get(path) {
            let total = snapshot.total_files.max(1);
            IndexingProgress {
                total_files: snapshot.total_files,
                indexed: snapshot.file_count,
                percentage: (snapshot.file_count as f64 / total as f64) * 100.0,
            }
        } else {
            IndexingProgress {
                total_files: 0,
                indexed: 0,
                percentage: 0.0,
            }
        }
    }

    /// Reindex a single changed file.
    pub async fn reindex_file(&self, path: &str, file: &str) -> anyhow::Result<()> {
        // Verify the file still exists
        if !Path::new(file).exists() {
            // File was deleted — remove from index
            let mut state = self.index_state.write().await;
            if let Some(snapshot) = state.get_mut(path) {
                snapshot.files.retain(|f| f != file);
                snapshot.file_count = snapshot.files.len();
                snapshot.last_indexed = chrono::Utc::now();
            }
            tracing::info!("Removed deleted file from index: {}", file);
            return Ok(());
        }

        // Update the file in the index
        let mut state = self.index_state.write().await;
        if let Some(snapshot) = state.get_mut(path) {
            if !snapshot.files.contains(&file.to_string()) {
                snapshot.files.push(file.to_string());
            }
            snapshot.file_count = snapshot.files.len();
            snapshot.last_indexed = chrono::Utc::now();
            tracing::info!("Reindexed file: {}", file);
        }

        Ok(())
    }
}
