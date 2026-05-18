//! # Prime Application Entrypoint
//!
//! This binary supports two modes:
//! - **GUI mode** (default): launches the normal Tauri window via [`PrimeApp::run()`].
//! - **Headless/server mode** (`prime headless`): starts the WebSocket control server
//!   without opening any window.
//!
//! ## Usage
//!
//! ```bash
//! # Launch GUI (default)
//! prime
//!
//! # Start headless WebSocket server on port 9876
//! PRIME_WS_TOKEN=secret-token prime headless
//!
//! # Start server on custom port
//! prime headless --port 8765
//! ```

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use clap::Parser;
use prime::cli::Cli;
use prime::server::{ServerContext, WebSocketServer};
use prime::{ai, browser, dev, mcp, tools, PrimeApp};
use prime_core::security;
use std::sync::Arc;

fn main() {
    // Launch GUI directly if no args at all (skip clap's eager help suggestion)
    if std::env::args().len() <= 1 {
        return PrimeApp::run();
    }

    match Cli::try_parse() {
        Ok(Cli::Headless { port, token }) => {
            init_tracing();
            tracing::info!(
                "🚀 Prime v{} — Headless/Server Mode (port {})",
                env!("CARGO_PKG_VERSION"),
                port
            );
            run_headless_server(port, token);
        }
        Ok(Cli::Gui) => PrimeApp::run(),
        Ok(cmd) => {
            init_tracing();
            let rt = tokio::runtime::Runtime::new()
                .expect("failed to create tokio runtime");
            let result: anyhow::Result<()> = rt.block_on(async {
                let ctx = initialize_systems().await;
                match cmd {
                    Cli::Chat { model, message } => {
                        let msg = ai::ChatMessage {
                            role: "user".into(),
                            content: message,
                            tool_calls: None,
                            timestamp: None,
                        };
                        let response = ctx.ai.chat(vec![msg], &model).await?;
                        println!("{}", response);
                        Ok(())
                    }
                    Cli::Execute { agent, task } => {
                        let session_id = ctx.dev.start_session(".").await;
                        println!("Session: {} | Agent: {} | Task: {}", session_id, agent, task);
                        Ok(())
                    }
                    Cli::Screenshot => {
                        let snap = ctx.browser.snapshot().await?;
                        println!("Page: {} | URL: {}", snap.title, snap.url);
                        Ok(())
                    }
                    Cli::Status => {
                        let agents = ctx.dev.agents.available_agents().await.len();
                        println!("Prime v{} | Agents: {} | Systems: online",
                            env!("CARGO_PKG_VERSION"), agents);
                        Ok(())
                    }
                    Cli::Gui | Cli::Headless { .. } => unreachable!(),
                }
            });
            if let Err(e) = result {
                tracing::error!("Command failed: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            // --help, --version, bad flag → let clap print & exit with correct code
            e.exit();
        }
    }
}

// =============================================================================
// Tracing Initialization
// =============================================================================

fn init_tracing() {
    let env_filter = tracing_subscriber::EnvFilter::from_default_env()
        .add_directive("prime=debug".parse().expect("invalid tracing filter directive"))
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

// =============================================================================
// Headless Server Mode
// =============================================================================

fn run_headless_server(port: u16, token: Option<String>) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

    rt.block_on(async {
        let context = initialize_systems().await;

        let server = match token {
            Some(t) => WebSocketServer::new(port, context).with_token(t),
            None => WebSocketServer::new(port, context),
        };

        tracing::info!("Starting WebSocket server on port {}", port);

        if let Err(e) = server.start().await {
            tracing::error!("WebSocket server terminated with error: {}", e);
        }
    });
}

// =============================================================================
// System Initialization
// =============================================================================

async fn initialize_systems() -> Arc<ServerContext> {
    tracing::info!("Initializing Prime subsystems...");

    let _runtime = Arc::new(prime_core::core::Runtime::new());
    let memory = Arc::new(prime::memory::System::new());
    let _skills = Arc::new(prime::skills::System::new());
    let _execution = Arc::new(prime::execution::Engine::new());
    let _verification = Arc::new(prime::verification::System::new());
    let code_intel = Arc::new(prime::code_intel::Engine::new());
    let browser = Arc::new(browser::System::new());

    let dev = Arc::new(dev::Engine::new());
    dev.init_retrieval(
        memory.vector.clone(),
        code_intel.symbols.clone(),
        code_intel.deps.clone(),
    );
    dev.seed_agents().await;
    tracing::info!("Seeded {} agents", dev.agents.available_agents().await.len());

    let _system_monitor = Arc::new(prime::system::SystemMonitor::new());

    let mcp = Arc::new(mcp::ServerManager::new());
    {
        let mc = mcp.clone();
        let br = browser.clone();
        let mem = memory.clone();
        let ci = code_intel.clone();
        let dv = dev.clone();
        tokio::spawn(async move {
            mc.register(Arc::new(mcp::filesystem::FilesystemMcp::new(vec![]))).await;
            mc.register(Arc::new(mcp::git::GitMcp::new())).await;
            mc.register(Arc::new(mcp::terminal::TerminalMcp::new())).await;
            mc.register(Arc::new(mcp::browser::BrowserMcp::new(br))).await;
            mc.register(Arc::new(mcp::memory::MemoryMcp::new(mem))).await;
            mc.register(Arc::new(mcp::search::SearchMcp::new(ci, dv))).await;
            mc.register(Arc::new(mcp::os::OsMcp::new())).await;
        });
    }

    let mcp_handle = mcp.clone();
    tokio::spawn(async move {
        if let Err(e) = mcp_handle.start().await {
            tracing::error!("MCP server manager failed to start: {:?}", e);
        }
    });
    mcp.start_health_checks();

    let ai = Arc::new(ai::Router::new());
    let security = Arc::new(security::System::new());
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
