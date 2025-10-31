//! Storage stress testing benchmarks
//!
//! Extreme stress tests for storage layer focusing on:
//! - Memory pressure and resource exhaustion
//! - Sustained load and endurance
//! - Lock contention and concurrent access patterns
//!
//! ## Warning
//!
//! These benchmarks are INTENSIVE and may:
//! - Consume significant memory (1GB+)
//! - Take several minutes to complete
//! - Stress system resources (CPU, disk I/O)
//! - Create large temporary databases (100MB+)
//!
//! ## Running Stress Tests
//!
//! ```bash
//! # Run all stress tests (may take 10-30 minutes)
//! cargo bench --bench storage_stress_bench -p pulsearc-common --features platform
//!
//! # Run specific stress category
//! cargo bench --bench storage_stress_bench -- memory_pressure
//! cargo bench --bench storage_stress_bench -- sustained_load
//! cargo bench --bench storage_stress_bench -- contention
//!
//! # Run with longer measurement time for more accurate results
//! cargo bench --bench storage_stress_bench -- --measurement-time 30
//! ```
//!
//! ## Expected Behavior
//!
//! - **Memory Pressure**: Should handle large datasets without crashing
//! - **Sustained Load**: Performance should remain stable over time
//! - **Contention**: Should gracefully handle lock contention and timeouts

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::{Duration, Instant};

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

fn create_stress_pool(max_size: u32, timeout_secs: u64) -> (TempDir, Arc<SqlCipherPool>) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("stress.db");

    let config = SqlCipherPoolConfig {
        max_size,
        connection_timeout: Duration::from_secs(timeout_secs),
        busy_timeout: Duration::from_secs(timeout_secs),
        ..SqlCipherPoolConfig::default()
    };

    let pool = SqlCipherPool::new(&db_path, test_key(), config).expect("Failed to create pool");

    (temp_dir, Arc::new(pool))
}

fn setup_stress_table(pool: &Arc<SqlCipherPool>) {
    let conn = pool.get_connection().expect("Failed to get connection");
    conn.execute(
        "CREATE TABLE IF NOT EXISTS stress_test (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            thread_id INTEGER NOT NULL,
            counter INTEGER NOT NULL,
            data BLOB,
            large_text TEXT,
            timestamp INTEGER NOT NULL
        )",
        &[],
    )
    .expect("Failed to create table");

    // Create index for contention tests
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_thread_counter ON stress_test(thread_id, counter)",
        &[],
    )
    .expect("Failed to create index");
}

// ============================================================================
// Memory Pressure Tests
// ============================================================================

fn bench_large_result_set(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure_large_results");
    group.sample_size(10); // Reduce samples for intensive tests

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    // Insert large dataset
    eprintln!("Setting up large dataset (100K rows)...");
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;
        for batch in 0..100 {
            for i in 0..1000 {
                let data = vec![0u8; 1024]; // 1KB per row
                let large_text = format!("Row {} in batch {}: {}", i, batch, "x".repeat(500));
                conn.execute(
                    "INSERT INTO stress_test (thread_id, counter, data, large_text, timestamp) VALUES (?, ?, ?, ?, ?)",
                    &[
                        &0i64 as &dyn ToSql,
                        &counter,
                        &data.as_slice(),
                        &large_text.as_str(),
                        &counter,
                    ],
                )
                .expect("Failed to insert");
                counter += 1;
            }
        }
    }
    eprintln!("Setup complete. Running benchmarks...");

    group.bench_function("scan_100k_rows", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        b.iter(|| {
            let count: i64 =
                conn.execute("SELECT COUNT(*) FROM stress_test", &[]).expect("Query failed") as i64;
            black_box(count);
        });
    });

    group.bench_function("fetch_all_100k_rows", |b| {
        let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
        b.iter(|| {
            let mut stmt = conn.prepare("SELECT * FROM stress_test").expect("Failed to prepare");
            let results = stmt
                .query_map(&[], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, i64>(2)?,
                        row.get::<_, Vec<u8>>(3)?,
                    ))
                })
                .expect("Query failed");

            let mut count = 0;
            for row_data in results {
                black_box(row_data);
                count += 1;
            }
            black_box(count);
        });
    });

    group.finish();
}

fn bench_large_blob_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure_large_blobs");
    group.sample_size(10);

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    let blob_sizes = vec![("1MB", 1024 * 1024), ("10MB", 10 * 1024 * 1024)];

    for (name, size) in blob_sizes {
        let blob = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("insert_blob", name), &blob, |b, blob| {
            let conn = pool.get_connection().expect("Failed to get connection");
            let mut counter = 0;

            b.iter(|| {
                conn.execute(
                    "INSERT INTO stress_test (thread_id, counter, data, timestamp) VALUES (?, ?, ?, ?)",
                    &[&0i64 as &dyn ToSql, &counter, &blob.as_slice(), &counter],
                )
                .expect("Insert failed");
                counter += 1;
            });
        });

        // Insert a blob for read test
        {
            let conn = pool.get_connection().expect("Failed to get connection");
            conn.execute(
                "INSERT INTO stress_test (thread_id, counter, data, timestamp) VALUES (?, ?, ?, ?)",
                &[&999i64 as &dyn ToSql, &999i64, &blob.as_slice(), &999i64],
            )
            .expect("Insert failed");
        }

        group.bench_function(BenchmarkId::new("read_blob", name), |b| {
            let conn = pool.get_sqlcipher_connection().expect("Failed to get connection");
            b.iter(|| {
                let data: Vec<u8> = conn
                    .query_row(
                        "SELECT data FROM stress_test WHERE thread_id = 999 AND counter = 999",
                        &[],
                        |row| row.get(0),
                    )
                    .expect("Query failed");
                black_box(data);
            });
        });
    }

    group.finish();
}

fn bench_connection_pool_exhaustion(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure_pool_exhaustion");
    group.sample_size(10);

    // Small pool to force exhaustion
    let (_temp_dir, pool) = create_stress_pool(3, 5);
    setup_stress_table(&pool);

    group.bench_function("hold_all_connections", |b| {
        b.iter(|| {
            // Acquire all connections
            let conn1 = pool.get_connection().expect("Failed to get connection 1");
            let conn2 = pool.get_connection().expect("Failed to get connection 2");
            let conn3 = pool.get_connection().expect("Failed to get connection 3");

            // Do some work
            for i in 0..10 {
                conn1
                    .execute(
                        "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                        &[&1i64 as &dyn ToSql, &i, &i],
                    )
                    .expect("Insert failed");
            }

            black_box((conn1, conn2, conn3));
            // Connections released on drop
        });
    });

    group.bench_function("parallel_acquisition_stress", |b| {
        b.iter(|| {
            let mut handles = vec![];

            for t in 0..10 {
                let pool_clone = Arc::clone(&pool);
                let handle = thread::spawn(move || {
                    // Each thread tries to get a connection (pool has only 3)
                    match pool_clone.get_connection() {
                        Ok(conn) => {
                            conn.execute(
                                "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                                &[&(t as i64) as &dyn ToSql, &0i64, &0i64],
                            )
                            .ok();
                        }
                        Err(_) => {
                            // Expected - pool exhaustion
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().ok();
            }
        });
    });

    group.finish();
}

fn bench_massive_batch_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_pressure_massive_batch");
    group.sample_size(10);

    let (_temp_dir, pool) = create_stress_pool(10, 60);
    setup_stress_table(&pool);

    for batch_size in [10_000, 50_000] {
        group.throughput(Throughput::Elements(batch_size as u64));

        group.bench_with_input(
            BenchmarkId::new("transactional_insert", batch_size),
            &batch_size,
            |b, &batch_size| {
                let mut conn = pool.get_sqlcipher_connection().expect("Failed to get connection");

                b.iter(|| {
                    let tx = conn.transaction().expect("Failed to begin transaction");
                    for i in 0..batch_size {
                        let data = vec![0u8; 128];
                        tx.execute(
                            "INSERT INTO stress_test (thread_id, counter, data, timestamp) VALUES (?, ?, ?, ?)",
                            &[&0i64 as &dyn ToSql, &(i as i64), &data.as_slice(), &(i as i64)],
                        )
                        .expect("Insert failed");
                    }
                    tx.commit().expect("Commit failed");
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Sustained Load Tests
// ============================================================================

fn bench_sustained_write_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load_writes");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30)); // Longer measurement

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    group.bench_function("continuous_writes_30s", |b| {
        let conn = pool.get_connection().expect("Failed to get connection");
        let mut counter = 0;

        b.iter(|| {
            // Continuous writes
            let start = Instant::now();
            let mut ops = 0;

            while start.elapsed() < Duration::from_millis(100) {
                let data = vec![0u8; 256];
                conn.execute(
                    "INSERT INTO stress_test (thread_id, counter, data, timestamp) VALUES (?, ?, ?, ?)",
                    &[&0i64 as &dyn ToSql, &counter, &data.as_slice(), &counter],
                )
                .expect("Insert failed");
                counter += 1;
                ops += 1;
            }

            black_box(ops);
        });
    });

    group.finish();
}

fn bench_sustained_mixed_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load_mixed");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    // Pre-populate data
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        for i in 0..10_000 {
            conn.execute(
                "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                &[&0i64 as &dyn ToSql, &i, &i],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("mixed_read_write_30s", |b| {
        let pool_clone = Arc::clone(&pool);
        let mut counter = 10_000;

        b.iter(|| {
            let start = Instant::now();
            let mut ops = 0;

            while start.elapsed() < Duration::from_millis(100) {
                let conn = pool_clone.get_connection().expect("Failed to get connection");

                // 70% reads, 30% writes
                if ops % 10 < 7 {
                    // Read
                    let id = (ops % 10_000) + 1;
                    conn.execute("SELECT * FROM stress_test WHERE id = ?", &[&id as &dyn ToSql])
                        .ok();
                } else {
                    // Write
                    conn.execute(
                        "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                        &[&0i64 as &dyn ToSql, &counter, &counter],
                    )
                    .ok();
                    counter += 1;
                }

                ops += 1;
            }

            black_box(ops);
        });
    });

    group.finish();
}

fn bench_sustained_concurrent_load(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load_concurrent");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    group.bench_function("8_threads_sustained_30s", |b| {
        b.iter(|| {
            let stop = Arc::new(AtomicBool::new(false));
            let ops_counter = Arc::new(AtomicU64::new(0));
            let mut handles = vec![];

            for t in 0..8 {
                let pool_clone = Arc::clone(&pool);
                let stop_clone = Arc::clone(&stop);
                let ops_clone = Arc::clone(&ops_counter);

                let handle = thread::spawn(move || {
                    let conn = pool_clone.get_connection().expect("Failed to get connection");
                    let mut local_counter = 0;

                    while !stop_clone.load(Ordering::Relaxed) {
                        conn.execute(
                            "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                            &[&(t as i64) as &dyn ToSql, &local_counter, &local_counter],
                        )
                        .ok();

                        local_counter += 1;
                        ops_clone.fetch_add(1, Ordering::Relaxed);
                    }
                });
                handles.push(handle);
            }

            // Run for a short duration
            thread::sleep(Duration::from_millis(100));
            stop.store(true, Ordering::Relaxed);

            for handle in handles {
                handle.join().ok();
            }

            let total_ops = ops_counter.load(Ordering::Relaxed);
            black_box(total_ops);
        });
    });

    group.finish();
}

fn bench_pool_stability_over_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("sustained_load_pool_stability");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    group.bench_function("connection_churn_30s", |b| {
        b.iter(|| {
            let start = Instant::now();
            let mut connections_created = 0;

            // Rapidly acquire and release connections
            while start.elapsed() < Duration::from_millis(100) {
                {
                    let _conn1 = pool.get_connection().expect("Failed to get connection");
                    let _conn2 = pool.get_connection().expect("Failed to get connection");
                    let _conn3 = pool.get_connection().expect("Failed to get connection");
                    connections_created += 3;
                }
                // Connections dropped and returned to pool
            }

            black_box(connections_created);
        });
    });

    group.bench_function("health_check_under_load", |b| {
        // Start background load
        let pool_clone = Arc::clone(&pool);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_clone = Arc::clone(&stop);

        let load_handle = thread::spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                if let Ok(conn) = pool_clone.get_connection() {
                    conn.execute(
                        "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                        &[&0i64 as &dyn ToSql, &0i64, &0i64],
                    )
                    .ok();
                }
            }
        });

        b.iter(|| {
            // Check health while under load
            let health = pool.health_check().expect("Health check failed");
            black_box(health);
        });

        stop.store(true, Ordering::Relaxed);
        load_handle.join().ok();
    });

    group.finish();
}

// ============================================================================
// Contention & Lock Stress Tests
// ============================================================================

fn bench_hot_row_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention_hot_row");
    group.sample_size(10);

    let (_temp_dir, pool) = create_stress_pool(20, 30);
    setup_stress_table(&pool);

    // Create a hot row that everyone will update
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        conn.execute(
            "INSERT INTO stress_test (id, thread_id, counter, timestamp) VALUES (?, ?, ?, ?)",
            &[&1i64 as &dyn ToSql, &0i64, &0i64, &0i64],
        )
        .expect("Insert failed");
    }

    for thread_count in [4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("concurrent_updates", thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let barrier = Arc::new(Barrier::new(thread_count));
                    let mut handles = vec![];

                    for _ in 0..thread_count {
                        let pool_clone = Arc::clone(&pool);
                        let barrier_clone = Arc::clone(&barrier);

                        let handle = thread::spawn(move || {
                            // Wait for all threads to be ready
                            barrier_clone.wait();

                            // All threads try to update the same row simultaneously
                            let conn =
                                pool_clone.get_connection().expect("Failed to get connection");
                            for _ in 0..10 {
                                conn.execute(
                                    "UPDATE stress_test SET counter = counter + 1 WHERE id = 1",
                                    &[],
                                )
                                .ok(); // May fail due to contention
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().ok();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_write_write_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention_write_write");
    group.sample_size(10);

    let (_temp_dir, pool) = create_stress_pool(20, 30);
    setup_stress_table(&pool);

    for thread_count in [4, 8, 16] {
        group.bench_with_input(
            BenchmarkId::new("parallel_inserts", thread_count),
            &thread_count,
            |b, &thread_count| {
                let counter = Arc::new(AtomicU64::new(0));

                b.iter(|| {
                    let barrier = Arc::new(Barrier::new(thread_count));
                    let mut handles = vec![];

                    for t in 0..thread_count {
                        let pool_clone = Arc::clone(&pool);
                        let barrier_clone = Arc::clone(&barrier);
                        let counter_clone = Arc::clone(&counter);

                        let handle = thread::spawn(move || {
                            barrier_clone.wait();

                            let conn = pool_clone.get_connection().expect("Failed to get connection");
                            for _ in 0..50 {
                                let count = counter_clone.fetch_add(1, Ordering::Relaxed);
                                conn.execute(
                                    "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                                    &[&(t as i64) as &dyn ToSql, &(count as i64), &(count as i64)],
                                )
                                .ok();
                            }
                        });
                        handles.push(handle);
                    }

                    for handle in handles {
                        handle.join().ok();
                    }
                });
            },
        );
    }

    group.finish();
}

fn bench_reader_writer_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention_reader_writer");
    group.sample_size(10);

    let (_temp_dir, pool) = create_stress_pool(20, 30);
    setup_stress_table(&pool);

    // Pre-populate data
    {
        let conn = pool.get_connection().expect("Failed to get connection");
        for i in 0..1000 {
            conn.execute(
                "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                &[&0i64 as &dyn ToSql, &i, &i],
            )
            .expect("Insert failed");
        }
    }

    group.bench_function("many_readers_few_writers", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(16));
            let mut handles = vec![];

            // 12 readers, 4 writers
            for t in 0..16 {
                let pool_clone = Arc::clone(&pool);
                let barrier_clone = Arc::clone(&barrier);

                let handle = thread::spawn(move || {
                    barrier_clone.wait();

                    let conn = pool_clone.get_connection().expect("Failed to get connection");

                    if t < 12 {
                        // Reader
                        for i in 0..20 {
                            let id = (i % 1000) + 1;
                            conn.execute(
                                "SELECT * FROM stress_test WHERE id = ?",
                                &[&id as &dyn ToSql],
                            )
                            .ok();
                        }
                    } else {
                        // Writer
                        for i in 0..20 {
                            conn.execute(
                                "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                                &[&(t as i64) as &dyn ToSql, &i, &i],
                            )
                            .ok();
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().ok();
            }
        });
    });

    group.finish();
}

fn bench_transaction_contention(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention_transactions");
    group.sample_size(10);

    let (_temp_dir, pool) = create_stress_pool(10, 30);
    setup_stress_table(&pool);

    group.bench_function("concurrent_transactions", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(8));
            let mut handles = vec![];

            for t in 0..8 {
                let pool_clone = Arc::clone(&pool);
                let barrier_clone = Arc::clone(&barrier);

                let handle = thread::spawn(move || {
                    barrier_clone.wait();

                    let mut conn =
                        pool_clone.get_sqlcipher_connection().expect("Failed to get connection");

                    // Start transaction in its own scope
                    {
                        let tx_result = conn.transaction();
                        if let Ok(tx) = tx_result {
                            for i in 0..10 {
                                tx.execute(
                                    "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                                    &[&(t as i64) as &dyn ToSql, &i, &i],
                                )
                                .ok();
                            }
                            tx.commit().ok();
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().ok();
            }
        });
    });

    group.finish();
}

fn bench_lock_timeout_scenarios(c: &mut Criterion) {
    let mut group = c.benchmark_group("contention_lock_timeouts");
    group.sample_size(10);

    // Small timeout to force timeouts
    let (_temp_dir, pool) = create_stress_pool(5, 1);
    setup_stress_table(&pool);

    group.bench_function("forced_timeout_scenario", |b| {
        b.iter(|| {
            let barrier = Arc::new(Barrier::new(10));
            let mut handles = vec![];
            let success_count = Arc::new(AtomicU64::new(0));
            let failure_count = Arc::new(AtomicU64::new(0));

            for t in 0..10 {
                let pool_clone = Arc::clone(&pool);
                let barrier_clone = Arc::clone(&barrier);
                let success_clone = Arc::clone(&success_count);
                let failure_clone = Arc::clone(&failure_count);

                let handle = thread::spawn(move || {
                    barrier_clone.wait();

                    match pool_clone.get_connection() {
                        Ok(conn) => {
                            // Hold connection for a bit
                            for i in 0..5 {
                                conn.execute(
                                    "INSERT INTO stress_test (thread_id, counter, timestamp) VALUES (?, ?, ?)",
                                    &[&(t as i64) as &dyn ToSql, &i, &i],
                                )
                                .ok();
                            }
                            success_clone.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(_) => {
                            failure_clone.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.join().ok();
            }

            let successes = success_count.load(Ordering::Relaxed);
            let failures = failure_count.load(Ordering::Relaxed);
            black_box((successes, failures));
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups Registration
// ============================================================================

criterion_group!(
    memory_pressure,
    bench_large_result_set,
    bench_large_blob_storage,
    bench_connection_pool_exhaustion,
    bench_massive_batch_operations,
);

criterion_group!(
    sustained_load,
    bench_sustained_write_load,
    bench_sustained_mixed_load,
    bench_sustained_concurrent_load,
    bench_pool_stability_over_time,
);

criterion_group!(
    contention,
    bench_hot_row_updates,
    bench_write_write_contention,
    bench_reader_writer_contention,
    bench_transaction_contention,
    bench_lock_timeout_scenarios,
);

criterion_main!(memory_pressure, sustained_load, contention);
