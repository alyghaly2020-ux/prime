/// T1: Smart Model Orchestrator.
///
/// Uses Phi Brain to intelligently route requests to the best model
/// based on task type, system health, and user preferences.

use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ai::ChatMessage;
use crate::phi_brain::client::OllamaClient;
use crate::phi_brain::profile_db::UserProfile;

/// Routing decision from Phi Brain analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub model: String,
    pub mode: String,       // "single", "race", "parallel"
    pub max_tokens: u32,
    pub temperature: f32,
    pub reasoning: String,
}

impl Default for RoutingDecision {
    fn default() -> Self {
        Self {
            model: "auto".to_string(),
            mode: "single".to_string(),
            max_tokens: 4096,
            temperature: 0.7,
            reasoning: "default routing".to_string(),
        }
    }
}

/// System health snapshot for routing decisions.
#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub cpu_percent: f32,
    pub ram_percent: f32,
    pub temp_celsius: f32,
}

impl Default for SystemHealth {
    fn default() -> Self {
        Self {
            cpu_percent: 0.0,
            ram_percent: 0.0,
            temp_celsius: 0.0,
        }
    }
}

/// Smart orchestrator that decides the best model for each request.
pub struct SmartOrchestrator {
    client: Arc<OllamaClient>,
}

impl SmartOrchestrator {
    pub fn new() -> Self {
        Self {
            client: OllamaClient::new().shared(),
        }
    }

    /// Create with a shared OllamaClient.
    pub fn with_client(client: Arc<OllamaClient>) -> Self {
        Self { client }
    }

    /// Check if Ollama is reachable.
    pub async fn check_ollama(&self) -> anyhow::Result<()> {
        self.client.check_health().await
    }

    /// Analyze the request and decide the best routing.
    pub async fn decide(
        &self,
        messages: &[ChatMessage],
        system_health: &SystemHealth,
        user_profile: &UserProfile,
        available_models: &[String],
    ) -> RoutingDecision {
        // If Ollama is not available, fall back to default routing
        if self.client.check_health().await.is_err() {
            tracing::debug!("Phi Brain: Ollama not available, using default routing");
            return RoutingDecision::default();
        }

        let last_msg = messages.last().map(|m| m.content.clone()).unwrap_or_default();
        let task_type = UserProfile::classify_task(&last_msg);
        let lang = UserProfile::detect_language(&last_msg);

        // Build the routing prompt
        let available_str = available_models.join(", ");
        let prompt = format!(
            r#"You are an expert AI model router. Analyze this request and choose the best model.

Request analysis:
- Task type: {task_type}
- Language: {lang}
- Message length: {msg_len} chars
- System: CPU {cpu}%, RAM {ram}%, Temp {temp}°C
- User prefers: {pref_style} responses
- Available models: {available}

Choose the best model and settings. Reply ONLY in JSON:
{{"model":"model_id","mode":"single|race|parallel","max_tokens":N,"temperature":0.0-1.0,"reasoning":"brief explanation"}}

Rules:
- Code tasks → use coding-specialized models
- Simple queries → use fast/small models
- Complex analysis → use larger models
- High system load → prefer local/smaller models
- User prefers concise → lower max_tokens
"#,
            task_type = task_type,
            lang = lang,
            msg_len = last_msg.len(),
            cpu = system_health.cpu_percent,
            ram = system_health.ram_percent,
            temp = system_health.temp_celsius,
            pref_style = user_profile.preferred_response_style,
            available = available_str,
        );

        // Call Phi Brain via unified client
        match self.client.generate(&prompt, 0.1, 256).await {
            Ok(response) => {
                // Parse JSON from response
                if let Ok(decision) = serde_json::from_str::<RoutingDecision>(&response) {
                    tracing::info!(
                        "Phi Brain routing: model={}, mode={}, reason={}",
                        decision.model,
                        decision.mode,
                        decision.reasoning
                    );
                    decision
                } else {
                    // If JSON parsing fails, try to extract model name
                    tracing::warn!("Phi Brain: failed to parse JSON response, using default");
                    RoutingDecision::default()
                }
            }
            Err(e) => {
                tracing::warn!("Phi Brain: generation failed: {}", e);
                RoutingDecision::default()
            }
        }
    }

    /// Get recommended model for a task type based on user profile.
    pub fn recommend_from_profile(
        &self,
        task_type: &str,
        user_profile: &UserProfile,
        available_models: &[String],
    ) -> String {
        // Check if user has a preferred model for this task
        if let Some(preferred) = user_profile.preferred_models_per_task.get(task_type) {
            if available_models.contains(&preferred.to_string()) {
                return preferred.clone();
            }
        }

        // Default recommendations by task type
        match task_type {
            "code" => {
                // Prefer coding models
                for m in available_models {
                    if m.contains("coder") || m.contains("code") || m.contains("deepseek") {
                        return m.clone();
                    }
                }
            }
            "analysis" | "summarization" => {
                // Prefer analytical models
                for m in available_models {
                    if m.contains("sonnet") || m.contains("claude") || m.contains("gpt-4") {
                        return m.clone();
                    }
                }
            }
            "writing" | "translation" => {
                // Prefer language models
                for m in available_models {
                    if m.contains("gpt") || m.contains("claude") {
                        return m.clone();
                    }
                }
            }
            _ => {}
        }

        // Fallback: use the most satisfied model
        if let Some((model, _)) = user_profile
            .model_satisfaction
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
        {
            if available_models.contains(model) {
                return model.clone();
            }
        }

        // Ultimate fallback
        available_models.first().cloned().unwrap_or_else(|| "auto".to_string())
    }
}
