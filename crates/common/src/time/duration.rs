//! Duration parsing from strings
//!
//! Provides utilities to parse duration strings into `std::time::Duration`.

use std::time::Duration;

use thiserror::Error;

/// Error type for duration parsing
#[derive(Debug, Error, Clone, PartialEq)]
pub enum DurationParseError {
    #[error("Invalid duration format: {0}")]
    InvalidFormat(String),

    #[error("Invalid number: {0}")]
    InvalidNumber(String),

    #[error("Unknown unit: {0}")]
    UnknownUnit(String),

    #[error("Empty duration string")]
    EmptyString,
}

/// Parse a duration string into a Duration
///
/// Supports the following formats:
/// - "5s" - 5 seconds
/// - "10m" - 10 minutes
/// - "2h" - 2 hours
/// - "3d" - 3 days
/// - "1h 30m" - 1 hour 30 minutes
/// - "2h 15m 30s" - 2 hours, 15 minutes, 30 seconds
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::duration::parse_duration;
///
/// assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
/// assert_eq!(parse_duration("10m").unwrap(), Duration::from_secs(600));
/// assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
/// assert_eq!(parse_duration("1h 30m").unwrap(), Duration::from_secs(5400));
/// # }
/// ```
pub fn parse_duration(s: &str) -> Result<Duration, DurationParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(DurationParseError::EmptyString);
    }

    let mut total = Duration::ZERO;
    let mut current_number = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() || ch == '.' {
            current_number.push(ch);
        } else if ch.is_whitespace() {
            continue;
        } else {
            // Parse accumulated number
            if current_number.is_empty() {
                return Err(DurationParseError::InvalidFormat(
                    "Expected number before unit".to_string(),
                ));
            }

            let value: f64 = current_number
                .parse()
                .map_err(|_| DurationParseError::InvalidNumber(current_number.clone()))?;

            // Match unit
            let unit_duration = match ch {
                's' => Duration::from_secs_f64(value),
                'm' => Duration::from_secs_f64(value * 60.0),
                'h' => Duration::from_secs_f64(value * 3600.0),
                'd' => Duration::from_secs_f64(value * 86400.0),
                'w' => Duration::from_secs_f64(value * 604800.0),
                _ => return Err(DurationParseError::UnknownUnit(ch.to_string())),
            };

            total += unit_duration;
            current_number.clear();
        }
    }

    if !current_number.is_empty() {
        return Err(DurationParseError::InvalidFormat("Missing unit after number".to_string()));
    }

    Ok(total)
}

/// Parse a duration with milliseconds precision
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::duration::parse_duration_ms;
///
/// assert_eq!(parse_duration_ms("500ms").unwrap(), Duration::from_millis(500));
/// assert_eq!(parse_duration_ms("1s 500ms").unwrap(), Duration::from_millis(1500));
/// # }
/// ```
pub fn parse_duration_ms(s: &str) -> Result<Duration, DurationParseError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(DurationParseError::EmptyString);
    }

    let mut total = Duration::ZERO;
    let mut current_number = String::new();
    let mut chars = s.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_ascii_digit() || ch == '.' {
            current_number.push(ch);
        } else if ch.is_whitespace() {
            continue;
        } else {
            // Check for two-character units
            let unit = if ch == 'm' && chars.peek() == Some(&'s') {
                chars.next(); // consume 's'
                "ms"
            } else if ch == 'u' && chars.peek() == Some(&'s') {
                chars.next(); // consume 's'
                "us"
            } else {
                &ch.to_string()
            };

            if current_number.is_empty() {
                return Err(DurationParseError::InvalidFormat(
                    "Expected number before unit".to_string(),
                ));
            }

            let value: f64 = current_number
                .parse()
                .map_err(|_| DurationParseError::InvalidNumber(current_number.clone()))?;

            let unit_duration = match unit {
                "us" => Duration::from_micros(value as u64),
                "ms" => Duration::from_millis(value as u64),
                "s" => Duration::from_secs_f64(value),
                "m" => Duration::from_secs_f64(value * 60.0),
                "h" => Duration::from_secs_f64(value * 3600.0),
                "d" => Duration::from_secs_f64(value * 86400.0),
                _ => return Err(DurationParseError::UnknownUnit(unit.to_string())),
            };

            total += unit_duration;
            current_number.clear();
        }
    }

    if !current_number.is_empty() {
        return Err(DurationParseError::InvalidFormat("Missing unit after number".to_string()));
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    //! Unit tests for time::duration.
    use super::*;

    /// Validates `Duration::from_secs` behavior for the parse seconds scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("5s").unwrap()` equals
    ///   `Duration::from_secs(5)`.
    /// - Confirms `parse_duration("0s").unwrap()` equals
    ///   `Duration::from_secs(0)`.
    #[test]
    fn test_parse_seconds() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("0s").unwrap(), Duration::from_secs(0));
    }

    /// Validates `Duration::from_secs` behavior for the parse minutes scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("10m").unwrap()` equals
    ///   `Duration::from_secs(600)`.
    #[test]
    fn test_parse_minutes() {
        assert_eq!(parse_duration("10m").unwrap(), Duration::from_secs(600));
    }

    /// Validates `Duration::from_secs` behavior for the parse hours scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("2h").unwrap()` equals
    ///   `Duration::from_secs(7200)`.
    #[test]
    fn test_parse_hours() {
        assert_eq!(parse_duration("2h").unwrap(), Duration::from_secs(7200));
    }

    /// Validates `Duration::from_secs` behavior for the parse days scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("1d").unwrap()` equals
    ///   `Duration::from_secs(86400)`.
    #[test]
    fn test_parse_days() {
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
    }

    /// Validates `Duration::from_secs` behavior for the parse weeks scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("1w").unwrap()` equals
    ///   `Duration::from_secs(604800)`.
    #[test]
    fn test_parse_weeks() {
        assert_eq!(parse_duration("1w").unwrap(), Duration::from_secs(604800));
    }

    /// Validates `Duration::from_secs` behavior for the parse compound
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("1h 30m").unwrap()` equals
    ///   `Duration::from_secs(5400)`.
    /// - Confirms `parse_duration("2h 15m 30s").unwrap()` equals
    ///   `Duration::from_secs(8130)`.
    /// - Confirms `parse_duration("1d 2h 30m").unwrap()` equals
    ///   `Duration::from_secs(95400)`.
    #[test]
    fn test_parse_compound() {
        assert_eq!(parse_duration("1h 30m").unwrap(), Duration::from_secs(5400));
        assert_eq!(parse_duration("2h 15m 30s").unwrap(), Duration::from_secs(8130));
        assert_eq!(parse_duration("1d 2h 30m").unwrap(), Duration::from_secs(95400));
    }

    /// Validates `Duration::from_secs` behavior for the parse with whitespace
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration(" 5s ").unwrap()` equals
    ///   `Duration::from_secs(5)`.
    /// - Confirms `parse_duration("1h 30m").unwrap()` equals
    ///   `Duration::from_secs(5400)`.
    #[test]
    fn test_parse_with_whitespace() {
        assert_eq!(parse_duration("  5s  ").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("1h  30m").unwrap(), Duration::from_secs(5400));
    }

    /// Validates `Duration::from_secs_f64` behavior for the parse decimals
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration("1.5h").unwrap()` equals
    ///   `Duration::from_secs_f64(5400.0)`.
    /// - Confirms `parse_duration("0.5s").unwrap()` equals
    ///   `Duration::from_millis(500)`.
    #[test]
    fn test_parse_decimals() {
        assert_eq!(parse_duration("1.5h").unwrap(), Duration::from_secs_f64(5400.0));
        assert_eq!(parse_duration("0.5s").unwrap(), Duration::from_millis(500));
    }

    /// Validates `Duration::from_millis` behavior for the parse ms scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration_ms("500ms").unwrap()` equals
    ///   `Duration::from_millis(500)`.
    /// - Confirms `parse_duration_ms("1s 500ms").unwrap()` equals
    ///   `Duration::from_millis(1500)`.
    #[test]
    fn test_parse_ms() {
        assert_eq!(parse_duration_ms("500ms").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration_ms("1s 500ms").unwrap(), Duration::from_millis(1500));
    }

    /// Validates `Duration::from_micros` behavior for the parse us scenario.
    ///
    /// Assertions:
    /// - Confirms `parse_duration_ms("1000us").unwrap()` equals
    ///   `Duration::from_micros(1000)`.
    #[test]
    fn test_parse_us() {
        assert_eq!(parse_duration_ms("1000us").unwrap(), Duration::from_micros(1000));
    }

    /// Validates the parse errors scenario.
    ///
    /// Assertions:
    /// - Ensures `parse_duration("").is_err()` evaluates to true.
    /// - Ensures `parse_duration("5").is_err()` evaluates to true.
    /// - Ensures `parse_duration("x").is_err()` evaluates to true.
    /// - Ensures `parse_duration("5x").is_err()` evaluates to true.
    #[test]
    fn test_parse_errors() {
        assert!(parse_duration("").is_err());
        assert!(parse_duration("5").is_err());
        assert!(parse_duration("x").is_err());
        assert!(parse_duration("5x").is_err());
    }
}
