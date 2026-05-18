//! Prime Contracts Layer
//!
//! Central interface definitions that prevent module spaghetti.
//! All cross-module communication MUST go through these traits.
//! No module should import another module's internal types directly.

pub mod mcp;
pub mod memory;
pub mod runtime;
pub mod skills;

use async_trait::async_trait;

// ─── Core Runtime Contracts ───

#[async_trait]
pub trait RuntimeProvider: Send + Sync {
    async fn state(&self) -> anyhow::Result<serde_json::Value>;
    async fn execute(&self, code: &str, language: &str) -> anyhow::Result<ExecutionResult>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

// ─── Memory Contracts ───

#[async_trait]
pub trait MemoryProvider: Send + Sync {
    async fn store(
        &self,
        memory_type: &str,
        content: String,
        metadata: serde_json::Value,
    ) -> anyhow::Result<String>;
    async fn recall(&self, memory_type: &str, query: &str) -> anyhow::Result<Vec<MemoryEntry>>;
    async fn consolidate(&self) -> anyhow::Result<()>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub memory_type: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub created_at: String,
    pub importance: f32,
}

// ─── AI Contracts ───

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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default)]
    pub timestamp: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelConfig {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub streaming: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub model: String,
    pub usage: Usage,
    pub finish_reason: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ─── Execution Contracts ───

#[async_trait]
pub trait ExecutionProvider: Send + Sync {
    async fn run(&self, code: &str, language: &str) -> anyhow::Result<ExecutionResult>;
    async fn rollback(&self, checkpoint_id: &str) -> anyhow::Result<()>;
}

// ─── Skills Contracts ───

#[async_trait]
pub trait SkillProvider: Send + Sync {
    async fn load(&self, path: &str) -> anyhow::Result<String>;
    async fn invoke(&self, id: &str, input: &str) -> anyhow::Result<String>;
    async fn unload(&self, id: &str) -> anyhow::Result<()>;
}

// ─── Security Contracts ───

#[async_trait]
pub trait SecurityProvider: Send + Sync {
    async fn check_permission(&self, subject: &str, resource: &str, action: &str) -> bool;
    async fn enforce_limits(&self) -> Result<(), String>;
}

// ─── Browser Contracts ───

#[async_trait]
pub trait BrowserProvider: Send + Sync {
    async fn navigate(&self, url: &str) -> anyhow::Result<PageSnapshot>;
    async fn snapshot(&self) -> anyhow::Result<PageSnapshot>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PageSnapshot {
    pub url: String,
    pub title: String,
    pub text: String,
    pub screenshot: Option<Vec<u8>>,
}

// ─── Architecture Event Contracts ───

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SystemEvent {
    pub id: String,
    pub event_type: String,
    pub source: String,
    pub payload: serde_json::Value,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

// ─── Verification Contracts ───

#[async_trait]
pub trait VerificationProvider: Send + Sync {
    async fn verify(&self, code: &str, language: &str) -> VerificationResult;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerificationResult {
    pub passed: bool,
    pub score: f64,
    pub errors: Vec<Issue>,
    pub warnings: Vec<Issue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Issue {
    pub severity: IssueSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}
