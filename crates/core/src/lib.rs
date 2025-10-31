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

pub mod classification;
pub mod sync;
pub mod tracking;
pub mod utils;

// Feature-gated integration modules
#[cfg(feature = "calendar")]
pub mod calendar_ports;

#[cfg(feature = "sap")]
pub mod sap_ports;

// Re-export specific items to avoid ambiguity
pub use classification::ports::{BlockRepository, Classifier, ProjectMatcher, TimeEntryRepository};
pub use classification::ClassificationService;
pub use sync::ports::OutboxQueue;
pub use tracking::ports::{
    ActivityEnricher, ActivityProvider, ActivityRepository, CalendarEventRepository,
    SegmentRepository, SnapshotRepository,
};
pub use tracking::TrackingService;

// Re-export utilities
pub use utils::patterns;
