//! Comprehensive time module benchmarks
//!
//! Benchmarks cover duration parsing/formatting, cron evaluation, interval
//! scheduling, and timer utilities to ensure the time module stays performant.
//!
//! Run with: `cargo bench --bench time_bench -p pulsearc-common --features
//! runtime`

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{TimeZone, Utc};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::time::duration::{parse_duration, parse_duration_ms};
use pulsearc_common::time::format::{
    format_duration, format_duration_compact, format_duration_ms, format_duration_verbose,
};
use pulsearc_common::time::interval::{Interval, IntervalConfig};
use pulsearc_common::time::timer::{recurring, timeout, Timer};
use pulsearc_common::time::CronExpression;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use tokio::task::yield_now;

type ParseScenario = (&'static str, &'static [&'static str]);

// -----------------------------------------------------------------------------
// Helpers
// -----------------------------------------------------------------------------

fn build_paused_runtime() -> Runtime {
    let rt = RuntimeBuilder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build tokio runtime for time benchmarks");

    rt.block_on(async {
        tokio::time::pause();
    });

    rt
}

// -----------------------------------------------------------------------------
// Duration parsing benchmarks
// -----------------------------------------------------------------------------

fn bench_duration_parsing(c: &mut Criterion) {
    const SIMPLE_INPUTS: &[&str] = &["5s", "30s", "10m", "2h", "3d", "1w"];
    const COMPOUND_INPUTS: &[&str] = &["1h 30m", "2h 15m 30s", "3d 4h 5m", "6h 45m", "12h 5m 30s"];
    const FRACTIONAL_INPUTS: &[&str] = &["1.5s", "2.25m", "0.5h", "1.75d"];
    const MILLIS_INPUTS: &[&str] = &["500ms", "1s 250ms", "2m 15s 10ms", "750ms"];
    const MICROS_INPUTS: &[&str] = &["100us", "250us", "999us", "1s 500us"];
    const INVALID_INPUTS: &[&str] = &["", "5", "abc", "60x", "1h20"];

    let mut group = c.benchmark_group("duration_parsing");

    let parse_scenarios: &[ParseScenario] = &[
        ("simple", SIMPLE_INPUTS),
        ("compound", COMPOUND_INPUTS),
        ("fractional", FRACTIONAL_INPUTS),
    ];

    for (name, inputs) in parse_scenarios {
        group.throughput(Throughput::Elements(inputs.len() as u64));
        group.bench_with_input(BenchmarkId::new("parse_duration", *name), inputs, |b, inputs| {
            b.iter(|| {
                for &input in (*inputs).iter() {
                    black_box(parse_duration(black_box(input)).unwrap());
                }
            });
        });
    }

    let parse_ms_scenarios: &[ParseScenario] =
        &[("millis", MILLIS_INPUTS), ("micros", MICROS_INPUTS)];

    for (name, inputs) in parse_ms_scenarios {
        group.throughput(Throughput::Elements(inputs.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("parse_duration_ms", *name),
            inputs,
            |b, inputs| {
                b.iter(|| {
                    for &input in (*inputs).iter() {
                        black_box(parse_duration_ms(black_box(input)).unwrap());
                    }
                });
            },
        );
    }

    group.bench_with_input(
        BenchmarkId::new("parse_duration", "invalid_inputs"),
        INVALID_INPUTS,
        |b, inputs| {
            b.iter(|| {
                for &input in (*inputs).iter() {
                    let err = parse_duration(black_box(input)).unwrap_err();
                    black_box(err);
                }
            });
        },
    );

    group.finish();
}

// -----------------------------------------------------------------------------
// Duration formatting benchmarks
// -----------------------------------------------------------------------------

fn bench_duration_formatting(c: &mut Criterion) {
    let mut group = c.benchmark_group("duration_formatting");

    let canonical_durations = vec![
        Duration::from_micros(250),
        Duration::from_millis(1),
        Duration::from_millis(275),
        Duration::from_secs(45),
        Duration::from_secs(65),
        Duration::from_secs(3665),
        Duration::from_secs(172_801),
    ];

    let ms_precision = vec![
        Duration::from_millis(5),
        Duration::from_millis(125),
        Duration::from_secs(1) + Duration::from_millis(500),
        Duration::from_secs(61) + Duration::from_millis(275),
    ];

    group.throughput(Throughput::Elements(canonical_durations.len() as u64));
    group.bench_function("format_standard", |b| {
        b.iter(|| {
            for duration in &canonical_durations {
                black_box(format_duration(black_box(*duration)));
            }
        });
    });

    group.throughput(Throughput::Elements(ms_precision.len() as u64));
    group.bench_function("format_ms_precision", |b| {
        b.iter(|| {
            for duration in &ms_precision {
                black_box(format_duration_ms(black_box(*duration)));
            }
        });
    });

    group.throughput(Throughput::Elements(canonical_durations.len() as u64));
    group.bench_function("format_compact", |b| {
        b.iter(|| {
            for duration in &canonical_durations {
                black_box(format_duration_compact(black_box(*duration)));
            }
        });
    });

    group.throughput(Throughput::Elements(canonical_durations.len() as u64));
    group.bench_function("format_verbose", |b| {
        b.iter(|| {
            for duration in &canonical_durations {
                black_box(format_duration_verbose(black_box(*duration)));
            }
        });
    });

    group.finish();
}

// -----------------------------------------------------------------------------
// Cron expression benchmarks
// -----------------------------------------------------------------------------

fn bench_cron_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cron_operations");

    let expressions = vec![
        "* * * * *",
        "*/5 * * * *",
        "0 9 * * 1",
        "30 14 * * 1,3,5",
        "0 0 1 * *",
        "15 6 1-5 * 1-5",
        "0 0 */2 1-6 0",
    ];

    group.throughput(Throughput::Elements(expressions.len() as u64));
    group.bench_function("parse", |b| {
        b.iter(|| {
            for expr in &expressions {
                black_box(CronExpression::parse(black_box(expr)).unwrap());
            }
        });
    });

    let parsed: Vec<CronExpression> =
        expressions.iter().map(|expr| CronExpression::parse(expr).unwrap()).collect();

    let datetimes = vec![
        Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2024, 1, 2, 9, 0, 0).unwrap(),
        Utc.with_ymd_and_hms(2024, 1, 3, 14, 30, 0).unwrap(),
        Utc.with_ymd_and_hms(2024, 2, 15, 6, 15, 0).unwrap(),
        Utc.with_ymd_and_hms(2024, 6, 30, 23, 55, 0).unwrap(),
    ];

    let match_operations = (parsed.len() * datetimes.len()) as u64;
    group.throughput(Throughput::Elements(match_operations));
    group.bench_function("matches", |b| {
        b.iter(|| {
            for cron in &parsed {
                for dt in &datetimes {
                    black_box(cron.matches(dt));
                }
            }
        });
    });

    let next_inputs: Vec<(&CronExpression, _)> = parsed.iter().zip(datetimes.iter()).collect();
    group.throughput(Throughput::Elements(next_inputs.len() as u64));
    group.bench_function("next_after", |b| {
        b.iter(|| {
            for (expr, dt) in &next_inputs {
                black_box(expr.next_after(dt));
            }
        });
    });

    group.finish();
}

// -----------------------------------------------------------------------------
// Interval benchmarks
// -----------------------------------------------------------------------------

fn bench_interval_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("interval_operations");
    let rt = build_paused_runtime();

    group.bench_function("config_builder", |b| {
        b.iter(|| {
            let config = IntervalConfig::new(Duration::from_millis(250))
                .with_jitter(0.3)
                .skip_missed_ticks(true);
            black_box(config);
        });
    });

    group.bench_function("simple_tick", |b| {
        b.to_async(&rt).iter(|| async {
            let mut interval = Interval::simple(Duration::from_millis(100));
            black_box(interval.tick().await); // first tick is immediate
            tokio::time::advance(Duration::from_millis(100)).await;
            black_box(interval.tick().await);
        });
    });

    group.bench_function("simple_reset", |b| {
        b.to_async(&rt).iter(|| async {
            let mut interval = Interval::simple(Duration::from_millis(200));
            interval.reset();
            tokio::time::advance(Duration::from_millis(200)).await;
            black_box(interval.tick().await);
        });
    });

    group.bench_function("jitter_tick", |b| {
        b.to_async(&rt).iter(|| async {
            let mut interval = Interval::with_jitter(Duration::from_millis(120), 0.25);
            let advancer = tokio::spawn(async {
                tokio::time::advance(Duration::from_millis(300)).await;
            });
            black_box(interval.tick().await);
            advancer.await.unwrap();
        });
    });

    group.finish();
}

// -----------------------------------------------------------------------------
// Timer benchmarks
// -----------------------------------------------------------------------------

fn bench_timer_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("timer_operations");
    let rt = build_paused_runtime();

    group.bench_function("timer_handle_cancel", |b| {
        b.iter(|| {
            let timer = Timer::after(Duration::from_secs(1));
            let handle = timer.handle();
            handle.cancel();
            black_box(handle.is_cancelled());
        });
    });

    group.bench_function("timer_wait", |b| {
        b.to_async(&rt).iter(|| async {
            let timer = Timer::after(Duration::from_millis(150));
            let wait_task = tokio::spawn(async move {
                timer.wait(Duration::from_millis(150)).await;
            });
            tokio::time::advance(Duration::from_millis(200)).await;
            wait_task.await.unwrap();
        });
    });

    group.bench_function("timeout_fire", |b| {
        b.to_async(&rt).iter(|| async {
            let fired = Arc::new(AtomicBool::new(false));
            let fired_clone = Arc::clone(&fired);

            let handle = timeout(Duration::from_millis(100), move || {
                fired_clone.store(true, Ordering::SeqCst);
            })
            .await;

            tokio::time::advance(Duration::from_millis(150)).await;
            yield_now().await;

            black_box(fired.load(Ordering::SeqCst));
            handle.cancel();
        });
    });

    group.bench_function("timeout_cancel", |b| {
        b.to_async(&rt).iter(|| async {
            let handle = timeout(Duration::from_secs(1), || {}).await;
            handle.cancel();
            tokio::time::advance(Duration::from_secs(2)).await;
            yield_now().await;
            black_box(handle.is_cancelled());
        });
    });

    group.bench_function("recurring_ticks", |b| {
        b.to_async(&rt).iter(|| async {
            let counter = Arc::new(AtomicU32::new(0));
            let counter_clone = Arc::clone(&counter);

            let handle = recurring(Duration::from_millis(50), move || {
                counter_clone.fetch_add(1, Ordering::SeqCst);
            });

            tokio::time::advance(Duration::from_millis(250)).await;
            yield_now().await;
            handle.cancel();
            yield_now().await;

            black_box(counter.load(Ordering::SeqCst));
        });
    });

    group.finish();
}

criterion_group!(
    time_benches,
    bench_duration_parsing,
    bench_duration_formatting,
    bench_cron_operations,
    bench_interval_operations,
    bench_timer_operations,
);
criterion_main!(time_benches);
