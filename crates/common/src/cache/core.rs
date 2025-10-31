//! Core cache implementation with configurable eviction policies
//!
//! This module provides a generic, thread-safe cache with support for
//! multiple eviction policies (LRU, LFU, FIFO, Random, None) and TTL
//! expiration.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, RwLock};
use std::time::Instant;

use super::config::{CacheConfig, EvictionPolicy};
use super::stats::{CacheStats, MetricsCollector};
use crate::resilience::{Clock, SystemClock};

/// Entry stored in the cache with metadata for eviction policies
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

/// Internal storage for cache entries
#[derive(Debug)]
struct CacheStorage<K, V>
where
    K: Eq + Hash + Clone,
{
    entries: HashMap<K, CacheEntry<V>>,
    /// Tracks order for LRU/FIFO eviction
    access_order: Vec<K>,
}

impl<K, V> CacheStorage<K, V>
where
    K: Eq + Hash + Clone,
{
    fn new() -> Self {
        Self { entries: HashMap::new(), access_order: Vec::new() }
    }
}

/// Generic thread-safe cache with configurable eviction policies
///
/// # Type Parameters
/// - `K`: Key type (must be `Eq + Hash + Clone`)
/// - `V`: Value type (must be `Clone`)
/// - `C`: Clock type for time-based operations (defaults to `SystemClock`)
///
/// # Example
/// ```
/// use std::time::Duration;
///
/// use pulsearc_common::cache::{Cache, CacheConfig};
///
/// // Create a simple LRU cache
/// let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));
/// cache.insert("key".to_string(), 42);
/// assert_eq!(cache.get(&"key".to_string()), Some(42));
/// ```
#[allow(clippy::type_complexity)]
pub struct Cache<K, V, C = SystemClock>
where
    K: Eq + Hash + Clone,
    V: Clone,
    C: Clock,
{
    storage: Arc<RwLock<CacheStorage<K, V>>>,
    config: CacheConfig,
    metrics: MetricsCollector,
    clock: C,
}

impl<K, V> Cache<K, V, SystemClock>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Create a new cache with the given configuration using system clock
    pub fn new(config: CacheConfig) -> Self {
        Self::with_clock(config, SystemClock)
    }
}

impl<K, V, C> Cache<K, V, C>
where
    K: Eq + Hash + Clone,
    V: Clone,
    C: Clock + Clone,
{
    /// Create a new cache with a custom clock (useful for testing)
    pub fn with_clock(config: CacheConfig, clock: C) -> Self {
        Self {
            storage: Arc::new(RwLock::new(CacheStorage::new())),
            config,
            metrics: MetricsCollector::new(),
            clock,
        }
    }

    /// Insert a value into the cache
    ///
    /// If the cache is at capacity, an entry will be evicted according to the
    /// configured eviction policy before inserting the new entry.
    pub fn insert(&self, key: K, value: V) {
        let mut storage = self.storage.write().unwrap();

        // Check if eviction is needed
        if let Some(max_size) = self.config.max_size {
            if storage.entries.len() >= max_size && !storage.entries.contains_key(&key) {
                self.evict_one(&mut storage);
            }
        }

        let now = self.clock.now();
        let entry = CacheEntry { value, inserted_at: now, last_accessed: now, access_count: 0 };

        storage.entries.insert(key.clone(), entry);

        // Update access order for LRU/FIFO policies
        if matches!(self.config.eviction_policy, EvictionPolicy::LRU | EvictionPolicy::FIFO) {
            storage.access_order.retain(|k| k != &key);
            storage.access_order.push(key);
        }

        if self.config.track_metrics {
            self.metrics.record_insert();
        }
    }

    /// Get a value from the cache
    ///
    /// Returns `None` if the key doesn't exist or if the entry has expired.
    /// Updates access metadata for eviction policies.
    pub fn get(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().unwrap();

        // Check if key exists and isn't expired
        let entry_exists = storage.entries.contains_key(key);
        if !entry_exists {
            if self.config.track_metrics {
                self.metrics.record_miss();
            }
            return None;
        }

        // Check TTL expiration
        if let Some(ttl) = self.config.ttl {
            if let Some(entry) = storage.entries.get(key) {
                let now = self.clock.now();
                let elapsed = now.duration_since(entry.inserted_at);
                if elapsed >= ttl {
                    // Entry expired, remove it
                    storage.entries.remove(key);
                    storage.access_order.retain(|k| k != key);

                    if self.config.track_metrics {
                        self.metrics.record_miss();
                        self.metrics.record_expiration();
                    }
                    return None;
                }
            }
        }

        // Update access metadata and get value
        if let Some(entry) = storage.entries.get_mut(key) {
            entry.last_accessed = self.clock.now();
            entry.access_count += 1;
            let value = entry.value.clone();

            // Update LRU order (after releasing mutable borrow of entry)
            let _ = entry;
            if self.config.eviction_policy == EvictionPolicy::LRU {
                storage.access_order.retain(|k| k != key);
                storage.access_order.push(key.clone());
            }

            if self.config.track_metrics {
                self.metrics.record_hit();
            }

            Some(value)
        } else {
            if self.config.track_metrics {
                self.metrics.record_miss();
            }
            None
        }
    }

    /// Get or insert with a generator function
    ///
    /// If the key exists and hasn't expired, returns the cached value.
    /// Otherwise, generates a new value using the provided function.
    ///
    /// # Example
    /// ```
    /// use pulsearc_common::cache::{Cache, CacheConfig};
    ///
    /// let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));
    /// let value = cache.get_or_insert_with("key".to_string(), || 42);
    /// assert_eq!(value, 42);
    /// ```
    pub fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        // Try to get existing value
        if let Some(value) = self.get(&key) {
            return value;
        }

        // Generate new value
        let value = f();
        self.insert(key, value.clone());
        value
    }

    /// Remove a value from the cache
    pub fn remove(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().unwrap();
        storage.access_order.retain(|k| k != key);
        storage.entries.remove(key).map(|e| e.value)
    }

    /// Clear all entries from the cache
    pub fn clear(&self) {
        let mut storage = self.storage.write().unwrap();
        storage.entries.clear();
        storage.access_order.clear();

        if self.config.track_metrics {
            self.metrics.reset();
        }
    }

    /// Get the current number of entries
    pub fn len(&self) -> usize {
        self.storage.read().unwrap().entries.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove expired entries
    ///
    /// Returns the number of entries removed.
    pub fn cleanup_expired(&self) -> usize {
        if self.config.ttl.is_none() {
            return 0;
        }

        let ttl = self.config.ttl.unwrap();
        let now = self.clock.now();
        let mut storage = self.storage.write().unwrap();

        // Collect keys to remove (avoid borrow conflict)
        let keys_to_remove: Vec<K> = storage
            .entries
            .iter()
            .filter(|(_, entry)| {
                let elapsed = now.duration_since(entry.inserted_at);
                elapsed >= ttl
            })
            .map(|(k, _)| k.clone())
            .collect();

        // Remove expired entries
        for key in &keys_to_remove {
            storage.entries.remove(key);
            storage.access_order.retain(|k| k != key);

            if self.config.track_metrics {
                self.metrics.record_expiration();
            }
        }

        keys_to_remove.len()
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let size = self.len();
        self.metrics.snapshot(size, self.config.max_size)
    }

    /// Evict one entry based on the configured policy
    fn evict_one(&self, storage: &mut CacheStorage<K, V>) {
        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Least recently used (first in access order)
                storage.access_order.first().cloned()
            }

            EvictionPolicy::LFU => {
                // Least frequently used (lowest access count)
                storage
                    .entries
                    .iter()
                    .min_by_key(|(_, entry)| entry.access_count)
                    .map(|(k, _)| k.clone())
            }

            EvictionPolicy::FIFO => {
                // First in first out (oldest insertion, first in access order)
                storage.access_order.first().cloned()
            }

            EvictionPolicy::Random => {
                // Random eviction
                use rand::seq::IteratorRandom;
                let mut rng = rand::thread_rng();
                storage.entries.keys().choose(&mut rng).cloned()
            }

            EvictionPolicy::None => None,
        };

        if let Some(key) = key_to_evict {
            storage.entries.remove(&key);
            storage.access_order.retain(|k| k != &key);

            if self.config.track_metrics {
                self.metrics.record_eviction();
            }
        }
    }
}

impl<K, V, C> Clone for Cache<K, V, C>
where
    K: Eq + Hash + Clone,
    V: Clone,
    C: Clock + Clone,
{
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            clock: self.clock.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for cache::core.
    use std::thread;
    use std::time::Duration;

    use super::*;
    use crate::resilience::MockClock;

    /// Validates `Cache::new` behavior for the cache new scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `0`.
    /// - Ensures `cache.is_empty()` evaluates to true.
    #[test]
    fn test_cache_new() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::default());
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    /// Validates `Cache::new` behavior for the cache insert and get scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key1".to_string())` equals `Some(42)`.
    /// - Confirms `cache.get(&"key2".to_string())` equals `Some(84)`.
    /// - Confirms `cache.get(&"key3".to_string())` equals `None`.
    /// - Confirms `cache.len()` equals `2`.
    #[test]
    fn test_cache_insert_and_get() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

        cache.insert("key1".to_string(), 42);
        cache.insert("key2".to_string(), 84);

        assert_eq!(cache.get(&"key1".to_string()), Some(42));
        assert_eq!(cache.get(&"key2".to_string()), Some(84));
        assert_eq!(cache.get(&"key3".to_string()), None);
        assert_eq!(cache.len(), 2);
    }

    /// Validates `Cache::new` behavior for the cache update existing scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key".to_string())` equals `Some(42)`.
    /// - Confirms `cache.get(&"key".to_string())` equals `Some(84)`.
    /// - Confirms `cache.len()` equals `1`.
    #[test]
    fn test_cache_update_existing() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

        cache.insert("key".to_string(), 42);
        assert_eq!(cache.get(&"key".to_string()), Some(42));

        cache.insert("key".to_string(), 84);
        assert_eq!(cache.get(&"key".to_string()), Some(84));
        assert_eq!(cache.len(), 1);
    }

    /// Validates `Cache::new` behavior for the cache remove scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `1`.
    /// - Confirms `removed` equals `Some(42)`.
    /// - Confirms `cache.len()` equals `0`.
    /// - Confirms `cache.get(&"key".to_string())` equals `None`.
    #[test]
    fn test_cache_remove() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

        cache.insert("key".to_string(), 42);
        assert_eq!(cache.len(), 1);

        let removed = cache.remove(&"key".to_string());
        assert_eq!(removed, Some(42));
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.get(&"key".to_string()), None);
    }

    /// Validates `Cache::new` behavior for the cache clear scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `2`.
    /// - Confirms `cache.len()` equals `0`.
    /// - Ensures `cache.is_empty()` evaluates to true.
    #[test]
    fn test_cache_clear() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

        cache.insert("key1".to_string(), 42);
        cache.insert("key2".to_string(), 84);
        assert_eq!(cache.len(), 2);

        cache.clear();
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    /// Validates `Cache::new` behavior for the cache lru eviction scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a".to_string())` equals `None`.
    /// - Confirms `cache.get(&"b".to_string())` equals `Some(2)`.
    /// - Confirms `cache.get(&"c".to_string())` equals `Some(3)`.
    /// - Confirms `cache.len()` equals `2`.
    #[test]
    fn test_cache_lru_eviction() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(2));

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3); // Should evict "a"

        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
        assert_eq!(cache.len(), 2);
    }

    /// Validates `Cache::new` behavior for the cache lru access updates order
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a".to_string())` equals `Some(1)`.
    /// - Confirms `cache.get(&"b".to_string())` equals `None`.
    /// - Confirms `cache.get(&"c".to_string())` equals `Some(3)`.
    #[test]
    fn test_cache_lru_access_updates_order() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(2));

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);

        // Access "a" to make it recently used
        let _ = cache.get(&"a".to_string());

        cache.insert("c".to_string(), 3); // Should evict "b", not "a"

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), None);
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    /// Validates `MockClock::new` behavior for the cache ttl expiration
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key".to_string())` equals `Some(42)`.
    /// - Confirms `cache.get(&"key".to_string())` equals `None`.
    /// - Confirms `cache.len()` equals `0`.
    #[test]
    fn test_cache_ttl_expiration() {
        let clock = MockClock::new();
        let config = CacheConfig::ttl(Duration::from_secs(10));
        let cache: Cache<String, i32, MockClock> = Cache::with_clock(config, clock.clone());

        cache.insert("key".to_string(), 42);
        assert_eq!(cache.get(&"key".to_string()), Some(42));

        // Advance time past TTL
        clock.advance(Duration::from_secs(11));

        assert_eq!(cache.get(&"key".to_string()), None);
        assert_eq!(cache.len(), 0);
    }

    /// Validates `MockClock::new` behavior for the cache ttl not expired
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key".to_string())` equals `Some(42)`.
    /// - Confirms `cache.len()` equals `1`.
    #[test]
    fn test_cache_ttl_not_expired() {
        let clock = MockClock::new();
        let config = CacheConfig::ttl(Duration::from_secs(10));
        let cache: Cache<String, i32, MockClock> = Cache::with_clock(config, clock.clone());

        cache.insert("key".to_string(), 42);

        // Advance time but not past TTL
        clock.advance(Duration::from_secs(5));

        assert_eq!(cache.get(&"key".to_string()), Some(42));
        assert_eq!(cache.len(), 1);
    }

    /// Validates `MockClock::new` behavior for the cache cleanup expired
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `removed` equals `3`.
    /// - Confirms `cache.len()` equals `0`.
    #[test]
    fn test_cache_cleanup_expired() {
        let clock = MockClock::new();
        let config = CacheConfig::ttl(Duration::from_secs(10));
        let cache: Cache<String, i32, MockClock> = Cache::with_clock(config, clock.clone());

        cache.insert("key1".to_string(), 1);
        cache.insert("key2".to_string(), 2);
        cache.insert("key3".to_string(), 3);

        clock.advance(Duration::from_secs(11));

        let removed = cache.cleanup_expired();
        assert_eq!(removed, 3);
        assert_eq!(cache.len(), 0);
    }

    /// Validates `Cache::new` behavior for the cache get or insert with
    /// existing scenario.
    ///
    /// Assertions:
    /// - Confirms `value` equals `42`.
    #[test]
    fn test_cache_get_or_insert_with_existing() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

        cache.insert("key".to_string(), 42);

        let value = cache.get_or_insert_with("key".to_string(), || 99);
        assert_eq!(value, 42); // Should return existing value
    }

    /// Validates `Cache::new` behavior for the cache get or insert with new
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `value` equals `42`.
    /// - Confirms `cache.get(&"key".to_string())` equals `Some(42)`.
    #[test]
    fn test_cache_get_or_insert_with_new() {
        let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

        let value = cache.get_or_insert_with("key".to_string(), || 42);
        assert_eq!(value, 42);
        assert_eq!(cache.get(&"key".to_string()), Some(42));
    }

    /// Validates `CacheConfig::builder` behavior for the cache stats tracking
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.size` equals `2`.
    /// - Confirms `stats.hits` equals `2`.
    /// - Confirms `stats.misses` equals `1`.
    /// - Confirms `stats.inserts` equals `2`.
    /// - Confirms `stats.hit_rate()` equals `2.0 / 3.0`.
    #[test]
    fn test_cache_stats_tracking() {
        let config = CacheConfig::builder().max_size(10).track_metrics(true).build();
        let cache: Cache<String, i32> = Cache::new(config);

        cache.insert("key1".to_string(), 1);
        cache.insert("key2".to_string(), 2);

        let _ = cache.get(&"key1".to_string()); // Hit
        let _ = cache.get(&"key1".to_string()); // Hit
        let _ = cache.get(&"key3".to_string()); // Miss

        let stats = cache.stats();
        assert_eq!(stats.size, 2);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.inserts, 2);
        assert_eq!(stats.hit_rate(), 2.0 / 3.0);
    }

    /// Validates `CacheConfig::builder` behavior for the cache lfu eviction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a".to_string())` equals `Some(1)`.
    /// - Confirms `cache.get(&"b".to_string())` equals `None`.
    /// - Confirms `cache.get(&"c".to_string())` equals `Some(3)`.
    #[test]
    fn test_cache_lfu_eviction() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::LFU).build();
        let cache: Cache<String, i32> = Cache::new(config);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);

        // Access "a" multiple times
        let _ = cache.get(&"a".to_string());
        let _ = cache.get(&"a".to_string());
        let _ = cache.get(&"b".to_string());

        cache.insert("c".to_string(), 3); // Should evict "b" (least frequently used)

        assert_eq!(cache.get(&"a".to_string()), Some(1));
        assert_eq!(cache.get(&"b".to_string()), None);
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    /// Validates `CacheConfig::builder` behavior for the cache fifo eviction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a".to_string())` equals `None`.
    /// - Confirms `cache.get(&"b".to_string())` equals `Some(2)`.
    /// - Confirms `cache.get(&"c".to_string())` equals `Some(3)`.
    #[test]
    fn test_cache_fifo_eviction() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::FIFO).build();
        let cache: Cache<String, i32> = Cache::new(config);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);

        // Access "a" (shouldn't affect FIFO order)
        let _ = cache.get(&"a".to_string());

        cache.insert("c".to_string(), 3); // Should evict "a" (first inserted)

        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));
    }

    /// Validates `CacheConfig::builder` behavior for the cache random eviction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `2`.
    /// - Ensures `has_a || has_b` evaluates to true.
    /// - Ensures `has_c` evaluates to true.
    #[test]
    fn test_cache_random_eviction() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::Random).build();
        let cache: Cache<String, i32> = Cache::new(config);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3); // Should evict one randomly

        // Should have exactly 2 entries
        assert_eq!(cache.len(), 2);

        // At least one should be present
        let has_a = cache.get(&"a".to_string()).is_some();
        let has_b = cache.get(&"b".to_string()).is_some();
        let has_c = cache.get(&"c".to_string()).is_some();

        assert!(has_a || has_b);
        assert!(has_c); // "c" was just inserted, should be present
    }

    /// Validates `CacheConfig::builder` behavior for the cache no eviction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `3`.
    #[test]
    fn test_cache_no_eviction() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::None).build();
        let cache: Cache<String, i32> = Cache::new(config);

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3); // Should NOT evict

        // All entries should be present (EvictionPolicy::None)
        assert_eq!(cache.len(), 3);
    }

    /// Validates `Arc::new` behavior for the cache thread safety scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `100`.
    #[test]
    fn test_cache_thread_safety() {
        let cache = Arc::new(Cache::new(CacheConfig::lru(100)));
        let mut handles = vec![];

        // Spawn 10 threads, each inserting 10 entries
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = thread::spawn(move || {
                for j in 0..10 {
                    let key = format!("key-{}-{}", i, j);
                    cache_clone.insert(key, i * 10 + j);
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(cache.len(), 100);
    }

    /// Validates `Cache::new` behavior for the cache clone shares storage
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache2.get(&"key".to_string())` equals `Some(42)`.
    /// - Confirms `cache1.get(&"key2".to_string())` equals `Some(84)`.
    #[test]
    fn test_cache_clone_shares_storage() {
        let cache1: Cache<String, i32> = Cache::new(CacheConfig::lru(10));
        cache1.insert("key".to_string(), 42);

        let cache2 = cache1.clone();
        assert_eq!(cache2.get(&"key".to_string()), Some(42));

        cache2.insert("key2".to_string(), 84);
        assert_eq!(cache1.get(&"key2".to_string()), Some(84));
    }

    /// Validates `MockClock::new` behavior for the cache ttl and lru combined
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a".to_string())` equals `None`.
    /// - Confirms `cache.get(&"b".to_string())` equals `Some(2)`.
    /// - Confirms `cache.get(&"c".to_string())` equals `Some(3)`.
    /// - Confirms `cache.get(&"b".to_string())` equals `None`.
    /// - Confirms `cache.get(&"c".to_string())` equals `None`.
    #[test]
    fn test_cache_ttl_and_lru_combined() {
        let clock = MockClock::new();
        let config = CacheConfig::ttl_lru(Duration::from_secs(10), 2);
        let cache: Cache<String, i32, MockClock> = Cache::with_clock(config, clock.clone());

        cache.insert("a".to_string(), 1);
        cache.insert("b".to_string(), 2);
        cache.insert("c".to_string(), 3); // Should evict "a" via LRU

        assert_eq!(cache.get(&"a".to_string()), None);
        assert_eq!(cache.get(&"b".to_string()), Some(2));
        assert_eq!(cache.get(&"c".to_string()), Some(3));

        // Advance time past TTL
        clock.advance(Duration::from_secs(11));

        assert_eq!(cache.get(&"b".to_string()), None); // Expired
        assert_eq!(cache.get(&"c".to_string()), None); // Expired
    }
}
