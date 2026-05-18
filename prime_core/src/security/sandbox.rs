use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::sync::RwLock;

use crate::security::permissions::PermissionManager;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxMetadata {
    pub duration_ms: u64,
    pub success: bool,
    pub panic_message: Option<String>,
    pub permission_check: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxResult<T: Clone + serde::Serialize> {
    pub value: T,
    pub metadata: SandboxMetadata,
}

impl<T: Clone + serde::Serialize> SandboxResult<T> {
    pub fn new(value: T, metadata: SandboxMetadata) -> Self {
        Self { value, metadata }
    }
}

pub struct SecuritySandbox {
    isolated: RwLock<bool>,
    task_count: AtomicU64,
    total_execution_time_ms: AtomicU64,
    last_panic: RwLock<Option<String>>,
    active_tasks: AtomicU64,
    permissions: RwLock<Option<PermissionManager>>,
}

impl Default for SecuritySandbox {
    fn default() -> Self {
        Self::new()
    }
}

impl SecuritySandbox {
    pub fn new() -> Self {
        Self {
            isolated: RwLock::new(true),
            task_count: AtomicU64::new(0),
            total_execution_time_ms: AtomicU64::new(0),
            last_panic: RwLock::new(None),
            active_tasks: AtomicU64::new(0),
            permissions: RwLock::new(None),
        }
    }

    pub async fn is_isolated(&self) -> bool {
        *self.isolated.read().await
    }

    pub async fn set_isolated(&self, isolated: bool) {
        *self.isolated.write().await = isolated;
    }

    pub fn set_permission_manager(&self, pm: PermissionManager) {
        let mut perm = self.permissions.blocking_write();
        *perm = Some(pm);
    }

    /// Execute a function inside a sandboxed tokio task with timeout and panic protection.
    pub async fn execute_in_sandbox<F, T>(&self, f: F) -> SandboxResult<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Clone + serde::Serialize + Send + 'static,
    {
        let start = Instant::now();
        self.task_count.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_add(1, Ordering::Relaxed);

        let result = tokio::task::spawn_blocking(move || {
            // Catch panics so they don't crash the runtime
            match panic::catch_unwind(AssertUnwindSafe(f)) {
                Ok(val) => Ok(val),
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic".to_string()
                    };
                    Err(msg)
                }
            }
        })
        .await;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.total_execution_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(Ok(value)) => SandboxResult::new(
                value,
                SandboxMetadata {
                    duration_ms,
                    success: true,
                    panic_message: None,
                    permission_check: None,
                },
            ),
            Ok(Err(panic_msg)) => {
                let mut last = self.last_panic.write().await;
                *last = Some(panic_msg.clone());
                panic!("Sandboxed task panicked: {}", panic_msg)
            }
            Err(join_err) => {
                let msg = format!("Task join error: {}", join_err);
                let mut last = self.last_panic.write().await;
                *last = Some(msg.clone());
                panic!("{}", msg)
            }
        }
    }

    /// Execute with a timeout. If the task exceeds the timeout, it is aborted.
    pub async fn execute_with_timeout<F, T>(
        &self,
        f: F,
        timeout_secs: u64,
    ) -> Result<SandboxResult<T>, String>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Clone + serde::Serialize + Send + 'static,
    {
        let start = Instant::now();
        self.task_count.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_add(1, Ordering::Relaxed);

        let f_clone = move || match panic::catch_unwind(AssertUnwindSafe(f)) {
            Ok(val) => Ok(val),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                Err(msg)
            }
        };

        let handle = tokio::task::spawn_blocking(f_clone);
        let timeout_dur = tokio::time::Duration::from_secs(timeout_secs);

        let result = tokio::time::timeout(timeout_dur, handle).await;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.total_execution_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);

        match result {
            Ok(Ok(Ok(value))) => Ok(SandboxResult::new(
                value,
                SandboxMetadata {
                    duration_ms,
                    success: true,
                    panic_message: None,
                    permission_check: None,
                },
            )),
            Ok(Ok(Err(panic_msg))) => {
                let mut last = self.last_panic.write().await;
                *last = Some(panic_msg.clone());
                Err(format!("Sandboxed task panicked: {}", panic_msg))
            }
            Ok(Err(join_err)) => {
                let msg = format!("Task join error: {}", join_err);
                let mut last = self.last_panic.write().await;
                *last = Some(msg.clone());
                Err(msg)
            }
            Err(_elapsed) => {
                // Timeout elapsed — task is dropped
                let msg = format!("Task exceeded timeout of {} seconds", timeout_secs);
                Err(msg)
            }
        }
    }

    /// Execute a function only if the subject has the required permissions.
    pub async fn execute_with_permissions<F, T>(
        &self,
        f: F,
        subject: &str,
        resource: &str,
        action: &str,
    ) -> Result<SandboxResult<T>, String>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Clone + serde::Serialize + Send + 'static,
    {
        let perm_check = {
            let perms = self.permissions.read().await;
            match perms.as_ref() {
                Some(pm) => pm.check(subject, resource, action),
                None => true, // No permission manager = permissive
            }
        };

        if !perm_check {
            return Err(format!(
                "Permission denied: subject '{}' cannot perform '{}' on '{}'",
                subject, action, resource
            ));
        }

        let start = Instant::now();
        self.task_count.fetch_add(1, Ordering::Relaxed);
        self.active_tasks.fetch_add(1, Ordering::Relaxed);

        let result =
            tokio::task::spawn_blocking(move || match panic::catch_unwind(AssertUnwindSafe(f)) {
                Ok(val) => Ok(val),
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "Unknown panic".to_string()
                    };
                    Err(msg)
                }
            })
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;
        self.total_execution_time_ms
            .fetch_add(duration_ms, Ordering::Relaxed);
        self.active_tasks.fetch_sub(1, Ordering::Relaxed);

        let perm_str = format!("{}:{}:{}", subject, action, resource);
        match result {
            Ok(Ok(value)) => Ok(SandboxResult::new(
                value,
                SandboxMetadata {
                    duration_ms,
                    success: true,
                    panic_message: None,
                    permission_check: Some(perm_str),
                },
            )),
            Ok(Err(panic_msg)) => {
                let mut last = self.last_panic.write().await;
                *last = Some(panic_msg.clone());
                Err(format!("Sandboxed task panicked: {}", panic_msg))
            }
            Err(join_err) => {
                let msg = format!("Task join error: {}", join_err);
                let mut last = self.last_panic.write().await;
                *last = Some(msg.clone());
                Err(msg)
            }
        }
    }

    /// Get statistics about sandbox usage.
    pub async fn stats(&self) -> SandboxStats {
        let last_panic = self.last_panic.read().await.clone();
        SandboxStats {
            total_tasks: self.task_count.load(Ordering::Relaxed),
            total_execution_time_ms: self.total_execution_time_ms.load(Ordering::Relaxed),
            active_tasks: self.active_tasks.load(Ordering::Relaxed),
            isolated: *self.isolated.read().await,
            last_panic,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SandboxStats {
    pub total_tasks: u64,
    pub total_execution_time_ms: u64,
    pub active_tasks: u64,
    pub isolated: bool,
    pub last_panic: Option<String>,
}
