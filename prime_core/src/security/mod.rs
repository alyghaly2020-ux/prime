//! Security and sandboxing system. AES-256-GCM encryption with Argon2 key derivation, capability-based permissions, process sandboxing, resource limits, rate limiting, tamper-evident audit logging, and file integrity verification.

pub mod audit;
pub mod encryption;
pub mod integrity;
pub mod permissions;
pub mod rate_limiter;
pub mod resource_limits;
pub mod sandbox;

use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicy {
    pub sandbox_enabled: bool,
    pub permission_model: PermissionModel,
    pub max_cpu_cores: f64,
    pub max_memory_mb: u64,
    pub max_timeout_secs: u64,
    pub encryption_at_rest: bool,
    pub allowed_networks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PermissionModel {
    Strict,
    Moderate,
    Permissive,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            sandbox_enabled: true,
            permission_model: PermissionModel::Moderate,
            max_cpu_cores: 4.0,
            max_memory_mb: 1024,
            max_timeout_secs: 60,
            encryption_at_rest: true,
            allowed_networks: vec![],
        }
    }
}

pub struct System {
    pub sandbox: Arc<sandbox::SecuritySandbox>,
    pub permissions: Arc<permissions::PermissionManager>,
    pub resource_limits: Arc<resource_limits::ResourceLimiter>,
    pub encryption: Arc<encryption::EncryptionEngine>,
    pub audit: Arc<audit::AuditLog>,
    pub rate_limiter: Arc<rate_limiter::RateLimiter>,
    pub integrity: Arc<integrity::IntegrityChecker>,
    policy: Arc<parking_lot::RwLock<SecurityPolicy>>,
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn new() -> Self {
        Self {
            sandbox: Arc::new(sandbox::SecuritySandbox::new()),
            permissions: Arc::new(permissions::PermissionManager::new()),
            resource_limits: Arc::new(resource_limits::ResourceLimiter::new()),
            encryption: Arc::new(encryption::EncryptionEngine::new()),
            audit: Arc::new(audit::AuditLog::new()),
            rate_limiter: Arc::new(rate_limiter::RateLimiter::new()),
            integrity: Arc::new(integrity::IntegrityChecker::new()),
            policy: Arc::new(parking_lot::RwLock::new(SecurityPolicy::default())),
        }
    }

    pub fn policy(&self) -> parking_lot::RwLockUpgradableReadGuard<'_, SecurityPolicy> {
        self.policy.upgradable_read()
    }

    pub fn set_policy(&self, policy: SecurityPolicy) {
        *self.policy.write() = policy;
    }
}
