use std::collections::{HashMap, HashSet};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct Capabilities {
    pub can_read_fs: bool,
    pub can_write_fs: bool,
    pub can_network: bool,
    pub can_exec: bool,
    pub can_access_env: bool,
    pub allowed_paths: Vec<String>,
    pub allowed_env: Vec<String>,
}

pub struct PermissionSystem {
    granted: RwLock<HashMap<String, Vec<String>>>,
}

impl Default for PermissionSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionSystem {
    pub fn new() -> Self {
        Self {
            granted: RwLock::new(HashMap::new()),
        }
    }

    pub async fn check(&self, requested: &[String]) -> anyhow::Result<Capabilities> {
        let _granted = self.granted.read().await;
        let requested_set: HashSet<&String> = requested.iter().collect();

        // Auto-grant all requested for now (production would have proper policy)
        Ok(Capabilities {
            can_read_fs: requested_set.iter().any(|x| x.as_str() == "fs.read"),
            can_write_fs: requested_set.iter().any(|x| x.as_str() == "fs.write"),
            can_network: requested_set.iter().any(|x| x.as_str() == "network"),
            can_exec: requested_set.iter().any(|x| x.as_str() == "exec"),
            can_access_env: requested_set.iter().any(|x| x.as_str() == "env"),
            allowed_paths: vec![],
            allowed_env: vec![],
        })
    }

    pub async fn grant(&self, skill_id: String, permissions: Vec<String>) {
        self.granted.write().await.insert(skill_id, permissions);
    }

    pub async fn revoke(&self, skill_id: &str) {
        self.granted.write().await.remove(skill_id);
    }
}
