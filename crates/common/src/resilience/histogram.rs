//! Histogram for tracking latency distributions
//!
//! Provides lightweight latency tracking using logarithmic buckets.
//! Useful for measuring operation durations and identifying performance issues.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

/// Histogram for tracking latency measurements
///
/// Uses logarithmic buckets to efficiently track latency distribution
/// across a wide range of durations (microseconds to seconds).
///
/// # Examples
///
/// ```rust
/// use std::time::Instant;
///
/// use pulsearc_common::resilience::Histogram;
///
/// let histogram = Histogram::new();
///
/// let start = Instant::now();
/// // ... do work ...
/// histogram.record(start.elapsed());
///
/// let stats = histogram.snapshot();
/// println!("p50: {:?}", stats.percentile(0.5));
/// println!("p99: {:?}", stats.percentile(0.99));
/// ```
#[derive(Debug)]
pub struct Histogram {
    buckets: Arc<[AtomicU64; Self::NUM_BUCKETS]>,
    count: Arc<AtomicU64>,
    sum_micros: Arc<AtomicU64>,
    min_micros: Arc<AtomicU64>,
    max_micros: Arc<AtomicU64>,
}

impl Histogram {
    /// Number of buckets (covers 1µs to ~1 hour with logarithmic spacing)
    const NUM_BUCKETS: usize = 50;
    const MIN_MICROS: u64 = 1;
    const MAX_MICROS: u64 = 3_600_000_000; // 1 hour

    /// Create a new histogram
    pub fn new() -> Self {
        // Initialize array of atomic buckets
        let buckets: [AtomicU64; Self::NUM_BUCKETS] = std::array::from_fn(|_| AtomicU64::new(0));

        Self {
            buckets: Arc::new(buckets),
            count: Arc::new(AtomicU64::new(0)),
            sum_micros: Arc::new(AtomicU64::new(0)),
            min_micros: Arc::new(AtomicU64::new(u64::MAX)),
            max_micros: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a duration measurement
    pub fn record(&self, duration: Duration) {
        let micros_u128 = duration.as_micros();
        let mut micros = micros_u128.min(Self::MAX_MICROS as u128) as u64;
        let bucket = Self::duration_to_bucket(micros);

        self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        Self::saturating_fetch_add(&self.sum_micros, micros);

        if micros == 0 {
            micros = Self::MIN_MICROS;
        }

        // Update min
        let mut current_min = self.min_micros.load(Ordering::Acquire);
        while micros < current_min {
            match self.min_micros.compare_exchange_weak(
                current_min,
                micros,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current_min = actual,
            }
        }

        // Update max
        let mut current_max = self.max_micros.load(Ordering::Acquire);
        while micros > current_max {
            match self.max_micros.compare_exchange_weak(
                current_max,
                micros,
                Ordering::Release,
                Ordering::Acquire,
            ) {
                Ok(_) => break,
                Err(actual) => current_max = actual,
            }
        }
    }

    /// Record the elapsed time from a given instant
    pub fn record_since(&self, start: Instant) {
        self.record(start.elapsed());
    }

    /// Convert duration in microseconds to bucket index (logarithmic)
    fn duration_to_bucket(micros: u64) -> usize {
        if micros == 0 {
            return 0;
        }

        let clamped = micros.clamp(Self::MIN_MICROS, Self::MAX_MICROS);
        let (_ratio, ratio_ln) = Self::bucket_scalars();
        let numerator = (clamped as f64 / Self::MIN_MICROS as f64).ln();
        let bucket = (numerator / ratio_ln).floor() as usize;

        bucket.min(Self::NUM_BUCKETS - 1)
    }

    /// Convert bucket index back to approximate duration (midpoint)
    fn bucket_to_micros(bucket: usize) -> u64 {
        if bucket == 0 {
            return Self::MIN_MICROS;
        }

        let (ratio, _) = Self::bucket_scalars();
        let value = (Self::MIN_MICROS as f64) * ratio.powf(bucket as f64 + 0.5); // midpoint of bucket
        value.round() as u64
    }

    /// Get a snapshot of current statistics
    pub fn snapshot(&self) -> HistogramSnapshot {
        let count = self.count.load(Ordering::Acquire);
        let sum_micros = self.sum_micros.load(Ordering::Acquire);
        let min_micros = self.min_micros.load(Ordering::Acquire);
        let max_micros = self.max_micros.load(Ordering::Acquire);

        let mut buckets = [0u64; Self::NUM_BUCKETS];
        for (i, bucket) in self.buckets.iter().enumerate() {
            buckets[i] = bucket.load(Ordering::Acquire);
        }

        HistogramSnapshot {
            buckets,
            count,
            sum_micros,
            min_micros: if min_micros == u64::MAX { 0 } else { min_micros },
            max_micros,
        }
    }

    /// Reset all measurements
    pub fn reset(&self) {
        for bucket in self.buckets.iter() {
            bucket.store(0, Ordering::Release);
        }
        self.count.store(0, Ordering::Release);
        self.sum_micros.store(0, Ordering::Release);
        self.min_micros.store(u64::MAX, Ordering::Release);
        self.max_micros.store(0, Ordering::Release);
    }

    /// Get the number of recorded measurements
    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Acquire)
    }

    fn bucket_scalars() -> (f64, f64) {
        static SCALARS: OnceLock<(f64, f64)> = OnceLock::new();
        *SCALARS.get_or_init(|| {
            let ratio = (Self::MAX_MICROS as f64 / Self::MIN_MICROS as f64)
                .powf(1.0 / (Self::NUM_BUCKETS as f64 - 1.0));
            let ratio_ln = ratio.ln();
            (ratio, ratio_ln)
        })
    }

    fn saturating_fetch_add(target: &AtomicU64, value: u64) {
        let mut current = target.load(Ordering::Relaxed);
        loop {
            let new_value = current.saturating_add(value);
            match target.compare_exchange_weak(
                current,
                new_value,
                Ordering::Release,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => current = actual,
            }
        }
    }
}

impl Clone for Histogram {
    fn clone(&self) -> Self {
        Self {
            buckets: Arc::clone(&self.buckets),
            count: Arc::clone(&self.count),
            sum_micros: Arc::clone(&self.sum_micros),
            min_micros: Arc::clone(&self.min_micros),
            max_micros: Arc::clone(&self.max_micros),
        }
    }
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

/// Immutable snapshot of histogram statistics
#[derive(Debug, Clone)]
pub struct HistogramSnapshot {
    buckets: [u64; Histogram::NUM_BUCKETS],
    count: u64,
    sum_micros: u64,
    min_micros: u64,
    max_micros: u64,
}

impl HistogramSnapshot {
    /// Get the total number of measurements
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Get the mean (average) latency
    pub fn mean(&self) -> Option<Duration> {
        if self.count == 0 {
            return None;
        }
        Some(Duration::from_micros(self.sum_micros / self.count))
    }

    /// Get the minimum latency
    pub fn min(&self) -> Option<Duration> {
        if self.count == 0 {
            return None;
        }
        Some(Duration::from_micros(self.min_micros))
    }

    /// Get the maximum latency
    pub fn max(&self) -> Option<Duration> {
        if self.count == 0 {
            return None;
        }
        Some(Duration::from_micros(self.max_micros))
    }

    /// Calculate a percentile (0.0 to 1.0)
    ///
    /// Returns the latency value at which the given percentage of measurements
    /// fall below.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use pulsearc_common::resilience::Histogram;
    /// # use std::time::Duration;
    /// let histogram = Histogram::new();
    /// histogram.record(Duration::from_millis(10));
    /// histogram.record(Duration::from_millis(20));
    /// histogram.record(Duration::from_millis(30));
    ///
    /// let snapshot = histogram.snapshot();
    /// let p50 = snapshot.percentile(0.5); // Median
    /// let p99 = snapshot.percentile(0.99); // 99th percentile
    /// ```
    pub fn percentile(&self, p: f64) -> Option<Duration> {
        if self.count == 0 || p < 0.0 || p > 1.0 {
            return None;
        }

        let rank = if p == 1.0 {
            self.count - 1
        } else {
            ((self.count as f64 - 1.0) * p).ceil().max(0.0) as u64
        };

        let mut accumulated = 0u64;

        for (bucket_idx, &count) in self.buckets.iter().enumerate() {
            accumulated += count;
            if accumulated > rank {
                let micros = Histogram::bucket_to_micros(bucket_idx);
                return Some(Duration::from_micros(micros));
            }
        }

        // Fallback to max if we didn't find it (shouldn't happen)
        self.max()
    }

    /// Get the standard deviation
    pub fn stddev(&self) -> Option<Duration> {
        if self.count < 2 {
            return None;
        }

        let mean = (self.sum_micros / self.count) as f64;
        let mut variance_sum = 0.0f64;

        for (bucket_idx, &count) in self.buckets.iter().enumerate() {
            if count == 0 {
                continue;
            }

            let bucket_value = Histogram::bucket_to_micros(bucket_idx) as f64;
            let diff = bucket_value - mean;
            variance_sum += diff * diff * count as f64;
        }

        let variance = variance_sum / (self.count - 1) as f64;
        let stddev_micros = variance.sqrt() as u64;

        Some(Duration::from_micros(stddev_micros))
    }

    /// Get common percentiles (p50, p95, p99, p999)
    pub fn percentiles(&self) -> Percentiles {
        Percentiles {
            p50: self.percentile(0.50),
            p95: self.percentile(0.95),
            p99: self.percentile(0.99),
            p999: self.percentile(0.999),
        }
    }

    /// Format as a human-readable summary
    pub fn summary(&self) -> String {
        if self.count == 0 {
            return "No measurements recorded".to_string();
        }

        let mean = self.mean().map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string());
        let min = self.min().map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string());
        let max = self.max().map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string());
        let p50 =
            self.percentile(0.5).map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string());
        let p99 = self
            .percentile(0.99)
            .map(|d| format!("{:.2?}", d))
            .unwrap_or_else(|| "N/A".to_string());

        format!(
            "count={}, mean={}, min={}, max={}, p50={}, p99={}",
            self.count, mean, min, max, p50, p99
        )
    }
}

/// Common percentile values
#[derive(Debug, Clone)]
pub struct Percentiles {
    pub p50: Option<Duration>,
    pub p95: Option<Duration>,
    pub p99: Option<Duration>,
    pub p999: Option<Duration>,
}

impl Percentiles {
    /// Format as a human-readable string
    pub fn format(&self) -> String {
        format!(
            "p50={:?}, p95={:?}, p99={:?}, p999={:?}",
            self.p50.map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string()),
            self.p95.map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string()),
            self.p99.map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string()),
            self.p999.map(|d| format!("{:.2?}", d)).unwrap_or_else(|| "N/A".to_string()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_histogram_basic() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        histogram.record(Duration::from_millis(20));
        histogram.record(Duration::from_millis(30));

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 3);

        let mean = snapshot.mean().unwrap();
        assert!(mean >= Duration::from_millis(15) && mean <= Duration::from_millis(25));
    }

    #[test]
    fn test_histogram_percentiles() {
        let histogram = Histogram::new();

        // Record 100 measurements from 1ms to 100ms
        for i in 1..=100 {
            histogram.record(Duration::from_millis(i));
        }

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 100);

        // p50 should be around 50ms
        let p50 = snapshot.percentile(0.5).unwrap();
        assert!(p50 >= Duration::from_millis(40) && p50 <= Duration::from_millis(70));

        // p99 should be around 99ms
        let p99 = snapshot.percentile(0.99).unwrap();
        assert!(p99 >= Duration::from_millis(90));
    }

    #[test]
    fn test_histogram_min_max() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(5));
        histogram.record(Duration::from_millis(50));
        histogram.record(Duration::from_millis(25));

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.min(), Some(Duration::from_micros(5000)));
        assert_eq!(snapshot.max(), Some(Duration::from_micros(50000)));
    }

    #[test]
    fn test_histogram_record_since() {
        let histogram = Histogram::new();
        let start = Instant::now();

        // Simulate some work
        std::thread::sleep(Duration::from_millis(10));

        histogram.record_since(start);

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 1);
        assert!(snapshot.min().unwrap() >= Duration::from_millis(10));
    }

    #[test]
    fn test_histogram_reset() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        assert_eq!(histogram.count(), 1);

        histogram.reset();
        assert_eq!(histogram.count(), 0);

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 0);
        assert_eq!(snapshot.mean(), None);
    }

    #[test]
    fn test_histogram_clone() {
        let histogram1 = Histogram::new();
        histogram1.record(Duration::from_millis(10));

        let histogram2 = histogram1.clone();
        histogram2.record(Duration::from_millis(20));

        // Both should see both measurements (shared Arc)
        assert_eq!(histogram1.count(), 2);
        assert_eq!(histogram2.count(), 2);
    }

    #[test]
    fn test_empty_histogram() {
        let histogram = Histogram::new();
        let snapshot = histogram.snapshot();

        assert_eq!(snapshot.count(), 0);
        assert_eq!(snapshot.mean(), None);
        assert_eq!(snapshot.min(), None);
        assert_eq!(snapshot.max(), None);
        assert_eq!(snapshot.percentile(0.5), None);
    }

    #[test]
    fn test_percentiles_struct() {
        let histogram = Histogram::new();

        for i in 1..=1000 {
            histogram.record(Duration::from_micros(i));
        }

        let snapshot = histogram.snapshot();
        let percentiles = snapshot.percentiles();

        assert!(percentiles.p50.is_some());
        assert!(percentiles.p95.is_some());
        assert!(percentiles.p99.is_some());
        assert!(percentiles.p999.is_some());
    }

    #[test]
    fn test_summary_format() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        histogram.record(Duration::from_millis(20));

        let snapshot = histogram.snapshot();
        let summary = snapshot.summary();

        assert!(summary.contains("count=2"));
        assert!(summary.contains("mean="));
        assert!(summary.contains("p50="));
    }

    #[test]
    fn test_histogram_large_durations() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_secs(30));
        histogram.record(Duration::from_secs(45));

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 2);
        assert!(snapshot.max().unwrap() >= Duration::from_secs(30));
    }

    #[test]
    fn test_percentile_single_sample() {
        let histogram = Histogram::new();
        histogram.record(Duration::from_millis(150));

        let snapshot = histogram.snapshot();

        let p50 = snapshot.percentile(0.5).unwrap();
        assert!(p50 >= Duration::from_millis(100));
        let p99 = snapshot.percentile(0.99).unwrap();
        assert!(p99 >= Duration::from_millis(100));
    }
}
