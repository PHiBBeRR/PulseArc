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

pub mod types;
pub mod errors;
pub mod config;

// Re-export commonly used items
pub use types::*;
pub use errors::*;
pub use config::*;
