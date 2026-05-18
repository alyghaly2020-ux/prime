use super::McpServer;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct MemoryMcp {
    id: String,
    memory: Arc<crate::memory::System>,
}

impl MemoryMcp {
    pub fn new(memory: Arc<crate::memory::System>) -> Self {
        Self {
            id: "memory".to_string(),
            memory,
        }
    }
}

#[async_trait]
impl McpServer for MemoryMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Memory MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Memory MCP server ready");
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
            "store" => {
                let content = params["content"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing content parameter"))?;
                let memory_type = params["memory_type"].as_str().unwrap_or("working");
                let metadata = params
                    .get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let id = self
                    .memory
                    .store(memory_type, content.to_string(), metadata)
                    .await?;
                Ok(serde_json::json!({ "status": "stored", "id": id }))
            }

            "recall" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;
                let memory_type = params["memory_type"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing memory_type parameter"))?;

                let results = self.memory.recall(memory_type, query).await?;
                let entries: Vec<serde_json::Value> = results
                    .into_iter()
                    .map(|e| {
                        serde_json::json!({
                            "id": e.id,
                            "memory_type": format!("{:?}", e.memory_type),
                            "content": e.content,
                            "metadata": e.metadata,
                            "created_at": e.created_at.to_rfc3339(),
                            "importance": e.importance,
                        })
                    })
                    .collect();

                Ok(serde_json::json!({ "results": entries }))
            }

            "search" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;

                // Search across all memory types
                let mut all_results = Vec::new();
                for mtype in &["working", "episodic", "semantic", "vector"] {
                    if let Ok(results) = self.memory.recall(mtype, query).await {
                        for e in results {
                            all_results.push(serde_json::json!({
                                "id": e.id,
                                "memory_type": format!("{:?}", e.memory_type),
                                "content": e.content,
                                "metadata": e.metadata,
                                "created_at": e.created_at.to_rfc3339(),
                                "importance": e.importance,
                            }));
                        }
                    }
                }

                // Sort by importance descending
                all_results.sort_by(|a, b| {
                    b["importance"]
                        .as_f64()
                        .unwrap_or(0.0)
                        .partial_cmp(&a["importance"].as_f64().unwrap_or(0.0))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                Ok(serde_json::json!({ "results": all_results }))
            }

            "stats" => {
                let working_len = self.memory.working.get_all().await.len();
                // For other types, count via recall with empty string
                let episodic_count = {
                    let r = self.memory.recall("episodic", "").await;
                    r.map(|v| v.len()).unwrap_or(0)
                };
                let semantic_count = {
                    let r = self.memory.recall("semantic", "").await;
                    r.map(|v| v.len()).unwrap_or(0)
                };
                let vector_count = {
                    let r = self.memory.recall("vector", "").await;
                    r.map(|v| v.len()).unwrap_or(0)
                };

                Ok(serde_json::json!({
                    "working": working_len,
                    "episodic": episodic_count,
                    "semantic": semantic_count,
                    "vector": vector_count,
                    "total": working_len + episodic_count + semantic_count + vector_count,
                }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}
