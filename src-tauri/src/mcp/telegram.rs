use super::McpServer;
use async_trait::async_trait;
use std::collections::HashMap;

/// Telegram MCP server — exposes Telegram Bot API methods via the MCP interface.
///
/// Reads `TELEGRAM_BOT_TOKEN` from the environment for authentication. All API
/// calls are forwarded to `https://api.telegram.org/bot{token}/...` using a
/// shared `reqwest::Client` with a 30-second timeout.
///
/// # Environment Variables
///
/// | Variable | Required | Default | Description |
/// |---|---|---|---|
/// | `TELEGRAM_BOT_TOKEN` | Yes | — | Bot token from [@BotFather](https://t.me/BotFather) |
/// | `TELEGRAM_SESSION` | No | `data/telegram_session.session` | Session file path (reserved for future MTProto use) |
///
/// # Supported Methods
///
/// | Method | Params | Description |
/// |---|---|---|
/// | `send_message` | `chat_id`, `text`, `parse_mode?`, `disable_web_page_preview?` | Send a text message |
/// | `get_updates` | `offset?`, `limit?`, `timeout?` | Poll for incoming updates (long polling) |
/// | `get_me` | — | Get bot identity information |
/// | `set_webhook` | `url`, `allowed_updates?` | Set a webhook URL for updates |
/// | `delete_webhook` | `drop_pending_updates?` | Remove the webhook |
/// | `get_chat` | `chat_id` | Get chat metadata |
///
/// All methods return `Err(...)` when `TELEGRAM_BOT_TOKEN` is not configured.
#[derive(Debug)]
pub struct TelegramMcp {
    /// Telegram Bot API token, loaded from `TELEGRAM_BOT_TOKEN` env var.
    bot_token: Option<String>,
    /// Path to the local session file (for future MTProto support).
    session_file: String,
    /// Shared HTTP client with a 30-second request timeout.
    client: reqwest::Client,
}

impl Default for TelegramMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl TelegramMcp {
    /// Creates a new `TelegramMcp` server from environment variables.
    ///
    /// Reads configuration from environment variables:
    /// - `TELEGRAM_BOT_TOKEN` — bot token (optional; methods will fail without it)
    /// - `TELEGRAM_SESSION` — session file path (defaults to `data/telegram_session.session`)
    pub fn new() -> Self {
        Self::with_config(&HashMap::new())
    }

    /// Creates a `TelegramMcp` server from config fields, with env var fallback.
    ///
    /// Config fields (from `UserConfig.connection_configs["telegram_bot"].fields`):
    /// - `bot_token` — Telegram Bot API token
    /// - `session_file` — path to session file
    ///
    /// Falls back to `TELEGRAM_BOT_TOKEN` / `TELEGRAM_SESSION` env vars if fields are empty.
    pub fn with_config(fields: &HashMap<String, String>) -> Self {
        let bot_token = fields
            .get("bot_token")
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| std::env::var("TELEGRAM_BOT_TOKEN").ok());

        let session_file = fields
            .get("session_file")
            .filter(|v| !v.is_empty())
            .cloned()
            .or_else(|| std::env::var("TELEGRAM_SESSION").ok())
            .unwrap_or_else(|| "data/telegram_session.session".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest::Client");

        Self {
            bot_token,
            session_file,
            client,
        }
    }

    /// Builds the base URL for Bot API calls: `https://api.telegram.org/bot{token}/`
    fn api_url(&self) -> anyhow::Result<String> {
        let token = self
            .bot_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("TELEGRAM_BOT_TOKEN not set"))?;
        Ok(format!("https://api.telegram.org/bot{}/", token))
    }

    /// Sends a POST request to the given Bot API method with the provided JSON body.
    async fn post(&self, method: &str, body: serde_json::Value) -> anyhow::Result<serde_json::Value> {
        let base = self.api_url()?;
        let url = format!("{}{}", base, method);

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Telegram API request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<no response body>".to_string());

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Telegram API returned {}: {}",
                status.as_u16(),
                text
            ));
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse Telegram API response: {}", e))?;

        Ok(parsed)
    }

    // -------------------------------------------------------------------------
    // Handlers for each method
    // -------------------------------------------------------------------------

    /// Handles `send_message` — sends a text message to a given chat.
    ///
    /// Required params:
    /// - `chat_id` (string) — Unique identifier for the target chat or username
    ///   of the target channel (in the format `@channelusername`)
    /// - `text` (string) — Text of the message to be sent
    ///
    /// Optional params:
    /// - `parse_mode` (string) — Mode for parsing entities (`MarkdownV2`, `HTML`)
    /// - `disable_web_page_preview` (bool) — Disables link previews for links in
    ///   this message
    async fn handle_send_message(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let chat_id = params["chat_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: chat_id"))?;
        let text = params["text"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: text"))?;

        let mut body = serde_json::json!({
            "chat_id": chat_id,
            "text": text,
        });

        // Forward optional parameters
        if let Some(parse_mode) = params["parse_mode"].as_str() {
            body["parse_mode"] = serde_json::Value::String(parse_mode.to_string());
        }
        if let Some(disable_preview) = params["disable_web_page_preview"].as_bool() {
            body["disable_web_page_preview"] = serde_json::Value::Bool(disable_preview);
        }

        let result = self.post("sendMessage", body).await?;

        let message_id = result["result"]["message_id"].as_i64().unwrap_or(0);
        Ok(serde_json::json!({
            "ok": true,
            "message_id": message_id,
        }))
    }

    /// Handles `get_updates` — polls for incoming updates using long polling.
    ///
    /// Optional params:
    /// - `offset` (int) — Identifier of the first update to be returned
    /// - `limit` (int) — Limits the number of updates to be retrieved (1–100)
    /// - `timeout` (int) — Timeout in seconds for long polling
    async fn handle_get_updates(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let mut body = serde_json::json!({});

        if let Some(offset) = params["offset"].as_i64() {
            body["offset"] = serde_json::Value::Number(offset.into());
        }
        if let Some(limit) = params["limit"].as_i64() {
            body["limit"] = serde_json::Value::Number(limit.into());
        }
        if let Some(timeout) = params["timeout"].as_i64() {
            body["timeout"] = serde_json::Value::Number(timeout.into());
        }

        self.post("getUpdates", body).await
    }

    /// Handles `get_me` — returns the bot's identity information.
    async fn handle_get_me(&self) -> anyhow::Result<serde_json::Value> {
        self.post("getMe", serde_json::json!({})).await
    }

    /// Handles `set_webhook` — sets a webhook URL for receiving updates.
    ///
    /// Required params:
    /// - `url` (string) — HTTPS URL to send updates to
    ///
    /// Optional params:
    /// - `allowed_updates` (array of strings) — List of update types to receive
    async fn handle_set_webhook(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let url = params["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: url"))?;

        let mut body = serde_json::json!({
            "url": url,
        });

        if let Some(allowed) = params["allowed_updates"].as_array() {
            body["allowed_updates"] = serde_json::Value::Array(allowed.clone());
        }

        let result = self.post("setWebhook", body).await?;
        Ok(serde_json::json!({
            "ok": result["ok"].as_bool().unwrap_or(false),
            "description": result["description"].as_str().unwrap_or(""),
        }))
    }

    /// Handles `delete_webhook` — removes the webhook integration.
    async fn handle_delete_webhook(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let mut body = serde_json::json!({});
        if let Some(drop_pending) = params["drop_pending_updates"].as_bool() {
            body["drop_pending_updates"] = serde_json::Value::Bool(drop_pending);
        }
        let result = self.post("deleteWebhook", body).await?;
        Ok(serde_json::json!({
            "ok": result["ok"].as_bool().unwrap_or(false),
            "description": result["description"].as_str().unwrap_or(""),
        }))
    }

    /// Handles `get_chat` — returns information about a chat.
    ///
    /// Required params:
    /// - `chat_id` (string) — Unique identifier for the target chat or username
    async fn handle_get_chat(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let chat_id = params["chat_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: chat_id"))?;

        self.post("getChat", serde_json::json!({ "chat_id": chat_id })).await
    }
}

#[async_trait]
impl McpServer for TelegramMcp {
    fn id(&self) -> &str {
        "telegram"
    }

    fn name(&self) -> &str {
        "Telegram MCP"
    }

    /// Starts the Telegram MCP server.
    ///
    /// Logs whether the bot token and session file are configured.
    async fn start(&self) -> anyhow::Result<()> {
        if self.bot_token.is_some() {
            tracing::info!(
                "Telegram MCP server ready (session: {})",
                self.session_file
            );
        } else {
            tracing::warn!(
                "Telegram MCP server started without TELEGRAM_BOT_TOKEN — methods will return errors"
            );
        }
        Ok(())
    }

    /// Stops the Telegram MCP server. Currently a no-op (stateless).
    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Returns `true` if the bot token is configured (server is operational).
    async fn is_running(&self) -> bool {
        self.bot_token.is_some()
    }

    /// Routes an incoming MCP request to the appropriate Telegram Bot API method.
    ///
    /// # Supported Methods
    ///
    /// See [struct-level documentation](TelegramMcp) for the full list of
    /// supported methods, their required and optional parameters.
    async fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "send_message" => self.handle_send_message(&params).await,
            "get_updates" => self.handle_get_updates(&params).await,
            "get_me" => self.handle_get_me().await,
            "set_webhook" => self.handle_set_webhook(&params).await,
            "delete_webhook" => self.handle_delete_webhook(&params).await,
            "get_chat" => self.handle_get_chat(&params).await,
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
