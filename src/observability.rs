/// Observability — atomic request counters + uptime tracking.
///
/// `Counters` is `Arc`-shared on `AppState` so all cloned handler instances
/// increment the same global totals.
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug)]
pub struct Counters {
    pub requests_total: AtomicU64,
    pub errors_total: AtomicU64,
    pub upstream_calls: AtomicU64,
    pub upstream_errors: AtomicU64,
    pub start: Instant,
}

impl Counters {
    #[must_use]
    pub fn new() -> Self {
        Self {
            requests_total: AtomicU64::new(0),
            errors_total: AtomicU64::new(0),
            upstream_calls: AtomicU64::new(0),
            upstream_errors: AtomicU64::new(0),
            start: Instant::now(),
        }
    }

    pub fn inc_requests(&self) {
        self.requests_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_errors(&self) {
        self.errors_total.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_upstream(&self) {
        self.upstream_calls.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_upstream_errors(&self) {
        self.upstream_errors.fetch_add(1, Ordering::Relaxed);
    }

    #[must_use]
    pub fn uptime_secs(&self) -> u64 {
        self.start.elapsed().as_secs()
    }

    #[must_use]
    pub fn snapshot(&self) -> CounterSnapshot {
        CounterSnapshot {
            requests_total: self.requests_total.load(Ordering::Relaxed),
            errors_total: self.errors_total.load(Ordering::Relaxed),
            upstream_calls: self.upstream_calls.load(Ordering::Relaxed),
            upstream_errors: self.upstream_errors.load(Ordering::Relaxed),
            uptime_secs: self.uptime_secs(),
        }
    }
}

impl Default for Counters {
    fn default() -> Self {
        Self::new()
    }
}

/// A point-in-time snapshot of all counters (non-atomic, safe to serialize).
#[derive(Debug, serde::Serialize)]
pub struct CounterSnapshot {
    pub requests_total: u64,
    pub errors_total: u64,
    pub upstream_calls: u64,
    pub upstream_errors: u64,
    pub uptime_secs: u64,
}
