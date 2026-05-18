use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::event_bus::{EventBusCore, SystemEvent};

/// Generates hierarchical execution IDs in the format:
///
///   `{prefix}-{short_uuid}-{seq:04}`
///
/// Examples: `task-a1b2c3d4-0001`, `agent-seek-0042`
fn generate_hierarchical_id(prefix: &str, counter: &AtomicU64) -> String {
    let short = &Uuid::new_v4().to_string()[..8];
    let seq = counter.fetch_add(1, Ordering::SeqCst);
    format!("{}-{}-{:04}", prefix, short, seq)
}

// ---------------------------------------------------------------------------
// AgentState
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct AgentState {
    id: String,
    task: String,
    status: String,
    started_at: chrono::DateTime<chrono::Utc>,
}

// ---------------------------------------------------------------------------
// Orchestrator
// ---------------------------------------------------------------------------

pub struct Orchestrator {
    active_agents: RwLock<HashMap<String, AgentState>>,
    bus: Arc<EventBusCore>,
    counter: AtomicU64,
}

impl Orchestrator {
    pub fn new(bus: Arc<EventBusCore>) -> Self {
        Self {
            active_agents: RwLock::new(HashMap::new()),
            bus,
            counter: AtomicU64::new(1),
        }
    }

    /// Spawn a new agent with a hierarchical execution ID.
    ///
    /// The returned ID has the form `{agent_type}-{short_uuid}-{seq:04}`,
    /// making it easy to trace in logs and UIs.
    pub async fn spawn_agent(&self, task: &str, agent_type: &str) -> String {
        let id = generate_hierarchical_id(agent_type, &self.counter);

        self.active_agents.write().await.insert(
            id.clone(),
            AgentState {
                id: id.clone(),
                task: task.to_string(),
                status: "spawned".to_string(),
                started_at: chrono::Utc::now(),
            },
        );

        let event = SystemEvent {
            id: Uuid::new_v4().to_string(),
            event_type: "agent.spawned".to_string(),
            source: "orchestrator".to_string(),
            payload: serde_json::json!({
                "agent_id": id,
                "task": task,
                "agent_type": agent_type,
            }),
            timestamp: chrono::Utc::now(),
        };

        self.bus.emit(event).await;
        tracing::info!("Agent spawned: {} (type: {})", id, agent_type);
        id
    }

    pub async fn get_status(&self, id: &str) -> Option<String> {
        self.active_agents
            .read()
            .await
            .get(id)
            .map(|s| s.status.clone())
    }

    pub async fn list_agents(&self) -> Vec<String> {
        self.active_agents.read().await.keys().cloned().collect()
    }

    pub async fn agent_completed(&self, id: &str) {
        if let Some(agent) = self.active_agents.write().await.get_mut(id) {
            agent.status = "completed".to_string();
        }
    }
}
