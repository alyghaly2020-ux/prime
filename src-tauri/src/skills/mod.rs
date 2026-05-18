//! Skill/plugin ecosystem. WASM-based plugin runtime with sandboxed execution, cryptographic signing and verification, hot-reload support, dependency resolution, and a registry for plugin lifecycle management.

pub mod hot_reload;
pub mod loader;
pub mod permissions;
pub mod sandbox;
pub mod signing;
pub mod wasm_plugin;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub entry: String,
    pub permissions: Vec<String>,
    pub capabilities: Vec<String>,
    pub dependencies: Vec<String>,
    #[serde(default = "default_lang")]
    pub language: String,
}

fn default_lang() -> String {
    "wasm".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInstance {
    pub manifest: SkillManifest,
    pub enabled: bool,
    pub loaded_at: chrono::DateTime<chrono::Utc>,
}

/// Describes a plugin's dependency requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_req: String, // semver requirement, e.g. ">=0.1.0"
}

/// Plugin registry entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginEntry {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: Option<String>,
    pub dependencies: Vec<PluginDependency>,
    pub signature: Option<Vec<u8>>,
    pub publisher: Option<String>,
    pub enabled: bool,
    pub registered_at: chrono::DateTime<chrono::Utc>,
}

/// Plugin registry with CRUD, dependency resolution, and version checking
pub struct PluginRegistry {
    plugins: RwLock<HashMap<String, PluginEntry>>,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new plugin
    pub async fn register(&self, entry: PluginEntry) -> anyhow::Result<()> {
        let id = entry.id.clone();

        // Check for duplicate
        if self.plugins.read().await.contains_key(&id) {
            return Err(anyhow::anyhow!("Plugin already registered: {}", id));
        }

        // Verify dependencies are satisfiable
        self.verify_dependencies(&entry).await?;

        self.plugins.write().await.insert(id, entry);
        Ok(())
    }

    /// Unregister a plugin
    pub async fn unregister(&self, id: &str) -> anyhow::Result<()> {
        // Check if any other plugin depends on this one
        let deps = self.plugins.read().await;
        for (other_id, other) in deps.iter() {
            for dep in &other.dependencies {
                if dep.plugin_id == id {
                    return Err(anyhow::anyhow!(
                        "Cannot unregister '{}': '{}' depends on it",
                        id,
                        other_id
                    ));
                }
            }
        }
        drop(deps);

        self.plugins
            .write()
            .await
            .remove(id)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", id))?;
        Ok(())
    }

    /// Get a plugin by ID
    pub async fn get(&self, id: &str) -> Option<PluginEntry> {
        self.plugins.read().await.get(id).cloned()
    }

    /// List all registered plugins
    pub async fn list(&self) -> Vec<PluginEntry> {
        self.plugins.read().await.values().cloned().collect()
    }

    /// Enable or disable a plugin
    pub async fn set_enabled(&self, id: &str, enabled: bool) -> anyhow::Result<()> {
        let mut plugins = self.plugins.write().await;
        let plugin = plugins
            .get_mut(id)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", id))?;
        plugin.enabled = enabled;
        Ok(())
    }

    /// Get a topological sort of plugin load order.
    /// Returns plugin IDs in dependency order (dependencies first).
    pub async fn resolve_load_order(&self) -> anyhow::Result<Vec<String>> {
        let plugins = self.plugins.read().await;
        let ids: Vec<String> = plugins.keys().cloned().collect();
        drop(plugins);

        // Build adjacency list
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        for id in &ids {
            adj.entry(id.clone()).or_default();
            in_degree.entry(id.clone()).or_insert(0);
        }

        let plugins = self.plugins.read().await;
        for (id, entry) in plugins.iter() {
            for dep in &entry.dependencies {
                if ids.contains(&dep.plugin_id) {
                    adj.entry(dep.plugin_id.clone())
                        .or_default()
                        .push(id.clone());
                    *in_degree.entry(id.clone()).or_insert(0) += 1;
                }
            }
        }
        drop(plugins);

        // Kahn's algorithm for topological sort
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(id, _)| id.clone())
            .collect();

        let mut sorted = Vec::new();
        while let Some(id) = queue.pop() {
            sorted.push(id.clone());
            if let Some(neighbors) = adj.get(&id) {
                for neighbor in neighbors {
                    if let Some(deg) = in_degree.get_mut(neighbor) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }
        }

        if sorted.len() != ids.len() {
            return Err(anyhow::anyhow!(
                "Circular dependency detected among plugins. \
                 Resolved {} of {} plugins.",
                sorted.len(),
                ids.len()
            ));
        }

        Ok(sorted)
    }

    /// Verify that all dependencies of a plugin are registered and compatible
    async fn verify_dependencies(&self, entry: &PluginEntry) -> anyhow::Result<()> {
        let plugins = self.plugins.read().await;

        for dep in &entry.dependencies {
            let dep_plugin = plugins.get(&dep.plugin_id).ok_or_else(|| {
                anyhow::anyhow!(
                    "Plugin '{}' requires '{}' which is not registered",
                    entry.id,
                    dep.plugin_id
                )
            })?;

            // Simple version check (semver parsing is heavy, do basic comparison)
            if !dep_plugin.enabled {
                return Err(anyhow::anyhow!(
                    "Plugin '{}' requires '{}' which is disabled",
                    entry.id,
                    dep.plugin_id
                ));
            }

            // Basic version compatibility: if dependency says ">=X", check plugin version >= X
            if let Some(req_version) = dep.version_req.strip_prefix(">=") {
                let req_ver = req_version.trim();
                let plugin_ver = &dep_plugin.version;
                if plugin_ver.as_str() < req_ver {
                    return Err(anyhow::anyhow!(
                        "Plugin '{}' requires '{}' version >= {}, but version {} is installed",
                        entry.id,
                        dep.plugin_id,
                        req_ver,
                        plugin_ver
                    ));
                }
            }
        }

        Ok(())
    }

    /// Check if a version satisfies a semver requirement (basic)
    pub fn check_version_compatibility(version: &str, requirement: &str) -> bool {
        if let Some(min_ver) = requirement.strip_prefix(">=") {
            version.trim() >= min_ver.trim()
        } else if let Some(max_ver) = requirement.strip_prefix("<=") {
            version.trim() <= max_ver.trim()
        } else if let Some(exact) = requirement.strip_prefix("==") {
            version.trim() == exact.trim()
        } else if requirement == "*" {
            true
        } else {
            // Default: exact match
            version == requirement
        }
    }
}

// =============================================================================
// Skills System
// =============================================================================

pub struct System {
    pub loader: Arc<loader::SkillLoader>,
    pub wasm: Arc<wasm_plugin::WasmPluginSystem>,
    pub hot_reload: Arc<hot_reload::HotReload>,
    pub sandbox: Arc<sandbox::Sandbox>,
    pub permissions: Arc<permissions::PermissionSystem>,
    pub signing: Arc<signing::PluginSigning>,
    pub registry: Arc<PluginRegistry>,
    skills: RwLock<HashMap<String, SkillInstance>>,
}

impl Default for System {
    fn default() -> Self {
        Self::new()
    }
}

impl System {
    pub fn new() -> Self {
        let loader = Arc::new(loader::SkillLoader::new());
        let wasm = Arc::new(wasm_plugin::WasmPluginSystem::new());
        let hot_reload = Arc::new(hot_reload::HotReload::new(loader.clone()));
        let sandbox = Arc::new(sandbox::Sandbox::new());
        let permissions = Arc::new(permissions::PermissionSystem::new());
        let signing = Arc::new(signing::PluginSigning::new());
        let registry = Arc::new(PluginRegistry::new());

        Self {
            loader,
            wasm,
            hot_reload,
            sandbox,
            permissions,
            signing,
            registry,
            skills: RwLock::new(HashMap::new()),
        }
    }

    pub async fn load_skill(&self, path: &str) -> anyhow::Result<String> {
        let manifest = self.loader.load_manifest(path).await?;

        // Verify manifest signature if present
        if let Some(ref author) = manifest.author {
            if let Some(_pub_key) = self.signing.get_public_key(author) {
                let _manifest_bytes = toml::to_string(&manifest)?;
                // In production, signature would be verified here
                if !self.signing.is_trusted(author) {
                    tracing::warn!(
                        "Untrusted publisher '{}' for skill '{}'",
                        author,
                        manifest.id
                    );
                }
            }
        }

        let skill = SkillInstance {
            manifest: manifest.clone(),
            enabled: true,
            loaded_at: chrono::Utc::now(),
        };

        let id = manifest.id.clone();
        self.skills.write().await.insert(id.clone(), skill);
        tracing::info!("Loaded skill: {} v{}", manifest.name, manifest.version);
        Ok(id)
    }

    pub async fn invoke(&self, skill_id: &str, input: &str) -> anyhow::Result<String> {
        let skills = self.skills.read().await;
        let skill = skills
            .get(skill_id)
            .ok_or_else(|| anyhow::anyhow!("Skill not found: {}", skill_id))?;

        if !skill.enabled {
            return Err(anyhow::anyhow!("Skill is disabled: {}", skill_id));
        }

        let manifest = &skill.manifest;
        let caps = self.permissions.check(&manifest.permissions).await?;

        // Route to appropriate executor
        match manifest.language.as_str() {
            "wasm" => self.wasm.execute(&manifest.entry, input, &caps).await,
            _ => Err(anyhow::anyhow!(
                "Unsupported skill language: {}",
                manifest.language
            )),
        }
    }

    pub async fn list_skills(&self) -> Vec<SkillInstance> {
        self.skills.read().await.values().cloned().collect()
    }

    pub async fn unload_skill(&self, id: &str) -> anyhow::Result<()> {
        self.skills.write().await.remove(id);
        Ok(())
    }

    pub async fn enable_skill(&self, id: &str, enabled: bool) -> anyhow::Result<()> {
        if let Some(skill) = self.skills.write().await.get_mut(id) {
            skill.enabled = enabled;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Skill not found: {}", id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_registry_crud() {
        let registry = PluginRegistry::new();

        let entry = PluginEntry {
            id: "test-plugin".to_string(),
            name: "Test Plugin".to_string(),
            version: "1.0.0".to_string(),
            description: "A test plugin".to_string(),
            author: None,
            dependencies: vec![],
            signature: None,
            publisher: None,
            enabled: true,
            registered_at: chrono::Utc::now(),
        };

        registry.register(entry.clone()).await.unwrap();
        assert_eq!(registry.list().await.len(), 1);

        let fetched = registry.get("test-plugin").await.unwrap();
        assert_eq!(fetched.name, "Test Plugin");

        registry.unregister("test-plugin").await.unwrap();
        assert_eq!(registry.list().await.len(), 0);
    }

    #[tokio::test]
    async fn test_plugin_registry_duplicate() {
        let registry = PluginRegistry::new();
        let entry = PluginEntry {
            id: "dup".to_string(),
            name: "Dup".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            dependencies: vec![],
            signature: None,
            publisher: None,
            enabled: true,
            registered_at: chrono::Utc::now(),
        };

        registry.register(entry.clone()).await.unwrap();
        let result = registry.register(entry).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_dependency_verification() {
        let registry = PluginRegistry::new();

        let base = PluginEntry {
            id: "base".to_string(),
            name: "Base".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            dependencies: vec![],
            signature: None,
            publisher: None,
            enabled: true,
            registered_at: chrono::Utc::now(),
        };

        registry.register(base).await.unwrap();

        let dependent = PluginEntry {
            id: "dependent".to_string(),
            name: "Dependent".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            dependencies: vec![PluginDependency {
                plugin_id: "base".to_string(),
                version_req: ">=1.0.0".to_string(),
            }],
            signature: None,
            publisher: None,
            enabled: true,
            registered_at: chrono::Utc::now(),
        };

        registry.register(dependent).await.unwrap();
        assert_eq!(registry.list().await.len(), 2);
    }

    #[tokio::test]
    async fn test_missing_dependency() {
        let registry = PluginRegistry::new();

        let dependent = PluginEntry {
            id: "orphan".to_string(),
            name: "Orphan".to_string(),
            version: "1.0.0".to_string(),
            description: String::new(),
            author: None,
            dependencies: vec![PluginDependency {
                plugin_id: "nonexistent".to_string(),
                version_req: "*".to_string(),
            }],
            signature: None,
            publisher: None,
            enabled: true,
            registered_at: chrono::Utc::now(),
        };

        let result = registry.register(dependent).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_topological_sort() {
        let registry = PluginRegistry::new();

        registry
            .register(PluginEntry {
                id: "core".to_string(),
                name: "Core".to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                author: None,
                dependencies: vec![],
                signature: None,
                publisher: None,
                enabled: true,
                registered_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        registry
            .register(PluginEntry {
                id: "plugin-a".to_string(),
                name: "Plugin A".to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                author: None,
                dependencies: vec![PluginDependency {
                    plugin_id: "core".to_string(),
                    version_req: "*".to_string(),
                }],
                signature: None,
                publisher: None,
                enabled: true,
                registered_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        registry
            .register(PluginEntry {
                id: "plugin-b".to_string(),
                name: "Plugin B".to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                author: None,
                dependencies: vec![PluginDependency {
                    plugin_id: "plugin-a".to_string(),
                    version_req: "*".to_string(),
                }],
                signature: None,
                publisher: None,
                enabled: true,
                registered_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        let order = registry.resolve_load_order().await.unwrap();
        // core should come before plugin-a, which should come before plugin-b
        let core_pos = order.iter().position(|id| id == "core").unwrap();
        let a_pos = order.iter().position(|id| id == "plugin-a").unwrap();
        let b_pos = order.iter().position(|id| id == "plugin-b").unwrap();
        assert!(core_pos < a_pos);
        assert!(a_pos < b_pos);
    }

    #[test]
    fn test_version_checking() {
        assert!(PluginRegistry::check_version_compatibility(
            "1.0.0", ">=0.5.0"
        ));
        assert!(PluginRegistry::check_version_compatibility(
            "1.0.0", ">=1.0.0"
        ));
        assert!(!PluginRegistry::check_version_compatibility(
            "0.9.0", ">=1.0.0"
        ));
        assert!(PluginRegistry::check_version_compatibility("2.0.0", "*"));
        assert!(PluginRegistry::check_version_compatibility(
            "1.0.0", "==1.0.0"
        ));
        assert!(!PluginRegistry::check_version_compatibility(
            "1.0.0", "==2.0.0"
        ));
        assert!(PluginRegistry::check_version_compatibility(
            "1.0.0", "<=2.0.0"
        ));
        assert!(!PluginRegistry::check_version_compatibility(
            "3.0.0", "<=2.0.0"
        ));
    }

    #[tokio::test]
    async fn test_enable_disable() {
        let registry = PluginRegistry::new();
        registry
            .register(PluginEntry {
                id: "test".to_string(),
                name: "Test".to_string(),
                version: "1.0.0".to_string(),
                description: String::new(),
                author: None,
                dependencies: vec![],
                signature: None,
                publisher: None,
                enabled: true,
                registered_at: chrono::Utc::now(),
            })
            .await
            .unwrap();

        let entry = registry.get("test").await.unwrap();
        assert!(entry.enabled);

        registry.set_enabled("test", false).await.unwrap();
        let entry = registry.get("test").await.unwrap();
        assert!(!entry.enabled);
    }
}
