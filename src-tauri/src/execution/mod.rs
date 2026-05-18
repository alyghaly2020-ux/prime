//! Command execution engine. Supervises processes with timeout/restart, manages PTY terminals, computes diffs/patches, creates checkpoints for rollback, and streams real-time output.

pub mod checkpoint;
pub mod diff;
pub mod output_streamer;
pub mod patch;
pub mod retry;
pub mod rollback;
pub mod supervisor;
pub mod terminal;

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
    pub checkpoint_id: Option<String>,
}

// ---------------------------------------------------------------------------
// CommandValidator – whitelist + shell-escape guard for ALL command execution
// ---------------------------------------------------------------------------

/// Reusable validator that checks commands against a whitelist and sanitizes
/// shell scripts to prevent injection. Used by TerminalSandbox, MCP terminal,
/// and ProcessSupervisor.
pub struct CommandValidator {
    whitelist: Arc<supervisor::CommandWhitelist>,
}

impl Default for CommandValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandValidator {
    pub fn new() -> Self {
        Self {
            whitelist: Arc::new(supervisor::CommandWhitelist::new()),
        }
    }

    /// Check that the base command (first token) is allowed.
    /// Returns Err with a description if the command is rejected.
    pub fn verify_cmd(&self, full_cmd: &str) -> Result<(), String> {
        let trimmed = full_cmd.trim();
        if trimmed.is_empty() {
            return Err("Empty command".into());
        }
        if !self.whitelist.is_allowed(trimmed) {
            return Err(format!("Command rejected by whitelist: '{}'", trimmed));
        }
        Ok(())
    }

    /// Verify a safe (non-shell) command with separate args.
    pub fn verify_safe(&self, cmd: &str, _args: &[&str]) -> Result<(), String> {
        self.verify_cmd(cmd)
    }

    /// Sanitize a shell script string by escaping metacharacters that are
    /// dangerous inside double-quoted shell strings: backticks, $(), ${}.
    /// Semicolons are NOT escaped — they are literal inside double quotes and
    /// required for legitimate shell constructs (for/while loops).
    pub fn sanitize_script(&self, script: &str) -> String {
        let mut out = String::with_capacity(script.len() + 16);
        let mut chars = script.chars().peekable();
        while let Some(ch) = chars.next() {
            match ch {
                '`' => out.push_str("\\`"),
                '$' if chars.peek() == Some(&'(') || chars.peek() == Some(&'{') => {
                    out.push('\\');
                    out.push(ch);
                }
                '\\' => out.push_str("\\\\"),
                _ => out.push(ch),
            }
        }
        out
    }

    /// Validate and sanitize a shell script. Returns the sanitized version
    /// or Err if the first command is not whitelisted.
    pub fn validate_script(&self, script: &str) -> Result<String, String> {
        let first_cmd = script.split_whitespace().next().unwrap_or("");
        if !self.whitelist.is_allowed(first_cmd) {
            return Err(format!(
                "First command '{}' is not in the execution whitelist",
                first_cmd
            ));
        }
        Ok(self.sanitize_script(script))
    }
}

// ---------------------------------------------------------------------------
// ProcessIsolation – sandboxed command execution
// ---------------------------------------------------------------------------

/// Restricts execution of sub-processes to approved working directories,
/// strips sensitive environment variables, enforces a timeout, and caps
/// output size.
pub struct ProcessIsolation {
    allowed_workdirs: Vec<PathBuf>,
    blocked_env_keys: Vec<String>,
    timeout_secs: u64,
    max_output_bytes: usize,
}

impl Default for ProcessIsolation {
    fn default() -> Self {
        Self::new()
    }
}

impl ProcessIsolation {
    pub fn new() -> Self {
        Self {
            allowed_workdirs: Vec::new(),
            blocked_env_keys: vec![
                "AWS_SECRET_ACCESS_KEY".to_string(),
                "AWS_ACCESS_KEY_ID".to_string(),
                "AWS_SESSION_TOKEN".to_string(),
                "GH_TOKEN".to_string(),
                "GITHUB_TOKEN".to_string(),
                "API_KEY".to_string(),
                "API_SECRET".to_string(),
                "DB_PASSWORD".to_string(),
                "DATABASE_URL".to_string(),
                "REDIS_PASSWORD".to_string(),
                "SECRET_KEY".to_string(),
                "PRIVATE_KEY".to_string(),
                "TOKEN".to_string(),
            ],
            timeout_secs: 30,
            max_output_bytes: 10 * 1024 * 1024, // 10 MB
        }
    }

    /// Register an additional allowed working directory.
    pub fn add_allowed_workdir(&mut self, dir: impl Into<PathBuf>) {
        self.allowed_workdirs.push(dir.into());
    }

    /// Set a custom timeout (seconds).
    pub fn set_timeout(&mut self, secs: u64) {
        self.timeout_secs = secs;
    }

    /// Set a custom output size cap (bytes).
    pub fn set_max_output_bytes(&mut self, bytes: usize) {
        self.max_output_bytes = bytes;
    }

    /// Check whether `path` resides inside one of the allowed directories.
    /// When no allowed directories are configured the check passes (open mode).
    pub fn is_path_safe(&self, path: &Path) -> bool {
        if self.allowed_workdirs.is_empty() {
            return true;
        }
        let canonical = std::fs::canonicalize(path).ok();
        let path = canonical.as_deref().unwrap_or(path);
        self.allowed_workdirs.iter().any(|d| path.starts_with(d))
    }

    /// Remove sensitive environment variables from `cmd`.
    pub fn filter_env(&self, cmd: &mut tokio::process::Command) {
        for key in &self.blocked_env_keys {
            cmd.env_remove(key);
        }
    }

    /// Run a command under isolation and return the result.
    pub async fn run_safe(
        &self,
        cmd: &str,
        args: &[&str],
        workdir: Option<&Path>,
    ) -> anyhow::Result<ExecutionResult> {
        let start = std::time::Instant::now();

        // Verify working directory
        if let Some(wd) = workdir {
            if !self.is_path_safe(wd) {
                return Ok(ExecutionResult {
                    success: false,
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: format!(
                        "Access denied: '{}' is not in the allowed working directories",
                        wd.display()
                    ),
                    duration_ms: start.elapsed().as_millis() as u64,
                    checkpoint_id: None,
                });
            }
        }

        let mut command = tokio::process::Command::new(cmd);
        command.args(args);
        if let Some(wd) = workdir {
            command.current_dir(wd);
        }
        self.filter_env(&mut command);

        let result = timeout(Duration::from_secs(self.timeout_secs), command.output()).await;

        let duration = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();

                // Enforce output size cap
                let (stdout, stderr) = if stdout.len() > self.max_output_bytes
                    || stderr.len() > self.max_output_bytes
                {
                    let truncated = "[output truncated]".to_string();
                    (truncated.clone(), truncated)
                } else {
                    (stdout, stderr)
                };

                Ok(ExecutionResult {
                    success: output.status.success(),
                    exit_code: output.status.code().unwrap_or(-1),
                    stdout,
                    stderr,
                    duration_ms: duration,
                    checkpoint_id: None,
                })
            }
            Ok(Err(e)) => Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Isolated execution error: {}", e),
                duration_ms: duration,
                checkpoint_id: None,
            }),
            Err(_) => Ok(ExecutionResult {
                success: false,
                exit_code: -1,
                stdout: String::new(),
                stderr: format!("Timeout after {}s", self.timeout_secs),
                duration_ms: duration,
                checkpoint_id: None,
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// SafeFileOps – guarded file read / write / delete / backup
// ---------------------------------------------------------------------------

/// Safe file operations with path allowlisting, automatic backups, and
/// trash-safe deletion.
pub struct SafeFileOps {
    allowed_paths: Vec<PathBuf>,
    trash_dir: PathBuf,
}

impl Default for SafeFileOps {
    fn default() -> Self {
        Self::new()
    }
}

impl SafeFileOps {
    pub fn new() -> Self {
        let trash_dir = std::env::temp_dir().join("prime_trash");
        // Ensure trash dir exists
        let _ = std::fs::create_dir_all(&trash_dir);
        Self {
            allowed_paths: Vec::new(),
            trash_dir,
        }
    }

    /// Register a path (or parent) that is allowed for file operations.
    pub fn add_allowed_path(&mut self, path: impl Into<PathBuf>) {
        self.allowed_paths.push(path.into());
    }

    /// Check whether `path` is under one of the allowed paths.
    fn is_path_allowed(&self, path: &Path) -> bool {
        if self.allowed_paths.is_empty() {
            // When no whitelist is configured, allow all (opt-in security).
            return true;
        }
        let canonical = std::fs::canonicalize(path).ok();
        let path = canonical.as_deref().unwrap_or(path);
        self.allowed_paths.iter().any(|a| path.starts_with(a))
    }

    /// Safely read a file – checks allowlist first.
    pub async fn read_safe(&self, path: &Path) -> anyhow::Result<String> {
        if !self.is_path_allowed(path) {
            anyhow::bail!(
                "Read denied: '{}' is not in the allowed paths",
                path.display()
            );
        }
        Ok(std::fs::read_to_string(path)?)
    }

    /// Safely write content to a file – checks allowlist and creates a backup
    /// (.bak) before overwriting.
    pub async fn write_safe(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        if !self.is_path_allowed(path) {
            anyhow::bail!(
                "Write denied: '{}' is not in the allowed paths",
                path.display()
            );
        }
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Create backup before overwriting
        self.backup_before_write(path, content).await?;
        Ok(())
    }

    /// Move a file to the trash directory instead of permanently deleting it.
    pub async fn delete_safe(&self, path: &Path) -> anyhow::Result<()> {
        if !path.exists() {
            anyhow::bail!("File not found: '{}'", path.display());
        }
        if !self.is_path_allowed(path) {
            anyhow::bail!(
                "Delete denied: '{}' is not in the allowed paths",
                path.display()
            );
        }
        let trash_path = self.trash_dir.join(format!(
            "{}_{}",
            chrono::Utc::now().timestamp(),
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        ));
        std::fs::rename(path, &trash_path)?;
        tracing::info!(
            "Moved '{}' to trash: {}",
            path.display(),
            trash_path.display()
        );
        Ok(())
    }

    /// Create a `.bak` copy of `path` (if it already exists) and then write
    /// `content` to it.
    pub async fn backup_before_write(&self, path: &Path, content: &str) -> anyhow::Result<()> {
        if path.exists() {
            let backup_path = path.with_extension("bak");
            std::fs::copy(path, &backup_path)?;
            tracing::info!(
                "Backed up '{}' to '{}'",
                path.display(),
                backup_path.display()
            );
        }
        std::fs::write(path, content)?;
        Ok(())
    }
}

pub struct Engine {
    pub terminal: Arc<terminal::TerminalSandbox>,
    pub supervisor: Arc<supervisor::ProcessSupervisor>,
    pub patch: Arc<patch::PatchEngine>,
    pub diff: Arc<diff::DiffEngine>,
    pub rollback: Arc<rollback::RollbackSystem>,
    pub retry: Arc<retry::RetryLoop>,
    pub checkpoint: Arc<checkpoint::CheckpointSystem>,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            terminal: Arc::new(terminal::TerminalSandbox::new()),
            supervisor: Arc::new(supervisor::ProcessSupervisor::new()),
            patch: Arc::new(patch::PatchEngine::new()),
            diff: Arc::new(diff::DiffEngine::new()),
            rollback: Arc::new(rollback::RollbackSystem::new()),
            retry: Arc::new(retry::RetryLoop::new()),
            checkpoint: Arc::new(checkpoint::CheckpointSystem::new()),
        }
    }

    pub async fn execute(&self, code: &str, language: &str) -> anyhow::Result<ExecutionResult> {
        // 1. Create checkpoint
        let cp_id = self.checkpoint.create().await?;

        // 2. Execute with retry
        let result = self
            .retry
            .execute(|| async {
                let _checkpoint = checkpoint::RestorePoint::new();
                self.supervisor.run(code, language).await
            })
            .await?;

        // 3. Create diff
        let diff = self.diff.compute(code, "").await;

        // 4. Register rollback point
        self.rollback.register(cp_id.clone(), diff).await;

        Ok(ExecutionResult {
            checkpoint_id: Some(cp_id),
            ..result
        })
    }

    pub async fn rollback_last(&self) -> anyhow::Result<()> {
        self.rollback.undo_last().await.map(|_| ())
    }

    pub async fn apply_patch(&self, original: &str, patch_str: &str) -> anyhow::Result<String> {
        self.patch.apply(original, patch_str).await
    }
}
