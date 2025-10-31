//! Macro for implementing Display and FromStr for status enums
//!
//! This macro eliminates boilerplate for status enum conversions by providing
//! a single implementation for both Display and FromStr traits. It handles
//! case-insensitive parsing and consistent string representation.
//!
//! # Example
//!
//! ```rust
//! use pulsearc_domain::impl_domain_status_conversions;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! pub enum BatchStatus {
//!     Pending,
//!     Processing,
//!     Completed,
//!     Failed,
//! }
//!
//! impl_domain_status_conversions!(BatchStatus {
//!     Pending => "pending",
//!     Processing => "processing",
//!     Completed => "completed",
//!     Failed => "failed",
//! });
//! ```

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
#[macro_export]
macro_rules! impl_domain_status_conversions {
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
    use std::str::FromStr;

    // Test enum for macro validation
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestStatus {
        Pending,
        Processing,
        Completed,
        Failed,
    }

    impl_domain_status_conversions!(TestStatus {
        Pending => "pending",
        Processing => "processing",
        Completed => "completed",
        Failed => "failed",
    });

    #[test]
    fn test_display_conversion() {
        assert_eq!(TestStatus::Pending.to_string(), "pending");
        assert_eq!(TestStatus::Processing.to_string(), "processing");
        assert_eq!(TestStatus::Completed.to_string(), "completed");
        assert_eq!(TestStatus::Failed.to_string(), "failed");
    }

    #[test]
    fn test_fromstr_lowercase() {
        assert_eq!(TestStatus::from_str("pending").unwrap(), TestStatus::Pending);
        assert_eq!(TestStatus::from_str("processing").unwrap(), TestStatus::Processing);
        assert_eq!(TestStatus::from_str("completed").unwrap(), TestStatus::Completed);
        assert_eq!(TestStatus::from_str("failed").unwrap(), TestStatus::Failed);
    }

    #[test]
    fn test_fromstr_uppercase() {
        assert_eq!(TestStatus::from_str("PENDING").unwrap(), TestStatus::Pending);
        assert_eq!(TestStatus::from_str("PROCESSING").unwrap(), TestStatus::Processing);
        assert_eq!(TestStatus::from_str("COMPLETED").unwrap(), TestStatus::Completed);
        assert_eq!(TestStatus::from_str("FAILED").unwrap(), TestStatus::Failed);
    }

    #[test]
    fn test_fromstr_mixed_case() {
        assert_eq!(TestStatus::from_str("Pending").unwrap(), TestStatus::Pending);
        assert_eq!(TestStatus::from_str("ProCessing").unwrap(), TestStatus::Processing);
        assert_eq!(TestStatus::from_str("CompLeted").unwrap(), TestStatus::Completed);
        assert_eq!(TestStatus::from_str("FaILeD").unwrap(), TestStatus::Failed);
    }

    #[test]
    fn test_fromstr_invalid() {
        let result = TestStatus::from_str("invalid");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid TestStatus: invalid"));
    }

    #[test]
    fn test_fromstr_empty() {
        let result = TestStatus::from_str("");
        assert!(result.is_err());
    }

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
