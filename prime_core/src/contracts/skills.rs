//! Skills System Contract
//! Defines the plugin/skill lifecycle interface.

use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub entry: String,
    pub language: SkillLanguage,
    pub permissions: Vec<String>,
    pub capabilities: Vec<String>,
    pub checksum: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SkillLanguage {
    Wasm,
    Python,
    JavaScript,
    Native,
}

impl std::fmt::Display for SkillLanguage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillLanguage::Wasm => write!(f, "wasm"),
            SkillLanguage::Python => write!(f, "python"),
            SkillLanguage::JavaScript => write!(f, "javascript"),
            SkillLanguage::Native => write!(f, "native"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillExecution {
    pub skill_id: String,
    pub input: String,
    pub timeout_secs: u64,
    pub context: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: String,
    pub duration_ms: u64,
    pub memory_used_kb: u64,
    pub error: Option<String>,
}

#[async_trait]
pub trait SkillRuntime: Send + Sync {
    async fn load(&self, metadata: SkillMetadata, blob: &[u8]) -> anyhow::Result<String>;
    async fn execute(&self, execution: SkillExecution) -> anyhow::Result<SkillResult>;
    async fn unload(&self, id: &str) -> anyhow::Result<()>;
    async fn list(&self) -> Vec<SkillMetadata>;
}
