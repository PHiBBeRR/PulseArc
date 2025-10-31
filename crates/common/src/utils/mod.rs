//! Common utility functions and helper macros
//!
//! This module provides reusable utilities including:
//! - **[`macros`]**: Utility macros for reducing boilerplate code
//! - **[`serde`]**: Serialization helpers for common data types

#[macro_use]
pub mod macros;
pub mod serde;

// Re-export commonly used items for convenience
pub use self::serde::duration_millis;
