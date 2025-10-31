//! Observability infrastructure for metrics, logging, and tracing
//!
//! This module provides production-ready metrics collection with:
//! - Thread-safe counters, gauges, and histograms
//! - Percentile calculations (P50/P95/P99)
//! - Datadog DogStatsD integration
//! - Poison-safe mutex handling
//! - Label cardinality enforcement
//!
//! ## Design Principles
//!
//! 1. **Poison Recovery**: All mutex locks use explicit poison recovery
//!    pattern: ```rust let guard = match mutex.lock() { Ok(guard) => guard,
//!    Err(poison_err) => { tracing::warn!("Mutex poisoned, recovering");
//!    poison_err.into_inner() } }; ```
//!
//! 2. **Future-Proof Returns**: All record methods return `MetricsResult<()>`
//!    for future extensibility (cardinality limits, quotas, validation), but
//!    currently always succeed (return `Ok(())`).
//!
//! 3. **Ring Buffers**: VecDeque for O(1) eviction (not Vec with remove(0))
//!
//! 4. **Memory Ordering**: SeqCst for derived metrics (rates, percentiles),
//!    Acquire/Release for independent counters
//!
//! ## Error Handling
//!
//! ```rust
//! use pulsearc_infra::observability::metrics::PerformanceMetrics;
//!
//! let metrics = PerformanceMetrics::new();
//!
//! // Recommended: Handle future errors gracefully
//! if let Err(e) = metrics.record_call() {
//!     tracing::warn!("Failed to record metric: {}", e);
//!     // Continue execution, metric dropped
//! }
//! ```

pub mod exporters;
pub mod metrics;

use std::io;

/// Metrics error type
///
/// All metrics recording methods return `MetricsResult<()>` for consistency
/// and future extensibility, but **currently always succeed** (return
/// `Ok(())`).
///
/// ## Current Behavior
/// - **Poison recovery:** Transparent (logs warning, continues with recovered
///   data)
/// - **Ring buffer overflow:** Automatic eviction (FIFO, no error)
/// - **Label cardinality:** LRU eviction (logs warning, no error)
///
/// ## Future Extensions
/// The `MetricsResult<()>` return type allows future additions without API
/// breakage:
/// - Hard cardinality limits (return `CardinalityExceeded` instead of evicting)
/// - Quota enforcement (return `QuotaExceeded` if too many metrics recorded)
/// - Validation (return `InvalidMetricName` for malformed names)
#[derive(Debug, thiserror::Error)]
pub enum MetricsError {
    /// Empty data set - cannot calculate aggregate metric
    #[error("Empty data: cannot calculate {metric}")]
    EmptyData {
        /// Metric name that failed (e.g., "P95", "P50", "average")
        metric: &'static str,
    },

    /// Label cardinality limit exceeded
    ///
    /// Currently only used as warning (LRU evicts oldest), but could become
    /// a hard error in future versions.
    #[error("Label cardinality exceeded for metric '{metric}': {count} > {limit}")]
    CardinalityExceeded {
        /// Metric name
        metric: String,
        /// Current unique label combination count
        count: usize,
        /// Configured limit
        limit: usize,
    },

    /// Network send failed (Datadog UDP)
    #[error("Network send failed: {source}")]
    SendFailed {
        /// Underlying IO error
        #[from]
        source: io::Error,
    },
}

/// Result type for metrics operations
pub type MetricsResult<T> = Result<T, MetricsError>;
