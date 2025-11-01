//! Metrics module for agent observability
//!
//! This module organizes performance and classification metrics into logical
//! categories.
//!
//! Ported from macos-production/src-tauri/src/observability/metrics/

pub mod classification;

// Re-export commonly used types
pub use classification::{ClassificationMetrics, MetricsTracker};

/// Performance metrics placeholder
#[derive(Debug, Default)]
pub struct PerformanceMetrics {
    // Placeholder for future expansion
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self::default()
    }
}
