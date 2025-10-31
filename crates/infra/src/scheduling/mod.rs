//! Scheduling infrastructure for automated task execution
//!
//! This module provides cron-based schedulers for various background tasks:
//! - Block generation scheduling (inference blocks)
//! - Classification scheduling (periodic classification jobs)
//! - Sync scheduling (API outbox processing - always compiled)
//! - SAP scheduler (batch forwarding - feature-gated)
//! - Calendar scheduler (calendar sync - feature-gated)
//!
//! All schedulers follow CLAUDE.md runtime rules:
//! - Explicit lifecycle management (start/stop)
//! - Join handles for spawned tasks
//! - Cancellation token support
//! - Timeout wrapping on all async operations
//! - Structured tracing with PerformanceMetrics integration

pub mod block_scheduler;
pub mod classification_scheduler;
pub mod error;
pub mod sync_scheduler;

#[cfg(feature = "sap")]
pub mod sap_scheduler;

#[cfg(feature = "calendar")]
pub mod calendar_scheduler;

pub use block_scheduler::{BlockJob, BlockScheduler, BlockSchedulerConfig};
#[cfg(feature = "calendar")]
pub use calendar_scheduler::{CalendarScheduler, CalendarSchedulerConfig};
pub use classification_scheduler::{
    ClassificationJob, ClassificationScheduler, ClassificationSchedulerConfig,
};
pub use error::{SchedulerError, SchedulerResult};
#[cfg(feature = "sap")]
pub use sap_scheduler::{SapScheduler, SapSchedulerConfig};
pub use sync_scheduler::{SyncScheduler, SyncSchedulerConfig};
