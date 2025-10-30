//! # PulseArc API
//!
//! Tauri application layer - commands and main entry point.
//!
//! This crate contains:
//! - Tauri commands (frontend → backend bridge)
//! - Application context (dependency injection)
//! - Main entry point and setup
//!
//! ## Architecture
//! - Depends on `shared`, `core`, and `infra`
//! - Wires up the hexagonal architecture
//! - Provides Tauri commands for the frontend

pub mod commands;
pub mod context;

// Re-export for convenience
pub use commands::*;
pub use context::*;
