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
        if self.count == 0 || !(0.0..=1.0).contains(&p) {
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

    #[test]
    fn test_concurrent_recording() {
        use std::sync::Arc;
        use std::thread;

        let histogram = Arc::new(Histogram::new());
        let mut handles = vec![];

        // Spawn 10 threads, each recording 1000 measurements
        for thread_id in 0..10 {
            let hist = Arc::clone(&histogram);
            handles.push(thread::spawn(move || {
                for i in 0..1000 {
                    hist.record(Duration::from_micros((thread_id * 100 + i) as u64));
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 10_000, "Should record all measurements from all threads");
        assert!(snapshot.min().is_some());
        assert!(snapshot.max().is_some());
        assert!(snapshot.mean().is_some());
    }

    /// Test standard deviation with uniform values
    #[test]
    fn test_stddev_uniform_values() {
        let histogram = Histogram::new();

        // Record 100 identical values
        for _ in 0..100 {
            histogram.record(Duration::from_millis(100));
        }

        let snapshot = histogram.snapshot();
        let stddev = snapshot.stddev().unwrap();

        // With identical values, stddev should be very low (accounting for bucket
        // approximation)
        assert!(
            stddev < Duration::from_millis(20),
            "Expected low stddev for uniform values, got {:?}",
            stddev
        );
    }

    /// Test standard deviation with high variance
    #[test]
    fn test_stddev_with_variance() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        histogram.record(Duration::from_millis(50));
        histogram.record(Duration::from_millis(90));

        let snapshot = histogram.snapshot();
        let stddev = snapshot.stddev().unwrap();

        // Should have significant variance
        assert!(
            stddev > Duration::from_millis(10),
            "Expected high stddev for varied values, got {:?}",
            stddev
        );
    }

    /// Test standard deviation with single sample (should return None)
    #[test]
    fn test_stddev_single_sample() {
        let histogram = Histogram::new();
        histogram.record(Duration::from_millis(100));

        let snapshot = histogram.snapshot();
        // stddev requires at least 2 samples
        assert_eq!(snapshot.stddev(), None, "stddev should be None with single sample");
    }

    /// Test standard deviation with empty histogram
    #[test]
    fn test_stddev_empty() {
        let histogram = Histogram::new();
        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.stddev(), None, "stddev should be None with no samples");
    }

    /// Test bucket boundaries - minimum value
    #[test]
    fn test_bucket_boundary_min() {
        let histogram = Histogram::new();

        // Test minimum boundary (1µs)
        histogram.record(Duration::from_micros(1));

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 1);
        let min = snapshot.min().unwrap();
        assert!(min >= Duration::from_micros(1));
    }

    /// Test bucket boundaries - maximum value
    #[test]
    fn test_bucket_boundary_max() {
        let histogram = Histogram::new();

        // Test maximum boundary (1 hour)
        histogram.record(Duration::from_secs(3600));

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 1);
        assert!(snapshot.max().is_some());
    }

    /// Test zero duration handling
    #[test]
    fn test_zero_duration() {
        let histogram = Histogram::new();

        // Test zero duration
        histogram.record(Duration::ZERO);

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 1);
        // Zero duration should be mapped to MIN_MICROS (1µs)
        let min = snapshot.min().unwrap();
        assert!(min >= Duration::from_micros(1));
    }

    /// Test duration clamping beyond maximum
    #[test]
    fn test_duration_clamping() {
        let histogram = Histogram::new();

        // Record duration beyond MAX_MICROS (1 hour)
        histogram.record(Duration::from_secs(10_000)); // Way beyond 1 hour

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 1);

        // Should be clamped to MAX_MICROS
        let max = snapshot.max().unwrap();
        assert!(max <= Duration::from_secs(3700), "Duration should be clamped, got {:?}", max);
    }

    /// Test mixed boundary values
    #[test]
    fn test_mixed_boundaries() {
        let histogram = Histogram::new();

        histogram.record(Duration::ZERO);
        histogram.record(Duration::from_micros(1));
        histogram.record(Duration::from_secs(3600));
        histogram.record(Duration::from_millis(100));

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 4);
        assert!(snapshot.min().is_some());
        assert!(snapshot.max().is_some());
        assert!(snapshot.mean().is_some());
    }

    /// Test percentile boundary values
    #[test]
    fn test_percentile_boundaries() {
        let histogram = Histogram::new();

        for i in 1..=100 {
            histogram.record(Duration::from_millis(i));
        }

        let snapshot = histogram.snapshot();

        // Test boundary percentiles
        assert!(snapshot.percentile(0.0).is_some(), "p0 should exist");
        assert!(snapshot.percentile(1.0).is_some(), "p100 should exist");

        // Test invalid percentiles
        assert_eq!(snapshot.percentile(-0.1), None, "Negative percentile should be None");
        assert_eq!(snapshot.percentile(1.1), None, "Percentile > 1.0 should be None");
    }

    /// Test percentile ordering (monotonicity)
    #[test]
    fn test_percentile_ordering() {
        let histogram = Histogram::new();

        for i in 1..=1000 {
            histogram.record(Duration::from_micros(i));
        }

        let snapshot = histogram.snapshot();

        let p25 = snapshot.percentile(0.25).unwrap();
        let p50 = snapshot.percentile(0.50).unwrap();
        let p75 = snapshot.percentile(0.75).unwrap();
        let p95 = snapshot.percentile(0.95).unwrap();
        let p99 = snapshot.percentile(0.99).unwrap();

        // Percentiles should be monotonically increasing
        assert!(p25 <= p50, "p25 should be <= p50");
        assert!(p50 <= p75, "p50 should be <= p75");
        assert!(p75 <= p95, "p75 should be <= p95");
        assert!(p95 <= p99, "p95 should be <= p99");
    }

    /// Test snapshot immutability
    #[test]
    fn test_snapshot_immutability() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        histogram.record(Duration::from_millis(20));

        let snapshot1 = histogram.snapshot();
        assert_eq!(snapshot1.count(), 2);

        // Record more data
        histogram.record(Duration::from_millis(30));
        histogram.record(Duration::from_millis(40));

        // Original snapshot should be unchanged
        assert_eq!(
            snapshot1.count(),
            2,
            "Snapshot should be immutable and not reflect new recordings"
        );

        let snapshot2 = histogram.snapshot();
        assert_eq!(snapshot2.count(), 4, "New snapshot should have updated count");
    }

    /// Test snapshot statistics don't change
    #[test]
    fn test_snapshot_stats_immutable() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        histogram.record(Duration::from_millis(20));

        let snapshot = histogram.snapshot();
        let original_mean = snapshot.mean();
        let original_min = snapshot.min();
        let original_max = snapshot.max();

        // Record very different values
        histogram.record(Duration::from_secs(1));

        // Snapshot stats should be unchanged
        assert_eq!(snapshot.mean(), original_mean);
        assert_eq!(snapshot.min(), original_min);
        assert_eq!(snapshot.max(), original_max);
    }

    /// Test bucket conversion accuracy
    #[test]
    fn test_bucket_conversion_accuracy() {
        // Test that bucket conversion is reasonably accurate
        for micros in [1, 10, 100, 1_000, 10_000, 100_000, 1_000_000] {
            let bucket = Histogram::duration_to_bucket(micros);
            let recovered = Histogram::bucket_to_micros(bucket);

            // Should be within same order of magnitude (logarithmic bucketing)
            let ratio = recovered as f64 / micros as f64;
            assert!(
                (0.5..=2.0).contains(&ratio),
                "Bucket conversion too inaccurate for {}µs: got bucket {}, recovered {}µs (ratio: {:.2})",
                micros,
                bucket,
                recovered,
                ratio
            );
        }
    }

    /// Test bucket indices are in valid range
    #[test]
    fn test_bucket_indices_valid() {
        for micros in [0, 1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 1_000_000_000] {
            let bucket = Histogram::duration_to_bucket(micros);
            assert!(
                bucket < Histogram::NUM_BUCKETS,
                "Bucket index {} out of range for {}µs",
                bucket,
                micros
            );
        }
    }

    /// Test high volume recording
    #[test]
    fn test_high_volume_recording() {
        let histogram = Histogram::new();

        // Record 100,000 measurements (reduced from 1M for test speed)
        for i in 0..100_000 {
            histogram.record(Duration::from_micros(i % 10_000));
        }

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 100_000);

        // Should still compute percentiles efficiently
        let p99 = snapshot.percentile(0.99);
        assert!(p99.is_some(), "Should compute p99 even with high volume");

        let mean = snapshot.mean();
        assert!(mean.is_some(), "Should compute mean even with high volume");
    }

    /// Test concurrent recording with high contention
    #[test]
    fn test_high_contention_concurrent() {
        use std::sync::Arc;
        use std::thread;

        let histogram = Arc::new(Histogram::new());
        let mut handles = vec![];

        // Spawn 50 threads recording the same value (high contention on min/max)
        for _ in 0..50 {
            let hist = Arc::clone(&histogram);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    hist.record(Duration::from_micros(1000));
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = histogram.snapshot();
        assert_eq!(snapshot.count(), 5000);
    }

    /// Test percentiles format output
    #[test]
    fn test_percentiles_format() {
        let histogram = Histogram::new();

        for i in 1..=1000 {
            histogram.record(Duration::from_micros(i));
        }

        let snapshot = histogram.snapshot();
        let percentiles = snapshot.percentiles();
        let formatted = percentiles.format();

        assert!(formatted.contains("p50="), "Format should include p50");
        assert!(formatted.contains("p95="), "Format should include p95");
        assert!(formatted.contains("p99="), "Format should include p99");
        assert!(formatted.contains("p999="), "Format should include p999");
    }

    /// Test empty histogram summary
    #[test]
    fn test_empty_summary() {
        let histogram = Histogram::new();
        let snapshot = histogram.snapshot();
        let summary = snapshot.summary();

        assert!(summary.contains("No measurements"), "Empty histogram should have special message");
    }

    /// Test mean calculation accuracy
    #[test]
    fn test_mean_accuracy() {
        let histogram = Histogram::new();

        // Record known values
        histogram.record(Duration::from_millis(100));
        histogram.record(Duration::from_millis(200));
        histogram.record(Duration::from_millis(300));

        let snapshot = histogram.snapshot();
        let mean = snapshot.mean().unwrap();

        // Mean should be around 200ms (allowing for bucket approximation)
        assert!(
            mean >= Duration::from_millis(150) && mean <= Duration::from_millis(250),
            "Mean should be approximately 200ms, got {:?}",
            mean
        );
    }

    /// Test min/max with concurrent updates
    #[test]
    fn test_concurrent_min_max_updates() {
        use std::sync::Arc;
        use std::thread;

        let histogram = Arc::new(Histogram::new());
        let mut handles = vec![];

        // Each thread records different ranges
        for thread_id in 0..10 {
            let hist = Arc::clone(&histogram);
            handles.push(thread::spawn(move || {
                let base = (thread_id * 1000) as u64;
                for i in 0..100 {
                    hist.record(Duration::from_micros(base + i));
                }
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }

        let snapshot = histogram.snapshot();
        let min = snapshot.min().unwrap();
        let max = snapshot.max().unwrap();

        // Min should be close to 0, max should be close to 9099
        assert!(min < Duration::from_micros(100), "Min should be very small");
        assert!(max > Duration::from_micros(9000), "Max should be close to 9099");
    }

    /// Test percentile with exact rank calculation
    #[test]
    fn test_percentile_rank() {
        let histogram = Histogram::new();

        // Record exactly 10 values
        for i in 1..=10 {
            histogram.record(Duration::from_millis(i * 10));
        }

        let snapshot = histogram.snapshot();

        // p50 should be around the 5th value (50ms)
        let p50 = snapshot.percentile(0.5).unwrap();
        assert!(
            p50 >= Duration::from_millis(30) && p50 <= Duration::from_millis(70),
            "p50 should be around median, got {:?}",
            p50
        );
    }

    /// Test reset clears all statistics
    #[test]
    fn test_reset_clears_all() {
        let histogram = Histogram::new();

        histogram.record(Duration::from_millis(10));
        histogram.record(Duration::from_millis(50));
        histogram.record(Duration::from_millis(100));

        let before_snapshot = histogram.snapshot();
        assert_eq!(before_snapshot.count(), 3);
        assert!(before_snapshot.min().is_some());
        assert!(before_snapshot.max().is_some());

        histogram.reset();

        let after_snapshot = histogram.snapshot();
        assert_eq!(after_snapshot.count(), 0);
        assert_eq!(after_snapshot.min(), None);
        assert_eq!(after_snapshot.max(), None);
        assert_eq!(after_snapshot.mean(), None);
        assert_eq!(after_snapshot.percentile(0.5), None);
    }

    /// Test default trait implementation
    #[test]
    fn test_default_trait() {
        let histogram = Histogram::default();
        assert_eq!(histogram.count(), 0);

        histogram.record(Duration::from_millis(10));
        assert_eq!(histogram.count(), 1);
    }
}
