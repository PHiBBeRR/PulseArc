//! Metrics collection modules
//!
//! Thread-safe metrics for various subsystems.

pub mod call;

// Re-export metric types for convenience
pub use call::CallMetrics;
