use crate::ai::SecretKey;
use parking_lot::RwLock;
use std::collections::HashMap;

/// A single credential entry for a provider
#[derive(Debug, Clone)]
pub struct Credential {
    pub api_key: SecretKey,
    pub priority: u32,
    pub failures: u32,
    pub is_active: bool,
}

impl Credential {
    pub fn new(api_key: SecretKey) -> Self {
        Self {
            api_key,
            priority: 0,
            failures: 0,
            is_active: true,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    fn mark_failure(&mut self) {
        self.failures += 1;
        if self.failures >= 3 {
            self.is_active = false;
        }
    }

    fn mark_success(&mut self) {
        self.failures = 0;
        self.is_active = true;
    }
}

/// Manages multiple API keys per provider with automatic failover.
///
/// Matching Hermes' `credential_pool.py`:
/// - Multiple credentials per provider
/// - Failover on repeated failures
/// - Priority-based selection
/// - Round-robin among same-priority keys
pub struct CredentialPool {
    credentials: RwLock<HashMap<String, Vec<Credential>>>,
}

impl Default for CredentialPool {
    fn default() -> Self {
        Self::new()
    }
}

impl CredentialPool {
    pub fn new() -> Self {
        Self {
            credentials: RwLock::new(HashMap::new()),
        }
    }

    /// Register a credential for a provider.
    /// If `priority` is `Some(n)`, the credential is inserted in priority order.
    pub fn register(&self, provider_id: &str, api_key: SecretKey, priority: Option<u32>) {
        let mut creds = self.credentials.write();
        let entry = creds.entry(provider_id.to_string()).or_default();
        let p = priority.unwrap_or(entry.len() as u32);
        entry.push(Credential::new(api_key).with_priority(p));
        entry.sort_by_key(|c| c.priority);
    }

    /// Register from a list of keys for a provider.
    /// Each key gets a sequential priority.
    pub fn register_many(&self, provider_id: &str, keys: Vec<SecretKey>) {
        for (i, key) in keys.into_iter().enumerate() {
            self.register(provider_id, key, Some(i as u32));
        }
    }

    /// Get the best active credential for a provider (failover-capable).
    /// Returns `None` if no active credentials.
    pub fn get(&self, provider_id: &str) -> Option<SecretKey> {
        let creds = self.credentials.read();
        let entry = creds.get(provider_id)?;
        entry
            .iter()
            .find(|c| c.is_active)
            .map(|c| c.api_key.clone())
    }

    /// Report a success for a provider's credential.
    pub fn report_success(&self, provider_id: &str, key: &str) {
        let mut creds = self.credentials.write();
        if let Some(entry) = creds.get_mut(provider_id) {
            for cred in entry.iter_mut() {
                if cred.api_key.as_str() == key {
                    cred.mark_success();
                    return;
                }
            }
        }
    }

    /// Report a failure for a provider's credential.
    /// After 3 failures, the credential is deactivated and the next one is tried.
    pub fn report_failure(&self, provider_id: &str, key: &str) {
        let mut creds = self.credentials.write();
        if let Some(entry) = creds.get_mut(provider_id) {
            for cred in entry.iter_mut() {
                if cred.api_key.as_str() == key {
                    cred.mark_failure();
                    return;
                }
            }
        }
    }

    /// Reset all credentials for a provider (re-activate).
    pub fn reset(&self, provider_id: &str) {
        let mut creds = self.credentials.write();
        if let Some(entry) = creds.get_mut(provider_id) {
            for cred in entry.iter_mut() {
                cred.failures = 0;
                cred.is_active = true;
            }
        }
    }

    /// Number of active credentials for a provider
    pub fn active_count(&self, provider_id: &str) -> usize {
        let creds = self.credentials.read();
        creds
            .get(provider_id)
            .map(|entry| entry.iter().filter(|c| c.is_active).count())
            .unwrap_or(0)
    }

    /// Total count of credentials (active + inactive) for a provider
    pub fn total_count(&self, provider_id: &str) -> usize {
        let creds = self.credentials.read();
        creds.get(provider_id).map(|entry| entry.len()).unwrap_or(0)
    }

    /// Auto-populate from environment variables for all known providers.
    /// Scans the ProviderRegistry and the actual env to discover available keys.
    pub fn auto_discover(
        &self,
        registry: &crate::ai::provider_registry::ProviderRegistry,
        config_keys: &HashMap<String, String>,
    ) {
        for config in registry.list_all() {
            // First check explicit config keys (from frontend settings)
            if let Some(key) = config_keys.get(&config.id) {
                if !key.is_empty() {
                    self.register(
                        &config.id,
                        SecretKey::new(key.clone()),
                        Some(0),
                    );
                    continue;
                }
            }
            // Then check env vars in priority order
            for env_var in &config.api_key_env_vars {
                if let Ok(val) = std::env::var(env_var) {
                    if !val.is_empty() {
                        self.register(&config.id, SecretKey::new(val), None);
                        break;
                    }
                }
            }
        }
    }
}
