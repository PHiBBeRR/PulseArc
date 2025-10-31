//! Secure Hashing Module - Portable Core
//!
//! This module provides portable, domain-independent hash functionality
//! for privacy-preserving domain logging.

pub mod config;
pub mod error;
pub mod hasher;
pub mod metrics;

// Re-export commonly used types
pub use config::{HashAlgorithm, HashConfig};
pub use error::{HashError, HashResult};
pub use hasher::SecureHasher;
pub use metrics::{
    ComplianceMetrics, HashMetricsCollector, HashMetricsSnapshot, HashPerformanceMetrics,
    PerformanceSummary, SaltMetrics, SecurityMetrics,
};
