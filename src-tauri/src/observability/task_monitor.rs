//! Task Monitor
//!
//! Tracks all running/pending tasks across the system.
//! Provides summary statistics on task health and duration.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskInfo {
    pub id: String,
    pub metadata: HashMap<String, String>,
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, PartialEq, Eq)]
pub enum TaskStatus {
    Running,
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TaskSummary {
    pub running: u64,
    pub pending: u64,
    pub completed: u64,
    pub failed: u64,
    pub total: u64,
    pub avg_duration_ms: f64,
}

pub struct TaskMonitor {
    tasks: RwLock<HashMap<String, TaskInfo>>,
    total_completed: AtomicU64,
    total_failed: AtomicU64,
    total_duration_ms: AtomicU64,
    duration_count: AtomicU64,
}

impl Default for TaskMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskMonitor {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            total_completed: AtomicU64::new(0),
            total_failed: AtomicU64::new(0),
            total_duration_ms: AtomicU64::new(0),
            duration_count: AtomicU64::new(0),
        }
    }

    /// Start tracking a new task.
    pub fn register_task(&self, id: String, metadata: HashMap<String, String>) {
        let tid = id.clone();
        let mut tasks = self.tasks.write();
        tasks.insert(
            tid,
            TaskInfo {
                id,
                status: TaskStatus::Pending,
                metadata,
                created_at: chrono::Utc::now(),
                duration_ms: None,
                error: None,
            },
        );
    }

    /// Mark a task as completed.
    pub fn complete_task(&self, id: &str, result: &str) -> bool {
        let mut tasks = self.tasks.write();
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Completed;
            let start = task.created_at;
            let dur_ms = (chrono::Utc::now() - start).num_milliseconds() as u64;
            task.duration_ms = Some(dur_ms);
            task.metadata
                .insert("result".to_string(), result.to_string());
            self.total_completed.fetch_add(1, Ordering::Relaxed);
            self.total_duration_ms.fetch_add(dur_ms, Ordering::Relaxed);
            self.duration_count.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Mark a task as failed.
    pub fn fail_task(&self, id: &str, error: &str) -> bool {
        let mut tasks = self.tasks.write();
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Failed;
            let start = task.created_at;
            let dur_ms = (chrono::Utc::now() - start).num_milliseconds() as u64;
            task.duration_ms = Some(dur_ms);
            task.error = Some(error.to_string());
            self.total_failed.fetch_add(1, Ordering::Relaxed);
            self.total_duration_ms.fetch_add(dur_ms, Ordering::Relaxed);
            self.duration_count.fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Get a snapshot summary of all tasks.
    pub fn get_task_summary(&self) -> TaskSummary {
        let tasks = self.tasks.read();
        let mut running = 0;
        let mut pending = 0;
        let mut completed = 0;
        let mut failed = 0;

        for task in tasks.values() {
            match task.status {
                TaskStatus::Running => running += 1,
                TaskStatus::Pending => pending += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Failed => failed += 1,
            }
        }

        let dc = self.duration_count.load(Ordering::Relaxed);
        let avg_duration = if dc > 0 {
            self.total_duration_ms.load(Ordering::Relaxed) as f64 / dc as f64
        } else {
            0.0
        };

        TaskSummary {
            running,
            pending,
            completed,
            failed,
            total: tasks.len() as u64,
            avg_duration_ms: avg_duration,
        }
    }

    /// Get a specific task's info.
    pub fn get_task(&self, id: &str) -> Option<TaskInfo> {
        self.tasks.read().get(id).cloned()
    }

    /// List all tasks with an optional status filter.
    pub fn list_tasks(&self, status_filter: Option<TaskStatus>) -> Vec<TaskInfo> {
        let tasks = self.tasks.read();
        tasks
            .values()
            .filter(|t| status_filter.as_ref().is_none_or(|s| t.status == *s))
            .cloned()
            .collect()
    }

    /// Remove a task from tracking.
    pub fn remove_task(&self, id: &str) -> bool {
        self.tasks.write().remove(id).is_some()
    }

    /// Clear all completed/failed tasks.
    pub fn clean_completed(&self) {
        let mut tasks = self.tasks.write();
        tasks.retain(|_, t| matches!(t.status, TaskStatus::Running | TaskStatus::Pending));
    }
}
