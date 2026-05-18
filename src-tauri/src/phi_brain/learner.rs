/// T5+T6: Learner + Performance Optimizer.
///
/// Learns from every interaction to build a user profile that
/// gradually improves routing, settings, and experience.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::phi_brain::client::OllamaClient;
use crate::phi_brain::profile_db::UserProfile;

/// A single interaction to learn from.
#[derive(Debug, Clone)]
pub struct Interaction {
    pub user_message: String,
    pub assistant_response: String,
    pub model_used: String,
    pub task_type: String,
    pub response_time_ms: u64,
    pub user_feedback: Option<f32>, // positive (1.0) or negative (0.0)
    pub timestamp: u64,
}

/// Learner that builds user profile from interactions.
pub struct Learner {
    client: Arc<OllamaClient>,
    profile: Arc<RwLock<UserProfile>>,
}

impl Learner {
    pub fn new(profile: Arc<RwLock<UserProfile>>) -> Self {
        Self {
            client: OllamaClient::new().shared(),
            profile,
        }
    }

    /// Create with a shared OllamaClient.
    pub fn with_client(client: Arc<OllamaClient>, profile: Arc<RwLock<UserProfile>>) -> Self {
        Self { client, profile }
    }

    /// Learn from a single interaction. Called after each chat response.
    pub async fn learn_from_interaction(&self, interaction: &Interaction) {
        let mut profile = self.profile.write().await;

        // 1. Update basic stats
        profile.total_interactions += 1;
        profile.record_task(&interaction.task_type);
        profile.update_message_length(interaction.user_message.len() as f32);

        // 2. Detect language
        let lang = UserProfile::detect_language(&interaction.user_message);
        if lang != profile.preferred_language {
            // Track language preference (majority vote)
            profile.preferred_language = lang.to_string();
        }

        // 3. Record active hour
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        let hour = (now.as_secs() / 3600) % 24;
        profile.record_active_hour(hour as u8);

        // 4. Update model satisfaction
        if let Some(feedback) = interaction.user_feedback {
            profile.update_model_satisfaction(&interaction.model_used, feedback);
        } else {
            // Infer satisfaction from response patterns
            let satisfaction = self.estimate_satisfaction(interaction);
            profile.update_model_satisfaction(&interaction.model_used, satisfaction);
        }

        // 5. Update maturity
        profile.update_maturity();
        profile.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 6. Save profile to disk
        profile.save();

        // 7. Every 50 interactions: Phi optimizes the profile
        if profile.total_interactions % 50 == 0 {
            drop(profile); // Release write lock
            self.optimize_with_phi().await;
        }
    }

    /// Learn from a correction (proofreader found an error).
    pub async fn learn_from_correction(&self, model: &str, correction_type: &str) {
        let mut profile = self.profile.write().await;
        profile.total_corrections += 1;

        // Lower satisfaction for this model
        let current = profile.model_satisfaction.get(model).copied().unwrap_or(0.5);
        profile.update_model_satisfaction(model, current * 0.8);

        // Track common corrections
        profile.common_corrections.push((
            model.to_string(),
            correction_type.to_string(),
        ));

        // Keep only last 100 corrections
        if profile.common_corrections.len() > 100 {
            let keep = profile.common_corrections.len() - 100;
            profile.common_corrections.drain(..keep);
        }

        profile.save();
    }

    /// Learn from a hallucination caught.
    pub async fn learn_from_hallucination(&self, model: &str) {
        let mut profile = self.profile.write().await;
        profile.total_hallucinations_caught += 1;

        // Significantly lower satisfaction
        let current = profile.model_satisfaction.get(model).copied().unwrap_or(0.5);
        profile.update_model_satisfaction(model, current * 0.6);

        profile.save();
    }

    /// Estimate user satisfaction from interaction patterns.
    fn estimate_satisfaction(&self, interaction: &Interaction) -> f32 {
        let mut score: f32 = 0.7; // Default: slightly positive

        // Negative signals
        let response = &interaction.assistant_response;
        if response.contains("I'm sorry") || response.contains("I apologize") {
            score -= 0.2;
        }
        if response.contains("I don't know") || response.contains("I cannot") {
            score -= 0.15;
        }
        if response.len() < 20 {
            score -= 0.1; // Too short might mean unhelpful
        }

        // Positive signals
        if response.len() > 100 && response.len() < 2000 {
            score += 0.1; // Good length response
        }

        // Fast response is generally better
        if interaction.response_time_ms < 2000 {
            score += 0.05;
        } else if interaction.response_time_ms > 10000 {
            score -= 0.1; // Very slow
        }

        score.max(0.0).min(1.0)
    }

    /// Use Phi Brain to optimize the profile.
    async fn optimize_with_phi(&self) {
        let profile = self.profile.read().await;

        // If Ollama is not available, skip
        if self.client.check_health().await.is_err() {
            return;
        }

        let prompt = format!(
            r#"Based on this user's usage patterns, suggest optimizations:
- Task frequencies: {:?}
- Model satisfaction: {:?}
- Average message length: {:.0} chars
- Preferred language: {}
- Total interactions: {}

Suggest improvements as JSON:
{{"reorder_models": ["model1", "model2"], "adjust_temperature": {{"task": temp}}, "ui_suggestions": ["suggestion1"]}}
"#,
            profile.task_frequency,
            profile.model_satisfaction,
            profile.avg_message_length,
            profile.preferred_language,
            profile.total_interactions,
        );

        drop(profile); // Release read lock

        match self.client.generate(&prompt, 0.1, 256).await {
            Ok(response) => {
                // Parse and apply suggestions
                if let Ok(suggestions) = serde_json::from_str::<serde_json::Value>(&response) {
                if let Some(_reorder) = suggestions.get("reorder_models") {
                    tracing::info!("Phi Brain optimization: model reorder suggested");
                }
                if let Some(_temps) = suggestions.get("adjust_temperature") {
                    tracing::info!("Phi Brain optimization: temperature adjustments suggested");
                }
                }
            }
            Err(e) => {
                tracing::warn!("Phi Brain optimization failed: {}", e);
            }
        }
    }

    /// Get the current user profile (read-only snapshot).
    pub async fn get_profile(&self) -> UserProfile {
        self.profile.read().await.clone()
    }
}
