//! Compliance primitives for enterprise applications
//!
//! Generic compliance infrastructure including audit logging, configuration
//! management, and feature flag systems.

pub mod audit;
pub mod config;
pub mod feature_flags;

// Re-export commonly used types
pub use audit::{AuditContext, AuditEvent, AuditSeverity, GlobalAuditLogger};
pub use config::{ConfigManager, RemoteConfig};
pub use feature_flags::{FeatureFlag, FeatureFlagManager};
