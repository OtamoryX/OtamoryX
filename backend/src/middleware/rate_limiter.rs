use dashmap::DashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

const MAX_ATTEMPTS: u32 = 5;
const WINDOW_DURATION: Duration = Duration::from_secs(5 * 60); // 5 minutes

pub struct LoginRateLimiter {
    attempts: DashMap<String, (u32, Instant)>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: DashMap::new(),
        }
    }

    pub fn check_rate_limit(&self, key: &str) -> Result<(), Duration> {
        if let Some(entry) = self.attempts.get(key) {
            let (count, first_attempt) = entry.value();
            let elapsed = first_attempt.elapsed();
            if elapsed < WINDOW_DURATION && *count >= MAX_ATTEMPTS {
                return Err(WINDOW_DURATION - elapsed);
            }
        }
        Ok(())
    }

    pub fn record_failure(&self, key: &str) {
        self.attempts
            .entry(key.to_string())
            .and_modify(|(count, first_attempt)| {
                if first_attempt.elapsed() >= WINDOW_DURATION {
                    // Window expired, reset
                    *count = 1;
                    *first_attempt = Instant::now();
                } else {
                    *count += 1;
                }
            })
            .or_insert((1, Instant::now()));
    }

    pub fn reset(&self, key: &str) {
        self.attempts.remove(key);
    }

    pub fn start_cleanup_task(self: &Arc<Self>) {
        let limiter = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                limiter
                    .attempts
                    .retain(|_, (_, first_attempt)| first_attempt.elapsed() < WINDOW_DURATION);
            }
        });
    }
}
