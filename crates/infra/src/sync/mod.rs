//! Sync infrastructure for PulseArc
//!
//! This module provides background synchronization services:
//! - NeonClient: Postgres database sync to remote
//! - CostTracker: API usage tracking and cost monitoring
//! - CleanupService: Periodic cleanup of stale data
//! - OutboxWorker: Batch processing and forwarding of outbox entries
//!
//! All modules follow CLAUDE.md runtime rules with explicit lifecycle
//! management, join handle tracking, and cancellation support.

pub mod cleanup;
pub mod cost_tracker;
mod errors;
pub mod neon_client;
pub mod outbox_worker;

pub use cleanup::{CleanupConfig, CleanupService, CleanupStats};
pub use cost_tracker::{CostMetrics, CostRateConfig, CostTracker, DailyCost};
pub use errors::SyncError;
pub use neon_client::{NeonClient, NeonClientConfig};
pub use outbox_worker::{OutboxWorker, OutboxWorkerConfig};
