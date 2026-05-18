//! Model Context Protocol server runtime. Manages 12 MCP servers (filesystem, git, terminal, browser, memory, search, docs, database, os, telegram, discord, whatsapp). Provides permission middleware, rate limiting, health checks, and a registry for server metadata.

pub mod browser;
pub mod database;
pub mod docs;
pub mod filesystem;
pub mod git;
pub mod memory;
pub mod os;
pub mod search;
pub mod discord;
pub mod telegram;
pub mod terminal;
pub mod whatsapp;

pub use crate::contracts::mcp::McpServer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

// =============================================================================
// Server Metadata & Status
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
    pub status: ServerStatus,
    pub uptime_secs: u64,
    pub requests_served: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServerStatus {
    Online,
    Offline,
    Degraded,
}

impl std::fmt::Display for ServerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerStatus::Online => write!(f, "online"),
            ServerStatus::Offline => write!(f, "offline"),
            ServerStatus::Degraded => write!(f, "degraded"),
        }
    }
}

// =============================================================================
// Server Info (for Tauri command responses)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub protocol: String,
    pub running: bool,
    pub connections: usize,
}

// =============================================================================
// Permission Middleware
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct PermissionConfig {
    /// Map of server_id -> list of allowed method names
    pub allowed_methods: HashMap<String, Vec<String>>,
    /// Global toggle for the permission layer
    pub enabled: bool,
}


pub struct PermissionMiddleware {
    config: RwLock<PermissionConfig>,
}

impl Default for PermissionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionMiddleware {
    pub fn new() -> Self {
        Self {
            config: RwLock::new(PermissionConfig::default()),
        }
    }

    pub async fn set_config(&self, config: PermissionConfig) {
        *self.config.write().await = config;
    }

    pub async fn is_allowed(&self, server_id: &str, method: &str) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return true;
        }
        config
            .allowed_methods
            .get(server_id)
            .map(|methods| methods.contains(&method.to_string()))
            .unwrap_or(false)
    }

    pub async fn restrict(&self, server_id: &str, methods: Vec<String>) {
        let mut config = self.config.write().await;
        config.enabled = true;
        config
            .allowed_methods
            .insert(server_id.to_string(), methods);
    }
}

// =============================================================================
// Rate Limiter
// =============================================================================

struct RateLimitState {
    timestamps: Vec<Instant>,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimitState {
    fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            timestamps: Vec::new(),
            max_requests,
            window_secs,
        }
    }

    fn check_and_record(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(self.window_secs);
        self.timestamps.retain(|&t| t > cutoff);
        if self.timestamps.len() >= self.max_requests as usize {
            false
        } else {
            self.timestamps.push(now);
            true
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub max_requests_per_window: u32,
    pub window_secs: u64,
    pub enabled: bool,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests_per_window: 60,
            window_secs: 60,
            enabled: false,
        }
    }
}

pub struct RateLimiter {
    states: RwLock<HashMap<String, RateLimitState>>,
    config: RwLock<RateLimitConfig>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            states: RwLock::new(HashMap::new()),
            config: RwLock::new(RateLimitConfig::default()),
        }
    }

    pub async fn set_config(&self, config: RateLimitConfig) {
        *self.config.write().await = config;
    }

    pub async fn check(&self, server_id: &str) -> bool {
        let config = self.config.read().await;
        if !config.enabled {
            return true;
        }
        let max = config.max_requests_per_window;
        let window = config.window_secs;
        drop(config);

        let mut states = self.states.write().await;
        let state = states
            .entry(server_id.to_string())
            .or_insert_with(|| RateLimitState::new(max, window));
        state.check_and_record()
    }
}

// =============================================================================
// MCP Registry — discovers, tracks, and monitors servers
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    pub health_checks_enabled: bool,
    pub health_check_interval_secs: u64,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            health_checks_enabled: true,
            health_check_interval_secs: 30,
        }
    }
}

pub struct McpRegistry {
    metadata: RwLock<HashMap<String, ServerMetadata>>,
    config: RwLock<RegistryConfig>,
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl McpRegistry {
    pub fn new() -> Self {
        Self {
            metadata: RwLock::new(HashMap::new()),
            config: RwLock::new(RegistryConfig::default()),
        }
    }

    pub async fn register(&self, id: &str, name: &str, capabilities: Vec<String>) {
        let mut meta = self.metadata.write().await;
        meta.insert(
            id.to_string(),
            ServerMetadata {
                id: id.to_string(),
                name: name.to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities,
                status: ServerStatus::Online,
                uptime_secs: 0,
                requests_served: 0,
                last_error: None,
            },
        );
    }

    pub async fn record_request(&self, id: &str) {
        let mut meta = self.metadata.write().await;
        if let Some(m) = meta.get_mut(id) {
            m.requests_served += 1;
        }
    }

    pub async fn record_error(&self, id: &str, error: &str) {
        let mut meta = self.metadata.write().await;
        if let Some(m) = meta.get_mut(id) {
            m.last_error = Some(error.to_string());
            m.status = ServerStatus::Degraded;
        }
    }

    pub async fn set_status(&self, id: &str, status: ServerStatus) {
        let mut meta = self.metadata.write().await;
        if let Some(m) = meta.get_mut(id) {
            m.status = status;
        }
    }

    pub async fn get_metadata(&self, id: &str) -> Option<ServerMetadata> {
        self.metadata.read().await.get(id).cloned()
    }

    pub async fn list_metadata(&self) -> Vec<ServerMetadata> {
        self.metadata.read().await.values().cloned().collect()
    }
}

// =============================================================================
// ServerManager — orchestrates all MCP servers
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ManagerStatus {
    Starting,
    Running,
    Stopped,
    Error(String),
}

pub struct ServerManager {
    servers: RwLock<HashMap<String, Arc<dyn McpServer + Send + Sync>>>,
    permission: PermissionMiddleware,
    rate_limiter: RateLimiter,
    registry: McpRegistry,
    status: RwLock<ManagerStatus>,
    configs: RwLock<HashMap<String, String>>,
}

impl Default for ServerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerManager {
    pub fn new() -> Self {
        Self {
            servers: RwLock::new(HashMap::new()),
            permission: PermissionMiddleware::new(),
            rate_limiter: RateLimiter::new(),
            registry: McpRegistry::new(),
            status: RwLock::new(ManagerStatus::Starting),
            configs: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new MCP server and its metadata in the registry.
    pub async fn register(&self, server: Arc<dyn McpServer + Send + Sync>) {
        let id = server.id().to_string();
        let name = server.name().to_string();
        self.servers.write().await.insert(id.clone(), server);

        self.registry
            .register(&id, &name, vec!["default".to_string()])
            .await;
    }

    pub async fn start(&self) -> anyhow::Result<()> {
        let servers = self.servers.read().await;
        for (_id, server) in servers.iter() {
            server.start().await?;
        }
        *self.status.write().await = ManagerStatus::Running;
        tracing::info!("All MCP servers started");
        Ok(())
    }

    pub async fn start_server(&self, id: &str) -> anyhow::Result<()> {
        let servers = self.servers.read().await;
        let server = servers.get(id).ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", id))?;
        server.start().await?;
        tracing::info!("MCP server '{}' started", id);
        Ok(())
    }

    pub async fn stop_server(&self, id: &str) -> anyhow::Result<()> {
        let servers = self.servers.read().await;
        let server = servers.get(id).ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", id))?;
        server.stop().await?;
        tracing::info!("MCP server '{}' stopped", id);
        Ok(())
    }

    pub async fn stop_all(&self) -> anyhow::Result<()> {
        let servers = self.servers.read().await;
        for (_id, server) in servers.iter() {
            server.stop().await?;
        }
        *self.status.write().await = ManagerStatus::Stopped;
        Ok(())
    }

    /// Start background health checks. Requires the ServerManager to be wrapped
    /// in an Arc so the spawned task can hold a reference.
    pub fn start_health_checks(self: &Arc<Self>) {
        let manager = Arc::clone(self);
        tokio::spawn(async move {
            manager.health_check_loop().await;
        });
    }

    async fn health_check_loop(&self) {
        let interval = {
            let config = self.registry.config.read().await;
            if !config.health_checks_enabled {
                return;
            }
            std::time::Duration::from_secs(config.health_check_interval_secs)
        };

        loop {
            tokio::time::sleep(interval).await;
            let servers = self.servers.read().await;
            for (id, server) in servers.iter() {
                let status = match server.is_running().await {
                    true => ServerStatus::Online,
                    false => ServerStatus::Offline,
                };
                let mut meta = self.registry.metadata.write().await;
                if let Some(m) = meta.get_mut(id) {
                    m.status = status.clone();
                }
                tracing::debug!("Health check for MCP '{}': {}", id, status);
            }
        }
    }

    pub async fn call(
        &self,
        server_id: &str,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        // Permission check
        if !self.permission.is_allowed(server_id, method).await {
            return Err(anyhow::anyhow!(
                "Method '{}' not allowed on server '{}'",
                method,
                server_id
            ));
        }

        // Rate limit check
        if !self.rate_limiter.check(server_id).await {
            return Err(anyhow::anyhow!(
                "Rate limit exceeded for server '{}'",
                server_id
            ));
        }

        let servers = self.servers.read().await;
        let server = servers
            .get(server_id)
            .ok_or_else(|| anyhow::anyhow!("MCP server not found: {}", server_id))?;

        match server.handle_request(method, params).await {
            Ok(result) => {
                self.registry.record_request(server_id).await;
                Ok(result)
            }
            Err(e) => {
                self.registry.record_error(server_id, &e.to_string()).await;
                Err(e)
            }
        }
    }

    pub async fn list_servers(&self) -> Vec<ServerInfo> {
        let server_ids: Vec<String> = {
            let servers = self.servers.read().await;
            servers.keys().cloned().collect()
        };

        let mut result = Vec::new();
        for id in server_ids {
            let meta = self.registry.get_metadata(&id).await;
            result.push(ServerInfo {
                id: id.clone(),
                name: id.clone(),
                version: meta
                    .as_ref()
                    .map(|m| m.version.clone())
                    .unwrap_or_else(|| "0.1.0".to_string()),
                protocol: "mcp".to_string(),
                running: true,
                connections: 0,
            });
        }

        result
    }

    pub fn permission(&self) -> &PermissionMiddleware {
        &self.permission
    }

    pub fn rate_limiter(&self) -> &RateLimiter {
        &self.rate_limiter
    }

    pub fn registry(&self) -> &McpRegistry {
        &self.registry
    }

    pub async fn manager_status(&self) -> ManagerStatus {
        self.status.read().await.clone()
    }

    pub async fn add_config(&self, id: String, config: String) {
        self.configs.write().await.insert(id, config);
    }

    pub async fn remove_config(&self, id: &str) -> Option<String> {
        self.configs.write().await.remove(id)
    }

    pub async fn list_configs(&self) -> HashMap<String, String> {
        self.configs.read().await.clone()
    }
}
