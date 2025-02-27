use std::time::Duration;

pub struct RetryConfig {
    pub base_delay_ms: u64,
    pub max_delay_secs: u64,
}

impl RetryConfig {
    pub fn new(base_delay_ms: u64, max_delay_secs: u64) -> Self {
        Self {
            base_delay_ms,
            max_delay_secs,
        }
    }

    pub fn get_delay(&self, attempt: u32) -> Duration {
        let base = self.base_delay_ms as f64;
        let max = self.max_delay_secs * 1000;
        let delay = (base * (2_f64.powi(attempt as i32 - 1))) as u64;
        Duration::from_millis(delay.min(max))
    }
}