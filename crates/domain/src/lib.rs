//! # PulseArc Domain
//!
//! Business domain types and models for PulseArc.
//!
//! This crate contains:
//! - Domain data types (ActivityContext, TimeEntry, etc.)
//! - Domain error types and Result definitions
//! - Configuration structures
//! - Domain constants and models
//!
//! ## Architecture
//! - No dependencies on other PulseArc crates
//! - Only external dependencies allowed
//! - Pure domain models and data structures

pub mod config;
pub mod errors;
pub mod types;

// Re-export commonly used items
pub use config::*;
pub use errors::*;
pub use types::*;
