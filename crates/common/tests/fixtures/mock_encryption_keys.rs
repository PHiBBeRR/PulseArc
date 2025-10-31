//! Encryption Key Test Fixtures
//!
//! **What this file contains:**
//! Test fixtures for generating encryption keys and measuring performance.
//!
//! ## EncryptionKeyFixture
//! Builder for creating encryption keys and key sources:
//! - `generate()` - Random encryption key
//! - `fixed(length)` - Deterministic key for reproducible tests
//! - `direct_source()` - Direct key source
//! - `env_source()` - Environment variable key source
//! - `keychain_source()` - Keychain key source
//!
//! ## PerformanceMeasurement
//! Utility for measuring and asserting operation duration:
//! - `start(operation)` - Begin timing
//! - `stop()` - End timing and report
//! - `assert_below(duration)` - Assert operation completed within threshold
//!
//! ## Usage
//! ```rust
//! use fixtures::mock_encryption_keys::*;
//!
//! let key = EncryptionKeyFixture::generate();
//! let key_source = EncryptionKeyFixture::direct_source("test");
//!
//! let perf = PerformanceMeasurement::start("encryption test");
//! // ... perform operation ...
//! perf.assert_below(Duration::from_millis(100));
//! ```

use pulsearc_common::security::encryption::{generate_encryption_key, SecureString};
use pulsearc_common::storage::config::KeySource;

// ============================================================================
// Encryption Fixtures
// ============================================================================

/// Test fixture for encryption keys
pub struct EncryptionKeyFixture;

impl EncryptionKeyFixture {
    /// Generate a test encryption key
    pub fn generate() -> SecureString {
        generate_encryption_key()
    }

    /// Create a fixed-length test key (for reproducible tests)
    pub fn fixed(length: usize) -> SecureString {
        let key = "a".repeat(length.max(32)); // Minimum 32 chars
        SecureString::new(key)
    }

    /// Create a key source for direct key testing
    pub fn direct_source(suffix: &str) -> KeySource {
        KeySource::Direct { key: format!("test_key_32_chars_long_{}_aaaaaaaaa", suffix) }
    }

    /// Create a key source from environment variable
    pub fn env_source(var_name: &str) -> KeySource {
        KeySource::Environment { var_name: var_name.to_string() }
    }

    /// Create a key source from keychain
    pub fn keychain_source(service: &str, username: &str) -> KeySource {
        KeySource::Keychain { service: service.to_string(), username: username.to_string() }
    }
}

// ============================================================================
// Performance Test Helpers
// ============================================================================

/// Helper to measure operation duration
pub struct PerformanceMeasurement {
    start: std::time::Instant,
    operation: String,
}

impl PerformanceMeasurement {
    /// Start measuring an operation
    pub fn start(operation: impl Into<String>) -> Self {
        Self { start: std::time::Instant::now(), operation: operation.into() }
    }

    /// Stop and report the measurement
    pub fn stop(self) -> std::time::Duration {
        let duration = self.start.elapsed();
        println!("{} took: {:?}", self.operation, duration);
        duration
    }

    /// Assert duration is below threshold
    pub fn assert_below(self, threshold: std::time::Duration) {
        let duration = self.start.elapsed();
        assert!(
            duration < threshold,
            "{} took {:?}, expected < {:?}",
            self.operation,
            duration,
            threshold
        );
    }
}
