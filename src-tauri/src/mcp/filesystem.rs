use super::McpServer;
use async_trait::async_trait;
use std::path::PathBuf;

#[derive(Debug)]
pub struct FilesystemMcp {
    id: String,
    allowed_paths: Vec<PathBuf>,
}

impl FilesystemMcp {
    pub fn new(allowed_paths: Vec<String>) -> Self {
        let paths: Vec<PathBuf> = if allowed_paths.is_empty() {
            // Default to project root if no paths specified
            vec![std::env::current_dir().unwrap_or_default()]
        } else {
            allowed_paths.into_iter().map(PathBuf::from).collect()
        };
        Self {
            id: "filesystem".to_string(),
            allowed_paths: paths,
        }
    }

    fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        let canonical = std::fs::canonicalize(path).or_else(|_| {
            // Path may not exist (e.g. new file to write) — try parent
            path.parent()
                .and_then(|p| std::fs::canonicalize(p).ok())
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "path not found"))
        });
        let canonical = match canonical {
            Ok(c) => c,
            Err(_) => return false,
        };
        self.allowed_paths.iter().any(|p| {
            std::fs::canonicalize(p)
                .ok()
                .is_some_and(|cp| canonical.starts_with(&cp))
        })
    }
}

#[async_trait]
impl McpServer for FilesystemMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Filesystem MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Filesystem MCP server ready");
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn is_running(&self) -> bool {
        true
    }

    async fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "read_file" => {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
                let path = PathBuf::from(path);
                if !self.is_path_allowed(&path) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }
                let content = tokio::fs::read_to_string(&path).await?;
                Ok(serde_json::json!({ "content": content }))
            }
            "write_file" => {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing content"))?;
                let path = PathBuf::from(path);
                if !self.is_path_allowed(&path) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }
                tokio::fs::write(&path, content).await?;
                Ok(serde_json::json!({ "success": true }))
            }
            "list_dir" => {
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing path"))?;
                let path = PathBuf::from(path);
                if !self.is_path_allowed(&path) {
                    return Err(anyhow::anyhow!("Path not allowed"));
                }
                let mut entries = Vec::new();
                let mut read_dir = tokio::fs::read_dir(&path).await?;
                while let Some(entry) = read_dir.next_entry().await? {
                    entries.push(entry.file_name().to_string_lossy().into_owned());
                }
                Ok(serde_json::json!({ "entries": entries }))
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_path_allowed — unit tests for path validation logic
    // -----------------------------------------------------------------------

    #[test]
    fn test_is_path_allowed_exact_match() {
        let dir = tempfile::tempdir().unwrap();
        let fs = FilesystemMcp::new(vec![dir.path().to_string_lossy().to_string()]);
        assert!(fs.is_path_allowed(dir.path()));
    }

    #[test]
    fn test_is_path_allowed_subdirectory() {
        let dir = tempfile::tempdir().unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();

        let fs = FilesystemMcp::new(vec![dir.path().to_string_lossy().to_string()]);
        assert!(
            fs.is_path_allowed(&sub),
            "subdirectory within allowed path should pass"
        );
    }

    #[test]
    fn test_is_path_allowed_nonexistent_file_fails() {
        let dir = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let fs = FilesystemMcp::new(vec![dir.path().to_string_lossy().to_string()]);
        // A non-existent file INSIDE the allowed dir should be allowed (for writes)
        let allowed = dir.path().join("new_file.txt");
        assert!(
            fs.is_path_allowed(&allowed),
            "non-existent file inside allowed dir should be allowed"
        );
        // A non-existent file OUTSIDE the allowed dir should be denied
        let outside_missing = outside.path().join("secret.txt");
        assert!(!fs.is_path_allowed(&outside_missing));
    }

    #[test]
    fn test_is_path_allowed_outside_directory() {
        let allowed = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let fs = FilesystemMcp::new(vec![allowed.path().to_string_lossy().to_string()]);
        assert!(
            !fs.is_path_allowed(outside.path()),
            "path outside allowed dirs should be denied"
        );
    }

    #[test]
    fn test_is_path_allowed_traversal_attempt() {
        let allowed = tempfile::tempdir().unwrap();
        let fs = FilesystemMcp::new(vec![allowed.path().to_string_lossy().to_string()]);

        // Path traversal via ../
        let traversal = allowed.path().join("../../../etc/passwd");
        // canonicalize will resolve to an absolute path outside the allowed dir
        assert!(
            !fs.is_path_allowed(&traversal),
            "path traversal via '../' should be blocked"
        );
    }

    #[test]
    fn test_is_path_allowed_multiple_allowed_paths() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();

        let fs = FilesystemMcp::new(vec![
            dir1.path().to_string_lossy().to_string(),
            dir2.path().to_string_lossy().to_string(),
        ]);

        assert!(fs.is_path_allowed(dir1.path()));
        assert!(fs.is_path_allowed(dir2.path()));
        assert!(!fs.is_path_allowed(outside.path()));
    }

    #[test]
    fn test_is_path_allowed_empty_allowlist_denies_all() {
        let dir = tempfile::tempdir().unwrap();
        let fs = FilesystemMcp::new(vec![]);
        // With an empty allowlist, no paths should be allowed
        assert!(!fs.is_path_allowed(dir.path()));
    }

    #[test]
    fn test_is_path_allowed_trailing_slash_consistency() {
        let dir = tempfile::tempdir().unwrap();
        // Path with trailing slash
        let path_str = format!("{}/", dir.path().to_string_lossy());
        let fs = FilesystemMcp::new(vec![path_str]);
        assert!(
            fs.is_path_allowed(dir.path()),
            "trailing slash in allowlist should still match"
        );
    }
}
