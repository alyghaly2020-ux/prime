use super::McpServer;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct BrowserMcp {
    id: String,
    browser: Arc<crate::browser::System>,
}

impl BrowserMcp {
    pub fn new(browser: Arc<crate::browser::System>) -> Self {
        Self {
            id: "browser".to_string(),
            browser,
        }
    }
}

#[async_trait]
impl McpServer for BrowserMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Browser MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Browser MCP server ready");
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
            "navigate" => {
                let url = params["url"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing url parameter"))?;
                let snapshot = self.browser.navigate(url).await?;
                Ok(serde_json::json!({
                    "url": snapshot.url,
                    "title": snapshot.title,
                    "text": snapshot.text,
                }))
            }

            "screenshot" => {
                let snapshot = self.browser.snapshot().await?;
                let screenshot_b64 = snapshot.screenshot.as_ref().map(|bytes| {
                    use base64::Engine;
                    base64::engine::general_purpose::STANDARD.encode(bytes)
                });
                Ok(serde_json::json!({
                    "screenshot": screenshot_b64,
                    "url": snapshot.url,
                    "title": snapshot.title,
                }))
            }

            "evaluate" => {
                let script = params["script"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing script parameter"))?;
                let action = crate::browser::BrowserAction {
                    action_type: "evaluate".to_string(),
                    selector: None,
                    value: Some(script.to_string()),
                    url: None,
                    wait_ms: None,
                };
                let snapshot = self.browser.execute(&action).await?;
                Ok(serde_json::json!({ "result": snapshot.text }))
            }

            "click" => {
                let selector = params["selector"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing selector parameter"))?;
                let action = crate::browser::BrowserAction {
                    action_type: "click".to_string(),
                    selector: Some(selector.to_string()),
                    value: None,
                    url: None,
                    wait_ms: params["wait_ms"].as_u64(),
                };
                let snapshot = self.browser.execute(&action).await?;
                Ok(serde_json::json!({
                    "status": "clicked",
                    "url": snapshot.url,
                    "title": snapshot.title,
                }))
            }

            "snapshot" => {
                let snapshot = self.browser.snapshot().await?;
                Ok(serde_json::json!({
                    "url": snapshot.url,
                    "title": snapshot.title,
                    "a11y_tree": snapshot.a11y_tree,
                    "text": snapshot.text,
                    "html": snapshot.html,
                }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
