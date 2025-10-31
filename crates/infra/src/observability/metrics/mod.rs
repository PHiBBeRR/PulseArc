//! Metrics collection modules
//!
//! Thread-safe metrics for various subsystems.

/// Default ring buffer capacity for percentile-tracking metrics.
pub(crate) const DEFAULT_RING_BUFFER_CAPACITY: usize = 1_000;

pub mod cache;
pub mod call;
pub mod db;
pub mod fetch;
pub mod observer;
pub mod performance;

// Re-export metric types for convenience
pub use cache::CacheMetrics;
pub use call::CallMetrics;
pub use db::{DbMetrics, DbStats};
pub use fetch::FetchMetrics;
pub use observer::{ObserverMetrics, ObserverStats};
pub use performance::PerformanceMetrics;
