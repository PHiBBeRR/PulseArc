//! Scheduling infrastructure for automated task execution
//!
//! This module provides cron-based schedulers for various background tasks:
//! - Block generation scheduling (inference blocks)
//! - Classification scheduling (periodic classification jobs)
//! - Integration schedulers (SAP, Calendar sync - feature-gated)
//!
//! All schedulers follow CLAUDE.md runtime rules:
//! - Explicit lifecycle management (start/stop)
//! - Join handles for spawned tasks
//! - Cancellation token support
//! - Timeout wrapping on all async operations
//! - Structured tracing with PerformanceMetrics integration

pub mod block_scheduler;

pub use block_scheduler::BlockScheduler;
