//! SQLCipher connection pool
//!
//! Provides r2d2-based connection pooling for SQLCipher databases.
//! Based on macos/db/manager.rs DbManager implementation.
//!
//! Uses agent/common/resilience for circuit breaker protection.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use tracing::{debug, info, instrument, warn};

use super::cipher::{configure_sqlcipher, verify_encryption, SqlCipherConfig};
use super::config::SqlCipherPoolConfig;
use super::connection::SqlCipherConnection;
use super::pragmas::apply_connection_pragmas;
use crate::resilience::{CircuitBreaker, CircuitBreakerConfigBuilder};
use crate::storage::error::{StorageError, StorageResult};
use crate::storage::metrics::StorageMetrics;
use crate::storage::types::{
    Connection as ConnectionTrait, ConnectionPool, HealthStatus, PoolMetrics,
};

/// SQLCipher connection pool
///
/// Manages a pool of encrypted SQLite connections using r2d2.
///
/// # Enterprise Features
/// - Connection pooling (default: 10 connections)
/// - Circuit breaker protection (from agent/common/resilience)
/// - Automatic encryption key application
/// - WAL mode for concurrency
/// - Connection timeout handling
/// - Structured tracing and logging
/// - Health checks with metrics
///
/// # Source
/// Based on macos/db/manager.rs DbManager (lines 39-150)
/// Enhanced with agent/common/resilience patterns
#[derive(Debug)]
pub struct SqlCipherPool {
    pool: Pool<SqliteConnectionManager>,
    config: SqlCipherPoolConfig,
    metrics: Arc<StorageMetrics>,
    circuit_breaker: Arc<CircuitBreaker>,
}

impl SqlCipherPool {
    /// Create a new SQLCipher connection pool
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    /// * `encryption_key` - Encryption key for SQLCipher
    /// * `config` - Pool configuration
    ///
    /// # Process
    /// 1. Create connection manager with encryption pragmas
    /// 2. Build r2d2 pool with configured size and timeouts
    /// 3. Test a connection to verify encryption works
    /// 4. Return pool
    ///
    /// # Errors
    /// Returns an error if:
    /// - Database file can't be accessed
    /// - Encryption key is wrong
    /// - Pool creation fails
    #[instrument(skip(encryption_key), fields(db_path = ?path, pool_size = config.max_size))]
    pub fn new(
        path: &Path,
        encryption_key: String,
        config: SqlCipherPoolConfig,
    ) -> StorageResult<Self> {
        info!("Creating SQLCipher connection pool");

        // Create metrics
        let metrics = Arc::new(StorageMetrics::new(config.max_size));

        // Create encryption config
        let cipher_config = SqlCipherConfig::new(encryption_key);

        // Create connection manager with initialization callback
        let pool_config = config.clone();
        let cipher_config_clone = cipher_config.clone();

        let manager = SqliteConnectionManager::file(path).with_init(move |conn| {
            // Apply SQLCipher encryption
            configure_sqlcipher(conn, &cipher_config_clone)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            // Apply connection pragmas
            apply_connection_pragmas(conn, &pool_config)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            Ok(())
        });

        // Build r2d2 pool
        let pool = Pool::builder()
            .max_size(config.max_size)
            .connection_timeout(config.connection_timeout)
            .build(manager)
            .map_err(|e| {
                warn!("Failed to create connection pool: {}", e);
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("file is not a database")
                    || err_str.contains("file is encrypted")
                    || err_str.contains("database disk image is malformed")
                    || err_str.contains("notadb")
                {
                    StorageError::WrongKeyOrNotEncrypted
                } else {
                    StorageError::Connection(format!("Failed to create pool: {}", e))
                }
            })?;

        // Verify encryption works and run migrations
        {
            let conn = pool.get().map_err(|e| {
                warn!("Failed to get test connection: {}", e);
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("file is not a database")
                    || err_str.contains("file is encrypted")
                    || err_str.contains("database disk image is malformed")
                    || err_str.contains("notadb")
                {
                    StorageError::WrongKeyOrNotEncrypted
                } else {
                    StorageError::Connection(format!("Failed to get test connection: {}", e))
                }
            })?;

            verify_encryption(&conn)?;
            debug!("Encryption verified successfully");

            // Note: Schema migrations should be handled by the application
            // layer, not in the common infrastructure. Each
            // application using this pool should manage its own
            // schema.
        }

        // Create circuit breaker for connection pooling
        // Use agent/common/resilience infrastructure
        let circuit_breaker_config = CircuitBreakerConfigBuilder::new()
            .failure_threshold(5)
            .timeout(Duration::from_secs(30))
            .success_threshold(2)
            .half_open_max_calls(3)
            .build()
            .map_err(|e| StorageError::InvalidConfig(e.to_string()))?;

        let circuit_breaker = Arc::new(
            CircuitBreaker::new(circuit_breaker_config)
                .map_err(|e| StorageError::InvalidConfig(e.to_string()))?,
        );

        info!("SQLCipher pool created successfully with {} connections", config.max_size);

        Ok(Self { pool, config, metrics, circuit_breaker })
    }

    /// Get the pool metrics
    pub fn metrics(&self) -> &Arc<StorageMetrics> {
        &self.metrics
    }
}

impl SqlCipherPool {
    /// Get a SqlCipherConnection from the pool (enterprise method with circuit
    /// breaker)
    #[instrument(skip(self), fields(pool_size = self.config.max_size))]
    pub fn get_sqlcipher_connection(&self) -> StorageResult<SqlCipherConnection> {
        let start = std::time::Instant::now();

        // Check circuit breaker before attempting connection
        if !self.circuit_breaker.can_execute() {
            // Record as connection error since this is still a failed connection attempt
            self.metrics.record_connection_error();
            warn!("Circuit breaker open, rejecting connection request");
            return Err(StorageError::Connection(
                "Circuit breaker open - connection pool temporarily unavailable".to_string(),
            ));
        }

        // Attempt to get connection from pool
        match self.pool.get() {
            Ok(conn) => {
                let duration_ms = start.elapsed().as_millis() as u64;
                self.metrics.record_connection_acquired(duration_ms);
                self.circuit_breaker.record_success();

                debug!("Connection acquired in {}ms", duration_ms);

                // Wrap in our connection type (conn is already PooledConnection)
                Ok(SqlCipherConnection::new(conn))
            }
            Err(e) => {
                // Record failure for circuit breaker
                self.circuit_breaker.record_failure();

                // Classify error
                let err_str = e.to_string().to_lowercase();

                if err_str.contains("timeout") {
                    self.metrics.record_connection_timeout();
                    warn!("Connection timeout after {:?}", self.config.connection_timeout);
                    Err(StorageError::Timeout(self.config.connection_timeout.as_secs()))
                } else {
                    self.metrics.record_connection_error();
                    warn!("Connection error: {}", e);
                    Err(StorageError::Connection(format!("Failed to get connection: {}", e)))
                }
            }
        }
    }
}

impl ConnectionPool for SqlCipherPool {
    #[instrument(skip(self), fields(pool_size = self.config.max_size))]
    fn get_connection(&self) -> StorageResult<Box<dyn ConnectionTrait>> {
        self.get_sqlcipher_connection().map(|c| Box::new(c) as Box<dyn ConnectionTrait>)
    }

    fn health_check(&self) -> StorageResult<HealthStatus> {
        // Get pool state
        let state = self.pool.state();

        // Check if we can get a connection
        match self.pool.get() {
            Ok(_conn) => Ok(HealthStatus::healthy(
                state.connections as usize,
                state.idle_connections as usize,
                self.config.max_size as usize,
            )),
            Err(e) => Ok(HealthStatus::unhealthy(format!("Pool unhealthy: {}", e))),
        }
    }

    fn metrics(&self) -> PoolMetrics {
        PoolMetrics {
            connections_acquired: self
                .metrics
                .connections_acquired
                .load(std::sync::atomic::Ordering::Relaxed),
            connections_timeout: self
                .metrics
                .connections_timeout
                .load(std::sync::atomic::Ordering::Relaxed),
            connections_error: self
                .metrics
                .connections_error
                .load(std::sync::atomic::Ordering::Relaxed),
            avg_acquisition_time_ms: self.metrics.avg_connection_time_ms(),
            queries_executed: self
                .metrics
                .queries_executed
                .load(std::sync::atomic::Ordering::Relaxed),
            queries_failed: self.metrics.queries_failed.load(std::sync::atomic::Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for storage::sqlcipher::pool.
    use tempfile::TempDir;

    use super::*;

    fn test_key() -> String {
        "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()
    }

    /// Validates `TempDir::new` behavior for the pool creation scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_pool_creation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = SqlCipherPool::new(&db_path, test_key(), config).unwrap();

        // Verify we can get a connection
        let conn = pool.get_connection().unwrap();

        // Verify we can execute queries
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", &[]).unwrap();
    }

    /// Validates `TempDir::new` behavior for the concurrent connections
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `count` equals `5`.
    #[test]
    fn test_concurrent_connections() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = Arc::new(SqlCipherPool::new(&db_path, test_key(), config).unwrap());

        // Create table
        {
            let conn = pool.get_connection().unwrap();
            conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
        }

        // Spawn multiple threads
        let mut handles = vec![];

        for i in 0..5 {
            let pool_clone = Arc::clone(&pool);
            let handle = std::thread::spawn(move || {
                let conn = pool_clone.get_connection().unwrap();
                let value = format!("thread_{}", i);
                conn.execute("INSERT INTO test (value) VALUES (?)", &[&value]).unwrap();
            });
            handles.push(handle);
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        // Verify all inserts
        let conn = pool.get_sqlcipher_connection().unwrap();
        let count: i32 =
            conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
        assert_eq!(count, 5);
    }

    /// Validates `TempDir::new` behavior for the health check scenario.
    ///
    /// Assertions:
    /// - Ensures `health.healthy` evaluates to true.
    /// - Confirms `health.max_connections` equals `10`.
    #[test]
    fn test_health_check() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = SqlCipherPoolConfig::default();
        let pool = SqlCipherPool::new(&db_path, test_key(), config).unwrap();

        let health = pool.health_check().unwrap();
        assert!(health.healthy);
        assert_eq!(health.max_connections, 10);
    }

    /// Validates `TempDir::new` behavior for the wrong encryption key scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(result, Err(StorageError::WrongKeyOrNotEncrypted))`
    ///   evaluates to true.
    #[test]
    fn test_wrong_encryption_key() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create database with one key
        {
            let config = SqlCipherPoolConfig::default();
            let pool = SqlCipherPool::new(&db_path, test_key(), config).unwrap();
            let conn = pool.get_connection().unwrap();
            conn.execute("CREATE TABLE test (id INTEGER)", &[]).unwrap();
        }

        // Try to open with wrong key
        let config = SqlCipherPoolConfig::default();
        let result = SqlCipherPool::new(
            &db_path,
            "wrong_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            config,
        );

        assert!(matches!(result, Err(StorageError::WrongKeyOrNotEncrypted)));
    }
}
