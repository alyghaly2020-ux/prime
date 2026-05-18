use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::tools::compression::CompressionPipeline;
use crate::tools::identity::IdentityMask;
use crate::tools::obfuscation::{ObfuscationMode, ObfuscationPipeline};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    pub identity_masking: bool,
    pub compression_level: String,
    pub obfuscation_mode: String,
    pub proxy_rotation: bool,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            identity_masking: true,
            compression_level: "medium".into(),
            obfuscation_mode: "light".into(),
            proxy_rotation: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub original_prompt: String,
    pub processed_prompt: String,
    pub identity_injected: bool,
    pub compressed: bool,
    pub compression_ratio: Option<f64>,
    pub obfuscated: bool,
    pub pii_removed: Vec<String>,
    pub tokens_saved: Option<usize>,
}

pub struct Pipeline {
    pub identity: Arc<IdentityMask>,
    pub compression: Arc<CompressionPipeline>,
    pub obfuscation: tokio::sync::RwLock<ObfuscationPipeline>,
    pub config: tokio::sync::RwLock<PipelineConfig>,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    pub fn new() -> Self {
        Self {
            identity: Arc::new(IdentityMask::new()),
            compression: Arc::new(CompressionPipeline::new()),
            obfuscation: tokio::sync::RwLock::new(ObfuscationPipeline::new()),
            config: tokio::sync::RwLock::new(PipelineConfig::default()),
        }
    }

    #[allow(dead_code)]
    pub async fn process(&self, prompt: &str) -> PipelineResult {
        let config = self.config.read().await;
        let mut processed = prompt.to_string();
        let mut identity_injected = false;
        let mut compressed = false;
        let mut compression_ratio = None;
        let mut obfuscated = false;
        let mut pii_removed = vec![];

        // 1. Identity masking
        if config.identity_masking {
            identity_injected = true;
        }

        // 2. Compression
        match config.compression_level.as_str() {
            "light" | "medium" | "aggressive" => {
                let compressed_text = self.compression.compress(&processed);
                let original_tokens = CompressionPipeline::estimate_tokens(&processed);
                let compressed_tokens = CompressionPipeline::estimate_tokens(&compressed_text);
                compression_ratio = Some(1.0 - (compressed_tokens as f64 / original_tokens as f64));
                processed = compressed_text;
                compressed = true;
            }
            _ => {}
        }

        // 3. Obfuscation
        let mode = match config.obfuscation_mode.as_str() {
            "light" => ObfuscationMode::Light,
            "medium" => ObfuscationMode::Medium,
            "aggressive" => ObfuscationMode::Aggressive,
            _ => ObfuscationMode::Off,
        };
        if mode != ObfuscationMode::Off {
            let mut obf = self.obfuscation.write().await;
            obf.set_mode(mode);
            let result = obf.process(&processed);
            pii_removed = result.pii_removed;
            processed = result.sanitized;
            obfuscated = true;
        }

        let tokens_saved = if compressed {
            let orig_len = CompressionPipeline::estimate_tokens(prompt);
            let new_len = CompressionPipeline::estimate_tokens(&processed);
            Some(orig_len.saturating_sub(new_len))
        } else {
            None
        };

        PipelineResult {
            original_prompt: prompt.to_string(),
            processed_prompt: processed,
            identity_injected,
            compressed,
            compression_ratio,
            obfuscated,
            pii_removed,
            tokens_saved,
        }
    }
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn process_prompt(
    pipeline: tauri::State<'_, Arc<Pipeline>>,
    prompt: String,
) -> Result<String, crate::AppError> {
    let result = pipeline.process(&prompt).await;
    serde_json::to_string(&result).map_err(|e| crate::AppError::Workspace(e.to_string()))
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn get_pipeline_config(
    pipeline: tauri::State<'_, Arc<Pipeline>>,
) -> Result<String, crate::AppError> {
    let config = pipeline.config.read().await;
    serde_json::to_string(&*config).map_err(|e| crate::AppError::Workspace(e.to_string()))
}

#[allow(dead_code)]
#[tauri::command]
pub(crate) async fn set_pipeline_config(
    pipeline: tauri::State<'_, Arc<Pipeline>>,
    config: String,
) -> Result<(), crate::AppError> {
    let parsed: PipelineConfig = serde_json::from_str(&config)
        .map_err(|e| crate::AppError::Workspace(format!("Invalid config: {}", e)))?;
    *pipeline.config.write().await = parsed;
    Ok(())
}
