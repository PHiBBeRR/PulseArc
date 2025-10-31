//! Observability primitives - monitoring, metrics, and error handling
//!
//! This module consolidates all observability concerns:
//! - Error types and handling (errors/)
//! - Performance metrics and tracking (metrics/)
//! - Trait abstractions for audit, metrics, and tracing (traits/)
//!
//! Centralizing these concerns makes it easier to add logging, tracing,
//! and other observability features in the future.

pub mod errors;
pub mod metrics;
pub mod traits;

// Re-export commonly used types for convenience
pub use errors::{
    ActionHint, AiError, AppError, AppResult, ErrorCode, HttpError, MetricsError, UiError,
};
pub use metrics::{ClassificationMetrics, MetricsTracker, PerformanceMetrics};
// Re-export trait abstractions
pub use traits::{
    AuditLogEntry, AuditLogger, AuditSeverity, MetricsCollector, NoOpAuditLogger,
    NoOpMetricsCollector, NoOpTracer, TraceSpan, Tracer,
};
