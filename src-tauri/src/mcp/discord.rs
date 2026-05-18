use super::McpServer;
use async_trait::async_trait;

/// Discord MCP server — exposes Discord Bot API methods via the MCP interface.
///
/// Reads `DISCORD_BOT_TOKEN` from the environment for authentication. All API
/// calls are forwarded to `https://discord.com/api/v10/...` using a shared
/// `reqwest::Client` with a 30-second timeout.
///
/// # Environment Variables
///
/// | Variable | Required | Default | Description |
/// |---|---|---|---|
/// | `DISCORD_BOT_TOKEN` | Yes | — | Bot token from the [Discord Developer Portal](https://discord.com/developers/applications) |
///
/// # Supported Methods
///
/// | Method | Params | Description |
/// |---|---|---|
/// | `send_message` | `channel_id`, `content` | Send a message to a channel |
/// | `get_channel` | `channel_id` | Get channel information |
/// | `get_guild` | `guild_id` | Get guild (server) information |
/// | `get_guild_channels` | `guild_id` | List channels in a guild |
/// | `get_guild_roles` | `guild_id` | List roles in a guild |
/// | `get_current_user` | — | Get bot user information |
/// | `create_dm` | `recipient_id` | Create a DM channel with a user |
/// | `add_reaction` | `channel_id`, `message_id`, `emoji` | Add a reaction to a message |
///
/// All methods return `Err(...)` when `DISCORD_BOT_TOKEN` is not configured.
#[derive(Debug)]
pub struct DiscordMcp {
    /// Discord Bot API token, loaded from `DISCORD_BOT_TOKEN` env var.
    bot_token: Option<String>,
    /// Shared HTTP client with a 30-second request timeout.
    client: reqwest::Client,
}

impl Default for DiscordMcp {
    fn default() -> Self {
        Self::new()
    }
}

impl DiscordMcp {
    /// Creates a new `DiscordMcp` server.
    ///
    /// Reads `DISCORD_BOT_TOKEN` from the environment (optional at construction;
    /// methods will fail at runtime without it).
    pub fn new() -> Self {
        let bot_token = std::env::var("DISCORD_BOT_TOKEN").ok();

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build reqwest::Client");

        Self { bot_token, client }
    }

    /// Builds the common headers for Discord API requests, including the Bot
    /// authorization header. Returns an error if `DISCORD_BOT_TOKEN` is not set.
    fn auth_header(&self) -> anyhow::Result<reqwest::header::HeaderMap> {
        let token = self
            .bot_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("DISCORD_BOT_TOKEN not set"))?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            reqwest::header::HeaderValue::from_str(&format!("Bot {}", token))
                .map_err(|e| anyhow::anyhow!("Invalid auth header: {}", e))?,
        );
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        Ok(headers)
    }

    /// Sends a GET request to the given Discord API endpoint and returns the
    /// parsed JSON response.
    async fn get(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let headers = self.auth_header()?;
        let url = format!("https://discord.com/api/v10{}", path);

        let resp = self
            .client
            .get(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Discord API request failed: {}", e))?;

        Self::parse_response(resp).await
    }

    /// Sends a POST request to the given Discord API endpoint with the provided
    /// JSON body and returns the parsed JSON response.
    async fn post(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let headers = self.auth_header()?;
        let url = format!("https://discord.com/api/v10{}", path);

        let resp = self
            .client
            .post(&url)
            .headers(headers)
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Discord API request failed: {}", e))?;

        Self::parse_response(resp).await
    }

    /// Sends a PUT request to the given Discord API endpoint and returns the
    /// parsed JSON response.
    async fn put(&self, path: &str) -> anyhow::Result<serde_json::Value> {
        let headers = self.auth_header()?;
        let url = format!("https://discord.com/api/v10{}", path);

        let resp = self
            .client
            .put(&url)
            .headers(headers)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Discord API request failed: {}", e))?;

        Self::parse_response(resp).await
    }

    /// Parses an HTTP response from the Discord API into a JSON value. Returns
    /// an error with the status code and body text on non-success responses.
    async fn parse_response(resp: reqwest::Response) -> anyhow::Result<serde_json::Value> {
        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "<no response body>".to_string());

        if !status.is_success() {
            return Err(anyhow::anyhow!(
                "Discord API returned {}: {}",
                status.as_u16(),
                text
            ));
        }

        let parsed: serde_json::Value = serde_json::from_str(&text)
            .map_err(|e| anyhow::anyhow!("Failed to parse Discord API response: {}", e))?;

        Ok(parsed)
    }

    // -------------------------------------------------------------------------
    // Handlers for each method
    // -------------------------------------------------------------------------

    /// Handles `send_message` — sends a message to a channel.
    ///
    /// Required params:
    /// - `channel_id` (string) — ID of the channel to send the message to
    /// - `content` (string) — Content of the message
    async fn handle_send_message(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let channel_id = params["channel_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: channel_id"))?;
        let content = params["content"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: content"))?;

        let body = serde_json::json!({
            "content": content,
        });

        let path = format!("/channels/{}/messages", channel_id);
        self.post(&path, body).await
    }

    /// Handles `get_channel` — returns channel information.
    ///
    /// Required params:
    /// - `channel_id` (string) — ID of the channel
    async fn handle_get_channel(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let channel_id = params["channel_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: channel_id"))?;

        let path = format!("/channels/{}", channel_id);
        self.get(&path).await
    }

    /// Handles `get_guild` — returns guild (server) information.
    ///
    /// Required params:
    /// - `guild_id` (string) — ID of the guild
    async fn handle_get_guild(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let guild_id = params["guild_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: guild_id"))?;

        let path = format!("/guilds/{}", guild_id);
        self.get(&path).await
    }

    /// Handles `get_guild_channels` — returns a list of channels in a guild.
    ///
    /// Required params:
    /// - `guild_id` (string) — ID of the guild
    async fn handle_get_guild_channels(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let guild_id = params["guild_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: guild_id"))?;

        let path = format!("/guilds/{}/channels", guild_id);
        self.get(&path).await
    }

    /// Handles `get_guild_roles` — returns a list of roles in a guild.
    ///
    /// Required params:
    /// - `guild_id` (string) — ID of the guild
    async fn handle_get_guild_roles(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let guild_id = params["guild_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: guild_id"))?;

        let path = format!("/guilds/{}/roles", guild_id);
        self.get(&path).await
    }

    /// Handles `get_current_user` — returns information about the bot user.
    async fn handle_get_current_user(&self) -> anyhow::Result<serde_json::Value> {
        self.get("/users/@me").await
    }

    /// Handles `create_dm` — creates a DM channel with a recipient.
    ///
    /// Required params:
    /// - `recipient_id` (string) — ID of the user to DM
    async fn handle_create_dm(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let recipient_id = params["recipient_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: recipient_id"))?;

        let body = serde_json::json!({
            "recipient_id": recipient_id,
        });

        self.post("/users/@me/channels", body).await
    }

    /// Handles `add_reaction` — adds a reaction to a message.
    ///
    /// Required params:
    /// - `channel_id` (string) — ID of the channel containing the message
    /// - `message_id` (string) — ID of the message to react to
    /// - `emoji` (string) — Emoji to react with (URL-encoded; for custom emoji
    ///   use `name:id` format)
    async fn handle_add_reaction(
        &self,
        params: &serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        let channel_id = params["channel_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: channel_id"))?;
        let message_id = params["message_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: message_id"))?;
        let emoji = params["emoji"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required param: emoji"))?;

        // URL-encode the emoji to handle unicode emoji and special characters
        let encoded_emoji: String = urlencoding::encode(emoji).into_owned();
        let path = format!(
            "/channels/{}/messages/{}/reactions/{}/@me",
            channel_id, message_id, encoded_emoji
        );

        self.put(&path).await
    }
}

#[async_trait]
impl McpServer for DiscordMcp {
    fn id(&self) -> &str {
        "discord"
    }

    fn name(&self) -> &str {
        "Discord MCP"
    }

    /// Starts the Discord MCP server.
    ///
    /// Logs whether the bot token is configured.
    async fn start(&self) -> anyhow::Result<()> {
        if self.bot_token.is_some() {
            tracing::info!("Discord MCP server ready");
        } else {
            tracing::warn!(
                "Discord MCP server started without DISCORD_BOT_TOKEN — methods will return errors"
            );
        }
        Ok(())
    }

    /// Stops the Discord MCP server. Currently a no-op (stateless).
    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }

    /// Returns `true` if the bot token is configured (server is operational).
    async fn is_running(&self) -> bool {
        self.bot_token.is_some()
    }

    /// Routes an incoming MCP request to the appropriate Discord Bot API method.
    ///
    /// # Supported Methods
    ///
    /// See [struct-level documentation](DiscordMcp) for the full list of
    /// supported methods, their required and optional parameters.
    async fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "send_message" => self.handle_send_message(&params).await,
            "get_channel" => self.handle_get_channel(&params).await,
            "get_guild" => self.handle_get_guild(&params).await,
            "get_guild_channels" => self.handle_get_guild_channels(&params).await,
            "get_guild_roles" => self.handle_get_guild_roles(&params).await,
            "get_current_user" => self.handle_get_current_user().await,
            "create_dm" => self.handle_create_dm(&params).await,
            "add_reaction" => self.handle_add_reaction(&params).await,
            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
