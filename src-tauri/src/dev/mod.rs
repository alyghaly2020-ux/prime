//! Developer tools. Agent registry (93 agents with dynamic model routing), workspace file indexing, semantic retrieval, code patch generation, live-reload, and workspace synchronization.

pub mod agents;
pub mod governance;
pub mod indexing;
pub mod live_reload;
pub mod patches;
pub mod retrieval;
pub mod workspace;

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::code_intel::deps::DepGraph;
use crate::code_intel::symbols::SymbolIndex;
use crate::memory::vector::VectorMemory;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevSession {
    pub id: String,
    pub workspace_path: String,
    pub active: bool,
    pub indexed: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct Engine {
    pub indexing: Arc<indexing::RepoIndexer>,
    pub retrieval: Arc<retrieval::SemanticRetrieval>,
    pub patches: Arc<patches::CodePatches>,
    pub live_reload: Arc<live_reload::LiveReload>,
    pub workspace: Arc<workspace::WorkspaceSync>,
    pub agents: Arc<agents::MultiAgentWorkflow>,
    pub governance: Arc<governance::Governance>,
    sessions: RwLock<Vec<DevSession>>,
}

impl std::fmt::Debug for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish_non_exhaustive()
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        Self {
            indexing: Arc::new(indexing::RepoIndexer::new()),
            retrieval: Arc::new(retrieval::SemanticRetrieval::new()),
            patches: Arc::new(patches::CodePatches::new()),
            live_reload: Arc::new(live_reload::LiveReload::new()),
            workspace: Arc::new(workspace::WorkspaceSync::new()),
            agents: Arc::new(agents::MultiAgentWorkflow::new()),
            governance: Arc::new(governance::Governance::new()),
            sessions: RwLock::new(Vec::new()),
        }
    }

    /// Seed all 93 agent definitions and validate them. Call once at startup.
    pub async fn seed_agents(&self) {
        self.agents.seed_all().await;

        // Validate all seeded agents
        let agents = self.agents.available_agents().await;
        let report = self.governance.validate_agents(&agents);
        for err in &report.agent_errors {
            tracing::error!("[Governance] {}: {}", err.category, err.message);
        }
        for warn in &report.warnings {
            tracing::warn!("[Governance] {}: {}", warn.category, warn.message);
        }
        if report.passed {
            tracing::info!(
                "[Governance] All {} agents passed validation",
                report.total_agents
            );
        } else {
            tracing::error!(
                "[Governance] {} agent(s) failed validation!",
                report.agent_errors.len()
            );
        }

        // Validate default workflows
        let workflows = self.agents.list_workflow_defs().await;
        let wf_report = self.governance.validate_workflows(&workflows, &agents);
        for err in &wf_report.workflow_errors {
            tracing::error!("[Governance] {}: {}", err.category, err.message);
        }
        if wf_report.passed {
            tracing::info!(
                "[Governance] All {} workflows passed validation",
                wf_report.total_workflows
            );
        } else {
            tracing::error!(
                "[Governance] {} workflow(s) failed validation!",
                wf_report.workflow_errors.len()
            );
        }
    }

    pub async fn start_session(&self, workspace: &str) -> String {
        let session = DevSession {
            id: uuid::Uuid::new_v4().to_string(),
            workspace_path: workspace.to_string(),
            active: true,
            indexed: false,
            created_at: chrono::Utc::now(),
        };
        let id = session.id.clone();
        self.sessions.write().await.push(session);
        id
    }

    pub async fn index_workspace(&self, session_id: &str) -> anyhow::Result<()> {
        let sessions = self.sessions.read().await;
        if let Some(session) = sessions.iter().find(|s| s.id == session_id) {
            self.indexing.index(&session.workspace_path).await?;
        }
        Ok(())
    }

    /// Initialize the retrieval subsystem with the required code intelligence
    /// and memory subsystems. Must be called before using retrieval methods.
    pub fn init_retrieval(
        &self,
        vector_memory: Arc<VectorMemory>,
        symbol_index: Arc<SymbolIndex>,
        dep_graph: Arc<DepGraph>,
    ) {
        self.retrieval.init(vector_memory, symbol_index, dep_graph);
    }
}
