//! # PulseArc Infrastructure
//!
//! Infrastructure implementations of core domain ports.
//!
//! This crate contains:
//! - Database implementations (SQLite/SQLCipher)
//! - HTTP client implementations
//! - Platform-specific code (macOS Accessibility API)
//! - External service integrations (Calendar, SAP)
//!
//! ## Architecture
//! - Implements traits defined in `pulsearc-core`
//! - Depends on `pulsearc-common` and `pulsearc-core`
//! - Contains all "impure" code (I/O, platform APIs)

pub mod database;
pub mod http;
pub mod instance_lock;
pub mod integrations;
pub mod key_manager;
pub mod platform;

// Re-export commonly used items
pub use database::*;
pub use http::*;
pub use instance_lock::*;
pub use integrations::*;
pub use key_manager::*;
pub use platform::*;
