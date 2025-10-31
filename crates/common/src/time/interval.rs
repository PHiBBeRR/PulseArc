//! Recurring intervals with jitter support
//!
//! Provides utilities for creating recurring intervals with optional jitter.

use std::time::Duration;

use rand::Rng;
use tokio::time::{sleep, Instant, Interval as TokioInterval};

/// Configuration for an interval
#[derive(Debug, Clone)]
pub struct IntervalConfig {
    /// Base duration for the interval
    pub duration: Duration,

    /// Optional jitter factor (0.0 - 1.0)
    /// 0.0 = no jitter, 1.0 = up to 100% jitter
    pub jitter: Option<f64>,

    /// Whether to skip missed ticks
    pub skip_missed_ticks: bool,
}

impl IntervalConfig {
    /// Create a new interval configuration
    pub fn new(duration: Duration) -> Self {
        Self { duration, jitter: None, skip_missed_ticks: false }
    }

    /// Set the jitter factor (0.0 - 1.0)
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter = Some(jitter.clamp(0.0, 1.0));
        self
    }

    /// Set whether to skip missed ticks
    pub fn skip_missed_ticks(mut self, skip: bool) -> Self {
        self.skip_missed_ticks = skip;
        self
    }
}

/// A recurring interval with optional jitter
pub struct Interval {
    config: IntervalConfig,
    inner: Option<TokioInterval>,
}

impl Interval {
    /// Create a new interval
    pub fn new(config: IntervalConfig) -> Self {
        let inner = if config.jitter.is_none() {
            let mut interval = tokio::time::interval(config.duration);
            if config.skip_missed_ticks {
                interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            }
            Some(interval)
        } else {
            None
        };

        Self { config, inner }
    }

    /// Create a simple interval without jitter
    pub fn simple(duration: Duration) -> Self {
        Self::new(IntervalConfig::new(duration))
    }

    /// Create an interval with jitter
    pub fn with_jitter(duration: Duration, jitter: f64) -> Self {
        Self::new(IntervalConfig::new(duration).with_jitter(jitter))
    }

    /// Wait for the next tick
    pub async fn tick(&mut self) -> Instant {
        if let Some(ref mut inner) = self.inner {
            // No jitter - use tokio interval
            inner.tick().await
        } else {
            // With jitter - calculate next delay
            let base_duration = self.config.duration;
            let jitter_factor = self.config.jitter.unwrap_or(0.0);

            let jitter_range = base_duration.as_secs_f64() * jitter_factor;
            let jitter_offset = rand::thread_rng().gen_range(-jitter_range..jitter_range);
            let delay =
                Duration::from_secs_f64((base_duration.as_secs_f64() + jitter_offset).max(0.0));

            sleep(delay).await;
            Instant::now()
        }
    }

    /// Reset the interval to start immediately on the next tick
    pub fn reset(&mut self) {
        if let Some(ref mut inner) = self.inner {
            inner.reset();
        }
    }
}

/// Create a simple interval
pub fn interval(duration: Duration) -> Interval {
    Interval::simple(duration)
}

/// Create an interval with jitter
pub fn interval_with_jitter(duration: Duration, jitter: f64) -> Interval {
    Interval::with_jitter(duration, jitter)
}

#[cfg(test)]
mod tests {
    //! Unit tests for time::interval.
    use super::*;

    /// Validates `Interval::simple` behavior for the simple interval scenario.
    ///
    /// Assertions:
    /// - Ensures `first.duration_since(start) < Duration::from_millis(5)`
    ///   evaluates to true.
    /// - Ensures `elapsed >= Duration::from_millis(8)` evaluates to true.
    /// - Ensures `elapsed <= Duration::from_millis(15)` evaluates to true.
    #[tokio::test]
    async fn test_simple_interval() {
        // Pause time for deterministic testing
        tokio::time::pause();

        let mut interval = Interval::simple(Duration::from_millis(10));

        let start = Instant::now();
        interval.tick().await; // First tick is immediate
        let first = Instant::now();

        interval.tick().await; // Second tick after duration
        let second = Instant::now();

        // First tick should be roughly immediate
        assert!(first.duration_since(start) < Duration::from_millis(5));

        // Second tick should be after the interval duration
        let elapsed = second.duration_since(first);
        assert!(elapsed >= Duration::from_millis(8)); // Allow some tolerance
        assert!(elapsed <= Duration::from_millis(15));
    }

    /// Validates `Interval::with_jitter` behavior for the interval with jitter
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `elapsed >= Duration::from_millis(70)` evaluates to true.
    /// - Ensures `elapsed <= Duration::from_millis(130)` evaluates to true.
    #[tokio::test]
    async fn test_interval_with_jitter() {
        let mut interval = Interval::with_jitter(Duration::from_millis(100), 0.2);

        let start = Instant::now();
        interval.tick().await;
        let elapsed = Instant::now().duration_since(start);

        // With 20% jitter on 100ms, expect 80-120ms range
        assert!(elapsed >= Duration::from_millis(70));
        assert!(elapsed <= Duration::from_millis(130));
    }

    /// Validates `IntervalConfig::new` behavior for the interval config
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.duration` equals `Duration::from_secs(1)`.
    /// - Confirms `config.jitter` equals `Some(0.5)`.
    /// - Ensures `config.skip_missed_ticks` evaluates to true.
    #[test]
    fn test_interval_config() {
        let config =
            IntervalConfig::new(Duration::from_secs(1)).with_jitter(0.5).skip_missed_ticks(true);

        assert_eq!(config.duration, Duration::from_secs(1));
        assert_eq!(config.jitter, Some(0.5));
        assert!(config.skip_missed_ticks);
    }

    /// Validates `IntervalConfig::new` behavior for the jitter clamping
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.jitter` equals `Some(1.0)`.
    /// - Confirms `config.jitter` equals `Some(0.0)`.
    #[test]
    fn test_jitter_clamping() {
        let config = IntervalConfig::new(Duration::from_secs(1)).with_jitter(1.5);
        assert_eq!(config.jitter, Some(1.0));

        let config = IntervalConfig::new(Duration::from_secs(1)).with_jitter(-0.5);
        assert_eq!(config.jitter, Some(0.0));
    }
}
