//! AI Provider implementations — 20+ providers with real HTTP API calls.
//!
//! Each provider reads its API key from an environment variable.
//! Proxy support: set `HTTP_PROXY` or `HTTPS_PROXY` env vars.
//!
//! # Providers
//!
//! | Provider | Env Var | Base URL | Format |
//! |----------|---------|----------|--------|
//! | OpenAI | `OPENAI_API_KEY` | api.openai.com | OpenAI |
//! | Anthropic | `ANTHROPIC_API_KEY` | api.anthropic.com | Anthropic |
//! | Google Gemini | `GOOGLE_API_KEY` | generativelanguage.googleapis.com | Gemini |
//! | Groq | `GROQ_API_KEY` | api.groq.com | OpenAI |
//! | DeepSeek | `DEEPSEEK_API_KEY` | api.deepseek.com | OpenAI |
//! | OpenRouter | `OPENROUTER_API_KEY` | openrouter.ai | OpenAI |
//! | xAI/Grok | `XAI_API_KEY` | api.x.ai | OpenAI |
//! | Fireworks | `FIREWORKS_API_KEY` | api.fireworks.ai | OpenAI |
//! | Together | `TOGETHER_API_KEY` | api.together.xyz | OpenAI |
//! | Perplexity | `PERPLEXITY_API_KEY` | api.perplexity.ai | OpenAI |
//! | Mistral | `MISTRAL_API_KEY` | api.mistral.ai | OpenAI |
//! | Moonshot/Kimi | `MOONSHOT_API_KEY` | api.moonshot.cn | OpenAI |
//! | Qwen | `QWEN_API_KEY` | dashscope.aliyuncs.com | OpenAI |
//! | MiniMax | `MINIMAX_API_KEY` | api.minimax.chat | MiniMax |
//! | Ollama | (local) | localhost:11434 | OpenAI |
//! | vLLM | `VLLM_API_KEY` | configurable | OpenAI |
//! | Azure | `AZURE_OPENAI_API_KEY` | {resource}.openai.azure.com | Azure |
//! | BytePlus | `BYTEPLUS_API_KEY` | configurable | OpenAI |
//! | Venice AI | `VENICE_API_KEY` | api.venice.ai | OpenAI |

use super::{ChatMessage, ChatResponse, ModelConfig, Usage};
use async_trait::async_trait;
use futures::StreamExt;
use parking_lot::RwLock;
use std::sync::Arc;

// =============================================================================
// Provider Trait
// =============================================================================

#[async_trait]
pub trait AiProvider: Send + Sync {
    fn name(&self) -> &str;
    async fn chat(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> anyhow::Result<ChatResponse>;
    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>>;
}

// =============================================================================
// HTTP Helper — builds a client with optional proxy support
// =============================================================================

fn build_client(timeout_secs: u64) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(timeout_secs));

    // Check for proxy env vars
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("HTTP_PROXY"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
            tracing::debug!("Using proxy: {}", proxy_url);
        }
    }

    Ok(builder.build()?)
}

// =============================================================================
// Provider Manager
// =============================================================================

pub struct ProviderManager {
    providers: RwLock<Vec<(Arc<dyn AiProvider>, u32)>>,
}

impl Default for ProviderManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderManager {
    /// Creates a ProviderManager with no pre-registered providers.
    /// Useful for tests or fully custom setups.
    pub fn new_empty() -> Self {
        Self {
            providers: RwLock::new(Vec::new()),
        }
    }

    pub fn new() -> Self {
        let pm = Self {
            providers: RwLock::new(Vec::new()),
        };
        
        // Register all OpenAI-compatible providers
        pm.register(Arc::new(OpenAIProvider::new()), 1);
        pm.register(Arc::new(GroqProvider::new()), 1);
        pm.register(Arc::new(DeepSeekProvider::new()), 1);
        pm.register(Arc::new(OpenRouterProvider::new()), 1);
        pm.register(Arc::new(XAIProvider::new()), 1);
        pm.register(Arc::new(FireworksProvider::new()), 1);
        pm.register(Arc::new(TogetherProvider::new()), 1);
        pm.register(Arc::new(PerplexityProvider::new()), 1);
        pm.register(Arc::new(MistralProvider::new()), 1);
        pm.register(Arc::new(MoonshotProvider::new()), 1);
        pm.register(Arc::new(QwenProvider::new()), 1);
        pm.register(Arc::new(VeniceProvider::new()), 1);
        pm.register(Arc::new(CohereProvider::new()), 1);
        pm.register(Arc::new(ReplicateProvider::new()), 1);
        pm.register(Arc::new(HuggingFaceProvider::new()), 1);
        pm.register(Arc::new(AnyscaleProvider::new()), 1);
        pm.register(Arc::new(LMStudioProvider::new()), 1);
        pm.register(Arc::new(LocalAIProvider::new()), 1);
        pm.register(Arc::new(GroqCloudProvider::new()), 1);
        pm.register(Arc::new(CustomOpenAIProvider::new()), 1);
        pm.register(Arc::new(SambaNovaProvider::new()), 1);
        pm.register(Arc::new(WriterProvider::new()), 1);
        pm.register(Arc::new(AI21Provider::new()), 1);
        pm.register(Arc::new(BaiduProvider::new()), 1);
        pm.register(Arc::new(AlibabaProvider::new()), 1);
        pm.register(Arc::new(TencentProvider::new()), 1);
        pm.register(Arc::new(ZhipuProvider::new()), 1);
        pm.register(Arc::new(MetaProvider::new()), 1);

        // Register local and native providers
        pm.register(Arc::new(OllamaProvider::new()), 1);
        pm.register(Arc::new(VLLMProvider::new()), 1);
        pm.register(Arc::new(AzureProvider::new()), 1);
        pm.register(Arc::new(BytePlusProvider::new()), 1);
        pm.register(Arc::new(NousProvider::new()), 1);
        pm.register(Arc::new(GLMProvider::new()), 1);
                pm.register(Arc::new(GoogleProvider::new()), 1);
        pm.register(Arc::new(MiniMaxProvider::new()), 1);
        pm.register(Arc::new(AnthropicProvider::new()), 1);

        pm
    }

    pub fn register(&self, provider: Arc<dyn AiProvider>, priority: u32) {
        self.providers.write().push((provider, priority));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn AiProvider>> {
        self.providers
            .read()
            .iter()
            .find(|(p, _)| p.name() == name)
            .map(|(p, _)| p.clone())
    }

    pub fn get_best(&self) -> Option<Arc<dyn AiProvider>> {
        self.providers
            .read()
            .iter()
            .max_by_key(|(_, priority)| *priority)
            .map(|(p, _)| p.clone())
    }

    pub fn list_providers(&self) -> Vec<String> {
        self.providers
            .read()
            .iter()
            .map(|(p, _)| p.name().to_string())
            .collect()
    }
}

// =============================================================================
// OpenAI-Compatible response parser (standalone — used by macro and custom providers)
// =============================================================================

fn parse_openai_response(body: &serde_json::Value, model_name: &str) -> anyhow::Result<ChatResponse> {
    let content = body["choices"][0]["message"]["content"]
        .as_str().unwrap_or("").to_string();
    let finish_reason = body["choices"][0]["finish_reason"]
        .as_str().unwrap_or("stop").to_string();
    let usage = body.get("usage").map(|u| Usage {
        prompt_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        completion_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        total_tokens: u.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
    }).unwrap_or(Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 });
    Ok(ChatResponse { content, model: model_name.to_string(), usage, finish_reason })
}

// =============================================================================
// OpenAI-Compatible Provider (generic — covers 15+ APIs)
// =============================================================================

macro_rules! openai_compatible_provider {
    ($name:ident, $display:expr, $base_url:expr, $env_key:expr) => {
        paste::paste! {
            #[derive(Default)]
            pub struct $name;

            impl $name {
                pub fn new() -> Self { Self }

                fn build_request_body(
                    messages: &[ChatMessage],
                    config: &ModelConfig,
                    stream: bool,
                ) -> serde_json::Value {
                    let msgs: Vec<serde_json::Value> = messages.iter().map(|m| {
                        serde_json::json!({"role": m.role, "content": m.content})
                    }).collect();
                    serde_json::json!({
                        "model": config.model,
                        "messages": msgs,
                        "max_tokens": config.max_tokens,
                        "temperature": config.temperature,
                        "top_p": config.top_p,
                        "stream": stream,
                    })
                }

                async fn do_chat(
                    messages: &[ChatMessage],
                    config: &ModelConfig,
                    stream: bool,
                ) -> anyhow::Result<ChatResponse> {
                    let api_key = crate::get_api_key($env_key, $display)
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    let client = build_client(120)?;
                    let body = Self::build_request_body(messages, config, stream);
                    
                    // Resolve dynamic base URL from registry
                    let base_url = super::provider_registry::ProviderRegistry::new()
                        .resolve_base_url(&config.provider)
                        .unwrap_or_else(|| $base_url.to_string());
                    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

                    let response = client
                        .post(&url)
                        .header("Authorization", format!("Bearer {}", api_key))
                        .json(&body)
                        .send()
                        .await
                        .map_err(|e| anyhow::anyhow!("{} request failed: {}", $display, e))?;
                    let status = response.status();
                    if !status.is_success() {
                        crate::report_api_failure($display, &api_key);
                        let body_snippet = response.text().await.unwrap_or_default();
                        let safe = if body_snippet.len() > 200 {
                            format!("{}… (truncated)", &body_snippet[..200])
                        } else {
                            crate::ai::redact_sensitive(&body_snippet)
                        };
                        return Err(anyhow::anyhow!("{} returned HTTP {}: {}", $display, status, safe));
                    }
                    crate::report_api_success($display, &api_key);
                    let body: serde_json::Value = response.json().await?;
                    parse_openai_response(&body, $display)
                }
            }

            impl $name {
                async fn do_chat_stream(
                    messages: &[ChatMessage],
                    config: &ModelConfig,
                ) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
                    let api_key = crate::get_api_key($env_key, $display)
                        .map_err(|e| anyhow::anyhow!("{}", e))?;
                    let (tx, rx) = tokio::sync::mpsc::channel(100);
                    let body = Self::build_request_body(messages, config, true);
                    
                    // Resolve dynamic base URL from registry
                    let base_url = super::provider_registry::ProviderRegistry::new()
                        .resolve_base_url(&config.provider)
                        .unwrap_or_else(|| $base_url.to_string());
                    let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

                    let provider_name = $display;
                    tokio::spawn(async move {
                        let client = match build_client(120) {
                            Ok(c) => c,
                            Err(e) => { let _ = tx.send(format!("Error: {}", e)).await; return; }
                        };
                        let response = match client
                            .post(&url)
                            .header("Authorization", format!("Bearer {}", api_key))
                            .json(&body)
                            .send().await
                        {
                            Ok(r) => r,
                            Err(e) => { let _ = tx.send(format!("Error: {}", e)).await; return; }
                        };
                        let status = response.status();
                        if !status.is_success() {
                            crate::report_api_failure(provider_name, &api_key);
                            let _ = tx.send(format!("HTTP {}", status)).await;
                            return;
                        }
                        crate::report_api_success(provider_name, &api_key);
                        let mut stream = response.bytes_stream();
                        let mut buffer = String::new();
                        while let Some(chunk_result) = stream.next().await {
                            let chunk = chunk_result.unwrap_or_default();
                            let chunk_str = String::from_utf8_lossy(&chunk).into_owned();
                            buffer.push_str(&chunk_str);
                            while let Some(nl_pos) = buffer.find('\n') {
                                let line = buffer[..nl_pos].trim().to_string();
                                buffer = buffer[nl_pos + 1..].to_string();
                                if let Some(data) = line.strip_prefix("data: ") {
                                    if data == "[DONE]" { return; }
                                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                                            if !content.is_empty() && tx.send(content.to_string()).await.is_err() { return; }
                                        }
                                    }
                                }
                            }
                        }
                        tracing::debug!("{} stream finished", provider_name);
                    });
                    Ok(rx)
                }
            }

            #[async_trait]
            impl AiProvider for $name {
                fn name(&self) -> &str { $display }

                async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
                    Self::do_chat(messages, config, false).await
                }

                async fn chat_stream(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
                    Self::do_chat_stream(messages, config).await
                }
            }
        }
    };
}

// =============================================================================
// Register all OpenAI-compatible providers
// =============================================================================

// (name, display_name, base_url, env_var)
openai_compatible_provider!(OpenAIProvider, "openai", "https://api.openai.com/v1", "OPENAI_API_KEY");
openai_compatible_provider!(GoogleProvider, "google", "https://gemini.134.209.188.103.nip.io/v1", "GOOGLE_API_KEY");
openai_compatible_provider!(GroqProvider, "groq", "https://api.groq.com/openai/v1", "GROQ_API_KEY");
openai_compatible_provider!(DeepSeekProvider, "deepseek", "https://api.deepseek.com/v1", "DEEPSEEK_API_KEY");
openai_compatible_provider!(OpenRouterProvider, "openrouter", "https://openrouter.ai/api/v1", "OPENROUTER_API_KEY");
openai_compatible_provider!(XAIProvider, "xai", "https://api.x.ai/v1", "XAI_API_KEY");
openai_compatible_provider!(FireworksProvider, "fireworks", "https://api.fireworks.ai/inference/v1", "FIREWORKS_API_KEY");
openai_compatible_provider!(TogetherProvider, "together", "https://api.together.xyz/v1", "TOGETHER_API_KEY");
openai_compatible_provider!(PerplexityProvider, "perplexity", "https://api.perplexity.ai", "PERPLEXITY_API_KEY");
openai_compatible_provider!(MistralProvider, "mistral", "https://api.mistral.ai/v1", "MISTRAL_API_KEY");
openai_compatible_provider!(MoonshotProvider, "moonshot", "https://api.moonshot.cn/v1", "MOONSHOT_API_KEY");
openai_compatible_provider!(QwenProvider, "qwen", "https://dashscope.aliyuncs.com/compatible-mode/v1", "QWEN_API_KEY");
openai_compatible_provider!(VeniceProvider, "venice", "https://api.venice.ai/api/v1", "VENICE_API_KEY");

// Missing and Custom OpenAI-Compatible Providers
openai_compatible_provider!(CohereProvider, "cohere", "https://api.cohere.com/v2", "COHERE_API_KEY");
openai_compatible_provider!(ReplicateProvider, "replicate", "https://api.replicate.com/v1", "REPLICATE_API_KEY");
openai_compatible_provider!(HuggingFaceProvider, "huggingface", "https://router.huggingface.co/v1", "HF_TOKEN");
openai_compatible_provider!(AnyscaleProvider, "anyscale", "https://api.endpoints.anyscale.com/v1", "ANYSCALE_API_KEY");
openai_compatible_provider!(LMStudioProvider, "lmstudio", "http://localhost:1234/v1", "LMSTUDIO_API_KEY");
openai_compatible_provider!(LocalAIProvider, "localai", "http://localhost:8080/v1", "LOCALAI_API_KEY");
openai_compatible_provider!(GroqCloudProvider, "groqcloud", "https://api.groq.com/openai/v1", "GROQ_API_KEY");
openai_compatible_provider!(CustomOpenAIProvider, "custom_openai", "http://localhost:8080/v1", "CUSTOM_OPENAI_API_KEY");
openai_compatible_provider!(SambaNovaProvider, "sambanova", "https://api.sambanova.ai/v1", "SAMBANOVA_API_KEY");
openai_compatible_provider!(WriterProvider, "writer", "https://api.writer.com/v1", "WRITER_API_KEY");
openai_compatible_provider!(AI21Provider, "ai21", "https://api.ai21.com/v1", "AI21_API_KEY");
openai_compatible_provider!(BaiduProvider, "baidu", "https://qianfan.baidubce.com/v2", "BAIDU_API_KEY");
openai_compatible_provider!(AlibabaProvider, "alibaba", "https://dashscope-intl.aliyuncs.com/compatible-mode/v1", "DASHSCOPE_API_KEY");
openai_compatible_provider!(TencentProvider, "tencent", "https://hunyuan.tencentcloudapi.com", "TENCENT_API_KEY");
openai_compatible_provider!(ZhipuProvider, "zhipu", "https://open.bigmodel.cn/api/paas/v4", "ZHIPU_API_KEY");
openai_compatible_provider!(MetaProvider, "meta", "https://api.meta.com/v1", "META_API_KEY");

// =============================================================================
// Ollama Provider (local — no API key needed)
// =============================================================================

pub struct OllamaProvider {
    base_url: String,
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self {
            base_url: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        }
    }

    #[allow(dead_code)]
    fn list_local_models() -> Vec<String> {
        // Check common models that might be pulled locally
        let candidates = vec![
            "llama3.2", "llama3.1:70b", "llama3.1:8b", "codellama:70b",
            "codellama:34b", "codellama:7b", "mistral", "mixtral",
            "deepseek-r1:70b", "deepseek-r1:7b", "qwen2.5:72b",
            "qwen2.5:7b", "gemma2:27b", "gemma2:9b", "phi3:14b",
            "phi3:mini", "neural-chat", "starling-lm",
        ];
        candidates.into_iter().map(|s| s.to_string()).collect()
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn chat(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> anyhow::Result<ChatResponse> {
        let client = build_client(300)?;
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": msgs,
            "stream": false,
            "options": {
                "temperature": config.temperature,
                "num_predict": config.max_tokens,
                "top_p": config.top_p,
            }
        });

        let response = client
            .post(format!("{}/api/chat", self.base_url))
            .json(&body)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("Ollama request failed: {}", e))?;

        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!("Ollama returned HTTP {}: {}", status, text));
        }

        let body: serde_json::Value = response.json().await?;
        let content = body["message"]["content"].as_str().unwrap_or("").to_string();

        let usage = Usage {
            prompt_tokens: body["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
            completion_tokens: body["eval_count"].as_u64().unwrap_or(0) as u32,
            total_tokens: 0,
        };

        Ok(ChatResponse {
            content,
            model: config.model.clone(),
            usage,
            finish_reason: "stop".to_string(),
        })
    }

    async fn chat_stream(
        &self,
        messages: &[ChatMessage],
        config: &ModelConfig,
    ) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let base_url = self.base_url.clone();
        let msgs: Vec<serde_json::Value> = messages
            .iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let model = config.model.clone();

        tokio::spawn(async move {
            let client = match build_client(300) {
                Ok(c) => c,
                Err(e) => { let _ = tx.send(format!("Error: {}", e)).await; return; }
            };

            let body = serde_json::json!({
                "model": model,
                "messages": msgs,
                "stream": true,
                "options": { "temperature": 0.7, "num_predict": 4096 }
            });

            let response = match client
                .post(format!("{}/api/chat", base_url))
                .json(&body)
                .send().await
            {
                Ok(r) => r,
                Err(e) => { let _ = tx.send(format!("Error: {}", e)).await; return; }
            };

            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.unwrap_or_default();
                let chunk_str = String::from_utf8_lossy(&chunk);
                buffer.push_str(&chunk_str);
                while let Some(nl_pos) = buffer.find('\n') {
                    let line = buffer[..nl_pos].trim().to_string();
                    buffer = buffer[nl_pos + 1..].to_string();
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&line) {
                        if json.get("done").and_then(|v| v.as_bool()).unwrap_or(false) {
                            return;
                        }
                        if let Some(content) = json["message"]["content"].as_str() {
                            if !content.is_empty() && tx.send(content.to_string()).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            }
        });

        Ok(rx)
    }
}

// =============================================================================
// Anthropic Provider
// =============================================================================

pub struct AnthropicProvider;

impl Default for AnthropicProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AnthropicProvider {
    pub fn new() -> Self { Self }

    fn build_request_body(messages: &[ChatMessage], config: &ModelConfig) -> serde_json::Value {
        let filtered: Vec<serde_json::Value> = messages
            .iter().filter(|m| m.role != "system")
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let system = messages.iter().find(|m| m.role == "system").map(|m| m.content.as_str());
        let mut body = serde_json::json!({
            "model": config.model,
            "messages": filtered,
            "max_tokens": config.max_tokens.min(8192),
        });
        if let Some(sys) = system {
            body["system"] = serde_json::Value::String(sys.to_string());
        }
        body
    }
}

#[async_trait]
impl AiProvider for AnthropicProvider {
    fn name(&self) -> &str { "anthropic" }

    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let api_key = crate::get_api_key("ANTHROPIC_API_KEY", "anthropic")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let client = build_client(120)?;
        let body = Self::build_request_body(messages, config);
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send().await
            .map_err(|e| anyhow::anyhow!("Anthropic request failed: {}", e))?;
        let status = response.status();
        if !status.is_success() {
            crate::report_api_failure("anthropic", &api_key);
            return Err(anyhow::anyhow!("Anthropic HTTP {}: {}", status, response.text().await.unwrap_or_default()));
        }
        crate::report_api_success("anthropic", &api_key);
        let body: serde_json::Value = response.json().await?;
        let content = body["content"].as_array()
            .and_then(|a| a.first())
            .and_then(|b| b["text"].as_str())
            .unwrap_or("").to_string();
        let usage = body.get("usage").map(|u| Usage {
            prompt_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            completion_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
            total_tokens: 0,
        }).unwrap_or(Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 });
        Ok(ChatResponse { content, model: config.model.clone(), usage, finish_reason: "stop".to_string() })
    }

    async fn chat_stream(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        let api_key = crate::get_api_key("ANTHROPIC_API_KEY", "anthropic")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let (tx, rx) = tokio::sync::mpsc::channel(100);
        let body = serde_json::json!({
            "model": config.model,
            "messages": messages.iter().filter(|m| m.role != "system")
                .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
                .collect::<Vec<_>>(),
            "max_tokens": config.max_tokens.min(8192),
            "stream": true,
        });
        tokio::spawn(async move {
            let client = match build_client(120) { Ok(c) => c, Err(e) => { let _ = tx.send(format!("Error: {}", e)).await; return; } };
            let response = match client.post("https://api.anthropic.com/v1/messages")
                .header("x-api-key", &api_key).header("anthropic-version", "2023-06-01")
                .json(&body).send().await
            {
                Ok(r) => r,
                Err(e) => { let _ = tx.send(format!("Error: {}", e)).await; return; }
            };
            if !response.status().is_success() {
                crate::report_api_failure("anthropic", &api_key);
                return;
            }
            crate::report_api_success("anthropic", &api_key);
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();
            while let Some(chunk_result) = stream.next().await {
                let chunk = chunk_result.unwrap_or_default();
                let chunk_str = String::from_utf8_lossy(&chunk).into_owned();
                buffer.push_str(&chunk_str);
                while let Some(nl_pos) = buffer.find('\n') {
                    let line = buffer[..nl_pos].trim().to_string();
                    buffer = buffer[nl_pos + 1..].to_string();
                    if let Some(data) = line.strip_prefix("data: ") {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {

                            if let Some(text) = json["candidates"].as_array()
                                .and_then(|a| a.first())
                                .and_then(|c| c["content"]["parts"].as_array())
                                .and_then(|p| p.first())
                                .and_then(|p| p["text"].as_str())
                            {
                                if !text.is_empty() && tx.send(text.to_string()).await.is_err() { return; }
                            }
                        }
                    }
                }
            }
        });
        Ok(rx)
    }
}

// =============================================================================
// MiniMax Provider (native API format)
// =============================================================================

pub struct MiniMaxProvider;

impl Default for MiniMaxProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl MiniMaxProvider {
    pub fn new() -> Self { Self }

    async fn do_chat(messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let api_key = crate::get_api_key("MINIMAX_API_KEY", "minimax")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let group_id = std::env::var("MINIMAX_GROUP_ID")
            .unwrap_or_else(|_| "default".to_string());
        let client = build_client(60)?;

        let msgs: Vec<serde_json::Value> = messages.iter().map(|m| {
            serde_json::json!({"role": m.role, "text": m.content})
        }).collect();

        let body = serde_json::json!({
            "model": config.model,
            "messages": msgs,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
            "top_p": config.top_p,
        });

        let url = format!(
            "https://api.minimax.chat/v1/text/chatcompletion_v2?GroupId={}",
            group_id
        );

        let response = client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send().await
            .map_err(|e| anyhow::anyhow!("MiniMax request failed: {}", e))?;

        if !response.status().is_success() {
            crate::report_api_failure("minimax", &api_key);
            return Err(anyhow::anyhow!("MiniMax HTTP {}: {}", response.status(), response.text().await.unwrap_or_default()));
        }
        crate::report_api_success("minimax", &api_key);

        let body: serde_json::Value = response.json().await?;
        let content = body["reply"].as_str().unwrap_or("").to_string();
        Ok(ChatResponse { content, model: config.model.clone(), usage: Usage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 }, finish_reason: "stop".to_string() })
    }
}

#[async_trait]
impl AiProvider for MiniMaxProvider {
    fn name(&self) -> &str { "minimax" }
    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        Self::do_chat(messages, config).await
    }
    async fn chat_stream(&self, _messages: &[ChatMessage], _config: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        Err(anyhow::anyhow!("MiniMax streaming not yet supported"))
    }
}

// =============================================================================
// Azure OpenAI Provider
// =============================================================================

pub struct AzureProvider;

impl Default for AzureProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl AzureProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl AiProvider for AzureProvider {
    fn name(&self) -> &str { "azure" }

    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let api_key = crate::get_api_key("AZURE_OPENAI_API_KEY", "azure")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let resource = std::env::var("AZURE_OPENAI_RESOURCE")
            .map_err(|_| anyhow::anyhow!("AZURE_OPENAI_RESOURCE not set"))?;
        let deployment = std::env::var("AZURE_OPENAI_DEPLOYMENT")
            .unwrap_or_else(|_| config.model.clone());

        let client = build_client(120)?;
        let url = format!(
            "https://{}.openai.azure.com/openai/deployments/{}/chat/completions?api-version=2024-08-01-preview",
            resource, deployment
        );

        let msgs: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();

        let body = serde_json::json!({
            "messages": msgs,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
        });

        let response = client.post(&url)
            .header("api-key", &api_key)
            .json(&body)
            .send().await
            .map_err(|e| anyhow::anyhow!("Azure request failed: {}", e))?;

        if !response.status().is_success() {
            crate::report_api_failure("azure", &api_key);
            return Err(anyhow::anyhow!("Azure HTTP {}: {}", response.status(), response.text().await.unwrap_or_default()));
        }
        crate::report_api_success("azure", &api_key);

        let body: serde_json::Value = response.json().await?;
        parse_openai_response(&body, "azure")
    }

    async fn chat_stream(&self, _messages: &[ChatMessage], _config: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        Err(anyhow::anyhow!("Azure streaming not yet supported"))
    }
}

// =============================================================================
// BytePlus/Volcano Engine Provider
// =============================================================================

pub struct BytePlusProvider;

impl Default for BytePlusProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl BytePlusProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl AiProvider for BytePlusProvider {
    fn name(&self) -> &str { "byteplus" }

    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let api_key = crate::get_api_key("BYTEPLUS_API_KEY", "byteplus")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let endpoint = std::env::var("BYTEPLUS_ENDPOINT")
            .unwrap_or_else(|_| "https://ark.cn-beijing.volces.com".to_string());
        let client = build_client(120)?;

        let msgs: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let body = serde_json::json!({"model": config.model, "messages": msgs});

        let response = client.post(format!("{}/api/v3/chat/completions", endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body).send().await
            .map_err(|e| anyhow::anyhow!("BytePlus request failed: {}", e))?;

        if !response.status().is_success() {
            crate::report_api_failure("byteplus", &api_key);
            return Err(anyhow::anyhow!("BytePlus HTTP {}: {}", response.status(), response.text().await.unwrap_or_default()));
        }
        crate::report_api_success("byteplus", &api_key);

        let body: serde_json::Value = response.json().await?;
        parse_openai_response(&body, "byteplus")
    }

    async fn chat_stream(&self, _: &[ChatMessage], _: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        Err(anyhow::anyhow!("BytePlus streaming not yet supported"))
    }
}

// =============================================================================
// vLLM Provider (local — configurable URL)
// =============================================================================

pub struct VLLMProvider {
    base_url: String,
}

impl Default for VLLMProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl VLLMProvider {
    pub fn new() -> Self {
        Self {
            base_url: std::env::var("VLLM_HOST")
                .unwrap_or_else(|_| "http://localhost:8000".to_string()),
        }
    }
}

#[async_trait]
impl AiProvider for VLLMProvider {
    fn name(&self) -> &str { "vllm" }

    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let client = build_client(300)?;
        let msgs: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let body = serde_json::json!({
            "model": config.model,
            "messages": msgs,
            "max_tokens": config.max_tokens,
            "temperature": config.temperature,
        });
        let response = client.post(format!("{}/v1/chat/completions", self.base_url))
            .json(&body).send().await
            .map_err(|e| anyhow::anyhow!("vLLM request failed: {}", e))?;
        if !response.status().is_success() {
            return Err(anyhow::anyhow!("vLLM HTTP {}: {}", response.status(), response.text().await.unwrap_or_default()));
        }
        let body: serde_json::Value = response.json().await?;
        parse_openai_response(&body, "vllm")
    }

    async fn chat_stream(&self, _: &[ChatMessage], _: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        Err(anyhow::anyhow!("vLLM streaming not yet supported"))
    }
}

// =============================================================================
// Nous Portal Provider
// =============================================================================

pub struct NousProvider;

impl Default for NousProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl NousProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl AiProvider for NousProvider {
    fn name(&self) -> &str { "nous" }

    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let api_key = crate::get_api_key("NOUS_API_KEY", "nous")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let client = build_client(120)?;
        let msgs: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let body = serde_json::json!({"model": config.model, "messages": msgs, "max_tokens": config.max_tokens});
        let response = client.post("https://api.nousresearch.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body).send().await
            .map_err(|e| anyhow::anyhow!("Nous request failed: {}", e))?;
        if !response.status().is_success() {
            crate::report_api_failure("nous", &api_key);
            return Err(anyhow::anyhow!("Nous HTTP {}: {}", response.status(), response.text().await.unwrap_or_default()));
        }
        crate::report_api_success("nous", &api_key);
        let body: serde_json::Value = response.json().await?;
        parse_openai_response(&body, "nous")
    }

    async fn chat_stream(&self, _: &[ChatMessage], _: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        Err(anyhow::anyhow!("Nous streaming not yet supported"))
    }
}

// =============================================================================
// GLM (z.ai) Provider
// =============================================================================

pub struct GLMProvider;

impl Default for GLMProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl GLMProvider {
    pub fn new() -> Self { Self }
}

#[async_trait]
impl AiProvider for GLMProvider {
    fn name(&self) -> &str { "glm" }

    async fn chat(&self, messages: &[ChatMessage], config: &ModelConfig) -> anyhow::Result<ChatResponse> {
        let api_key = crate::get_api_key("GLM_API_KEY", "glm")
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        let client = build_client(120)?;
        let msgs: Vec<serde_json::Value> = messages.iter()
            .map(|m| serde_json::json!({"role": m.role, "content": m.content}))
            .collect();
        let body = serde_json::json!({"model": config.model, "messages": msgs, "max_tokens": config.max_tokens});
        let response = client.post("https://open.bigmodel.cn/api/paas/v4/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body).send().await
            .map_err(|e| anyhow::anyhow!("GLM request failed: {}", e))?;
        if !response.status().is_success() {
            crate::report_api_failure("glm", &api_key);
            return Err(anyhow::anyhow!("GLM HTTP {}: {}", response.status(), response.text().await.unwrap_or_default()));
        }
        crate::report_api_success("glm", &api_key);
        let body: serde_json::Value = response.json().await?;
        parse_openai_response(&body, "glm")
    }

    async fn chat_stream(&self, _: &[ChatMessage], _: &ModelConfig) -> anyhow::Result<tokio::sync::mpsc::Receiver<String>> {
        Err(anyhow::anyhow!("GLM streaming not yet supported"))
    }
}
