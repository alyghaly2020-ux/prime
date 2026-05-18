//! Memory System Contract
//! Defines the formal interface between memory and all other modules.

use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum MemoryType {
    Working,
    Episodic,
    Semantic,
    Vector,
    Procedural,
    Rag,
    Cache,
    Compression,
}

impl std::fmt::Display for MemoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryType::Working => write!(f, "working"),
            MemoryType::Episodic => write!(f, "episodic"),
            MemoryType::Semantic => write!(f, "semantic"),
            MemoryType::Vector => write!(f, "vector"),
            MemoryType::Procedural => write!(f, "procedural"),
            MemoryType::Rag => write!(f, "rag"),
            MemoryType::Cache => write!(f, "cache"),
            MemoryType::Compression => write!(f, "compression"),
        }
    }
}

impl std::str::FromStr for MemoryType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "working" => Ok(MemoryType::Working),
            "episodic" => Ok(MemoryType::Episodic),
            "semantic" => Ok(MemoryType::Semantic),
            "vector" => Ok(MemoryType::Vector),
            "procedural" => Ok(MemoryType::Procedural),
            "rag" => Ok(MemoryType::Rag),
            "cache" => Ok(MemoryType::Cache),
            "compression" => Ok(MemoryType::Compression),
            _ => Err(format!("Unknown memory type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryQuery {
    pub query: String,
    pub memory_type: Option<MemoryType>,
    pub limit: Option<usize>,
    pub min_importance: Option<f32>,
    pub time_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MemoryRecord {
    pub id: String,
    pub memory_type: MemoryType,
    pub content: String,
    pub metadata: serde_json::Value,
    pub embedding: Option<Vec<f32>>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub access_count: u64,
    pub importance: f32,
    pub ttl: Option<chrono::Duration>,
}

#[async_trait]
pub trait MemoryStorage: Send + Sync {
    async fn insert(&self, record: MemoryRecord) -> anyhow::Result<String>;
    async fn query(&self, query: MemoryQuery) -> anyhow::Result<Vec<MemoryRecord>>;
    async fn delete(&self, id: &str) -> anyhow::Result<()>;
    async fn consolidate(&self) -> anyhow::Result<ConsolidationReport>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConsolidationReport {
    pub entries_before: usize,
    pub entries_after: usize,
    pub compressed: bool,
    pub duration_ms: u64,
}
