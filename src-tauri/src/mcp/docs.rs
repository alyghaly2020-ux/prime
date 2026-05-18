use super::McpServer;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct DocsMcp {
    id: String,
    storage: Arc<crate::core::storage::StorageEngine>,
}

impl DocsMcp {
    pub fn new(storage: Arc<crate::core::storage::StorageEngine>) -> Self {
        Self {
            id: "docs".to_string(),
            storage,
        }
    }
}

#[async_trait]
impl McpServer for DocsMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Docs MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Docs MCP server ready");
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
            "search_docs" => {
                let query = params["query"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing query parameter"))?;
                let limit = params["limit"].as_u64().unwrap_or(10) as usize;

                let results = self.storage.search(query, limit)?;
                let mut docs = Vec::new();
                for (id, score) in results {
                    let content = get_doc_content(&self.storage, &id).unwrap_or_default();
                    docs.push(serde_json::json!({
                        "id": id,
                        "content": content,
                        "score": score,
                    }));
                }

                Ok(serde_json::json!({ "results": docs }))
            }

            "get_doc" => {
                let id = params["id"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing id parameter"))?;
                let db = self.storage.lock();
                let mut stmt = db.prepare(
                    "SELECT id, memory_type, content, metadata, created_at, updated_at \
                     FROM memories WHERE id = ?1",
                )?;

                let doc = stmt
                    .query_row([id], |row| {
                        Ok(serde_json::json!({
                            "id": row.get::<_, String>(0)?,
                            "memory_type": row.get::<_, String>(1)?,
                            "content": row.get::<_, String>(2)?,
                            "metadata": row.get::<_, String>(3)?,
                            "created_at": row.get::<_, String>(4)?,
                            "updated_at": row.get::<_, String>(5)?,
                        }))
                    })
                    .ok();

                Ok(serde_json::json!({ "doc": doc }))
            }

            "list_collections" => {
                let db = self.storage.lock();
                let mut stmt =
                    db.prepare("SELECT DISTINCT memory_type FROM memories ORDER BY memory_type")?;
                let collections: Vec<String> = stmt
                    .query_map([], |row| row.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();

                Ok(serde_json::json!({ "collections": collections }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}

fn get_doc_content(
    storage: &crate::core::storage::StorageEngine,
    id: &str,
) -> anyhow::Result<String> {
    let db = storage.lock();
    let mut stmt = db.prepare("SELECT content FROM memories WHERE id = ?1")?;
    let content: String = stmt.query_row([id], |row| row.get(0))?;
    Ok(content)
}
