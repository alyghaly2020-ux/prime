# Connections — Integration Guide

Prime supports 10 connection types, each configurable via Settings → Connections. This guide covers the connection model, config wiring, and MCP server integration.

## Connection Model

All connections are stored in `UserConfig.connection_configs` as a `HashMap<String, ConnectionConfig>`:

```rust
pub struct ConnectionConfig {
    pub enabled: bool,
    pub label: String,
    pub fields: HashMap<String, String>,  // provider-specific key-value pairs
}
```

## Connection Types

| ID | Label | MCP Server | Env Fallbacks | Config Fields |
|----|-------|------------|---------------|---------------|
| `telegram` | Telegram (User API) | — | — | `api_id`, `api_hash`, `phone` |
| `telegram_bot` | Telegram Bot | `TelegramMcp` | `TELEGRAM_BOT_TOKEN`, `TELEGRAM_SESSION` | `bot_token`, `session_file` |
| `whatsapp` | WhatsApp | `WhatsAppMcp` | `WHATSAPP_API_KEY`, `WHATSAPP_API_URL`, `WHATSAPP_PHONE_NUMBER_ID` | `api_key`, `api_url`, `phone_number_id` |
| `discord` | Discord | `DiscordMcp` | — | `bot_token`, `client_id` |
| `slack` | Slack | — | — | `bot_token`, `signing_secret` |
| `email` | Email | — | — | `smtp_host`, `smtp_port`, `username`, `password` |
| `wechat` | WeChat | — | — | `app_id`, `app_secret` |
| `signal` | Signal | — | — | `phone_number`, `signal_url` |
| `matrix` | Matrix | — | — | `homeserver_url`, `access_token` |
| `irc` | IRC | — | — | `server`, `port`, `nickname`, `channel` |

## Config Wired MCP Servers

### Telegram Bot (`TelegramMcp`)

Constructor: `TelegramMcp::with_config(&fields)` or `TelegramMcp::new()` (env fallback)

Config fields → env fallback:
- `bot_token` → `TELEGRAM_BOT_TOKEN`
- `session_file` → `TELEGRAM_SESSION` → `data/telegram_session.session`

Registration in `lib.rs`:

```rust
let telegram_fields = load_config_inner()
    .ok()
    .and_then(|c| c.connection_configs.get("telegram_bot").cloned())
    .map(|cc| cc.fields)
    .unwrap_or_default();
mc.register(Arc::new(TelegramMcp::with_config(&telegram_fields))).await;
```

### WhatsApp (`WhatsAppMcp`)

Constructor: `WhatsAppMcp::with_config(&fields)` or `WhatsAppMcp::new()` (env fallback)

Config fields → env fallback:
- `api_key` → `WHATSAPP_API_KEY`
- `api_url` → `WHATSAPP_API_URL` → `https://graph.facebook.com/v18.0`
- `phone_number_id` → `WHATSAPP_PHONE_NUMBER_ID`

## Adding a New Connection Type

1. Add ID to `ALL_CONNECTIONS` in `src-tauri/src/lib.rs`
2. Create config fields mapping in SettingsPanel.tsx
3. Create MCP server implementing `McpServer` trait
4. Add `with_config()` constructor with env fallback
5. Register in `setup()` with config fields loaded before spawn

## Frontend Integration

`ConnectionsPanel.tsx` reads/writes via IPC:
- `invoke("get_config")` → load connection configs
- `invoke("save_connection_config", { id, config_json })` → persist changes
- Connect/disconnect toggles update `enabled` flag

Settings panel at `src/components/SettingsPanel.tsx` renders all 10 connections with field inputs, helper links, and method selectors (e.g., WhatsApp QR Code vs API).
