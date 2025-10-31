//! Macros for reducing boilerplate code
//!
//! This module contains declarative macros that eliminate repetitive
//! implementations across the codebase, particularly for common patterns like
//! status enum conversions.

/// Implements Display and FromStr traits for status enums
///
/// This macro generates:
/// - Display trait: converts enum variants to lowercase strings
/// - FromStr trait: parses case-insensitive strings to enum variants
///
/// # Arguments
///
/// * `$enum_name` - The name of the enum type
/// * `$variant => $str` - Mapping of enum variants to their string
///   representations
///
/// # Features
///
/// - Case-insensitive parsing (e.g., "PENDING", "pending", "Pending" all work)
/// - Consistent lowercase string output
/// - Descriptive error messages with enum name
///
/// # Example
///
/// ```rust
/// use pulsearc_common::impl_status_conversions;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// pub enum BatchStatus {
///     Pending,
///     Processing,
///     Completed,
///     Failed,
/// }
///
/// impl_status_conversions!(BatchStatus {
///     Pending => "pending",
///     Processing => "processing",
///     Completed => "completed",
///     Failed => "failed",
/// });
/// ```
#[macro_export]
macro_rules! impl_status_conversions {
    ($enum_name:ident { $($variant:ident => $str:expr),+ $(,)? }) => {
        impl std::fmt::Display for $enum_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    $(Self::$variant => write!(f, $str),)+
                }
            }
        }

        impl std::str::FromStr for $enum_name {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s.to_lowercase().as_str() {
                    $($str => Ok(Self::$variant),)+
                    _ => Err(format!("Invalid {}: {}", stringify!($enum_name), s)),
                }
            }
        }
    };
}

#[cfg(test)]
mod tests {
    //! Unit tests for utils::macros.
    use std::str::FromStr;

    // Test enum for macro validation
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestStatus {
        Pending,
        Processing,
        Completed,
        Failed,
    }

    impl_status_conversions!(TestStatus {
        Pending => "pending",
        Processing => "processing",
        Completed => "completed",
        Failed => "failed",
    });

    /// Validates `TestStatus::Pending` behavior for the display conversion
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `TestStatus::Pending.to_string()` equals `"pending"`.
    /// - Confirms `TestStatus::Processing.to_string()` equals `"processing"`.
    /// - Confirms `TestStatus::Completed.to_string()` equals `"completed"`.
    /// - Confirms `TestStatus::Failed.to_string()` equals `"failed"`.
    #[test]
    fn test_display_conversion() {
        assert_eq!(TestStatus::Pending.to_string(), "pending");
        assert_eq!(TestStatus::Processing.to_string(), "processing");
        assert_eq!(TestStatus::Completed.to_string(), "completed");
        assert_eq!(TestStatus::Failed.to_string(), "failed");
    }

    /// Validates `TestStatus::from_str` behavior for the fromstr lowercase
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `TestStatus::from_str("pending").unwrap()` equals
    ///   `TestStatus::Pending`.
    /// - Confirms `TestStatus::from_str("processing").unwrap()` equals
    ///   `TestStatus::Processing`.
    /// - Confirms `TestStatus::from_str("completed").unwrap()` equals
    ///   `TestStatus::Completed`.
    /// - Confirms `TestStatus::from_str("failed").unwrap()` equals
    ///   `TestStatus::Failed`.
    #[test]
    fn test_fromstr_lowercase() {
        assert_eq!(TestStatus::from_str("pending").unwrap(), TestStatus::Pending);
        assert_eq!(TestStatus::from_str("processing").unwrap(), TestStatus::Processing);
        assert_eq!(TestStatus::from_str("completed").unwrap(), TestStatus::Completed);
        assert_eq!(TestStatus::from_str("failed").unwrap(), TestStatus::Failed);
    }

    /// Validates `TestStatus::from_str` behavior for the fromstr uppercase
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `TestStatus::from_str("PENDING").unwrap()` equals
    ///   `TestStatus::Pending`.
    /// - Confirms `TestStatus::from_str("PROCESSING").unwrap()` equals
    ///   `TestStatus::Processing`.
    /// - Confirms `TestStatus::from_str("COMPLETED").unwrap()` equals
    ///   `TestStatus::Completed`.
    /// - Confirms `TestStatus::from_str("FAILED").unwrap()` equals
    ///   `TestStatus::Failed`.
    #[test]
    fn test_fromstr_uppercase() {
        assert_eq!(TestStatus::from_str("PENDING").unwrap(), TestStatus::Pending);
        assert_eq!(TestStatus::from_str("PROCESSING").unwrap(), TestStatus::Processing);
        assert_eq!(TestStatus::from_str("COMPLETED").unwrap(), TestStatus::Completed);
        assert_eq!(TestStatus::from_str("FAILED").unwrap(), TestStatus::Failed);
    }

    /// Validates `TestStatus::from_str` behavior for the fromstr mixed case
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `TestStatus::from_str("Pending").unwrap()` equals
    ///   `TestStatus::Pending`.
    /// - Confirms `TestStatus::from_str("ProCessing").unwrap()` equals
    ///   `TestStatus::Processing`.
    /// - Confirms `TestStatus::from_str("CompLeted").unwrap()` equals
    ///   `TestStatus::Completed`.
    /// - Confirms `TestStatus::from_str("FaILeD").unwrap()` equals
    ///   `TestStatus::Failed`.
    #[test]
    fn test_fromstr_mixed_case() {
        assert_eq!(TestStatus::from_str("Pending").unwrap(), TestStatus::Pending);
        assert_eq!(TestStatus::from_str("ProCessing").unwrap(), TestStatus::Processing);
        assert_eq!(TestStatus::from_str("CompLeted").unwrap(), TestStatus::Completed);
        assert_eq!(TestStatus::from_str("FaILeD").unwrap(), TestStatus::Failed);
    }

    /// Validates `TestStatus::from_str` behavior for the fromstr invalid
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("Invalid TestStatus: invalid")`
    ///   evaluates to true.
    #[test]
    fn test_fromstr_invalid() {
        let result = TestStatus::from_str("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid TestStatus: invalid"));
    }

    /// Validates `TestStatus::from_str` behavior for the fromstr empty
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    #[test]
    fn test_fromstr_empty() {
        let result = TestStatus::from_str("");
        assert!(result.is_err());
    }

    /// Validates `TestStatus::Pending` behavior for the roundtrip scenario.
    ///
    /// Assertions:
    /// - Confirms `status` equals `parsed`.
    #[test]
    fn test_roundtrip() {
        let statuses = vec![
            TestStatus::Pending,
            TestStatus::Processing,
            TestStatus::Completed,
            TestStatus::Failed,
        ];

        for status in statuses {
            let string = status.to_string();
            let parsed = TestStatus::from_str(&string).unwrap();
            assert_eq!(status, parsed);
        }
    }
}
