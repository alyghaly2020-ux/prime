//! # Prime Headless Binary Entrypoint
//!
//! This binary can run in three modes:
//!
//! 1. **GUI mode** (default / `gui` subcommand) — launches the normal Tauri window
//!    by delegating to [`PrimeApp::run()`].
//! 2. **Server mode** (`server` subcommand) — starts the WebSocket control server
//!    without opening any window.
//! 3. **Command mode** (`chat`, `execute`, `screenshot`, `status`) — runs a single
//!    command and prints the JSON result to stdout, then exits.
//!
//! ## Usage
//!
//! ```bash
//! # Start headless WebSocket server on port 9876
//! PRIME_WS_TOKEN=secret-token prime-headless server
//!
//! # Chat with a model (one-shot, exits after response)
//! prime-headless chat gpt-5 "What is the capital of France?"
//!
//! # Execute an agent task
//! prime-headless execute code-review "Check src/main.rs for issues"
//!
//! # Take a browser screenshot
//! prime-headless screenshot
//!
//! # Get system status
//! prime-headless status
//!
//! # Launch GUI (default)
//! prime-headless gui
//! prime-headless
//! ```

use clap::Parser;
use prime::cli::Cli;
use prime::server::{ServerContext, WebSocketServer};
use prime::{ai, browser, dev, mcp, tools};
use prime_core::security;
use std::sync::Arc;

// =============================================================================
// Entrypoint
// =============================================================================

fn main() {
    let cli = Cli::parse();

    match cli {
        Cli::Gui | Cli::Headless { .. } => {
            // Both GUI and Server mode need system initialization.
            // For Server mode, we skip the Tauri window entirely.
            // For GUI mode, we delegate to the normal PrimeApp.
            let is_server = matches!(cli, Cli::Headless { .. });

            if is_server {
                run_headless_server(cli);
            } else {
                // Delegate to the normal Tauri GUI
                prime::PrimeApp::run();
            }
        }
        Cli::Chat { model, message } => {
            run_single_chat(&model, &message);
        }
        Cli::Execute { agent, task } => {
            run_single_execute(&agent, &task);
        }
        Cli::Screenshot => {
            run_single_screenshot();
        }
        Cli::Status => {
            run_single_status();
        }
    }
}

// =============================================================================
// Headless Server Mode
// =============================================================================

/// Initialize systems and start the WebSocket server (no GUI window).
fn run_headless_server(cli: Cli) {
    // Initialize tracing (same as PrimeApp but without Tauri)
    init_tracing();

    tracing::info!("🚀 Prime v{} — Headless Server Mode", env!("CARGO_PKG_VERSION"));

    // Build the async runtime and run the server
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        // Initialize all subsystems
        let context = initialize_systems().await;

        // Extract server config
        let port = match &cli {
            Cli::Headless { port, .. } => *port,
            _ => 9876,
        };
        let token = match &cli {
            Cli::Headless { token, .. } => token.clone(),
            _ => None,
        };

        let server = match token {
            Some(t) => WebSocketServer::new(port, context).with_token(t),
            None => WebSocketServer::new(port, context),
        };

        tracing::info!("Starting WebSocket server on port {}", port);

        // Run the server (blocking until shutdown)
        if let Err(e) = server.start().await {
            tracing::error!("WebSocket server terminated with error: {}", e);
        }
    });
}

// =============================================================================
// Single Command Modes
// =============================================================================

/// Run a single chat command with the given model and message, print JSON result.
fn run_single_chat(model: &str, message: &str) {
    init_tracing();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        let ctx = initialize_systems().await;

        let msg = ai::ChatMessage {
            role: "user".to_string(),
            content: message.to_string(),
            tool_calls: None,
            timestamp: None,
        };

        match ctx.ai.chat(vec![msg], model).await {
            Ok(content) => {
                let result = serde_json::json!({
                    "type": "chat_response",
                    "content": content,
                    "model": model,
                    "error": null,
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            Err(e) => {
                let result = serde_json::json!({
                    "type": "chat_response",
                    "content": null,
                    "model": model,
                    "error": ai::redact_sensitive(&e.to_string()),
                });
                eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
                std::process::exit(1);
            }
        }
    });
}

/// Run a single agent execute command, print JSON result.
fn run_single_execute(agent_id: &str, task: &str) {
    init_tracing();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        let ctx = initialize_systems().await;

        let agents = ctx.dev.agents.available_agents().await;
        let agent = agents.iter().find(|a| a.id == agent_id).cloned();

        match agent {
            Some(agent_def) => {
                let model = ctx.ai.resolve_agent_model(&agent_def.capabilities, &[]);

                let system_msg = ai::ChatMessage {
                    role: "system".to_string(),
                    content: agent_def.system_prompt.clone(),
                    tool_calls: None,
                    timestamp: None,
                };
                let task_msg = ai::ChatMessage {
                    role: "user".to_string(),
                    content: task.to_string(),
                    tool_calls: None,
                    timestamp: None,
                };

                match ctx.ai.chat(vec![system_msg, task_msg], &model).await {
                    Ok(content) => {
                        let result = serde_json::json!({
                            "type": "agent_response",
                            "agent_id": agent_id,
                            "agent_name": agent_def.name,
                            "content": content,
                            "model": model,
                            "error": null,
                        });
                        println!("{}", serde_json::to_string_pretty(&result).unwrap());
                    }
                    Err(e) => {
                        let result = serde_json::json!({
                            "type": "agent_response",
                            "agent_id": agent_id,
                            "content": null,
                            "error": ai::redact_sensitive(&e.to_string()),
                        });
                        eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
                        std::process::exit(1);
                    }
                }
            }
            None => {
                let result = serde_json::json!({
                    "type": "agent_response",
                    "agent_id": agent_id,
                    "error": format!("agent '{}' not found", agent_id),
                });
                eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
                std::process::exit(1);
            }
        }
    });
}

/// Take a browser screenshot, print JSON result with base64-encoded image.
fn run_single_screenshot() {
    init_tracing();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        let ctx = initialize_systems().await;

        match ctx.browser.snapshot().await {
            Ok(snapshot) => {
                let screenshot_b64 = snapshot.screenshot.map(|bytes| {
                    base64::Engine::encode(
                        &base64::engine::general_purpose::STANDARD,
                        &bytes,
                    )
                });

                let result = serde_json::json!({
                    "type": "computer_use_response",
                    "action": "screenshot",
                    "url": snapshot.url,
                    "title": snapshot.title,
                    "screenshot": screenshot_b64,
                    "error": null,
                });
                println!("{}", serde_json::to_string_pretty(&result).unwrap());
            }
            Err(e) => {
                let result = serde_json::json!({
                    "type": "computer_use_response",
                    "action": "screenshot",
                    "error": e.to_string(),
                });
                eprintln!("{}", serde_json::to_string_pretty(&result).unwrap());
                std::process::exit(1);
            }
        }
    });
}

/// Print system status as JSON.
fn run_single_status() {
    init_tracing();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        let ctx = initialize_systems().await;

        let agent_count = ctx.dev.agents.available_agents().await.len();
        let models = ctx.ai.list_models();
        let mcp_servers = ctx.mcp.list_servers().await;

        let server_list: Vec<serde_json::Value> = mcp_servers
            .iter()
            .map(|s| {
                serde_json::json!({
                    "id": s.id,
                    "name": s.name,
                    "version": s.version,
                    "running": s.running,
                })
            })
            .collect();

        let model_list: Vec<serde_json::Value> = models
            .into_iter()
            .map(|cfg| {
                serde_json::json!({
                    "id": cfg.id,
                    "provider": cfg.provider,
                    "model": cfg.model,
                })
            })
            .collect();

        let result = serde_json::json!({
            "type": "system_response",
            "action": "status",
            "status": "running",
            "version": env!("CARGO_PKG_VERSION"),
            "agents": agent_count,
            "models": model_list,
            "mcp_servers": server_list,
            "error": null,
        });

        println!("{}", serde_json::to_string_pretty(&result).unwrap());
    });
}

// =============================================================================
// System Initialization
// =============================================================================

/// Initialize the tracing/logging subsystem.
fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive(
            "prime=debug"
                .parse()
                .expect("invalid tracing filter directive"),
        )
        .add_directive("reqwest=warn".parse().expect("invalid filter directive"))
        .add_directive("hyper=warn".parse().expect("invalid filter directive"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();
}

/// Initialize all Prime subsystems needed for headless operation.
///
/// This mirrors the initialization in [`PrimeApp::run()`]'s `setup` closure
/// but without the Tauri dependency. Creates core systems, seeds agents,
/// registers MCP servers, and starts background guardian tasks.
async fn initialize_systems() -> Arc<ServerContext> {
    tracing::info!("Initializing Prime subsystems...");

    // Core runtime
    let _runtime = Arc::new(prime_core::core::Runtime::new());

    // Memory system
    let memory = Arc::new(prime::memory::System::new());

    // Skills system
    let _skills = Arc::new(prime::skills::System::new());

    // Execution engine
    let _execution = Arc::new(prime::execution::Engine::new());

    // Verification system
    let _verification = Arc::new(prime::verification::System::new());

    // Code intelligence
    let code_intel = Arc::new(prime::code_intel::Engine::new());

    // Browser automation
    let browser = Arc::new(browser::System::new());

    // Development engine (agents, indexing, governance)
    let dev = Arc::new(dev::Engine::new());
    dev.init_retrieval(
        memory.vector.clone(),
        code_intel.symbols.clone(),
        code_intel.deps.clone(),
    );
    dev.seed_agents().await;
    tracing::info!("Seeded {} agents", dev.agents.available_agents().await.len());

    // System monitor (guardian)
    let _system_monitor = Arc::new(prime::system::SystemMonitor::new());

    // MCP server manager
    let mcp = Arc::new(mcp::ServerManager::new());
    {
        let mc = mcp.clone();
        let br = browser.clone();
        let mem = memory.clone();
        let ci = code_intel.clone();
        let dv = dev.clone();
        let _st = _runtime.storage.clone();
        tokio::spawn(async move {
            mc.register(Arc::new(mcp::filesystem::FilesystemMcp::new(vec![] as Vec<String>))).await;
            mc.register(Arc::new(mcp::git::GitMcp::new())).await;
            mc.register(Arc::new(mcp::terminal::TerminalMcp::new())).await;
            mc.register(Arc::new(mcp::browser::BrowserMcp::new(br))).await;
            mc.register(Arc::new(mcp::memory::MemoryMcp::new(mem))).await;
            mc.register(Arc::new(mcp::search::SearchMcp::new(ci, dv))).await;
            mc.register(Arc::new(mcp::os::OsMcp::new())).await;
        });
    }

    // Start MCP server manager
    let mcp_handle = mcp.clone();
    tokio::spawn(async move {
        if let Err(e) = mcp_handle.start().await {
            tracing::error!("MCP server manager failed to start: {:?}", e);
        }
    });
    mcp.start_health_checks();

    // AI router
    let ai = Arc::new(ai::Router::new());

    // Security system
    let security = Arc::new(security::System::new());

    // Payments manager
    let payments = Arc::new(tools::PaymentsManager::new());

    tracing::info!("Prime subsystems initialized successfully");

    Arc::new(ServerContext {
        ai,
        dev,
        browser,
        payments,
        mcp,
        security,
    })
}
