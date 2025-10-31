//! Comprehensive sync module benchmarks
//!
//! Benchmarks cover queue throughput, batch operations, failure handling,
//! retry strategy execution paths, and retry budget token management.
//!
//! Run with: `cargo bench --bench sync_bench -p pulsearc-common --features
//! runtime`

use std::io::Error as IoError;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::sync::{
    MockClock, Priority, QueueConfig, RetryBudget, RetryStrategy, SyncItem, SyncQueue,
};
use serde_json::json;
use tokio::runtime::Builder as RuntimeBuilder;

// ============================================================================
// Helpers
// ============================================================================

fn build_runtime() -> tokio::runtime::Runtime {
    RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for sync benchmarks")
}

// ============================================================================
// Queue Benchmarks
// ============================================================================

fn bench_queue_enqueue_dequeue(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("sync_queue_enqueue_dequeue");

    for &count in &[256usize, 1024, 4096] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::new("default_config", count), &count, |b, &count| {
            b.to_async(&runtime).iter(|| async move {
                let queue = SyncQueue::new();

                for idx in 0..count {
                    let payload = json!({ "index": idx, "variant": "default" });
                    let item = SyncItem::with_id(format!("item-{idx}"), payload, Priority::Normal);
                    queue.push(item).await.expect("enqueue item");
                }

                for _ in 0..count {
                    let item = queue
                        .pop()
                        .await
                        .expect("pop item")
                        .expect("item should exist while draining queue");
                    queue.mark_completed(&item.id).await.expect("complete item");
                }

                queue.shutdown().await.expect("shutdown queue");
            });
        });

        group.bench_with_input(
            BenchmarkId::new("high_throughput_config", count),
            &count,
            |b, &count| {
                b.to_async(&runtime).iter(|| async move {
                    let queue = SyncQueue::with_config(QueueConfig::high_performance())
                        .expect("queue should build with high throughput config");

                    for idx in 0..count {
                        let payload = json!({ "index": idx, "variant": "high_performance" });
                        let item =
                            SyncItem::with_id(format!("item-ht-{idx}"), payload, Priority::High);
                        queue.push(item).await.expect("enqueue item");
                    }

                    for _ in 0..count {
                        let item = queue
                            .pop()
                            .await
                            .expect("pop item")
                            .expect("item should exist while draining queue");
                        queue.mark_completed(&item.id).await.expect("complete item");
                    }

                    queue.shutdown().await.expect("shutdown queue");
                });
            },
        );
    }

    group.finish();
}

fn bench_queue_batch_processing(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("sync_queue_batch_processing");

    for &batch in &[32usize, 128, 512] {
        group.throughput(Throughput::Elements(batch as u64));

        group.bench_with_input(BenchmarkId::from_parameter(batch), &batch, |b, &batch| {
            b.to_async(&runtime).iter(|| async move {
                let config = QueueConfig {
                    batch_size: batch,
                    max_capacity: batch * 8,
                    ..QueueConfig::default()
                };
                let queue = SyncQueue::with_config(config)
                    .expect("queue should build with batch benchmark configuration");

                let mut items = Vec::with_capacity(batch);
                for idx in 0..batch {
                    let payload = json!({ "index": idx, "variant": "batch" });
                    let item = SyncItem::with_id(format!("batch-{idx}"), payload, Priority::Normal);
                    items.push(item);
                }

                let pushed = queue.push_batch(items).await.expect("batch push succeeds");
                black_box(pushed.len());

                let popped = queue.pop_batch(batch).await.expect("batch pop succeeds");
                for item in popped {
                    queue.mark_completed(&item.id).await.expect("complete item");
                }

                queue.shutdown().await.expect("shutdown queue");
            });
        });
    }

    group.finish();
}

fn bench_queue_failure_path(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("sync_queue_failure_path");

    for &count in &[64usize, 256] {
        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &count| {
            b.to_async(&runtime).iter(|| async move {
                let config = QueueConfig {
                    max_capacity: count * 4,
                    base_retry_delay: Duration::from_millis(1),
                    ..QueueConfig::default()
                };
                let queue = SyncQueue::with_config(config)
                    .expect("queue should build with failure benchmark configuration");

                for idx in 0..count {
                    let item = SyncItem::with_id(
                        format!("fail-{idx}"),
                        json!({ "index": idx, "variant": "failure" }),
                        Priority::High,
                    )
                    .with_max_retries(0);
                    queue.push(item).await.expect("enqueue item");
                }

                for _ in 0..count {
                    let item = queue
                        .pop()
                        .await
                        .expect("pop item")
                        .expect("item should exist while draining failure benchmark queue");
                    let retried = queue
                        .mark_failed(&item.id, Some("synthetic failure".to_string()))
                        .await
                        .expect("mark_failed succeeds");
                    black_box(retried);
                }

                queue.clear().await.expect("clear queue after failure benchmark");
                queue.shutdown().await.expect("shutdown queue");
            });
        });
    }

    group.finish();
}

// ============================================================================
// Retry Benchmarks
// ============================================================================

fn bench_retry_strategy_async(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("sync_retry_strategy_async");

    group.bench_function("immediate_success", |b| {
        let strategy = RetryStrategy::new().with_max_attempts(3).expect("valid attempts");

        b.to_async(&runtime).iter(|| {
            let strategy = strategy.clone();

            async move {
                let (result, metrics) = strategy
                    .execute_with_metrics("immediate_success", || async { Ok::<_, IoError>(()) })
                    .await;

                if result.is_err() {
                    panic!("retry strategy immediate success path failed");
                }

                black_box(metrics.attempts);
            }
        });
    });

    group.bench_function("transient_failures", |b| {
        let strategy = RetryStrategy::new()
            .with_max_attempts(5)
            .expect("valid attempts")
            .with_base_delay(Duration::from_millis(10))
            .expect("valid base delay")
            .with_max_delay(Duration::from_millis(50))
            .expect("valid max delay")
            .with_jitter_factor(0.2);

        b.to_async(&runtime).iter(|| {
            let strategy = strategy.clone();

            async move {
                let mut remaining_failures = 3i32;
                let (result, metrics) = strategy
                    .execute_with_metrics("transient_failures", move || {
                        let should_fail = remaining_failures > 0;
                        if should_fail {
                            remaining_failures -= 1;
                        }
                        async move {
                            if should_fail {
                                Err::<(), IoError>(IoError::other("transient failure"))
                            } else {
                                Ok::<(), IoError>(())
                            }
                        }
                    })
                    .await;

                if result.is_err() {
                    panic!("transient retry strategy should eventually succeed");
                }

                black_box(metrics.total_delay);
            }
        });
    });

    group.bench_function("always_fail", |b| {
        let strategy = RetryStrategy::new()
            .with_max_attempts(4)
            .expect("valid attempts")
            .with_base_delay(Duration::from_millis(5))
            .expect("valid base delay")
            .with_max_delay(Duration::from_millis(20))
            .expect("valid max delay");

        b.to_async(&runtime).iter(|| {
            let strategy = strategy.clone();

            async move {
                let (result, metrics) = strategy
                    .execute_with_metrics("always_fail", || async {
                        Err::<(), IoError>(IoError::other("permanent failure"))
                    })
                    .await;

                if result.is_ok() {
                    panic!("always_fail benchmark should exhaust retries");
                }

                black_box(metrics.attempts);
            }
        });
    });

    group.finish();
}

fn bench_retry_budget(c: &mut Criterion) {
    let mut group = c.benchmark_group("sync_retry_budget");

    group.bench_function("acquire_and_refill", |b| {
        b.iter(|| {
            let clock = MockClock::new();
            let budget = RetryBudget::with_clock(100, 20.0, clock.clone());

            let mut acquired = 0u32;
            for _ in 0..100 {
                if budget.try_acquire() {
                    acquired += 1;
                }
            }
            black_box(acquired);

            clock.advance(Duration::from_secs(3));
            black_box(budget.available());

            budget.return_tokens(10);
            black_box(budget.available());
        });
    });

    group.bench_function("multi_token_acquire", |b| {
        b.iter(|| {
            let budget = RetryBudget::new(50, 25.0);

            let mut successes = 0u32;
            for _ in 0..10 {
                if budget.try_acquire_multiple(5) {
                    successes += 1;
                }
            }

            black_box(successes);
            budget.reset();
        });
    });

    group.finish();
}

criterion_group!(
    sync_queue_benches,
    bench_queue_enqueue_dequeue,
    bench_queue_batch_processing,
    bench_queue_failure_path,
);
criterion_group!(sync_retry_benches, bench_retry_strategy_async, bench_retry_budget);
criterion_main!(sync_queue_benches, sync_retry_benches);
