use super::McpServer;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct SearchMcp {
    id: String,
    code_intel: Arc<crate::code_intel::Engine>,
    dev: Arc<crate::dev::Engine>,
}

impl SearchMcp {
    pub fn new(code_intel: Arc<crate::code_intel::Engine>, dev: Arc<crate::dev::Engine>) -> Self {
        Self {
            id: "search".to_string(),
            code_intel,
            dev,
        }
    }
}

#[async_trait]
impl McpServer for SearchMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Search MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Search MCP server ready");
        Ok(())
    }

    async fn stop(&self) -> anyhow::Result<()> {
        Ok(())
    }
    async fn is_running(&self) -> bool {
        true
    }

    async fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> anyhow::Result<serde_json::Value> {
        match method {
            "code_search" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;
                let path = params["path"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing path parameter"))?;

                let results = self.code_intel.search(query, path).await?;
                let hits: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|r| {
                        serde_json::json!({
                            "file": r.file,
                            "line": r.line,
                            "column": r.column,
                            "content": r.content,
                            "score": r.score,
                            "symbol_type": r.symbol_type,
                            "context_before": r.context_before,
                            "context_after": r.context_after,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({ "results": hits }))
            }

            "web_search" => {
                let query = params["query"].as_str().unwrap_or("");

                // Placeholder: actual web search would use a search API via reqwest
                Ok(serde_json::json!({
                    "results": [],
                    "note": format!("Would search for: {}", query),
                }))
            }

            "semantic_search" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;
                let path = params["path"].as_str().unwrap_or(".");

                let results = self.dev.retrieval.search_similar(query, path).await;
                let hits: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|content| serde_json::json!({ "content": content }))
                    .collect();

                Ok(serde_json::json!({ "results": hits }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
