//! API Gateway integration for Prime.
//!
//! Manages multiple AI gateway backends:
//!
//! | Gateway           | Type   | Port  | Source                |
//! |-------------------|--------|-------|-----------------------|
//! | OmniRoute         | Docker | 8088  | diegosouzapw/omniroute |
//! | Plexus            | Docker | 8080  | mcowger/plexus         |
//! | SmarterRouter     | pip    | —     | smarter-router         |
//! | FreeRouter        | pip    | —     | freerouter             |
//! | LunarGate         | Go bin | —     | lunargate-ai/gateway   |
//! | FerroGateway      | Go bin | —     | ferro-labs/ai-gateway  |
//! | SummonedGateway   | npm    | —     | @summoned/gateway      |
//! | pLLM              | Go bin | —     | andreimerfu/pllm       |
//! | LLMRouter         | pip    | —     | llmrouter              |
//!
//! Provides lifecycle management (start/stop), HTTP health checks,
//! and request routing through the active gateway.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::time::Instant;
use tokio::sync::RwLock;

// =============================================================================
// Gateway Types
// =============================================================================

/// Supported AI gateway backends.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum GatewayType {
    OmniRoute,
    Plexus,
    SmarterRouter,
    FreeRouter,
    LunarGate,
    FerroGateway,
    SummonedGateway,
    Pllm,
    LlmRouter,
}

impl GatewayType {
    /// Human-readable label.
    fn label(&self) -> &'static str {
        match self {
            Self::OmniRoute => "OmniRoute",
            Self::Plexus => "Plexus",
            Self::SmarterRouter => "SmarterRouter",
            Self::FreeRouter => "FreeRouter",
            Self::LunarGate => "LunarGate",
            Self::FerroGateway => "FerroGateway",
            Self::SummonedGateway => "SummonedGateway",
            Self::Pllm => "pLLM",
            Self::LlmRouter => "LLMRouter",
        }
    }

    /// Default port if the gateway exposes an HTTP endpoint.
    fn default_port(&self) -> u16 {
        match self {
            Self::OmniRoute => 8088,
            Self::Plexus => 8080,
            Self::SmarterRouter => 9090,
            Self::FreeRouter => 9091,
            Self::LunarGate => 8081,
            Self::FerroGateway => 8082,
            Self::SummonedGateway => 8083,
            Self::Pllm => 8084,
            Self::LlmRouter => 9092,
        }
    }

    /// Default endpoint URL (used when the gateway is reachable via HTTP).
    fn default_endpoint(&self) -> String {
        format!("http://localhost:{}", self.default_port())
    }

    /// Path for health checks.
    fn health_path(&self) -> &'static str {
        match self {
            Self::OmniRoute | Self::Plexus => "/health",
            Self::SmarterRouter => "/health",
            Self::FreeRouter => "/healthz",
            _ => "/health",
        }
    }

    /// Docker image name, if applicable.
    fn docker_image(&self) -> Option<&'static str> {
        match self {
            Self::OmniRoute => Some("diegosouzapw/omniroute"),
            Self::Plexus => Some("mcowger/plexus"),
            _ => None,
        }
    }

    /// Package name for pip/npm/go install.
    fn package_name(&self) -> &'static str {
        match self {
            Self::SmarterRouter => "smarter-router",
            Self::FreeRouter => "freerouter",
            Self::LunarGate => "github.com/lunargate-ai/gateway@latest",
            Self::FerroGateway => "github.com/ferro-labs/ai-gateway@latest",
            Self::SummonedGateway => "@summoned/gateway",
            Self::Pllm => "github.com/andreimerfu/pllm@latest",
            Self::LlmRouter => "llmrouter",
            Self::OmniRoute | Self::Plexus => "",
        }
    }

    /// Binary name after installation.
    fn binary_name(&self) -> &'static str {
        match self {
            Self::SmarterRouter => "smarter-router",
            Self::FreeRouter => "freerouter",
            Self::LunarGate => "gateway",
            Self::FerroGateway => "ai-gateway",
            Self::SummonedGateway => "summoned-gateway",
            Self::Pllm => "pllm",
            Self::LlmRouter => "llmrouter",
            Self::OmniRoute | Self::Plexus => "",
        }
    }
}

// =============================================================================
// Gateway Status
// =============================================================================

/// Runtime status of a gateway instance.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum GatewayStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

impl std::fmt::Display for GatewayStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stopped => write!(f, "Stopped"),
            Self::Starting => write!(f, "Starting"),
            Self::Running => write!(f, "Running"),
            Self::Error(e) => write!(f, "Error: {e}"),
        }
    }
}

// =============================================================================
// Gateway Instance
// =============================================================================

/// A single gateway instance with its full configuration and runtime state.
pub struct GatewayInstance {
    pub id: String,
    pub name: String,
    pub gateway_type: GatewayType,
    pub endpoint: String,
    pub port: u16,
    pub status: GatewayStatus,
    pub uptime_secs: u64,
    /// Docker container ID (if Docker-based).
    container_id: Option<String>,
    /// Handle to the spawned child process (if binary/npm/pip).
    child: Option<Child>,
    /// Timestamp when the gateway was started.
    started_at: Option<Instant>,
}

impl GatewayInstance {
    fn new(id: String, name: String, gateway_type: GatewayType) -> Self {
        let port = gateway_type.default_port();
        let endpoint = gateway_type.default_endpoint();
        Self {
            id,
            name,
            gateway_type,
            endpoint,
            port,
            status: GatewayStatus::Stopped,
            uptime_secs: 0,
            container_id: None,
            child: None,
            started_at: None,
        }
    }
}

// =============================================================================
// Public info struct (for Tauri serialisation)
// =============================================================================

/// Serializable summary of a gateway for the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct GatewayInfo {
    pub id: String,
    pub name: String,
    pub gateway_type: String,
    pub endpoint: String,
    pub port: u16,
    pub status: String,
    pub uptime_secs: u64,
    pub has_health_endpoint: bool,
}

// =============================================================================
// Gateway Manager
// =============================================================================

/// Manages the lifecycle of all configured AI gateway backends.
///
/// Thread-safe: uses `tokio::sync::RwLock` for interior mutability.
/// All public methods are `async` and can be called from Tauri commands.
#[allow(dead_code)]
pub struct GatewayManager {
    gateways: RwLock<HashMap<String, GatewayInstance>>,
    active_gateway: RwLock<Option<String>>,
}

impl Default for GatewayManager {
    fn default() -> Self {
        Self::new()
    }
}

impl GatewayManager {
    /// Create a new manager with all nine default gateway configurations.
    pub fn new() -> Self {
        let mut map = HashMap::new();

        // Docker gateways
        map.insert(
            "omniroute".into(),
            GatewayInstance::new(
                "omniroute".into(),
                "OmniRoute".into(),
                GatewayType::OmniRoute,
            ),
        );
        map.insert(
            "plexus".into(),
            GatewayInstance::new("plexus".into(), "Plexus".into(), GatewayType::Plexus),
        );

        // pip gateways
        map.insert(
            "smarter-router".into(),
            GatewayInstance::new(
                "smarter-router".into(),
                "SmarterRouter".into(),
                GatewayType::SmarterRouter,
            ),
        );
        map.insert(
            "freerouter".into(),
            GatewayInstance::new(
                "freerouter".into(),
                "FreeRouter".into(),
                GatewayType::FreeRouter,
            ),
        );
        map.insert(
            "llmrouter".into(),
            GatewayInstance::new(
                "llmrouter".into(),
                "LLMRouter".into(),
                GatewayType::LlmRouter,
            ),
        );

        // Go binary gateways
        map.insert(
            "lunargate".into(),
            GatewayInstance::new(
                "lunargate".into(),
                "LunarGate".into(),
                GatewayType::LunarGate,
            ),
        );
        map.insert(
            "ferro-gateway".into(),
            GatewayInstance::new(
                "ferro-gateway".into(),
                "Ferro Labs AI Gateway".into(),
                GatewayType::FerroGateway,
            ),
        );
        map.insert(
            "pllm".into(),
            GatewayInstance::new("pllm".into(), "pLLM".into(), GatewayType::Pllm),
        );

        // npm gateway
        map.insert(
            "summoned-gateway".into(),
            GatewayInstance::new(
                "summoned-gateway".into(),
                "Summoned Gateway".into(),
                GatewayType::SummonedGateway,
            ),
        );

        Self {
            gateways: RwLock::new(map),
            active_gateway: RwLock::new(None),
        }
    }

    // -------------------------------------------------------------------------
    // Lifecycle
    // -------------------------------------------------------------------------

    /// Start a gateway by its ID.
    ///
    /// For Docker-based gateways (OmniRoute, Plexus) this pulls & runs a container.
    /// For pip/npm/Go gateways it spawns the binary as a child process.
    pub async fn start_gateway(&self, id: &str) -> anyhow::Result<()> {
        let mut gateways = self.gateways.write().await;
        let gw = gateways
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Gateway not found: {id}"))?;

        gw.status = GatewayStatus::Starting;
        gw.started_at = Some(Instant::now());

        match gw.gateway_type {
            GatewayType::OmniRoute | GatewayType::Plexus => {
                let image = gw
                    .gateway_type
                    .docker_image()
                    .ok_or_else(|| anyhow::anyhow!("No Docker image for {:?}", gw.gateway_type))?;
                Self::start_docker(gw, image).await?;
            }
            GatewayType::SmarterRouter
            | GatewayType::FreeRouter
            | GatewayType::LlmRouter => {
                let bin = gw.gateway_type.binary_name();
                Self::start_binary(gw, bin, &[]).await?;
            }
            GatewayType::LunarGate | GatewayType::FerroGateway | GatewayType::Pllm => {
                let bin = gw.gateway_type.binary_name();
                Self::start_binary(gw, bin, &[]).await?;
            }
            GatewayType::SummonedGateway => {
                Self::start_npx(gw, &[]).await?;
            }
        }

        gw.status = GatewayStatus::Running;
        tracing::info!("Gateway '{}' started (port {})", gw.name, gw.port);
        Ok(())
    }

    /// Stop a running gateway.
    ///
    /// Kills the child process or stops the Docker container.
    pub async fn stop_gateway(&self, id: &str) -> anyhow::Result<()> {
        let mut gateways = self.gateways.write().await;
        let gw = gateways
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Gateway not found: {id}"))?;

        // Docker-based — stop the container
        if let Some(cid) = gw.container_id.take() {
            let output = Command::new("docker")
                .args(["stop", &cid])
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to stop Docker container {cid}: {e}"))?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                tracing::warn!("docker stop {cid} had issues: {stderr}");
            }
        }

        // Process-based — kill the child
        if let Some(mut child) = gw.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }

        gw.status = GatewayStatus::Stopped;
        gw.uptime_secs = gw
            .started_at
            .map(|t| t.elapsed().as_secs())
            .unwrap_or(0);
        gw.started_at = None;

        // Clear active gateway if this was the active one
        let mut active = self.active_gateway.write().await;
        if active.as_deref() == Some(id) {
            *active = None;
        }

        tracing::info!("Gateway '{}' stopped", gw.name);
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Health
    // -------------------------------------------------------------------------

    /// Perform an HTTP health check against the gateway.
    ///
    /// Returns `true` if the gateway responds with a 2xx status code,
    /// or if the process is still running for non-HTTP gateways.
    pub async fn health_check(&self, id: &str) -> bool {
        // Take a write lock so we can call `try_wait` on the child process.
        let mut gateways = self.gateways.write().await;
        let gw = match gateways.get_mut(id) {
            Some(g) => g,
            None => return false,
        };

        match &gw.status {
            GatewayStatus::Running | GatewayStatus::Starting => {}
            _ => return false,
        }

        // Try HTTP health endpoint first
        let health_url = format!("{}{}", gw.endpoint, gw.gateway_type.health_path());
        match reqwest::get(&health_url).await {
            Ok(resp) => {
                let healthy = resp.status().is_success();
                if !healthy {
                    tracing::debug!(
                        "Health check for '{}' returned {}",
                        gw.name,
                        resp.status()
                    );
                }
                healthy
            }
            Err(_) => {
                // Fallback: check if the child process is still alive
                if let Some(ref mut child) = gw.child {
                    match child.try_wait() {
                        Ok(None) => true,    // still running
                        Ok(Some(_)) => false, // exited
                        Err(_) => false,
                    }
                } else {
                    false
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Routing
    // -------------------------------------------------------------------------

    /// Proxy an AI request through the active gateway.
    ///
    /// Sends a POST to `{active_gateway_endpoint}/v1/chat/completions`
    /// with the provided JSON body and returns the gateway's response.
    ///
    /// Returns an error if no gateway is active or if the request fails.
    pub async fn route_through_gateway(&self, request: Value) -> anyhow::Result<Value> {
        let active_id = self
            .active_gateway
            .read()
            .await
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No active gateway. Start and select one first."))?;

        let gateways = self.gateways.read().await;
        let gw = gateways.get(&active_id).ok_or_else(|| {
            anyhow::anyhow!("Active gateway '{active_id}' not found in registry")
        })?;

        match &gw.status {
            GatewayStatus::Running => {}
            GatewayStatus::Starting => {
                anyhow::bail!("Gateway '{}' is still starting up", gw.name);
            }
            GatewayStatus::Stopped => {
                anyhow::bail!("Gateway '{}' is stopped", gw.name);
            }
            GatewayStatus::Error(e) => {
                anyhow::bail!("Gateway '{}' is in error state: {e}", gw.name);
            }
        }

        let url = format!("{}/v1/chat/completions", gw.endpoint);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()?;

        let response = client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Gateway request failed for '{}': {e}", gw.name))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!(
                "Gateway '{}' returned {status}: {body}",
                gw.name
            );
        }

        let json: Value = response
            .json()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to parse gateway response: {e}"))?;

        Ok(json)
    }

    /// Select the active gateway by ID. All subsequent `route_through_gateway`
    /// calls will use this gateway.
    pub async fn set_active_gateway(&self, id: &str) -> anyhow::Result<()> {
        let gateways = self.gateways.read().await;
        if !gateways.contains_key(id) {
            anyhow::bail!("Gateway not found: {id}");
        }
        drop(gateways);

        let mut active = self.active_gateway.write().await;
        *active = Some(id.to_string());
        tracing::info!("Active gateway set to '{id}'");
        Ok(())
    }

    /// Return the ID of the currently active gateway, if any.
    pub async fn get_active_gateway(&self) -> Option<String> {
        self.active_gateway.read().await.clone()
    }

    // -------------------------------------------------------------------------
    // Listing
    // -------------------------------------------------------------------------

    /// Return serialisable info for all configured gateways.
    pub async fn list_all(&self) -> Vec<GatewayInfo> {
        let gateways = self.gateways.read().await;
        let mut result: Vec<GatewayInfo> = gateways
            .values()
            .map(|gw| {
                let uptime = match &gw.status {
                    GatewayStatus::Running | GatewayStatus::Starting => gw
                        .started_at
                        .map(|t| t.elapsed().as_secs())
                        .unwrap_or(0),
                    _ => gw.uptime_secs,
                };
                GatewayInfo {
                    id: gw.id.clone(),
                    name: gw.name.clone(),
                    gateway_type: gw.gateway_type.label().to_string(),
                    endpoint: gw.endpoint.clone(),
                    port: gw.port,
                    status: gw.status.to_string(),
                    uptime_secs: uptime,
                    has_health_endpoint: matches!(
                        gw.gateway_type,
                        GatewayType::OmniRoute | GatewayType::Plexus
                    ),
                }
            })
            .collect();
        result.sort_by(|a, b| a.id.cmp(&b.id));
        result
    }

    /// Return info for a single gateway by ID.
    pub async fn get_info(&self, id: &str) -> Option<GatewayInfo> {
        let gateways = self.gateways.read().await;
        gateways.get(id).map(|gw| {
            let uptime = match &gw.status {
                GatewayStatus::Running | GatewayStatus::Starting => gw
                    .started_at
                    .map(|t| t.elapsed().as_secs())
                    .unwrap_or(0),
                _ => gw.uptime_secs,
            };
            GatewayInfo {
                id: gw.id.clone(),
                name: gw.name.clone(),
                gateway_type: gw.gateway_type.label().to_string(),
                endpoint: gw.endpoint.clone(),
                port: gw.port,
                status: gw.status.to_string(),
                uptime_secs: uptime,
                has_health_endpoint: matches!(
                    gw.gateway_type,
                    GatewayType::OmniRoute | GatewayType::Plexus
                ),
            }
        })
    }

    // -------------------------------------------------------------------------
    // Private helpers
    // -------------------------------------------------------------------------

    /// Start a Docker-based gateway.
    async fn start_docker(gw: &mut GatewayInstance, image: &str) -> anyhow::Result<()> {
        let output = Command::new("docker")
            .args([
                "run",
                "-d",
                "--rm",
                "-p",
                &format!("{}:{}", gw.port, gw.port),
                image,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run Docker for '{}': {e}", gw.name))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Docker run failed for '{}': {stderr}", gw.name);
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if container_id.is_empty() {
            anyhow::bail!("Docker run for '{}' returned empty container ID", gw.name);
        }

        gw.container_id = Some(container_id);
        Ok(())
    }

    /// Start a binary-based gateway (pip / Go binary).
    async fn start_binary(gw: &mut GatewayInstance, bin: &str, args: &[&str]) -> anyhow::Result<()> {
        let child = Command::new(bin)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to spawn '{}' for '{}': {e}. Is it installed?",
                    bin,
                    gw.name
                )
            })?;

        gw.child = Some(child);
        Ok(())
    }

    /// Start an npm-based gateway via npx.
    async fn start_npx(gw: &mut GatewayInstance, args: &[&str]) -> anyhow::Result<()> {
        let package = gw.gateway_type.package_name();
        let mut all_args = vec![package];
        all_args.extend_from_slice(args);

        let child = Command::new("npx")
            .args(&all_args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                anyhow::anyhow!(
                    "Failed to run npx for '{}': {e}. Is npm/npx installed?",
                    gw.name
                )
            })?;

        gw.child = Some(child);
        Ok(())
    }
}

// =============================================================================
// Tauri command
// =============================================================================

/// Tauri command: return serialisable info for all configured gateways.
///
/// Must return `Result` because Tauri v2 requires async commands with
/// reference inputs (like `State`) to return `Result`.
#[allow(dead_code)]
#[tauri::command]
pub async fn list_gateways(
    manager: tauri::State<'_, std::sync::Arc<GatewayManager>>,
) -> Result<Vec<GatewayInfo>, String> {
    Ok(manager.list_all().await)
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_creates_all_gateways() {
        let manager = GatewayManager::new();
        let gateways = manager.gateways.blocking_read();
        assert_eq!(gateways.len(), 9, "expected 9 default gateways");

        for id in &[
            "omniroute",
            "plexus",
            "smarter-router",
            "freerouter",
            "lunargate",
            "ferro-gateway",
            "summoned-gateway",
            "pllm",
            "llmrouter",
        ] {
            assert!(gateways.contains_key(*id), "missing gateway: {id}");
        }
    }

    #[test]
    fn test_all_gateways_initialise_as_stopped() {
        let manager = GatewayManager::new();
        let gateways = manager.gateways.blocking_read();
        for gw in gateways.values() {
            assert!(
                matches!(gw.status, GatewayStatus::Stopped),
                "gateway '{}' should be Stopped, got {:?}",
                gw.id,
                gw.status
            );
        }
    }

    #[test]
    fn test_gateway_type_labels() {
        assert_eq!(GatewayType::OmniRoute.label(), "OmniRoute");
        assert_eq!(GatewayType::Plexus.label(), "Plexus");
        assert_eq!(GatewayType::SmarterRouter.label(), "SmarterRouter");
        assert_eq!(GatewayType::FreeRouter.label(), "FreeRouter");
        assert_eq!(GatewayType::LunarGate.label(), "LunarGate");
        assert_eq!(GatewayType::FerroGateway.label(), "FerroGateway");
        assert_eq!(GatewayType::SummonedGateway.label(), "SummonedGateway");
        assert_eq!(GatewayType::Pllm.label(), "pLLM");
        assert_eq!(GatewayType::LlmRouter.label(), "LLMRouter");
    }

    #[test]
    fn test_docker_image_assignment() {
        assert_eq!(
            GatewayType::OmniRoute.docker_image(),
            Some("diegosouzapw/omniroute")
        );
        assert_eq!(GatewayType::Plexus.docker_image(), Some("mcowger/plexus"));
        assert_eq!(GatewayType::SmarterRouter.docker_image(), None);
    }

    #[test]
    fn test_default_ports() {
        assert_eq!(GatewayType::OmniRoute.default_port(), 8088);
        assert_eq!(GatewayType::Plexus.default_port(), 8080);
    }

    #[test]
    fn test_default_endpoint_format() {
        let ep = GatewayType::OmniRoute.default_endpoint();
        assert!(ep.contains("8088"));
        assert!(ep.starts_with("http://"));
    }

    #[test]
    fn test_gateway_info_serializable() {
        // Verify that GatewayInfo derives Serialize (compile-time check)
        fn assert_serialize<T: Serialize>() {}
        assert_serialize::<GatewayInfo>();
    }

    #[test]
    fn test_list_all_returns_sorted() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let list = rt.block_on(manager.list_all());
        assert_eq!(list.len(), 9);

        // Verify sorted by id
        for window in list.windows(2) {
            assert!(window[0].id <= window[1].id, "list not sorted");
        }

        // All start as Stopped
        for info in &list {
            assert_eq!(info.status, "Stopped");
        }
    }

    #[test]
    fn test_get_info_returns_none_for_unknown() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let info = rt.block_on(manager.get_info("nonexistent"));
        assert!(info.is_none());
    }

    #[test]
    fn test_get_info_returns_some_for_known() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let info = rt.block_on(manager.get_info("omniroute"));
        assert!(info.is_some());
        let info = info.unwrap();
        assert_eq!(info.id, "omniroute");
        assert_eq!(info.port, 8088);
    }

    #[test]
    fn test_health_check_returns_false_when_stopped() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let healthy = rt.block_on(manager.health_check("omniroute"));
        assert!(!healthy, "stopped gateway should not be healthy");
    }

    #[test]
    fn test_set_active_gateway_errors_on_unknown() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(manager.set_active_gateway("nope"));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_set_and_get_active_gateway() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        assert!(rt.block_on(manager.get_active_gateway()).is_none());

        rt.block_on(manager.set_active_gateway("plexus")).unwrap();
        let active = rt.block_on(manager.get_active_gateway());
        assert_eq!(active.as_deref(), Some("plexus"));
    }

    #[test]
    fn test_stop_clears_active() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(manager.set_active_gateway("omniroute")).unwrap();
        rt.block_on(manager.stop_gateway("omniroute")).unwrap();

        let active = rt.block_on(manager.get_active_gateway());
        assert!(active.is_none(), "active should clear after stop");
    }

    #[test]
    fn test_route_through_gateway_fails_with_no_active() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(manager.route_through_gateway(serde_json::json!({})));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("No active gateway"),
            "expected 'No active gateway', got: {err}"
        );
    }

    #[test]
    fn test_route_through_gateway_fails_when_stopped() {
        let manager = GatewayManager::new();
        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(manager.set_active_gateway("omniroute")).unwrap();
        // omniroute is still Stopped — routing should fail
        let result = rt.block_on(manager.route_through_gateway(serde_json::json!({})));
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("stopped") || err.contains("stopp"),
            "expected 'stopped' error, got: {err}"
        );
    }

    #[test]
    fn test_gateway_status_display() {
        assert_eq!(GatewayStatus::Stopped.to_string(), "Stopped");
        assert_eq!(GatewayStatus::Starting.to_string(), "Starting");
        assert_eq!(GatewayStatus::Running.to_string(), "Running");
        let err = GatewayStatus::Error("oops".into());
        assert!(err.to_string().contains("Error"));
        assert!(err.to_string().contains("oops"));
    }

    #[test]
    fn test_health_paths() {
        assert_eq!(GatewayType::OmniRoute.health_path(), "/health");
        assert_eq!(GatewayType::FreeRouter.health_path(), "/healthz");
    }

    #[test]
    fn test_package_names() {
        assert_eq!(
            GatewayType::SmarterRouter.package_name(),
            "smarter-router"
        );
        assert_eq!(
            GatewayType::SummonedGateway.package_name(),
            "@summoned/gateway"
        );
        assert_eq!(
            GatewayType::LunarGate.package_name(),
            "github.com/lunargate-ai/gateway@latest"
        );
        assert_eq!(
            GatewayType::Pllm.package_name(),
            "github.com/andreimerfu/pllm@latest"
        );
    }

    #[test]
    fn test_binary_names() {
        assert_eq!(GatewayType::LunarGate.binary_name(), "gateway");
        assert_eq!(GatewayType::FerroGateway.binary_name(), "ai-gateway");
        assert_eq!(GatewayType::SmarterRouter.binary_name(), "smarter-router");
        assert_eq!(GatewayType::OmniRoute.binary_name(), "");
    }
}
