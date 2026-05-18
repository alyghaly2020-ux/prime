use std::collections::HashMap;
use tokio::sync::RwLock;

#[derive(Debug)]
pub struct WorkspaceSync {
    sessions: RwLock<HashMap<String, WorkspaceState>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct WorkspaceState {
    path: String,
    active: bool,
    last_sync: chrono::DateTime<chrono::Utc>,
}

impl Default for WorkspaceSync {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSync {
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    pub async fn open(&self, id: String, path: String) {
        self.sessions.write().await.insert(
            id,
            WorkspaceState {
                path,
                active: true,
                last_sync: chrono::Utc::now(),
            },
        );
    }

    pub async fn close(&self, id: &str) {
        if let Some(state) = self.sessions.write().await.get_mut(id) {
            state.active = false;
        }
    }

    pub async fn get_path(&self, id: &str) -> Option<String> {
        self.sessions.read().await.get(id).map(|s| s.path.clone())
    }
}
