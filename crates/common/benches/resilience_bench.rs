//! Comprehensive resilience benchmarks
//!
//! Benchmarks for circuit breaker and retry primitives including synchronous
//! and asynchronous execution paths, state-machine transitions, and backoff
//! calculations.
//!
//! Run with: `cargo bench --bench resilience_bench -p pulsearc-common
//! --features runtime`

use std::error::Error;
use std::fmt::{Display, Formatter};
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use pulsearc_common::resilience::{
    policies, BackoffStrategy, CircuitBreaker, CircuitBreakerConfigBuilder, Jitter, MockClock,
    ResilienceError, RetryConfigBuilder, RetryExecutor,
};
use tokio::runtime::Builder as RuntimeBuilder;

// ============================================================================
// Circuit Breaker Benchmarks
// ============================================================================

fn bench_circuit_breaker_sync_paths(c: &mut Criterion) {
    let mut group = c.benchmark_group("circuit_breaker_sync_paths");

    group.bench_function("call_success", |b| {
        let breaker = CircuitBreaker::with_defaults();
        b.iter(|| {
            let result: Result<_, ResilienceError<std::io::Error>> =
                breaker.call(|| Ok::<_, std::io::Error>(()));
            if let Err(err) = result {
                panic!("circuit breaker success path failed: {err}");
            }
        });
    });

    group.bench_function("call_fail_to_open", |b| {
        b.iter(|| {
            let config = CircuitBreakerConfigBuilder::new()
                .failure_threshold(5)
                .success_threshold(2)
                .timeout(Duration::from_secs(30))
                .half_open_max_calls(3)
                .reset_on_success(false)
                .build()
                .expect("valid circuit breaker config for benchmarks");

            let breaker = CircuitBreaker::new(config)
                .expect("circuit breaker should build with benchmark configuration");

            for _ in 0..5 {
                let result: Result<_, ResilienceError<std::io::Error>> =
                    breaker.call(|| Err::<(), _>(std::io::Error::other("benchmark failure")));
                let _result = black_box(result);
            }

            black_box(breaker.get_state());
        });
    });

    group.bench_function("open_short_circuit", |b| {
        let config = CircuitBreakerConfigBuilder::new()
            .failure_threshold(1)
            .success_threshold(1)
            .timeout(Duration::from_secs(60))
            .half_open_max_calls(1)
            .reset_on_success(false)
            .build()
            .expect("valid circuit breaker config for benchmarks");
        let breaker =
            CircuitBreaker::new(config).expect("circuit breaker should build for short-circuit");

        // Trip the breaker so it remains open for the benchmark iterations.
        let _ = breaker.call(|| Err::<(), _>(std::io::Error::other("initial failure")));

        b.iter(|| {
            let result: Result<_, ResilienceError<std::io::Error>> =
                breaker.call(|| Ok::<_, std::io::Error>(()));
            let _result = black_box(result);
        });
    });

    group.finish();
}

fn bench_circuit_breaker_state_machine(c: &mut Criterion) {
    let mut group = c.benchmark_group("circuit_breaker_state_machine");

    group.bench_function("open_half_open_recover", |b| {
        b.iter(|| {
            let clock = MockClock::new();
            let breaker = CircuitBreaker::builder()
                .failure_threshold(3)
                .success_threshold(2)
                .timeout(Duration::from_millis(10))
                .half_open_max_calls(2)
                .reset_on_success(true)
                .clock(clock.clone())
                .build()
                .expect("circuit breaker should build with mock clock");

            for _ in 0..3 {
                let _ = breaker.call(|| Err::<(), _>(std::io::Error::other("state transition")));
            }
            black_box(breaker.get_state());

            clock.advance(Duration::from_millis(10));
            let _ = breaker.can_execute();

            let _ = breaker.call(|| Ok::<_, std::io::Error>(()));
            let _ = breaker.call(|| Ok::<_, std::io::Error>(()));

            black_box(breaker.get_state());
        });
    });

    group.finish();
}

// ============================================================================
// Retry Benchmarks
// ============================================================================

fn build_runtime() -> tokio::runtime::Runtime {
    RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime should build for benchmarks")
}

#[derive(Debug, Clone)]
struct BenchError(&'static str);

impl Display for BenchError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for BenchError {}

fn bench_retry_executor_outcomes(c: &mut Criterion) {
    let mut group = c.benchmark_group("retry_executor_outcomes");
    let runtime = build_runtime();

    group.bench_function("immediate_success", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = RetryConfigBuilder::new()
                .max_attempts(3)
                .fixed_backoff(Duration::ZERO)
                .no_jitter()
                .reset_on_success(false)
                .build()
                .expect("retry config should build for immediate success");
            let executor = RetryExecutor::new(config, policies::AlwaysRetry);

            let result: Result<_, _> = executor.execute(|| async { Ok::<_, BenchError>(()) }).await;
            if let Err(err) = result {
                panic!("retry immediate success failed: {err:?}");
            }
        });
    });

    group.bench_function("transient_failures_then_success", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = RetryConfigBuilder::new()
                .max_attempts(5)
                .fixed_backoff(Duration::ZERO)
                .no_jitter()
                .reset_on_success(true)
                .build()
                .expect("retry config should build for transient failures");
            let executor = RetryExecutor::new(config, policies::AlwaysRetry);

            let mut remaining_failures = 3u32;
            let result: Result<_, _> = executor
                .execute(move || {
                    let fail_now = remaining_failures > 0;
                    if fail_now {
                        remaining_failures -= 1;
                    }
                    async move {
                        if fail_now {
                            Err::<(), _>(BenchError("transient failure"))
                        } else {
                            Ok::<_, BenchError>(())
                        }
                    }
                })
                .await;

            if let Err(err) = result {
                panic!("retry transient failure path exhausted: {err:?}");
            }
        });
    });

    group.bench_function("always_fail", |b| {
        b.to_async(&runtime).iter(|| async {
            let config = RetryConfigBuilder::new()
                .max_attempts(4)
                .fixed_backoff(Duration::ZERO)
                .no_jitter()
                .reset_on_success(false)
                .build()
                .expect("retry config should build for always fail case");
            let executor = RetryExecutor::new(config, policies::AlwaysRetry);

            let result: Result<(), _> =
                executor.execute(|| async { Err::<(), _>(BenchError("permanent failure")) }).await;
            let _result = black_box(result);
        });
    });

    group.finish();
}

fn bench_retry_backoff_calculations(c: &mut Criterion) {
    let mut group = c.benchmark_group("retry_backoff_calculations");
    let attempts = [0u32, 1, 5, 10];

    let strategies = [
        ("fixed", BackoffStrategy::Fixed(Duration::from_millis(1))),
        (
            "linear",
            BackoffStrategy::Linear {
                initial_delay: Duration::from_millis(1),
                increment: Duration::from_millis(5),
            },
        ),
        (
            "exponential",
            BackoffStrategy::Exponential {
                initial_delay: Duration::from_millis(1),
                base: 2.0,
                max_delay: Duration::from_secs(1),
            },
        ),
    ];

    for (name, strategy) in strategies {
        group.bench_with_input(BenchmarkId::new("calculate_delay", name), &strategy, |b, strat| {
            b.iter(|| {
                for attempt in attempts {
                    black_box(strat.calculate_delay(attempt));
                }
            });
        });
    }

    group.finish();
}

fn bench_retry_jitter(c: &mut Criterion) {
    let mut group = c.benchmark_group("retry_jitter");
    let delays = [Duration::from_millis(1), Duration::from_millis(5), Duration::from_millis(10)];
    let attempts = [0u32, 1, 5, 10];

    let jitters = [
        ("none", Jitter::None),
        ("full", Jitter::Full),
        ("equal", Jitter::Equal),
        ("decorrelated", Jitter::Decorrelated { base: Duration::from_millis(2) }),
    ];

    for (name, jitter) in jitters {
        group.bench_with_input(BenchmarkId::new("apply", name), &jitter, |b, jitter| {
            b.iter(|| {
                for delay in delays {
                    for attempt in attempts {
                        black_box(jitter.apply(delay, attempt));
                    }
                }
            });
        });
    }

    group.finish();
}

criterion_group!(
    resilience,
    bench_circuit_breaker_sync_paths,
    bench_circuit_breaker_state_machine,
    bench_retry_executor_outcomes,
    bench_retry_backoff_calculations,
    bench_retry_jitter
);
criterion_main!(resilience);
