//! Test data generators and sample entities
//!
//! This module provides sample data structures and batch generators for
//! testing:
//!
//! ## Sample Entities
//! - [`sample_entities`] - Sample users, projects, and configuration structures
//!   - `TestUser` - Sample user entities with batch generation
//!   - `TestProject` - Sample project entities with batch generation
//!   - `TestConfig` - Sample configuration structures
//!   - Email/URL generators for validation testing

/// Sample entities (users, projects, configs) and batch generators
pub mod sample_entities;

// Re-export all sample data types and generators for convenience
// Removed unused import: sample_entities::*
