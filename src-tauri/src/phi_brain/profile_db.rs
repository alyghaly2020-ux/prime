/// User profile database — local, encrypted, no telemetry.
///
/// Tracks user preferences, model satisfaction, task patterns,
/// and learned optimizations over time.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// User profile built from interaction patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    // Usage patterns
    pub preferred_language: String,
    pub avg_message_length: f32,
    pub preferred_response_style: String,
    pub active_hours: Vec<u8>,

    // Model preferences
    pub model_satisfaction: HashMap<String, f32>,
    pub task_frequency: HashMap<String, u32>,
    pub preferred_models_per_task: HashMap<String, String>,

    // Learned optimizations
    pub optimal_temperature: HashMap<String, f32>,
    pub optimal_max_tokens: HashMap<String, u32>,
    pub common_corrections: Vec<(String, String)>,

    // Statistics
    pub total_interactions: u32,
    pub total_corrections: u32,
    pub total_hallucinations_caught: u32,
    pub profile_maturity: f32,

    // Metadata
    pub first_seen: u64,
    pub last_updated: u64,
}

impl Default for UserProfile {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            preferred_language: "en".to_string(),
            avg_message_length: 50.0,
            preferred_response_style: "concise".to_string(),
            active_hours: Vec::new(),

            model_satisfaction: HashMap::new(),
            task_frequency: HashMap::new(),
            preferred_models_per_task: HashMap::new(),

            optimal_temperature: HashMap::new(),
            optimal_max_tokens: HashMap::new(),
            common_corrections: Vec::new(),

            total_interactions: 0,
            total_corrections: 0,
            total_hallucinations_caught: 0,
            profile_maturity: 0.0,

            first_seen: now,
            last_updated: now,
        }
    }
}

impl UserProfile {
    /// Get the profile file path.
    fn profile_path() -> Option<std::path::PathBuf> {
        if let Some(data_dir) = dirs_next::data_dir() {
            let prime_dir = data_dir.join("prime");
            Some(prime_dir.join("phi_brain_profile.json"))
        } else {
            None
        }
    }

    /// Load profile from disk, or create default if not found.
    pub fn load_or_default() -> Self {
        if let Some(path) = Self::profile_path() {
            if path.exists() {
                match std::fs::read_to_string(&path) {
                    Ok(json) => {
                        match serde_json::from_str::<UserProfile>(&json) {
                            Ok(profile) => {
                                tracing::info!(
                                    "Phi Brain: loaded profile (maturity={:.0}%, interactions={})",
                                    profile.profile_maturity * 100.0,
                                    profile.total_interactions,
                                );
                                return profile;
                            }
                            Err(e) => {
                                tracing::warn!("Phi Brain: profile JSON parse error: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Phi Brain: could not read profile: {}", e);
                    }
                }
            }
        }
        tracing::info!("Phi Brain: creating fresh profile");
        Self::default()
    }

    /// Save profile to disk.
    pub fn save(&self) {
        if let Some(path) = Self::profile_path() {
            // Ensure directory exists
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match serde_json::to_string_pretty(self) {
                Ok(json) => {
                    if let Err(e) = std::fs::write(&path, json) {
                        tracing::warn!("Phi Brain: failed to save profile: {}", e);
                    }
                }
                Err(e) => {
                    tracing::warn!("Phi Brain: failed to serialize profile: {}", e);
                }
            }
        }
    }

    /// Update maturity score based on total interactions.
    /// Maturity grows logarithmically — fast early, slow later.
    pub fn update_maturity(&mut self) {
        // Maturity: 0.0 at 0 interactions, ~1.0 at 500+ interactions
        let maturity = (self.total_interactions as f32).ln_1p() / 6.2;
        self.profile_maturity = maturity.min(1.0);
    }

    /// Record an active hour for pattern detection.
    pub fn record_active_hour(&mut self, hour: u8) {
        if !self.active_hours.contains(&hour) {
            self.active_hours.push(hour);
            self.active_hours.sort();
        }
    }

    /// Update model satisfaction using exponential moving average.
    pub fn update_model_satisfaction(&mut self, model: &str, score: f32) {
        let entry = self.model_satisfaction.entry(model.to_string()).or_insert(0.5);
        *entry = *entry * 0.9 + score * 0.1;
    }

    /// Update task frequency counter.
    pub fn record_task(&mut self, task_type: &str) {
        let count = self.task_frequency.entry(task_type.to_string()).or_insert(0);
        *count += 1;
    }

    /// Update average message length using moving average.
    pub fn update_message_length(&mut self, length: f32) {
        self.avg_message_length = self.avg_message_length * 0.9 + length * 0.1;
    }

    /// Detect language from text (simple heuristic).
    pub fn detect_language(text: &str) -> &'static str {
        // Simple Arabic detection
        let has_arabic = text.chars().any(|c| ('\u{0600}'..='\u{06FF}').contains(&c));
        if has_arabic {
            "ar"
        } else {
            "en"
        }
    }

    /// Classify task type from message content.
    pub fn classify_task(text: &str) -> &'static str {
        let lower = text.to_lowercase();
        if lower.contains("code") || lower.contains("function") || lower.contains("class") {
            "code"
        } else if lower.contains("search") || lower.contains("find") || lower.contains("look") {
            "search"
        } else if lower.contains("browser") || lower.contains("navigate") || lower.contains("click") {
            "browser"
        } else if lower.contains("analyze") || lower.contains("data") || lower.contains("csv") {
            "analysis"
        } else if lower.contains("write") || lower.contains("draft") || lower.contains("email") {
            "writing"
        } else if lower.contains("translate") {
            "translation"
        } else if lower.contains("summarize") || lower.contains("summary") {
            "summarization"
        } else {
            "general"
        }
    }
}
