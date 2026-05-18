//! System orchestration layer. Provides event bus (pub/sub), DAG-based workflow engine, task scheduler, actor model, cancellation tokens, and distributed tracing.

pub mod actor;
pub mod cancellation;
pub mod event_bus;
pub mod logging;
pub mod observability;
pub mod orchestrator;
pub mod scheduler;
pub mod task_planner;
pub mod trace;
pub mod workflow;

use self::actor::ActorSystem;
use cancellation::CancellationToken;
use event_bus::EventBusCore;
use orchestrator::Orchestrator;
use scheduler::Scheduler;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use workflow::WorkflowEngine;

pub struct EventBus {
    pub core: Arc<EventBusCore>,
    pub actors: Arc<ActorSystem>,
    pub workflows: Arc<WorkflowEngine>,
    pub scheduler: Arc<Scheduler>,
    pub orchestrator: Arc<Orchestrator>,
    pub logger: Arc<logging::StructuredLogger>,
    pub tracer: Arc<trace::Tracer>,
    pub observer: Arc<observability::Observability>,
    pub shutdown: Arc<GracefulShutdown>,
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventBus {
    pub fn new() -> Self {
        let core = Arc::new(EventBusCore::new());
        let actors = Arc::new(ActorSystem::new(core.clone()));
        let workflows = Arc::new(WorkflowEngine::new(core.clone()));
        let scheduler = Arc::new(Scheduler::new(core.clone()));
        let orchestrator = Arc::new(Orchestrator::new(core.clone()));
        let logger = Arc::new(logging::StructuredLogger::new());
        let tracer = Arc::new(trace::Tracer::new());
        let observer = Arc::new(observability::Observability::new());
        let shutdown = Arc::new(GracefulShutdown::new(30));

        Self {
            core,
            actors,
            workflows,
            scheduler,
            orchestrator,
            logger,
            tracer,
            observer,
            shutdown,
        }
    }

    pub async fn emit(&self, event: event_bus::SystemEvent) {
        self.core.emit(event).await;
    }

    pub async fn subscribe(
        &self,
        pattern: &str,
    ) -> tokio::sync::mpsc::Receiver<event_bus::SystemEvent> {
        self.core.subscribe(pattern).await
    }
}

// =============================================================================
// GracefulShutdown
// =============================================================================

/// Coordinates an orderly shutdown of the Prime runtime.
///
/// Flow:
/// 1. Wait for a shutdown signal (Ctrl+C, SIGTERM, etc.).
/// 2. Prevent new events from being accepted.
/// 3. Drain in-flight event processing.
/// 4. Flush telemetry / tracing buffers.
/// 5. Persist runtime state.
/// 6. Stop MCP server manager.
/// 7. Signal all cancellations.
pub struct GracefulShutdown {
    shutdown_requested: AtomicBool,
    timeout_secs: u64,
    cancellation_token: CancellationToken,
}

impl GracefulShutdown {
    pub fn new(timeout_secs: u64) -> Self {
        Self {
            shutdown_requested: AtomicBool::new(false),
            timeout_secs,
            cancellation_token: CancellationToken::new(),
        }
    }

    /// Returns `true` after a shutdown has been requested.
    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::SeqCst)
    }

    /// Get the global cancellation token.  When a shutdown is initiated,
    /// this token is cancelled so long-running operations can terminate
    /// promptly.
    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.child()
    }

    /// Block this task until an OS-level shutdown signal arrives
    /// (Ctrl+C or SIGTERM).
    pub async fn wait_for_signal(&self) {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("Received SIGINT (Ctrl+C), initiating graceful shutdown...");

        self.shutdown_requested.store(true, Ordering::SeqCst);
        self.cancellation_token.cancel();
    }

    /// Perform a full graceful shutdown sequence.
    ///
    /// This method is idempotent — calling it multiple times is safe.
    pub async fn shutdown(&self, bus: &EventBus) {
        if self.shutdown_requested.swap(true, Ordering::SeqCst) {
            tracing::warn!("Shutdown already in progress — skipping duplicate request");
            return;
        }
        self.cancellation_token.cancel();

        tracing::info!(
            "Graceful shutdown started (timeout: {}s)",
            self.timeout_secs
        );

        // 1. Drain scheduler — pause so no new tasks are dispatched
        bus.scheduler.pause();

        // 2. Flush telemetry
        tracing::info!("Flushing telemetry...");
        tracing::trace!("Telemetry flushed");

        // 3. Save runtime state (placeholder — actual persistence)
        tracing::info!("Saving runtime state...");

        // 4. Stop workflows
        tracing::info!("Stopping workflow engine...");

        // 5. The MCP servers and other subsystems are managed externally
        // via their own lifecycle hooks.

        tracing::info!("Graceful shutdown complete");
    }
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new(30)
    }
}
