// Metrics for retry operations
use std::fmt;
use std::time::Duration;

/// Metrics collected during retry operations
#[derive(Debug, Clone, Default)]
pub struct RetryMetrics {
    /// Number of attempts made
    pub attempts: u32,
    /// Total delay accumulated across all retries
    pub total_delay: Duration,
    /// Whether the operation ultimately succeeded
    pub succeeded: bool,
    /// Whether the operation timed out
    pub timed_out: bool,
}

impl RetryMetrics {
    /// Create new metrics with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the average delay between attempts
    pub fn average_delay(&self) -> Option<Duration> {
        if self.attempts <= 1 {
            None
        } else {
            Some(self.total_delay / (self.attempts - 1))
        }
    }

    /// Get success rate (1.0 if succeeded, 0.0 if failed)
    pub fn success_rate(&self) -> f64 {
        if self.succeeded {
            1.0
        } else {
            0.0
        }
    }
}

impl fmt::Display for RetryMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "RetryMetrics {{ attempts: {}, total_delay: {:?}, succeeded: {}, timed_out: {} }}",
            self.attempts, self.total_delay, self.succeeded, self.timed_out
        )
    }
}
