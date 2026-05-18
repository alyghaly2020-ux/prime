/// Unified Ollama client for Phi Brain.
///
/// Single shared HTTP client used by all Phi Brain subsystems
/// (orchestrator, proofreader, guardian, learner) instead of
/// each creating its own reqwest::Client.

use std::sync::Arc;

/// Shared Ollama client configuration and HTTP connection.
#[derive(Clone)]
pub struct OllamaClient {
    client: reqwest::Client,
    base_url: String,
    model: String,
}

impl OllamaClient {
    /// Create a new Ollama client.
    ///
    /// Reads `OLLAMA_HOST` (default: `http://localhost:11434`) and
    /// `PHI_BRAIN_MODEL` (default: `phi4-mini`) from environment.
    pub fn new() -> Self {
        let base_url = std::env::var("OLLAMA_HOST")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = std::env::var("PHI_BRAIN_MODEL")
            .unwrap_or_else(|_| "phi4-mini".to_string());

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .pool_max_idle_per_host(2)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            base_url,
            model,
        }
    }

    /// Check if Ollama is reachable (3s timeout).
    pub async fn check_health(&self) -> anyhow::Result<()> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .timeout(std::time::Duration::from_secs(3))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(anyhow::anyhow!("Ollama returned status {}", resp.status()))
        }
    }

    /// Generate text using the Phi model.
    ///
    /// # Arguments
    /// * `prompt` - The prompt to send to the model
    /// * `temperature` - Sampling temperature (0.0 - 1.0)
    /// * `max_tokens` - Maximum tokens to generate
    pub async fn generate(
        &self,
        prompt: &str,
        temperature: f32,
        max_tokens: u32,
    ) -> anyhow::Result<String> {
        let body = serde_json::json!({
            "model": self.model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": temperature,
                "num_predict": max_tokens,
            }
        });

        let resp = self
            .client
            .post(format!("{}/api/generate", self.base_url))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Ollama returned HTTP {}: {}",
                status,
                &text[..text.len().min(200)]
            ));
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(body["response"].as_str().unwrap_or("").to_string())
    }

    /// Get the model name.
    pub fn model_name(&self) -> &str {
        &self.model
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Create a shared Arc for use across subsystems.
    pub fn shared(self) -> Arc<Self> {
        Arc::new(self)
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}
