//! Browser stealth manager. Manages 14 browser stealth tools for undetectable
//! browser automation: fingerprint spoofing, bot detection bypass, reCAPTCHA
//! solving, and TLS fingerprint masking.
//!
//! Provides a unified interface to launch/stop stealth browser providers from
//! three categories:
//! - **McpServer**: MCP-based (mcp-stealth-chrome, ZenDriver, Wick, etc.)
//! - **NpmPackage**: npm-based (CloakBrowser, Camoufox, etc.)
//! - **BuiltIn**: Native Rust headless browser (Obscura)

use std::collections::HashMap;
use std::process::{Child, Command};
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::RwLock;

use crate::AppError;

// =============================================================================
// Stealth Provider Types & Status
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StealthProviderType {
    McpServer,
    NpmPackage,
    BuiltIn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StealthStatus {
    Available,
    Running,
    Error(String),
}

impl std::fmt::Display for StealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StealthStatus::Available => write!(f, "available"),
            StealthStatus::Running => write!(f, "running"),
            StealthStatus::Error(e) => write!(f, "error: {e}"),
        }
    }
}

// =============================================================================
// StealthProvider
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StealthProvider {
    pub id: String,
    pub name: String,
    pub provider_type: StealthProviderType,
    pub capabilities: Vec<String>,
    pub config: Value,
    pub status: StealthStatus,
}

// =============================================================================
// BrowserStealthManager
// =============================================================================

pub struct BrowserStealthManager {
    providers: RwLock<HashMap<String, StealthProvider>>,
    active_provider: RwLock<Option<String>>,
    /// Track spawned child processes so `shutdown` can kill them.
    processes: RwLock<HashMap<String, Child>>,
}

impl std::fmt::Debug for BrowserStealthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrowserStealthManager").finish_non_exhaustive()
    }
}

impl Default for BrowserStealthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl BrowserStealthManager {
    // -------------------------------------------------------------------------
    // Construction
    // -------------------------------------------------------------------------

    pub fn new() -> Self {
        let providers = Self::init_providers();
        Self {
            providers: RwLock::new(providers),
            active_provider: RwLock::new(None),
            processes: RwLock::new(HashMap::new()),
        }
    }

    /// (Re-)register all 14 stealth providers from the tool config table.
    /// Existing providers keep their `status`; new entries are added.
    pub async fn register_defaults(&self) {
        use crate::tools::config::{ToolCategory, ToolSource, all_tool_configs};

        let configs = all_tool_configs();
        let mut providers = self.providers.write().await;

        for tool in configs
            .into_iter()
            .filter(|t| t.category == ToolCategory::BrowserStealth)
        {
            let provider_type = match tool.source {
                ToolSource::Mcp => StealthProviderType::McpServer,
                ToolSource::Npm => StealthProviderType::NpmPackage,
                ToolSource::Rust => StealthProviderType::BuiltIn,
                _ => continue,
            };

            let capabilities = description_capabilities(&tool.description);
            let config = json!({
                "install_cmd": tool.install_cmd,
                "run_cmd": tool.run_cmd,
                "health_check_url": tool.health_check_url,
                "port": tool.port,
                "homepage": tool.homepage,
                "version": tool.version,
            });

            providers.entry(tool.id.clone()).or_insert_with(|| {
                let id = tool.id.clone();
                StealthProvider {
                    id,
                    name: tool.name,
                    provider_type,
                    capabilities,
                    config,
                    status: StealthStatus::Available,
                }
            });
        }
    }

    // -------------------------------------------------------------------------
    // Querying
    // -------------------------------------------------------------------------

    /// Return a snapshot of every registered provider.
    pub async fn list_providers(&self) -> Vec<StealthProvider> {
        self.providers.read().await.values().cloned().collect()
    }

    /// Return a single provider by id.
    pub async fn get_provider(&self, id: &str) -> Option<StealthProvider> {
        self.providers.read().await.get(id).cloned()
    }

    /// Pick the best provider whose capabilities match the given requirements.
    ///
    /// Each requirement is a substring-matched against provider capabilities.
    /// Returns the id of the provider with the most matches (ties → first).
    pub async fn select_provider(&self, requirements: &[&str]) -> Option<String> {
        let providers = self.providers.read().await;
        providers
            .iter()
            .filter_map(|(id, p)| {
                let score = requirements
                    .iter()
                    .filter(|req| p.capabilities.iter().any(|c| c.contains(*req)))
                    .count();
                if score > 0 { Some((id.clone(), score)) } else { None }
            })
            .max_by_key(|(_, score)| *score)
            .map(|(id, _)| id)
    }

    // -------------------------------------------------------------------------
    // Lifecycle
    // -------------------------------------------------------------------------

    /// Launch a stealth provider by id.
    ///
    /// - **McpServer / NpmPackage**: spawns the process via `run_cmd`.
    /// - **BuiltIn**: no process needed (already linked in the browser module).
    pub async fn launch(&self, id: &str) -> anyhow::Result<()> {
        let (provider_type, run_cmd) = {
            let providers = self.providers.read().await;
            let p = providers
                .get(id)
                .ok_or_else(|| anyhow::anyhow!("Unknown stealth provider: {id}"))?;
            (p.provider_type.clone(), p.config.get("run_cmd").and_then(|v| v.as_str().map(String::from)))
        };

        match provider_type {
            StealthProviderType::BuiltIn => {
                // Obscura is compiled in — nothing to spawn.
            }
            StealthProviderType::McpServer | StealthProviderType::NpmPackage => {
                if let Some(cmd) = run_cmd {
                    let child = spawn_command(&cmd)?;
                    self.processes.write().await.insert(id.to_string(), child);
                } else {
                    anyhow::bail!("No run_cmd configured for provider '{id}'");
                }
            }
        }

        let mut providers = self.providers.write().await;
        if let Some(p) = providers.get_mut(id) {
            p.status = StealthStatus::Running;
        }
        *self.active_provider.write().await = Some(id.to_string());

        Ok(())
    }

    /// Stop a running stealth provider.
    ///
    /// Kills the spawned child process (if any) and resets status to Available.
    pub async fn shutdown(&self, id: &str) -> anyhow::Result<()> {
        // Kill the child process if we have one.
        let mut processes = self.processes.write().await;
        if let Some(mut child) = processes.remove(id) {
            child.kill()?;
            child.wait()?;
        }
        drop(processes);

        // Reset provider status.
        let mut providers = self.providers.write().await;
        if let Some(p) = providers.get_mut(id) {
            p.status = StealthStatus::Available;
        }

        // Clear active marker if this was the active provider.
        let mut active = self.active_provider.write().await;
        if active.as_deref() == Some(id) {
            *active = None;
        }

        Ok(())
    }

    // =====================================================================
    // Private helpers
    // =====================================================================

    /// Build the default 14-provider set synchronously (called from `new()`).
    fn init_providers() -> HashMap<String, StealthProvider> {
        let mut m: HashMap<String, StealthProvider> = HashMap::new();

        // ---- NPM PACKAGES (6) ----

        m.insert(
            "cloakbrowser".into(),
            StealthProvider {
                id: "cloakbrowser".into(),
                name: "CloakBrowser".into(),
                provider_type: StealthProviderType::NpmPackage,
                capabilities: vec![
                    "stealth-chromium".into(),
                    "cpp-patches".into(),
                    "bot-detection-bypass".into(),
                    "anti-fingerprinting".into(),
                    "popup-blocking".into(),
                ],
                config: provider_config(
                    "npm install cloakbrowser",
                    "npx cloakbrowser",
                    None,
                    None,
                    "https://github.com/your/cloakbrowser",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "camoufox".into(),
            StealthProvider {
                id: "camoufox".into(),
                name: "Camoufox Browser Server".into(),
                provider_type: StealthProviderType::NpmPackage,
                capabilities: vec![
                    "firefox-fork".into(),
                    "fingerprint-spoofing".into(),
                    "stealth-server".into(),
                    "ai-agent".into(),
                ],
                config: provider_config(
                    "npm install camoufox",
                    "npx camoufox",
                    None,
                    None,
                    "https://github.com/redf0x1/camofox-browser",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "invisible-playwright".into(),
            StealthProvider {
                id: "invisible-playwright".into(),
                name: "invisible_playwright".into(),
                provider_type: StealthProviderType::NpmPackage,
                capabilities: vec![
                    "firefox-150".into(),
                    "playwright-wrapper".into(),
                    "recaptcha-v3-bypass".into(),
                    "fingerprintpro-bypass".into(),
                    "cpp-level-stealth".into(),
                ],
                config: provider_config(
                    "npm install invisible_playwright",
                    "npx invisible_playwright",
                    None,
                    None,
                    "https://github.com/feder-cr/invisible_playwright",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "botbrowser".into(),
            StealthProvider {
                id: "botbrowser".into(),
                name: "BotBrowser".into(),
                provider_type: StealthProviderType::NpmPackage,
                capabilities: vec![
                    "privacy-browser".into(),
                    "cloudflare-bypass".into(),
                    "akamai-bypass".into(),
                    "kasada-bypass".into(),
                    "datadome-bypass".into(),
                    "hcaptcha-bypass".into(),
                ],
                config: provider_config(
                    "npm install botbrowser",
                    "npx botbrowser",
                    None,
                    None,
                    "https://github.com/botswin/BotBrowser",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "puppeteer-fingerprints".into(),
            StealthProvider {
                id: "puppeteer-fingerprints".into(),
                name: "puppeteer-with-fingerprints".into(),
                provider_type: StealthProviderType::NpmPackage,
                capabilities: vec![
                    "puppeteer-plugin".into(),
                    "fingerprint-rotation".into(),
                    "virtual-identity".into(),
                    "browser-fingerprint".into(),
                ],
                config: provider_config(
                    "npm install puppeteer-with-fingerprints",
                    "npx puppeteer-with-fingerprints",
                    None,
                    None,
                    "https://github.com/your/puppeteer-fingerprints",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "fingerprint-suite".into(),
            StealthProvider {
                id: "fingerprint-suite".into(),
                name: "fingerprint-suite".into(),
                provider_type: StealthProviderType::NpmPackage,
                capabilities: vec![
                    "fingerprint-generator".into(),
                    "fingerprint-injector".into(),
                    "modular-toolkit".into(),
                    "playwright-injection".into(),
                    "puppeteer-injection".into(),
                ],
                config: provider_config(
                    "npm install fingerprint-generator fingerprint-injector",
                    None,
                    None,
                    None,
                    "https://github.com/your/fingerprint-suite",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        // ---- MCP SERVERS (7) ----

        m.insert(
            "mcp-stealth-chrome".into(),
            StealthProvider {
                id: "mcp-stealth-chrome".into(),
                name: "MCP Stealth Chrome".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "133-tools".into(),
                    "cloudflare-turnstile-bypass".into(),
                    "tls-fingerprint-masking".into(),
                    "single-line-bypass".into(),
                ],
                config: provider_config(
                    "npx @RobithYusuf/mcp-stealth-chrome",
                    "npx @RobithYusuf/mcp-stealth-chrome",
                    None,
                    None,
                    "https://github.com/RobithYusuf/mcp-stealth-chrome",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "zendriver-mcp".into(),
            StealthProvider {
                id: "zendriver-mcp".into(),
                name: "ZenDriver MCP Server".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "cdp-devtools".into(),
                    "undetectable-automation".into(),
                    "96-tools".into(),
                    "bot-detection-bypass".into(),
                ],
                config: provider_config(
                    "npx @bituq/zendriver-mcp",
                    "npx @bituq/zendriver-mcp",
                    None,
                    None,
                    "https://github.com/bituq/zendriver-mcp",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "cybr-ghost".into(),
            StealthProvider {
                id: "cybr-ghost".into(),
                name: "Cybr Ghost".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "stealth-browser".into(),
                    "any-mcp-client".into(),
                    "universal-compatibility".into(),
                ],
                config: provider_config(
                    "npx cybr-ghost",
                    "npx cybr-ghost",
                    None,
                    None,
                    "https://lobehub.com/cybr-ghost",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "wick-mcp".into(),
            StealthProvider {
                id: "wick-mcp".into(),
                name: "Wick MCP".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "real-chrome-stack".into(),
                    "tls-fingerprint-match".into(),
                    "network-stack".into(),
                ],
                config: provider_config(
                    "npx @wickproject/wick",
                    "npx @wickproject/wick",
                    None,
                    None,
                    "https://github.com/wickproject/wick",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "autosurfer-mcp".into(),
            StealthProvider {
                id: "autosurfer-mcp".into(),
                name: "AutoSurfer MCP".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "autonomous-browsing".into(),
                    "playwright-engine".into(),
                    "stealth-extensions".into(),
                    "llm-agent".into(),
                ],
                config: provider_config(
                    "npx autosurfer-mcp",
                    "npx autosurfer-mcp",
                    None,
                    None,
                    "https://codeberg.org/autosurfer-mcp",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "openchrome".into(),
            StealthProvider {
                id: "openchrome".into(),
                name: "openchrome".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "real-chrome-cdp".into(),
                    "27-subsystems".into(),
                    "browser-automation".into(),
                    "open-source".into(),
                ],
                config: provider_config(
                    "npx openchrome",
                    "npx openchrome",
                    None,
                    None,
                    "https://github.com/shaun0927/openchrome",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        m.insert(
            "byob".into(),
            StealthProvider {
                id: "byob".into(),
                name: "BYOB".into(),
                provider_type: StealthProviderType::McpServer,
                capabilities: vec![
                    "mcp-protocol".into(),
                    "real-logged-in-chrome".into(),
                    "local-server".into(),
                    "existing-session".into(),
                ],
                config: provider_config(
                    "npx byob",
                    "npx byob",
                    None,
                    None,
                    "https://github.com/wxtsky/byob",
                    "1.0.0",
                ),
                status: StealthStatus::Available,
            },
        );

        // ---- BUILT-IN (1) ----

        m.insert(
            "obscura".into(),
            StealthProvider {
                id: "obscura".into(),
                name: "Obscura".into(),
                provider_type: StealthProviderType::BuiltIn,
                capabilities: vec![
                    "headless".into(),
                    "rust-native".into(),
                    "lightweight".into(),
                    "low-memory-30mb".into(),
                    "stealth-mode".into(),
                ],
                config: provider_config(None, None, None, None, "https://github.com/h4ckf0r0day/obscura", "0.1.0"),
                status: StealthStatus::Available,
            },
        );

        m
    }
}

// =============================================================================
// Internal helpers
// =============================================================================

/// Build a JSON config value for a stealth provider.
fn provider_config(
    install_cmd: impl Into<Option<&'static str>>,
    run_cmd: impl Into<Option<&'static str>>,
    health_check_url: impl Into<Option<&'static str>>,
    port: impl Into<Option<u16>>,
    homepage: &str,
    version: &str,
) -> Value {
    json!({
        "install_cmd": install_cmd.into(),
        "run_cmd": run_cmd.into(),
        "health_check_url": health_check_url.into(),
        "port": port.into(),
        "homepage": homepage,
        "version": version,
    })
}

/// Parse a command string (`"npx @scope/pkg arg"`) into a spawned `Child`.
fn spawn_command(cmd_str: &str) -> anyhow::Result<Child> {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        anyhow::bail!("Empty run_cmd string");
    }
    let program = parts[0];
    let args: &[&str] = &parts[1..];
    let child = Command::new(program).args(args).spawn()?;
    Ok(child)
}

/// Extract keyword-like capabilities from a tool description string.
fn description_capabilities(description: &str) -> Vec<String> {
    description
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|s| s.len() >= 4 && !s.chars().all(|c| c.is_ascii_digit()))
        .map(|s| s.to_lowercase())
        .collect()
}

// =============================================================================
// Tauri Command Wrappers
// =============================================================================

/// Return every registered stealth provider and their status.
#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn list_stealth_providers(
    stealth: tauri::State<'_, Arc<BrowserStealthManager>>,
) -> Result<Vec<StealthProvider>, AppError> {
    Ok(stealth.list_providers().await)
}

/// Launch a stealth provider by id.
#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn launch_stealth_provider(
    stealth: tauri::State<'_, Arc<BrowserStealthManager>>,
    id: String,
) -> Result<(), AppError> {
    stealth
        .launch(&id)
        .await
        .map_err(|e| AppError::Execution(e.to_string()))
}

/// Shutdown a running stealth provider by id.
#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn shutdown_stealth_provider(
    stealth: tauri::State<'_, Arc<BrowserStealthManager>>,
    id: String,
) -> Result<(), AppError> {
    stealth
        .shutdown(&id)
        .await
        .map_err(|e| AppError::Execution(e.to_string()))
}
