//! Core storage trait definitions
//!
//! This module defines platform-agnostic traits that allow different storage
//! backends (SQLCipher, PostgreSQL, etc.) to be used interchangeably.

use std::fmt::Debug;

use rusqlite::{ToSql, Transaction as RusqliteTransaction};

use super::error::{StorageError, StorageResult};

/// Platform-agnostic connection pool trait
///
/// Implementations must provide thread-safe access to database connections.
pub trait ConnectionPool: Send + Sync + Debug {
    /// Get a connection from the pool
    ///
    /// This may block if the pool is exhausted, up to the configured timeout.
    /// Returns a boxed SqlCipherConnection (concrete type, not trait object)
    fn get_connection(&self) -> StorageResult<Box<dyn Connection>>;

    /// Check pool health
    fn health_check(&self) -> StorageResult<HealthStatus>;

    /// Get pool metrics
    fn metrics(&self) -> PoolMetrics;
}

/// Platform-agnostic database connection
///
/// Note: This is a marker trait. Actual operations use the SqlCipherConnection
/// concrete type to avoid trait object safety issues.
pub trait Connection: Send {
    /// Execute a SQL statement that doesn't return rows
    fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> StorageResult<usize>;

    /// Set busy timeout in milliseconds
    fn busy_timeout(&self, timeout_ms: u64) -> StorageResult<()>;
}

/// Prepared SQL statement (marker trait)
pub trait Statement {
    /// Execute the statement with parameters
    fn execute(&mut self, params: &[&dyn ToSql]) -> StorageResult<usize>;
}

/// Transaction wrapper
///
/// Transactions automatically rollback on drop unless committed.
pub struct Transaction<'conn> {
    inner: Option<RusqliteTransaction<'conn>>,
}

impl<'conn> Transaction<'conn> {
    /// Create a new transaction wrapper
    pub fn new(transaction: RusqliteTransaction<'conn>) -> Self {
        Self { inner: Some(transaction) }
    }

    /// Commit the transaction
    pub fn commit(mut self) -> StorageResult<()> {
        if let Some(tx) = self.inner.take() {
            tx.commit().map_err(StorageError::from)
        } else {
            Err(StorageError::Query("Transaction already consumed".to_string()))
        }
    }

    /// Rollback the transaction
    pub fn rollback(mut self) -> StorageResult<()> {
        if let Some(tx) = self.inner.take() {
            tx.rollback().map_err(StorageError::from)
        } else {
            Err(StorageError::Query("Transaction already consumed".to_string()))
        }
    }

    /// Execute a statement within the transaction
    pub fn execute(&self, sql: &str, params: &[&dyn ToSql]) -> StorageResult<usize> {
        if let Some(ref tx) = self.inner {
            tx.execute(sql, params).map_err(StorageError::from)
        } else {
            Err(StorageError::Query("Transaction already consumed".to_string()))
        }
    }
}

impl<'conn> Drop for Transaction<'conn> {
    fn drop(&mut self) {
        if let Some(tx) = self.inner.take() {
            // Auto-rollback on drop
            let _ = tx.rollback();
        }
    }
}

/// Health status of the storage system
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthStatus {
    /// Whether the pool is healthy
    pub healthy: bool,

    /// Number of active connections
    pub active_connections: usize,

    /// Number of idle connections
    pub idle_connections: usize,

    /// Maximum pool size
    pub max_connections: usize,

    /// Optional error message if unhealthy
    pub message: Option<String>,
}

impl HealthStatus {
    /// Create a healthy status
    pub fn healthy(active: usize, idle: usize, max: usize) -> Self {
        Self {
            healthy: true,
            active_connections: active,
            idle_connections: idle,
            max_connections: max,
            message: None,
        }
    }

    /// Create an unhealthy status
    pub fn unhealthy(message: String) -> Self {
        Self {
            healthy: false,
            active_connections: 0,
            idle_connections: 0,
            max_connections: 0,
            message: Some(message),
        }
    }
}

/// Connection pool metrics
#[derive(Debug, Clone, Default)]
pub struct PoolMetrics {
    /// Total connections acquired
    pub connections_acquired: u64,

    /// Total connection timeouts
    pub connections_timeout: u64,

    /// Total connection errors
    pub connections_error: u64,

    /// Average connection acquisition time (milliseconds)
    pub avg_acquisition_time_ms: u64,

    /// Total queries executed
    pub queries_executed: u64,

    /// Total query failures
    pub queries_failed: u64,
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::types.
    use super::*;

    /// Tests creating a healthy HealthStatus instance.
    ///
    /// Verifies all fields are set correctly for healthy state.
    #[test]
    fn test_health_status_healthy() {
        let status = HealthStatus::healthy(3, 7, 10);

        assert!(status.healthy, "Status should be marked as healthy");
        assert_eq!(status.active_connections, 3, "Should have 3 active connections");
        assert_eq!(status.idle_connections, 7, "Should have 7 idle connections");
        assert_eq!(status.max_connections, 10, "Should have max of 10 connections");
        assert!(status.message.is_none(), "Healthy status should have no error message");
    }

    /// Tests creating an unhealthy HealthStatus instance.
    ///
    /// Verifies error state is properly represented with message.
    #[test]
    fn test_health_status_unhealthy() {
        let status = HealthStatus::unhealthy("Pool exhausted".to_string());

        assert!(!status.healthy, "Status should be marked as unhealthy");
        assert_eq!(
            status.active_connections, 0,
            "Unhealthy status should zero out active connections"
        );
        assert_eq!(status.idle_connections, 0, "Unhealthy status should zero out idle connections");
        assert_eq!(status.max_connections, 0, "Unhealthy status should zero out max connections");
        assert_eq!(
            status.message.as_deref(),
            Some("Pool exhausted"),
            "Unhealthy status should contain error message"
        );
    }

    /// Tests that HealthStatus implements Clone and PartialEq correctly.
    ///
    /// Verifies cloned instances are equal to originals.
    #[test]
    fn test_health_status_clone() {
        let status1 = HealthStatus::healthy(5, 5, 10);
        let status2 = status1.clone();

        assert_eq!(status1, status2, "Cloned health status should be equal to original");
    }

    /// Tests that PoolMetrics::default() initializes all fields to zero.
    ///
    /// Verifies clean starting state for metric collection.
    #[test]
    fn test_pool_metrics_default() {
        let metrics = PoolMetrics::default();

        assert_eq!(
            metrics.connections_acquired, 0,
            "Default metrics should have zero connections acquired"
        );
        assert_eq!(metrics.connections_timeout, 0, "Default metrics should have zero timeouts");
        assert_eq!(metrics.connections_error, 0, "Default metrics should have zero errors");
        assert_eq!(
            metrics.avg_acquisition_time_ms, 0,
            "Default metrics should have zero avg acquisition time"
        );
        assert_eq!(
            metrics.queries_executed, 0,
            "Default metrics should have zero queries executed"
        );
        assert_eq!(metrics.queries_failed, 0, "Default metrics should have zero failed queries");
    }

    /// Tests that PoolMetrics can be cloned successfully.
    ///
    /// Verifies cloned metrics have same values as original.
    #[test]
    fn test_pool_metrics_clone() {
        let metrics1 = PoolMetrics {
            connections_acquired: 100,
            connections_timeout: 5,
            connections_error: 2,
            avg_acquisition_time_ms: 50,
            queries_executed: 1000,
            queries_failed: 10,
        };

        let metrics2 = metrics1.clone();

        assert_eq!(
            metrics1.connections_acquired, metrics2.connections_acquired,
            "Cloned metrics should have same connections_acquired"
        );
        assert_eq!(
            metrics1.queries_executed, metrics2.queries_executed,
            "Cloned metrics should have same queries_executed"
        );
    }
}
