use super::CommandValidator;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::RwLock;

pub struct TerminalSandbox {
    working_dir: RwLock<String>,
    _env_clean: bool,
    validator: Arc<CommandValidator>,
}

impl Default for TerminalSandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalSandbox {
    pub fn new() -> Self {
        Self {
            working_dir: RwLock::new(
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            ),
            _env_clean: true,
            validator: Arc::new(CommandValidator::new()),
        }
    }

    /// Run a single command with separate arguments (no shell injection risk).
    /// Each arg is passed as a separate argv element to the OS.
    pub async fn run_command(
        &self,
        cmd: &str,
        args: &[&str],
    ) -> anyhow::Result<(String, String, i32)> {
        self.validator.verify_safe(cmd, args).map_err(|e| anyhow::anyhow!("{}", e))?;

        let wd = self.working_dir.read().await.clone();
        let output = Command::new(cmd)
            .args(args)
            .current_dir(&wd)
            .output()
            .await?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        Ok((stdout, stderr, exit_code))
    }

    /// Run a script through the system shell with injection protection.
    /// The script is validated against the whitelist and dangerous
    /// metacharacters are sanitized.
    pub async fn run_script(&self, script: &str) -> anyhow::Result<(String, String, i32)> {
        let safe = self.validator.validate_script(script)
            .map_err(|e| anyhow::anyhow!("Script rejected: {}", e))?;

        let wd = self.working_dir.read().await.clone();
        #[cfg(target_os = "windows")]
        let output = Command::new("pwsh")
            .args(["-NoProfile", "-Command", &safe])
            .current_dir(&wd)
            .output()
            .await?;
        #[cfg(not(target_os = "windows"))]
        let output = Command::new("sh")
            .args(["-c", &safe])
            .current_dir(&wd)
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let exit_code = output.status.code().unwrap_or(-1);
        Ok((stdout, stderr, exit_code))
    }

    pub async fn set_working_dir(&self, dir: String) {
        *self.working_dir.write().await = dir;
    }
}
