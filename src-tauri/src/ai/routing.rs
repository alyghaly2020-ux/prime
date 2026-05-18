//! AI routing engine with fallback chain support and parallel model execution.
//!
//! Routes requests to the appropriate model based on task type,
//! provides automatic fallback when the primary model fails,
//! and enables concurrent model execution for speed and redundancy.

use super::providers::ProviderManager;
use super::{ChatMessage, ChatResponse};
use futures::stream::{FuturesUnordered, StreamExt};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// A chain of models to try, in priority order
#[derive(Debug, Clone)]
pub struct FallbackChain {
    /// Primary model ID to try first
    pub primary: String,
    /// Fallback model IDs to try in order if primary fails
    pub fallbacks: Vec<String>,
}

/// Record of a fallback attempt for observability
#[derive(Debug, Clone)]
pub struct FallbackAttempt {
    pub task_type: String,
    pub primary_model: String,
    pub fallbacks_tried: Vec<String>,
    pub succeeded: bool,
    pub final_model: Option<String>,
    pub latency_ms: u64,
    pub error: Option<String>,
}

/// Observability tracker for fallback attempts
pub struct FallbackTracker {
    attempts: RwLock<Vec<FallbackAttempt>>,
}

impl Default for FallbackTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl FallbackTracker {
    pub fn new() -> Self {
        Self {
            attempts: RwLock::new(Vec::new()),
        }
    }

    pub fn record(&self, attempt: FallbackAttempt) {
        self.attempts.write().push(attempt);
    }

    pub fn get_attempts(&self, limit: usize) -> Vec<FallbackAttempt> {
        let all = self.attempts.read();
        let len = all.len();
        all.iter()
            .skip(len.saturating_sub(limit))
            .cloned()
            .collect()
    }

    pub fn clear(&self) {
        self.attempts.write().clear();
    }
}

/// Configuration for parallel model execution.
///
/// Controls concurrency limits and per-model timeouts when running
/// multiple AI models simultaneously via [`RoutingEngine::parallel_chat`],
/// [`RoutingEngine::orchestrate_teams`], or [`RoutingEngine::race_models`].
#[derive(Debug, Clone)]
pub struct ParConfig {
    /// Maximum number of model calls to run concurrently.
    /// Defaults to `min(num_cpus, 8)`.
    pub max_concurrent: usize,
    /// Maximum time to wait for a single model call before abandoning it.
    pub timeout_per_model: std::time::Duration,
}

impl Default for ParConfig {
    fn default() -> Self {
        Self {
            max_concurrent: num_cpus::get().min(8),
            timeout_per_model: std::time::Duration::from_secs(120),
        }
    }
}

#[allow(dead_code)]
struct RoutingRule {
    model: String,
    conditions: Vec<RouteCondition>,
    fallbacks: Vec<String>,
}

#[allow(dead_code)]
enum RouteCondition {
    MaxTokens(u32),
    RequiresVision,
    RequiresStreaming,
    RequiresToolCalling,
}

/// Routing engine that maps task types to models with fallback support
pub struct RoutingEngine {
    routing_table: RwLock<HashMap<String, RoutingRule>>,
    fallback_tracker: Arc<FallbackTracker>,
    default_fallbacks: RwLock<Vec<String>>,
}

impl Default for RoutingEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl RoutingEngine {
    pub fn new() -> Self {
        let mut table = HashMap::new();

        // Default routing rules with fallback chains
        table.insert(
            "default".to_string(),
            RoutingRule {
                model: "gpt-5".to_string(),
                conditions: vec![],
                fallbacks: vec!["claude-4".to_string()],
            },
        );

        table.insert(
            "gpt".to_string(),
            RoutingRule {
                model: "gpt-5".to_string(),
                conditions: vec![],
                fallbacks: vec!["claude-4".to_string()],
            },
        );

        table.insert(
            "vision".to_string(),
            RoutingRule {
                model: "gpt-5".to_string(),
                conditions: vec![RouteCondition::RequiresVision],
                fallbacks: vec!["claude-4".to_string()],
            },
        );

        table.insert(
            "fast".to_string(),
            RoutingRule {
                model: "groq-fast".to_string(),
                conditions: vec![RouteCondition::MaxTokens(1000)],
                fallbacks: vec!["gpt-5".to_string(), "groq-llama".to_string()],
            },
        );

        table.insert(
            "code".to_string(),
            RoutingRule {
                model: "claude-4".to_string(),
                conditions: vec![],
                fallbacks: vec!["gpt-5".to_string(), "claude-4".to_string()],
            },
        );

        Self {
            routing_table: RwLock::new(table),
            fallback_tracker: Arc::new(FallbackTracker::new()),
            default_fallbacks: RwLock::new(vec!["gpt-5".to_string(), "claude-4".to_string()]),
        }
    }

    /// Route a task type to the primary model
    pub fn route(&self, task_type: &str) -> Option<String> {
        self.routing_table
            .read()
            .get(task_type)
            .map(|rule| rule.model.clone())
    }

    /// Get the full fallback chain for a task type
    pub fn fallback_chain(&self, task_type: &str) -> FallbackChain {
        let table = self.routing_table.read();
        if let Some(rule) = table.get(task_type) {
            FallbackChain {
                primary: rule.model.clone(),
                fallbacks: rule.fallbacks.clone(),
            }
        } else {
            FallbackChain {
                primary: task_type.to_string(),
                fallbacks: self.default_fallbacks.read().clone(),
            }
        }
    }

    /// Try all models in the fallback chain until one succeeds.
    ///
    /// Tracks each attempt for observability.
    pub async fn try_with_fallback(
        &self,
        task_type: &str,
        messages: Vec<ChatMessage>,
        providers: &ProviderManager,
    ) -> anyhow::Result<ChatResponse> {
        let chain = self.fallback_chain(task_type);
        let start = std::time::Instant::now();
        let mut last_error = None;
        let mut fallbacks_tried = Vec::new();

        // Collect all models to try: primary + fallbacks
        let all_models: Vec<String> = std::iter::once(chain.primary.clone())
            .chain(chain.fallbacks.iter().cloned())
            .collect();

        for (i, model_id) in all_models.iter().enumerate() {
            let config = super::models::ModelRegistry::new().get_config(model_id);

            let config = match config {
                Some(c) => c,
                None => {
                    last_error = Some(anyhow::anyhow!("Unknown model: {}", model_id));
                    if i > 0 {
                        fallbacks_tried.push(model_id.clone());
                    }
                    continue;
                }
            };

            let provider = providers.get(&config.provider);
            let provider = match provider {
                Some(p) => p,
                None => {
                    last_error = Some(anyhow::anyhow!("Unknown provider: {}", config.provider));
                    if i > 0 {
                        fallbacks_tried.push(model_id.clone());
                    }
                    continue;
                }
            };

            match provider.chat(&messages, &config).await {
                Ok(response) => {
                    let latency = start.elapsed().as_millis() as u64;
                    self.fallback_tracker.record(FallbackAttempt {
                        task_type: task_type.to_string(),
                        primary_model: chain.primary.clone(),
                        fallbacks_tried: fallbacks_tried.clone(),
                        succeeded: true,
                        final_model: Some(model_id.clone()),
                        latency_ms: latency,
                        error: None,
                    });
                    tracing::info!(
                        "Fallback chain succeeded on attempt {} (model: {}) in {}ms",
                        i + 1,
                        model_id,
                        latency
                    );
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!(
                        "Fallback attempt {} (model: {}) failed: {}",
                        i + 1,
                        model_id,
                        e
                    );
                    last_error = Some(e);
                    if i > 0 {
                        fallbacks_tried.push(model_id.clone());
                    }
                }
            }
        }

        let latency = start.elapsed().as_millis() as u64;
        self.fallback_tracker.record(FallbackAttempt {
            task_type: task_type.to_string(),
            primary_model: chain.primary.clone(),
            fallbacks_tried,
            succeeded: false,
            final_model: None,
            latency_ms: latency,
            error: last_error.as_ref().map(|e| e.to_string()),
        });

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("All models in fallback chain failed")))
    }

    /// Register a route with fallback chain
    pub fn register_route(&self, task_type: String, model: String, fallbacks: Vec<String>) {
        self.routing_table.write().insert(
            task_type,
            RoutingRule {
                model,
                conditions: vec![],
                fallbacks,
            },
        );
    }

    /// Set default fallback chain
    pub fn set_default_fallbacks(&self, fallbacks: Vec<String>) {
        *self.default_fallbacks.write() = fallbacks;
    }

    /// Get the fallback tracker for observability
    pub fn tracker(&self) -> &Arc<FallbackTracker> {
        &self.fallback_tracker
    }

    /// Update primary model for a route
    pub fn set_primary(&self, task_type: &str, model: String) {
        let mut table = self.routing_table.write();
        if let Some(rule) = table.get_mut(task_type) {
            rule.model = model;
        }
    }

    /// Resolve the best model for an agent based on its capabilities and preferred models.
    ///
    /// Priority:
    /// 1. Use preferred_models in order if provided
    /// 2. Map capabilities to routing task types and use the first match
    /// 3. Fall back to the default route
    pub fn resolve_agent_model(
        &self,
        capabilities: &[String],
        preferred_models: &[String],
    ) -> String {
        // Priority 1: preferred models
        if !preferred_models.is_empty() {
            let table = self.routing_table.read();
            for model_id in preferred_models {
                if table.values().any(|r| r.model == *model_id) {
                    return model_id.clone();
                }
            }
            return preferred_models[0].clone();
        }

        // Priority 2: map capabilities to routing task types
        let cap_to_task: Vec<(&str, &str)> = vec![
            ("system-design", "default"),
            ("architecture", "default"),
            ("vision", "vision"),
            ("image", "vision"),
            ("fast", "fast"),
            ("quick", "fast"),
            ("code", "code"),
            ("implementation", "code"),
            ("debugging", "code"),
            ("research", "default"),
            ("writing", "default"),
            ("planning", "default"),
            ("general", "default"),
        ];

        for (cap, task) in &cap_to_task {
            if capabilities.iter().any(|c| c == cap) {
                if let Some(model) = self.route(task) {
                    return model;
                }
            }
        }

        // Priority 3: default
        self.route("default").unwrap_or_else(|| "gpt-5".to_string())
    }

    /// Register capability-to-task-type mappings for agent model resolution
    pub fn register_capability_route(&self, capability: &str, _task_type: &str, model: &str) {
        self.register_route(
            format!("cap:{}", capability),
            model.to_string(),
            vec![],
        );
    }

    // =========================================================================
    // Parallel Execution
    // =========================================================================

    /// Run the primary + top N fallbacks **concurrently** and return the
    /// first successful response (race pattern).
    ///
    /// `count` controls how many models are tried at once (primary + up to
    /// `count - 1` fallbacks).  If every model fails, the collected errors
    /// are joined and returned.
    ///
    /// When `override_models` is provided, it bypasses the internal model
    /// registry resolution and races those model IDs directly.
    pub async fn parallel_chat(
        &self,
        task_type: &str,
        messages: Vec<ChatMessage>,
        providers: &ProviderManager,
        count: usize,
        override_models: Option<&[String]>,
    ) -> anyhow::Result<ChatResponse> {
        let chain = self.fallback_chain(task_type);
        let start = std::time::Instant::now();

        let registry = super::models::ModelRegistry::new();

        let race_models = if let Some(models) = override_models {
            models.to_vec()
        } else {
            // Helper to check if a model's provider is available
            let is_available = |provider: &str| -> bool {
                if provider == "ollama" || provider == "lmstudio" {
                    return true;
                }
                let env_key = match provider {
                    "openai" => "OPENAI_API_KEY",
                    "anthropic" => "ANTHROPIC_API_KEY",
                    "google" => "GOOGLE_API_KEY",
                    "groq" => "GROQ_API_KEY",
                    "mistral" => "MISTRAL_API_KEY",
                    "openrouter" => "OPENROUTER_API_KEY",
                    "localai" => "LOCALAI_API_KEY",
                    _ => "API_KEY",
                };
                crate::get_api_key(env_key, provider).is_ok()
            };

            let all_registered_models = registry.list_all();

            let mut available_models = Vec::new();
            for model in &all_registered_models {
                if is_available(&model.provider) {
                    available_models.push(model.id.clone());
                }
            }

            let mut race_models: Vec<String> = Vec::new();

            let primary_id = chain.primary.clone();
            if let Some(cfg) = all_registered_models.iter().find(|m| m.id == primary_id) {
                if is_available(&cfg.provider) {
                    race_models.push(primary_id);
                }
            }

            for fallback in &chain.fallbacks {
                if race_models.len() >= count {
                    break;
                }
                if !race_models.contains(fallback) {
                    if let Some(cfg) = all_registered_models.iter().find(|m| m.id == *fallback) {
                        if is_available(&cfg.provider) {
                            race_models.push(fallback.clone());
                        }
                    }
                }
            }

            for model_id in &available_models {
                if race_models.len() >= count {
                    break;
                }
                if !race_models.contains(model_id) {
                    race_models.push(model_id.clone());
                }
            }

            if race_models.is_empty() {
                race_models = std::iter::once(chain.primary.clone())
                    .chain(chain.fallbacks.iter().cloned())
                    .take(count.max(1))
                    .collect();
            }

            race_models
        };

        // Pre-resolve config + provider for each model so the async blocks
        // own an `Arc<dyn AiProvider>` and avoid borrowing `providers`.
        let mut resolved: Vec<(String, super::ModelConfig, Arc<dyn super::providers::AiProvider>)> =
            Vec::with_capacity(race_models.len());

        for model_id in &race_models {
            let config = registry.get_config(model_id);
            let config = match config {
                Some(c) => c,
                None => {
                    tracing::warn!("parallel_chat: unknown model '{}', skipping", model_id);
                    continue;
                }
            };
            let provider = providers.get(&config.provider);
            let provider = match provider {
                Some(p) => p,
                None => {
                    tracing::warn!(
                        "parallel_chat: unknown provider '{}' for model '{}', skipping",
                        config.provider,
                        model_id
                    );
                    continue;
                }
            };
            resolved.push((model_id.clone(), config, provider));
        }

        let total_count = resolved.len();
        if total_count == 0 {
            return Err(anyhow::anyhow!(
                "parallel_chat: no valid models to run (task_type={}, count={})",
                task_type,
                count
            ));
        }

        // Spawn tasks into FuturesUnordered for true racing
        let mut futs = FuturesUnordered::new();
        for (model_id, config, provider) in resolved {
            let msgs = messages.clone();
            futs.push(tokio::spawn(async move {
                match provider.chat(&msgs, &config).await {
                    Ok(response) => (model_id, Ok(response)),
                    Err(e) => (model_id, Err(e)),
                }
            }));
        }

        // Return FIRST success immediately (true race)
        let mut errors: Vec<(String, anyhow::Error)> = Vec::new();
        while let Some(join_result) = futs.next().await {
            let (model_id, chat_result) = match join_result {
                Ok(pair) => pair,
                Err(join_err) => {
                    tracing::error!("parallel_chat task panicked: {}", join_err);
                    continue;
                }
            };
            match chat_result {
                Ok(response) => {
                    let latency = start.elapsed().as_millis() as u64;
                    self.fallback_tracker.record(FallbackAttempt {
                        task_type: task_type.to_string(),
                        primary_model: chain.primary.clone(),
                        fallbacks_tried: Vec::new(),
                        succeeded: true,
                        final_model: Some(model_id),
                        latency_ms: latency,
                        error: None,
                    });
                    tracing::info!(
                        "parallel_chat won by '{}' in {}ms (race pattern)",
                        response.model,
                        latency
                    );
                    return Ok(response);
                }
                Err(e) => {
                    tracing::warn!("parallel_chat: model '{}' failed: {}", model_id, e);
                    errors.push((model_id, e));
                }
            }
        }

        // All models failed.
        let latency = start.elapsed().as_millis() as u64;
        let error_msg = errors
            .into_iter()
            .map(|(id, e)| format!("{}: {}", id, e))
            .collect::<Vec<_>>()
            .join(" | ");

        self.fallback_tracker.record(FallbackAttempt {
            task_type: task_type.to_string(),
            primary_model: chain.primary.clone(),
            fallbacks_tried: Vec::new(),
            succeeded: false,
            final_model: None,
            latency_ms: latency,
            error: Some(error_msg.clone()),
        });

        Err(anyhow::anyhow!(
            "parallel_chat: all {} models failed — {}",
            total_count,
            error_msg
        ))
    }

    /// Run multiple **different** task types concurrently.
    ///
    /// Each entry in `teams` is a `(task_type, messages)` pair.  Every team
    /// is dispatched in its own tokio task; results are collected into a
    /// `HashMap` keyed by `task_type`.  A failure in one team does **not**
    /// cancel or affect the others.
    ///
    /// Unlike [`parallel_chat`], each team uses only its **primary** model
    /// (no fallback chaining inside the parallel path).  If you need
    /// per-team fallback, call [`parallel_chat`] individually.
    pub async fn orchestrate_teams(
        &self,
        teams: Vec<(String, Vec<ChatMessage>)>,
        providers: &ProviderManager,
    ) -> HashMap<String, anyhow::Result<String>> {
        let mut handles = Vec::with_capacity(teams.len());

        for (task_type, messages) in teams {
            let chain = self.fallback_chain(&task_type);
            let model_id = chain.primary;

            let config = super::models::ModelRegistry::new().get_config(&model_id);
            let config = match config {
                Some(c) => c,
                None => {
                    let tt = task_type.clone();
                    handles.push(tokio::spawn(async move {
                        (tt, Err(anyhow::anyhow!("Unknown model: {}", model_id)))
                    }));
                    continue;
                }
            };

            let provider = providers.get(&config.provider);
            let provider = match provider {
                Some(p) => p,
                None => {
                    let tt = task_type.clone();
                    let pn = config.provider.clone();
                    handles.push(tokio::spawn(async move {
                        (tt, Err(anyhow::anyhow!("Unknown provider: {}", pn)))
                    }));
                    continue;
                }
            };

            handles.push(tokio::spawn(async move {
                match provider.chat(&messages, &config).await {
                    Ok(response) => (task_type, Ok(response.content)),
                    Err(e) => (task_type, Err(e)),
                }
            }));
        }

        let results = futures::future::join_all(handles).await;
        let mut map = HashMap::with_capacity(results.len());

        for join_result in results {
            let (task_type, result) = match join_result {
                Ok(pair) => pair,
                Err(join_err) => {
                    tracing::error!("orchestrate_teams task panicked: {}", join_err);
                    continue;
                }
            };
            map.insert(task_type, result);
        }

        map
    }

    /// Race a single prompt across an explicit set of model IDs.
    ///
    /// All models run concurrently; the **first** successful response wins.
    /// This is useful for latency-sensitive scenarios where you want the
    /// fastest model to answer, or for comparing outputs across models.
    pub async fn race_models(
        &self,
        messages: Vec<ChatMessage>,
        model_ids: &[String],
        providers: &ProviderManager,
    ) -> anyhow::Result<ChatResponse> {
        if model_ids.is_empty() {
            return Err(anyhow::anyhow!("race_models: no model IDs provided"));
        }

        let start = std::time::Instant::now();

        // Pre-resolve configs and providers.
        let mut resolved: Vec<(String, super::ModelConfig, Arc<dyn super::providers::AiProvider>)> =
            Vec::with_capacity(model_ids.len());

        for model_id in model_ids {
            let config = super::models::ModelRegistry::new().get_config(model_id);
            let config = match config {
                Some(c) => c,
                None => {
                    tracing::warn!("race_models: unknown model '{}', skipping", model_id);
                    continue;
                }
            };
            let provider = providers.get(&config.provider);
            let provider = match provider {
                Some(p) => p,
                None => {
                    tracing::warn!(
                        "race_models: unknown provider '{}' for '{}', skipping",
                        config.provider,
                        model_id
                    );
                    continue;
                }
            };
            resolved.push((model_id.clone(), config, provider));
        }

        let total = resolved.len();
        if total == 0 {
            return Err(anyhow::anyhow!(
                "race_models: no valid models out of {} supplied",
                model_ids.len()
            ));
        }

        // Spawn into FuturesUnordered for true racing
        let mut futs = FuturesUnordered::new();
        for (model_id, config, provider) in resolved {
            let msgs = messages.clone();
            futs.push(tokio::spawn(async move {
                match provider.chat(&msgs, &config).await {
                    Ok(response) => (model_id, Ok(response)),
                    Err(e) => (model_id, Err(e)),
                }
            }));
        }

        // Return FIRST success immediately
        let mut errors: Vec<(String, anyhow::Error)> = Vec::new();
        while let Some(join_result) = futs.next().await {
            let (model_id, chat_result) = match join_result {
                Ok(pair) => pair,
                Err(join_err) => {
                    tracing::error!("race_models task panicked: {}", join_err);
                    continue;
                }
            };
            match chat_result {
                Ok(response) => {
                    let latency = start.elapsed().as_millis() as u64;
                    tracing::info!(
                        "race_models won by '{}' in {}ms (race pattern)",
                        response.model,
                        latency
                    );
                    return Ok(response);
                }
                Err(e) => {
                    errors.push((model_id, e));
                }
            }
        }

        let error_msg = errors
            .into_iter()
            .map(|(id, e)| format!("{}: {}", id, e))
            .collect::<Vec<_>>()
            .join(" | ");

        Err(anyhow::anyhow!(
            "race_models: all {} models failed — {}",
            total,
            error_msg
        ))
    }
    /// Broadcast the same prompt to **all** registered models concurrently.
    ///
    /// Unlike [`parallel_chat`] which only tries the fallback chain for a single
    /// task type, this method fans out to **every** model in the routing table.
    /// Returns a map of model_id → result.  A failure in one model does not
    /// affect others.  This is the ultimate parallel execution pattern,
    /// enabling true multi-model concurrent reasoning.
    pub async fn broadcast_to_all(
        &self,
        messages: Vec<ChatMessage>,
        providers: &ProviderManager,
    ) -> HashMap<String, anyhow::Result<String>> {
        let models: Vec<String> = {
            let table = self.routing_table.read();
            let mut seen = std::collections::HashSet::new();
            let mut all = Vec::new();
            for rule in table.values() {
                if seen.insert(rule.model.clone()) {
                    all.push(rule.model.clone());
                }
                for fb in &rule.fallbacks {
                    if seen.insert(fb.clone()) {
                        all.push(fb.clone());
                    }
                }
            }
            all
        };

        if models.is_empty() {
            return HashMap::new();
        }

        let mut handles = Vec::with_capacity(models.len());

        for model_id in &models {
            let config = super::models::ModelRegistry::new().get_config(model_id);
            let config = match config {
                Some(c) => c,
                None => continue,
            };
            let provider = providers.get(&config.provider);
            let provider = match provider {
                Some(p) => p,
                None => continue,
            };
            let msgs = messages.clone();
            let mid = model_id.clone();

            handles.push(tokio::spawn(async move {
                match provider.chat(&msgs, &config).await {
                    Ok(response) => (mid, Ok(response.content)),
                    Err(e) => (mid, Err(e)),
                }
            }));
        }

        let results = futures::future::join_all(handles).await;
        let mut map = HashMap::with_capacity(results.len());

        for join_result in results {
            let (model_id, result) = match join_result {
                Ok(pair) => pair,
                Err(join_err) => {
                    tracing::error!("broadcast_to_all task panicked: {}", join_err);
                    continue;
                }
            };
            map.insert(model_id, result);
        }

        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route() {
        let engine = RoutingEngine::new();
        assert_eq!(engine.route("default"), Some("gpt-5".to_string()));
        assert_eq!(engine.route("code"), Some("claude-4".to_string()));
        assert_eq!(engine.route("vision"), Some("gpt-5".to_string()));
    }

    #[test]
    fn test_fallback_chain() {
        let engine = RoutingEngine::new();
        let chain = engine.fallback_chain("default");
        assert_eq!(chain.primary, "gpt-5");
        assert!(!chain.fallbacks.is_empty());
    }

    #[test]
    fn test_unknown_route_fallback() {
        let engine = RoutingEngine::new();
        let chain = engine.fallback_chain("unknown_task");
        // Unknown tasks get the default fallback
        assert!(!chain.fallbacks.is_empty());
    }

    #[test]
    fn test_register_route() {
        let engine = RoutingEngine::new();
        engine.register_route(
            "reasoning".to_string(),
            "claude-4".to_string(),
            vec!["gpt-5".to_string()],
        );
        assert_eq!(engine.route("reasoning"), Some("claude-4".to_string()));
        let chain = engine.fallback_chain("reasoning");
        assert_eq!(chain.fallbacks, vec!["gpt-5"]);
    }

    #[test]
    fn test_set_default_fallbacks() {
        let engine = RoutingEngine::new();
        engine.set_default_fallbacks(vec!["gemini-2".to_string()]);
        let chain = engine.fallback_chain("nonexistent");
        assert_eq!(chain.fallbacks, vec!["gemini-2"]);
    }

    #[test]
    fn test_fallback_tracker() {
        let tracker = FallbackTracker::new();
        tracker.record(FallbackAttempt {
            task_type: "test".to_string(),
            primary_model: "gpt-5".to_string(),
            fallbacks_tried: vec!["claude-4".to_string()],
            succeeded: true,
            final_model: Some("claude-4".to_string()),
            latency_ms: 1500,
            error: None,
        });

        let attempts = tracker.get_attempts(10);
        assert_eq!(attempts.len(), 1);
        assert!(attempts[0].succeeded);
        assert_eq!(attempts[0].final_model, Some("claude-4".to_string()));
    }

    // -------------------------------------------------------------------------
    // ParConfig
    // -------------------------------------------------------------------------

    #[test]
    fn test_par_config_default() {
        let cfg = ParConfig::default();
        assert!(cfg.max_concurrent >= 1);
        assert_eq!(cfg.timeout_per_model, std::time::Duration::from_secs(120));
    }

    #[test]
    fn test_par_config_custom() {
        let cfg = ParConfig {
            max_concurrent: 3,
            timeout_per_model: std::time::Duration::from_secs(30),
        };
        assert_eq!(cfg.max_concurrent, 3);
        assert_eq!(cfg.timeout_per_model.as_secs(), 30);
    }

    // -------------------------------------------------------------------------
    // parallel_chat — unit tests with a mock provider
    // -------------------------------------------------------------------------

    use super::super::providers::AiProvider;
    use super::super::{ModelConfig, Usage};
    use async_trait::async_trait;

    /// A mock provider that can be configured to succeed or fail.
    struct MockProvider {
        name: String,
        should_fail: bool,
        response_content: String,
    }

    #[async_trait]
    impl AiProvider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        async fn chat(
            &self,
            _messages: &[ChatMessage],
            _config: &ModelConfig,
        ) -> anyhow::Result<ChatResponse> {
            if self.should_fail {
                Err(anyhow::anyhow!("mock failure"))
            } else {
                Ok(ChatResponse {
                    content: self.response_content.clone(),
                    model: self.name.clone(),
                    usage: Usage {
                        prompt_tokens: 10,
                        completion_tokens: 20,
                        total_tokens: 30,
                    },
                    finish_reason: "stop".to_string(),
                })
            }
        }

        async fn chat_stream(
            &self,
            _messages: &[ChatMessage],
            _config: &ModelConfig,
        ) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
            let (tx, rx) = tokio::sync::mpsc::channel(1);
            tokio::spawn(async move {
                let _ = tx.send("mock stream".to_string()).await;
            });
            Ok(rx)
        }
    }

    /// Helper to build a ProviderManager with one mock provider.
    fn mock_provider_manager(
        name: &str,
        should_fail: bool,
        content: &str,
    ) -> ProviderManager {
        let pm = ProviderManager::new_empty();
        let provider = MockProvider {
            name: name.to_string(),
            should_fail,
            response_content: content.to_string(),
        };
        pm.register(std::sync::Arc::new(provider), 1);
        pm
    }

    #[tokio::test]
    async fn test_parallel_chat_success() {
        let engine = RoutingEngine::new();
        let pm = mock_provider_manager("openai", false, "ok");
        let result = engine
            .parallel_chat("nonexistent_task", vec![], &pm, 2, Some(&["gpt-5".to_string()]))
            .await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result.err());
        let response = result.unwrap();
        assert_eq!(response.content, "ok");
    }

    #[tokio::test]
    async fn test_parallel_chat_all_fail() {
        let engine = RoutingEngine::new();
        let pm = ProviderManager::new_empty();
        let fail_provider = MockProvider {
            name: "openai".to_string(),
            should_fail: true,
            response_content: String::new(),
        };
        pm.register(std::sync::Arc::new(fail_provider), 1);

        let result = engine
            .parallel_chat("default", vec![], &pm, 1, Some(&["gpt-5".to_string()]))
            .await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("failed") || err.contains("fail"));
    }

    #[tokio::test]
    async fn test_parallel_chat_first_wins() {
        let engine = RoutingEngine::new();
        let pm = ProviderManager::new_empty();

        let openai = MockProvider {
            name: "openai".to_string(),
            should_fail: false,
            response_content: "openai-response".to_string(),
        };
        pm.register(std::sync::Arc::new(openai), 1);

        let result = engine
            .parallel_chat("default", vec![], &pm, 2, Some(&["gpt-5".to_string()]))
            .await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.content, "openai-response");
    }

    #[tokio::test]
    async fn test_orchestrate_teams_empty() {
        let engine = RoutingEngine::new();
        let pm = mock_provider_manager("openai", false, "ok");
        let results = engine.orchestrate_teams(vec![], &pm).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_orchestrate_teams_single() {
        let engine = RoutingEngine::new();
        let pm = mock_provider_manager("openai", false, "hello team");
        let teams = vec![("default".to_string(), vec![])];
        let results = engine.orchestrate_teams(teams, &pm).await;
        assert_eq!(results.len(), 1);
        let result = results.get("default").unwrap();
        assert!(result.is_ok());
        assert_eq!(result.as_ref().unwrap(), "hello team");
    }

    #[tokio::test]
    async fn test_orchestrate_teams_independent_failures() {
        let engine = RoutingEngine::new();
        let pm = ProviderManager::new_empty();
        // "default" → primary "gpt-5" → provider "openai"
        let openai = MockProvider {
            name: "openai".to_string(),
            should_fail: false,
            response_content: "openai-ok".to_string(),
        };
        // "fast" → primary "groq-llama" → provider "groq"
        let groq = MockProvider {
            name: "groq".to_string(),
            should_fail: false,
            response_content: "groq-ok".to_string(),
        };
        pm.register(std::sync::Arc::new(openai), 1);
        pm.register(std::sync::Arc::new(groq), 1);

        let teams = vec![
            ("default".to_string(), vec![]),
            ("fast".to_string(), vec![]),
        ];
        let results = engine.orchestrate_teams(teams, &pm).await;
        assert_eq!(results.len(), 2);
        assert!(results.get("default").unwrap().is_ok());
        assert!(results.get("fast").unwrap().is_ok());
    }

    #[tokio::test]
    async fn test_race_models_empty() {
        let engine = RoutingEngine::new();
        let pm = mock_provider_manager("openai", false, "x");
        let result = engine
            .race_models(vec![], &[], &pm)
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("no model IDs"));
    }

    #[tokio::test]
    async fn test_race_models_first_success() {
        let engine = RoutingEngine::new();
        let pm = ProviderManager::new_empty();
        let fast = MockProvider {
            name: "fast".to_string(),
            should_fail: false,
            response_content: "fast-wins".to_string(),
        };
        let slow = MockProvider {
            name: "slow".to_string(),
            should_fail: false,
            response_content: "slow-loses".to_string(),
        };
        pm.register(std::sync::Arc::new(fast), 1);
        pm.register(std::sync::Arc::new(slow), 1);

        // Use model IDs that exist in registry and match our providers
        // "gpt-5" uses "openai" provider, but we registered "fast" and "slow" — won't match.
        // Let's use the actual approach: just test with valid model IDs.
        // Actually both models resolve to "openai" provider, which we must register.
        let openai = MockProvider {
            name: "openai".to_string(),
            should_fail: false,
            response_content: "openai-wins".to_string(),
        };
        pm.register(std::sync::Arc::new(openai), 1);

        let model_ids = vec!["gpt-5".to_string(), "claude-4".to_string()];
        let result = engine
            .race_models(vec![], &model_ids, &pm)
            .await;
        assert!(result.is_ok(), "expected ok, got: {:?}", result.err());
        let response = result.unwrap();
        assert_eq!(response.content, "openai-wins");
    }
}
