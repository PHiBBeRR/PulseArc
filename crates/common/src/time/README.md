# Time Utilities

Comprehensive time handling utilities including duration formatting, cron parsing, timers, and intervals.

## Overview

The time module provides:

- **Clock abstractions**: Real and mock time for testing (re-exported from sync)
- **Duration parsing**: Parse human-readable duration strings ("2h 30m")
- **Duration formatting**: Format durations as human-readable strings
- **Timers**: One-shot and recurring timers with cancellation
- **Intervals**: Recurring intervals with jitter support
- **Cron expressions**: Parse and evaluate cron schedules

## Features

- ✅ Human-readable duration parsing and formatting
- ✅ Mock clock for deterministic testing
- ✅ Cron expression parsing and evaluation
- ✅ Recurring timers with cancellation
- ✅ Intervals with jitter support
- ✅ Zero external coupling

## Quick Start

### Duration Parsing

```rust
use agent::common::time::{parse_duration, parse_duration_ms};
use std::time::Duration;

// Parse simple durations
let duration = parse_duration("5s").unwrap();
assert_eq!(duration, Duration::from_secs(5));

// Parse compound durations
let duration = parse_duration("2h 30m").unwrap();
assert_eq!(duration, Duration::from_secs(9000));

// Parse with milliseconds
let duration = parse_duration_ms("1s 500ms").unwrap();
assert_eq!(duration, Duration::from_millis(1500));
```

### Duration Formatting

```rust
use agent::common::time::{format_duration, format_duration_verbose, format_duration_compact};
use std::time::Duration;

// Standard format
let formatted = format_duration(Duration::from_secs(3665));
assert_eq!(formatted, "1h 1m 5s");

// Verbose format
let formatted = format_duration_verbose(Duration::from_secs(65));
assert_eq!(formatted, "1 minute 5 seconds");

// Compact format
let formatted = format_duration_compact(Duration::from_secs(5400));
assert_eq!(formatted, "1h30m0s");
```

### Mock Clock for Testing

```rust
use agent::common::time::{Clock, MockClock};
use std::time::Duration;

let clock = MockClock::new();
let start = clock.now();

// Advance time
clock.advance(Duration::from_secs(5));

let end = clock.now();
assert_eq!(end.duration_since(start), Duration::from_secs(5));
```

### Cron Expressions

```rust
use agent::common::time::{CronExpression, CronSchedule};

// Parse cron expression
let cron = CronExpression::parse("0 9 * * 1").unwrap(); // Every Monday at 9am

// Check if a datetime matches
let dt = Utc.with_ymd_and_hms(2024, 1, 1, 9, 0, 0).unwrap();
assert!(cron.matches(&dt));

// Get next occurrence
let schedule = CronSchedule::new("0 * * * *").unwrap(); // Every hour
let next = schedule.next();
```

### Timers

```rust
use agent::common::time::timer::{timeout, recurring};
use std::time::Duration;

// One-shot timer
let handle = timeout(Duration::from_secs(5), || {
    println!("Timer fired!");
}).await;

// Cancel if needed
handle.cancel();

// Recurring timer
let handle = recurring(Duration::from_secs(1), || {
    println!("Tick!");
});

// Cancel when done
tokio::time::sleep(Duration::from_secs(10)).await;
handle.cancel();
```

### Intervals

```rust
use agent::common::time::{Interval, IntervalConfig, interval, interval_with_jitter};
use std::time::Duration;

// Simple interval
let mut interval = interval(Duration::from_secs(1));
loop {
    interval.tick().await;
    println!("Tick");
}

// Interval with jitter
let mut interval = interval_with_jitter(Duration::from_secs(1), 0.2);
loop {
    interval.tick().await;
    // Fires every 0.8-1.2 seconds (20% jitter)
}

// Custom configuration
let config = IntervalConfig::new(Duration::from_secs(1))
    .with_jitter(0.3)
    .skip_missed_ticks(true);

let mut interval = Interval::new(config);
```

## API Reference

### Duration Parsing

**Functions:**
- `parse_duration(s: &str) -> Result<Duration, DurationParseError>` - Parse duration string
- `parse_duration_ms(s: &str) -> Result<Duration, DurationParseError>` - Parse with milliseconds

**Supported units:**
- `us` - microseconds
- `ms` - milliseconds  
- `s` - seconds
- `m` - minutes
- `h` - hours
- `d` - days
- `w` - weeks

### Duration Formatting

**Functions:**
- `format_duration(duration: Duration) -> String` - Format as "1h 2m 3s"
- `format_duration_ms(duration: Duration) -> String` - Include milliseconds
- `format_duration_compact(duration: Duration) -> String` - Compact format "1h2m3s"
- `format_duration_verbose(duration: Duration) -> String` - Verbose "1 hour 2 minutes"

### Clock Abstractions

Re-exported from `common::sync`:

**Clock trait:**
- `now() -> Instant` - Get current instant
- `system_time() -> SystemTime` - Get system time
- `millis_since_epoch() -> u64` - Get milliseconds since epoch

**SystemClock:**
- Real system clock implementation

**MockClock:**
- `new()` - Create new mock clock
- `advance(duration)` - Advance time by duration
- `set_elapsed(duration)` - Set elapsed time
- `elapsed()` - Get current elapsed time

### Cron Expressions

**CronExpression:**
- `parse(expr: &str) -> Result<Self, CronParseError>` - Parse cron string
- `matches(dt: &DateTime<Utc>) -> bool` - Check if datetime matches
- `next_after(dt: &DateTime<Utc>) -> Option<DateTime<Utc>>` - Get next occurrence

**CronSchedule:**
- `new(expr: &str) -> Result<Self, CronParseError>` - Create schedule
- `next() -> Option<DateTime<Utc>>` - Get next occurrence
- `next_after(dt: &DateTime<Utc>) -> Option<DateTime<Utc>>` - Get next after datetime
- `matches_now() -> bool` - Check if matches current time

**Cron format:** `minute hour day month weekday`
- `*` - Any value
- `5` - Specific value
- `1,3,5` - List of values
- `1-5` - Range
- `*/5` - Step values

### Timers

**Functions:**
- `timeout<F>(duration, callback) -> TimerHandle` - One-shot timer
- `recurring<F>(duration, callback) -> TimerHandle` - Recurring timer

**TimerHandle:**
- `cancel()` - Cancel the timer
- `is_cancelled() -> bool` - Check if cancelled

**Timer:**
- `after(duration) -> Timer` - Create timer
- `handle() -> TimerHandle` - Get handle
- `wait(duration)` - Wait for timer
- `is_cancelled() -> bool` - Check if cancelled

### Intervals

**Interval:**
- `new(config: IntervalConfig) -> Self` - Create with config
- `simple(duration: Duration) -> Self` - Simple interval
- `with_jitter(duration: Duration, jitter: f64) -> Self` - With jitter
- `tick() -> Instant` - Wait for next tick
- `reset()` - Reset interval

**IntervalConfig:**
- `new(duration) -> Self` - Create config
- `with_jitter(jitter: f64) -> Self` - Set jitter (0.0-1.0)
- `skip_missed_ticks(skip: bool) -> Self` - Skip missed ticks

**Functions:**
- `interval(duration) -> Interval` - Create simple interval
- `interval_with_jitter(duration, jitter) -> Interval` - Create with jitter

## Examples

### Retry with Exponential Backoff

```rust
use agent::common::time::parse_duration;
use std::time::Duration;

async fn retry_with_backoff<F, T, E>(mut operation: F) -> Result<T, E>
where
    F: FnMut() -> Result<T, E>,
{
    let delays = vec!["1s", "2s", "4s", "8s", "16s"];
    
    for delay_str in delays {
        match operation() {
            Ok(result) => return Ok(result),
            Err(e) => {
                let delay = parse_duration(delay_str).unwrap();
                tokio::time::sleep(delay).await;
            }
        }
    }
    
    operation()
}
```

### Scheduled Task with Cron

```rust
use agent::common::time::CronSchedule;
use std::time::Duration;

async fn scheduled_task() {
    let schedule = CronSchedule::new("0 */6 * * *").unwrap(); // Every 6 hours
    
    loop {
        if let Some(next) = schedule.next() {
            let now = Utc::now();
            let delay = (next - now).to_std().unwrap();
            
            tokio::time::sleep(delay).await;
            
            // Run task
            println!("Running scheduled task");
        }
    }
}
```

### Rate-Limited Loop with Jitter

```rust
use agent::common::time::interval_with_jitter;
use std::time::Duration;

async fn rate_limited_processing() {
    // Process items every 1 second with 20% jitter to prevent thundering herd
    let mut interval = interval_with_jitter(Duration::from_secs(1), 0.2);
    
    loop {
        interval.tick().await;
        
        // Process item
        process_item().await;
    }
}
```

### Timeout with Fallback

```rust
use agent::common::time::timer::timeout;
use std::time::Duration;

async fn fetch_with_timeout(url: &str) -> Result<String, String> {
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    
    let handle = timeout(Duration::from_secs(5), move || {
        *result_clone.lock().unwrap() = Some(Err("Timeout".to_string()));
    }).await;
    
    match fetch_data(url).await {
        Ok(data) => {
            handle.cancel();
            Ok(data)
        }
        Err(e) => {
            // Wait for timeout or return error immediately
            Err(e.to_string())
        }
    }
}
```

## Best Practices

### 1. Use Duration Parsing for Configuration

```rust
// Good: Parse from config
let timeout = parse_duration(&config.timeout)?;

// Less flexible
let timeout = Duration::from_secs(30);
```

### 2. Use Mock Clock in Tests

```rust
#[cfg(test)]
mod tests {
    use agent::common::time::MockClock;
    
    #[test]
    fn test_with_mock_time() {
        let clock = MockClock::new();
        clock.advance(Duration::from_secs(5));
        // Test with controlled time
    }
}
```

### 3. Cancel Timers and Intervals

```rust
// Good: Always cancel when done
let handle = recurring(Duration::from_secs(1), || {});
// ... do work ...
handle.cancel();

// Bad: May leak resources
let _handle = recurring(Duration::from_secs(1), || {});
```

### 4. Add Jitter to Intervals

```rust
// Good: Prevents thundering herd
let interval = interval_with_jitter(Duration::from_secs(60), 0.1);

// Can cause synchronized load spikes
let interval = interval(Duration::from_secs(60));
```

### 5. Use Cron for Complex Schedules

```rust
// Good: Clear intent
let schedule = CronSchedule::new("0 2 * * 0")?; // Sundays at 2am

// Less clear
let is_sunday_2am = now.weekday() == Weekday::Sun && now.hour() == 2;
```

## Testing

```bash
# Run all tests
cargo test --lib common::time

# Run specific module tests
cargo test --lib common::time::duration
cargo test --lib common::time::cron

# Run with all features
cargo test --all-features --lib common::time
```

## Dependencies

```toml
[dependencies]
chrono = "0.4"
tokio = { version = "1.0", features = ["time"] }
rand = "0.8"
thiserror = "1.0"
```

## Related Modules

- **common::testing** - Mock clock and test utilities
- **common::sync** - Original Clock implementations
- **common::async_utils** - Async utilities (timeouts, etc.)

## License

See the root LICENSE file for licensing information.

