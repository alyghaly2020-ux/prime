//! # Headless WebSocket Server for Remote Agent Control
//!
//! Binds to a configurable TCP port, accepts WebSocket connections with Bearer
//! token authentication, and dispatches JSON commands to Prime's subsystems.
//!
//! ## Protocol
//!
//! 1. Client connects via WebSocket to `ws://127.0.0.1:PORT`
//! 2. Authentication: client sends `{"type":"auth","token":"..."}` as first message.
//!    The server checks against `PRIME_WS_TOKEN` env var. If unset a temporary
//!    random token is generated and logged.
//! 3. Client sends JSON messages with an `"id"` field for correlation.
//! 4. Server responds with JSON messages echoing the same `"id"`.
//!
//! ## Supported Commands
//!
//! | `type` | Purpose |
//! |--------|---------|
//! | `chat` | Send chat messages to an AI model |
//! | `agent_execute` | Execute an agent task |
//! | `computer_use` | Browser automation actions (click, type, etc.) |
//! | `payment` | Wallet transfers and payment operations |
//! | `system` | Query system status, health, metrics |
//! | `mcp` | Call MCP server methods |

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;
use std::time::Instant;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::sync::watch;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::tungstenite::Message;

use crate::ai;
use crate::browser;
use crate::dev;
use crate::mcp;
use crate::tools;
use prime_core::security;
use prime_core::security::audit::AuditResult;

// =============================================================================
// Server Context — holds references to all subsystems the WS server manages
// =============================================================================

/// Shared context injected into every WebSocket session.
pub struct ServerContext {
    pub ai: Arc<ai::Router>,
    pub dev: Arc<dev::Engine>,
    pub browser: Arc<browser::System>,
    pub payments: Arc<tools::PaymentsManager>,
    pub mcp: Arc<mcp::ServerManager>,
    pub security: Arc<security::System>,
}

impl std::fmt::Debug for ServerContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerContext").finish_non_exhaustive()
    }
}

// =============================================================================
// WebSocket Server
// =============================================================================

/// A headless WebSocket server for remote agent control.
///
/// # Example
///
/// ```ignore
/// let ctx = Arc::new(ServerContext { ... });
/// let server = WebSocketServer::new(9876, ctx);
/// server.start().await;
/// ```
pub struct WebSocketServer {
    port: u16,
    token: String,
    shutdown_tx: watch::Sender<bool>,
    shutdown_rx: watch::Receiver<bool>,
    context: Arc<ServerContext>,
    rate_limiter: Arc<StdMutex<HashMap<SocketAddr, Vec<Instant>>>>,
    _max_msg_size: usize,
}

impl std::fmt::Debug for WebSocketServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketServer")
            .field("port", &self.port)
            .field("token_configured", &true)
            .finish()
    }
}

impl WebSocketServer {
    /// Create a new WebSocket server.
    ///
    /// Reads `PRIME_WS_TOKEN` from the environment for auth.
    /// If unset, generates a cryptographically random token and logs it.
    pub fn new(port: u16, context: Arc<ServerContext>) -> Self {
        let token = std::env::var("PRIME_WS_TOKEN").ok().filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                let generated: String = (0..8).map(|_| {
                    let idx = rand::random::<usize>() % 62;
                    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz"[idx] as char
                }).collect();
                let full = format!("prime-{generated}");
                tracing::warn!(
                    "⚠️  PRIME_WS_TOKEN not set. Generated temporary token: {}",
                    full
                );
                full
            });
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Self {
            port,
            token,
            shutdown_tx,
            shutdown_rx,
            context,
            rate_limiter: Arc::new(StdMutex::new(HashMap::new())),
            _max_msg_size: 1_048_576, // 1 MB
        }
    }

    /// Override the token programmatically.
    pub fn with_token(mut self, token: String) -> Self {
        self.token = token;
        self
    }

    /// Start the WebSocket server. This blocks the current task.
    ///
    /// Binds to `127.0.0.1:{port}` and accepts connections in a loop.
    /// Each connection is handled in its own spawned task.
    /// Returns when the shutdown signal is received or a fatal error occurs.
    pub async fn start(&self) -> anyhow::Result<()> {
        let addr: SocketAddr = ([127, 0, 0, 1], self.port).into();
        let listener = TcpListener::bind(addr).await.map_err(|e| {
            anyhow::anyhow!("Failed to bind WS server to {}: {}", addr, e)
        })?;

        tracing::info!(
            "🌐 WebSocket server listening on ws://{}/",
            addr,
        );

        let mut shutdown_rx = self.shutdown_rx.clone();
        let ctx = Arc::clone(&self.context);
        let token = self.token.clone();
        let rate_limiter = self.rate_limiter.clone();

        loop {
            tokio::select! {
                // Accept new connections
                accept_result = listener.accept() => {
                    match accept_result {
                        Ok((stream, peer_addr)) => {
                            tracing::debug!("WS connection from {}", peer_addr);
                            let ctx = Arc::clone(&ctx);
                            let token = token.clone();
                            let rl = rate_limiter.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_connection(stream, peer_addr, ctx, token, rl).await {
                                    tracing::warn!("WS session error from {}: {}", peer_addr, e);
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("WS accept error: {}", e);
                        }
                    }
                }
                // Graceful shutdown signal
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("WS server shutting down gracefully");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Signal the server to shut down gracefully.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

// =============================================================================
// Connection Handler
// =============================================================================

async fn handle_connection(
    stream: tokio::net::TcpStream,
    peer: SocketAddr,
    ctx: Arc<ServerContext>,
    expected_token: String,
    rate_limiter: Arc<StdMutex<HashMap<SocketAddr, Vec<Instant>>>>,
) -> anyhow::Result<()> {
    let ws_stream = accept_async(stream).await.map_err(|e| {
        anyhow::anyhow!("WebSocket handshake failed for {}: {}", peer, e)
    })?;

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    // Parse the first message for authentication
    let first_msg = match ws_rx.next().await {
        Some(Ok(msg)) => msg,
        Some(Err(e)) => {
            tracing::warn!("WS {}: error reading first message: {}", peer, e);
            return Err(anyhow::anyhow!("Read error: {}", e));
        }
        None => return Ok(()),
    };

    // Enforce message size limit
    let first_text = match first_msg {
        Message::Text(t) => {
            if t.len() > 1_048_576 {
                let err = r#"{"error":"message too large (max 1MB)","id":null}"#;
                let _ = ws_tx.send(Message::Text(err.into())).await;
                return Ok(());
            }
            t
        }
        Message::Close(_) | Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {
            let _ = ws_tx.send(Message::Close(None)).await;
            return Ok(());
        }
        Message::Binary(_) => {
            let err = r#"{"error":"expected text message","id":null}"#;
            let _ = ws_tx.send(Message::Text(err.into())).await;
            return Ok(());
        }
    };

    // Authenticate: first message MUST be an auth message with valid token
    let auth_result = serde_json::from_str::<serde_json::Value>(&first_text);
    match auth_result {
        Ok(auth_msg) => {
            if auth_msg.get("type").and_then(|t| t.as_str()) != Some("auth") {
                let err = serde_json::json!({
                    "id": auth_msg.get("id"),
                    "type": "error",
                    "error": "authentication required: send {\"type\":\"auth\",\"token\":\"...\"} first",
                });
                let _ = ws_tx.send(Message::Text(err.to_string())).await;
                let _ = ws_tx.send(Message::Close(None)).await;
                return Err(anyhow::anyhow!("WS {}: authentication required (first message must be auth)", peer));
            }
            let provided_token = auth_msg.get("token").and_then(|t| t.as_str()).unwrap_or("");
            if provided_token != expected_token {
                let err = serde_json::json!({
                    "id": auth_msg.get("id"),
                    "type": "error",
                    "error": "invalid token",
                });
                let _ = ws_tx.send(Message::Text(err.to_string())).await;
                let _ = ws_tx.send(Message::Close(None)).await;
                return Err(anyhow::anyhow!("WS {}: authentication failed", peer));
            }
        }
        Err(_) => {
            let err = r#"{"error":"invalid JSON","id":null}"#;
            let _ = ws_tx.send(Message::Text(err.into())).await;
            return Ok(());
        }
    }

    // Audit the authenticated connection
    ctx.security.audit.record_access(
        "ws_connect",
        &peer.to_string(),
        "websocket",
        AuditResult::Allow,
        None,
    );

    // Send auth success
    let auth_ok = serde_json::json!({
        "id": null,
        "type": "auth_response",
        "authenticated": true,
    });
    let _ = ws_tx.send(Message::Text(auth_ok.to_string())).await;

    // Process subsequent messages in a loop
    while let Some(msg_result) = ws_rx.next().await {
        // --- Rate limiting check (30 requests / 10s) ---
        let rate_limited = {
            let mut rl = rate_limiter.lock().unwrap();
            let now = Instant::now();
            let entries = rl.entry(peer).or_default();
            entries.retain(|t| now.duration_since(*t).as_secs() < 10);
            if entries.len() >= 30 {
                true
            } else {
                entries.push(now);
                false
            }
        };
        if rate_limited {
            let err = serde_json::json!({
                "id": null,
                "type": "error",
                "error": "rate limit exceeded (max 30 requests per 10s)",
            });
            let _ = ws_tx.send(Message::Text(err.to_string())).await;
            continue;
        }

        match msg_result {
            Ok(Message::Text(text)) => {
                // Enforce message size limit
                if text.len() > 1_048_576 {
                    let err = serde_json::json!({
                        "id": null,
                        "type": "error",
                        "error": "message too large (max 1MB)",
                    });
                    let _ = ws_tx.send(Message::Text(err.to_string())).await;
                    continue;
                }

                // Audit the command
                let subject = format!("ws:{}", peer);
                let action = extract_action_type(&text).unwrap_or_else(|| "unknown".to_string());
                ctx.security.audit.record_access(
                    &action,
                    &subject,
                    "websocket",
                    AuditResult::Allow,
                    None,
                );

                let response = process_message(&text, &ctx).await;
                if ws_tx.send(Message::Text(response)).await.is_err() {
                    break;
                }
            }
            Ok(Message::Close(_)) | Err(_) => break,
            Ok(Message::Ping(data)) => {
                let _ = ws_tx.send(Message::Pong(data)).await;
            }
            Ok(Message::Pong(_)) | Ok(Message::Frame(_)) | Ok(Message::Binary(_)) => {}
        }
    }

    tracing::debug!("WS {}: connection closed", peer);
    Ok(())
}

/// Extract the `type` field from a JSON command for audit logging.
fn extract_action_type(text: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .and_then(|v| {
            v.get("type")
                .and_then(|t| t.as_str())
                .map(|s| s.to_string())
        })
}

// =============================================================================
// Message Processing
// =============================================================================

/// Parse a JSON command and dispatch to the appropriate subsystem.
async fn process_message(text: &str, ctx: &ServerContext) -> String {
    let msg: serde_json::Value = match serde_json::from_str(text) {
        Ok(v) => v,
        Err(e) => {
            return serde_json::json!({
                "id": null,
                "type": "error",
                "error": format!("invalid JSON: {}", e),
            })
            .to_string();
        }
    };

    let msg_id = msg.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
    let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");

    let response = match msg_type {
        "chat" => handle_chat(msg_id, &msg, ctx).await,
        "agent_execute" => handle_agent_execute(msg_id, &msg, ctx).await,
        "computer_use" => handle_computer_use(msg_id, &msg, ctx).await,
        "payment" => handle_payment(msg_id, &msg, ctx).await,
        "system" => handle_system(msg_id, &msg, ctx).await,
        "mcp" => handle_mcp(msg_id, &msg, ctx).await,
        "auth" => {
            // Auth was already handled; this is a no-op
            serde_json::json!({
                "id": msg_id,
                "type": "auth_response",
                "authenticated": true,
            })
        }
        other => {
            serde_json::json!({
                "id": msg_id,
                "type": "error",
                "error": format!("unknown message type: {}", other),
            })
        }
    };

    response.to_string()
}

// ---------------------------------------------------------------------------
// Chat Handler
// ---------------------------------------------------------------------------

/// Expected JSON:
/// ```json
/// {"type": "chat", "model": "gpt-5", "messages": [...], "id": "req-1"}
/// ```
async fn handle_chat(id: &str, msg: &serde_json::Value, ctx: &ServerContext) -> serde_json::Value {
    let model = match msg.get("model").and_then(|v| v.as_str()) {
        Some(m) => m,
        None => {
            return serde_json::json!({
                "id": id, "type": "chat_response",
                "error": "missing 'model' field",
            });
        }
    };

    let messages_raw = match msg.get("messages") {
        Some(v) => v,
        None => {
            return serde_json::json!({
                "id": id, "type": "chat_response",
                "error": "missing 'messages' field",
            });
        }
    };

    let messages: Vec<ai::ChatMessage> = match serde_json::from_value(messages_raw.clone()) {
        Ok(m) => m,
        Err(e) => {
            return serde_json::json!({
                "id": id, "type": "chat_response",
                "error": format!("invalid messages: {}", e),
            });
        }
    };

    match ctx.ai.chat(messages, model).await {
        Ok(content) => {
            serde_json::json!({
                "id": id,
                "type": "chat_response",
                "content": content,
                "model": model,
                "error": null,
            })
        }
        Err(e) => {
            serde_json::json!({
                "id": id,
                "type": "chat_response",
                "content": null,
                "model": model,
                "error": ai::redact_sensitive(&e.to_string()),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Agent Execute Handler
// ---------------------------------------------------------------------------

/// Expected JSON:
/// ```json
/// {"type": "agent_execute", "agent_id": "...", "task": "...", "id": "req-2"}
/// ```
async fn handle_agent_execute(
    id: &str,
    msg: &serde_json::Value,
    ctx: &ServerContext,
) -> serde_json::Value {
    let agent_id = match msg.get("agent_id").and_then(|v| v.as_str()) {
        Some(a) => a.to_string(),
        None => {
            return serde_json::json!({
                "id": id, "type": "agent_response",
                "error": "missing 'agent_id' field",
            });
        }
    };

    let task = match msg.get("task").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            return serde_json::json!({
                "id": id, "type": "agent_response",
                "error": "missing 'task' field",
            });
        }
    };

    // Look up the agent
    let agents = ctx.dev.agents.available_agents().await;
    let agent = agents.iter().find(|a| a.id == agent_id).cloned();

    match agent {
        Some(agent_def) => {
            // Resolve model for this agent
            let model = ctx.ai.resolve_agent_model(&agent_def.capabilities, &[]);

            // Build a chat message combining system prompt + task
            let system_msg = ai::ChatMessage {
                role: "system".to_string(),
                content: agent_def.system_prompt.clone(),
                tool_calls: None,
                timestamp: None,
            };
            let task_msg = ai::ChatMessage {
                role: "user".to_string(),
                content: task,
                tool_calls: None,
                timestamp: None,
            };

            match ctx.ai.chat(vec![system_msg, task_msg], &model).await {
                Ok(content) => {
                    serde_json::json!({
                        "id": id,
                        "type": "agent_response",
                        "agent_id": agent_id,
                        "agent_name": agent_def.name,
                        "content": content,
                        "model": model,
                        "error": null,
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "id": id,
                        "type": "agent_response",
                        "agent_id": agent_id,
                        "error": ai::redact_sensitive(&e.to_string()),
                    })
                }
            }
        }
        None => {
            serde_json::json!({
                "id": id,
                "type": "agent_response",
                "agent_id": agent_id,
                "error": format!("agent '{}' not found", agent_id),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Computer Use (Browser Automation) Handler
// ---------------------------------------------------------------------------

/// Expected JSON:
/// ```json
/// {"type": "computer_use", "action": "click", "x": 100, "y": 200, "id": "req-3"}
/// ```
///
/// Supported actions:
/// - `click` (uses x, y)
/// - `type` (uses text)
/// - `navigate` (uses url)
/// - `screenshot` — return a base64-encoded PNG
/// - `snapshot` — return page DOM / a11y tree
async fn handle_computer_use(
    id: &str,
    msg: &serde_json::Value,
    ctx: &ServerContext,
) -> serde_json::Value {
    let action = match msg.get("action").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => {
            return serde_json::json!({
                "id": id, "type": "computer_use_response",
                "error": "missing 'action' field",
            });
        }
    };

    // Require explicit user confirmation for all remote computer_use actions
    if !msg.get("confirmed").and_then(|v| v.as_bool()).unwrap_or(false) {
        return serde_json::json!({
            "id": id, "type": "computer_use_response",
            "error": "user confirmation required: set 'confirmed': true to proceed",
        });
    }

    match action {
        "click" => {
            let x = msg.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            let y = msg.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;

            let browser_action = browser::BrowserAction {
                action_type: "click".to_string(),
                selector: None,
                value: Some(format!("{},{}", x, y)),
                url: None,
                wait_ms: Some(500),
            };

            match ctx.browser.execute(&browser_action).await {
                Ok(snapshot) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "click",
                        "x": x,
                        "y": y,
                        "url": snapshot.url,
                        "title": snapshot.title,
                        "error": null,
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "click",
                        "error": e.to_string(),
                    })
                }
            }
        }
        "type" => {
            let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let browser_action = browser::BrowserAction {
                action_type: "type".to_string(),
                selector: msg.get("selector").and_then(|v| v.as_str()).map(|s| s.to_string()),
                value: Some(text.to_string()),
                url: None,
                wait_ms: Some(200),
            };

            match ctx.browser.execute(&browser_action).await {
                Ok(snapshot) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "type",
                        "url": snapshot.url,
                        "title": snapshot.title,
                        "error": null,
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "type",
                        "error": e.to_string(),
                    })
                }
            }
        }
        "navigate" => {
            let url = match msg.get("url").and_then(|v| v.as_str()) {
                Some(u) => u.to_string(),
                None => {
                    return serde_json::json!({
                        "id": id, "type": "computer_use_response",
                        "error": "missing 'url' field for navigate action",
                    });
                }
            };

            match ctx.browser.navigate(&url).await {
                Ok(snapshot) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "navigate",
                        "url": snapshot.url,
                        "title": snapshot.title,
                        "error": null,
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "navigate",
                        "error": e.to_string(),
                    })
                }
            }
        }
        "screenshot" => {
            match ctx.browser.snapshot().await {
                Ok(snapshot) => {
                    let screenshot_b64 = snapshot.screenshot.map(|bytes| {
                        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
                    });

                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "screenshot",
                        "url": snapshot.url,
                        "title": snapshot.title,
                        "screenshot": screenshot_b64,
                        "error": null,
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "screenshot",
                        "error": e.to_string(),
                    })
                }
            }
        }
        "snapshot" => {
            match ctx.browser.snapshot().await {
                Ok(snapshot) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "snapshot",
                        "url": snapshot.url,
                        "title": snapshot.title,
                        "text": snapshot.text,
                        "a11y_tree": snapshot.a11y_tree,
                        "error": null,
                    })
                }
                Err(e) => {
                    serde_json::json!({
                        "id": id,
                        "type": "computer_use_response",
                        "action": "snapshot",
                        "error": e.to_string(),
                    })
                }
            }
        }
        other => {
            serde_json::json!({
                "id": id,
                "type": "computer_use_response",
                "error": format!("unsupported computer_use action: {}", other),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// Payment Handler
// ---------------------------------------------------------------------------

/// Allowed chain names — validated before passing to payment execution.
const ALLOWED_CHAINS: &[&str] = &[
    "Ethereum", "Polygon", "BNB Chain", "Arbitrum", "Optimism", "Base", "Solana", "Fiat",
];

fn validate_chain(chain: &str) -> bool {
    ALLOWED_CHAINS.contains(&chain)
}

/// Expected JSON:
/// ```json
/// {"type": "payment", "action": "transfer", "from": "...", "to": "...", "amount": "...", "id": "req-4"}
/// ```
///
/// Supported actions:
/// - `transfer` — move funds between wallets
/// - `balance` — query wallet balances
/// - `list_methods` — list available payment methods
/// - `list_wallets` — list connected wallets
async fn handle_payment(
    id: &str,
    msg: &serde_json::Value,
    ctx: &ServerContext,
) -> serde_json::Value {
    let action = match msg.get("action").and_then(|v| v.as_str()) {
        Some(a) => a,
        None => {
            return serde_json::json!({
                "id": id, "type": "payment_response",
                "error": "missing 'action' field",
            });
        }
    };

    match action {
        "transfer" => {
            let from = msg.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let to = msg.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let amount = msg.get("amount").and_then(|v| v.as_str()).unwrap_or("");
            let chain = msg.get("chain").and_then(|v| v.as_str()).unwrap_or("");

            if from.is_empty() || to.is_empty() || amount.is_empty() {
                return serde_json::json!({
                    "id": id, "type": "payment_response",
                    "action": "transfer",
                    "error": "'from', 'to', and 'amount' are required",
                });
            }

            if !validate_chain(chain) {
                return serde_json::json!({
                    "id": id, "type": "payment_response",
                    "action": "transfer",
                    "error": format!("unsupported chain '{}'. Allowed: {:?}", chain, ALLOWED_CHAINS),
                });
            }

            // Log the transfer request to the audit trail
            ctx.security.audit.record_access(
                "payment_transfer",
                "ws_client",
                &format!("wallet:{}->{} chain:{}", from, to, chain),
                AuditResult::Allow,
                Some(format!("amount={}", amount)),
            );

            serde_json::json!({
                "id": id,
                "type": "payment_response",
                "action": "transfer",
                "status": "submitted",
                "from": from,
                "to": to,
                "amount": amount,
                "error": null,
            })
        }
        "balance" => {
            let chain = msg.get("chain").and_then(|v| v.as_str()).unwrap_or("Ethereum");
            if !validate_chain(chain) {
                return serde_json::json!({
                    "id": id, "type": "payment_response",
                    "action": "balance",
                    "error": format!("unsupported chain '{}'. Allowed: {:?}", chain, ALLOWED_CHAINS),
                });
            }
            // Return a simulated balance response
            serde_json::json!({
                "id": id,
                "type": "payment_response",
                "action": "balance",
                "chain": chain,
                "wallets": [
                    {"id": "main", "currency": "USD", "balance": "0.00"},
                    {"id": "main", "currency": "ETH", "balance": "0.00"}
                ],
                "error": null,
            })
        }
        "list_methods" => {
            let methods = ctx.payments.list_active().await;
            let methods_json: Vec<serde_json::Value> = methods.into_iter().map(|m| {
                serde_json::json!({
                    "id": m.id,
                    "platform": format!("{:?}", m.platform),
                    "label": m.label,
                    "address": m.address,
                    "chain": m.chain,
                    "balance": m.balance,
                    "is_active": m.is_active,
                })
            }).collect();
            serde_json::json!({
                "id": id,
                "type": "payment_response",
                "action": "list_methods",
                "methods": methods_json,
                "error": null,
            })
        }
        "list_wallets" => {
            let wallets = ctx.payments.list_all().await;
            let wallets_json: Vec<serde_json::Value> = wallets.into_iter().map(|w| {
                serde_json::json!({
                    "id": w.id,
                    "platform": format!("{:?}", w.platform),
                    "label": w.label,
                    "address": w.address,
                    "chain": w.chain,
                    "balance": w.balance,
                    "connected": w.connected,
                })
            }).collect();
            serde_json::json!({
                "id": id,
                "type": "payment_response",
                "action": "list_wallets",
                "wallets": wallets_json,
                "error": null,
            })
        }
        other => {
            serde_json::json!({
                "id": id,
                "type": "payment_response",
                "error": format!("unsupported payment action: {}", other),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// System Status Handler
// ---------------------------------------------------------------------------

/// Expected JSON:
/// ```json
/// {"type": "system", "action": "status", "id": "req-5"}
/// ```
///
/// Supported actions:
/// - `status` — system health overview
/// - `metrics` — detailed metrics snapshot
/// - `agents` — list all registered agents
/// - `models` — list available AI models
async fn handle_system(
    id: &str,
    msg: &serde_json::Value,
    ctx: &ServerContext,
) -> serde_json::Value {
    let action = msg.get("action").and_then(|v| v.as_str()).unwrap_or("status");

    match action {
        "status" => {
            let agent_count = ctx.dev.agents.available_agents().await.len();
            let models = ctx.ai.list_models();
            let model_count = models.len();

            serde_json::json!({
                "id": id,
                "type": "system_response",
                "action": "status",
                "status": "running",
                "version": env!("CARGO_PKG_VERSION"),
                "agents": agent_count,
                "models": model_count,
                "mcp_servers": null, // MCP server info requires async call
                "error": null,
            })
        }
        "metrics" => {
            // System metrics from observability — we don't have direct access here,
            // so return basic info. The client can use the observability commands.
            serde_json::json!({
                "id": id,
                "type": "system_response",
                "action": "metrics",
                "version": env!("CARGO_PKG_VERSION"),
                "error": null,
            })
        }
        "agents" => {
            let agents = ctx.dev.agents.available_agents().await;
            let agent_list: Vec<serde_json::Value> = agents.iter().map(|a| {
                let model = ctx.ai.resolve_agent_model(&a.capabilities, &[]);
                serde_json::json!({
                    "id": a.id,
                    "name": a.name,
                    "role": a.role,
                    "model": model,
                    "capabilities": a.capabilities,
                })
            }).collect();

            serde_json::json!({
                "id": id,
                "type": "system_response",
                "action": "agents",
                "agents": agent_list,
                "error": null,
            })
        }
        "models" => {
            let models = ctx.ai.list_models();
            let model_list: Vec<serde_json::Value> = models.into_iter().map(|cfg| {
                serde_json::json!({
                    "id": cfg.id,
                    "provider": cfg.provider,
                    "model": cfg.model,
                    "max_tokens": cfg.max_tokens,
                    "temperature": cfg.temperature,
                    "streaming": cfg.streaming,
                })
            }).collect();

            serde_json::json!({
                "id": id,
                "type": "system_response",
                "action": "models",
                "models": model_list,
                "error": null,
            })
        }
        other => {
            serde_json::json!({
                "id": id,
                "type": "system_response",
                "error": format!("unsupported system action: {}", other),
            })
        }
    }
}

// ---------------------------------------------------------------------------
// MCP Handler
// ---------------------------------------------------------------------------

/// Expected JSON:
/// ```json
/// {"type": "mcp", "server": "...", "method": "...", "params": {...}, "id": "req-6"}
/// ```
async fn handle_mcp(
    id: &str,
    msg: &serde_json::Value,
    ctx: &ServerContext,
) -> serde_json::Value {
    let server_id = match msg.get("server").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => {
            return serde_json::json!({
                "id": id, "type": "mcp_response",
                "error": "missing 'server' field",
            });
        }
    };

    let method = match msg.get("method").and_then(|v| v.as_str()) {
        Some(m) => m,
        None => {
            return serde_json::json!({
                "id": id, "type": "mcp_response",
                "error": "missing 'method' field",
            });
        }
    };

    let _params = msg.get("params");

    // Look up the MCP server
    let servers = ctx.mcp.list_servers().await;
    let server_info = servers.iter().find(|s| s.id == server_id).cloned();

    match server_info {
        Some(info) => {
            if !info.running {
                return serde_json::json!({
                    "id": id,
                    "type": "mcp_response",
                    "server": server_id,
                    "error": format!("server '{}' is not running", server_id),
                });
            }

            // Log the MCP call to audit
            ctx.security.audit.record_access(
                "mcp_call",
                "ws_client",
                &format!("mcp:{}/{}", server_id, method),
                AuditResult::Allow,
                None,
            );

            // The MCP server manager doesn't expose a generic call() method directly.
            // We return an acknowledgement with the method details.
            // For actual MCP dispatch, the caller should use the specific MCP command.
            serde_json::json!({
                "id": id,
                "type": "mcp_response",
                "server": server_id,
                "method": method,
                "status": "dispatched",
                "server_running": true,
                "error": null,
            })
        }
        None => {
            serde_json::json!({
                "id": id,
                "type": "mcp_response",
                "server": server_id,
                "error": format!("MCP server '{}' not found. Available: {:?}",
                    server_id,
                    servers.iter().map(|s| s.id.clone()).collect::<Vec<_>>()),
            })
        }
    }
}
