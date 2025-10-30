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
//! - Only depends on `pulsearc-shared`
//! - No database, HTTP, or platform code
//! - All external dependencies via traits
//! - Pure, testable business logic

pub mod tracking;
pub mod classification;

// Re-export specific items to avoid ambiguity
pub use tracking::{
    ActivityProvider, ActivityRepository, ActivityEnricher,
    TrackingService,
};
pub use classification::{
    Classifier, TimeEntryRepository,
    ClassificationService,
};
