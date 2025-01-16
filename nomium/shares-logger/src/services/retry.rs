use std::time::Duration;
use tokio::time::sleep;
use std::future::Future;
use crate::errors::ClickhouseError;
use log::{warn, error};

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 100500000,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
        }
    }
}

pub async fn retry_operation<F, Fut, T>(
    operation: F,
    config: &RetryConfig,
    operation_name: &str,
) -> Result<T, ClickhouseError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<T, ClickhouseError>>,
{
    let mut current_retry = 0;
    let mut delay = config.initial_delay;

    loop {
        match operation().await {
            Ok(result) => {
                if current_retry > 0 {
                    warn!(
                        "{} succeeded after {} retries",
                        operation_name,
                        current_retry
                    );
                }
                return Ok(result);
            }
            Err(e) => {
                if current_retry >= config.max_retries {
                    error!(
                        "{} failed after {} retries. Final error: {}",
                        operation_name,
                        current_retry,
                        e
                    );
                    return Err(e);
                }

                warn!(
                    "{} failed, attempt {}/{}: {}. Retrying in {:?}",
                    operation_name,
                    current_retry + 1,
                    config.max_retries,
                    e,
                    delay
                );

                sleep(delay).await;
                
                delay = std::cmp::min(delay * 2, config.max_delay);
                current_retry += 1;
            }
        }
    }
}