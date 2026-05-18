use super::McpServer;
use async_trait::async_trait;

#[derive(Debug)]
pub struct OsMcp {
    id: String,
}

impl Default for OsMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl OsMcp {
    pub fn new() -> Self {
        Self {
            id: "os".to_string(),
        }
    }
}

#[async_trait]
impl McpServer for OsMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "OS MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("OS MCP server ready");
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
            "info" => Ok(serde_json::json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "family": std::env::consts::FAMILY,
                "hostname": hostname::get().ok().map(|h| h.to_string_lossy().to_string()),
                "cpus": num_cpus::get(),
                "memory": "query system for memory",
            })),

            "env" => {
                let key = params["key"].as_str().unwrap_or("");
                let val = std::env::var(key).ok();
                Ok(serde_json::json!({ "key": key, "value": val }))
            }

            "platform" => Ok(serde_json::json!({
                "os": std::env::consts::OS,
                "arch": std::env::consts::ARCH,
                "family": std::env::consts::FAMILY,
                "exe_suffix": std::env::consts::EXE_SUFFIX,
                "hostname": hostname::get().ok().map(|h| h.to_string_lossy().to_string()),
                "cpus": num_cpus::get(),
                "username": std::env::var("USER")
                    .or_else(|_| std::env::var("USERNAME"))
                    .ok(),
                "home_dir": dirs_next::home_dir().map(|p| p.to_string_lossy().to_string()),
                "data_dir": dirs_next::data_dir().map(|p| p.to_string_lossy().to_string()),
                "current_dir": std::env::current_dir().ok().map(|p| p.to_string_lossy().to_string()),
                "temp_dir": std::env::temp_dir().to_string_lossy().to_string(),
            })),

            "clipboard" => {
                let action = params["action"].as_str().unwrap_or("get");
                match action {
                    "get" => {
                        let mut clipboard = arboard::Clipboard::new()
                            .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
                        let content = clipboard
                            .get_text()
                            .map_err(|e| anyhow::anyhow!("Clipboard read error: {}", e))?;
                        Ok(serde_json::json!({ "content": content }))
                    }
                    "set" => {
                        let content = params["content"].as_str().ok_or_else(|| {
                            anyhow::anyhow!("Missing content parameter for clipboard set")
                        })?;
                        let mut clipboard = arboard::Clipboard::new()
                            .map_err(|e| anyhow::anyhow!("Clipboard error: {}", e))?;
                        clipboard
                            .set_text(content)
                            .map_err(|e| anyhow::anyhow!("Clipboard write error: {}", e))?;
                        Ok(serde_json::json!({ "success": true }))
                    }
                    _ => Err(anyhow::anyhow!("Unknown clipboard action: {}", action)),
                }
            }

            "notify" => {
                let title = params["title"].as_str().unwrap_or("Prime");
                let body = params["body"].as_str().unwrap_or("");
                let timeout_ms = params["timeout_ms"].as_u64().unwrap_or(5000);

                notify_rust::Notification::new()
                    .summary(title)
                    .body(body)
                    .appname("Prime")
                    .timeout(notify_rust::Timeout::Milliseconds(timeout_ms as u32))
                    .show()
                    .map_err(|e| anyhow::anyhow!("Notification error: {}", e))?;

                Ok(serde_json::json!({ "sent": true }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
