//! # PulseArc Shared
//!
//! Common types, errors, and utilities shared across all crates.
//!
//! This crate contains:
//! - Common data types (ActivityContext, TimeEntry, etc.)
//! - Error types and Result definitions
//! - Configuration structures
//! - Shared constants and utilities
//!
//! ## Architecture
//! - No dependencies on other PulseArc crates
//! - Only external dependencies allowed
//! - Pure data structures and utilities

pub mod config;
pub mod errors;
pub mod types;

// Re-export commonly used items
pub use config::*;
pub use errors::*;
pub use types::*;
