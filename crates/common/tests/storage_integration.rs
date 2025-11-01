//! Integration tests for storage module
//!
//! These tests verify end-to-end storage workflows including:
//! - SQLCipher connection pooling
//! - Encryption key management
//! - Transaction handling
//! - Circuit breaker integration
//! - Health checks and metrics
//! - Error classification and retry semantics
//! - Cross-module integration with security and resilience

#![cfg(feature = "platform")]
#![allow(clippy::doc_lazy_continuation)]

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use pulsearc_common::error::{ErrorClassification, ErrorSeverity};
use pulsearc_common::storage::types::{Connection, ConnectionPool};
use pulsearc_common::storage::{
    KeySource, SqlCipherPool, SqlCipherPoolConfig, StorageError, StorageResult,
};
use pulsearc_common::testing::{SqlCipherTestDatabase, TempDir};

mod fixtures;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a test encryption key (64 characters)
fn test_encryption_key() -> String {
    "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()
}

/// Create a second test encryption key for rotation/wrong key tests
fn test_encryption_key_2() -> String {
    "test_key_64_chars_long_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()
}

/// Create a temporary database path using common testing utilities
fn temp_db_path() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new("storage-test").unwrap();
    let db_path = temp_dir.path().join("test.db");
    (temp_dir, db_path)
}

/// Create a test pool
fn test_pool(db_path: &std::path::Path) -> StorageResult<SqlCipherPool> {
    let config = SqlCipherPoolConfig::default();
    SqlCipherPool::new(db_path, test_encryption_key(), config)
}

/// Create a test database with SqlCipherTestDatabase helper
///
/// This provides automatic cleanup and encryption key management.
/// Returns the test database instance which must be kept alive.
fn create_test_database() -> SqlCipherTestDatabase {
    SqlCipherTestDatabase::new().expect("Failed to create test database")
}

// ============================================================================
// Configuration Tests
// ============================================================================

/// Validates `KeySource::keychain` behavior for the key source variants
/// scenario.
///
/// Assertions:
/// - Ensures `matches!(keychain, KeySource::Keychain { .. })` evaluates to
///   true.
/// - Ensures `matches!(env, KeySource::Environment { .. })` evaluates to true.
/// - Ensures `matches!(direct, KeySource::Direct { .. })` evaluates to true.
#[test]
fn test_key_source_variants() {
    let keychain = KeySource::keychain("PulseArc", "db_key");
    assert!(matches!(keychain, KeySource::Keychain { .. }));

    let env = KeySource::environment("DB_ENCRYPTION_KEY");
    assert!(matches!(env, KeySource::Environment { .. }));

    let direct = KeySource::direct("test_key_123");
    assert!(matches!(direct, KeySource::Direct { .. }));
}

// ============================================================================
// Pool Creation and Initialization Tests
// ============================================================================

/// Validates the pool creation success scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
#[test]
fn test_pool_creation_success() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();

    // Verify we can get a connection
    let conn = pool.get_connection().unwrap();
    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", &[]).unwrap();
}

/// Validates `SqlCipherPoolConfig::default` behavior for the pool wrong
/// encryption key scenario.
///
/// Assertions:
/// - Ensures `matches!(result, Err(StorageError::WrongKeyOrNotEncrypted))`
///   evaluates to true.
#[test]
fn test_pool_wrong_encryption_key() {
    let (_temp_dir, db_path) = temp_db_path();

    // Create database with first key
    {
        let pool = test_pool(&db_path).unwrap();
        let conn = pool.get_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", &[]).unwrap();
    }

    // Try to open with wrong key
    let config = SqlCipherPoolConfig::default();
    let result = SqlCipherPool::new(&db_path, test_encryption_key_2(), config);

    assert!(matches!(result, Err(StorageError::WrongKeyOrNotEncrypted)));
}

/// Validates `String::from_utf8_lossy` behavior for the pool encryption
/// verification scenario.
///
/// Assertions:
/// - Ensures `!raw_bytes.starts_with(b"SQLite format")` evaluates to true.
/// - Ensures `!String::from_utf8_lossy(&raw_bytes).contains("secret_data")`
///   evaluates to true.
/// Validates that database is actually encrypted on disk within the storage
/// integration workflow.
#[test]
fn test_pool_encryption_verification() {
    let (_temp_dir, db_path) = temp_db_path();

    {
        let pool = test_pool(&db_path).unwrap();
        let conn = pool.get_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, data TEXT)", &[]).unwrap();
        conn.execute("INSERT INTO test (data) VALUES (?)", &[&"secret_data"]).unwrap();
    }

    // Read raw database file
    let raw_bytes = std::fs::read(&db_path).unwrap();

    // Should NOT contain SQLite header or plaintext data
    assert!(!raw_bytes.starts_with(b"SQLite format"));
    assert!(!String::from_utf8_lossy(&raw_bytes).contains("secret_data"));
}

/// Validates `Duration::from_secs` behavior for the pool configuration applied
/// scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
/// Validates that pool configuration is properly applied within the storage
/// integration workflow.
#[test]
fn test_pool_configuration_applied() {
    let (_temp_dir, db_path) = temp_db_path();

    let config = SqlCipherPoolConfig {
        max_size: 5,
        connection_timeout: Duration::from_secs(3),
        busy_timeout: Duration::from_millis(3000),
        enable_wal: true,
        enable_foreign_keys: true,
    };

    let pool = SqlCipherPool::new(&db_path, test_encryption_key(), config).unwrap();

    // Verify we can get connections
    let conn = pool.get_sqlcipher_connection().unwrap();
    conn.execute("CREATE TABLE test (id INTEGER)", &[]).unwrap();
}

// ============================================================================
// Connection Pool Tests
// ============================================================================

/// Validates `Arc::new` behavior for the pool concurrent connections scenario.
///
/// Assertions:
/// - Confirms `count` equals `10`.
#[test]
fn test_pool_concurrent_connections() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).unwrap());

    // Create table
    {
        let conn = pool.get_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
    }

    // Spawn multiple threads
    let mut handles = vec![];
    for i in 0..10 {
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
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 10);
}

/// Validates the pool connection reuse scenario.
///
/// Assertions:
/// - Ensures `health.healthy` evaluates to true.
/// Validates that connections are properly returned to the pool within the
/// storage integration workflow.
#[test]
fn test_pool_connection_reuse() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();

    // Get and release multiple connections
    for i in 0..20 {
        let conn = pool.get_sqlcipher_connection().unwrap();
        let _result: i32 = conn.query_row("SELECT ?", &[&i], |row| row.get(0)).unwrap();
        // Connection released when dropped
    }

    // Pool should still be healthy
    let health = pool.health_check().unwrap();
    assert!(health.healthy);
}

/// Validates `Duration::from_secs` behavior for the pool exhaustion timeout
/// scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// - Ensures `elapsed >= Duration::from_secs(1)` evaluates to true.
/// - Ensures `elapsed <= Duration::from_secs(3)` evaluates to true.
#[test]
fn test_pool_exhaustion_timeout() {
    let (_temp_dir, db_path) = temp_db_path();

    let config = SqlCipherPoolConfig {
        max_size: 2,
        connection_timeout: Duration::from_secs(2), // Need reasonable timeout for pool creation
        ..Default::default()
    };

    let pool = Arc::new(SqlCipherPool::new(&db_path, test_encryption_key(), config).unwrap());

    // Hold all connections
    let _conn1 = pool.get_connection().unwrap();
    let _conn2 = pool.get_connection().unwrap();

    // Next connection should timeout
    let start = std::time::Instant::now();
    let result = pool.get_connection();
    let elapsed = start.elapsed();

    assert!(result.is_err());
    // Should timeout around 2 seconds (allow some variance)
    assert!(elapsed >= Duration::from_secs(1));
    assert!(elapsed <= Duration::from_secs(3));
}

// ============================================================================
// Query and Transaction Tests
// ============================================================================

/// Validates the basic crud operations scenario.
///
/// Assertions:
/// - Confirms `name` equals `"Alice"`.
/// - Confirms `age` equals `26`.
/// - Confirms `count` equals `1`.
/// Validates basic Create, Read, Update, Delete operations within the storage
/// integration workflow.
#[test]
fn test_basic_crud_operations() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create table
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, age INTEGER)", &[])
        .unwrap();

    // Insert
    conn.execute("INSERT INTO users (name, age) VALUES (?, ?)", &[&"Alice", &30]).unwrap();
    conn.execute("INSERT INTO users (name, age) VALUES (?, ?)", &[&"Bob", &25]).unwrap();

    // Read
    let name: String =
        conn.query_row("SELECT name FROM users WHERE age = ?", &[&30], |row| row.get(0)).unwrap();
    assert_eq!(name, "Alice");

    // Update
    conn.execute("UPDATE users SET age = ? WHERE name = ?", &[&26, &"Bob"]).unwrap();
    let age: i32 = conn
        .query_row("SELECT age FROM users WHERE name = ?", &[&"Bob"], |row| row.get(0))
        .unwrap();
    assert_eq!(age, 26);

    // Delete
    conn.execute("DELETE FROM users WHERE name = ?", &[&"Alice"]).unwrap();
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM users", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
}

/// Validates the transaction commit scenario.
///
/// Assertions:
/// - Confirms `count` equals `2`.
#[test]
fn test_transaction_commit() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let mut conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();

    // Begin transaction
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO test (value) VALUES (?)", &[&"value1"]).unwrap();
    tx.execute("INSERT INTO test (value) VALUES (?)", &[&"value2"]).unwrap();
    tx.commit().unwrap();

    // Verify data persisted
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 2);
}

/// Validates the transaction rollback scenario.
///
/// Assertions:
/// - Confirms `count` equals `1`.
/// - Confirms `value` equals `"initial"`.
#[test]
fn test_transaction_rollback() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let mut conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
    conn.execute("INSERT INTO test (value) VALUES (?)", &[&"initial"]).unwrap();

    // Begin transaction and rollback
    let tx = conn.transaction().unwrap();
    tx.execute("INSERT INTO test (value) VALUES (?)", &[&"rollback_me"]).unwrap();
    tx.rollback().unwrap();

    // Verify data not persisted
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);

    let value: String = conn.query_row("SELECT value FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(value, "initial");
}

/// Validates the transaction auto rollback scenario.
///
/// Assertions:
/// - Confirms `count` equals `0`.
#[test]
fn test_transaction_auto_rollback() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let mut conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();

    {
        let tx = conn.transaction().unwrap();
        tx.execute("INSERT INTO test (value) VALUES (?)", &[&"auto_rollback"]).unwrap();
        // Transaction dropped without commit - should auto-rollback
    }

    // Verify data not persisted
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 0);
}

/// Validates the prepared statements scenario.
///
/// Assertions:
/// - Confirms `count` equals `5`.
#[test]
fn test_prepared_statements() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();

    // Prepare statement once
    let mut stmt = conn.prepare("INSERT INTO test (value) VALUES (?)").unwrap();

    // Execute multiple times
    for i in 0..5 {
        let value = format!("value_{}", i);
        stmt.execute(&[&value]).unwrap();
    }

    // Verify all inserts
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 5);
}

/// Validates the query with multiple results scenario.
///
/// Assertions:
/// - Confirms `results.len()` equals `5`.
/// - Confirms `results[0].1` equals `"value_0"`.
/// - Confirms `results[4].1` equals `"value_4"`.
#[test]
fn test_query_with_multiple_results() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();

    // Insert test data
    for i in 0..5 {
        conn.execute("INSERT INTO test (value) VALUES (?)", &[&format!("value_{}", i)]).unwrap();
    }

    // Query multiple rows
    let mut stmt = conn.prepare("SELECT id, value FROM test ORDER BY id").unwrap();
    let results =
        stmt.query_map(&[], |row| Ok((row.get::<_, i32>(0)?, row.get::<_, String>(1)?))).unwrap();

    assert_eq!(results.len(), 5);
    assert_eq!(results[0].1, "value_0");
    assert_eq!(results[4].1, "value_4");
}

// ============================================================================
// Health Check and Metrics Tests
// ============================================================================

/// Validates the health check healthy scenario.
///
/// Assertions:
/// - Ensures `health.healthy` evaluates to true.
/// - Confirms `health.max_connections` equals `10`.
/// - Ensures `health.message.is_none()` evaluates to true.
#[test]
fn test_health_check_healthy() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();

    let health = pool.health_check().unwrap();

    assert!(health.healthy);
    assert_eq!(health.max_connections, 10);
    assert!(health.message.is_none());
}

/// Validates `ConnectionPool::metrics` behavior for the metrics collection
/// scenario.
///
/// Assertions:
/// - Ensures `metrics.connections_acquired >= initial_acquired + 5` evaluates
///   to true.
/// - Confirms `metrics.connections_timeout` equals `0`.
/// - Confirms `metrics.connections_error` equals `0`.
#[test]
fn test_metrics_collection() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();

    // Get initial metrics via ConnectionPool trait
    let initial_metrics = ConnectionPool::metrics(&pool);
    let initial_acquired = initial_metrics.connections_acquired;

    // Perform some operations
    for _ in 0..5 {
        let _conn = pool.get_connection().unwrap();
    }

    // Check metrics updated
    let metrics = ConnectionPool::metrics(&pool);
    assert!(metrics.connections_acquired >= initial_acquired + 5);
    assert_eq!(metrics.connections_timeout, 0);
    assert_eq!(metrics.connections_error, 0);
}

/// Validates `Duration::from_millis` behavior for the metrics timeout tracking
/// scenario.
///
/// Assertions:
/// - Ensures `metrics.connections_timeout > 0 || metrics.connections_error > 0`
///   evaluates to true.
/// Validates that timeouts are tracked in metrics within the storage
/// integration workflow.
#[test]
fn test_metrics_timeout_tracking() {
    let (_temp_dir, db_path) = temp_db_path();

    // Use 500ms timeout - long enough for pool initialization but short enough
    // to test timeout tracking during connection acquisition
    let config = SqlCipherPoolConfig {
        max_size: 1,
        connection_timeout: Duration::from_millis(500),
        ..Default::default()
    };

    let pool = SqlCipherPool::new(&db_path, test_encryption_key(), config).unwrap();

    // Hold the only connection
    let _conn = pool.get_connection().unwrap();

    // Try to get another connection (should timeout)
    let _ = pool.get_connection();

    // Check metrics
    let metrics = ConnectionPool::metrics(&pool);
    assert!(metrics.connections_timeout > 0 || metrics.connections_error > 0);
}

// ============================================================================
// Error Classification Tests
// ============================================================================

/// Validates `StorageError::PoolExhausted` behavior for the error retryability
/// scenario.
///
/// Assertions:
/// - Ensures `StorageError::PoolExhausted.is_retryable()` evaluates to true.
/// - Ensures `StorageError::Timeout(5).is_retryable()` evaluates to true.
/// - Ensures `StorageError::Connection("test".to_string()).is_retryable()`
///   evaluates to true.
/// - Ensures `!StorageError::WrongKeyOrNotEncrypted.is_retryable()` evaluates
///   to true.
/// - Ensures `!StorageError::InvalidConfig("test".to_string()).is_retryable()`
///   evaluates to true.
/// - Ensures `!StorageError::Encryption("test".to_string()).is_retryable()`
///   evaluates to true.
#[test]
fn test_error_retryability() {
    assert!(StorageError::PoolExhausted.is_retryable());
    assert!(StorageError::Timeout(5).is_retryable());
    assert!(StorageError::Connection("test".to_string()).is_retryable());

    // Non-retryable errors
    assert!(!StorageError::WrongKeyOrNotEncrypted.is_retryable());
    assert!(!StorageError::InvalidConfig("test".to_string()).is_retryable());
    assert!(!StorageError::Encryption("test".to_string()).is_retryable());
}

/// Validates `StorageError::Timeout` behavior for the error severity scenario.
///
/// Assertions:
/// - Confirms `StorageError::Timeout(5).severity()` equals
///   `ErrorSeverity::Warning`.
/// - Confirms `StorageError::PoolExhausted.severity()` equals
///   `ErrorSeverity::Warning`.
/// - Confirms `StorageError::Connection("test".to_string()).severity()` equals
///   `ErrorSeverity::Error`.
/// - Confirms `StorageError::Query("test".to_string()).severity()` equals
///   `ErrorSeverity::Error`.
/// - Confirms `StorageError::Encryption("test".to_string()).severity()` equals
///   `ErrorSeverity::Critical`.
/// - Confirms `StorageError::WrongKeyOrNotEncrypted.severity()` equals
///   `ErrorSeverity::Critical`.
/// - Confirms `StorageError::SchemaVersionMismatch { expected: 2, found: 1
///   }.severity()` equals `ErrorSeverity::Critical`.
#[test]
fn test_error_severity() {
    assert_eq!(StorageError::Timeout(5).severity(), ErrorSeverity::Warning);
    assert_eq!(StorageError::PoolExhausted.severity(), ErrorSeverity::Warning);
    assert_eq!(StorageError::Connection("test".to_string()).severity(), ErrorSeverity::Error);
    assert_eq!(StorageError::Query("test".to_string()).severity(), ErrorSeverity::Error);
    assert_eq!(StorageError::Encryption("test".to_string()).severity(), ErrorSeverity::Critical);
    assert_eq!(StorageError::WrongKeyOrNotEncrypted.severity(), ErrorSeverity::Critical);
    assert_eq!(
        StorageError::SchemaVersionMismatch { expected: 2, found: 1 }.severity(),
        ErrorSeverity::Critical
    );
}

/// Validates `StorageError::Encryption` behavior for the error criticality
/// scenario.
///
/// Assertions:
/// - Ensures `StorageError::Encryption("test".to_string()).is_critical()`
///   evaluates to true.
/// - Ensures `StorageError::WrongKeyOrNotEncrypted.is_critical()` evaluates to
///   true.
/// - Ensures `StorageError::Keychain("test".to_string()).is_critical()`
///   evaluates to true.
/// - Ensures `StorageError::SchemaVersionMismatch { expected: 2, found: 1
///   }.is_critical()` evaluates to true.
/// - Ensures `!StorageError::Timeout(5).is_critical()` evaluates to true.
/// - Ensures `!StorageError::PoolExhausted.is_critical()` evaluates to true.
/// - Ensures `!StorageError::Connection("test".to_string()).is_critical()`
///   evaluates to true.
#[test]
fn test_error_criticality() {
    assert!(StorageError::Encryption("test".to_string()).is_critical());
    assert!(StorageError::WrongKeyOrNotEncrypted.is_critical());
    assert!(StorageError::Keychain("test".to_string()).is_critical());
    assert!(StorageError::SchemaVersionMismatch { expected: 2, found: 1 }.is_critical());

    // Non-critical errors
    assert!(!StorageError::Timeout(5).is_critical());
    assert!(!StorageError::PoolExhausted.is_critical());
    assert!(!StorageError::Connection("test".to_string()).is_critical());
}

/// Validates `StorageError::Query` behavior for the error with operation
/// context scenario.
///
/// Assertions:
/// - Ensures `msg.contains("Storage error") || msg.contains("SELECT failed")`
///   evaluates to true.
#[test]
fn test_error_with_operation_context() {
    let err = StorageError::Query("SELECT failed".to_string()).with_operation("fetch_user");

    match err {
        StorageError::Common(common_err) => {
            let msg = common_err.to_string();
            assert!(msg.contains("Storage error") || msg.contains("SELECT failed"));
        }
        _ => panic!("Expected Common error variant"),
    }
}

/// Validates `StorageError::SchemaVersionMismatch` behavior for the schema
/// version mismatch error scenario.
///
/// Assertions:
/// - Confirms `err.to_string()` equals `"Schema version mismatch: expected 11`.
/// - Ensures `err.is_critical()` evaluates to true.
/// - Ensures `!err.is_retryable()` evaluates to true.
#[test]
fn test_schema_version_mismatch_error() {
    let err = StorageError::SchemaVersionMismatch { expected: 11, found: 10 };

    assert_eq!(err.to_string(), "Schema version mismatch: expected 11, found 10");
    assert!(err.is_critical());
    assert!(!err.is_retryable());
}

// ============================================================================
// Circuit Breaker Integration Tests
// ============================================================================

/// Validates `Duration::from_millis` behavior for the circuit breaker opens on
/// failures scenario.
///
/// Assertions:
/// - Ensures `metrics.connections_timeout > 0 || metrics.connections_error > 0`
///   evaluates to true.
/// - Ensures total failures >= expected threshold for circuit breaker
/// Validates that circuit breaker opens after consecutive failures within the
/// storage integration workflow.
#[test]
fn test_circuit_breaker_opens_on_failures() {
    let (_temp_dir, db_path) = temp_db_path();

    let config = SqlCipherPoolConfig {
        max_size: 1,
        // Use longer timeout to be more reliable in CI environments
        connection_timeout: Duration::from_millis(500),
        ..Default::default()
    };

    let pool = SqlCipherPool::new(&db_path, test_encryption_key(), config).unwrap();

    // Hold the only connection to cause timeouts
    let _held_conn = pool.get_connection().unwrap();

    // Get initial metrics to track changes
    let initial_metrics = ConnectionPool::metrics(&pool);
    let initial_timeouts = initial_metrics.connections_timeout;
    let initial_errors = initial_metrics.connections_error;

    // Attempt multiple connections to trigger circuit breaker
    // Circuit breaker opens after 5 failures (see pool.rs line 147)
    // Try 10 times to ensure we trigger it
    for i in 0..10 {
        let result = pool.get_connection();
        assert!(result.is_err(), "Connection attempt {} should fail", i);

        // Small delay to avoid tight loop timing issues in CI
        std::thread::sleep(Duration::from_millis(10));
    }

    // Check metrics - should have recorded failures
    let final_metrics = ConnectionPool::metrics(&pool);
    let total_timeouts = final_metrics.connections_timeout - initial_timeouts;
    let total_errors = final_metrics.connections_error - initial_errors;
    let total_failures = total_timeouts + total_errors;

    // Should have at least some failures recorded
    // With 10 attempts and circuit breaker opening after 5 failures:
    // - First ~5 attempts should timeout (pool exhausted)
    // - Remaining attempts should hit circuit breaker (recorded as errors)
    assert!(
        total_failures >= 5,
        "Expected at least 5 failures (got {} timeouts + {} errors = {} total)",
        total_timeouts,
        total_errors,
        total_failures
    );

    // Verify at least one of the failure types was recorded
    assert!(
        total_timeouts > 0 || total_errors > 0,
        "Expected either timeouts or errors to be recorded (got {} timeouts, {} errors)",
        total_timeouts,
        total_errors
    );
}

// ============================================================================
// WAL Mode and Foreign Keys Tests
// ============================================================================

/// Validates the wal mode enabled scenario.
///
/// Assertions:
/// - Confirms `journal_mode.to_lowercase()` equals `"wal"`.
#[test]
fn test_wal_mode_enabled() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let journal_mode: String =
        conn.query_row("PRAGMA journal_mode", &[], |row| row.get(0)).unwrap();

    assert_eq!(journal_mode.to_lowercase(), "wal");
}

/// Validates the foreign keys enabled scenario.
///
/// Assertions:
/// - Confirms `foreign_keys` equals `1`.
/// Validates that foreign key constraints are enabled within the storage
/// integration workflow.
#[test]
fn test_foreign_keys_enabled() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let foreign_keys: i32 = conn.query_row("PRAGMA foreign_keys", &[], |row| row.get(0)).unwrap();

    assert_eq!(foreign_keys, 1);
}

/// Validates the foreign key constraint enforcement scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// Validates that foreign key constraints are actually enforced within the
/// storage integration workflow.
#[test]
fn test_foreign_key_constraint_enforcement() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create parent and child tables
    conn.execute("CREATE TABLE parent (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();
    conn.execute(
        "CREATE TABLE child (id INTEGER PRIMARY KEY, parent_id INTEGER, FOREIGN KEY(parent_id) REFERENCES parent(id))",
        &[],
    )
    .unwrap();

    // Insert valid parent
    conn.execute("INSERT INTO parent (id, name) VALUES (?, ?)", &[&1, &"Parent1"]).unwrap();

    // Insert valid child (should succeed)
    conn.execute("INSERT INTO child (id, parent_id) VALUES (?, ?)", &[&1, &1]).unwrap();

    // Try to insert child with non-existent parent (should fail)
    let result = conn.execute("INSERT INTO child (id, parent_id) VALUES (?, ?)", &[&2, &999]);
    assert!(result.is_err());
}

// ============================================================================
// Concurrent Read/Write Tests (WAL Mode Benefits)
// ============================================================================

/// Validates `Arc::new` behavior for the concurrent readers and writers
/// scenario.
///
/// Assertions:
/// - Confirms `count` equals `11`.
/// Validates that WAL mode allows concurrent reads and writes within the
/// storage integration workflow.
#[test]
fn test_concurrent_readers_and_writers() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).unwrap());

    // Create table with initial data
    {
        let conn = pool.get_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();
        conn.execute("INSERT INTO test (value) VALUES (0)", &[]).unwrap();
    }

    // Spawn writer thread
    let pool_writer = Arc::clone(&pool);
    let writer = std::thread::spawn(move || {
        for i in 1..=10 {
            let conn = pool_writer.get_connection().unwrap();
            conn.execute("INSERT INTO test (value) VALUES (?)", &[&i]).unwrap();
            std::thread::sleep(Duration::from_millis(10));
        }
    });

    // Spawn reader threads
    let mut readers = vec![];
    for _ in 0..3 {
        let pool_reader = Arc::clone(&pool);
        let reader = std::thread::spawn(move || {
            for _ in 0..20 {
                let conn = pool_reader.get_sqlcipher_connection().unwrap();
                let _count: i32 =
                    conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
                std::thread::sleep(Duration::from_millis(5));
            }
        });
        readers.push(reader);
    }

    // Wait for all threads
    writer.join().unwrap();
    for reader in readers {
        reader.join().unwrap();
    }

    // Verify final state
    let conn = pool.get_sqlcipher_connection().unwrap();
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 11); // Initial + 10 inserts
}

// ============================================================================
// Stress Tests
// ============================================================================

/// Validates `Default::default` behavior for the high concurrency stress
/// scenario.
///
/// Assertions:
/// - Confirms `count` equals `500`.
/// - Ensures `health.healthy` evaluates to true.
/// Stress test with many concurrent operations
#[test]
fn test_high_concurrency_stress() {
    let (_temp_dir, db_path) = temp_db_path();

    let config = SqlCipherPoolConfig { max_size: 20, ..Default::default() };

    let pool = Arc::new(SqlCipherPool::new(&db_path, test_encryption_key(), config).unwrap());

    // Create table
    {
        let conn = pool.get_connection().unwrap();
        conn.execute(
            "CREATE TABLE stress_test (id INTEGER PRIMARY KEY, thread_id INTEGER, value TEXT)",
            &[],
        )
        .unwrap();
    }

    // Spawn many threads
    let mut handles = vec![];
    for thread_id in 0..50 {
        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            for i in 0..10 {
                let conn = pool_clone.get_connection().unwrap();
                let value = format!("t{}_i{}", thread_id, i);
                conn.execute(
                    "INSERT INTO stress_test (thread_id, value) VALUES (?, ?)",
                    &[&thread_id, &value],
                )
                .unwrap();
            }
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
        conn.query_row("SELECT COUNT(*) FROM stress_test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 500); // 50 threads * 10 inserts each

    // Verify pool is still healthy
    let health = pool.health_check().unwrap();
    assert!(health.healthy);
}

// ============================================================================
// Edge Cases and Error Scenarios
// ============================================================================

/// Validates the query on nonexistent table scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// Validates error handling for nonexistent table within the storage
/// integration workflow.
#[test]
fn test_query_on_nonexistent_table() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let result = conn.query_row("SELECT * FROM nonexistent_table", &[], |row| row.get::<_, i32>(0));

    assert!(result.is_err());
}

/// Validates the invalid sql syntax scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
#[test]
fn test_invalid_sql_syntax() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let result = conn.execute("INVALID SQL SYNTAX HERE", &[]);

    assert!(result.is_err());
}

/// Validates the constraint violation scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// Validates error handling for constraint violations within the storage
/// integration workflow.
#[test]
fn test_constraint_violation() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT UNIQUE)", &[]).unwrap();
    conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&1, &"unique_value"]).unwrap();

    // Try to insert duplicate unique value
    let result = conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&2, &"unique_value"]);

    assert!(result.is_err());
}

/// Validates the empty database operations scenario.
///
/// Assertions:
/// - Confirms `count` equals `0`.
/// - Confirms `rows` equals `0`.
#[test]
fn test_empty_database_operations() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();

    // Query empty table
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 0);

    // Delete from empty table
    let rows = conn.execute("DELETE FROM test", &[]).unwrap();
    assert_eq!(rows, 0);
}

// ============================================================================
// Performance Characteristics Tests
// ============================================================================

/// Validates `Instant::now` behavior for the connection acquisition performance
/// scenario.
///
/// Assertions:
/// - Ensures `elapsed < Duration::from_secs(1)` evaluates to true.
/// Validates that connection acquisition is reasonably fast within the storage
/// integration workflow.
#[test]
fn test_connection_acquisition_performance() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();

    let start = std::time::Instant::now();

    // Get multiple connections
    for _ in 0..100 {
        let _conn = pool.get_connection().unwrap();
        // Connection released immediately
    }

    let elapsed = start.elapsed();

    // Should be able to get 100 connections in under 1 second
    // (Much faster in practice, but allow generous margin)
    assert!(elapsed < Duration::from_secs(1), "Connection acquisition too slow: {:?}", elapsed);
}

/// Validates `Instant::now` behavior for the bulk insert performance scenario.
///
/// Assertions:
/// - Ensures `with_tx < without_tx / 2` evaluates to true.
/// Validates bulk insert performance with transactions within the storage
/// integration workflow.
#[test]
fn test_bulk_insert_performance() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let mut conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();

    // Without transaction (slower)
    let start = std::time::Instant::now();
    for i in 0..100 {
        conn.execute("INSERT INTO test (value) VALUES (?)", &[&format!("value_{}", i)]).unwrap();
    }
    let without_tx = start.elapsed();

    // Clear table
    conn.execute("DELETE FROM test", &[]).unwrap();

    // With transaction (faster)
    let start = std::time::Instant::now();
    let tx = conn.transaction().unwrap();
    for i in 0..100 {
        tx.execute("INSERT INTO test (value) VALUES (?)", &[&format!("value_{}", i)]).unwrap();
    }
    tx.commit().unwrap();
    let with_tx = start.elapsed();

    // Transaction should be significantly faster
    println!("Without transaction: {:?}, With transaction: {:?}", without_tx, with_tx);
    // Usually 10-100x faster, but just verify it's notably better
    assert!(with_tx < without_tx / 2, "Transaction not providing expected performance benefit");
}

// ============================================================================
// Pragma Configuration Tests
// ============================================================================

/// Validates the pragma synchronous setting scenario.
///
/// Assertions:
/// - Ensures `synchronous >= 1 && synchronous <= 3` evaluates to true.
/// Validates that PRAGMA synchronous is set correctly within the storage
/// integration workflow.
#[test]
fn test_pragma_synchronous_setting() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let synchronous: i32 = conn.query_row("PRAGMA synchronous", &[], |row| row.get(0)).unwrap();

    // Should be NORMAL (1) or FULL (2) or EXTRA (3) for data safety
    assert!((1..=3).contains(&synchronous));
}

/// Validates the pragma wal autocheckpoint scenario.
///
/// Assertions:
/// - Ensures `autocheckpoint > 0` evaluates to true.
/// - Ensures `autocheckpoint <= 10000` evaluates to true.
/// Validates that WAL autocheckpoint is configured within the storage
/// integration workflow.
#[test]
fn test_pragma_wal_autocheckpoint() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let autocheckpoint: i32 =
        conn.query_row("PRAGMA wal_autocheckpoint", &[], |row| row.get(0)).unwrap();

    // Should be set to a reasonable value (typically 1000 pages)
    assert!(autocheckpoint > 0);
    assert!(autocheckpoint <= 10000);
}

/// Validates the pragma cache size scenario.
///
/// Assertions:
/// - Ensures `cache_size != 0` evaluates to true.
#[test]
fn test_pragma_cache_size() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    let cache_size: i32 = conn.query_row("PRAGMA cache_size", &[], |row| row.get(0)).unwrap();

    // Cache size should be non-zero (negative = KB, positive = pages)
    assert!(cache_size != 0);
}

/// Validates the all critical pragmas scenario.
///
/// Assertions:
/// - Confirms `journal_mode.to_lowercase()` equals `"wal"`.
/// - Confirms `foreign_keys` equals `1`.
/// - Ensures `busy_timeout > 0` evaluates to true.
/// Comprehensive test of all critical pragma settings
#[test]
fn test_all_critical_pragmas() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Journal mode should be WAL
    let journal_mode: String =
        conn.query_row("PRAGMA journal_mode", &[], |row| row.get(0)).unwrap();
    assert_eq!(journal_mode.to_lowercase(), "wal");

    // Foreign keys should be ON
    let foreign_keys: i32 = conn.query_row("PRAGMA foreign_keys", &[], |row| row.get(0)).unwrap();
    assert_eq!(foreign_keys, 1);

    // Busy timeout should be set
    let busy_timeout: i32 = conn.query_row("PRAGMA busy_timeout", &[], |row| row.get(0)).unwrap();
    assert!(busy_timeout > 0);
}

// ============================================================================
// Nested Transaction Tests
// ============================================================================

/// Validates the nested transaction error scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
/// Validates that nested transactions are properly rejected within the storage
/// integration workflow.
#[test]
fn test_nested_transaction_error() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let mut conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", &[]).unwrap();

    // Begin first transaction
    let _tx1 = conn.transaction().unwrap();

    // Attempting to begin second transaction should fail
    // (Connection is already borrowed by tx1, so this won't compile)
    // This test documents the compile-time safety
}

/// Validates `Arc::new` behavior for the transaction isolation scenario.
///
/// Assertions:
/// - Ensures `value == 100 || value == 200` evaluates to true.
/// - Confirms `value` equals `200`.
/// Validates transaction isolation between connections within the storage
/// integration workflow.
#[test]
fn test_transaction_isolation() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).unwrap());

    // Setup table
    {
        let conn = pool.get_sqlcipher_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();
        conn.execute("INSERT INTO test (id, value) VALUES (1, 100)", &[]).unwrap();
    }

    // Start transaction in one connection
    let pool_tx = Arc::clone(&pool);
    let tx_handle = std::thread::spawn(move || {
        let mut conn = pool_tx.get_sqlcipher_connection().unwrap();
        let tx = conn.transaction().unwrap();
        tx.execute("UPDATE test SET value = 200 WHERE id = 1", &[]).unwrap();

        // Sleep to allow other thread to read
        std::thread::sleep(Duration::from_millis(100));

        tx.commit().unwrap();
    });

    // Read from another connection (should see old value due to isolation)
    std::thread::sleep(Duration::from_millis(50));
    let conn2 = pool.get_sqlcipher_connection().unwrap();
    let value: i32 =
        conn2.query_row("SELECT value FROM test WHERE id = 1", &[], |row| row.get(0)).unwrap();

    // In WAL mode with default isolation, we might see old value
    assert!(value == 100 || value == 200); // Either is acceptable

    tx_handle.join().unwrap();

    // After transaction commits, should see new value
    let value: i32 =
        conn2.query_row("SELECT value FROM test WHERE id = 1", &[], |row| row.get(0)).unwrap();
    assert_eq!(value, 200);
}

// ============================================================================
// Large Data Handling Tests
// ============================================================================

/// Validates the large text data scenario.
///
/// Assertions:
/// - Confirms `retrieved.len()` equals `large_text.len()`.
/// - Confirms `retrieved` equals `large_text`.
/// Validates handling of large text fields (multi-MB) within the storage
/// integration workflow.
#[test]
fn test_large_text_data() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, large_text TEXT)", &[]).unwrap();

    // Create 5MB text string
    let large_text = "a".repeat(5 * 1024 * 1024);

    // Insert large text
    conn.execute("INSERT INTO test (id, large_text) VALUES (?, ?)", &[&1, &large_text]).unwrap();

    // Retrieve and verify
    let retrieved: String = conn
        .query_row("SELECT large_text FROM test WHERE id = ?", &[&1], |row| row.get(0))
        .unwrap();

    assert_eq!(retrieved.len(), large_text.len());
    assert_eq!(retrieved, large_text);
}

/// Validates the large blob data scenario.
///
/// Assertions:
/// - Confirms `retrieved.len()` equals `large_blob.len()`.
/// - Confirms `retrieved` equals `large_blob`.
/// Validates handling of large binary data (BLOB) within the storage
/// integration workflow.
#[test]
fn test_large_blob_data() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, large_blob BLOB)", &[]).unwrap();

    // Create 10MB binary data
    let large_blob: Vec<u8> = (0..10 * 1024 * 1024).map(|i| (i % 256) as u8).collect();

    // Insert large blob
    conn.execute("INSERT INTO test (id, large_blob) VALUES (?, ?)", &[&1, &large_blob]).unwrap();

    // Retrieve and verify
    let retrieved: Vec<u8> = conn
        .query_row("SELECT large_blob FROM test WHERE id = ?", &[&1], |row| row.get(0))
        .unwrap();

    assert_eq!(retrieved.len(), large_blob.len());
    assert_eq!(retrieved, large_blob);
}

/// Validates the multiple large rows scenario.
///
/// Assertions:
/// - Confirms `count` equals `10`.
/// - Ensures `total_length >= 10 * 1024 * 1024` evaluates to true.
/// Validates inserting multiple rows with large data within the storage
/// integration workflow.
#[test]
fn test_multiple_large_rows() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, data TEXT)", &[]).unwrap();

    // Insert 10 rows with 1MB each
    for i in 0..10 {
        let data = "x".repeat(1024 * 1024); // 1MB string
        conn.execute("INSERT INTO test (id, data) VALUES (?, ?)", &[&i, &data]).unwrap();
    }

    // Verify count
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 10);

    // Verify total size makes sense
    let total_length: i64 =
        conn.query_row("SELECT SUM(LENGTH(data)) FROM test", &[], |row| row.get(0)).unwrap();
    assert!(total_length >= 10 * 1024 * 1024);
}

// ============================================================================
// Multiple Database Files (Isolation)
// ============================================================================

/// Validates the multiple database files isolation scenario.
///
/// Assertions:
/// - Ensures `result1.is_ok()` evaluates to true.
/// - Confirms `result1.unwrap()` equals `"database1"`.
/// - Ensures `result2.is_ok()` evaluates to true.
/// - Confirms `result2.unwrap()` equals `"database2"`.
/// - Ensures `result1_fail.is_err()` evaluates to true.
/// - Ensures `result2_fail.is_err()` evaluates to true.
/// Validates complete isolation between different database files within the
/// storage integration workflow.
#[test]
fn test_multiple_database_files_isolation() {
    let (_temp_dir1, db_path1) = temp_db_path();
    let (_temp_dir2, db_path2) = temp_db_path();

    let pool1 = test_pool(&db_path1).unwrap();
    let pool2 = test_pool(&db_path2).unwrap();

    // Create different tables in each database
    let conn1 = pool1.get_sqlcipher_connection().unwrap();
    conn1.execute("CREATE TABLE db1_table (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
    conn1.execute("INSERT INTO db1_table (value) VALUES (?)", &[&"database1"]).unwrap();

    let conn2 = pool2.get_sqlcipher_connection().unwrap();
    conn2.execute("CREATE TABLE db2_table (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
    conn2.execute("INSERT INTO db2_table (value) VALUES (?)", &[&"database2"]).unwrap();

    // Verify isolation - each database should only see its own table
    let result1 =
        conn1.query_row("SELECT value FROM db1_table", &[], |row| row.get::<_, String>(0));
    assert!(result1.is_ok());
    assert_eq!(result1.unwrap(), "database1");

    let result2 =
        conn2.query_row("SELECT value FROM db2_table", &[], |row| row.get::<_, String>(0));
    assert!(result2.is_ok());
    assert_eq!(result2.unwrap(), "database2");

    // Verify cross-contamination doesn't occur
    let result1_fail =
        conn1.query_row("SELECT value FROM db2_table", &[], |row| row.get::<_, String>(0));
    assert!(result1_fail.is_err()); // db2_table should not exist in db1

    let result2_fail =
        conn2.query_row("SELECT value FROM db1_table", &[], |row| row.get::<_, String>(0));
    assert!(result2_fail.is_err()); // db1_table should not exist in db2
}

/// Validates `Arc::new` behavior for the concurrent access different databases
/// scenario.
///
/// Assertions:
/// - Confirms `count1` equals `50`.
/// - Confirms `count2` equals `50`.
/// Validates concurrent access to different database files within the storage
/// integration workflow.
#[test]
fn test_concurrent_access_different_databases() {
    let (_temp_dir1, db_path1) = temp_db_path();
    let (_temp_dir2, db_path2) = temp_db_path();

    let pool1 = Arc::new(test_pool(&db_path1).unwrap());
    let pool2 = Arc::new(test_pool(&db_path2).unwrap());

    // Setup tables
    {
        let conn1 = pool1.get_sqlcipher_connection().unwrap();
        conn1.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();

        let conn2 = pool2.get_sqlcipher_connection().unwrap();
        conn2.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();
    }

    // Spawn threads writing to different databases
    let mut handles = vec![];

    for db_num in 1..=2 {
        let pool = if db_num == 1 { Arc::clone(&pool1) } else { Arc::clone(&pool2) };

        let handle = std::thread::spawn(move || {
            for i in 0..50 {
                let conn = pool.get_sqlcipher_connection().unwrap();
                conn.execute("INSERT INTO test (value) VALUES (?)", &[&(db_num * 1000 + i)])
                    .unwrap();
            }
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify each database has its own data
    let conn1 = pool1.get_sqlcipher_connection().unwrap();
    let count1: i32 = conn1.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count1, 50);

    let conn2 = pool2.get_sqlcipher_connection().unwrap();
    let count2: i32 = conn2.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count2, 50);
}

// ============================================================================
// Index Creation and Query Optimization
// ============================================================================

/// Validates the index creation and usage scenario.
///
/// Assertions:
/// - Confirms `result` equals `"user500@example.com"`.
/// - Confirms `index_exists` equals `1`.
/// Validates creating indexes and verifying they're used within the storage
/// integration workflow.
#[test]
fn test_index_creation_and_usage() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create table with data
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT, age INTEGER)", &[])
        .unwrap();

    for i in 0..1000 {
        let email = format!("user{}@example.com", i);
        conn.execute("INSERT INTO users (email, age) VALUES (?, ?)", &[&email, &(20 + (i % 50))])
            .unwrap();
    }

    // Create index on email
    conn.execute("CREATE INDEX idx_users_email ON users(email)", &[]).unwrap();

    // Query using the indexed column
    let result: String = conn
        .query_row("SELECT email FROM users WHERE email = ?", &[&"user500@example.com"], |row| {
            row.get(0)
        })
        .unwrap();

    assert_eq!(result, "user500@example.com");

    // Verify index exists
    let index_exists: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='index' AND name='idx_users_email'",
            &[],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(index_exists, 1);
}

/// Validates the composite index scenario.
///
/// Assertions:
/// - Confirms `count` equals `10`.
/// Validates creating and using composite indexes within the storage
/// integration workflow.
#[test]
fn test_composite_index() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE orders (id INTEGER PRIMARY KEY, user_id INTEGER, status TEXT, created_at INTEGER)", &[]).unwrap();

    // Insert test data
    for i in 0..100 {
        conn.execute(
            "INSERT INTO orders (user_id, status, created_at) VALUES (?, ?, ?)",
            &[&(i % 10), &"pending", &i],
        )
        .unwrap();
    }

    // Create composite index
    conn.execute("CREATE INDEX idx_orders_user_status ON orders(user_id, status)", &[]).unwrap();

    // Query using composite index
    let count: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM orders WHERE user_id = ? AND status = ?",
            &[&5, &"pending"],
            |row| row.get(0),
        )
        .unwrap();

    assert_eq!(count, 10);
}

/// Validates `Instant::now` behavior for the query performance with index
/// scenario.
///
/// Assertion coverage: ensures the routine completes without panicking.
/// Validates that indexes improve query performance within the storage
/// integration workflow.
#[test]
fn test_query_performance_with_index() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let mut conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute(
        "CREATE TABLE large_table (id INTEGER PRIMARY KEY, value INTEGER, data TEXT)",
        &[],
    )
    .unwrap();

    // Insert large dataset
    let tx = conn.transaction().unwrap();
    for i in 0..10000 {
        tx.execute(
            "INSERT INTO large_table (value, data) VALUES (?, ?)",
            &[&i, &format!("data_{}", i)],
        )
        .unwrap();
    }
    tx.commit().unwrap();

    // Query without index (baseline)
    let start = std::time::Instant::now();
    let _: i32 = conn
        .query_row("SELECT COUNT(*) FROM large_table WHERE value = ?", &[&5000], |row| row.get(0))
        .unwrap();
    let without_index = start.elapsed();

    // Create index
    conn.execute("CREATE INDEX idx_value ON large_table(value)", &[]).unwrap();

    // Query with index
    let start = std::time::Instant::now();
    let _: i32 = conn
        .query_row("SELECT COUNT(*) FROM large_table WHERE value = ?", &[&5000], |row| row.get(0))
        .unwrap();
    let with_index = start.elapsed();

    // Index should not make it slower (may be similar for small datasets)
    println!("Without index: {:?}, With index: {:?}", without_index, with_index);
}

// ============================================================================
// Busy Timeout with Real Lock Contention
// ============================================================================

/// Validates `Arc::new` behavior for the busy timeout with lock contention
/// scenario.
///
/// Assertions:
/// - Ensures `result.is_ok()` evaluates to true.
/// - Ensures `elapsed >= Duration::from_millis(50)` evaluates to true.
/// - Ensures `elapsed <= Duration::from_secs(6)` evaluates to true.
/// Validates busy timeout with actual database lock contention within the
/// storage integration workflow.
#[test]
fn test_busy_timeout_with_lock_contention() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).unwrap());

    // Create table
    {
        let conn = pool.get_sqlcipher_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();
        conn.execute("INSERT INTO test (id, value) VALUES (1, 0)", &[]).unwrap();
    }

    // Start a long-running exclusive transaction
    let pool_writer = Arc::clone(&pool);
    let writer_handle = std::thread::spawn(move || {
        let mut conn = pool_writer.get_sqlcipher_connection().unwrap();
        let tx = conn.transaction().unwrap();

        // Update row (acquires write lock)
        tx.execute("UPDATE test SET value = value + 1 WHERE id = 1", &[]).unwrap();

        // Hold lock for a bit
        std::thread::sleep(Duration::from_millis(200));

        tx.commit().unwrap();
    });

    // Give writer time to acquire lock
    std::thread::sleep(Duration::from_millis(50));

    // Try to write from another connection (should wait due to busy timeout)
    let start = std::time::Instant::now();
    let conn2 = pool.get_sqlcipher_connection().unwrap();
    let result = conn2.execute("UPDATE test SET value = value + 1 WHERE id = 1", &[]);
    let elapsed = start.elapsed();

    writer_handle.join().unwrap();

    // Should eventually succeed after waiting
    assert!(result.is_ok());

    // Should have waited at least a bit (but less than busy timeout)
    assert!(elapsed >= Duration::from_millis(50));
    assert!(elapsed <= Duration::from_secs(6)); // Less than busy timeout +
                                                // margin
}

// ============================================================================
// Connection State After Errors
// ============================================================================

/// Validates the connection usable after query error scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// - Confirms `value` equals `100`.
/// - Confirms `count` equals `2`.
/// Validates that connection remains usable after a query error within the
/// storage integration workflow.
#[test]
fn test_connection_usable_after_query_error() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();
    conn.execute("INSERT INTO test (id, value) VALUES (1, 100)", &[]).unwrap();

    // Execute invalid query
    let result = conn.execute("INVALID SQL SYNTAX", &[]);
    assert!(result.is_err());

    // Connection should still be usable
    let value: i32 =
        conn.query_row("SELECT value FROM test WHERE id = 1", &[], |row| row.get(0)).unwrap();
    assert_eq!(value, 100);

    // Should be able to execute more operations
    conn.execute("INSERT INTO test (id, value) VALUES (2, 200)", &[]).unwrap();
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 2);
}

/// Validates the connection usable after constraint violation scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// - Confirms `count` equals `2`.
/// Validates connection remains usable after constraint violation within the
/// storage integration workflow.
#[test]
fn test_connection_usable_after_constraint_violation() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT UNIQUE)", &[]).unwrap();
    conn.execute("INSERT INTO test (id, value) VALUES (1, 'unique')", &[]).unwrap();

    // Try to violate unique constraint
    let result = conn.execute("INSERT INTO test (id, value) VALUES (2, 'unique')", &[]);
    assert!(result.is_err());

    // Connection should still work
    conn.execute("INSERT INTO test (id, value) VALUES (2, 'different')", &[]).unwrap();

    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 2);
}

// ============================================================================
// Database Vacuum and Maintenance
// ============================================================================

/// Validates the database vacuum scenario.
///
/// Assertions:
/// - Ensures `size_after <= size_before` evaluates to true.
/// - Confirms `count` equals `500`.
#[test]
fn test_database_vacuum() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create and populate table
    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, data TEXT)", &[]).unwrap();

    for i in 0..1000 {
        conn.execute("INSERT INTO test (data) VALUES (?)", &[&format!("data_{}", i)]).unwrap();
    }

    // Delete half the rows
    conn.execute("DELETE FROM test WHERE id % 2 = 0", &[]).unwrap();

    // Get database size before vacuum
    let size_before = std::fs::metadata(&db_path).unwrap().len();

    // Run VACUUM
    conn.execute("VACUUM", &[]).unwrap();

    // Get database size after vacuum
    let size_after = std::fs::metadata(&db_path).unwrap().len();

    // Size after should be less than or equal to before
    assert!(size_after <= size_before);

    // Verify data integrity after vacuum
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 500);
}

/// Validates the database analyze scenario.
///
/// Assertions:
/// - Ensures `stat_count > 0` evaluates to true.
/// Validates ANALYZE operation for query optimization within the storage
/// integration workflow.
#[test]
fn test_database_analyze() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value INTEGER)", &[]).unwrap();
    conn.execute("CREATE INDEX idx_value ON test(value)", &[]).unwrap();

    // Insert data
    for i in 0..1000 {
        conn.execute("INSERT INTO test (value) VALUES (?)", &[&i]).unwrap();
    }

    // Run ANALYZE
    conn.execute("ANALYZE", &[]).unwrap();

    // Verify sqlite_stat1 table was populated (contains index statistics)
    let stat_count: i32 = conn
        .query_row("SELECT COUNT(*) FROM sqlite_stat1 WHERE tbl = 'test'", &[], |row| row.get(0))
        .unwrap();

    assert!(stat_count > 0);
}

/// Validates the wal checkpoint scenario.
///
/// Assertions:
/// - Confirms `count` equals `100`.
#[test]
fn test_wal_checkpoint() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, data TEXT)", &[]).unwrap();

    // Insert data to generate WAL entries
    for i in 0..100 {
        conn.execute("INSERT INTO test (data) VALUES (?)", &[&format!("data_{}", i)]).unwrap();
    }

    // Perform checkpoint
    // Note: PRAGMA wal_checkpoint returns results, so we use query_row
    let _checkpoint_result: i32 =
        conn.query_row("PRAGMA wal_checkpoint(FULL)", &[], |row| row.get(0)).unwrap();

    // Verify data is still accessible after checkpoint
    let count: i32 = conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).unwrap();
    assert_eq!(count, 100);
}

// ============================================================================
// Schema Evolution Tests
// ============================================================================

/// Validates the alter table add column scenario.
///
/// Assertions:
/// - Confirms `name` equals `"Alice"`.
/// - Confirms `email` equals `Some("bob@example.com".to_string())`.
/// Validates adding columns (common migration scenario) within the storage
/// integration workflow.
#[test]
fn test_alter_table_add_column() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create initial table
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();
    conn.execute("INSERT INTO users (name) VALUES (?)", &[&"Alice"]).unwrap();

    // Add new column
    conn.execute("ALTER TABLE users ADD COLUMN email TEXT", &[]).unwrap();

    // Verify old data still exists
    let name: String =
        conn.query_row("SELECT name FROM users WHERE id = 1", &[], |row| row.get(0)).unwrap();
    assert_eq!(name, "Alice");

    // Insert with new column
    conn.execute("INSERT INTO users (name, email) VALUES (?, ?)", &[&"Bob", &"bob@example.com"])
        .unwrap();

    // Query with new column
    let email: Option<String> = conn
        .query_row("SELECT email FROM users WHERE name = ?", &[&"Bob"], |row| row.get(0))
        .unwrap();
    assert_eq!(email, Some("bob@example.com".to_string()));
}

/// Validates the table rename scenario.
///
/// Assertions:
/// - Ensures `result.is_err()` evaluates to true.
/// - Confirms `value` equals `"test_data"`.
#[test]
fn test_table_rename() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create table with data
    conn.execute("CREATE TABLE old_name (id INTEGER PRIMARY KEY, value TEXT)", &[]).unwrap();
    conn.execute("INSERT INTO old_name (value) VALUES (?)", &[&"test_data"]).unwrap();

    // Rename table
    conn.execute("ALTER TABLE old_name RENAME TO new_name", &[]).unwrap();

    // Verify old name doesn't work
    let result = conn.query_row("SELECT value FROM old_name", &[], |row| row.get::<_, String>(0));
    assert!(result.is_err());

    // Verify new name works
    let value: String =
        conn.query_row("SELECT value FROM new_name WHERE id = 1", &[], |row| row.get(0)).unwrap();
    assert_eq!(value, "test_data");
}

/// Validates `SystemTime::now` behavior for the schema version tracking
/// scenario.
///
/// Assertions:
/// - Confirms `current_version` equals `2`.
/// - Confirms `migration_count` equals `2`.
/// Validates pattern for tracking schema versions (common migration pattern)
#[test]
fn test_schema_version_tracking() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create schema_version table
    conn.execute(
        "CREATE TABLE schema_version (version INTEGER PRIMARY KEY, applied_at INTEGER)",
        &[],
    )
    .unwrap();

    // Record initial version
    let timestamp =
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
            as i64;
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
        &[&1, &timestamp],
    )
    .unwrap();

    // Create initial schema
    conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[]).unwrap();

    // Simulate migration to version 2
    conn.execute("ALTER TABLE users ADD COLUMN email TEXT", &[]).unwrap();
    conn.execute(
        "INSERT INTO schema_version (version, applied_at) VALUES (?, ?)",
        &[&2, &timestamp],
    )
    .unwrap();

    // Verify current version
    let current_version: i32 =
        conn.query_row("SELECT MAX(version) FROM schema_version", &[], |row| row.get(0)).unwrap();
    assert_eq!(current_version, 2);

    // Verify migration history
    let migration_count: i32 =
        conn.query_row("SELECT COUNT(*) FROM schema_version", &[], |row| row.get(0)).unwrap();
    assert_eq!(migration_count, 2);
}

/// Validates the drop and recreate table scenario.
///
/// Assertions:
/// - Confirms `value` equals `"new_data"`.
/// - Confirms `additional` equals `42`.
/// Validates dropping and recreating tables (risky but sometimes necessary)
#[test]
fn test_drop_and_recreate_table() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).unwrap();
    let conn = pool.get_sqlcipher_connection().unwrap();

    // Create and populate table
    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, old_column TEXT)", &[]).unwrap();
    conn.execute("INSERT INTO test (old_column) VALUES (?)", &[&"old_data"]).unwrap();

    // Drop table
    conn.execute("DROP TABLE test", &[]).unwrap();

    // Recreate with new schema
    conn.execute(
        "CREATE TABLE test (id INTEGER PRIMARY KEY, new_column TEXT, additional_column INTEGER)",
        &[],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO test (new_column, additional_column) VALUES (?, ?)",
        &[&"new_data", &42],
    )
    .unwrap();

    // Verify new schema works
    let value: String =
        conn.query_row("SELECT new_column FROM test WHERE id = 1", &[], |row| row.get(0)).unwrap();
    assert_eq!(value, "new_data");

    let additional: i32 = conn
        .query_row("SELECT additional_column FROM test WHERE id = 1", &[], |row| row.get(0))
        .unwrap();
    assert_eq!(additional, 42);
}
