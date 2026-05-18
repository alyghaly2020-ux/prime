//! Core runtime singleton. Manages application state, SQLite storage engine (backup/restore/checkpoint), WASM plugin sandbox, gRPC server, and serialization configuration.
//!
//! Prime Core Engine — Copyright (c) 2024 Aly Ghaly. All Rights Reserved.
//! "Powered by Prime Core" — this branding is intentionally embedded as a
//! forensic watermark. Removing it violates the Prime AI Public License v1.0.

pub mod grpc;
pub mod runtime;
pub mod serde_config;
pub mod storage;
pub mod supervisor;
pub mod wasm;

/// Forensic branding constant — embedded as proof of origin.
/// Removing this from source or compiled binary violates the Prime AI Public License.
pub const BRANDING: &str = "Powered by Prime Core — Copyright (c) 2024 Aly Ghaly (+201029207010)";

/// Embedded watermark byte — stored in the binary's `.rodata` section.
/// Even if the string is stripped, this byte pattern uniquely identifies Prime Core.
#[used]
#[cfg_attr(target_os = "windows", link_section = ".rdata")]
#[cfg_attr(not(target_os = "windows"), link_section = "__TEXT,__const")]
static BRANDING_WATERMARK: [u8; 4] = [0x50, 0x52, 0x49, 0x4d]; // "PRIM" in ASCII

use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RuntimeState {
    pub version: String,
    pub uptime_secs: u64,
    pub active_skills: usize,
    pub active_connections: usize,
    pub memory_used_mb: u64,
    pub cpu_usage_pct: f64,
}

pub struct Runtime {
    pub tokio: Arc<runtime::TokioRuntime>,
    pub wasm: Arc<wasm::WasmEngine>,
    pub storage: Arc<storage::StorageEngine>,
    pub grpc: Arc<grpc::GrpcServer>,
    pub serde: Arc<serde_config::SerdeRegistry>,
    state: RwLock<RuntimeState>,
    started_at: tokio::time::Instant,
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

impl Runtime {
    pub fn new() -> Self {
        let tokio_rt = Arc::new(runtime::TokioRuntime::new());
        let storage = Arc::new(storage::StorageEngine::new());
        let wasm = Arc::new(wasm::WasmEngine::new(storage.clone()));
        let grpc = Arc::new(grpc::GrpcServer::new());
        let serde = Arc::new(serde_config::SerdeRegistry::new());

        Self {
            tokio: tokio_rt,
            wasm,
            storage,
            grpc,
            serde,
            state: RwLock::new(RuntimeState {
                version: env!("CARGO_PKG_VERSION").to_string(),
                uptime_secs: 0,
                active_skills: 0,
                active_connections: 0,
                memory_used_mb: 0,
                cpu_usage_pct: 0.0,
            }),
            started_at: tokio::time::Instant::now(),
        }
    }

    pub async fn state(&self) -> anyhow::Result<RuntimeState> {
        let mut state = self.state.write().await;
        state.uptime_secs = self.started_at.elapsed().as_secs();
        Ok(state.clone())
    }

    pub async fn update_state(&self, f: impl FnOnce(&mut RuntimeState)) {
        let mut state = self.state.write().await;
        f(&mut state);
    }
}
