//! Metrics exporters
//!
//! Exporters send collected metrics to external monitoring systems.

pub mod datadog;

// Re-export exporter types for convenience
pub use datadog::{DatadogClient, DEFAULT_DATADOG_ADDR};
