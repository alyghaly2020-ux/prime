use chrono::{DateTime, Utc};
use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RestorePoint {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub data: Option<Vec<u8>>,
}

impl Default for RestorePoint {
    fn default() -> Self {
        Self::new()
    }
}

impl RestorePoint {
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            created_at: Utc::now(),
            data: None,
        }
    }
}

pub struct CheckpointSystem {
    points: RwLock<HashMap<String, RestorePoint>>,
    max_points: usize,
}

impl Default for CheckpointSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl CheckpointSystem {
    pub fn new() -> Self {
        Self {
            points: RwLock::new(HashMap::new()),
            max_points: 50,
        }
    }

    pub async fn create(&self) -> anyhow::Result<String> {
        let point = RestorePoint::new();
        let id = point.id.clone();

        let mut points = self.points.write().await;
        if points.len() >= self.max_points {
            // Remove oldest
            let oldest = points
                .iter()
                .min_by_key(|(_, p)| p.created_at)
                .map(|(k, _)| k.clone());
            if let Some(oldest) = oldest {
                points.remove(&oldest);
            }
        }

        points.insert(id.clone(), point);
        tracing::debug!("Checkpoint created: {}", id);
        Ok(id)
    }

    pub async fn get(&self, id: &str) -> Option<RestorePoint> {
        self.points.read().await.get(id).cloned()
    }

    pub async fn delete(&self, id: &str) {
        self.points.write().await.remove(id);
    }

    pub async fn list(&self) -> Vec<RestorePoint> {
        self.points.read().await.values().cloned().collect()
    }

    pub async fn clear(&self) {
        self.points.write().await.clear();
    }
}
