//! Comprehensive storage benchmarks
//!
//! Benchmarks for storage operations including SQLCipher pool management,
//! connection acquisition, query execution, transactions, concurrent access,
//! and real-world scenarios.
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all storage benchmarks
//! cargo bench --bench storage_bench -p pulsearc-common --features platform
//!
//! # Run specific benchmark group
//! cargo bench --bench storage_bench -- pool_operations
//! cargo bench --bench storage_bench -- query_performance
//! cargo bench --bench storage_bench -- concurrent_access
//!
//! # Save baseline for comparison
//! cargo bench --bench storage_bench -- --save-baseline master
//!
//! # Compare against baseline
//! cargo bench --bench storage_bench -- --baseline master
//! ```
//!
//! ## Expected Performance Characteristics
//!
//! - **Pool Creation**: ~10-50ms (encryption key setup)
//! - **Connection Acquisition**: ~0.1-1ms (from pool)
//! - **Simple Query**: ~0.1-1ms
//! - **Prepared Statement**: ~0.05-0.5ms per execution
//! - **Transaction Commit**: ~1-5ms (WAL mode)
//! - **Batch Insert (100 rows)**: ~10-50ms

use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::storage::{ConnectionPool, SqlCipherPool, SqlCipherPoolConfig};
use rusqlite::ToSql;
use tempfile::TempDir;

// ============================================================================
// Test Utilities
// ============================================================================

fn test_key() -> String {
    "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string()
}

fn create_test_pool(max_size: u32) -> (TempDir, Arc<SqlCipherPool>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("bench.db");

    let config = SqlCipherPoolConfig {
        max_size,
        connection_timeout: Duration::from_secs(5),
        ..SqlCipherPoolConfig::default()
    };

    let pool = SqlCipherPool::new(&db_path, test_key(), config).expect("Failed to create pool");

    (temp_dir, Arc::new(pool))
}

fn setup_test_table(pool: &Arc<SqlCipherPool>) {
    let conn = pool.get_connection().expect("Failed to get connection");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS test_data (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            value INTEGER NOT NULL,
            timestamp INTEGER NOT NULL,
            metadata TEXT
        )",
        &[],
    )
    .expect("Failed to create table");
}

fn setup_large_test_table(pool: &Arc<SqlCipherPool>, rows: usize) {
    setup_test_table(pool);
    let conn = pool.get_connection().expect("Failed to get connection");

    for i in 0..rows {
        let name = format!("test_row_{}", i);
        let value = i as i64;
        let timestamp = i as i64;
        let metadata = format!("metadata_{}", i);
        conn.execute(
            "INSERT INTO test_data (name, value, timestamp, metadata) VALUES (?, ?, ?, ?)",
            &[&name as &dyn ToSql, &value, &timestamp, &metadata],
        )
        .expect("Failed to insert row");
    }
}

// ============================================================================
// Pool Creation and Configuration Benchmarks
// ============================================================================

fn bench_pool_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_creation");

    for max_size in [5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("create_pool", max_size),
            &max_size,
            |b, &max_size| {
                b.iter_batched(
                    || {
                        let temp_dir = TempDir::new().expect("Failed to create temp dir");
                        let db_path = temp_dir.path().join("bench.db");
                        (temp_dir, db_path)
                    },
                    |(_temp_dir, db_path)| {
                        let config = SqlCipherPoolConfig {
                            max_size: black_box(max_size),
                            ..SqlCipherPoolConfig::default()
                        };
                        let pool = SqlCipherPool::new(&db_path, test_key(), config)
                            .expect("Pool creation failed");
                        black_box(pool);
                    },
                    criterion::BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

fn bench_pool_with_existing_database(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_existing_database");

    group.bench_function("reopen_encrypted_db", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let db_path = temp_dir.path().join("bench.db");

                // Create and populate database
                {
                    let config = SqlCipherPoolConfig::default();
                    let pool = SqlCipherPool::new(&db_path, test_key(), config)
                        .expect("Failed to create initial pool");
                    let conn = pool.get_connection().expect("Failed to get connection");
                    conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", &[])
                        .expect("Failed to create table");
                }

                (temp_dir, db_path)
            },
            |(_temp_dir, db_path)| {
                let config = SqlCipherPoolConfig::default();
                let pool = SqlCipherPool::new(&db_path, test_key(), config)
                    .expect("Failed to reopen pool");
                black_box(pool);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// Connection Acquisition Benchmarks
// ============================================================================

fn bench_connection_acquisition(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_acquisition");

    for max_size in [5, 10, 20] {
        let (_temp_dir, pool) = create_test_pool(max_size);

        group.bench_with_input(BenchmarkId::new("get_connection", max_size), &pool, |b, pool| {
            b.iter(|| {
                let conn = pool.get_connection().expect("Failed to get connection");
                black_box(conn);
                // Connection is automatically returned to pool on drop
            });
        });
    }

    group.finish();
}

fn bench_connection_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("connection_reuse");

    let (_temp_dir, pool) = create_test_pool(10);

    group.bench_function("sequential_acquire_release", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let conn = pool.get_connection().expect("Failed to get connection");
                black_box(conn);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Query Performance Benchmarks
// ============================================================================

fn bench_simple_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("simple_queries");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_test_table(&pool);

    group.bench_function("insert_single_row", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            let name = format!("test_{}", counter);
            let value = counter;
            let timestamp = counter;
            conn.execute(
                "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                &[&name as &dyn ToSql, &value, &timestamp],
            )
            .expect("Insert failed");
            counter += 1;
        });
    });

    // Insert some data for SELECT benchmarks
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        for i in 0..1000 {
            let name = format!("test_{}", i);
            conn.execute(
                "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                &[&name as &dyn ToSql, &(i as i64), &(i as i64)],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("select_single_row_by_id", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 1;

        b.iter(|| {
            let id = (counter % 1000) + 1;
            conn.execute("SELECT * FROM test_data WHERE id = ?", &[&id as &dyn ToSql])
                .expect("Select failed");
            counter += 1;
        });
    });

    group.bench_function("count_all_rows", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute("SELECT COUNT(*) FROM test_data", &[]).expect("Count failed");
        });
    });

    group.bench_function("update_single_row", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 1;

        b.iter(|| {
            let id = (counter % 1000) + 1;
            let new_value = counter;
            conn.execute(
                "UPDATE test_data SET value = ? WHERE id = ?",
                &[&new_value as &dyn ToSql, &id],
            )
            .expect("Update failed");
            counter += 1;
        });
    });

    group.bench_function("delete_single_row", |b| {
        b.iter_batched(
            || {
                // Setup: Insert a row to delete
                let conn = pool.get_connection().expect("Failed to get connection");
                conn.execute(
                    "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                    &[&"temp" as &dyn ToSql, &999i64, &999i64],
                )
                .expect("Insert failed");
                // Get the last inserted row id
                let id: i64 = conn
                    .execute("SELECT last_insert_rowid()", &[])
                    .expect("Failed to get last id") as i64;
                id
            },
            |id| {
                let conn = pool.get_connection().expect("Failed to get connection");
                conn.execute("DELETE FROM test_data WHERE id = ?", &[&id as &dyn ToSql])
                    .expect("Delete failed");
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_prepared_statements(c: &mut Criterion) {
    let mut group = c.benchmark_group("prepared_statements");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_test_table(&pool);

    group.throughput(Throughput::Elements(1));

    group.bench_function("prepared_insert", |b| {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        let mut stmt = conn
            .prepare("INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)")
            .expect("Failed to prepare statement");
        let mut counter = 0;

        b.iter(|| {
            let name = format!("test_{}", counter);
            let value = counter;
            let timestamp = counter;
            stmt.execute(&[&name as &dyn ToSql, &value, &timestamp]).expect("Execute failed");
            counter += 1;
        });
    });

    // Insert data for SELECT benchmarks
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        for i in 0..1000 {
            let name = format!("test_{}", i);
            conn.execute(
                "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                &[&name as &dyn ToSql, &(i as i64), &(i as i64)],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("prepared_select", |b| {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        let mut stmt = conn
            .prepare("SELECT * FROM test_data WHERE id = ?")
            .expect("Failed to prepare statement");
        let mut counter = 1;

        b.iter(|| {
            let id = (counter % 1000) + 1;
            let results = stmt
                .query_map(&[&id as &dyn ToSql], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?))
                })
                .expect("Query failed");
            for row_data in results {
                black_box(row_data);
            }
            counter += 1;
        });
    });

    group.finish();
}

fn bench_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_operations");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_test_table(&pool);

    for batch_size in [10, 100, 1000] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("batch_insert", batch_size),
            &batch_size,
            |b, &batch_size| {
                let conn = pool.get_connection().expect("Failed to get connection");
                let mut counter = 0;

                b.iter(|| {
                    for _ in 0..batch_size {
                        let name = format!("test_{}", counter);
                        let value = counter;
                        let timestamp = counter;
                        conn.execute(
                            "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                            &[&name as &dyn ToSql, &value, &timestamp],
                        )
                        .expect("Insert failed");
                        counter += 1;
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Transaction Performance Benchmarks
// ============================================================================

fn bench_transactions(c: &mut Criterion) {
    let mut group = c.benchmark_group("transactions");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_test_table(&pool);

    group.bench_function("empty_transaction", |b| {
        let mut conn = pool.get_sqlcipher_connection().expect("Failed to get connection");

        b.iter(|| {
            let tx = conn.transaction().expect("Failed to begin transaction");
            tx.commit().expect("Failed to commit");
        });
    });

    for tx_size in [10, 100] {
        group.throughput(Throughput::Elements(tx_size as u64));

        group.bench_with_input(
            BenchmarkId::new("transaction_insert", tx_size),
            &tx_size,
            |b, &tx_size| {
                let mut conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
                let mut counter = 0;

                b.iter(|| {
                    let tx = conn.transaction().expect("Failed to begin transaction");
                    for _ in 0..tx_size {
                        let name = format!("test_{}", counter);
                        let value = counter;
                        let timestamp = counter;
                        tx.execute(
                            "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                            &[&name as &dyn ToSql, &value, &timestamp],
                        )
                        .expect("Insert failed");
                        counter += 1;
                    }
                    tx.commit().expect("Failed to commit");
                });
            },
        );
    }

    group.bench_function("transaction_rollback", |b| {
        let mut conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            let tx = conn.transaction().expect("Failed to begin transaction");
            let name = format!("test_{}", counter);
            tx.execute(
                "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                &[&name as &dyn ToSql, &counter, &counter],
            )
            .expect("Insert failed");
            tx.rollback().expect("Failed to rollback");
            counter += 1;
        });
    });

    group.finish();
}

// ============================================================================
// Concurrent Access Benchmarks
// ============================================================================

fn bench_concurrent_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_reads");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_large_test_table(&pool, 1000);

    for thread_count in [2, 4, 8] {
        group.throughput(Throughput::Elements(100 * thread_count as u64));

        group.bench_with_input(
            BenchmarkId::new("parallel_reads", thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let mut handles = vec![];

                    for t in 0..thread_count {
                        let pool_clone = Arc::clone(&pool);
                        let handle = std::thread::spawn(move || {
                            let conn =
                                pool_clone.get_connection().expect("Failed to get connection");
                            for i in 0..100 {
                                let id = ((t * 100 + i) % 1000) + 1;
                                conn.execute(
                                    "SELECT * FROM test_data WHERE id = ?",
                                    &[&id as &dyn ToSql],
                                )
                                .expect("Select failed");
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().expect("Thread panicked");
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_writes(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_writes");

    // Reduce thread count for writes due to SQLite serialization
    for thread_count in [2, 4] {
        let (_temp_dir, pool) = create_test_pool(thread_count * 2);
        setup_test_table(&pool);

        group.throughput(Throughput::Elements(50 * thread_count as u64));

        group.bench_with_input(
            BenchmarkId::new("parallel_writes", thread_count),
            &thread_count,
            |b, &thread_count| {
                let counter = Arc::new(std::sync::atomic::AtomicI64::new(0));

                b.iter(|| {
                    let mut handles = vec![];

                    for _ in 0..thread_count {
                        let pool_clone = Arc::clone(&pool);
                        let counter_clone = Arc::clone(&counter);
                        let handle = std::thread::spawn(move || {
                            let conn = pool_clone.get_connection().expect("Failed to get connection");
                            for _ in 0..50 {
                                let id =
                                    counter_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                let name = format!("test_{}", id);
                                conn.execute(
                                    "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                                    &[&name as &dyn ToSql, &id, &id],
                                )
                                .expect("Insert failed");
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().expect("Thread panicked");
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_mixed");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_large_test_table(&pool, 1000);

    for thread_count in [2, 4, 8] {
        group.throughput(Throughput::Elements(100 * thread_count as u64));

        group.bench_with_input(
            BenchmarkId::new("mixed_operations", thread_count),
            &thread_count,
            |b, &thread_count| {
                let counter = Arc::new(std::sync::atomic::AtomicI64::new(1000));

                b.iter(|| {
                    let mut handles = vec![];

                    for t in 0..thread_count {
                        let pool_clone = Arc::clone(&pool);
                        let counter_clone = Arc::clone(&counter);
                        let handle = std::thread::spawn(move || {
                            let conn = pool_clone.get_connection().expect("Failed to get connection");

                            for i in 0..100 {
                                if i % 3 == 0 {
                                    // Write
                                    let id = counter_clone
                                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                    let name = format!("test_{}", id);
                                    conn.execute(
                                        "INSERT INTO test_data (name, value, timestamp) VALUES (?, ?, ?)",
                                        &[&name as &dyn ToSql, &id, &id],
                                    )
                                    .expect("Insert failed");
                                } else {
                                    // Read
                                    let id = ((t * 100 + i) % 1000) + 1;
                                    conn.execute(
                                        "SELECT * FROM test_data WHERE id = ?",
                                        &[&id as &dyn ToSql],
                                    )
                                    .expect("Select failed");
                                }
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().expect("Thread panicked");
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Pool Stress Testing Benchmarks
// ============================================================================

fn bench_pool_exhaustion(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_exhaustion");

    // Small pool to test exhaustion scenarios
    let (_temp_dir, pool) = create_test_pool(3);
    setup_test_table(&pool);

    group.bench_function("sequential_at_capacity", |b| {
        b.iter(|| {
            // Acquire and hold all connections
            let _conn1 = pool.get_connection().expect("Failed to get connection 1");
            let _conn2 = pool.get_connection().expect("Failed to get connection 2");
            let _conn3 = pool.get_connection().expect("Failed to get connection 3");
            // All connections are returned on drop
        });
    });

    group.finish();
}

fn bench_pool_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("pool_metrics");

    let (_temp_dir, pool) = create_test_pool(10);

    group.bench_function("get_metrics", |b| {
        b.iter(|| {
            let metrics = pool.metrics();
            black_box(metrics);
        });
    });

    group.bench_function("health_check", |b| {
        b.iter(|| {
            let health = pool.health_check().expect("Health check failed");
            black_box(health);
        });
    });

    group.finish();
}

// ============================================================================
// Encryption Overhead Benchmarks
// ============================================================================

fn bench_encryption_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption_overhead");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_test_table(&pool);

    // Benchmark write performance with encryption
    for data_size in [100, 1000, 10000] {
        let metadata = "x".repeat(data_size);

        group.throughput(Throughput::Bytes(data_size as u64));

        group.bench_with_input(
            BenchmarkId::new("write_encrypted", data_size),
            &metadata,
            |b, metadata| {
                let conn = pool.get_connection().expect("Failed to get connection");
                let mut counter = 0;

                b.iter(|| {
                    let name = format!("test_{}", counter);
                    conn.execute(
                        "INSERT INTO test_data (name, value, timestamp, metadata) VALUES (?, ?, ?, ?)",
                        &[
                            &name as &dyn ToSql,
                            &counter,
                            &counter,
                            &metadata.as_str(),
                        ],
                    )
                    .expect("Insert failed");
                    counter += 1;
                });
            },
        );
    }

    // Insert data for read benchmarks
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        let large_metadata = "x".repeat(10000);
        for i in 0..100 {
            let name = format!("test_{}", i);
            conn.execute(
                "INSERT INTO test_data (name, value, timestamp, metadata) VALUES (?, ?, ?, ?)",
                &[&name as &dyn ToSql, &(i as i64), &(i as i64), &large_metadata.as_str()],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("read_encrypted_large", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 1;

        b.iter(|| {
            let id = (counter % 100) + 1;
            conn.execute("SELECT * FROM test_data WHERE id = ?", &[&id as &dyn ToSql])
                .expect("Select failed");
            counter += 1;
        });
    });

    group.finish();
}

// ============================================================================
// Complex Query Benchmarks
// ============================================================================

fn bench_complex_queries(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_queries");

    let (_temp_dir, pool) = create_test_pool(10);
    setup_large_test_table(&pool, 10000);

    group.bench_function("range_query", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute(
                "SELECT * FROM test_data WHERE value BETWEEN ? AND ?",
                &[&1000i64 as &dyn ToSql, &2000i64],
            )
            .expect("Query failed");
        });
    });

    group.bench_function("aggregate_query", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute("SELECT COUNT(*), AVG(value), MIN(value), MAX(value) FROM test_data", &[])
                .expect("Query failed");
        });
    });

    group.bench_function("like_query", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute("SELECT * FROM test_data WHERE name LIKE ?", &[&"%test_1%"])
                .expect("Query failed");
        });
    });

    group.bench_function("order_by_limit", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute("SELECT * FROM test_data ORDER BY value DESC LIMIT 100", &[])
                .expect("Query failed");
        });
    });

    group.finish();
}

// ============================================================================
// Real-World Scenario Benchmarks
// ============================================================================

fn bench_time_tracking_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_time_tracking");

    let (_temp_dir, pool) = create_test_pool(10);

    // Setup time entries table
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        conn.execute(
            "CREATE TABLE time_entries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id INTEGER NOT NULL,
                project_id INTEGER NOT NULL,
                start_time INTEGER NOT NULL,
                end_time INTEGER,
                duration INTEGER,
                description TEXT
            )",
            &[],
        )
        .expect("Failed to create table");

        // Create index for common queries
        conn.execute("CREATE INDEX idx_user_project ON time_entries(user_id, project_id)", &[])
            .expect("Failed to create index");
    }

    group.bench_function("start_timer", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            let user_id = 1;
            let project_id = (counter % 10) + 1;
            let start_time = counter;
            conn.execute(
                "INSERT INTO time_entries (user_id, project_id, start_time) VALUES (?, ?, ?)",
                &[&user_id as &dyn ToSql, &project_id, &start_time],
            )
            .expect("Insert failed");
            counter += 1;
        });
    });

    // Insert sample data
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        for i in 0..1000 {
            conn.execute(
                "INSERT INTO time_entries (user_id, project_id, start_time, end_time, duration) VALUES (?, ?, ?, ?, ?)",
                &[&1i64 as &dyn ToSql, &((i % 10) + 1), &(i * 1000), &((i * 1000) + 3600), &3600i64],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("get_user_entries", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute("SELECT * FROM time_entries WHERE user_id = ?", &[&1i64 as &dyn ToSql])
                .expect("Query failed");
        });
    });

    group.bench_function("project_summary", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");

        b.iter(|| {
            conn.execute(
                "SELECT project_id, COUNT(*), SUM(duration) FROM time_entries WHERE user_id = ? GROUP BY project_id",
                &[&1i64 as &dyn ToSql],
            )
            .expect("Query failed");
        });
    });

    group.finish();
}

fn bench_session_storage_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_session_storage");

    let (_temp_dir, pool) = create_test_pool(10);

    // Setup sessions table
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        conn.execute(
            "CREATE TABLE sessions (
                session_id TEXT PRIMARY KEY,
                user_id INTEGER NOT NULL,
                data TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL
            )",
            &[],
        )
        .expect("Failed to create table");
    }

    group.bench_function("create_session", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            let session_id = format!("session_{}", counter);
            let user_id = counter;
            let data = format!("{{\"user_id\": {}}}", counter);
            let created_at = counter;
            let expires_at = counter + 3600;
            conn.execute(
                "INSERT INTO sessions (session_id, user_id, data, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
                &[&session_id as &dyn ToSql, &user_id, &data.as_str(), &created_at, &expires_at],
            )
            .expect("Insert failed");
            counter += 1;
        });
    });

    // Insert sample sessions
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        for i in 0..1000 {
            let session_id = format!("session_{}", i);
            let data = format!("{{\"user_id\": {}}}", i);
            conn.execute(
                "INSERT INTO sessions (session_id, user_id, data, created_at, expires_at) VALUES (?, ?, ?, ?, ?)",
                &[&session_id as &dyn ToSql, &i, &data.as_str(), &i, &(i + 3600)],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("lookup_session", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            let session_id = format!("session_{}", counter % 1000);
            conn.execute(
                "SELECT * FROM sessions WHERE session_id = ?",
                &[&session_id as &dyn ToSql],
            )
            .expect("Query failed");
            counter += 1;
        });
    });

    group.bench_function("cleanup_expired", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            let current_time = counter;
            conn.execute(
                "DELETE FROM sessions WHERE expires_at < ?",
                &[&current_time as &dyn ToSql],
            )
            .expect("Delete failed");
            counter += 100; // Advance time
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups Registration
// ============================================================================

criterion_group!(
    pool_operations,
    bench_pool_creation,
    bench_pool_with_existing_database,
    bench_connection_acquisition,
    bench_connection_reuse,
);

criterion_group!(
    query_performance,
    bench_simple_queries,
    bench_prepared_statements,
    bench_batch_operations,
    bench_complex_queries,
);

criterion_group!(transactions, bench_transactions,);

criterion_group!(
    concurrent_access,
    bench_concurrent_reads,
    bench_concurrent_writes,
    bench_concurrent_mixed,
);

criterion_group!(stress_testing, bench_pool_exhaustion, bench_pool_metrics,);

criterion_group!(encryption, bench_encryption_overhead,);

criterion_group!(real_world, bench_time_tracking_scenario, bench_session_storage_scenario,);

criterion_main!(
    pool_operations,
    query_performance,
    transactions,
    concurrent_access,
    stress_testing,
    encryption,
    real_world,
);
