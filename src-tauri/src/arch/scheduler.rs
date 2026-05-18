use chrono::{DateTime, Duration as ChronoDuration, Utc};
use futures::FutureExt;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

use super::event_bus::{EventBusCore, SystemEvent};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: String,
    pub name: String,
    pub cron_expr: String,
    pub action: String,
    pub enabled: bool,
    pub last_run: Option<DateTime<Utc>>,
    pub next_run: Option<DateTime<Utc>>,
    pub interval_secs: u64,
}

#[derive(Debug, Clone, Default)]
pub struct SchedulerStats {
    pub total_executed: u64,
    pub running: u64,
    pub pending: u64,
    pub failed: u64,
}

// ---------------------------------------------------------------------------
// Scheduler
// ---------------------------------------------------------------------------

pub struct Scheduler {
    tasks: Arc<RwLock<HashMap<String, ScheduledTask>>>,
    bus: Arc<EventBusCore>,
    paused: Arc<AtomicBool>,
    stats: Arc<RwLock<SchedulerStats>>,
}

impl Scheduler {
    pub fn new(bus: Arc<EventBusCore>) -> Self {
        let tasks = Arc::new(RwLock::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(SchedulerStats::default()));
        let paused = Arc::new(AtomicBool::new(false));

        // Spawn background loop that ticks every second
        let loop_tasks = tasks.clone();
        let loop_bus = bus.clone();
        let loop_stats = stats.clone();
        let loop_paused = paused.clone();

        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(1));
            loop {
                tick.tick().await;
                Self::run_tick(&loop_tasks, &loop_bus, &loop_stats, &loop_paused).await;
            }
        });

        Self {
            tasks,
            bus,
            paused,
            stats,
        }
    }

    // ------------------------------------------------------------------
    // Lifecycle
    // ------------------------------------------------------------------

    /// Pause task execution.  In-flight tasks are allowed to finish.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
        tracing::info!("Scheduler paused");
    }

    /// Resume task execution.
    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
        tracing::info!("Scheduler resumed");
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    // ------------------------------------------------------------------
    // Stats
    // ------------------------------------------------------------------

    pub async fn get_stats(&self) -> SchedulerStats {
        let mut stats = self.stats.write().await;
        let pending = {
            let tasks = self.tasks.read().await;
            let now = Utc::now();
            tasks
                .values()
                .filter(|t| t.enabled && t.next_run.is_some_and(|n| n <= now))
                .count() as u64
        };
        stats.pending = pending;
        stats.clone()
    }

    // ------------------------------------------------------------------
    // Task CRUD
    // ------------------------------------------------------------------

    pub async fn register(&self, task: ScheduledTask) {
        let mut tasks = self.tasks.write().await;
        let mut task = task;
        if task.next_run.is_none() {
            task.next_run = Some(Utc::now());
        }
        tasks.insert(task.id.clone(), task);
        tracing::debug!("Scheduled task registered");
    }

    pub async fn unregister(&self, id: &str) {
        self.tasks.write().await.remove(id);
        tracing::debug!("Scheduled task unregistered: {}", id);
    }

    pub async fn list(&self) -> Vec<ScheduledTask> {
        self.tasks.read().await.values().cloned().collect()
    }

    // ------------------------------------------------------------------
    // Tick logic (shared between background loop and manual calls)
    // ------------------------------------------------------------------

    async fn run_tick(
        tasks: &Arc<RwLock<HashMap<String, ScheduledTask>>>,
        bus: &Arc<EventBusCore>,
        stats: &Arc<RwLock<SchedulerStats>>,
        paused: &AtomicBool,
    ) {
        if paused.load(Ordering::SeqCst) {
            return;
        }

        let now = Utc::now();
        let due: Vec<ScheduledTask> = {
            let guard = tasks.read().await;
            guard
                .values()
                .filter(|t| {
                    if !t.enabled {
                        return false;
                    }
                    match t.next_run {
                        Some(nr) => nr <= now,
                        None => false,
                    }
                })
                .cloned()
                .collect()
        };

        if due.is_empty() {
            return;
        }

        // Advance next_run for each due task
        {
            let mut guard = tasks.write().await;
            for task in &due {
                if let Some(t) = guard.get_mut(&task.id) {
                    t.last_run = Some(now);
                    t.next_run = Some(now + ChronoDuration::seconds(task.interval_secs as i64));
                }
            }
        }

        let current_pending = due.len() as u64;
        {
            let mut s = stats.write().await;
            s.pending = current_pending;
            s.running += current_pending;
        }

        // Spawn each task independently with panic catching
        for task in due {
            let bus = bus.clone();
            let stats = stats.clone();

            tokio::spawn(async move {
                let event = SystemEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    event_type: format!("scheduler.task.{}", task.action),
                    source: "scheduler".to_string(),
                    payload: serde_json::json!({
                        "task_id": task.id,
                        "name": task.name,
                    }),
                    timestamp: Utc::now(),
                };

                // catch_unwind prevents a panicking handler from
                // taking down the scheduler loop
                let result = std::panic::AssertUnwindSafe(bus.emit(event))
                    .catch_unwind()
                    .await;

                let mut s = stats.write().await;
                s.running = s.running.saturating_sub(1);
                match result {
                    Ok(_) => {
                        s.total_executed += 1;
                        tracing::debug!("Scheduled task completed: {}", task.name);
                    }
                    Err(_) => {
                        s.failed += 1;
                        tracing::error!("Scheduled task panicked: {}", task.name);
                    }
                }
            });
        }
    }

    /// Public tick — can be called externally to force a tick.
    pub async fn tick(&self) {
        Self::run_tick(&self.tasks, &self.bus, &self.stats, &self.paused).await;
    }
}
