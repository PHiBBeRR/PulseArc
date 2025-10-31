//! Cron expression parsing and evaluation
//!
//! Provides utilities for parsing and evaluating cron expressions.

use std::fmt;

use chrono::{DateTime, Datelike, Timelike, Utc};
use thiserror::Error;

/// Error type for cron parsing
#[derive(Debug, Error, Clone, PartialEq)]
pub enum CronParseError {
    #[error("Invalid cron expression: {0}")]
    InvalidExpression(String),

    #[error("Invalid field: {0}")]
    InvalidField(String),

    #[error("Invalid range: {0}")]
    InvalidRange(String),

    #[error("Too many fields: expected 5, got {0}")]
    TooManyFields(usize),

    #[error("Too few fields: expected 5, got {0}")]
    TooFewFields(usize),
}

/// A parsed cron expression
///
/// Supports standard cron format: minute hour day month weekday
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use pulsearc_common::time::cron::CronExpression;
///
/// // Every day at midnight
/// let cron = CronExpression::parse("0 0 * * *").unwrap();
///
/// // Every hour
/// let cron = CronExpression::parse("0 * * * *").unwrap();
///
/// // Every Monday at 9am
/// let cron = CronExpression::parse("0 9 * * 1").unwrap();
/// # }
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct CronExpression {
    minute: CronField,
    hour: CronField,
    day: CronField,
    month: CronField,
    weekday: CronField,
}

impl CronExpression {
    /// Parse a cron expression from a string
    pub fn parse(expr: &str) -> Result<Self, CronParseError> {
        let parts: Vec<&str> = expr.split_whitespace().collect();

        if parts.len() < 5 {
            return Err(CronParseError::TooFewFields(parts.len()));
        }
        if parts.len() > 5 {
            return Err(CronParseError::TooManyFields(parts.len()));
        }

        Ok(Self {
            minute: CronField::parse(parts[0], 0, 59)?,
            hour: CronField::parse(parts[1], 0, 23)?,
            day: CronField::parse(parts[2], 1, 31)?,
            month: CronField::parse(parts[3], 1, 12)?,
            weekday: CronField::parse(parts[4], 0, 6)?,
        })
    }

    /// Check if a datetime matches this cron expression
    pub fn matches(&self, dt: &DateTime<Utc>) -> bool {
        self.minute.matches(dt.minute())
            && self.hour.matches(dt.hour())
            && self.day.matches(dt.day())
            && self.month.matches(dt.month())
            && self.weekday.matches(dt.weekday().num_days_from_sunday())
    }

    /// Get the next occurrence after the given datetime
    pub fn next_after(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let mut current = *dt + chrono::Duration::minutes(1);

        // Search up to 4 years in the future
        for _ in 0..(4 * 365 * 24 * 60) {
            if self.matches(&current) {
                return Some(current);
            }
            current += chrono::Duration::minutes(1);
        }

        None
    }
}

impl fmt::Display for CronExpression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {} {} {}", self.minute, self.hour, self.day, self.month, self.weekday)
    }
}

/// A cron field (minute, hour, day, month, weekday)
#[derive(Debug, Clone, PartialEq)]
enum CronField {
    Any,
    Single(u32),
    List(Vec<u32>),
    Range(u32, u32),
    Step(u32, u32), // start, step
}

impl CronField {
    fn parse(s: &str, min: u32, max: u32) -> Result<Self, CronParseError> {
        if s == "*" {
            return Ok(CronField::Any);
        }

        if s.contains(',') {
            let values: Result<Vec<u32>, _> = s.split(',').map(|v| v.trim().parse()).collect();
            let values = values.map_err(|_| CronParseError::InvalidField(s.to_string()))?;

            for &v in &values {
                if v < min || v > max {
                    return Err(CronParseError::InvalidRange(format!(
                        "{} not in range {}-{}",
                        v, min, max
                    )));
                }
            }

            return Ok(CronField::List(values));
        }

        if s.contains('/') {
            let parts: Vec<&str> = s.split('/').collect();
            if parts.len() != 2 {
                return Err(CronParseError::InvalidField(s.to_string()));
            }

            let start = if parts[0] == "*" {
                min
            } else {
                parts[0].parse().map_err(|_| CronParseError::InvalidField(s.to_string()))?
            };

            let step: u32 =
                parts[1].parse().map_err(|_| CronParseError::InvalidField(s.to_string()))?;

            return Ok(CronField::Step(start, step));
        }

        if s.contains('-') {
            let parts: Vec<&str> = s.split('-').collect();
            if parts.len() != 2 {
                return Err(CronParseError::InvalidField(s.to_string()));
            }

            let start: u32 =
                parts[0].parse().map_err(|_| CronParseError::InvalidField(s.to_string()))?;
            let end: u32 =
                parts[1].parse().map_err(|_| CronParseError::InvalidField(s.to_string()))?;

            if start < min || end > max || start > end {
                return Err(CronParseError::InvalidRange(format!(
                    "{}-{} not valid in range {}-{}",
                    start, end, min, max
                )));
            }

            return Ok(CronField::Range(start, end));
        }

        let value: u32 = s.parse().map_err(|_| CronParseError::InvalidField(s.to_string()))?;

        if value < min || value > max {
            return Err(CronParseError::InvalidRange(format!(
                "{} not in range {}-{}",
                value, min, max
            )));
        }

        Ok(CronField::Single(value))
    }

    fn matches(&self, value: u32) -> bool {
        match self {
            CronField::Any => true,
            CronField::Single(v) => *v == value,
            CronField::List(values) => values.contains(&value),
            CronField::Range(start, end) => value >= *start && value <= *end,
            CronField::Step(start, step) => {
                value >= *start && (value - start).is_multiple_of(*step)
            }
        }
    }
}

impl fmt::Display for CronField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CronField::Any => write!(f, "*"),
            CronField::Single(v) => write!(f, "{}", v),
            CronField::List(values) => {
                let strs: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", strs.join(","))
            }
            CronField::Range(start, end) => write!(f, "{}-{}", start, end),
            CronField::Step(start, step) => write!(f, "{}/{}", start, step),
        }
    }
}

/// A cron schedule that can be used to get next occurrences
pub struct CronSchedule {
    expression: CronExpression,
}

impl CronSchedule {
    /// Create a new cron schedule
    pub fn new(expr: &str) -> Result<Self, CronParseError> {
        Ok(Self { expression: CronExpression::parse(expr)? })
    }

    /// Get the next occurrence after now
    pub fn next(&self) -> Option<DateTime<Utc>> {
        self.expression.next_after(&Utc::now())
    }

    /// Get the next occurrence after a specific datetime
    pub fn next_after(&self, dt: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        self.expression.next_after(dt)
    }

    /// Check if the schedule matches the current time
    pub fn matches_now(&self) -> bool {
        self.expression.matches(&Utc::now())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for time::cron.
    use chrono::TimeZone;

    use super::*;

    /// Validates `CronExpression::parse` behavior for the parse every minute
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cron.minute` equals `CronField::Any`.
    #[test]
    fn test_parse_every_minute() {
        let cron = CronExpression::parse("* * * * *").unwrap();
        assert_eq!(cron.minute, CronField::Any);
    }

    /// Validates `CronExpression::parse` behavior for the parse specific time
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cron.minute` equals `CronField::Single(30)`.
    /// - Confirms `cron.hour` equals `CronField::Single(14)`.
    #[test]
    fn test_parse_specific_time() {
        let cron = CronExpression::parse("30 14 * * *").unwrap();
        assert_eq!(cron.minute, CronField::Single(30));
        assert_eq!(cron.hour, CronField::Single(14));
    }

    /// Validates `CronExpression::parse` behavior for the parse range scenario.
    ///
    /// Assertions:
    /// - Confirms `cron.hour` equals `CronField::Range(9, 17)`.
    #[test]
    fn test_parse_range() {
        let cron = CronExpression::parse("0 9-17 * * *").unwrap();
        assert_eq!(cron.hour, CronField::Range(9, 17));
    }

    /// Validates `CronExpression::parse` behavior for the parse list scenario.
    ///
    /// Assertions:
    /// - Confirms `cron.weekday` equals `CronField::List(vec![1, 3, 5])`.
    #[test]
    fn test_parse_list() {
        let cron = CronExpression::parse("0 0 * * 1,3,5").unwrap();
        assert_eq!(cron.weekday, CronField::List(vec![1, 3, 5]));
    }

    /// Validates `CronExpression::parse` behavior for the parse step scenario.
    ///
    /// Assertions:
    /// - Confirms `cron.minute` equals `CronField::Step(0, 5)`.
    #[test]
    fn test_parse_step() {
        let cron = CronExpression::parse("*/5 * * * *").unwrap();
        assert_eq!(cron.minute, CronField::Step(0, 5));
    }

    /// Validates `CronExpression::parse` behavior for the matches scenario.
    ///
    /// Assertions:
    /// - Ensures `cron.matches(&dt)` evaluates to true.
    /// - Ensures `!cron.matches(&dt)` evaluates to true.
    #[test]
    fn test_matches() {
        let cron = CronExpression::parse("30 14 * * *").unwrap();

        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 14, 30, 0).unwrap();
        assert!(cron.matches(&dt));

        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 14, 31, 0).unwrap();
        assert!(!cron.matches(&dt));
    }

    /// Validates `CronExpression::parse` behavior for the next after scenario.
    ///
    /// Assertions:
    /// - Confirms `next.hour()` equals `0`.
    /// - Confirms `next.minute()` equals `0`.
    /// - Ensures `next > dt` evaluates to true.
    #[test]
    fn test_next_after() {
        let cron = CronExpression::parse("0 0 * * *").unwrap();

        let dt = Utc.with_ymd_and_hms(2024, 1, 1, 12, 0, 0).unwrap();
        let next = cron.next_after(&dt).unwrap();

        assert_eq!(next.hour(), 0);
        assert_eq!(next.minute(), 0);
        assert!(next > dt);
    }

    /// Validates `CronExpression::parse` behavior for the invalid expression
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `CronExpression::parse("invalid").is_err()` evaluates to true.
    /// - Ensures `CronExpression::parse("60 * * * *").is_err()` evaluates to
    ///   true.
    /// - Ensures `CronExpression::parse("* 25 * * *").is_err()` evaluates to
    ///   true.
    #[test]
    fn test_invalid_expression() {
        assert!(CronExpression::parse("invalid").is_err());
        assert!(CronExpression::parse("60 * * * *").is_err());
        assert!(CronExpression::parse("* 25 * * *").is_err());
    }

    /// Validates `CronSchedule::new` behavior for the cron schedule scenario.
    ///
    /// Assertions:
    /// - Ensures `next.is_some()` evaluates to true.
    #[test]
    fn test_cron_schedule() {
        let schedule = CronSchedule::new("0 * * * *").unwrap();
        let next = schedule.next();
        assert!(next.is_some());
    }
}
