//! Integration tests for the `time` module.
//!
//! These tests cover duration parsing/formatting, cron scheduling, interval
//! timing with jitter, timers, and the testing clock abstractions to ensure the
//! public runtime-facing APIs in `pulsearc_common::time` work together as
//! expected.

#![cfg(feature = "runtime")]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Datelike, TimeZone, Timelike, Utc};
use pulsearc_common::testing::{Clock, MockClock};
use pulsearc_common::time::duration::parse_duration_ms;
use pulsearc_common::time::format::{
    format_duration_compact, format_duration_ms, format_duration_verbose,
};
use pulsearc_common::time::timer::{recurring, timeout};
use pulsearc_common::time::{
    format_duration, parse_duration, CronExpression, CronSchedule, DurationParseError, Interval,
    Timer,
};

/// Verifies that textual durations can be parsed and formatted across the
/// different helpers.
#[test]
fn test_duration_parsing_and_formatting() {
    let cases = [
        ("45s", Duration::from_secs(45), "45s", "45s", "45 seconds"),
        ("2h 30m", Duration::from_secs(9_000), "2h 30m 0s", "2h30m0s", "2 hours 30 minutes"),
        (
            "1d 1h 1m 1s",
            Duration::from_secs(90_061),
            "1d 1h 1m 1s",
            "1d1h1m1s",
            "1 day 1 hour 1 minute 1 second",
        ),
        ("0.5s", Duration::from_millis(500), "500ms", "500ms", "500 milliseconds"),
        ("1.5h", Duration::from_secs(5_400), "1h 30m 0s", "1h30m0s", "1 hour 30 minutes"),
    ];

    for (input, expected_duration, expected_format, expected_compact, expected_verbose) in cases {
        let parsed = parse_duration(input).expect("duration should parse");
        assert_eq!(parsed, expected_duration, "parsed duration mismatch for input {input}");

        assert_eq!(
            format_duration(parsed),
            expected_format,
            "format_duration mismatch for input {input}"
        );
        assert_eq!(
            format_duration_compact(parsed),
            expected_compact,
            "format_duration_compact mismatch for input {input}"
        );
        assert_eq!(
            format_duration_verbose(parsed),
            expected_verbose,
            "format_duration_verbose mismatch for input {input}"
        );
    }
}

/// Ensures millisecond and microsecond parsing / formatting helpers stay in
/// sync and surface the correct error variants.
#[test]
fn test_duration_millisecond_precision() {
    let precise = parse_duration_ms("1s 250ms").expect("valid millisecond duration");
    assert_eq!(precise, Duration::from_millis(1_250));
    assert_eq!(format_duration_ms(precise), "1s 250ms");

    let micros = parse_duration_ms("500us").expect("valid microsecond duration");
    assert_eq!(micros, Duration::from_micros(500));
    assert_eq!(format_duration(Duration::from_micros(500)), "500us");
    assert_eq!(format_duration_ms(Duration::from_micros(500)), "0ms");

    let err = parse_duration_ms("15");
    assert!(
        matches!(err, Err(DurationParseError::InvalidFormat(ref message)) if message.contains("Missing unit")),
        "expected missing-unit format error, got {err:?}"
    );
}

/// Validates that the mock clock integrates with duration helpers and maintains
/// elapsed/system time consistency.
#[test]
fn test_mock_clock_advancement_and_duration_display() {
    let clock = MockClock::new();
    let base_instant = clock.now();
    let base_millis = clock.millis_since_epoch();

    let advance_by = parse_duration("2h 30m").expect("duration parsing succeeds");
    clock.advance(advance_by);

    let advanced_instant = clock.now();
    assert_eq!(
        advanced_instant.duration_since(base_instant),
        advance_by,
        "mock clock should advance by parsed duration"
    );

    let millis_delta = clock.millis_since_epoch() - base_millis;
    assert_eq!(
        millis_delta,
        advance_by.as_millis() as u64,
        "millis_since_epoch should advance in lockstep with elapsed duration"
    );

    assert_eq!(format_duration(advance_by), "2h 30m 0s");
    assert_eq!(format_duration_compact(advance_by), "2h30m0s");
    assert_eq!(format_duration_verbose(advance_by), "2 hours 30 minutes");
}

/// Exercises cron expression parsing, matching, and next-occurrence
/// calculations for a weekday schedule.
#[test]
fn test_cron_expression_weekday_schedule() {
    let cron = CronExpression::parse("0 9 * * 1-5").expect("valid cron expression");

    let friday_before_shift = Utc.with_ymd_and_hms(2024, 1, 5, 8, 15, 0).unwrap();
    let next = cron.next_after(&friday_before_shift).expect("next run should be available");
    assert_eq!(next.hour(), 9);
    assert_eq!(next.minute(), 0);
    assert_eq!(next.weekday().num_days_from_sunday(), 5);
    assert!(cron.matches(&next));

    let friday_after_shift = Utc.with_ymd_and_hms(2024, 1, 5, 10, 0, 0).unwrap();
    let following = cron.next_after(&friday_after_shift).expect("next run should be available");
    assert_eq!(following.hour(), 9);
    assert_eq!(following.minute(), 0);
    assert_eq!(following.weekday().num_days_from_sunday(), 1); // Monday
    assert!(cron.matches(&following));

    let elapsed = following - friday_after_shift;
    let expected_gap = parse_duration("2d 23h").expect("duration parsing succeeds");
    assert_eq!(elapsed.num_seconds(), expected_gap.as_secs() as i64);
}

/// Verifies daily cron schedules advance by the exact duration calculated from
/// the duration parsing helpers.
#[test]
fn test_cron_schedule_midnight_gap() {
    let schedule = CronSchedule::new("0 0 * * *").expect("valid cron schedule");
    let baseline = Utc.with_ymd_and_hms(2024, 1, 1, 12, 15, 0).unwrap();

    let next = schedule.next_after(&baseline).expect("next occurrence should exist");
    assert_eq!(next.hour(), 0);
    assert_eq!(next.minute(), 0);

    let diff = next - baseline;
    let expected = parse_duration("11h 45m").expect("duration parsing succeeds");
    assert_eq!(diff.num_minutes(), expected.as_secs() as i64 / 60);
}

/// Confirms that simple intervals tick immediately, respect their configured
/// cadence, and can be reset for immediate delivery.
#[tokio::test(flavor = "multi_thread")]
async fn test_interval_simple_reset_behavior() {
    let mut interval = Interval::simple(Duration::from_millis(40));

    let start = tokio::time::Instant::now();
    interval.tick().await;
    let initial_elapsed = start.elapsed();
    assert!(
        initial_elapsed < Duration::from_millis(10),
        "first tick should be near-immediate, got {initial_elapsed:?}"
    );

    let before_second = tokio::time::Instant::now();
    interval.tick().await;
    let second_elapsed = before_second.elapsed();
    assert!(
        second_elapsed >= Duration::from_millis(30) && second_elapsed <= Duration::from_millis(60),
        "second tick should land near configured cadence, got {second_elapsed:?}"
    );

    // Advance partway through the next cycle, then reset and ensure the countdown
    // restarts
    tokio::time::sleep(Duration::from_millis(20)).await;
    interval.reset();

    let reset_start = tokio::time::Instant::now();
    interval.tick().await;
    let reset_elapsed = reset_start.elapsed();
    assert!(
        reset_elapsed >= Duration::from_millis(30) && reset_elapsed <= Duration::from_millis(60),
        "reset should restart countdown close to full interval, got {reset_elapsed:?}"
    );
}

/// Checks that jittered intervals stay within the configured jitter bounds over
/// multiple ticks.
#[tokio::test(flavor = "multi_thread")]
async fn test_interval_with_jitter_bounds() {
    let mut interval = Interval::with_jitter(Duration::from_millis(80), 0.25);
    let mut last_tick = tokio::time::Instant::now();

    for _ in 0..3 {
        interval.tick().await;
        let now = tokio::time::Instant::now();
        let elapsed = now.duration_since(last_tick);
        assert!(
            elapsed >= Duration::from_millis(55) && elapsed <= Duration::from_millis(150),
            "elapsed {elapsed:?} outside jitter bounds"
        );
        last_tick = now;
    }
}

/// Verifies one-shot timers execute callbacks exactly once and honour
/// cancellation.
#[tokio::test(flavor = "multi_thread")]
async fn test_timeout_execution_and_cancellation() {
    let fired = Arc::new(AtomicU32::new(0));
    let fired_clone = Arc::clone(&fired);

    let handle = timeout(Duration::from_millis(30), move || {
        fired_clone.fetch_add(1, Ordering::SeqCst);
    })
    .await;

    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(fired.load(Ordering::SeqCst), 1);
    assert!(!handle.is_cancelled());

    let suppressed = Arc::new(AtomicU32::new(0));
    let suppressed_clone = Arc::clone(&suppressed);

    let cancellable = timeout(Duration::from_millis(60), move || {
        suppressed_clone.fetch_add(1, Ordering::SeqCst);
    })
    .await;

    cancellable.cancel();
    tokio::time::sleep(Duration::from_millis(80)).await;

    assert_eq!(suppressed.load(Ordering::SeqCst), 0);
    assert!(cancellable.is_cancelled());
}

/// Ensures recurring timers continue firing until explicitly cancelled.
#[tokio::test(flavor = "multi_thread")]
async fn test_recurring_timer_stops_after_cancel() {
    let counter = Arc::new(AtomicU32::new(0));
    let counter_clone = Arc::clone(&counter);

    let handle = recurring(Duration::from_millis(20), move || {
        counter_clone.fetch_add(1, Ordering::SeqCst);
    });

    tokio::time::sleep(Duration::from_millis(80)).await;
    handle.cancel();
    tokio::time::sleep(Duration::from_millis(40)).await;

    let observed = counter.load(Ordering::SeqCst);
    assert!((3..=5).contains(&observed), "expected between 3 and 5 invocations, got {observed}");
}

/// Confirms the one-shot timer helper waits for the expected duration.
#[tokio::test(flavor = "multi_thread")]
async fn test_timer_wait_completes() {
    let timer = Timer::after(Duration::from_millis(25));
    let handle = timer.handle();
    assert!(!handle.is_cancelled());

    let start = tokio::time::Instant::now();
    timer.wait(Duration::from_millis(25)).await;

    let elapsed = start.elapsed();
    assert!(
        elapsed >= Duration::from_millis(20),
        "timer should wait close to configured duration, got {elapsed:?}"
    );
    assert!(!handle.is_cancelled());
}
