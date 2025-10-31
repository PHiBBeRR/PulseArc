//! Practical examples for cache usage patterns
//!
//! This module demonstrates real-world usage patterns and best practices
//! for the cache implementation.

use std::sync::Arc;
use std::time::Duration;

use super::{Cache, CacheConfig, EvictionPolicy};

type ByteCache = Cache<String, Arc<Vec<u8>>>;
type JsonValueCache = Cache<String, Arc<Vec<serde_json::Value>>>;

/// Example: Using Arc<V> for large values to avoid expensive clones
///
/// When caching large values, wrap them in Arc to make clones cheap.
///
/// # Example
/// ```
/// use std::sync::Arc;
///
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// // Large data structure
/// #[derive(Clone)]
/// struct LargeData {
///     payload: Vec<u8>,
/// }
///
/// // Bad: Expensive clone on every get
/// let cache_bad: Cache<String, LargeData> = Cache::new(CacheConfig::lru(100));
/// // Every get() clones the entire Vec<u8>!
///
/// // Good: Cheap Arc clone
/// let cache_good: Cache<String, Arc<LargeData>> = Cache::new(CacheConfig::lru(100));
/// // get() only clones the Arc pointer (8 bytes)
/// ```
pub fn example_arc_pattern() -> ByteCache {
    let cache: ByteCache = Cache::new(CacheConfig::lru(1000));

    // Insert large value wrapped in Arc
    let large_data = Arc::new(vec![0u8; 1_000_000]); // 1MB
    cache.insert("large_file".to_string(), large_data);

    // Retrieving clones only the Arc (cheap!)
    let _retrieved = cache.get(&"large_file".to_string());

    cache
}

/// Example: API response caching with TTL
///
/// Common pattern for caching HTTP responses with automatic expiration.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// #[derive(Clone)]
/// struct ApiResponse {
///     status: u16,
///     body: String,
///     headers: Vec<(String, String)>,
/// }
///
/// // Cache responses for 5 minutes, max 500 entries
/// let cache: Cache<String, Arc<ApiResponse>> =
///     Cache::new(CacheConfig::ttl_lru(Duration::from_secs(300), 500));
/// ```
pub fn example_api_cache() -> ByteCache {
    // Cache API responses for 5 minutes, max 500 responses
    Cache::new(CacheConfig::ttl_lru(Duration::from_secs(300), 500))
}

/// Example: Session management with LRU eviction
///
/// Common pattern for managing user sessions with automatic cleanup
/// of inactive sessions.
///
/// # Example
/// ```
/// use std::time::Duration;
///
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// #[derive(Clone)]
/// struct UserSession {
///     user_id: String,
///     authenticated_at: std::time::Instant,
///     permissions: Vec<String>,
/// }
///
/// // Sessions expire after 30 minutes, LRU eviction for inactive sessions
/// let sessions: Cache<String, UserSession> =
///     Cache::new(CacheConfig::ttl_lru(Duration::from_secs(1800), 10_000));
/// ```
pub fn example_session_cache() -> JsonValueCache {
    // Sessions expire after 30 minutes of inactivity
    // LRU ensures least recently accessed sessions are evicted first
    Cache::new(CacheConfig::ttl_lru(Duration::from_secs(1800), 10_000))
}

/// Example: Computation memoization with LFU
///
/// For expensive computations, use LFU to keep frequently accessed results.
///
/// # Example
/// ```
/// use pulsearc_common::cache::{Cache, CacheConfig, EvictionPolicy};
///
/// fn expensive_computation(input: &str) -> String {
///     // Simulate expensive operation
///     std::thread::sleep(std::time::Duration::from_millis(100));
///     format!("result_{}", input)
/// }
///
/// let cache: Cache<String, String> = Cache::new(
///     CacheConfig::builder().max_size(1000).eviction_policy(EvictionPolicy::LFU).build(),
/// );
///
/// let result = cache.get_or_insert_with("input1".to_string(), || expensive_computation("input1"));
/// ```
pub fn example_memoization_cache() -> Cache<String, String> {
    // Use LFU to keep frequently accessed computation results
    Cache::new(CacheConfig::builder().max_size(1000).eviction_policy(EvictionPolicy::LFU).build())
}

/// Example: Database query caching with metrics
///
/// Cache database query results with metrics tracking for optimization.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use std::time::Duration;
///
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// #[derive(Clone)]
/// struct QueryResult {
///     rows: Vec<serde_json::Value>,
///     row_count: usize,
/// }
///
/// let cache: Cache<String, Arc<QueryResult>> = Cache::new(
///     CacheConfig::builder()
///         .max_size(500)
///         .ttl(Duration::from_secs(60))
///         .track_metrics(true)
///         .build(),
/// );
///
/// // Periodically check hit rate
/// let stats = cache.stats();
/// if stats.hit_rate() < 0.5 {
///     eprintln!("Low cache hit rate: {:.2}%", stats.hit_rate() * 100.0);
/// }
/// ```
pub fn example_query_cache_with_metrics() -> JsonValueCache {
    Cache::new(
        CacheConfig::builder()
            .max_size(500)
            .ttl(Duration::from_secs(60))
            .track_metrics(true)
            .build(),
    )
}

/// Example: Thread-safe shared cache
///
/// Pattern for sharing cache across multiple threads safely.
///
/// # Example
/// ```
/// use std::sync::Arc;
/// use std::thread;
///
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// let cache = Arc::new(Cache::new(CacheConfig::lru(1000)));
///
/// let mut handles = vec![];
/// for i in 0..10 {
///     let cache_clone = Arc::clone(&cache);
///     let handle = thread::spawn(move || {
///         cache_clone.insert(format!("key_{}", i), i);
///         cache_clone.get(&format!("key_{}", i))
///     });
///     handles.push(handle);
/// }
///
/// for handle in handles {
///     handle.join().unwrap();
/// }
/// ```
pub fn example_thread_safe_cache() -> Arc<Cache<String, i32>> {
    Arc::new(Cache::new(CacheConfig::lru(1000)))
}

/// Example: Lazy initialization with get_or_insert_with
///
/// Avoid computing values that are already cached.
///
/// # Example
/// ```
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// let cache: Cache<String, Vec<String>> = Cache::new(CacheConfig::lru(100));
///
/// // Expensive operation only runs if key doesn't exist
/// let value = cache.get_or_insert_with("users".to_string(), || {
///     // Simulate database query
///     vec!["user1".to_string(), "user2".to_string()]
/// });
/// ```
pub fn example_lazy_initialization() -> Cache<String, Vec<String>> {
    Cache::new(CacheConfig::lru(100))
}

/// Example: Cache warming strategy
///
/// Pre-populate cache with frequently accessed data.
///
/// # Example
/// ```
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// fn warm_cache(cache: &Cache<String, String>) {
///     let frequent_keys = vec!["config", "user_prefs", "feature_flags"];
///     for key in frequent_keys {
///         // Load from database/file and populate cache
///         let value = format!("loaded_{}", key);
///         cache.insert(key.to_string(), value);
///     }
/// }
///
/// let cache = Cache::new(CacheConfig::lru(1000));
/// warm_cache(&cache);
/// ```
pub fn example_cache_warming(cache: &Cache<String, String>, warm_keys: Vec<(String, String)>) {
    for (key, value) in warm_keys {
        cache.insert(key, value);
    }
}

/// Example: Cache key generation best practices
///
/// Generate consistent, unique cache keys.
///
/// # Example
/// ```
/// use sha2::{Digest, Sha256};
///
/// fn generate_cache_key(namespace: &str, params: &[(&str, &str)]) -> String {
///     let mut hasher = Sha256::new();
///     hasher.update(namespace.as_bytes());
///     for (key, value) in params {
///         hasher.update(b":");
///         hasher.update(key.as_bytes());
///         hasher.update(b"=");
///         hasher.update(value.as_bytes());
///     }
///     format!("{:x}", hasher.finalize())
/// }
///
/// let key = generate_cache_key("api", &[("user_id", "123"), ("action", "list")]);
/// ```
pub fn example_generate_cache_key(namespace: &str, params: &[(&str, &str)]) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    for (key, value) in params {
        hasher.update(b":");
        hasher.update(key.as_bytes());
        hasher.update(b"=");
        hasher.update(value.as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates the arc pattern scenario.
    ///
    /// Assertions:
    /// - Ensures `!cache.is_empty()` evaluates to true.
    /// - Confirms `cache.len()` equals `1`.
    #[test]
    fn test_arc_pattern() {
        let cache = example_arc_pattern();
        // Cache was populated by the example function
        assert!(!cache.is_empty());
        assert_eq!(cache.len(), 1);
    }

    /// Validates `Arc::new` behavior for the api cache scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"response".to_string())` equals `Some(data)`.
    #[test]
    fn test_api_cache() {
        let cache = example_api_cache();
        let data = Arc::new(vec![1, 2, 3]);
        cache.insert("response".to_string(), data.clone());
        assert_eq!(cache.get(&"response".to_string()), Some(data));
    }

    /// Validates the session cache scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.is_empty()` evaluates to true.
    #[test]
    fn test_session_cache() {
        let cache = example_session_cache();
        assert!(cache.is_empty());
    }

    /// Validates the memoization cache scenario.
    ///
    /// Assertions:
    /// - Confirms `result` equals `"computed"`.
    /// - Confirms `result2` equals `"computed"`.
    #[test]
    fn test_memoization_cache() {
        let cache = example_memoization_cache();
        let result = cache.get_or_insert_with("test".to_string(), || "computed".to_string());
        assert_eq!(result, "computed");
        // Second call should return cached value
        let result2 =
            cache.get_or_insert_with("test".to_string(), || "should_not_compute".to_string());
        assert_eq!(result2, "computed");
    }

    /// Validates `Arc::new` behavior for the query cache with metrics scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hits` equals `1`.
    /// - Confirms `stats.misses` equals `1`.
    /// - Ensures `(stats.hit_rate() - 0.5).abs() < 0.01` evaluates to true.
    #[test]
    fn test_query_cache_with_metrics() {
        let cache = example_query_cache_with_metrics();
        let data = Arc::new(vec![serde_json::json!({"id": 1})]);
        cache.insert("query".to_string(), data);

        let _ = cache.get(&"query".to_string());
        let _ = cache.get(&"missing".to_string());

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 0.01);
    }

    /// Validates the thread safe cache scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"test".to_string())` equals `Some(42)`.
    #[test]
    fn test_thread_safe_cache() {
        let cache = example_thread_safe_cache();
        cache.insert("test".to_string(), 42);
        assert_eq!(cache.get(&"test".to_string()), Some(42));
    }

    /// Validates `Cache::new` behavior for the cache warming scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `2`.
    #[test]
    fn test_cache_warming() {
        let cache = Cache::new(CacheConfig::lru(100));
        let warm_data = vec![
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ];
        example_cache_warming(&cache, warm_data);
        assert_eq!(cache.len(), 2);
    }

    /// Validates the generate cache key scenario.
    ///
    /// Assertions:
    /// - Confirms `key1` equals `key2`.
    /// - Confirms `key1` differs from `key3`.
    #[test]
    fn test_generate_cache_key() {
        let key1 = example_generate_cache_key("api", &[("user", "123")]);
        let key2 = example_generate_cache_key("api", &[("user", "123")]);
        let key3 = example_generate_cache_key("api", &[("user", "456")]);

        assert_eq!(key1, key2); // Same params = same key
        assert_ne!(key1, key3); // Different params = different key
    }
}
