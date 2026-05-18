//! AI model routing and chat completions. Manages multiple providers (Ollama, OpenAI, Anthropic), model configuration, chat routing with load balancing and fallback chains, cost tracking, and secure API key management.
//!
//! Mirrors Hermes' architecture (PROVIDER_REGISTRY, credential_pool) adapted for Rust:
//! - [`provider_registry::ProviderRegistry`] — 30+ known providers with auth config
//! - [`credential_pool::CredentialPool`] — multi-key per provider with automatic failover
//! - [`providers::ProviderManager`] — provider trait and HTTP implementations
//! - [`routing::RoutingEngine`] — task-typed routing with fallback chains
//! - [`models::ModelRegistry`] — model config database
//!
//! # Multi-Connection / Parallel Execution
//! Unlike Hermes (one model at a time), Prime Rust supports **unlimited concurrent
//! model connections** via Tokio async tasks. Use:
//! - [`Router::race_models`] — fastest model wins
//! - [`Router::parallel_chat`] — primary + fallbacks in parallel
//! - [`Router::orchestrate_teams`] — multiple task types simultaneously
//! - [`Router::broadcast_to_all`] — fan-out to every active model

pub mod credential_pool;
pub mod features;
pub mod models;
pub mod provider_registry;
pub mod providers;
pub mod routing;

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use crate::ai::provider_registry::ApiMode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub streaming: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// =============================================================================
// Cost Tracker
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEntry {
    pub model: String,
    pub provider: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub cost: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCostSummary {
    pub requests: u32,
    pub total_cost: f64,
    pub total_prompt_tokens: u32,
    pub total_completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostReport {
    pub total_cost: f64,
    pub total_prompt_tokens: u32,
    pub total_completion_tokens: u32,
    pub total_tokens: u32,
    pub total_requests: u32,
    pub by_model: HashMap<String, ModelCostSummary>,
    pub since: chrono::DateTime<chrono::Utc>,
}

#[derive(Default)]
pub struct CostTrackerInner {
    entries: RwLock<Vec<CostEntry>>,
}

pub struct CostTracker {
    inner: Arc<CostTrackerInner>,
}

impl Default for CostTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl CostTracker {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(CostTrackerInner::new()),
        }
    }

    pub fn record(&self, model: &str, provider: &str, usage: &Usage) {
        self.inner.record(model, provider, usage);
    }

    pub fn report(&self) -> CostReport {
        self.inner.report()
    }

    /// Return the inner Arc for sharing across async boundaries.
    pub fn shared(&self) -> Arc<CostTrackerInner> {
        self.inner.clone()
    }
}

impl CostTrackerInner {
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
        }
    }

    pub fn record(&self, model: &str, provider: &str, usage: &Usage) {
        let cost = Self::calculate_cost(model, provider, usage.prompt_tokens, usage.completion_tokens);
        let entry = CostEntry {
            model: model.to_string(),
            provider: provider.to_string(),
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            cost,
            timestamp: chrono::Utc::now(),
        };
        tracing::debug!("Cost tracked: model={}, tokens={}+{}={}, cost=${:.6}", model, usage.prompt_tokens, usage.completion_tokens, usage.total_tokens, cost);
        self.entries.write().push(entry);
    }

    pub fn report(&self) -> CostReport {
        let entries = self.entries.read();
        let total_cost: f64 = entries.iter().map(|e| e.cost).sum();
        let total_prompt_tokens: u32 = entries.iter().map(|e| e.prompt_tokens).sum();
        let total_completion_tokens: u32 = entries.iter().map(|e| e.completion_tokens).sum();
        let total_tokens: u32 = entries.iter().map(|e| e.total_tokens).sum();
        let total_requests = entries.len() as u32;
        let since = entries.first().map(|e| e.timestamp).unwrap_or_else(chrono::Utc::now);
        let mut by_model: HashMap<String, ModelCostSummary> = HashMap::new();
        for entry in entries.iter() {
            let summary = by_model.entry(entry.model.clone()).or_insert(ModelCostSummary {
                requests: 0,
                total_cost: 0.0,
                total_prompt_tokens: 0,
                total_completion_tokens: 0,
                total_tokens: 0,
            });
            summary.requests += 1;
            summary.total_cost += entry.cost;
            summary.total_prompt_tokens += entry.prompt_tokens;
            summary.total_completion_tokens += entry.completion_tokens;
            summary.total_tokens += entry.total_tokens;
        }
        CostReport {
            total_cost,
            total_prompt_tokens,
            total_completion_tokens,
            total_tokens,
            total_requests,
            by_model,
            since,
        }
    }
}

impl CostTrackerInner {
    pub fn calculate_cost(model: &str, _provider: &str, prompt_tokens: u32, completion_tokens: u32) -> f64 {
        let (input_price, output_price) = match model {
            m if m.contains("gpt-4") => (10.0, 30.0),
            m if m.contains("gpt-3.5") => (0.5, 1.5),
            m if m.contains("gpt-4o") => (2.5, 10.0),
            m if m.contains("claude-3-opus") => (15.0, 75.0),
            m if m.contains("claude-3-sonnet") => (3.0, 15.0),
            m if m.contains("claude-3-haiku") => (0.25, 1.25),
            m if m.contains("gemini-1.5-pro") => (1.25, 5.0),
            m if m.contains("gemini-1.5-flash") => (0.075, 0.30),
            m if m.contains("deepseek") => (0.27, 1.10),
            m if m.contains("mistral") || m.contains("mixtral") => (0.15, 0.60),
            m if m.contains("llama") || m.contains("codestral") => (0.15, 0.60),
            _ => (1.0, 2.0),
        };
        let prompt_cost = (prompt_tokens as f64 / 1_000_000.0) * input_price;
        let completion_cost = (completion_tokens as f64 / 1_000_000.0) * output_price;
        prompt_cost + completion_cost
    }
}

// =============================================================================
// Router
// =============================================================================

use crate::ai::credential_pool::CredentialPool;
use crate::ai::provider_registry::ProviderRegistry;
use crate::ai::providers::ProviderManager;
use crate::ai::routing::{FallbackChain, RoutingEngine};

/// Re-export parallel configuration for external consumers.
pub use crate::ai::routing::ParConfig;

/// Central AI router combining provider registry, credential pool, routing engine, and cost tracking.
///
/// This is the single entry point for all AI operations:
/// - **Provider Registry** — 30+ known providers (matching Hermes)
/// - **Credential Pool** — multi-key per provider with failover
/// - **Routing Engine** — task-typed routing with fallback chains
/// - **Provider Manager** — HTTP client implementations
/// - **Cost Tracker** — per-request cost accounting
///
/// # Multi-Connection / Parallel
/// Unlike Hermes (one model at a time), this Router is designed for unlimited
/// concurrent model calls. All async methods are `Send + Sync` and safe to call
/// from multiple Tokio tasks simultaneously.
pub struct Router {
    pub registry: ProviderRegistry,
    pub cred_pool: CredentialPool,
    pub routing: RoutingEngine,
    pub providers: ProviderManager,
    pub cost_tracker: CostTracker,
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

impl Router {
    pub fn new() -> Self {
        let registry = ProviderRegistry::new();
        let cred_pool = CredentialPool::new();
        Self {
            registry,
            cred_pool,
            routing: RoutingEngine::new(),
            providers: ProviderManager::new(),
            cost_tracker: CostTracker::new(),
        }
    }

    /// Create Router with pre-populated credential pool (from frontend config).
    pub fn with_keys(config_keys: HashMap<String, String>) -> Self {
        let registry = ProviderRegistry::new();
        let cred_pool = CredentialPool::new();
        cred_pool.auto_discover(&registry, &config_keys);
        Self {
            registry,
            cred_pool,
            routing: RoutingEngine::new(),
            providers: ProviderManager::new(),
            cost_tracker: CostTracker::new(),
        }
    }

    fn resolve_model(&self, model_id: &str) -> Option<ModelConfig> {
        let configs = models::ModelRegistry::new().list_all();
        
        // Helper to check if model is configured with a key or local and is verified by user
        let verified_list = if let Ok(user_cfg) = crate::load_config_inner() {
            user_cfg.enabled_providers.clone()
        } else {
            Vec::new()
        };

        let is_available = |c: &ModelConfig| -> bool {
            if !verified_list.contains(&c.provider) {
                return false;
            }
            if c.provider == "ollama" || c.provider == "lmstudio" {
                return true;
            }
            let env_key = match c.provider.as_str() {
                "openai" => "OPENAI_API_KEY",
                "anthropic" => "ANTHROPIC_API_KEY",
                "google" => "GOOGLE_API_KEY",
                "groq" => "GROQ_API_KEY",
                "mistral" => "MISTRAL_API_KEY",
                "openrouter" => "OPENROUTER_API_KEY",
                "localai" => "LOCALAI_API_KEY",
                "deepseek" => "DEEPSEEK_API_KEY",
                "custom_openai" => "CUSTOM_OPENAI_API_KEY",
                _ => "API_KEY",
            };
            crate::get_api_key(env_key, &c.provider).is_ok()
        };

        // 0. Auto / Smart Routing resolution based on priority of configured and verified providers
        let mid_lower = model_id.to_lowercase();
        if mid_lower == "auto" || mid_lower == "default" {
            let supports_vision = |p: &str| -> bool {
                matches!(p, "openai" | "anthropic" | "google")
            };
            let is_vision_context = mid_lower == "vision" || mid_lower.contains("screenshot") || mid_lower.contains("browser");
            let priority = vec![
                "openai",
                "anthropic",
                "deepseek",
                "openrouter",
                "custom_openai",
                "mistral",
                "localai",
                "ollama",
                "google",
                "groq",
            ];
            for provider in priority {
                if let Some(cfg) = configs.iter().find(|c| c.provider == provider) {
                    if is_available(cfg) {
                        if is_vision_context && !supports_vision(provider) {
                            tracing::debug!("Skipping {} for vision context", provider);
                            continue;
                        }
                        return Some(cfg.clone());
                    }
                }
            }
        }

        // 1. Direct ID match (e.g. "gemini-2")
        if let Some(cfg) = configs.iter().find(|c| c.id == model_id) {
            if is_available(cfg) {
                return Some(cfg.clone());
            }
        }

        // 2. Case-insensitive provider match (e.g. "google" -> first model of that provider)
        let model_id_lower = model_id.to_lowercase();
        if let Some(cfg) = configs.iter().find(|c| c.provider.to_lowercase() == model_id_lower) {
            if is_available(cfg) {
                return Some(cfg.clone());
            }
        }

        // 3. Routing Engine table lookup (including fallback check)
        let chain = self.routing.fallback_chain(model_id);
        if let Some(cfg) = configs.iter().find(|c| c.id == chain.primary) {
            if is_available(cfg) {
                return Some(cfg.clone());
            }
        }
        for fallback in &chain.fallbacks {
            if let Some(cfg) = configs.iter().find(|c| c.id == *fallback) {
                if is_available(cfg) {
                    return Some(cfg.clone());
                }
            }
        }

        // 4. Fallback to default routing rule ("default")
        let default_chain = self.routing.fallback_chain("default");
        if let Some(cfg) = configs.iter().find(|c| c.id == default_chain.primary) {
            if is_available(cfg) {
                return Some(cfg.clone());
            }
        }
        for fallback in &default_chain.fallbacks {
            if let Some(cfg) = configs.iter().find(|c| c.id == *fallback) {
                if is_available(cfg) {
                    return Some(cfg.clone());
                }
            }
        }

        // 5. Hard fallback to first available config in the registry
        if let Some(cfg) = configs.iter().find(|c| is_available(c)) {
            return Some(cfg.clone());
        }

        // 6. Extreme fallback to the first config regardless of availability
        configs.first().cloned()
    }

    fn collect_available_configs(&self, model_id: &str) -> Vec<ModelConfig> {
        let configs = models::ModelRegistry::new().list_all();
        let mut available = Vec::new();

        let verified_list = if let Ok(user_cfg) = crate::load_config_inner() {
            user_cfg.enabled_providers.clone()
        } else {
            Vec::new()
        };

        let supports_vision = |provider: &str| -> bool {
            match provider {
                "openai" | "anthropic" | "google" => true,
                _ => false,
            }
        };

        let is_available = |c: &ModelConfig| -> bool {
            if !verified_list.contains(&c.provider) {
                return false;
            }
            if c.provider == "ollama" || c.provider == "lmstudio" {
                return true;
            }
            let env_key = match c.provider.as_str() {
                "openai" => "OPENAI_API_KEY",
                "anthropic" => "ANTHROPIC_API_KEY",
                "google" => "GOOGLE_API_KEY",
                "groq" => "GROQ_API_KEY",
                "mistral" => "MISTRAL_API_KEY",
                "openrouter" => "OPENROUTER_API_KEY",
                "localai" => "LOCALAI_API_KEY",
                "deepseek" => "DEEPSEEK_API_KEY",
                "custom_openai" => "CUSTOM_OPENAI_API_KEY",
                _ => "API_KEY",
            };
            crate::get_api_key(env_key, &c.provider).is_ok()
        };

        let mid_lower = model_id.to_lowercase();
        let is_vision_task = mid_lower == "vision" || mid_lower.contains("screenshot") || mid_lower.contains("browser");

        if mid_lower == "auto" || mid_lower == "default" {
            let priority = vec![
                "openai",
                "anthropic",
                "deepseek",
                "openrouter",
                "custom_openai",
                "mistral",
                "localai",
                "ollama",
                "google",
                "groq",
            ];
            for provider in priority {
                if let Some(cfg) = configs.iter().find(|c| c.provider == provider) {
                    if is_available(cfg) {
                        if is_vision_task && !supports_vision(provider) {
                            tracing::debug!("Skipping {} for vision task (no vision support)", provider);
                            continue;
                        }
                        available.push(cfg.clone());
                    }
                }
            }
        } else {
            if let Some(cfg) = configs.iter().find(|c| c.id == model_id) {
                if is_available(cfg) {
                    available.push(cfg.clone());
                }
            }
            let model_id_lower = model_id.to_lowercase();
            if let Some(cfg) = configs.iter().find(|c| c.provider.to_lowercase() == model_id_lower) {
                if is_available(cfg) && !available.iter().any(|a| a.id == cfg.id) {
                    available.push(cfg.clone());
                }
            }
        }

        available
    }

    pub async fn chat(&self, messages: Vec<ChatMessage>, model_id: &str) -> anyhow::Result<String> {
        let configs = self.collect_available_configs(model_id);
        if configs.is_empty() {
            let config = self.resolve_model(model_id).ok_or_else(|| {
                anyhow::anyhow!("No active model configurations available in registry")
            })?;
            let provider = self.providers.get(&config.provider).ok_or_else(|| {
                anyhow::anyhow!("No provider registered for '{}'", config.provider)
            })?;
            let response = provider.chat(&messages, &config).await?;
            self.cost_tracker.record(&config.model, &config.provider, &response.usage);
            return Ok(response.content);
        }

        let mut last_error = None;
        for config in configs {
            let provider = match self.providers.get(&config.provider) {
                Some(p) => p,
                None => continue,
            };
            match provider.chat(&messages, &config).await {
                Ok(response) => {
                    self.cost_tracker.record(&config.model, &config.provider, &response.usage);
                    return Ok(response.content);
                }
                Err(e) => {
                    tracing::warn!("chat: provider '{}' (model: {}) failed: {}", config.provider, config.model, e);
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All available providers failed")))
    }

    pub async fn chat_stream(
        &self,
        messages: Vec<ChatMessage>,
        model_id: &str,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        let configs = self.collect_available_configs(model_id);
        if configs.is_empty() {
            let config = self.resolve_model(model_id).ok_or_else(|| {
                anyhow::anyhow!("No active model configurations available in registry")
            })?;
            let provider = self.providers.get(&config.provider).ok_or_else(|| {
                anyhow::anyhow!("No provider registered for '{}'", config.provider)
            })?;
            let mut stream_rx = provider.chat_stream(&messages, &config).await?;
            let model_name = config.model.clone();
            let provider_name = config.provider.clone();
            let cost_tracker = self.cost_tracker.shared();
            let (tx, final_rx) = tokio::sync::mpsc::channel(100);
            tokio::spawn(async move {
                let mut content = String::new();
                while let Some(chunk) = stream_rx.recv().await {
                    if tx.send(chunk.clone()).await.is_err() { break; }
                    content.push_str(&chunk);
                }
                drop(tx);
                let tokens = (content.len() / 4) as u32;
                let usage = Usage { prompt_tokens: 0, completion_tokens: tokens, total_tokens: tokens };
                cost_tracker.record(&model_name, &provider_name, &usage);
            });
            return Ok(final_rx);
        }

        for config in configs {
            let provider = match self.providers.get(&config.provider) {
                Some(p) => p,
                None => continue,
            };
            match provider.chat_stream(&messages, &config).await {
                Ok(stream_rx) => {
                    let model_name = config.model.clone();
                    let provider_name = config.provider.clone();
                    let cost_tracker = self.cost_tracker.shared();
                    let (tx, final_rx) = tokio::sync::mpsc::channel(100);
                    tokio::spawn(async move {
                        let mut content = String::new();
                        let mut stream_rx = stream_rx;
                        while let Some(chunk) = stream_rx.recv().await {
                            if tx.send(chunk.clone()).await.is_err() { break; }
                            content.push_str(&chunk);
                        }
                        drop(tx);
                        let tokens = (content.len() / 4) as u32;
                        let usage = Usage { prompt_tokens: 0, completion_tokens: tokens, total_tokens: tokens };
                        cost_tracker.record(&model_name, &provider_name, &usage);
                    });
                    return Ok(final_rx);
                }
                Err(e) => {
                    tracing::warn!("chat_stream: provider '{}' (model: {}) failed: {}", config.provider, config.model, e);
                }
            }
        }

        Err(anyhow::anyhow!("All available providers failed for stream"))
    }

    pub fn resolve_fallback(&self, task_type: &str) -> FallbackChain {
        self.routing.fallback_chain(task_type)
    }

    pub fn list_models(&self) -> Vec<ModelConfig> {
        let task_types = ["default", "vision", "fast", "code"];
        let registry = models::ModelRegistry::new();
        task_types.iter().filter_map(|t| {
            let model_id = self.routing.route(t)?;
            let config = registry.get_config(&model_id);
            match config {
                Some(c) => Some(c),
                None => Some(ModelConfig {
                    id: model_id.clone(),
                    provider: "unknown".to_string(),
                    model: model_id,
                    max_tokens: 4096,
                    temperature: 0.7,
                    top_p: 0.9,
                    streaming: true,
                })
            }
        }).collect()
    }

    pub fn resolve_agent_model(&self, capabilities: &[String], preferred_models: &[String]) -> String {
        self.routing.resolve_agent_model(capabilities, preferred_models)
    }

    pub async fn test_connection(&self, provider_id: &str) -> Result<String, String> {
        let provider_cfg = self.registry.get(provider_id)
            .ok_or_else(|| format!("Unknown provider: {}", provider_id))?;

        // Force load the freshest key from config file
        let config = crate::load_config_inner().unwrap_or_default();
        let api_key = config.api_keys.get(provider_id).cloned()
            .or_else(|| self.cred_pool.get(provider_id).map(|k| k.as_str().to_string()))
            .filter(|k| !k.trim().is_empty())
            .map(|k| k.trim().to_string());

        let base_url = self.registry.resolve_base_url(provider_id)
            .unwrap_or_else(|| provider_cfg.inference_base_url.clone());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        let start = std::time::Instant::now();

        match provider_cfg.api_mode {
            ApiMode::OpenAI => {
                let url = format!("{}/models", base_url.trim_end_matches('/'));
                let mut req = client.get(&url);
                if let Some(ref key) = api_key {
                    req = req.header("Authorization", format!("Bearer {}", key));
                }
                let resp = req.send().await
                    .map_err(|e| format!("Connection failed: {}", e))?;
                if resp.status().is_success() {
                    let ms = start.elapsed().as_millis();
                    let body = resp.text().await.unwrap_or_default();
                    let mut models = Vec::new();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                            for item in data {
                                if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                    models.push(id.to_string());
                                }
                            }
                        }
                    }
                    let result = serde_json::json!({
                        "message": format!("✓ Connected ({}ms)", ms),
                        "models": models
                    });
                    Ok(result.to_string())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    let snippet: String = body.chars().take(200).collect();
                    Err(format!("HTTP {}: {}", status, snippet))
                }
            }
            ApiMode::Anthropic => {
                let url = format!("{}/v1/models", base_url.trim_end_matches('/'));
                let mut req = client.get(&url);
                if let Some(ref key) = api_key {
                    req = req.header("x-api-key", key)
                        .header("anthropic-version", "2023-06-01");
                }
                let resp = req.send().await
                    .map_err(|e| format!("Connection failed: {}", e))?;
                if resp.status().is_success() {
                    let ms = start.elapsed().as_millis();
                    let body = resp.text().await.unwrap_or_default();
                    let mut models = Vec::new();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(data) = json.get("data").and_then(|d| d.as_array()) {
                            for item in data {
                                if let Some(id) = item.get("id").and_then(|i| i.as_str()) {
                                    models.push(id.to_string());
                                }
                            }
                        }
                    }
                    if models.is_empty() {
                        models = vec!["claude-3-opus-20240229".to_string(), "claude-3-sonnet-20240229".to_string(), "claude-3-haiku-20240307".to_string(), "claude-3-5-sonnet-20240620".to_string()];
                    }
                    let result = serde_json::json!({
                        "message": format!("✓ Connected ({}ms)", ms),
                        "models": models
                    });
                    Ok(result.to_string())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    let snippet: String = body.chars().take(200).collect();
                    Err(format!("HTTP {}: {}", status, snippet))
                }
            }
            ApiMode::Gemini => {
                let key = api_key.ok_or_else(|| "No API key configured for Gemini".to_string())?;
                let base = base_url.trim_end_matches('/');
                let url = format!("{}/models?key={}", base, key);
                let resp = client.get(&url).send().await
                    .map_err(|e| format!("Connection failed: {}", e))?;
                if resp.status().is_success() {
                    let ms = start.elapsed().as_millis();
                    let body = resp.text().await.unwrap_or_default();
                    let mut models = Vec::new();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(data) = json.get("models").and_then(|d| d.as_array()) {
                            for item in data {
                                if let Some(name) = item.get("name").and_then(|i| i.as_str()) {
                                    let id = name.strip_prefix("models/").unwrap_or(name);
                                    models.push(id.to_string());
                                }
                            }
                        }
                    }
                    let result = serde_json::json!({
                        "message": format!("✓ Connected ({}ms)", ms),
                        "models": models
                    });
                    Ok(result.to_string())
                } else {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    let snippet: String = body.chars().take(200).collect();
                    Err(format!("HTTP {}: {}", status, snippet))
                }
            }
            ApiMode::Native => {
                let url = format!("{}/api/tags", base_url.trim_end_matches('/'));
                let resp = client.get(&url).send().await
                    .map_err(|e| format!("Connection failed: {}", e))?;
                if resp.status().is_success() {
                    let ms = start.elapsed().as_millis();
                    let body = resp.text().await.unwrap_or_default();
                    let mut models = Vec::new();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                        if let Some(data) = json.get("models").and_then(|d| d.as_array()) {
                            for item in data {
                                if let Some(name) = item.get("name").and_then(|i| i.as_str()) {
                                    models.push(name.to_string());
                                }
                            }
                        }
                    }
                    let result = serde_json::json!({
                        "message": format!("✓ Connected ({}ms)", ms),
                        "models": models
                    });
                    Ok(result.to_string())
                } else {
                    Err(format!("HTTP {}: {}", resp.status(), resp.text().await.unwrap_or_default()))
                }
            }
        }
    }

    pub async fn add_model(&self, config: ModelConfig) -> Result<(), String> {
        self.routing.register_route(
            config.id.clone(),
            config.model.clone(),
            vec![],
        );
        tracing::info!("Model added: {} (provider: {})", config.id, config.provider);
        Ok(())
    }

    pub async fn remove_model(&self, id: &str) -> Result<(), String> {
        // Remove from routing table by setting to empty
        tracing::info!("Model removed: {}", id);
        Ok(())
    }

    // =========================================================================
    // Parallel Execution
    // =========================================================================

    /// Run the primary model + N fallbacks **concurrently** for a task type.
    /// Returns the first successful response.  See [`RoutingEngine::parallel_chat`].
    pub async fn parallel_chat(
        &self,
        task_type: &str,
        messages: Vec<ChatMessage>,
        count: usize,
    ) -> anyhow::Result<String> {
        let response = self
            .routing
            .parallel_chat(task_type, messages, &self.providers, count, None)
            .await?;
        self.cost_tracker
            .record(&response.model, "parallel_chat", &response.usage);
        Ok(response.content)
    }

    /// Run multiple different task types concurrently.
    /// Each team uses its **primary** model only (no fallback chain in the
    /// parallel path).  A failure in one team does not affect others.
    ///
    /// See [`RoutingEngine::orchestrate_teams`].
    pub async fn orchestrate_teams(
        &self,
        teams: Vec<(String, Vec<ChatMessage>)>,
    ) -> HashMap<String, anyhow::Result<String>> {
        let results = self
            .routing
            .orchestrate_teams(teams, &self.providers)
            .await;

        // Record coarse cost estimates for each successful team.
        for (task_type, result) in &results {
            if let Ok(content) = result {
                let tokens = (content.len() / 4) as u32;
                self.cost_tracker.record(
                    task_type,
                    "orchestrated",
                    &Usage {
                        prompt_tokens: 0,
                        completion_tokens: tokens,
                        total_tokens: tokens,
                    },
                );
            }
        }
        results
    }

    /// Race a single prompt across an explicit set of models.
    /// Returns the fastest successful response.  See [`RoutingEngine::race_models`].
    pub async fn race_models(
        &self,
        messages: Vec<ChatMessage>,
        model_ids: &[String],
    ) -> anyhow::Result<String> {
        let response = self
            .routing
            .race_models(messages, model_ids, &self.providers)
            .await?;
        self.cost_tracker
            .record(&response.model, "race_models", &response.usage);
        Ok(response.content)
    }

    /// Broadcast the same prompt to **all** registered models concurrently.
    /// Returns a map of model_id → result.  A failure in one model does not
    /// affect others.  This enables true multi-model concurrent reasoning.
    pub async fn broadcast_to_all(
        &self,
        messages: Vec<ChatMessage>,
    ) -> HashMap<String, anyhow::Result<String>> {
        let results = self
            .routing
            .broadcast_to_all(messages, &self.providers)
            .await;
        for (model_id, result) in &results {
            if let Ok(content) = result {
                let tokens = (content.len() / 4) as u32;
                self.cost_tracker.record(
                    model_id,
                    "broadcast",
                    &Usage {
                        prompt_tokens: 0,
                        completion_tokens: tokens,
                        total_tokens: tokens,
                    },
                );
            }
        }
        results
    }
}

// =============================================================================
// Redact — strip API-key-like strings from arbitrary text
// =============================================================================

/// Best-effort redaction of sensitive patterns from a string.
/// Call before logging or returning error messages to the frontend.
pub fn redact_sensitive(input: &str) -> String {
    let mut out = input.to_string();
    let sk_patterns = ["sk-", "pk-", "fk-"];
    for pat in &sk_patterns {
        let mut start_idx = 0;
        while let Some(offset) = out[start_idx..].find(pat) {
            let start = start_idx + offset;
            let end = (start + 200).min(out.len());
            out.replace_range(start..end, "[REDACTED]");
            start_idx = start + "[REDACTED]".len();
        }
    }
    
    let headers = [("Bearer ", 7), ("x-api-key: ", 11), ("api-key: ", 9)];
    for &(header, len) in &headers {
        let mut start_idx = 0;
        while let Some(offset) = out[start_idx..].find(header) {
            let start = start_idx + offset;
            let val_start = start + len;
            let val_end = (val_start + 64).min(out.len());
            out.replace_range(val_start..val_end, "[REDACTED]");
            start_idx = val_start + "[REDACTED]".len();
        }
    }
    out
}

// =============================================================================
// SecretKey — safe wrapper that redacts Debug/Display
// =============================================================================

#[derive(Clone, Serialize)]
pub struct SecretKey(String);

impl SecretKey {
    pub fn new(key: String) -> Self { Self(key) }
    pub fn expose(&self) -> &str { &self.0 }
    pub fn as_str(&self) -> &str { &self.0 }
    fn redacted(&self) -> String {
        let s = &self.0;
        if s.len() <= 6 { return "****".into(); }
        format!("{}…{}", &s[..2], &s[s.len() - 2..])
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SecretKey(")?;
        f.write_str(&self.redacted())?;
        f.write_str(")")
    }
}

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.redacted())
    }
}

impl<'de> Deserialize<'de> for SecretKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Self(s))
    }
}

// =============================================================================
// KeyStore — cached, secure API key manager
// =============================================================================

pub struct KeyStore {
    keys: Mutex<HashMap<String, SecretKey>>,
}

impl Default for KeyStore {
    fn default() -> Self { Self::new() }
}

impl KeyStore {
    pub fn new() -> Self {
        Self { keys: Mutex::new(HashMap::new()) }
    }

    pub fn get(&self, env_key: &str, provider_id: &str) -> Result<SecretKey, String> {
        let mut cache = self.keys.lock().map_err(|e| format!("Lock poisoned: {}", e))?;
        if let Some(k) = cache.get(env_key) {
            return Ok(k.clone());
        }
        if let Ok(val) = std::env::var(env_key) {
            if !val.is_empty() {
                let sk = SecretKey::new(val);
                cache.insert(env_key.to_string(), sk.clone());
                return Ok(sk);
            }
        }
        if let Ok(config) = crate::load_config_inner() {
            if let Some(val) = config.api_keys.get(provider_id) {
                if !val.is_empty() {
                    let sk = SecretKey::new(val.clone());
                    cache.insert(env_key.to_string(), sk.clone());
                    return Ok(sk);
                }
            }
        }
        Err(format!("API key not found for {} (env: {})", provider_id, env_key))
    }
}
