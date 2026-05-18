use tokio::time::{sleep, Duration};

pub struct RetryLoop {
    max_retries: u32,
    base_delay_ms: u64,
    max_delay_ms: u64,
}

impl Default for RetryLoop {
    fn default() -> Self {
        Self::new()
    }
}

impl RetryLoop {
    pub fn new() -> Self {
        Self {
            max_retries: 3,
            base_delay_ms: 500,
            max_delay_ms: 10_000,
        }
    }

    pub async fn execute<F, Fut, T, E>(&self, f: F) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut last_err: Option<E> = None;

        for attempt in 0..=self.max_retries {
            match f().await {
                Ok(value) => return Ok(value),
                Err(e) => {
                    tracing::warn!(
                        "Attempt {}/{} failed: {}",
                        attempt + 1,
                        self.max_retries + 1,
                        e
                    );
                    last_err = Some(e);

                    if attempt < self.max_retries {
                        let delay = self.calculate_delay(attempt);
                        sleep(Duration::from_millis(delay)).await;
                    }
                }
            }
        }

        Err(last_err.expect("Retry loop ended without error"))
    }

    fn calculate_delay(&self, attempt: u32) -> u64 {
        let delay = self.base_delay_ms * 2u64.pow(attempt);
        std::cmp::min(delay, self.max_delay_ms)
    }
}
