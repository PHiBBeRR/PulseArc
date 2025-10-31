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
pub mod tracking;

// Re-export specific items to avoid ambiguity
pub use classification::{ClassificationService, Classifier, TimeEntryRepository};
pub use tracking::{
    ActivityEnricher, ActivityProvider, ActivityRepository, SegmentRepository,
    SnapshotRepository, TrackingService,
};
