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
pub mod constants;
pub mod errors;
pub mod macros;
pub mod types;
pub mod utils;

// Re-export commonly used items
pub use config::*;
pub use errors::*;
pub use types::*;
// Re-export calendar parser utilities
pub use utils::calendar_parser::{parse_event_title, ParsedEventTitle};
