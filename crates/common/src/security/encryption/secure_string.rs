//! Secure string type with automatic memory zeroization
//!
//! Provides a wrapper around String that automatically zeroes memory on drop,
//! preventing sensitive data from remaining in memory.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Secure string that zeroes memory on drop
///
/// Wraps a String and ensures the underlying memory is zeroed when the
/// value is dropped, preventing sensitive data from remaining in memory.
///
/// # Security Note
/// While this type implements Eq and Hash for convenience, prefer using
/// `constant_time_eq()` for security-sensitive comparisons to prevent timing
/// attacks.
#[derive(Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct SecureString {
    #[serde(skip_serializing)]
    inner: String,
}

impl SecureString {
    /// Create a new secure string
    pub fn new(s: String) -> Self {
        Self { inner: s }
    }

    /// Expose the inner value (use with caution)
    ///
    /// # Security Warning
    /// The exposed value should not be stored or logged.
    /// Use only for immediate operations that require the string value.
    pub fn expose(&self) -> &str {
        &self.inner
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get an iterator over the characters
    ///
    /// # Security Warning
    /// This exposes the characters of the secure string. Use only when
    /// necessary and avoid storing the characters.
    pub fn chars(&self) -> std::str::Chars<'_> {
        self.inner.chars()
    }

    /// Compare with another secure string in constant time
    pub fn constant_time_eq(&self, other: &SecureString) -> bool {
        constant_time_eq(self.expose().as_bytes(), other.expose().as_bytes())
    }
}

// PartialEq and Eq implementations for convenience
// Note: These are NOT constant-time. Use constant_time_eq() for
// security-sensitive comparisons.
impl PartialEq for SecureString {
    fn eq(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

impl Eq for SecureString {}

// Hash implementation for use in HashSet/HashMap
// Note: This may leak information through timing. Use with caution in security
// contexts.
impl std::hash::Hash for SecureString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.hash(state);
    }
}

impl FromStr for SecureString {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.to_string()))
    }
}

impl fmt::Debug for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SecureString(***)")
    }
}

impl fmt::Display for SecureString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "***")
    }
}

/// Constant-time comparison to prevent timing attacks
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        result |= x ^ y;
    }

    result == 0
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::secure_string.
    use super::*;

    /// Validates `SecureString::new` behavior for the secure string creation
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `s.len()` equals `4`.
    /// - Confirms `s.expose()` equals `"test"`.
    #[test]
    fn test_secure_string_creation() {
        let s = SecureString::new("test".to_string());
        assert_eq!(s.len(), 4);
        assert_eq!(s.expose(), "test");
    }

    /// Validates the secure string from str scenario.
    ///
    /// Assertions:
    /// - Confirms `s.len()` equals `4`.
    #[test]
    fn test_secure_string_from_str() {
        let s = "test".parse::<SecureString>().unwrap();
        assert_eq!(s.len(), 4);
    }

    /// Validates `SecureString::new` behavior for the secure string empty
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `s.is_empty()` evaluates to true.
    #[test]
    fn test_secure_string_empty() {
        let s = SecureString::new(String::new());
        assert!(s.is_empty());
    }

    /// Validates `SecureString::new` behavior for the secure string debug
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `debug_str` equals `"SecureString(***)"`.
    /// - Ensures `!debug_str.contains("secret")` evaluates to true.
    #[test]
    fn test_secure_string_debug() {
        let s = SecureString::new("secret".to_string());
        let debug_str = format!("{:?}", s);
        assert_eq!(debug_str, "SecureString(***)");
        assert!(!debug_str.contains("secret"));
    }

    /// Validates `SecureString::new` behavior for the secure string display
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `display_str` equals `"***"`.
    /// - Ensures `!display_str.contains("secret")` evaluates to true.
    #[test]
    fn test_secure_string_display() {
        let s = SecureString::new("secret".to_string());
        let display_str = format!("{}", s);
        assert_eq!(display_str, "***");
        assert!(!display_str.contains("secret"));
    }

    /// Validates `SecureString::new` behavior for the constant time eq
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `s1.constant_time_eq(&s2)` evaluates to true.
    /// - Ensures `!s1.constant_time_eq(&s3)` evaluates to true.
    #[test]
    fn test_constant_time_eq() {
        let s1 = SecureString::new("test".to_string());
        let s2 = SecureString::new("test".to_string());
        let s3 = SecureString::new("different".to_string());

        assert!(s1.constant_time_eq(&s2));
        assert!(!s1.constant_time_eq(&s3));
    }

    /// Validates `SecureString::new` behavior for the constant time eq
    /// different lengths scenario.
    ///
    /// Assertions:
    /// - Ensures `!s1.constant_time_eq(&s2)` evaluates to true.
    #[test]
    fn test_constant_time_eq_different_lengths() {
        let s1 = SecureString::new("short".to_string());
        let s2 = SecureString::new("much longer string".to_string());

        assert!(!s1.constant_time_eq(&s2));
    }
}
