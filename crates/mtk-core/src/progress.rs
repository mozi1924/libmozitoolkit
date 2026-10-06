//! Unified physical progress reporting types, traits, and throttler.
//!
//! Provides host-agnostic, zero-cost abstractions for measuring and broadcasting
//! exact task execution milestones without synthetic heuristics or estimation.

use alloc::format;
use alloc::string::String;
use core::sync::atomic::{AtomicUsize, Ordering};

/// Snapshot of an active task execution milestone with exact physical counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressReport {
    /// Identifier of current stage, e.g. "load_chunks", "meshing_sections", "bake_models".
    pub stage: &'static str,
    /// Exact count of physical items processed so far.
    pub current: usize,
    /// Total count of physical items expected for this stage.
    pub total: usize,
    /// Human-readable description text.
    pub message: String,
}

impl ProgressReport {
    /// Constructs a new progress report milestone.
    pub fn new(
        stage: &'static str,
        current: usize,
        total: usize,
        message: impl Into<String>,
    ) -> Self {
        Self {
            stage,
            current,
            total,
            message: message.into(),
        }
    }

    /// Computes percentage in range `[0.0, 100.0]`.
    #[inline]
    pub fn percent(&self) -> f32 {
        if self.total == 0 {
            100.0
        } else {
            (self.current as f32 / self.total as f32) * 100.0
        }
    }
}

/// Callback function signature accepting progress reports.
pub type ProgressCallback<'a> = &'a (dyn Fn(ProgressReport) + Send + Sync);

/// Thread-safe progress reporter with atomic counters and frequency throttling.
///
/// Designed for Rayon and high-throughput loops to prevent performance degradation
/// and lock contention caused by microsecond-level callbacks across language boundaries.
pub struct ProgressThrottler<'a> {
    stage: &'static str,
    total: usize,
    step: usize,
    completed: AtomicUsize,
    last_reported: AtomicUsize,
    callback: Option<ProgressCallback<'a>>,
    description_prefix: Option<&'static str>,
}

impl<'a> ProgressThrottler<'a> {
    /// Creates a new throttler for `total` items.
    /// Default reporting step is 1% or at least 1 item.
    pub fn new(stage: &'static str, total: usize, callback: Option<ProgressCallback<'a>>) -> Self {
        let step = (total / 100).max(1);
        Self {
            stage,
            total,
            step,
            completed: AtomicUsize::new(0),
            last_reported: AtomicUsize::new(0),
            callback,
            description_prefix: None,
        }
    }

    /// Overrides minimum step delta between consecutive progress notifications.
    pub fn with_step(mut self, step: usize) -> Self {
        self.step = step.max(1);
        self
    }

    /// Sets human-readable description prefix (e.g. "Meshing chunk").
    pub fn with_prefix(mut self, prefix: &'static str) -> Self {
        self.description_prefix = Some(prefix);
        self
    }

    /// Increments completed counter by 1 and reports milestone if throttle threshold is met.
    #[inline]
    pub fn inc(&self) -> usize {
        self.inc_by(1)
    }

    /// Increments completed counter by `n` and reports milestone if throttle threshold is met.
    pub fn inc_by(&self, n: usize) -> usize {
        let current = self.completed.fetch_add(n, Ordering::SeqCst) + n;
        if let Some(cb) = self.callback {
            let last = self.last_reported.load(Ordering::Relaxed);
            if current >= self.total || current - last >= self.step {
                self.last_reported.store(current, Ordering::Relaxed);
                let msg = match self.description_prefix {
                    Some(prefix) => format!("{prefix} ({current}/{})", self.total),
                    None => format!("{}: ({current}/{})", self.stage, self.total),
                };
                cb(ProgressReport::new(self.stage, current, self.total, msg));
            }
        }
        current
    }

    /// Returns current completed count.
    #[inline]
    pub fn current(&self) -> usize {
        self.completed.load(Ordering::Relaxed)
    }

    /// Returns total item count.
    #[inline]
    pub fn total(&self) -> usize {
        self.total
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::sync::Arc;
    use core::sync::atomic::AtomicUsize;

    #[test]
    fn test_progress_report_percent() {
        let r = ProgressReport::new("meshing", 50, 100, "Halfway");
        assert_eq!(r.percent(), 50.0);
        let r_zero = ProgressReport::new("meshing", 0, 0, "Empty");
        assert_eq!(r_zero.percent(), 100.0);
    }

    #[test]
    fn test_progress_throttler() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let call_count_clone = Arc::clone(&call_count);

        let cb = move |r: ProgressReport| {
            assert_eq!(r.stage, "test_stage");
            call_count_clone.fetch_add(1, Ordering::SeqCst);
        };

        let throttler = ProgressThrottler::new("test_stage", 100, Some(&cb)).with_step(10);

        for _ in 0..100 {
            throttler.inc();
        }

        assert_eq!(throttler.current(), 100);
        // Step is 10, total is 100 => called ~10 times
        assert!(call_count.load(Ordering::SeqCst) >= 9);
    }
}
