//! Configuration loading and management
//!
//! This module provides utilities for loading application configuration
//! from environment variables and files.

pub mod loader;

// Re-export commonly used items
pub use loader::{load, load_from_env, load_from_file, probe_config_paths};
