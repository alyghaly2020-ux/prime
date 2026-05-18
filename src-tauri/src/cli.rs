//! CLI argument parsing for Prime headless mode.
//!
//! Use `clap` to define subcommands for the headless binary.
//! The normal Tauri GUI is the default when no subcommand is given.

use clap::Parser;

/// Prime AI Desktop — Headless Mode
///
/// Control Prime via command-line or WebSocket server.
/// Without any subcommand, the Tauri GUI is launched.
#[derive(Parser, Debug)]
#[command(name = "prime", about = "Prime AI Desktop - Headless Mode", version)]
pub enum Cli {
    /// Start WebSocket server for remote control (no GUI window)
    #[command(name = "headless", alias = "server")]
    Headless {
        /// Port to bind the WebSocket server to
        #[arg(long, default_value = "9876")]
        port: u16,

        /// Bearer token for WebSocket authentication
        /// (overrides PRIME_WS_TOKEN env var)
        #[arg(long)]
        token: Option<String>,
    },

    /// Execute a single chat command against an AI model
    Chat {
        /// Model ID to use (e.g., "gpt-5", "claude-4")
        model: String,

        /// Message to send to the model
        message: String,
    },

    /// Execute an agent task
    Execute {
        /// Agent ID to invoke
        agent: String,

        /// Task description for the agent
        task: String,
    },

    /// Take a browser screenshot and return analysis
    Screenshot,

    /// Get system status information
    Status,

    /// Open the GUI (default if no args provided)
    Gui,
}
