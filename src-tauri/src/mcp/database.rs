use super::McpServer;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug)]
pub struct DatabaseMcp {
    id: String,
    storage: Arc<crate::core::storage::StorageEngine>,
}

impl DatabaseMcp {
    pub fn new(storage: Arc<crate::core::storage::StorageEngine>) -> Self {
        Self {
            id: "database".to_string(),
            storage,
        }
    }
}

#[async_trait]
impl McpServer for DatabaseMcp {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> &str {
        "Database MCP"
    }

    async fn start(&self) -> anyhow::Result<()> {
        tracing::info!("Database MCP server ready");
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
            "query" => {
                let sql = params["sql"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing sql parameter"))?;
                let db = self.storage.lock();
                let mut stmt = db.prepare(sql)?;

                let col_names: Vec<String> =
                    stmt.column_names().iter().map(|&c| c.to_string()).collect();

                let rows: Vec<serde_json::Value> = stmt
                    .query_map([], {
                        let cols = col_names.clone();
                        move |row| {
                            let mut row_values = serde_json::Map::new();
                            for (i, col) in cols.iter().enumerate() {
                                let val: rusqlite::types::Value = row.get_unwrap(i);
                                row_values.insert(col.clone(), rusqlite_val_to_json(val));
                            }
                            Ok(serde_json::Value::Object(row_values))
                        }
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                Ok(serde_json::json!({ "rows": rows, "columns": col_names }))
            }

            "execute" => {
                let sql = params["sql"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("Missing sql parameter"))?;
                let db = self.storage.lock();
                let affected = db.execute(sql, [])?;
                Ok(serde_json::json!({ "affected_rows": affected }))
            }

            "list_tables" => {
                let db = self.storage.lock();
                let mut stmt =
                    db.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")?;
                let tables: Vec<String> = stmt
                    .query_map([], |row| row.get(0))?
                    .filter_map(|r| r.ok())
                    .collect();
                Ok(serde_json::json!({ "tables": tables }))
            }

            _ => Err(anyhow::anyhow!("Unknown method: {}", method)),
        }
    }
}

fn rusqlite_val_to_json(val: rusqlite::types::Value) -> serde_json::Value {
    match val {
        rusqlite::types::Value::Null => serde_json::Value::Null,
        rusqlite::types::Value::Integer(i) => serde_json::json!(i),
        rusqlite::types::Value::Real(f) => serde_json::json!(f),
        rusqlite::types::Value::Text(s) => serde_json::json!(s),
        rusqlite::types::Value::Blob(b) => {
            serde_json::Value::Array(b.into_iter().map(|byte| serde_json::json!(byte)).collect())
        }
    }
}
