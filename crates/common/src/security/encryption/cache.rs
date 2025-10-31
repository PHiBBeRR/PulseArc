//! Key caching for performance optimization
//!
//! Provides thread-safe caching of encryption keys to avoid repeated
//! keychain access or environment variable lookups.

use std::sync::OnceLock;

use tracing::{info, warn};

use crate::security::encryption::SecureString;
use crate::storage::config::KeySource;
use crate::storage::error::StorageResult;

/// Thread-safe cached key storage
///
/// Uses `OnceLock` to ensure the key is retrieved only once per process
/// lifetime, reducing keychain access overhead.
static CACHED_KEY: OnceLock<SecureString> = OnceLock::new();

/// Get or create encryption key with caching
///
/// On first call, retrieves/generates the key from the specified source
/// and caches it. Subsequent calls return the cached key instantly.
///
/// # Performance
/// - First call: Normal keychain/env lookup time
/// - Subsequent calls: ~0ms (memory access only)
///
/// # Arguments
/// * `key_source` - Source to retrieve the key from (only used on first call)
///
/// # Returns
/// A static reference to the cached `SecureString`
///
/// # Example
/// ```no_run
/// use pulsearc_common::security::encryption::cache::get_or_create_key_cached;
/// use pulsearc_common::storage::config::KeySource;
///
/// let key_source = KeySource::Keychain {
///     service: "PulseArc".to_string(),
///     username: "db_encryption_key".to_string(),
/// };
///
/// // First call - retrieves from keychain
/// let key1 = get_or_create_key_cached(&key_source).unwrap();
///
/// // Second call - instant (cached)
/// let key2 = get_or_create_key_cached(&key_source).unwrap();
///
/// // Both are the same reference
/// assert!(std::ptr::eq(key1, key2));
/// ```
///
/// # Thread Safety
/// Safe to call from multiple threads concurrently. The `OnceLock`
/// ensures only one thread performs the initialization.
pub fn get_or_create_key_cached(key_source: &KeySource) -> StorageResult<&'static SecureString> {
    // Check if already initialized
    if let Some(key) = CACHED_KEY.get() {
        return Ok(key);
    }

    // Initialize the key
    info!("Initializing encryption key (will be cached for process lifetime)");
    let key = crate::security::encryption::get_or_create_key(key_source)?;

    // Try to cache it - if another thread already cached, use theirs
    Ok(CACHED_KEY.get_or_init(|| key))
}

/// Check if a key is currently cached
///
/// Returns `true` if a key has been cached, `false` otherwise.
/// Useful for testing or diagnostics.
pub fn is_cached() -> bool {
    CACHED_KEY.get().is_some()
}

/// Clear the cached key
///
/// # Security Note
/// This function is intended for use during security events (e.g., suspected
/// compromise, policy changes) or key rotation procedures.
///
/// # Current Limitation
/// `OnceLock` doesn't support clearing in stable Rust (as of Rust 1.77).
/// This function logs the clear request for audit purposes, but the actual
/// cache clearing requires a **process restart**.
///
/// # Recommended Procedure for Key Rotation
/// 1. Call `clear_cache()` to log the security event
/// 2. Generate or retrieve new key from key source
/// 3. Restart the application process
/// 4. New process will load the new key on first access
///
/// # Future Enhancement
/// When `OnceLock::take()` or similar becomes available in stable Rust,
/// this will properly clear the cache without requiring process restart.
///
/// # Example
/// ```
/// use pulsearc_common::security::encryption::cache::clear_cache;
///
/// // On security event or key rotation
/// clear_cache(); // Logs event, requires restart for effect
/// ```
pub fn clear_cache() {
    if is_cached() {
        warn!("Key cache clear requested - this is a security-sensitive operation");
        warn!(
            "IMPORTANT: Process restart required for cache clear to take effect (OnceLock limitation)"
        );
        info!(
            "Key rotation procedure: (1) Clear cache, (2) Update key source, (3) Restart process"
        );
    } else {
        info!("Cache clear requested but no key is currently cached");
    }
}

/// Force clear cache on security event
///
/// Similar to `clear_cache()` but includes additional security event logging.
/// Use this when clearing the cache due to a security incident.
///
/// # Security Event Logging
/// This function logs the cache clear as a security event for audit trails.
///
/// # Arguments
/// * `reason` - Reason for the security event (e.g., "suspected_compromise",
///   "key_rotation")
///
/// # Example
/// ```
/// use pulsearc_common::security::encryption::cache::clear_cache_on_security_event;
///
/// // On suspected compromise
/// clear_cache_on_security_event("suspected_key_compromise");
/// ```
pub fn clear_cache_on_security_event(reason: &str) {
    warn!(
        reason = %reason,
        "SECURITY EVENT: Key cache clear requested"
    );
    clear_cache();
}

/// Get statistics about cache usage
#[derive(Debug)]
pub struct CacheStats {
    pub is_cached: bool,
    pub key_length: Option<usize>,
}

/// Get cache statistics
///
/// Useful for monitoring and diagnostics.
pub fn get_cache_stats() -> CacheStats {
    let key = CACHED_KEY.get();
    CacheStats { is_cached: key.is_some(), key_length: key.map(|k| k.len()) }
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::encryption::cache.
    use super::*;
    use crate::storage::config::KeySource;

    /// Validates `KeySource::Direct` behavior for the key caching scenario.
    ///
    /// Assertions:
    /// - Confirms `key1.len()` equals `33`.
    /// - Ensures `std::ptr::eq(key1, key2)` evaluates to true.
    #[test]
    fn test_key_caching() {
        // Note: This test uses a separate key source to avoid interfering
        // with other tests, but in a real process, only one key is cached

        let key_source = KeySource::Direct { key: "test_key_32_chars_long_aaaaaaaaaa".to_string() };

        // First call
        let key1 = get_or_create_key_cached(&key_source).unwrap();
        assert_eq!(key1.len(), 33); // The test key length

        // Second call should return the same instance
        let key2 = get_or_create_key_cached(&key_source).unwrap();

        // Verify they're the exact same reference (pointer equality)
        assert!(std::ptr::eq(key1, key2));
    }

    /// Validates `KeySource::Direct` behavior for the is cached scenario.
    ///
    /// Assertions:
    /// - Ensures `is_cached()` evaluates to true.
    #[test]
    fn test_is_cached() {
        // After the previous test, key should be cached
        // Note: In a real scenario, test isolation might clear this
        let key_source = KeySource::Direct { key: "test_key_32_chars_long_aaaaaaaaaa".to_string() };

        get_or_create_key_cached(&key_source).unwrap();
        assert!(is_cached());
    }

    /// Validates `KeySource::Direct` behavior for the cache stats scenario.
    ///
    /// Assertions:
    /// - Ensures `stats.is_cached` evaluates to true.
    /// - Ensures `stats.key_length.is_some()` evaluates to true.
    #[test]
    fn test_cache_stats() {
        let key_source = KeySource::Direct { key: "test_key_32_chars_long_aaaaaaaaaa".to_string() };

        get_or_create_key_cached(&key_source).unwrap();

        let stats = get_cache_stats();
        assert!(stats.is_cached);
        assert!(stats.key_length.is_some());
    }

    /// Validates `KeySource::Direct` behavior for the clear cache doesnt panic
    /// scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_clear_cache_doesnt_panic() {
        // Clear should not panic even if cache is empty or full
        clear_cache();

        let key_source = KeySource::Direct { key: "test_key_32_chars_long_aaaaaaaaaa".to_string() };

        get_or_create_key_cached(&key_source).unwrap();
        clear_cache(); // Should log but not fail
    }

    /// Validates `KeySource::Direct` behavior for the clear cache on security
    /// event scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    fn test_clear_cache_on_security_event() {
        use super::clear_cache_on_security_event;

        let key_source = KeySource::Direct { key: "test_key_32_chars_long_aaaaaaaaaa".to_string() };

        get_or_create_key_cached(&key_source).unwrap();

        // Should not panic
        clear_cache_on_security_event("test_security_event");
    }

    /// Validates `Arc::new` behavior for the thread safety scenario.
    ///
    /// Assertions:
    /// - Ensures `std::ptr::eq(keys[0], keys[i])` evaluates to true.
    #[test]
    fn test_thread_safety() {
        use std::sync::Arc;
        use std::thread;

        let key_source =
            Arc::new(KeySource::Direct { key: "test_key_32_chars_long_aaaaaaaaaa".to_string() });

        // Spawn multiple threads trying to access the cache
        let handles: Vec<_> = (0..10)
            .map(|_| {
                let key_source = Arc::clone(&key_source);
                thread::spawn(move || get_or_create_key_cached(&key_source).unwrap())
            })
            .collect();

        // All threads should get the same key
        let keys: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // Verify all pointers are equal
        for i in 1..keys.len() {
            assert!(std::ptr::eq(keys[0], keys[i]));
        }
    }
}
