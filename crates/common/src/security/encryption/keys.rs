//! Encryption key generation
//!
//! Provides secure key generation for database encryption.

use rand::Rng;

use super::SecureString;

/// Generate a cryptographically secure 64-character encryption key
///
/// Uses the `rand` crate with a cryptographically secure random number
/// generator to produce a 64-character key from the charset [A-Za-z0-9].
///
/// Returns a `SecureString` that automatically zeroes memory on drop.
///
/// # Example
/// ```
/// use pulsearc_common::security::encryption::keys::generate_encryption_key;
///
/// let key = generate_encryption_key();
/// assert_eq!(key.len(), 64);
/// ```
///
/// # Source
/// Based on macos/db/manager.rs lines 203-214
pub fn generate_encryption_key() -> SecureString {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const KEY_LENGTH: usize = 64;

    let mut rng = rand::thread_rng();

    let key: String = (0..KEY_LENGTH)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();

    SecureString::new(key)
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::keys.
    use std::collections::HashSet;

    use super::*;

    /// Validates the key length scenario.
    ///
    /// Assertions:
    /// - Confirms `key.len()` equals `64`.
    #[test]
    fn test_key_length() {
        let key = generate_encryption_key();
        assert_eq!(key.len(), 64);
    }

    /// Validates the key charset scenario.
    ///
    /// Assertions:
    /// - Ensures `c.is_ascii_alphanumeric()` evaluates to true.
    #[test]
    fn test_key_charset() {
        let key = generate_encryption_key();

        // Check all characters are from the valid charset
        for c in key.expose().chars() {
            assert!(c.is_ascii_alphanumeric(), "Invalid character: {}", c);
        }
    }

    /// Validates `HashSet::new` behavior for the key randomness scenario.
    ///
    /// Assertions:
    /// - Confirms `keys.len()` equals `10`.
    #[test]
    fn test_key_randomness() {
        // Generate multiple keys and ensure they're different
        let mut keys = HashSet::new();

        for _ in 0..10 {
            let key = generate_encryption_key();
            // Convert to string for comparison
            keys.insert(key.expose().to_string());
        }

        // All keys should be unique (with overwhelming probability)
        assert_eq!(keys.len(), 10, "Keys are not random");
    }

    /// Validates the key security scenario.
    ///
    /// Assertions:
    /// - Ensures `unique_chars.len() >= 30` evaluates to true.
    #[test]
    fn test_key_security() {
        let key = generate_encryption_key();

        // Check that the key has reasonable entropy
        // (at least 30 unique characters out of 64)
        let unique_chars: HashSet<char> = key.expose().chars().collect();
        assert!(
            unique_chars.len() >= 30,
            "Key has low entropy: only {} unique characters",
            unique_chars.len()
        );
    }
}
