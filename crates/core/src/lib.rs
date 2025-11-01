//! # PulseArc Core
//!
//! Pure business logic layer - no infrastructure dependencies.
//!
//! This crate contains:
//! - Domain models and business rules
//! - Port/adapter interfaces (traits)
//! - Use cases and services
//!
//! ## Architecture Principles
//! - Only depends on `pulsearc-common`
//! - No database, HTTP, or platform code
//! - All external dependencies via traits
//! - Pure, testable business logic

pub mod batch;
pub mod classification;
pub mod sync;
pub mod tracking;
pub mod user;
pub mod utils;

// Infrastructure ports
pub mod command_metrics_ports;
pub mod database_stats_ports;
pub mod feature_flags_ports;

// Feature-gated integration modules
#[cfg(feature = "calendar")]
pub mod calendar_ports;

#[cfg(feature = "sap")]
pub mod sap_ports;

// Re-export specific items to avoid ambiguity
pub use batch::ports::{BatchRepository, DlqRepository};
pub use classification::ports::{
    BlockRepository, Classifier, ProjectMatcher, TimeEntryRepository, WbsRepository,
};
pub use classification::ClassificationService;
pub use command_metrics_ports::{CommandMetric, CommandMetricsPort, CommandStats};
pub use database_stats_ports::DatabaseStatsPort;
pub use feature_flags_ports::{FeatureFlag, FeatureFlagEvaluation, FeatureFlagsPort};
pub use sync::ports::{IdMappingRepository, OutboxQueue, TokenUsageRepository};
pub use tracking::ports::{
    ActivityEnricher, ActivityProvider, ActivityRepository, CalendarEventRepository,
    SegmentRepository, SnapshotRepository,
};
pub use tracking::TrackingService;
pub use user::ports::UserProfileRepository;
// Re-export utilities
pub use utils::patterns;
