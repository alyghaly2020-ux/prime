//! Simulated WASM runtime engine.
//!
//! Since neither wasmtime nor wasmer are in the dependency tree, this module
//! provides a simulated WASM runtime that:
//! - Parses WASM binary headers to extract exports and metadata
//! - Maintains a function registry mapping plugin names to Rust handlers
//! - Supports plugin compatibility checking (WASI version, required features)
//!
//! When a real WASM runtime is added, replace the simulated execution path
//! with actual WASM instantiation and calling.

use super::storage::StorageEngine;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;

// WASM binary header constants
const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d]; // "\0asm"
const WASM_VERSION: u32 = 1;

/// Metadata extracted from parsing a WASM binary
#[derive(Debug, Clone)]
pub struct WasmMetadata {
    pub version: u32,
    pub exports: Vec<String>,
    pub wasi_version: Option<String>,
    pub required_features: Vec<String>,
}

/// WASM plugin descriptor
#[derive(Debug, Clone)]
pub struct WasmPlugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub entry: String,
    pub permissions: Vec<String>,
    pub exports: Vec<String>,
    pub wasi_version: Option<String>,
    pub required_features: Vec<String>,
}

type PluginHandler = Arc<dyn Fn(&str) -> anyhow::Result<String> + Send + Sync>;

/// Simulated WASM engine with a function registry for execution
#[allow(dead_code)]
pub struct WasmEngine {
    storage: Arc<StorageEngine>,
    plugins: RwLock<HashMap<String, WasmPlugin>>,
    /// Maps plugin_id -> handler function for simulated execution
    function_registry: RwLock<HashMap<String, PluginHandler>>,
    /// Maps function name -> plugin_id for reverse lookup
    export_index: RwLock<HashMap<String, String>>,
}

impl WasmEngine {
    pub fn new(storage: Arc<StorageEngine>) -> Self {
        Self {
            storage,
            plugins: RwLock::new(HashMap::new()),
            function_registry: RwLock::new(HashMap::new()),
            export_index: RwLock::new(HashMap::new()),
        }
    }

    /// Load a WASM plugin from a file path.
    /// Parses the binary header to extract exports and metadata.
    pub async fn load_plugin(&self, path: &Path) -> anyhow::Result<String> {
        let wasm_bytes = tokio::fs::read(path).await?;

        let metadata = parse_wasm_header(&wasm_bytes)?;

        let plugin_id = uuid::Uuid::new_v4().to_string();
        let plugin = WasmPlugin {
            id: plugin_id.clone(),
            name: path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            version: "0.1.0".to_string(),
            entry: path.to_string_lossy().to_string(),
            permissions: vec![],
            exports: metadata.exports.clone(),
            wasi_version: metadata.wasi_version,
            required_features: metadata.required_features,
        };

        // Register exports in the index
        for export in &plugin.exports {
            self.export_index
                .write()
                .await
                .insert(export.clone(), plugin_id.clone());
        }

        self.plugins.write().await.insert(plugin_id.clone(), plugin);
        tracing::info!(
            "Loaded WASM plugin: {} with {} exports",
            plugin_id,
            metadata.exports.len()
        );

        Ok(plugin_id)
    }

    /// Load a WASM plugin from raw bytes.
    /// Useful for plugins loaded from database storage.
    pub async fn load_plugin_from_bytes(
        &self,
        name: &str,
        wasm_bytes: &[u8],
    ) -> anyhow::Result<String> {
        let metadata = parse_wasm_header(wasm_bytes)?;

        let plugin_id = uuid::Uuid::new_v4().to_string();
        let plugin = WasmPlugin {
            id: plugin_id.clone(),
            name: name.to_string(),
            version: "0.1.0".to_string(),
            entry: String::new(),
            permissions: vec![],
            exports: metadata.exports.clone(),
            wasi_version: metadata.wasi_version,
            required_features: metadata.required_features,
        };

        for export in &plugin.exports {
            self.export_index
                .write()
                .await
                .insert(export.clone(), plugin_id.clone());
        }

        self.plugins.write().await.insert(plugin_id.clone(), plugin);
        Ok(plugin_id)
    }

    /// Execute a WASM plugin's entry point with the given input.
    ///
    /// This uses the function registry to resolve the plugin to a Rust handler.
    /// In production with a real WASM runtime, this would instantiate the WASM
    /// module and call the exported function.
    pub async fn execute_plugin(&self, plugin_id: &str, input: &str) -> anyhow::Result<String> {
        let plugins = self.plugins.read().await;
        let _plugin = plugins
            .get(plugin_id)
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))?;

        let registry = self.function_registry.read().await;
        if let Some(handler) = registry.get(plugin_id) {
            handler(input)
        } else {
            // Simulated execution: return plugin info
            Ok(format!(
                "Simulated WASM execution for plugin '{}' with input length: {}",
                _plugin.name,
                input.len()
            ))
        }
    }

    /// Execute a specific exported function by name.
    pub async fn call_export(&self, export_name: &str, input: &str) -> anyhow::Result<String> {
        let index = self.export_index.read().await;
        let plugin_id = index
            .get(export_name)
            .ok_or_else(|| anyhow::anyhow!("Export not found: {}", export_name))?;

        self.execute_plugin(plugin_id, input).await
    }

    /// Get list of exported function names for a plugin
    pub async fn get_exports(&self, plugin_id: &str) -> anyhow::Result<Vec<String>> {
        self.plugins
            .read()
            .await
            .get(plugin_id)
            .map(|p| p.exports.clone())
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", plugin_id))
    }

    /// Register a Rust function as the handler for a plugin.
    /// This is how "WASM plugins" are simulated: by mapping them to Rust closures.
    pub async fn register_function<F>(&self, plugin_id: &str, handler: F)
    where
        F: Fn(&str) -> anyhow::Result<String> + Send + Sync + 'static,
    {
        self.function_registry
            .write()
            .await
            .insert(plugin_id.to_string(), Arc::new(handler));
    }

    /// Check plugin compatibility against current runtime capabilities.
    /// Returns Ok(()) if compatible, or an error explaining incompatibility.
    pub fn check_compatibility(plugin: &WasmPlugin) -> anyhow::Result<()> {
        // Check WASI compatibility
        if let Some(ref wasi) = plugin.wasi_version {
            if !wasi.contains("wasi_snapshot_preview1") && !wasi.contains("wasi") {
                // Warn about unknown WASI version but don't block
                tracing::warn!("Unknown WASI version: {}", wasi);
            }
        }

        // Check for features we don't support
        for feature in &plugin.required_features {
            if feature == "emscripten" {
                return Err(anyhow::anyhow!(
                    "Emscripten-generated WASM is not supported"
                ));
            }
            if feature == "simd" {
                return Err(anyhow::anyhow!(
                    "SIMD WASM instructions are not supported in this runtime"
                ));
            }
        }

        Ok(())
    }

    /// Unload a plugin
    pub async fn unload_plugin(&self, id: &str) -> anyhow::Result<()> {
        let plugin = self.plugins.write().await.remove(id);
        if let Some(p) = plugin {
            for export in &p.exports {
                self.export_index.write().await.remove(export);
            }
            self.function_registry.write().await.remove(id);
            tracing::info!("Unloaded WASM plugin: {}", id);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Plugin not found: {}", id))
        }
    }

    /// List all loaded plugins
    pub async fn list_plugins(&self) -> Vec<WasmPlugin> {
        self.plugins.read().await.values().cloned().collect()
    }

    /// Get plugin by ID
    pub async fn get_plugin(&self, id: &str) -> Option<WasmPlugin> {
        self.plugins.read().await.get(id).cloned()
    }

    /// Get plugin metadata (version, features) for compatibility checking
    pub async fn get_plugin_metadata(&self, id: &str) -> anyhow::Result<WasmMetadata> {
        let plugin = self
            .plugins
            .read()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Plugin not found: {}", id))?;

        Ok(WasmMetadata {
            version: plugin.version.parse().unwrap_or(1),
            exports: plugin.exports.clone(),
            wasi_version: plugin.wasi_version.clone(),
            required_features: plugin.required_features.clone(),
        })
    }
}

// ---------------------------------------------------------------------------
// WASM Binary Format Parser
// ---------------------------------------------------------------------------

/// Parses a WASM binary header and extracts metadata.
///
/// WASM Binary Format:
/// - Magic: 4 bytes  (\0asm)
/// - Version: 4 bytes (u32, = 1)
/// - Sections (repeated):
///   - Section ID: 1 byte
///   - Section size: LEB128 u32
///   - Content: size bytes
pub fn parse_wasm_header(bytes: &[u8]) -> anyhow::Result<WasmMetadata> {
    if bytes.len() < 8 {
        return Err(anyhow::anyhow!(
            "WASM binary too short: {} bytes",
            bytes.len()
        ));
    }

    // Check magic number
    if bytes[0..4] != WASM_MAGIC {
        return Err(anyhow::anyhow!(
            "Invalid WASM magic number: {:02x?}",
            &bytes[0..4]
        ));
    }

    // Read version (little-endian u32)
    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    if version != WASM_VERSION {
        return Err(anyhow::anyhow!(
            "Unsupported WASM version: {}, expected 1",
            version
        ));
    }

    let offset = &mut 8usize;
    let mut exports = Vec::new();
    let mut required_features = Vec::new();
    let mut wasi_version: Option<String> = None;

    while *offset < bytes.len() {
        // Read section ID
        if *offset >= bytes.len() {
            break;
        }
        let section_id = bytes[*offset];
        *offset += 1;

        // Read section size (LEB128)
        let (section_size, bytes_read) = read_leb128_u32_at(bytes, *offset)
            .ok_or_else(|| anyhow::anyhow!("Invalid LEB128 at offset {}", offset))?;
        *offset += bytes_read;

        let section_start = *offset;
        let section_end = section_start.saturating_add(section_size as usize);
        if section_end > bytes.len() {
            // Truncated section, stop parsing
            break;
        }

        match section_id {
            0 => {
                // Custom section - may contain WASI metadata or module name
                let name_end = bytes[*offset..section_end]
                    .iter()
                    .position(|&b| b == 0)
                    .unwrap_or(section_size as usize);
                let name = String::from_utf8_lossy(&bytes[*offset..*offset + name_end]).to_string();

                if name.contains("wasi_snapshot_preview1") {
                    wasi_version = Some("wasi_snapshot_preview1".to_string());
                } else if name == "emscripten_meta" {
                    required_features.push("emscripten".to_string());
                }
            }
            7 => {
                // Export section
                let slice = &bytes[*offset..section_end];
                let mut ex_offset = 0usize;

                if let Some((count, cr)) = read_leb128_u32_at(slice, ex_offset) {
                    ex_offset += cr;
                    for _ in 0..count {
                        // Read export name
                        if let Some((name_len, nr)) = read_leb128_u32_at(slice, ex_offset) {
                            ex_offset += nr;
                            if ex_offset + name_len as usize > slice.len() {
                                break;
                            }
                            let name = String::from_utf8_lossy(
                                &slice[ex_offset..ex_offset + name_len as usize],
                            )
                            .to_string();
                            ex_offset += name_len as usize;

                            // Skip export kind (1 byte) and index (LEB128)
                            if ex_offset < slice.len() {
                                // export_kind: 0=func, 1=table, 2=mem, 3=global
                                ex_offset += 1;
                            }
                            if let Some((_, ir)) = read_leb128_u32_at(slice, ex_offset) {
                                ex_offset += ir;
                            }

                            exports.push(name);
                        } else {
                            break;
                        }
                    }
                }
            }
            _ => {
                // Skip other sections (type, import, function, memory, global, code, data, etc.)
            }
        }

        *offset = section_end;
    }

    Ok(WasmMetadata {
        version,
        exports,
        wasi_version,
        required_features,
    })
}

/// Read a LEB128-encoded u32 from the given slice.
/// Returns `(value, bytes_read)` or `None` if the slice is too short.
fn read_leb128_u32_at(bytes: &[u8], offset: usize) -> Option<(u32, usize)> {
    let mut result: u32 = 0;
    let mut shift: u32 = 0;
    let mut pos = offset;

    loop {
        if pos >= bytes.len() {
            return None;
        }
        let byte = bytes[pos];
        pos += 1;

        result |= ((byte & 0x7f) as u32) << shift;

        if byte & 0x80 == 0 {
            return Some((result, pos - offset));
        }

        shift += 7;

        // Prevent overflow: a u32 LEB128 can be at most 5 bytes (35 bits)
        if shift >= 35 {
            return None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid WASM binary (no exports)
    fn minimal_wasm() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&WASM_MAGIC); // magic
        bytes.extend_from_slice(&WASM_VERSION.to_le_bytes()); // version
        bytes
    }

    /// Create a WASM binary with a single export
    fn wasm_with_exports(exports: &[&str]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&WASM_MAGIC);
        bytes.extend_from_slice(&WASM_VERSION.to_le_bytes());

        // Type section (section 1) - needed for valid function exports
        bytes.push(1); // section ID: type
        let type_content = vec![1, 0x60, 0, 0];
        write_leb128(&mut bytes, type_content.len() as u32);
        bytes.extend_from_slice(&type_content);

        // Function section (section 3) - declare functions
        bytes.push(3);
        let mut func_content: Vec<u8> = vec![exports.len() as u8];
        func_content.resize(1 + exports.len(), 0);
        write_leb128(&mut bytes, func_content.len() as u32);
        bytes.extend_from_slice(&func_content);

        // Export section (section 7)
        bytes.push(7);
        let mut export_content = Vec::new();
        write_leb128(&mut export_content, exports.len() as u32);
        for &name in exports {
            write_leb128(&mut export_content, name.len() as u32);
            export_content.extend_from_slice(name.as_bytes());
            export_content.push(0); // export kind: function
            write_leb128(&mut export_content, 0); // function index
        }
        write_leb128(&mut bytes, export_content.len() as u32);
        bytes.extend_from_slice(&export_content);

        // Code section (section 10) - empty bodies
        bytes.push(10);
        let mut code_content = vec![exports.len() as u8];
        for _ in exports {
            write_leb128(&mut code_content, 1u32);
            code_content.push(0x0b); // end instruction only
        }
        write_leb128(&mut bytes, code_content.len() as u32);
        bytes.extend_from_slice(&code_content);

        bytes
    }

    fn write_leb128(buf: &mut Vec<u8>, value: u32) {
        let mut v = value;
        loop {
            let byte = (v & 0x7f) as u8;
            v >>= 7;
            if v != 0 {
                buf.push(byte | 0x80);
            } else {
                buf.push(byte);
                break;
            }
        }
    }

    #[test]
    fn test_parse_minimal_wasm() {
        let wasm = minimal_wasm();
        let meta = parse_wasm_header(&wasm).unwrap();
        assert_eq!(meta.version, 1);
        assert!(meta.exports.is_empty());
        assert!(meta.wasi_version.is_none());
    }

    #[test]
    fn test_parse_wasm_with_exports() {
        let wasm = wasm_with_exports(&["memory_grow", "get_state", "process"]);
        let meta = parse_wasm_header(&wasm).unwrap();
        assert_eq!(meta.version, 1);
        assert_eq!(meta.exports.len(), 3);
        assert_eq!(meta.exports[0], "memory_grow");
        assert_eq!(meta.exports[1], "get_state");
        assert_eq!(meta.exports[2], "process");
    }

    #[test]
    fn test_parse_invalid_magic() {
        let invalid = b"not wasm";
        assert!(parse_wasm_header(invalid).is_err());
    }

    #[test]
    fn test_parse_too_short() {
        assert!(parse_wasm_header(&[0u8; 4]).is_err());
    }

    #[test]
    fn test_leb128_u32() {
        let mut buf = Vec::new();
        write_leb128(&mut buf, 624485u32);
        let (val, read) = read_leb128_u32_at(&buf, 0).unwrap();
        assert_eq!(val, 624485);
        assert_eq!(read, 3);
    }

    #[test]
    fn test_leb128_small() {
        let mut buf = Vec::new();
        write_leb128(&mut buf, 42u32);
        let (val, read) = read_leb128_u32_at(&buf, 0).unwrap();
        assert_eq!(val, 42);
        assert_eq!(read, 1);
    }

    #[test]
    fn test_check_compatibility() {
        let plugin = WasmPlugin {
            id: "test".to_string(),
            name: "test".to_string(),
            version: "0.1.0".to_string(),
            entry: String::new(),
            permissions: vec![],
            exports: vec![],
            wasi_version: Some("wasi_snapshot_preview1".to_string()),
            required_features: vec![],
        };
        assert!(WasmEngine::check_compatibility(&plugin).is_ok());

        let emscripten_plugin = WasmPlugin {
            required_features: vec!["emscripten".to_string()],
            ..plugin.clone()
        };
        assert!(WasmEngine::check_compatibility(&emscripten_plugin).is_err());
    }

    #[tokio::test]
    async fn test_load_and_execute_plugin() {
        let storage = Arc::new(super::super::storage::StorageEngine::new());
        let engine = WasmEngine::new(storage);

        let wasm = wasm_with_exports(&["process"]);
        let id = engine
            .load_plugin_from_bytes("test-plugin", &wasm)
            .await
            .unwrap();

        let exports = engine.get_exports(&id).await.unwrap();
        assert_eq!(exports, vec!["process"]);

        let result = engine.execute_plugin(&id, "hello").await.unwrap();
        assert!(result.contains("test-plugin"));
        assert!(result.contains("5")); // input length of "hello"
    }

    #[tokio::test]
    async fn test_register_function() {
        let storage = Arc::new(super::super::storage::StorageEngine::new());
        let engine = WasmEngine::new(storage);

        let wasm = wasm_with_exports(&["process"]);
        let id = engine
            .load_plugin_from_bytes("custom", &wasm)
            .await
            .unwrap();

        engine
            .register_function(&id, |input| Ok(format!("custom: {}", input)))
            .await;

        let result = engine.execute_plugin(&id, "test").await.unwrap();
        assert_eq!(result, "custom: test");
    }

    #[tokio::test]
    async fn test_call_export() {
        let storage = Arc::new(super::super::storage::StorageEngine::new());
        let engine = WasmEngine::new(storage);

        let wasm = wasm_with_exports(&["handler"]);
        let _id = engine
            .load_plugin_from_bytes("exp-plugin", &wasm)
            .await
            .unwrap();

        let result = engine.call_export("handler", "data").await.unwrap();
        assert!(result.contains("exp-plugin"));
    }

    #[tokio::test]
    async fn test_unload_plugin() {
        let storage = Arc::new(super::super::storage::StorageEngine::new());
        let engine = WasmEngine::new(storage);

        let wasm = wasm_with_exports(&["func"]);
        let id = engine.load_plugin_from_bytes("temp", &wasm).await.unwrap();

        assert!(engine.get_plugin(&id).await.is_some());
        engine.unload_plugin(&id).await.unwrap();
        assert!(engine.get_plugin(&id).await.is_none());
    }
}
