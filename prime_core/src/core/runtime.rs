use std::sync::Arc;
use tokio::runtime::{self, Runtime as TokioRt};
use tokio::sync::Semaphore;

pub struct TokioRuntime {
    rt: Option<TokioRt>,
    task_semaphore: Arc<Semaphore>,
}

impl Default for TokioRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl TokioRuntime {
    pub fn new() -> Self {
        let task_semaphore = Arc::new(Semaphore::new(10_000));

        let rt = match tokio::runtime::Handle::try_current() {
            Ok(_) => {
                // Already inside a tokio context — don't nest runtimes
                None
            }
            Err(_) => {
                Some(
                    runtime::Builder::new_multi_thread()
                        .worker_threads(num_cpus::get())
                        .enable_all()
                        .thread_name("prime-worker")
                        .on_thread_start(|| tracing::debug!("Worker thread started"))
                        .on_thread_stop(|| tracing::debug!("Worker thread stopped"))
                        .build()
                        .expect("Failed to build Tokio runtime"),
                )
            }
        };

        Self {
            rt,
            task_semaphore,
        }
    }

    pub fn spawn<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        match &self.rt {
            Some(rt) => {
                rt.spawn(future);
            }
            None => {
                std::mem::drop(tokio::spawn(future));
            }
        }
    }

    pub fn spawn_with_limit<F>(&self, future: F)
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let semaphore = self.task_semaphore.clone();
        match &self.rt {
            Some(rt) => {
                rt.spawn(async move {
                    let _permit = semaphore.acquire().await;
                    future.await;
                });
            }
            None => {
                std::mem::drop(tokio::spawn(async move {
                    let _permit = semaphore.acquire().await;
                    future.await;
                }));
            }
        }
    }

    pub fn block_on<F: std::future::Future>(&self, future: F) -> F::Output {
        match &self.rt {
            Some(rt) => rt.block_on(future),
            None => tokio::runtime::Handle::current().block_on(future),
        }
    }

    pub fn handle(&self) -> runtime::Handle {
        self.rt
            .as_ref()
            .map(|rt| rt.handle().clone())
            .unwrap_or_else(tokio::runtime::Handle::current)
    }
}

impl Drop for TokioRuntime {
    fn drop(&mut self) {
        if let Some(rt) = self.rt.take() {
            tracing::info!("Shutting down Tokio runtime");
            rt.shutdown_timeout(std::time::Duration::from_secs(10));
        }
    }
}
