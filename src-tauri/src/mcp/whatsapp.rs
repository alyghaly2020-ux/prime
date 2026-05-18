use super::McpServer;
use async_trait::async_trait;
use std::collections::HashMap;

#[derive(Debug)]
pub struct WhatsAppMcp {
    id: String,
    api_key: Option<String>,
    api_url: String,
    phone_number_id: Option<String>,
}

impl Default for WhatsAppMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl WhatsAppMcp {
    pub fn new() -> Self {
        Self::with_config(&HashMap::new())
    }

    /// Creates a `WhatsAppMcp` from config fields, with env var fallback.
    ///
    /// Config fields (from `UserConfig.connection_configs["whatsapp"].fields`):
    /// - `api_key` — WhatsApp API key (Bearer token)
    /// - `api_url` — WhatsApp API base URL (default: `https://graph.facebook.com/v18.0`)
    /// - `phone_number_id` — WhatsApp Business Phone Number ID
    ///
    /// Falls back to `WHATSAPP_API_KEY` / `WHATSAPP_API_URL` / `WHATSAPP_PHONE_NUMBER_ID`
    /// env vars if fields are empty.
    pub fn with_config(fields: &HashMap<String, String>) -> Self {
        let api_key = fields
            .get("api_key")
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| std::env::var("WHATSAPP_API_KEY").ok());

        let api_url = fields
            .get("api_url")
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| std::env::var("WHATSAPP_API_URL").ok())
            .unwrap_or_else(|| "https://graph.facebook.com/v18.0".to_string());

        let phone_number_id = fields
            .get("phone_number_id")
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| std::env::var("WHATSAPP_PHONE_NUMBER_ID").ok());

        Self {
            id: "whatsapp".to_string(),
            api_key,
            api_url,
            phone_number_id,
        }
    }

    fn api_path(&self) -> anyhow::Result<String> {
        let phone_id = self
            .phone_number_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WHATSAPP_PHONE_NUMBER_ID not set"))?;
        Ok(format!("{}/{}/messages", self.api_url, phone_id))
    }

    async fn post(&self, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WHATSAPP_API_KEY not set"))?;
        let url = self.api_path()?;

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("WhatsApp API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "WhatsApp API error ({}): {}",
                status,
                text
            ));
        }

        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse WhatsApp API response: {}", e))
    }

    async fn get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WHATSAPP_API_KEY not set"))?;
        let phone_id = self
            .phone_number_id
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("WHATSAPP_PHONE_NUMBER_ID not set"))?;
        let url = format!("{}/{}/{}", self.api_url, phone_id, path);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("WhatsApp API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<no body>".to_string());

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "WhatsApp API error ({}): {}",
                status,
                text
            ));
        }

        serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse WhatsApp API response: {}", e))
    }
}

#[async_trait]
impl McpServer for WhatsAppMcp {
    fn id(&self) -> &str {
        &self.id
    }

    fn name(&self) -> &str {
        "WhatsApp MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("WhatsApp MCP server ready");
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
            "send_message" => {
                let phone = params["phone"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'phone' parameter"))?;
                let message = params["message"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'message' parameter"))?;

                let body = serde_json::json!({
                    "messaging_product": "whatsapp",
                    "to": phone,
                    "type": "text",
                    "text": { "body": message }
                });

                self.post(body).await.map(|_| serde_json::json!({ "success": true }))
            }

            "mark_read" => {
                let message_id = params["message_id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'message_id' parameter"))?;

                let body = serde_json::json!({
                    "messaging_product": "whatsapp",
                    "status": "read",
                    "message_id": message_id
                });

                self.post(body).await.map(|_| serde_json::json!({ "success": true }))
            }

            "get_templates" => {
                let result = self.get("message_templates").await?;
                Ok(result)
            }

            "send_template" => {
                let phone = params["phone"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'phone' parameter"))?;
                let template_name = params["template_name"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing 'template_name' parameter"))?;
                let language_code = params["language_code"]
                    .as_str()
                    .unwrap_or("en");

                let mut template = serde_json::json!({
                    "name": template_name,
                    "language": { "code": language_code }
                });

                if let Some(body_params) = params["body_params"].as_array() {
                    let components: Vec<serde_json::Value> = body_params
                        .iter()
                        .map(|p| {
                            serde_json::json!({
                                "type": "body",
                                "parameters": [{ "type": "text", "text": p }]
                            })
                        })
                        .collect();
                    template["components"] = serde_json::Value::Array(components);
                }

                let body = serde_json::json!({
                    "messaging_product": "whatsapp",
                    "to": phone,
                    "type": "template",
                    "template": template
                });

                self.post(body).await.map(|_| serde_json::json!({ "success": true }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
