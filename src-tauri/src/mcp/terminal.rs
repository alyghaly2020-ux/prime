use super::McpServer;
use crate::execution::CommandValidator;
use async_trait::async_trait;
use std::sync::Arc;
use tokio::process::Command;

pub struct TerminalMcp {
    id: String,
    validator: Arc<CommandValidator>,
}

impl Default for TerminalMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalMcp {
    pub fn new() -> Self {
        Self {
            id: "terminal".to_string(),
            validator: Arc::new(CommandValidator::new()),
        }
    }
}

#[async_trait]
impl McpServer for TerminalMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Terminal MCP"
    }
    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Terminal MCP ready");
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
            "execute" => {
                let cmd = params["command"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing command"))?;

                // Validate against whitelist before execution
                self.validator.verify_cmd(cmd)
                    .map_err(|e| anyhow::anyhow!("Command rejected: {}", e))?;

                let shell = if cfg!(target_os = "windows") {
                    "pwsh"
                } else {
                    "sh"
                };
                let flag = if cfg!(target_os = "windows") {
                    "-Command"
                } else {
                    "-c"
                };

                // Sanitize to prevent shell injection
                let safe = self.validator.sanitize_script(cmd);
                let output = Command::new(shell).args([flag, &safe]).output().await?;

                Ok(serde_json::json!({
                    "stdout": String::from_utf8_lossy(&output.stdout),
                    "stderr": String::from_utf8_lossy(&output.stderr),
                    "exit_code": output.status.code(),
                }))
            }
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
