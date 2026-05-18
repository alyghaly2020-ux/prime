use super::ModelConfig;
use parking_lot::RwLock;
use std::collections::HashMap;

pub struct ModelRegistry {
    models: RwLock<HashMap<String, ModelConfig>>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    pub fn new() -> Self {
        let mut models = HashMap::new();

        // GPT-5
        models.insert(
            "gpt-5".to_string(),
            ModelConfig {
                id: "gpt-5".to_string(),
                provider: "openai".to_string(),
                model: "gpt-5".to_string(),
                max_tokens: 128_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // Claude
        models.insert(
            "claude-4".to_string(),
            ModelConfig {
                id: "claude-4".to_string(),
                provider: "anthropic".to_string(),
                model: "claude-sonnet-4".to_string(),
                max_tokens: 200_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // Gemini
        models.insert(
            "gemini-2".to_string(),
            ModelConfig {
                id: "gemini-2".to_string(),
                provider: "google".to_string(),
                model: "gemini-2.0-flash".to_string(),
                max_tokens: 1_000_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // Groq
        models.insert(
            "groq-llama".to_string(),
            ModelConfig {
                id: "groq-llama".to_string(),
                provider: "groq".to_string(),
                model: "llama-3.3-70b-versatile".to_string(),
                max_tokens: 4_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        models.insert(
            "groq-fast".to_string(),
            ModelConfig {
                id: "groq-fast".to_string(),
                provider: "groq".to_string(),
                model: "llama-3.1-8b-instant".to_string(),
                max_tokens: 4_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // Ollama (local)
        models.insert(
            "ollama-codellama".to_string(),
            ModelConfig {
                id: "ollama-codellama".to_string(),
                provider: "ollama".to_string(),
                model: "codellama:70b".to_string(),
                max_tokens: 16_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // OpenRouter
        models.insert(
            "openrouter-best".to_string(),
            ModelConfig {
                id: "openrouter-best".to_string(),
                provider: "openrouter".to_string(),
                model: "openrouter/auto".to_string(),
                max_tokens: 128_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // Mistral
        models.insert(
            "mistral-large".to_string(),
            ModelConfig {
                id: "mistral-large".to_string(),
                provider: "mistral".to_string(),
                model: "mistral-large-latest".to_string(),
                max_tokens: 32_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // LocalAI
        models.insert(
            "local-ai".to_string(),
            ModelConfig {
                id: "local-ai".to_string(),
                provider: "localai".to_string(),
                model: "local-model".to_string(),
                max_tokens: 8_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // Custom OpenAI API
        models.insert(
            "custom_openai".to_string(),
            ModelConfig {
                id: "custom_openai".to_string(),
                provider: "custom_openai".to_string(),
                model: "custom-openai-model".to_string(),
                max_tokens: 128_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        // DeepSeek
        models.insert(
            "deepseek".to_string(),
            ModelConfig {
                id: "deepseek".to_string(),
                provider: "deepseek".to_string(),
                model: "deepseek-chat".to_string(),
                max_tokens: 64_000,
                temperature: 0.7,
                top_p: 0.95,
                streaming: true,
            },
        );

        Self {
            models: RwLock::new(models),
        }
    }

    pub fn get_config(&self, id: &str) -> Option<ModelConfig> {
        self.models.read().get(id).cloned()
    }

    /// Add a model config
    pub fn add_config(&self, config: ModelConfig) {
        self.models.write().insert(config.id.clone(), config);
    }

    /// Remove a model config by ID
    pub fn remove_config(&self, id: &str) -> Option<ModelConfig> {
        self.models.write().remove(id)
    }

    /// Register a model config (alias for add_config)
    pub fn register(&self, config: ModelConfig) {
        self.add_config(config);
    }

    pub fn list_all(&self) -> Vec<ModelConfig> {
        self.models.read().values().cloned().collect()
    }
}
