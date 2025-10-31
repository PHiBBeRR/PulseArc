//! Custom matchers for complex assertions
//!
//! Provides matcher functions that can be used with assertions.
//!
//! Note: This is a test utilities module. Error and panic documentation is
//! intentionally minimal as these functions are designed for use in test code
//! where detailed error documentation is less critical than in production APIs.

// Allow missing error/panic docs for test utilities - they are designed to be self-explanatory
// and are used in test contexts where comprehensive documentation is less critical
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

use std::fmt::Debug;

use regex::Regex;

/// Matcher result type
pub type MatchResult = Result<(), String>;

/// Check if a string contains a substring
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::matchers::contains_string;
///
/// assert!(contains_string("Hello World", "World").is_ok());
/// assert!(contains_string("Hello World", "Foo").is_err());
/// ```
pub fn contains_string(haystack: &str, needle: &str) -> MatchResult {
    if haystack.contains(needle) {
        Ok(())
    } else {
        Err(format!("String '{}' does not contain '{}'", haystack, needle))
    }
}

/// Check if a result is Ok
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::matchers::is_ok;
///
/// let result: Result<i32, String> = Ok(42);
/// assert!(is_ok(&result).is_ok());
/// ```
pub fn is_ok<T, E: Debug>(result: &Result<T, E>) -> MatchResult {
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Expected Ok but got Err: {:?}", e)),
    }
}

/// Check if a result is Err
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::matchers::is_error;
///
/// let result: Result<i32, String> = Err("error".to_string());
/// assert!(is_error(&result).is_ok());
/// ```
pub fn is_error<T: Debug, E>(result: &Result<T, E>) -> MatchResult {
    match result {
        Ok(v) => Err(format!("Expected Err but got Ok: {:?}", v)),
        Err(_) => Ok(()),
    }
}

/// Check if a string matches a regex pattern
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::matchers::matches_pattern;
///
/// assert!(matches_pattern("test@example.com", r"^\w+@\w+\.\w+$").is_ok());
/// ```
pub fn matches_pattern(text: &str, pattern: &str) -> MatchResult {
    let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;

    if regex.is_match(text) {
        Ok(())
    } else {
        Err(format!("String '{}' does not match pattern '{}'", text, pattern))
    }
}

/// Check if a value is within a range
pub fn in_range<T: PartialOrd + Debug>(value: T, min: T, max: T) -> MatchResult {
    if value >= min && value <= max {
        Ok(())
    } else {
        Err(format!("Value {:?} not in range [{:?}, {:?}]", value, min, max))
    }
}

/// Check if a collection is empty
pub fn is_empty<T>(collection: &[T]) -> MatchResult {
    if collection.is_empty() {
        Ok(())
    } else {
        Err(format!("Collection is not empty (len={})", collection.len()))
    }
}

/// Check if a collection is not empty
pub fn is_not_empty<T>(collection: &[T]) -> MatchResult {
    if !collection.is_empty() {
        Ok(())
    } else {
        Err("Collection is empty".to_string())
    }
}

/// Check if a collection has a specific length
pub fn has_length<T>(collection: &[T], expected: usize) -> MatchResult {
    let actual = collection.len();
    if actual == expected {
        Ok(())
    } else {
        Err(format!("Collection length mismatch: expected {}, got {}", expected, actual))
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::matchers.
    use super::*;

    /// Validates the contains string scenario.
    ///
    /// Assertions:
    /// - Ensures `contains_string("Hello World", "World").is_ok()` evaluates to
    ///   true.
    /// - Ensures `contains_string("Hello World", "Foo").is_err()` evaluates to
    ///   true.
    #[test]
    fn test_contains_string() {
        assert!(contains_string("Hello World", "World").is_ok());
        assert!(contains_string("Hello World", "Foo").is_err());
    }

    /// Validates the is ok scenario.
    ///
    /// Assertions:
    /// - Ensures `is_ok(&result).is_ok()` evaluates to true.
    /// - Ensures `is_ok(&result).is_err()` evaluates to true.
    #[test]
    fn test_is_ok() {
        let result: Result<i32, String> = Ok(42);
        assert!(is_ok(&result).is_ok());

        let result: Result<i32, String> = Err("error".to_string());
        assert!(is_ok(&result).is_err());
    }

    /// Validates the is error scenario.
    ///
    /// Assertions:
    /// - Ensures `is_error(&result).is_ok()` evaluates to true.
    /// - Ensures `is_error(&result).is_err()` evaluates to true.
    #[test]
    fn test_is_error() {
        let result: Result<i32, String> = Err("error".to_string());
        assert!(is_error(&result).is_ok());

        let result: Result<i32, String> = Ok(42);
        assert!(is_error(&result).is_err());
    }

    /// Validates the matches pattern scenario.
    ///
    /// Assertions:
    /// - Ensures `matches_pattern("test@example.com",
    ///   r"^\w+@\w+\.\w+$").is_ok()` evaluates to true.
    /// - Ensures `matches_pattern("not-an-email", r"^\w+@\w+\.\w+$").is_err()`
    ///   evaluates to true.
    #[test]
    fn test_matches_pattern() {
        assert!(matches_pattern("test@example.com", r"^\w+@\w+\.\w+$").is_ok());
        assert!(matches_pattern("not-an-email", r"^\w+@\w+\.\w+$").is_err());
    }

    /// Validates the in range scenario.
    ///
    /// Assertions:
    /// - Ensures `in_range(5, 1, 10).is_ok()` evaluates to true.
    /// - Ensures `in_range(0, 1, 10).is_err()` evaluates to true.
    /// - Ensures `in_range(11, 1, 10).is_err()` evaluates to true.
    #[test]
    fn test_in_range() {
        assert!(in_range(5, 1, 10).is_ok());
        assert!(in_range(0, 1, 10).is_err());
        assert!(in_range(11, 1, 10).is_err());
    }

    /// Validates the is empty scenario.
    ///
    /// Assertions:
    /// - Ensures `is_empty(&empty).is_ok()` evaluates to true.
    /// - Ensures `is_empty(&not_empty).is_err()` evaluates to true.
    #[test]
    fn test_is_empty() {
        let empty: Vec<i32> = vec![];
        assert!(is_empty(&empty).is_ok());

        let not_empty = vec![1, 2, 3];
        assert!(is_empty(&not_empty).is_err());
    }

    /// Validates the has length scenario.
    ///
    /// Assertions:
    /// - Ensures `has_length(&vec, 3).is_ok()` evaluates to true.
    /// - Ensures `has_length(&vec, 5).is_err()` evaluates to true.
    #[test]
    fn test_has_length() {
        let vec = vec![1, 2, 3];
        assert!(has_length(&vec, 3).is_ok());
        assert!(has_length(&vec, 5).is_err());
    }
}
