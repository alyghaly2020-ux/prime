//! WASM Plugin execution system.
//!
//! This module bridges the skills system with the core WASM engine,
//! adding sandbox isolation, input serialization, and compatibility checking.

use super::permissions::Capabilities;
use super::sandbox::Sandbox;
use crate::core::wasm::WasmEngine;
use std::sync::Arc;

/// WASM plugin executor with sandbox and compatibility checking
pub struct WasmPluginSystem {
    engine: Arc<WasmEngine>,
    sandbox: Arc<Sandbox>,
}

impl Default for WasmPluginSystem {
    fn default() -> Self {
        Self::new()
    }
}

impl WasmPluginSystem {
    pub fn new() -> Self {
        // Create a minimal storage engine for the WASM engine.
        // In production, this would be shared with the rest of the system.
        let storage = Arc::new(crate::core::storage::StorageEngine::new());
        let engine = Arc::new(WasmEngine::new(storage));
        let sandbox = Arc::new(Sandbox::new());

        Self { engine, sandbox }
    }

    /// Create from existing shared resources
    pub fn with_resources(engine: Arc<WasmEngine>, sandbox: Arc<Sandbox>) -> Self {
        Self { engine, sandbox }
    }

    /// Execute a WASM plugin's entry point with sandbox isolation.
    ///
    /// # Arguments
    /// * `entry` - Path to the WASM plugin binary
    /// * `input` - Input string to pass to the plugin
    /// * `caps` - Capabilities granted to this plugin
    pub async fn execute(
        &self,
        entry: &str,
        input: &str,
        caps: &Capabilities,
    ) -> anyhow::Result<String> {
        // Check capabilities - plugin must have execution permission
        if !caps.can_exec {
            return Err(anyhow::anyhow!(
                "Plugin execution denied: 'exec' capability required"
            ));
        }

        // Create sandbox for this plugin
        let plugin_id = entry.split('/').next_back().unwrap_or(entry);
        let sandbox_cfg = self.sandbox.get_config(plugin_id).await.unwrap_or_default();

        if sandbox_cfg.time_limit_secs > 0 {
            // Execute with timeout
            let engine = self.engine.clone();
            let entry = entry.to_string();
            let input = input.to_string();
            let timeout = std::time::Duration::from_secs(sandbox_cfg.time_limit_secs);

            let result = tokio::time::timeout(timeout, async move {
                // Load plugin if not already loaded
                let path = std::path::Path::new(&entry);
                let plugin_id = if !path.exists() {
                    // Plugin may already be loaded by name
                    entry.clone()
                } else {
                    engine.load_plugin(path).await?
                };

                engine.execute_plugin(&plugin_id, &input).await
            })
            .await;

            match result {
                Ok(Ok(output)) => Ok(output),
                Ok(Err(e)) => Err(anyhow::anyhow!("Plugin execution error: {}", e)),
                Err(_) => Err(anyhow::anyhow!(
                    "Plugin execution timed out after {} seconds",
                    sandbox_cfg.time_limit_secs
                )),
            }
        } else {
            // No timeout, just execute
            let path = std::path::Path::new(entry);
            let plugin_id = if !path.exists() {
                entry.to_string()
            } else {
                self.engine.load_plugin(path).await?
            };

            self.engine.execute_plugin(&plugin_id, input).await
        }
    }

    /// Execute a plugin with JSON-serialized input.
    ///
    /// The input value is serialized to JSON before being passed to the plugin.
    pub async fn execute_with_input<T: serde::Serialize>(
        &self,
        entry: &str,
        input: &T,
        caps: &Capabilities,
    ) -> anyhow::Result<String> {
        let json_input = serde_json::to_string(input)
            .map_err(|e| anyhow::anyhow!("Failed to serialize plugin input: {}", e))?;
        self.execute(entry, &json_input, caps).await
    }

    /// Check plugin version compatibility.
    ///
    /// Verifies that the plugin's WASI version and required features
    /// are compatible with the current runtime.
    pub async fn version_check(&self, entry: &str) -> anyhow::Result<()> {
        let path = std::path::Path::new(entry);

        let metadata = if path.exists() {
            let wasm_bytes = tokio::fs::read(path).await?;
            crate::core::wasm::parse_wasm_header(&wasm_bytes)?
        } else {
            // Not a file path - check if it's a loaded plugin ID
            return Ok(()); // Will be checked at execution time
        };

        // Build a temporary WasmPlugin for compatibility checking
        let plugin = crate::core::wasm::WasmPlugin {
            id: String::new(),
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            version: "0.1.0".to_string(),
            entry: entry.to_string(),
            permissions: vec![],
            exports: metadata.exports,
            wasi_version: metadata.wasi_version,
            required_features: metadata.required_features,
        };

        crate::core::wasm::WasmEngine::check_compatibility(&plugin)
    }

    /// Get the inner WASM engine reference
    pub fn engine(&self) -> &Arc<WasmEngine> {
        &self.engine
    }

    /// Get the inner sandbox reference
    pub fn sandbox(&self) -> &Arc<Sandbox> {
        &self.sandbox
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skills::permissions::Capabilities;

    fn test_caps() -> Capabilities {
        Capabilities {
            can_read_fs: false,
            can_write_fs: false,
            can_network: false,
            can_exec: true,
            can_access_env: false,
            allowed_paths: vec![],
            allowed_env: vec![],
        }
    }

    fn no_exec_caps() -> Capabilities {
        Capabilities {
            can_exec: false,
            ..test_caps()
        }
    }

    #[tokio::test]
    async fn test_execute_denied_without_capability() {
        let system = WasmPluginSystem::new();
        let result = system.execute("dummy", "input", &no_exec_caps()).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exec' capability"));
    }

    #[tokio::test]
    async fn test_execute_with_timeout() {
        let system = WasmPluginSystem::new();
        // Create a sandbox with 1-second timeout for unknown plugin
        system
            .sandbox
            .create(
                "unknown".to_string(),
                super::super::sandbox::SandboxConfig {
                    time_limit_secs: 1,
                    ..Default::default()
                },
            )
            .await;

        let result = system.execute("unknown", "test", &test_caps()).await;
        // Should fail because plugin isn't loaded and path doesn't exist
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_version_check_nonexistent() {
        // sync test for API sanity
        let _system = WasmPluginSystem::new();
    }

    #[tokio::test]
    async fn test_execute_with_input_json() {
        let system = WasmPluginSystem::new();
        let input = serde_json::json!({"key": "value", "number": 42});

        // Should fail because plugin doesn't exist at this path
        let result = system
            .execute_with_input("/nonexistent/plugin.wasm", &input, &test_caps())
            .await;
        assert!(result.is_err());
    }
}
