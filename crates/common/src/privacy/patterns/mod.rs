//! PII Pattern Detection - Portable Core
//!
//! This module provides portable, domain-independent PII pattern detection
//! functionality including pattern matching, configuration, and metrics.

pub mod config;
pub mod core;
pub mod error;
pub mod metrics;
pub mod types;

// Re-export commonly used types
pub use core::PatternMatcher;

pub use config::{ModelValidationConfig, PiiDetectionConfig, SecurityConfig};
pub use error::{PiiError, PiiResult};
pub use metrics::{
    ComplianceSnapshot, DetailedPerformanceReport, DetectionOperationParams, MetricsSnapshot,
    OperationalSnapshot, PerformanceSnapshot, PiiMetricsCollector, QualitySnapshot,
};
pub use types::{
    AnalysisContext, ComplianceFramework, ComplianceStatus, ComplianceViolation, ConfidenceScore,
    DetectionMethod, DetectionResult, MlModelInfo, PatternConfig, PerformanceMetrics,
    PiiAuditEntry, PiiEntity, PiiOperationType, PiiStatistics, PiiType, QualityMetrics,
    RedactionStrategy, SensitivityLevel, ViolationSeverity,
};
