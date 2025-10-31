//! Test fixture generators
//!
//! Provides functions to generate random test data.
//!
//! For deterministic tests, use the `*_seeded` variants with a fixed seed.

use rand::{Rng, SeedableRng};

/// Generate a random string of specified length
///
/// **Note:** This uses a non-deterministic RNG. For deterministic tests,
/// use [`random_string_seeded`] instead.
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::fixtures::random_string;
///
/// let s = random_string(10);
/// assert_eq!(s.len(), 10);
/// ```
pub fn random_string(len: usize) -> String {
    use rand::distributions::Alphanumeric;

    rand::thread_rng().sample_iter(&Alphanumeric).take(len).map(char::from).collect()
}

/// Generate a random string of specified length with a seed (deterministic)
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::fixtures::random_string_seeded;
///
/// let s1 = random_string_seeded(10, 42);
/// let s2 = random_string_seeded(10, 42);
/// assert_eq!(s1, s2); // Same seed produces same output
/// assert_eq!(s1.len(), 10);
/// ```
pub fn random_string_seeded(len: usize, seed: u64) -> String {
    use rand::distributions::Alphanumeric;

    let rng = rand::rngs::StdRng::seed_from_u64(seed);
    rng.sample_iter(&Alphanumeric).take(len).map(char::from).collect()
}

/// Generate a random email address
///
/// **Note:** This uses a non-deterministic RNG. For deterministic tests,
/// use [`random_email_seeded`] instead.
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::fixtures::random_email;
///
/// let email = random_email();
/// assert!(email.contains('@'));
/// assert!(email.ends_with(".com"));
/// ```
#[must_use]
pub fn random_email() -> String {
    format!("{}@example.com", random_string(10))
}

/// Generate a random email address with a seed (deterministic)
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::fixtures::random_email_seeded;
///
/// let email1 = random_email_seeded(42);
/// let email2 = random_email_seeded(42);
/// assert_eq!(email1, email2);
/// assert!(email1.contains('@'));
/// ```
#[must_use]
pub fn random_email_seeded(seed: u64) -> String {
    format!("{}@example.com", random_string_seeded(10, seed))
}

/// Generate a random u64
///
/// **Note:** This uses a non-deterministic RNG. For deterministic tests,
/// use [`random_u64_seeded`] instead.
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::fixtures::random_u64;
///
/// let num = random_u64();
/// assert!(num > 0);
/// ```
#[must_use]
pub fn random_u64() -> u64 {
    rand::thread_rng().gen()
}

/// Generate a random u64 with a seed (deterministic)
///
/// # Examples
///
/// ```
/// use pulsearc_common::testing::fixtures::random_u64_seeded;
///
/// let num1 = random_u64_seeded(42);
/// let num2 = random_u64_seeded(42);
/// assert_eq!(num1, num2);
/// ```
#[must_use]
pub fn random_u64_seeded(seed: u64) -> u64 {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    rng.gen()
}

/// Generate a random u32
///
/// **Note:** This uses a non-deterministic RNG.
#[must_use]
pub fn random_u32() -> u32 {
    rand::thread_rng().gen()
}

/// Generate a random u32 with a seed (deterministic)
#[must_use]
pub fn random_u32_seeded(seed: u64) -> u32 {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    rng.gen()
}

/// Generate a random boolean
///
/// **Note:** This uses a non-deterministic RNG.
#[must_use]
pub fn random_bool() -> bool {
    rand::thread_rng().gen()
}

/// Generate a random boolean with a seed (deterministic)
#[must_use]
pub fn random_bool_seeded(seed: u64) -> bool {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    rng.gen()
}

/// Generate a random u64 in a range
///
/// **Note:** This uses a non-deterministic RNG. For deterministic tests,
/// use [`random_u64_range_seeded`] instead.
#[must_use]
pub fn random_u64_range(min: u64, max: u64) -> u64 {
    rand::thread_rng().gen_range(min..max)
}

/// Generate a random u64 in a range with a seed (deterministic)
#[must_use]
pub fn random_u64_range_seeded(min: u64, max: u64, seed: u64) -> u64 {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    rng.gen_range(min..max)
}

/// Generate a random alphanumeric string
///
/// **Note:** This uses a non-deterministic RNG.
#[must_use]
pub fn random_alphanumeric(len: usize) -> String {
    random_string(len)
}

/// Generate a random numeric string
///
/// **Note:** This uses a non-deterministic RNG.
#[must_use]
pub fn random_numeric(len: usize) -> String {
    (0..len).map(|_| rand::thread_rng().gen_range(0..10).to_string()).collect()
}

/// Generate a random numeric string with a seed (deterministic)
#[must_use]
pub fn random_numeric_seeded(len: usize, seed: u64) -> String {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    (0..len).map(|_| rng.gen_range(0..10).to_string()).collect()
}

/// Generate a random hex string
///
/// **Note:** This uses a non-deterministic RNG.
#[must_use]
pub fn random_hex(len: usize) -> String {
    (0..len).map(|_| format!("{:x}", rand::thread_rng().gen_range(0..16))).collect()
}

/// Generate a random hex string with a seed (deterministic)
#[must_use]
pub fn random_hex_seeded(len: usize, seed: u64) -> String {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    (0..len).map(|_| format!("{:x}", rng.gen_range(0..16))).collect()
}

/// Generate random bytes
///
/// **Note:** This uses a non-deterministic RNG.
#[must_use]
pub fn random_bytes(len: usize) -> Vec<u8> {
    (0..len).map(|_| rand::thread_rng().gen()).collect()
}

/// Generate random bytes with a seed (deterministic)
#[must_use]
pub fn random_bytes_seeded(len: usize, seed: u64) -> Vec<u8> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(seed);
    (0..len).map(|_| rng.gen()).collect()
}

#[cfg(test)]
mod tests {
    //! Unit tests for testing::fixtures.
    use super::*;

    /// Validates the random string scenario.
    ///
    /// Assertions:
    /// - Confirms `s.len()` equals `10`.
    /// - Ensures `s.chars().all(char::is_alphanumeric)` evaluates to true.
    #[test]
    fn test_random_string() {
        let s = random_string(10);
        assert_eq!(s.len(), 10);
        assert!(s.chars().all(char::is_alphanumeric));
    }

    /// Validates the random email scenario.
    ///
    /// Assertions:
    /// - Ensures `email.contains('@')` evaluates to true.
    /// - Ensures `email.ends_with(".com")` evaluates to true.
    #[test]
    fn test_random_email() {
        let email = random_email();
        assert!(email.contains('@'));
        assert!(email.ends_with(".com"));
    }

    /// Validates the random u64 scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_random_u64() {
        let num = random_u64();
        let _ = num; // Just ensure it compiles and runs
    }

    /// Validates the random u64 range scenario.
    ///
    /// Assertions:
    /// - Ensures `(10..20).contains(&num)` evaluates to true.
    #[test]
    fn test_random_u64_range() {
        let num = random_u64_range(10, 20);
        assert!((10..20).contains(&num));
    }

    /// Validates the random numeric scenario.
    ///
    /// Assertions:
    /// - Confirms `s.len()` equals `5`.
    /// - Ensures `s.chars().all(char::is_numeric)` evaluates to true.
    #[test]
    fn test_random_numeric() {
        let s = random_numeric(5);
        assert_eq!(s.len(), 5);
        assert!(s.chars().all(char::is_numeric));
    }

    /// Validates the random hex scenario.
    ///
    /// Assertions:
    /// - Confirms `s.len()` equals `8`.
    /// - Ensures `s.chars().all(|c| c.is_ascii_hexdigit())` evaluates to true.
    #[test]
    fn test_random_hex() {
        let s = random_hex(8);
        assert_eq!(s.len(), 8);
        assert!(s.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Validates the random bytes scenario.
    ///
    /// Assertions:
    /// - Confirms `bytes.len()` equals `16`.
    #[test]
    fn test_random_bytes() {
        let bytes = random_bytes(16);
        assert_eq!(bytes.len(), 16);
    }

    // Tests for seeded variants (determinism)

    /// Validates the random string seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `s1` equals `s2`.
    /// - Confirms `s1.len()` equals `10`.
    #[test]
    fn test_random_string_seeded_deterministic() {
        let s1 = random_string_seeded(10, 42);
        let s2 = random_string_seeded(10, 42);
        assert_eq!(s1, s2);
        assert_eq!(s1.len(), 10);
    }

    /// Validates the random email seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `email1` equals `email2`.
    /// - Ensures `email1.contains('@')` evaluates to true.
    #[test]
    fn test_random_email_seeded_deterministic() {
        let email1 = random_email_seeded(42);
        let email2 = random_email_seeded(42);
        assert_eq!(email1, email2);
        assert!(email1.contains('@'));
    }

    /// Validates the random u64 seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `num1` equals `num2`.
    #[test]
    fn test_random_u64_seeded_deterministic() {
        let num1 = random_u64_seeded(42);
        let num2 = random_u64_seeded(42);
        assert_eq!(num1, num2);
    }

    /// Validates the random u32 seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `num1` equals `num2`.
    #[test]
    fn test_random_u32_seeded_deterministic() {
        let num1 = random_u32_seeded(42);
        let num2 = random_u32_seeded(42);
        assert_eq!(num1, num2);
    }

    /// Validates the random bool seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `bool1` equals `bool2`.
    #[test]
    fn test_random_bool_seeded_deterministic() {
        let bool1 = random_bool_seeded(42);
        let bool2 = random_bool_seeded(42);
        assert_eq!(bool1, bool2);
    }

    /// Validates the random u64 range seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `num1` equals `num2`.
    /// - Ensures `(10..20).contains(&num1)` evaluates to true.
    #[test]
    fn test_random_u64_range_seeded_deterministic() {
        let num1 = random_u64_range_seeded(10, 20, 42);
        let num2 = random_u64_range_seeded(10, 20, 42);
        assert_eq!(num1, num2);
        assert!((10..20).contains(&num1));
    }

    /// Validates the random numeric seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `s1` equals `s2`.
    /// - Confirms `s1.len()` equals `5`.
    /// - Ensures `s1.chars().all(char::is_numeric)` evaluates to true.
    #[test]
    fn test_random_numeric_seeded_deterministic() {
        let s1 = random_numeric_seeded(5, 42);
        let s2 = random_numeric_seeded(5, 42);
        assert_eq!(s1, s2);
        assert_eq!(s1.len(), 5);
        assert!(s1.chars().all(char::is_numeric));
    }

    /// Validates the random hex seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `s1` equals `s2`.
    /// - Confirms `s1.len()` equals `8`.
    /// - Ensures `s1.chars().all(|c| c.is_ascii_hexdigit())` evaluates to true.
    #[test]
    fn test_random_hex_seeded_deterministic() {
        let s1 = random_hex_seeded(8, 42);
        let s2 = random_hex_seeded(8, 42);
        assert_eq!(s1, s2);
        assert_eq!(s1.len(), 8);
        assert!(s1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Validates the random bytes seeded deterministic scenario.
    ///
    /// Assertions:
    /// - Confirms `bytes1` equals `bytes2`.
    /// - Confirms `bytes1.len()` equals `16`.
    #[test]
    fn test_random_bytes_seeded_deterministic() {
        let bytes1 = random_bytes_seeded(16, 42);
        let bytes2 = random_bytes_seeded(16, 42);
        assert_eq!(bytes1, bytes2);
        assert_eq!(bytes1.len(), 16);
    }
}
