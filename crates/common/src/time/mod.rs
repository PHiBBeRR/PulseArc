//! Time utilities and abstractions
//!
//! This module provides comprehensive time handling utilities including:
//! - **Clock abstractions**: Real and mock time for testing (re-exported from
//!   testing)
//! - **[`duration`]**: Duration formatting and parsing
//! - **[`format`]**: Human-readable duration formatting
//! - **[`interval`]**: Recurring intervals with jitter
//! - **[`timer`]**: One-shot and recurring timers
//! - **[`cron`]**: Cron expression parsing and evaluation
//!
//! ## Usage
//!
//! ```rust
//! # #[cfg(feature = "runtime")]
//! # {
//! use std::time::Duration;
//!
//! use pulsearc_common::time::{format_duration, parse_duration, MockClock};
//!
//! // Format durations
//! let formatted = format_duration(Duration::from_secs(3665));
//! assert_eq!(formatted, "1h 1m 5s");
//!
//! // Parse durations
//! let duration = parse_duration("2h 30m").unwrap();
//! assert_eq!(duration, Duration::from_secs(9000));
//!
//! // Mock time for testing
//! let clock = MockClock::new();
//! clock.advance(Duration::from_secs(5));
//! # }
//! ```

pub mod cron;
pub mod duration;
pub mod format;
pub mod interval;
pub mod timer;

// Re-export commonly used items
pub use cron::{CronExpression, CronParseError, CronSchedule};
pub use duration::{parse_duration, DurationParseError};
pub use format::format_duration;
pub use interval::{Interval, IntervalConfig};
pub use timer::{Timer, TimerHandle};

// Re-export Clock abstractions from testing module
pub use crate::testing::time::{Clock, MockClock, SystemClock};
