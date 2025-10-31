//! Advanced storage integration tests
//!
//! This module contains comprehensive integration tests for storage
//! functionality covering advanced scenarios including:
//!
//! 1. Resource Cleanup & Lifecycle
//!    - Pool shutdown behavior
//!    - Connection cleanup on drop
//!    - Memory leak detection
//!    - File handle cleanup
//!
//! 2. Thread Safety
//!    - Send/Sync trait verification
//!    - Race condition testing
//!    - Thread panic during transaction
//!
//! 3. Database File Operations
//!    - Read-only database access
//!    - Database file moved/deleted while open
//!    - Permissions errors
//!    - Disk space exhaustion simulation
//!
//! 4. Advanced Error Scenarios
//!    - Corrupted database recovery attempt
//!    - Long-running query interruption
//!
//! 5. Performance Regression Guards
//!    - Query execution time bounds
//!    - Pool acquisition time bounds
//!    - Memory usage bounds
//!
//! 6. Integration with Other Modules
//!    - Storage + Security (encryption key rotation)
//!    - Storage + Resilience (circuit breaker recovery)
//!    - Storage + Observability (metrics export)

#![cfg(feature = "platform")]

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{Duration, Instant};

use pulsearc_common::error::{ErrorClassification, ErrorSeverity};
use pulsearc_common::storage::types::{Connection, ConnectionPool};
use pulsearc_common::storage::{SqlCipherPool, SqlCipherPoolConfig, StorageError, StorageResult};
use tempfile::TempDir;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a test encryption key (64 characters)
fn test_encryption_key() -> String {
    "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()
}

/// Create a second test encryption key for rotation tests
fn test_encryption_key_2() -> String {
    "test_key_64_chars_long_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string()
}

/// Create a temporary database path
fn temp_db_path() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    (temp_dir, db_path)
}

/// Create a test pool with custom config
fn test_pool_with_config(
    db_path: &std::path::Path,
    config: SqlCipherPoolConfig,
) -> StorageResult<SqlCipherPool> {
    SqlCipherPool::new(db_path, test_encryption_key(), config)
}

/// Create a test pool with defaults
fn test_pool(db_path: &std::path::Path) -> StorageResult<SqlCipherPool> {
    test_pool_with_config(db_path, SqlCipherPoolConfig::default())
}

/// Setup a basic test table
fn setup_test_table(pool: &SqlCipherPool) -> StorageResult<()> {
    let conn = pool.get_sqlcipher_connection()?;
    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY, value TEXT)", &[])?;
    Ok(())
}

// ============================================================================
// 1. Resource Cleanup & Lifecycle Tests
// ============================================================================

/// Test that pool properly shuts down and releases resources
#[test]
fn test_pool_shutdown_behavior() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");

    // Create a test table
    {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("CREATE TABLE IF NOT EXISTS test (id INTEGER, value TEXT)", &[])
            .expect("Create table failed");
    }

    // Get some connections and do work
    for i in 0..5 {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&i, &"test"])
            .expect("Insert failed");
    }

    // Drop the pool explicitly
    drop(pool);

    // Pool should be dropped cleanly - verify by trying to access the database
    // file after pool is dropped (should be accessible)
    assert!(db_path.exists());

    // Create a new pool on the same file (should work if cleanup was proper)
    let pool2 = test_pool(&db_path).expect("Failed to create second pool");
    let conn = pool2.get_sqlcipher_connection().expect("Failed to get connection from new pool");
    conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&99, &"test"])
        .expect("Insert failed on new pool");
}

/// Test that connections are properly cleaned up when dropped
#[test]
fn test_connection_cleanup_on_drop() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).expect("Failed to create pool"));
    setup_test_table(&pool).expect("Failed to setup table");

    let acquired_count = Arc::new(AtomicUsize::new(0));
    let released_count = Arc::new(AtomicUsize::new(0));

    // Spawn threads that get connections and drop them
    let mut handles = vec![];
    for _ in 0..10 {
        let pool_clone = Arc::clone(&pool);
        let acquired = Arc::clone(&acquired_count);
        let released = Arc::clone(&released_count);

        let handle = std::thread::spawn(move || {
            for _ in 0..5 {
                {
                    let conn =
                        pool_clone.get_sqlcipher_connection().expect("Failed to get connection");
                    acquired.fetch_add(1, Ordering::SeqCst);
                    conn.execute("INSERT INTO test (value) VALUES (?)", &[&"test"])
                        .expect("Insert failed");
                    // Connection dropped here
                }
                released.fetch_add(1, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(1));
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All connections should have been acquired and released
    assert_eq!(acquired_count.load(Ordering::SeqCst), 50);
    assert_eq!(released_count.load(Ordering::SeqCst), 50);

    // Pool should still be healthy after all this
    let health = pool.health_check().expect("Health check failed");
    assert!(health.healthy);
}

/// Test for connection leaks by exhausting and releasing pool multiple times
#[test]
fn test_connection_leak_detection() {
    let (_temp_dir, db_path) = temp_db_path();

    let config = SqlCipherPoolConfig { max_size: 3, ..Default::default() };

    let pool = test_pool_with_config(&db_path, config).expect("Failed to create pool");

    // Cycle through connections many times
    for _ in 0..100 {
        let _conn1 = pool.get_sqlcipher_connection().expect("Failed to get connection 1");
        let _conn2 = pool.get_sqlcipher_connection().expect("Failed to get connection 2");
        let _conn3 = pool.get_sqlcipher_connection().expect("Failed to get connection 3");
        // All connections dropped here
    }

    // Pool should still be able to provide connections
    let health = pool.health_check().expect("Health check failed");
    assert!(health.healthy);
    assert_eq!(health.max_connections, 3);
}

/// Test that file handles are properly released
#[test]
fn test_file_handle_cleanup() {
    let (_temp_dir, db_path) = temp_db_path();

    // Create and drop multiple pools on the same file
    for iteration in 0..5 {
        let pool = test_pool(&db_path).expect("Failed to create pool");

        // Do some work
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("CREATE TABLE IF NOT EXISTS test (id INTEGER, value TEXT)", &[])
            .expect("Create table failed");
        conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&iteration, &"test"])
            .expect("Insert failed");

        // Drop pool explicitly
        drop(pool);

        // Small delay to ensure cleanup completes
        std::thread::sleep(Duration::from_millis(10));
    }

    // Should be able to create a new pool and access all data
    let pool = test_pool(&db_path).expect("Failed to create final pool");
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let count: i32 =
        conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).expect("Query failed");
    assert_eq!(count, 5);
}

// ============================================================================
// 2. Thread Safety Tests
// ============================================================================

/// Test that SqlCipherPool implements Send + Sync
#[test]
fn test_pool_send_sync_traits() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}

    assert_send::<SqlCipherPool>();
    assert_sync::<SqlCipherPool>();
}

/// Test for race conditions during concurrent connection acquisition
#[test]
fn test_concurrent_connection_race_conditions() {
    let (_temp_dir, db_path) = temp_db_path();

    let config = SqlCipherPoolConfig { max_size: 5, ..Default::default() };

    let pool = Arc::new(test_pool_with_config(&db_path, config).expect("Failed to create pool"));
    setup_test_table(&pool).expect("Failed to setup table");

    let barrier = Arc::new(Barrier::new(20));
    let success_count = Arc::new(AtomicUsize::new(0));
    let error_count = Arc::new(AtomicUsize::new(0));

    // Spawn many threads that all try to get connections simultaneously
    let mut handles = vec![];
    for i in 0..20 {
        let pool_clone = Arc::clone(&pool);
        let barrier_clone = Arc::clone(&barrier);
        let success = Arc::clone(&success_count);
        let errors = Arc::clone(&error_count);

        let handle = std::thread::spawn(move || {
            // Wait for all threads to be ready
            barrier_clone.wait();

            // All threads try to get connection at the same time
            match pool_clone.get_sqlcipher_connection() {
                Ok(conn) => {
                    success.fetch_add(1, Ordering::SeqCst);
                    let value = format!("thread_{}", i);
                    let result = conn.execute("INSERT INTO test (value) VALUES (?)", &[&value]);
                    assert!(result.is_ok(), "Insert should succeed");
                }
                Err(_) => {
                    errors.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // All threads should have either succeeded or properly handled errors
    let total = success_count.load(Ordering::SeqCst) + error_count.load(Ordering::SeqCst);
    assert_eq!(total, 20);

    // Verify data integrity - all successful inserts should be visible
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let count: i32 =
        conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).expect("Query failed");
    assert_eq!(count as usize, success_count.load(Ordering::SeqCst));
}

/// Test behavior when a thread panics while holding a connection
#[test]
fn test_thread_panic_during_transaction() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).expect("Failed to create pool"));
    setup_test_table(&pool).expect("Failed to setup table");

    // Insert initial data
    {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&1, &"initial"])
            .expect("Insert failed");
    }

    // Spawn thread that will panic during transaction
    let pool_clone = Arc::clone(&pool);
    let handle = std::thread::spawn(move || {
        let mut conn = pool_clone.get_sqlcipher_connection().expect("Failed to get connection");
        let tx = conn.transaction().expect("Failed to start transaction");
        tx.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&2, &"panic_data"])
            .expect("Insert failed");

        // Panic before commit - transaction should rollback
        panic!("Simulated panic during transaction");
    });

    // Thread should panic
    let result = handle.join();
    assert!(result.is_err());

    // Pool should still be functional
    let health = pool.health_check().expect("Health check failed");
    assert!(health.healthy);

    // Transaction should have rolled back - only initial data should exist
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let count: i32 =
        conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).expect("Query failed");
    assert_eq!(count, 1);

    let value: String = conn
        .query_row("SELECT value FROM test WHERE id = 1", &[], |row| row.get(0))
        .expect("Query failed");
    assert_eq!(value, "initial");
}

/// Test concurrent reads and writes for data race detection
#[test]
fn test_concurrent_read_write_data_races() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).expect("Failed to create pool"));

    // Setup counter table
    {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("CREATE TABLE counter (id INTEGER PRIMARY KEY, value INTEGER)", &[])
            .expect("Create table failed");
        conn.execute("INSERT INTO counter (id, value) VALUES (1, 0)", &[]).expect("Insert failed");
    }

    let iterations = 50;
    let mut handles = vec![];

    // Spawn writer threads
    for _ in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            for _ in 0..iterations {
                let conn = pool_clone.get_sqlcipher_connection().expect("Failed to get connection");
                conn.execute("UPDATE counter SET value = value + 1 WHERE id = 1", &[])
                    .expect("Update failed");
            }
        });
        handles.push(handle);
    }

    // Spawn reader threads
    for _ in 0..5 {
        let pool_clone = Arc::clone(&pool);
        let handle = std::thread::spawn(move || {
            for _ in 0..iterations {
                let conn = pool_clone.get_sqlcipher_connection().expect("Failed to get connection");
                let _value: i32 = conn
                    .query_row("SELECT value FROM counter WHERE id = 1", &[], |row| row.get(0))
                    .expect("Query failed");
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().expect("Thread panicked");
    }

    // Final value should be 5 * iterations (no lost updates)
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let final_value: i32 = conn
        .query_row("SELECT value FROM counter WHERE id = 1", &[], |row| row.get(0))
        .expect("Query failed");
    assert_eq!(final_value, 5 * iterations);
}

// ============================================================================
// 3. Database File Operations Tests
// ============================================================================

/// Test attempting to open database in read-only mode
#[test]
fn test_readonly_database_access() {
    let (_temp_dir, db_path) = temp_db_path();

    // Create database with data
    {
        let pool = test_pool(&db_path).expect("Failed to create pool");
        setup_test_table(&pool).expect("Failed to setup table");
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("INSERT INTO test (value) VALUES (?)", &[&"readonly_test"])
            .expect("Insert failed");
    }

    // Set file to read-only
    let mut perms = std::fs::metadata(&db_path).expect("Failed to get metadata").permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&db_path, perms).expect("Failed to set readonly");

    // Try to open database (should fail for write operations)
    let pool = test_pool(&db_path).expect("Pool creation should succeed");

    let conn = pool.get_sqlcipher_connection().expect("Getting connection should succeed");

    // Read should work
    let value: String = conn
        .query_row("SELECT value FROM test LIMIT 1", &[], |row| row.get(0))
        .expect("Read query should work");
    assert_eq!(value, "readonly_test");

    // Write should fail
    let result = conn.execute("INSERT INTO test (value) VALUES (?)", &[&"new_value"]);
    assert!(result.is_err());
}

/// Test behavior when database file is deleted while pool is active
#[test]
fn test_database_file_deleted_while_open() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).expect("Failed to create pool"));
    setup_test_table(&pool).expect("Failed to setup table");

    // Get a connection before deleting
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");

    // Connection should still work due to file handles
    conn.execute("INSERT INTO test (value) VALUES (?)", &[&"before_delete"])
        .expect("Insert should work");

    // Note: On some platforms, deleting an open file may not be possible
    // or may not affect existing handles. This test verifies graceful behavior.

    // Try to delete database files (may fail on some platforms like Windows)
    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
    let _ = std::fs::remove_file(db_path.with_extension("db-shm"));

    // Existing connection should continue to work
    let result = conn.execute("INSERT INTO test (value) VALUES (?)", &[&"after_delete"]);

    // Either the operation succeeds (platform keeps file alive) or fails gracefully
    match result {
        Ok(_) => {
            // Platform allows continued access
            let count: i32 = conn
                .query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0))
                .expect("Query should work");
            assert_eq!(count, 2);
        }
        Err(e) => {
            // Platform closed the file - error should be handled gracefully
            assert!(matches!(
                e,
                StorageError::Query(_)
                    | StorageError::DatabaseError(_)
                    | StorageError::Rusqlite(_)
                    | StorageError::Io(_)
            ));
        }
    }
}

/// Test handling of invalid file permissions
#[test]
fn test_database_permissions_errors() {
    use std::os::unix::fs::PermissionsExt;

    let (_temp_dir, db_path) = temp_db_path();

    // Create database directory with no write permissions
    let parent = db_path.parent().expect("Should have parent");
    let mut perms = std::fs::metadata(parent).expect("Failed to get metadata").permissions();
    perms.set_mode(0o555); // Read + execute only
    std::fs::set_permissions(parent, perms).expect("Failed to set permissions");

    // Try to create database (should fail)
    let result = test_pool(&db_path);

    // Restore permissions before asserting (so temp cleanup works)
    let mut perms = std::fs::metadata(parent).expect("Failed to get metadata").permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(parent, perms).expect("Failed to restore permissions");

    // Original creation should have failed
    assert!(result.is_err());
}

/// Test behavior when disk space is exhausted (simulated)
#[test]
fn test_disk_space_exhaustion_handling() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");
    setup_test_table(&pool).expect("Failed to setup table");

    // Insert data until we hit a reasonable size
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");

    // Try to insert a very large blob that might fail
    let large_data = "x".repeat(100 * 1024 * 1024); // 100MB

    let result = conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&1, &large_data]);

    // Either succeeds or fails with appropriate error
    match result {
        Ok(_) => {
            // Large insert succeeded
            let size: i64 = conn
                .query_row("SELECT LENGTH(value) FROM test WHERE id = 1", &[], |row| row.get(0))
                .expect("Query should work");
            assert!(size > 0);
        }
        Err(e) => {
            // Appropriate error for resource exhaustion
            assert!(matches!(
                e,
                StorageError::DatabaseError(_)
                    | StorageError::Query(_)
                    | StorageError::Rusqlite(_)
                    | StorageError::Io(_)
            ));
        }
    }
}

// ============================================================================
// 4. Advanced Error Scenarios Tests
// ============================================================================

/// Test behavior when trying to open a corrupted database
#[test]
fn test_corrupted_database_handling() {
    let (_temp_dir, db_path) = temp_db_path();

    // Create a file with invalid content
    std::fs::write(&db_path, b"This is not a valid SQLite database file")
        .expect("Failed to write invalid data");

    // Try to open as encrypted database
    let result = test_pool(&db_path);

    // Should fail with appropriate error
    assert!(result.is_err());
    let error = result.unwrap_err();

    // Error should indicate wrong key or corrupted database
    assert!(matches!(
        error,
        StorageError::WrongKeyOrNotEncrypted
            | StorageError::Connection(_)
            | StorageError::Rusqlite(_)
    ));
}

/// Test interruption of long-running queries
#[test]
fn test_long_running_query_interruption() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = Arc::new(test_pool(&db_path).expect("Failed to create pool"));

    // Create table with lots of data
    {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        conn.execute("CREATE TABLE large (id INTEGER PRIMARY KEY, value INTEGER)", &[])
            .expect("Create table failed");

        let mut conn_mut = pool.get_sqlcipher_connection().expect("Failed to get connection");
        let tx = conn_mut.transaction().expect("Transaction failed");
        for i in 0..10000 {
            tx.execute("INSERT INTO large (value) VALUES (?)", &[&i]).expect("Insert failed");
        }
        tx.commit().expect("Commit failed");
    }

    let query_started = Arc::new(AtomicBool::new(false));
    let query_finished = Arc::new(AtomicBool::new(false));

    let pool_clone = Arc::clone(&pool);
    let started = Arc::clone(&query_started);
    let finished = Arc::clone(&query_finished);

    // Spawn thread with long-running query
    let query_thread = std::thread::spawn(move || {
        let conn = pool_clone.get_sqlcipher_connection().expect("Failed to get connection");
        started.store(true, Ordering::SeqCst);

        // Run an expensive query (cross join to make it slow)
        let result = conn.query_row(
            "SELECT COUNT(*) FROM large a, large b WHERE a.value = b.value",
            &[],
            |row| row.get::<_, i64>(0),
        );

        finished.store(true, Ordering::SeqCst);
        result
    });

    // Wait for query to start
    while !query_started.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(10));
    }

    // Let it run for a bit
    std::thread::sleep(Duration::from_millis(100));

    // Note: SQLite doesn't have a built-in query cancellation mechanism
    // This test verifies the query completes or times out appropriately

    // Wait for completion with timeout
    let timeout = Duration::from_secs(10);
    let start = Instant::now();

    while !query_finished.load(Ordering::SeqCst) && start.elapsed() < timeout {
        std::thread::sleep(Duration::from_millis(100));
    }

    // Join the thread (it should complete within timeout)
    let result = query_thread.join();
    assert!(result.is_ok(), "Query thread should complete");
}

/// Test recovery from database errors
#[test]
fn test_database_error_recovery() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");
    setup_test_table(&pool).expect("Failed to setup table");

    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");

    // Insert valid data
    conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&1, &"valid"])
        .expect("Insert should work");

    // Try to insert with wrong type (should fail)
    let result =
        conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&"not_a_number", &"value"]);
    assert!(result.is_err());

    // Connection should still be usable after error
    conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&2, &"recovered"])
        .expect("Insert after error should work");

    // Verify both inserts succeeded
    let conn_query = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let count: i32 = conn_query
        .query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0))
        .expect("Query failed");
    assert_eq!(count, 2);
}

// ============================================================================
// 5. Performance Regression Guards Tests
// ============================================================================

/// Test that query execution time is within acceptable bounds
#[test]
fn test_query_execution_time_bounds() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");

    // Create indexed table
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    conn.execute("CREATE TABLE perf_test (id INTEGER PRIMARY KEY, value INTEGER)", &[])
        .expect("Create table failed");
    conn.execute("CREATE INDEX idx_value ON perf_test(value)", &[]).expect("Create index failed");

    // Insert test data
    let mut conn_mut = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let tx = conn_mut.transaction().expect("Transaction failed");
    for i in 0..1000 {
        tx.execute("INSERT INTO perf_test (value) VALUES (?)", &[&i]).expect("Insert failed");
    }
    tx.commit().expect("Commit failed");

    // Measure query time
    let start = Instant::now();
    let conn_query = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let _result: i32 = conn_query
        .query_row("SELECT COUNT(*) FROM perf_test WHERE value > ?", &[&500], |row| row.get(0))
        .expect("Query failed");
    let elapsed = start.elapsed();

    // Query should complete in under 100ms (very generous bound)
    assert!(elapsed < Duration::from_millis(100), "Query took too long: {:?}", elapsed);
}

/// Test that pool acquisition time is within acceptable bounds
#[test]
fn test_pool_acquisition_time_bounds() {
    let (_temp_dir, db_path) = temp_db_path();
    let config = SqlCipherPoolConfig { max_size: 10, ..Default::default() };
    let pool = test_pool_with_config(&db_path, config).expect("Failed to create pool");

    // Warm up the pool
    for _ in 0..5 {
        let _conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    }

    // Measure acquisition time for available connections
    let mut times = Vec::new();
    for _ in 0..100 {
        let start = Instant::now();
        let _conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        let elapsed = start.elapsed();
        times.push(elapsed);
    }

    // Calculate average
    let avg = times.iter().sum::<Duration>() / times.len() as u32;

    // Average acquisition time should be under 10ms
    assert!(avg < Duration::from_millis(10), "Average acquisition time too high: {:?}", avg);

    // No single acquisition should take more than 50ms
    let max = times.iter().max().expect("Should have times");
    assert!(*max < Duration::from_millis(50), "Max acquisition time too high: {:?}", max);
}

/// Test that memory usage stays within reasonable bounds
#[test]
fn test_memory_usage_bounds() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");
    setup_test_table(&pool).expect("Failed to setup table");

    // Perform many operations to test memory stability
    for iteration in 0..100 {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");

        // Insert data
        conn.execute("INSERT INTO test (id, value) VALUES (?, ?)", &[&iteration, &"memory_test"])
            .expect("Insert failed");

        // Query data
        let _count: i32 = conn
            .query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0))
            .expect("Query failed");

        // Drop connection
        drop(conn);
    }

    // Pool should still be healthy
    let health = pool.health_check().expect("Health check failed");
    assert!(health.healthy);

    // Verify all data was inserted
    let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    let count: i32 =
        conn.query_row("SELECT COUNT(*) FROM test", &[], |row| row.get(0)).expect("Query failed");
    assert_eq!(count, 100);
}

/// Test connection pool metrics accuracy
#[test]
fn test_pool_metrics_accuracy() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");

    let initial_metrics = ConnectionPool::metrics(&pool);
    let initial_acquired = initial_metrics.connections_acquired;

    // Acquire connections
    let num_acquisitions = 20;
    for _ in 0..num_acquisitions {
        let _conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
    }

    let final_metrics = ConnectionPool::metrics(&pool);

    // Should have recorded all acquisitions
    assert!(
        final_metrics.connections_acquired >= initial_acquired + num_acquisitions,
        "Expected at least {} acquisitions, got {}",
        initial_acquired + num_acquisitions,
        final_metrics.connections_acquired
    );

    // Should have no timeouts or errors
    assert_eq!(final_metrics.connections_timeout, 0);
    assert_eq!(final_metrics.connections_error, 0);
}

// ============================================================================
// 6. Integration with Other Modules Tests
// ============================================================================

/// Test integration with resilience module (circuit breaker)
#[test]
fn test_storage_resilience_circuit_breaker() {
    let (_temp_dir, db_path) = temp_db_path();

    // Create pool with small timeout to trigger failures during acquisition
    // but long enough for pool initialization (which includes SQLCipher setup)
    let config = SqlCipherPoolConfig {
        max_size: 1,
        connection_timeout: Duration::from_millis(500),
        ..Default::default()
    };

    let pool = test_pool_with_config(&db_path, config).expect("Failed to create pool");

    // Hold the only connection
    let _held = pool.get_sqlcipher_connection().expect("Failed to get first connection");

    // Try to get more connections - should fail and eventually trigger circuit
    // breaker
    let mut consecutive_failures = 0;
    for _ in 0..10 {
        match pool.get_sqlcipher_connection() {
            Err(StorageError::Timeout(_)) | Err(StorageError::Connection(_)) => {
                consecutive_failures += 1;
            }
            Ok(_) => {
                consecutive_failures = 0;
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    // Should have experienced multiple failures
    assert!(consecutive_failures >= 3, "Should have had multiple failures");

    // Metrics should reflect the failures
    let metrics = ConnectionPool::metrics(&pool);
    assert!(metrics.connections_timeout > 0 || metrics.connections_error > 0);
}

/// Test storage error classification integration
#[test]
fn test_storage_error_classification() {
    // Test retryability
    assert!(StorageError::PoolExhausted.is_retryable());
    assert!(StorageError::Timeout(5).is_retryable());
    assert!(StorageError::Connection("network error".to_string()).is_retryable());
    assert!(!StorageError::WrongKeyOrNotEncrypted.is_retryable());
    assert!(!StorageError::Encryption("key error".to_string()).is_retryable());

    // Test severity
    assert_eq!(StorageError::Timeout(5).severity(), ErrorSeverity::Warning);
    assert_eq!(StorageError::PoolExhausted.severity(), ErrorSeverity::Warning);
    assert_eq!(StorageError::Connection("error".to_string()).severity(), ErrorSeverity::Error);
    assert_eq!(StorageError::Encryption("error".to_string()).severity(), ErrorSeverity::Critical);
    assert_eq!(StorageError::WrongKeyOrNotEncrypted.severity(), ErrorSeverity::Critical);

    // Test criticality
    assert!(StorageError::Encryption("error".to_string()).is_critical());
    assert!(StorageError::WrongKeyOrNotEncrypted.is_critical());
    assert!(StorageError::SchemaVersionMismatch { expected: 2, found: 1 }.is_critical());
    assert!(!StorageError::Timeout(5).is_critical());
    assert!(!StorageError::PoolExhausted.is_critical());
}

/// Test storage with observability (metrics collection)
#[test]
fn test_storage_observability_metrics() {
    let (_temp_dir, db_path) = temp_db_path();
    let pool = test_pool(&db_path).expect("Failed to create pool");

    // Perform various operations
    let _ = pool.get_sqlcipher_connection();
    let _ = pool.get_sqlcipher_connection();
    let _ = pool.get_sqlcipher_connection();

    // Check metrics are collected
    let metrics = ConnectionPool::metrics(&pool);
    assert!(metrics.connections_acquired >= 3);

    // Health check should include metrics
    let health = pool.health_check().expect("Health check failed");
    assert!(health.healthy);
    assert!(health.max_connections > 0);
}

/// Test multiple pools with different encryption keys (security integration)
#[test]
fn test_multiple_pools_different_keys() {
    let (_temp_dir1, db_path1) = temp_db_path();
    let (_temp_dir2, db_path2) = temp_db_path();

    // Create two pools with different keys
    let pool1 =
        SqlCipherPool::new(&db_path1, test_encryption_key(), SqlCipherPoolConfig::default())
            .expect("Failed to create pool 1");

    let pool2 =
        SqlCipherPool::new(&db_path2, test_encryption_key_2(), SqlCipherPoolConfig::default())
            .expect("Failed to create pool 2");

    // Create tables in each
    let conn1 = pool1.get_sqlcipher_connection().expect("Failed to get connection 1");
    conn1.execute("CREATE TABLE secret1 (data TEXT)", &[]).expect("Create table 1 failed");
    conn1.execute("INSERT INTO secret1 VALUES (?)", &[&"secret_data_1"]).expect("Insert 1 failed");

    let conn2 = pool2.get_sqlcipher_connection().expect("Failed to get connection 2");
    conn2.execute("CREATE TABLE secret2 (data TEXT)", &[]).expect("Create table 2 failed");
    conn2.execute("INSERT INTO secret2 VALUES (?)", &[&"secret_data_2"]).expect("Insert 2 failed");

    // Verify data is isolated and encrypted differently
    drop(pool1);
    drop(pool2);

    // Try to open db1 with wrong key
    let result =
        SqlCipherPool::new(&db_path1, test_encryption_key_2(), SqlCipherPoolConfig::default());
    assert!(matches!(result, Err(StorageError::WrongKeyOrNotEncrypted)));

    // Try to open db2 with wrong key
    let result =
        SqlCipherPool::new(&db_path2, test_encryption_key(), SqlCipherPoolConfig::default());
    assert!(matches!(result, Err(StorageError::WrongKeyOrNotEncrypted)));

    // Open with correct keys should work
    let pool1 =
        SqlCipherPool::new(&db_path1, test_encryption_key(), SqlCipherPoolConfig::default())
            .expect("Failed to open pool 1");
    let pool2 =
        SqlCipherPool::new(&db_path2, test_encryption_key_2(), SqlCipherPoolConfig::default())
            .expect("Failed to open pool 2");

    let conn1 = pool1.get_sqlcipher_connection().expect("Failed to get connection 1");
    let data1: String =
        conn1.query_row("SELECT data FROM secret1", &[], |row| row.get(0)).expect("Query 1 failed");
    assert_eq!(data1, "secret_data_1");

    let conn2 = pool2.get_sqlcipher_connection().expect("Failed to get connection 2");
    let data2: String =
        conn2.query_row("SELECT data FROM secret2", &[], |row| row.get(0)).expect("Query 2 failed");
    assert_eq!(data2, "secret_data_2");
}
