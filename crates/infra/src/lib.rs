//! # PulseArc Infrastructure
//!
//! Infrastructure implementations of core domain ports.
//!
//! This crate contains:
//! - Configuration loading
//! - Database implementations (SQLite/SQLCipher)
//! - HTTP client implementations
//! - Platform-specific code (macOS Accessibility API)
//! - External service integrations (Calendar, SAP, API)
//! - Background services (schedulers, sync, cleanup)
//!
//! ## Architecture
//! - Implements traits defined in `pulsearc-core`
//! - Depends on `pulsearc-common` and `pulsearc-core`
//! - Contains all "impure" code (I/O, platform APIs)

pub mod api;
pub mod config;
pub mod database;
pub mod errors;
pub mod http;
pub mod instance_lock;
pub mod integrations;
pub mod key_manager;
pub mod mdm;
pub mod observability;
pub mod platform;
pub mod scheduling;
pub mod sync;
// Re-export commonly used items
pub use api::{ApiClient, ApiCommands, ApiForwarder, ApiScheduler};
pub use config::*;
pub use database::*;
pub use errors::*;
pub use http::{HttpClient, HttpClientBuilder};
pub use instance_lock::*;
#[cfg(feature = "calendar")]
pub use integrations::calendar;
#[cfg(feature = "sap")]
pub use integrations::sap;
pub use key_manager::*;
pub use mdm::*;
pub use platform::*;
pub use scheduling::{
    BlockJob, BlockScheduler, BlockSchedulerConfig, ClassificationJob, ClassificationScheduler,
    ClassificationSchedulerConfig, SchedulerError, SchedulerResult, SyncScheduler,
    SyncSchedulerConfig,
};
#[cfg(feature = "calendar")]
pub use scheduling::{CalendarScheduler, CalendarSchedulerConfig};
#[cfg(feature = "sap")]
pub use scheduling::{SapScheduler, SapSchedulerConfig};
pub use sync::{CleanupService, CostTracker, NeonClient, OutboxWorker, OutboxWorkerConfig};
