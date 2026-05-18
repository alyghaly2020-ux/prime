use parking_lot::RwLock;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Permission {
    pub resource: String,
    pub action: String,
    pub granted: bool,
}

pub struct PermissionManager {
    policies: RwLock<HashMap<String, Vec<Permission>>>,
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PermissionManager {
    pub fn new() -> Self {
        Self {
            policies: RwLock::new(HashMap::new()),
        }
    }

    pub fn check(&self, subject: &str, resource: &str, action: &str) -> bool {
        let policies = self.policies.read();
        if let Some(perms) = policies.get(subject) {
            perms
                .iter()
                .any(|p| p.resource == resource && p.action == action && p.granted)
        } else {
            false // Default deny
        }
    }

    pub fn grant(&self, subject: String, resource: String, action: String) {
        let mut policies = self.policies.write();
        policies.entry(subject).or_default().push(Permission {
            resource,
            action,
            granted: true,
        });
    }

    pub fn revoke(&self, subject: &str, resource: &str, action: &str) {
        let mut policies = self.policies.write();
        if let Some(perms) = policies.get_mut(subject) {
            perms.retain(|p| !(p.resource == resource && p.action == action));
        }
    }

    pub fn list_for(&self, subject: &str) -> Vec<Permission> {
        self.policies
            .read()
            .get(subject)
            .cloned()
            .unwrap_or_default()
    }
}
