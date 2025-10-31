//! Lifecycle management benchmarks
//!
//! Covers async state access patterns, managed state utilities, atomic
//! counters, registry lookups, and manager controller orchestration.
//!
//! Run with: `cargo bench --bench lifecycle_bench -p pulsearc-common --features
//! runtime`

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use futures::future::join_all;
use pulsearc_common::lifecycle::{
    shared_state, AtomicCounter, ManagedState, ManagerController, ManagerLifecycle, ManagerStatus,
    SharedState, StateBuilder, StateRegistry,
};
use tokio::runtime::Builder as RuntimeBuilder;
use tokio::sync::Mutex;

// ============================================================================
// Helpers
// ============================================================================

fn build_runtime() -> tokio::runtime::Runtime {
    RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for lifecycle benchmarks")
}

// ============================================================================
// SharedState Benchmarks
// ============================================================================

fn bench_shared_state_patterns(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("lifecycle_shared_state");

    for &concurrency in &[4usize, 16, 64] {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("mixed_access", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&runtime).iter(|| async move {
                    let state = SharedState::new(0usize, format!("bench-shared-{concurrency}"));

                    let tasks = (0..concurrency).map(|idx| {
                        let state = state.clone();
                        async move {
                            if idx % 2 == 0 {
                                let guard = state.read().await;
                                black_box(*guard);
                            } else {
                                let value = state
                                    .update(|value| {
                                        *value = value.wrapping_add(1);
                                        *value
                                    })
                                    .await;
                                black_box(value);
                            }
                        }
                    });

                    join_all(tasks).await;

                    let final_value = *state.read().await;
                    black_box(final_value);
                });
            },
        );
    }

    group.bench_function("timeout_accessors", |b| {
        b.to_async(&runtime).iter(|| async {
            let state = SharedState::new(42usize, "timeout-shared");
            let read_guard =
                state.read_timeout(Duration::from_millis(5)).await.expect("read timeout");
            black_box(*read_guard);
            drop(read_guard);

            let mut write_guard =
                state.write_timeout(Duration::from_millis(5)).await.expect("write timeout");
            *write_guard = write_guard.wrapping_add(1);
            black_box(*write_guard);
        });
    });

    group.finish();
}

// ============================================================================
// ManagedState Benchmarks
// ============================================================================

fn bench_managed_state_patterns(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("lifecycle_managed_state");

    group.bench_function("read_write_cycle", |b| {
        b.to_async(&runtime).iter(|| async {
            let state = ManagedState::new(vec![0u64; 128]);

            for index in 0..256usize {
                state
                    .modify(|slots| {
                        let slot = index % slots.len();
                        slots[slot] = slots[slot].wrapping_add(1);
                    })
                    .await;
            }

            let snapshot = state.clone_value().await;
            black_box(snapshot.len());

            let idle = state.idle_time().await;
            black_box(idle);
        });
    });

    for &size in &[16usize, 64, 256] {
        group.bench_with_input(BenchmarkId::new("state_builder", size), &size, |b, &size| {
            b.iter(|| {
                let builder = StateBuilder::default().with_initial_value(vec![0u8; size]);
                let state = builder.build().expect("builder should yield managed state");
                black_box(state.created_at());
            });
        });
    }

    group.finish();
}

// ============================================================================
// AtomicCounter Benchmarks
// ============================================================================

fn bench_atomic_counter_patterns(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("lifecycle_atomic_counter");

    for &concurrency in &[32usize, 128, 512] {
        group.throughput(Throughput::Elements(concurrency as u64));
        group.bench_with_input(
            BenchmarkId::new("increments", concurrency),
            &concurrency,
            |b, &concurrency| {
                b.to_async(&runtime).iter(|| async move {
                    let counter = Arc::new(AtomicCounter::new(0));

                    let tasks = (0..concurrency).map(|_| {
                        let counter = Arc::clone(&counter);
                        async move {
                            let value = counter.increment().await;
                            black_box(value);
                        }
                    });

                    join_all(tasks).await;

                    let final_value = counter.get().await;
                    black_box(final_value);
                });
            },
        );
    }

    group.bench_function("mutation_cycle", |b| {
        b.to_async(&runtime).iter(|| async {
            let counter = AtomicCounter::new(10);
            let added = counter.add(25).await;
            let decremented = counter.decrement().await;
            counter.set(0).await;
            let final_value = counter.get().await;
            black_box((added, decremented, final_value));
        });
    });

    group.finish();
}

// ============================================================================
// ManagerController Benchmarks
// ============================================================================

#[derive(Clone)]
struct BenchManager {
    name: String,
    status: ManagerStatus,
    setup_units: usize,
    teardown_units: usize,
    init_counter: Arc<Mutex<u64>>,
    shutdown_counter: Arc<Mutex<u64>>,
}

impl BenchManager {
    fn new(
        name: String,
        init_counter: Arc<Mutex<u64>>,
        shutdown_counter: Arc<Mutex<u64>>,
        setup_units: usize,
        teardown_units: usize,
    ) -> Self {
        Self {
            name,
            status: ManagerStatus::Created,
            setup_units,
            teardown_units,
            init_counter,
            shutdown_counter,
        }
    }
}

#[async_trait]
impl ManagerLifecycle for BenchManager {
    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.status = ManagerStatus::Initializing;
        if self.setup_units > 0 {
            let mut guard = self.init_counter.lock().await;
            *guard += self.setup_units as u64;
        }
        self.status = ManagerStatus::Running;
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.status = ManagerStatus::ShuttingDown;
        if self.teardown_units > 0 {
            let mut guard = self.shutdown_counter.lock().await;
            *guard += self.teardown_units as u64;
        }
        self.status = ManagerStatus::Shutdown;
        Ok(())
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn status(&self) -> ManagerStatus {
        self.status
    }
}

fn bench_manager_controller_lifecycle(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("lifecycle_manager_controller");

    for &manager_count in &[4usize, 16, 64] {
        group.throughput(Throughput::Elements(manager_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(manager_count),
            &manager_count,
            |b, &manager_count| {
                b.to_async(&runtime).iter(|| async move {
                    let mut controller = ManagerController::new();
                    let init_counter = Arc::new(Mutex::new(0u64));
                    let shutdown_counter = Arc::new(Mutex::new(0u64));

                    for idx in 0..manager_count {
                        let manager = BenchManager::new(
                            format!("bench-manager-{idx}"),
                            Arc::clone(&init_counter),
                            Arc::clone(&shutdown_counter),
                            64,
                            32,
                        );
                        controller.add_manager(manager);
                    }

                    controller.initialize_all().await.expect("initialize managers");
                    let status = controller.status().await;
                    black_box(status);

                    controller.shutdown_all().await.expect("shutdown managers");

                    let init_total = *init_counter.lock().await;
                    let shutdown_total = *shutdown_counter.lock().await;
                    black_box(init_total + shutdown_total);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// StateRegistry Benchmarks
// ============================================================================

fn bench_state_registry_operations(c: &mut Criterion) {
    let runtime = build_runtime();
    let mut group = c.benchmark_group("lifecycle_state_registry");

    for &entries in &[32usize, 128, 512] {
        group.throughput(Throughput::Elements(entries as u64));
        group.bench_with_input(BenchmarkId::from_parameter(entries), &entries, |b, &entries| {
            b.to_async(&runtime).iter(|| async move {
                let mut registry = StateRegistry::new();

                for idx in 0..entries {
                    registry.register(format!("state-{idx}"), shared_state(idx as u64));
                }

                let states: Vec<_> = (0..entries)
                    .map(|idx| {
                        let key = format!("state-{idx}");
                        Arc::clone(registry.get::<u64>(&key).expect("state exists"))
                    })
                    .collect();

                join_all(states.into_iter().map(|state| async move {
                    let guard = state.read().await;
                    black_box(*guard);
                }))
                .await;
            });
        });
    }

    group.finish();
}

criterion_group!(
    lifecycle_benches,
    bench_shared_state_patterns,
    bench_managed_state_patterns,
    bench_atomic_counter_patterns,
    bench_manager_controller_lifecycle,
    bench_state_registry_operations,
);
criterion_main!(lifecycle_benches);
