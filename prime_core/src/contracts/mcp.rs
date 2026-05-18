//! MCP (Model Context Protocol) Contract
//! Formal interface for all MCP server implementations.

use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpRequest {
    pub method: String,
    pub params: Value,
    pub request_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpResponse {
    pub result: Value,
    pub request_id: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

#[async_trait]
pub trait McpServer: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn capabilities(&self) -> Vec<String> { vec![] }
    async fn start(&self) -> anyhow::Result<()> { Ok(()) }
    async fn stop(&self) -> anyhow::Result<()> { Ok(()) }
    async fn is_running(&self) -> bool { true }
    async fn handle(&self, request: McpRequest) -> Result<McpResponse, McpError> {
        let start = std::time::Instant::now();
        let result = self.handle_request(&request.method, request.params).await
            .map_err(|e| McpError { code: -1, message: e.to_string(), data: None })?;
        Ok(McpResponse { result, request_id: request.request_id, duration_ms: start.elapsed().as_millis() as u64 })
    }
    async fn handle_request(&self, method: &str, params: serde_json::Value) -> anyhow::Result<serde_json::Value>;
    async fn health(&self) -> McpHealth {
        McpHealth {
            server_id: self.id().to_string(),
            status: if self.is_running().await { McpStatus::Healthy } else { McpStatus::Unhealthy },
            uptime_secs: 0,
            requests_served: 0,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpHealth {
    pub server_id: String,
    pub status: McpStatus,
    pub uptime_secs: u64,
    pub requests_served: u64,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum McpStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
