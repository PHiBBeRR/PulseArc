//! Storage primitives for encrypted databases
//!
//! This module provides generic storage infrastructure including SQLCipher
//! integration, encryption utilities, and core storage types.

pub mod config;
pub mod error;
pub mod metrics;
pub mod sqlcipher;
pub mod types;

// Re-export commonly used types
pub use config::{KeySource, StorageConfig, StorageConfigBuilder};
pub use error::{StorageError, StorageResult};
pub use metrics::StorageMetrics;
pub use sqlcipher::{
    apply_connection_pragmas, SqlCipherConnection, SqlCipherPool, SqlCipherPoolConfig,
};
pub use types::{Connection, ConnectionPool, HealthStatus, PoolMetrics, Statement, Transaction};
