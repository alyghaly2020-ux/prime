use super::diff::DiffResult;
use std::collections::VecDeque;
use tokio::sync::RwLock;

pub struct RollbackSystem {
    history: RwLock<VecDeque<RollbackPoint>>,
    max_history: usize,
}

#[allow(dead_code)]
#[derive(Clone)]
struct RollbackPoint {
    id: String,
    diff: DiffResult,
    timestamp: chrono::DateTime<chrono::Utc>,
    snapshot: Option<Vec<u8>>,
}

impl Default for RollbackSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl RollbackSystem {
    pub fn new() -> Self {
        Self {
            history: RwLock::new(VecDeque::with_capacity(100)),
            max_history: 100,
        }
    }

    pub async fn register(&self, id: String, diff: DiffResult) {
        let point = RollbackPoint {
            id,
            diff,
            timestamp: chrono::Utc::now(),
            snapshot: None,
        };

        let mut history = self.history.write().await;
        if history.len() >= self.max_history {
            history.pop_front();
        }
        history.push_back(point);
    }

    pub async fn undo_last(&self) -> anyhow::Result<Option<DiffResult>> {
        let mut history = self.history.write().await;
        if let Some(point) = history.pop_back() {
            tracing::info!("Rolling back to checkpoint: {}", point.id);
            Ok(Some(point.diff))
        } else {
            Ok(None)
        }
    }

    pub async fn undo_to(&self, id: &str) -> anyhow::Result<Vec<DiffResult>> {
        let mut history = self.history.write().await;
        let mut undone = Vec::new();

        while let Some(point) = history.pop_back() {
            undone.push(point.diff);
            if point.id == id {
                break;
            }
        }

        Ok(undone)
    }

    pub async fn history_len(&self) -> usize {
        self.history.read().await.len()
    }

    pub async fn clear(&self) {
        self.history.write().await.clear();
    }

    /// Restore a file from its `.bak` backup.
    pub async fn rollback_file(&self, path: &str) -> anyhow::Result<()> {
        let path = std::path::Path::new(path);
        let backup_path = path.with_extension("bak");

        if !backup_path.exists() {
            anyhow::bail!("No backup found for '{}'", path.display());
        }

        std::fs::copy(&backup_path, path)?;
        tracing::info!("Rolled back file '{}' from backup", path.display());
        Ok(())
    }

    /// Recursively snapshot a directory.  Returns a snapshot id that can be
    /// passed to `rollback_directory`.
    pub async fn snapshot_directory(&self, dir: &str) -> anyhow::Result<String> {
        let snapshot_id = uuid::Uuid::new_v4().to_string();
        let snapshot_dir = std::env::temp_dir()
            .join("prime_snapshots")
            .join(&snapshot_id);

        std::fs::create_dir_all(&snapshot_dir)?;

        let src = std::path::Path::new(dir);
        if src.exists() {
            Self::copy_dir_recursive(src, &snapshot_dir)?;
        }

        tracing::info!("Snapshot '{}' created for '{}'", snapshot_id, dir);
        Ok(snapshot_id)
    }

    /// Restore a directory from a previously taken snapshot.
    pub async fn rollback_directory(
        &self,
        snapshot_id: &str,
        target_dir: &str,
    ) -> anyhow::Result<()> {
        let snapshot_dir = std::env::temp_dir()
            .join("prime_snapshots")
            .join(snapshot_id);

        if !snapshot_dir.exists() {
            anyhow::bail!("Snapshot '{}' not found at {:?}", snapshot_id, snapshot_dir);
        }

        let target = std::path::Path::new(target_dir);

        // Clear target directory
        if target.exists() {
            std::fs::remove_dir_all(target)?;
        }
        std::fs::create_dir_all(target)?;

        // Copy snapshot contents back
        Self::copy_dir_recursive(&snapshot_dir, target)?;

        tracing::info!(
            "Directory '{}' rolled back from snapshot '{}'",
            target_dir,
            snapshot_id
        );
        Ok(())
    }

    /// Recursively copy a directory tree.
    fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
        use walkdir::WalkDir;

        for entry in WalkDir::new(src) {
            let entry = entry?;
            let relative = entry.path().strip_prefix(src).unwrap_or(entry.path());
            let target = dst.join(relative);

            if entry.file_type().is_dir() {
                std::fs::create_dir_all(target)?;
            } else {
                std::fs::copy(entry.path(), &target)?;
            }
        }
        Ok(())
    }
}
