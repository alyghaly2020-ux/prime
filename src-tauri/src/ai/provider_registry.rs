use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    ApiKey,
    OAuthDeviceCode,
    OAuthExternal,
    ExternalProcess,
    AwsSdk,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub auth_type: AuthType,
    pub inference_base_url: String,
    pub api_key_env_vars: Vec<String>,
    pub base_url_env_var: Option<String>,
    pub api_mode: ApiMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ApiMode {
    /// OpenAI-compatible /v1/chat/completions
    OpenAI,
    /// Anthropic Messages API /v1/messages
    Anthropic,
    /// Google Gemini generateContent
    Gemini,
    /// Native format (e.g. MiniMax)
    Native,
}

impl ProviderConfig {
    pub fn api_key_env_vars(&self) -> &[String] {
        &self.api_key_env_vars
    }
}

pub struct ProviderRegistry {
    providers: HashMap<&'static str, Arc<ProviderConfig>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        let mut providers: HashMap<&'static str, Arc<ProviderConfig>> = HashMap::new();

        macro_rules! define {
            ($id:expr, $name:expr, $auth:expr, $url:expr, $mode:expr$(,)?) => {
                define!($id, $name, $auth, $url, $mode, Vec::<String>::new(), Option::<String>::None)
            };
            ($id:expr, $name:expr, $auth:expr, $url:expr, $mode:expr, $keys:expr$(,)?) => {
                define!($id, $name, $auth, $url, $mode, $keys, Option::<String>::None)
            };
            ($id:expr, $name:expr, $auth:expr, $url:expr, $mode:expr, $keys:expr, $url_env:expr$(,)?) => {
                providers.insert(
                    $id,
                    Arc::new(ProviderConfig {
                        id: $id.to_string(),
                        name: $name.to_string(),
                        auth_type: $auth,
                        inference_base_url: $url.to_string(),
                        api_key_env_vars: $keys.iter().map(|s| s.to_string()).collect(),
                        base_url_env_var: $url_env.map(|s| s.to_string()),
                        api_mode: $mode,
                    }),
                );
            };
        }

        use AuthType::*;
        use ApiMode::*;

        // ── OAuth Providers ──
        define!("nous", "Nous Portal", OAuthDeviceCode, "https://api.nousresearch.com/v1", OpenAI);
        define!("openai-codex", "OpenAI Codex", OAuthExternal, "https://api.openai.com/v1", OpenAI);
        define!("qwen-oauth", "Qwen OAuth", OAuthExternal, "https://dashscope.aliyuncs.com/compatible-mode/v1", OpenAI);
        define!("google-gemini-cli", "Google Gemini (OAuth)", OAuthExternal, "https://generativelanguage.googleapis.com/v1beta", Gemini);

        // ── API Key Providers ──
        define!("copilot", "GitHub Copilot", ApiKey,
            "https://models.github.ai/inference", OpenAI,
            &["COPILOT_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"], Some("COPILOT_API_BASE_URL"));
        define!("copilot-acp", "GitHub Copilot ACP", ExternalProcess,
            "https://api.githubcopilot.com", OpenAI,
            Vec::<String>::new(), Some("COPILOT_ACP_BASE_URL".to_string()));
        define!("google", "Google AI Studio", ApiKey,
            "https://generativelanguage.googleapis.com/v1beta", Gemini,
            &["GOOGLE_API_KEY", "GEMINI_API_KEY"], Some("GEMINI_BASE_URL"));
        define!("zai", "Z.AI / GLM", ApiKey,
            "https://api.z.ai/api/paas/v4", OpenAI,
            &["GLM_API_KEY", "ZAI_API_KEY", "Z_AI_API_KEY"], Some("GLM_BASE_URL"));
        define!("kimi-coding", "Kimi / Moonshot", ApiKey,
            "https://api.moonshot.ai/v1", OpenAI,
            &["KIMI_API_KEY"], Some("KIMI_BASE_URL"));
        define!("kimi-coding-cn", "Kimi / Moonshot (China)", ApiKey,
            "https://api.moonshot.cn/v1", OpenAI,
            &["KIMI_CN_API_KEY"]);
        define!("arcee", "Arcee AI", ApiKey,
            "https://api.arcee.ai/api/v1", OpenAI,
            &["ARCEEAI_API_KEY"], Some("ARCEE_BASE_URL"));
        define!("minimax", "MiniMax", ApiKey,
            "https://api.minimax.io/anthropic", Anthropic,
            &["MINIMAX_API_KEY"], Some("MINIMAX_BASE_URL"));
        define!("anthropic", "Anthropic", ApiKey,
            "https://api.anthropic.com", Anthropic,
            &["ANTHROPIC_API_KEY", "ANTHROPIC_TOKEN", "CLAUDE_CODE_OAUTH_TOKEN"]);
        define!("alibaba", "Alibaba Cloud (DashScope)", ApiKey,
            "https://dashscope-intl.aliyuncs.com/compatible-mode/v1", OpenAI,
            &["DASHSCOPE_API_KEY"], Some("DASHSCOPE_BASE_URL"));
        define!("minimax-cn", "MiniMax (China)", ApiKey,
            "https://api.minimaxi.com/anthropic", Anthropic,
            &["MINIMAX_CN_API_KEY"], Some("MINIMAX_CN_BASE_URL"));
        define!("deepseek", "DeepSeek", ApiKey,
            "https://api.deepseek.com/v1", OpenAI,
            &["DEEPSEEK_API_KEY"], Some("DEEPSEEK_BASE_URL"));
        define!("xai", "xAI", ApiKey,
            "https://api.x.ai/v1", OpenAI,
            &["XAI_API_KEY"], Some("XAI_BASE_URL"));
        define!("nvidia", "NVIDIA NIM", ApiKey,
            "https://integrate.api.nvidia.com/v1", OpenAI,
            &["NVIDIA_API_KEY"], Some("NVIDIA_BASE_URL"));
        define!("ai-gateway", "Vercel AI Gateway", ApiKey,
            "https://ai-gateway.vercel.sh/v1", OpenAI,
            &["AI_GATEWAY_API_KEY"], Some("AI_GATEWAY_BASE_URL"));
        define!("opencode-zen", "OpenCode Zen", ApiKey,
            "https://opencode.ai/zen/v1", OpenAI,
            &["OPENCODE_ZEN_API_KEY"], Some("OPENCODE_ZEN_BASE_URL"));
        define!("opencode-go", "OpenCode Go", ApiKey,
            "https://opencode.ai/zen/go/v1", OpenAI,
            &["OPENCODE_GO_API_KEY"], Some("OPENCODE_GO_BASE_URL"));
        define!("kilocode", "Kilo Code", ApiKey,
            "https://api.kilo.ai/api/gateway", OpenAI,
            &["KILOCODE_API_KEY"], Some("KILOCODE_BASE_URL"));
        define!("huggingface", "Hugging Face", ApiKey,
            "https://router.huggingface.co/v1", OpenAI,
            &["HF_TOKEN"], Some("HF_BASE_URL"));
        define!("xiaomi", "Xiaomi MiMo", ApiKey,
            "https://api.xiaomimimo.com/v1", OpenAI,
            &["XIAOMI_API_KEY"], Some("XIAOMI_BASE_URL"));
        define!("ollama-cloud", "Ollama Cloud", ApiKey,
            "https://api.ollama.cloud/v1", OpenAI,
            &["OLLAMA_API_KEY"], Some("OLLAMA_BASE_URL"));
        define!("bedrock", "AWS Bedrock", AwsSdk,
            "https://bedrock-runtime.us-east-1.amazonaws.com", Anthropic,
            Vec::<String>::new(), Some("BEDROCK_BASE_URL".to_string()));

        // ── OpenAI-Compatible Third Party ──
        define!("openai", "OpenAI", ApiKey,
            "https://api.openai.com/v1", OpenAI,
            &["OPENAI_API_KEY"]);
        define!("groq", "Groq", ApiKey,
            "https://api.groq.com/openai/v1", OpenAI,
            &["GROQ_API_KEY"]);
        define!("openrouter", "OpenRouter", ApiKey,
            "https://openrouter.ai/api/v1", OpenAI,
            &["OPENROUTER_API_KEY"]);
        define!("fireworks", "Fireworks", ApiKey,
            "https://api.fireworks.ai/inference/v1", OpenAI,
            &["FIREWORKS_API_KEY"]);
        define!("together", "Together", ApiKey,
            "https://api.together.xyz/v1", OpenAI,
            &["TOGETHER_API_KEY"]);
        define!("perplexity", "Perplexity", ApiKey,
            "https://api.perplexity.ai", OpenAI,
            &["PERPLEXITY_API_KEY"]);
        define!("mistral", "Mistral", ApiKey,
            "https://api.mistral.ai/v1", OpenAI,
            &["MISTRAL_API_KEY"]);
        define!("moonshot", "Moonshot", ApiKey,
            "https://api.moonshot.cn/v1", OpenAI,
            &["MOONSHOT_API_KEY"]);

        // ── Local / Self-hosted ──
        define!("ollama", "Ollama", ApiKey,
            "http://localhost:11434", Native,
            Vec::<String>::new(), Some("OLLAMA_HOST".to_string()));
        define!("vllm", "vLLM", ApiKey,
            "http://localhost:8000", OpenAI,
            &["VLLM_API_KEY"], Some("VLLM_HOST"));
        define!("azure", "Azure OpenAI", ApiKey,
            "https://{resource}.openai.azure.com", OpenAI,
            &["AZURE_OPENAI_API_KEY"], Some("AZURE_OPENAI_BASE_URL"));
        define!("byteplus", "BytePlus", ApiKey,
            "https://ark.cn-beijing.volces.com", OpenAI,
            &["BYTEPLUS_API_KEY"], Some("BYTEPLUS_ENDPOINT"));
        define!("venice", "Venice AI", ApiKey,
            "https://api.venice.ai/api/v1", OpenAI,
            &["VENICE_API_KEY"]);

        // ── Custom / Generic / LocalAI / LM Studio / Groq Cloud / Anyscale ──
        define!("custom_openai", "Custom OpenAI API", ApiKey,
            "http://localhost:8080/v1", OpenAI,
            &["CUSTOM_OPENAI_API_KEY"], Some("CUSTOM_OPENAI_BASE_URL"));
        define!("localai", "LocalAI", ApiKey,
            "http://localhost:8080/v1", OpenAI,
            &["LOCALAI_API_KEY"], Some("LOCALAI_BASE_URL"));
        define!("lmstudio", "LM Studio", ApiKey,
            "http://localhost:1234/v1", OpenAI,
            &["LMSTUDIO_API_KEY"], Some("LMSTUDIO_BASE_URL"));
        define!("groqcloud", "Groq Cloud", ApiKey,
            "https://api.groq.com/openai/v1", OpenAI,
            &["GROQ_API_KEY"]);
        define!("anyscale", "Anyscale", ApiKey,
            "https://api.endpoints.anyscale.com/v1", OpenAI,
            &["ANYSCALE_API_KEY"]);

        Self { providers }
    }

    pub fn get(&self, id: &str) -> Option<Arc<ProviderConfig>> {
        self.providers.get(id).cloned()
    }

    pub fn list_ids(&self) -> Vec<String> {
        self.providers.keys().map(|k| k.to_string()).collect()
    }

    pub fn list_all(&self) -> Vec<Arc<ProviderConfig>> {
        self.providers.values().cloned().collect()
    }

    pub fn resolve_base_url(&self, id: &str) -> Option<String> {
        let config = self.get(id)?;
        
        // 1. Check environment variable override
        if let Some(ref var) = config.base_url_env_var {
            if let Ok(val) = std::env::var(var) {
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
        
        // 2. Check config.json user override under "api_keys" as `{id}_base_url` or `{id}`
        if let Ok(user_cfg) = crate::load_config_inner() {
            let base_url_key = format!("{}_base_url", id);
            if let Some(val) = user_cfg.api_keys.get(&base_url_key) {
                if !val.is_empty() {
                    return Some(val.clone());
                }
            }
            // Fallback: user entered the URL in the API key field directly (e.g. they put custom URL directly in "custom_openai")
            if let Some(val) = user_cfg.api_keys.get(id) {
                if val.starts_with("http://") || val.starts_with("https://") {
                    return Some(val.clone());
                }
            }
        }
        
        Some(config.inference_base_url.clone())
    }

    pub fn resolve_api_key(&self, id: &str) -> Option<String> {
        let config = self.get(id)?;
        for env_var in &config.api_key_env_vars {
            if let Ok(val) = std::env::var(env_var) {
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
        None
    }
}
