//! Human-readable duration formatting
//!
//! Provides utilities to format durations into human-readable strings.

use std::time::Duration;

/// Format a duration into a human-readable string
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::format::format_duration;
///
/// assert_eq!(format_duration(Duration::from_secs(5)), "5s");
/// assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
/// assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
/// # }
/// ```
pub fn format_duration(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs == 0 {
        let millis = duration.as_millis();
        if millis == 0 {
            return format!("{}us", duration.as_micros());
        }
        return format!("{}ms", millis);
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let components = [(days, "d"), (hours, "h"), (minutes, "m"), (seconds, "s")];
    let start_index =
        components.iter().position(|(value, _)| *value > 0).unwrap_or(components.len() - 1);

    components[start_index..]
        .iter()
        .map(|(value, suffix)| format!("{value}{suffix}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Format a duration with milliseconds precision
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::format::format_duration_ms;
///
/// assert_eq!(format_duration_ms(Duration::from_millis(1500)), "1s 500ms");
/// assert_eq!(format_duration_ms(Duration::from_millis(500)), "500ms");
/// # }
/// ```
pub fn format_duration_ms(duration: Duration) -> String {
    let total_millis = duration.as_millis();
    let seconds = total_millis / 1000;
    let millis = total_millis % 1000;

    if seconds == 0 {
        return format!("{}ms", millis);
    }

    let formatted = format_duration(Duration::from_secs(seconds as u64));

    if millis > 0 {
        format!("{} {}ms", formatted, millis)
    } else {
        formatted
    }
}

/// Format a duration as a compact string (e.g., "1h30m")
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::format::format_duration_compact;
///
/// assert_eq!(format_duration_compact(Duration::from_secs(5400)), "1h30m0s");
/// assert_eq!(format_duration_compact(Duration::from_secs(65)), "1m5s");
/// # }
/// ```
pub fn format_duration_compact(duration: Duration) -> String {
    format_duration(duration).replace(' ', "")
}

/// Format a duration in a verbose, human-friendly way
///
/// # Examples
///
/// ```
/// # #[cfg(feature = "runtime")]
/// # {
/// use std::time::Duration;
///
/// use pulsearc_common::time::format::format_duration_verbose;
///
/// assert_eq!(format_duration_verbose(Duration::from_secs(5)), "5 seconds");
/// assert_eq!(format_duration_verbose(Duration::from_secs(65)), "1 minute 5 seconds");
/// assert_eq!(format_duration_verbose(Duration::from_secs(3665)), "1 hour 1 minute 5 seconds");
/// # }
/// ```
pub fn format_duration_verbose(duration: Duration) -> String {
    let total_secs = duration.as_secs();

    if total_secs == 0 {
        let millis = duration.as_millis();
        if millis == 0 {
            return format!(
                "{} microsecond{}",
                duration.as_micros(),
                if duration.as_micros() == 1 { "" } else { "s" }
            );
        }
        return format!("{} millisecond{}", millis, if millis == 1 { "" } else { "s" });
    }

    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    let mut parts = Vec::new();

    if days > 0 {
        parts.push(format!("{} day{}", days, if days == 1 { "" } else { "s" }));
    }
    if hours > 0 {
        parts.push(format!("{} hour{}", hours, if hours == 1 { "" } else { "s" }));
    }
    if minutes > 0 {
        parts.push(format!("{} minute{}", minutes, if minutes == 1 { "" } else { "s" }));
    }
    if seconds > 0 || parts.is_empty() {
        parts.push(format!("{} second{}", seconds, if seconds == 1 { "" } else { "s" }));
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    //! Unit tests for time::format.
    use super::*;

    /// Validates `Duration::from_secs` behavior for the format seconds
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration(Duration::from_secs(5))` equals `"5s"`.
    /// - Confirms `format_duration(Duration::from_secs(59))` equals `"59s"`.
    #[test]
    fn test_format_seconds() {
        assert_eq!(format_duration(Duration::from_secs(5)), "5s");
        assert_eq!(format_duration(Duration::from_secs(59)), "59s");
    }

    /// Validates `Duration::from_secs` behavior for the format minutes
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration(Duration::from_secs(60))` equals `"1m 0s"`.
    /// - Confirms `format_duration(Duration::from_secs(65))` equals `"1m 5s"`.
    #[test]
    fn test_format_minutes() {
        assert_eq!(format_duration(Duration::from_secs(60)), "1m 0s");
        assert_eq!(format_duration(Duration::from_secs(65)), "1m 5s");
    }

    /// Validates `Duration::from_secs` behavior for the format hours scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration(Duration::from_secs(3600))` equals `"1h 0m
    ///   0s"`.
    /// - Confirms `format_duration(Duration::from_secs(3665))` equals `"1h 1m
    ///   5s"`.
    #[test]
    fn test_format_hours() {
        assert_eq!(format_duration(Duration::from_secs(3600)), "1h 0m 0s");
        assert_eq!(format_duration(Duration::from_secs(3665)), "1h 1m 5s");
    }

    /// Validates `Duration::from_secs` behavior for the format days scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration(Duration::from_secs(86400))` equals `"1d 0h
    ///   0m 0s"`.
    /// - Confirms `format_duration(Duration::from_secs(90061))` equals `"1d 1h
    ///   1m 1s"`.
    #[test]
    fn test_format_days() {
        assert_eq!(format_duration(Duration::from_secs(86400)), "1d 0h 0m 0s");
        assert_eq!(format_duration(Duration::from_secs(90061)), "1d 1h 1m 1s");
    }

    /// Validates `Duration::from_millis` behavior for the format milliseconds
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration(Duration::from_millis(500))` equals
    ///   `"500ms"`.
    /// - Confirms `format_duration_ms(Duration::from_millis(1500))` equals `"1s
    ///   500ms"`.
    #[test]
    fn test_format_milliseconds() {
        assert_eq!(format_duration(Duration::from_millis(500)), "500ms");
        assert_eq!(format_duration_ms(Duration::from_millis(1500)), "1s 500ms");
    }

    /// Validates `Duration::from_micros` behavior for the format microseconds
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration(Duration::from_micros(100))` equals
    ///   `"100us"`.
    #[test]
    fn test_format_microseconds() {
        assert_eq!(format_duration(Duration::from_micros(100)), "100us");
    }

    /// Validates `Duration::from_secs` behavior for the format compact
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration_compact(Duration::from_secs(5400))` equals
    ///   `"1h30m0s"`.
    /// - Confirms `format_duration_compact(Duration::from_secs(65))` equals
    ///   `"1m5s"`.
    #[test]
    fn test_format_compact() {
        assert_eq!(format_duration_compact(Duration::from_secs(5400)), "1h30m0s");
        assert_eq!(format_duration_compact(Duration::from_secs(65)), "1m5s");
    }

    /// Validates `Duration::from_secs` behavior for the format verbose
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `format_duration_verbose(Duration::from_secs(5))` equals `"5
    ///   seconds"`.
    /// - Confirms `format_duration_verbose(Duration::from_secs(1))` equals `"1
    ///   second"`.
    /// - Confirms `format_duration_verbose(Duration::from_secs(65))` equals `"1
    ///   minute 5 seconds"`.
    /// - Confirms `format_duration_verbose(Duration::from_secs(3665))` equals
    ///   `"1 hour 1 minute 5 seconds"`.
    #[test]
    fn test_format_verbose() {
        assert_eq!(format_duration_verbose(Duration::from_secs(5)), "5 seconds");
        assert_eq!(format_duration_verbose(Duration::from_secs(1)), "1 second");
        assert_eq!(format_duration_verbose(Duration::from_secs(65)), "1 minute 5 seconds");
        assert_eq!(format_duration_verbose(Duration::from_secs(3665)), "1 hour 1 minute 5 seconds");
    }
}
