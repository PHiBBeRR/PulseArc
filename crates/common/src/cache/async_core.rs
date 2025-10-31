//! Async cache implementation with configurable eviction policies and TTL
//! support.
//!
//! This module provides an async variant of the cache that uses
//! `tokio::sync::RwLock` for concurrent access in async contexts. It shares
//! configuration and metrics types with the synchronous cache implementation.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::config::{CacheConfig, EvictionPolicy};
use super::stats::{CacheStats, MetricsCollector};
use crate::resilience::Clock;

/// Internal storage entry with metadata for cache management.
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    value: V,
    inserted_at: std::time::Instant,
    last_accessed: std::time::Instant,
    access_count: u64,
    insertion_order: u64,
}

/// Internal storage structure for the async cache.
#[derive(Debug)]
struct CacheStorage<K, V>
where
    K: Eq + Hash,
{
    data: HashMap<K, CacheEntry<V>>,
    insertion_counter: u64,
}

impl<K, V> CacheStorage<K, V>
where
    K: Eq + Hash,
{
    fn new() -> Self {
        Self { data: HashMap::new(), insertion_counter: 0 }
    }
}

/// Async cache with configurable eviction policies and TTL support.
///
/// Uses `tokio::sync::RwLock` for async concurrent access. All access methods
/// are async and must be awaited.
///
/// # Type Parameters
///
/// * `K` - Key type (must implement `Eq + Hash + Clone`)
/// * `V` - Value type (must implement `Clone`)
/// * `C` - Clock type for time operations (defaults to `SystemClock`)
///
/// # Examples
///
/// ```
/// use pulsearc_common::cache::{AsyncCache, CacheConfig};
///
/// #[tokio::main]
/// async fn main() {
///     let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(100));
///
///     cache.insert("key".to_string(), 42).await;
///     assert_eq!(cache.get(&"key".to_string()).await, Some(42));
/// }
/// ```
#[allow(clippy::type_complexity)]
pub struct AsyncCache<K, V, C = crate::resilience::SystemClock>
where
    K: Eq + Hash + Clone,
    V: Clone,
    C: Clock + Clone,
{
    storage: Arc<RwLock<CacheStorage<K, V>>>,
    config: CacheConfig,
    metrics: MetricsCollector,
    clock: C,
}

impl<K, V> AsyncCache<K, V, crate::resilience::SystemClock>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    /// Creates a new async cache with the specified configuration and default
    /// system clock.
    pub fn new(config: CacheConfig) -> Self {
        Self::with_clock(config, crate::resilience::SystemClock)
    }
}

impl<K, V, C> AsyncCache<K, V, C>
where
    K: Eq + Hash + Clone,
    V: Clone,
    C: Clock + Clone,
{
    /// Creates a new async cache with the specified configuration and clock.
    pub fn with_clock(config: CacheConfig, clock: C) -> Self {
        Self {
            storage: Arc::new(RwLock::new(CacheStorage::new())),
            config,
            metrics: MetricsCollector::new(),
            clock,
        }
    }

    /// Inserts a key-value pair into the cache.
    ///
    /// If the cache is at capacity, evicts entries according to the eviction
    /// policy. If TTL is configured, the entry will expire after the
    /// specified duration.
    pub async fn insert(&self, key: K, value: V) {
        let now = self.clock.now();

        let mut storage = self.storage.write().await;

        // Check if we need to evict before inserting
        if let Some(max_size) = self.config.max_size {
            if storage.data.len() >= max_size && !storage.data.contains_key(&key) {
                self.evict_one(&mut storage).await;
            }
        }

        let entry = CacheEntry {
            value,
            inserted_at: now,
            last_accessed: now,
            access_count: 1,
            insertion_order: storage.insertion_counter,
        };

        storage.insertion_counter += 1;
        storage.data.insert(key, entry);
        self.metrics.record_insert();
    }

    /// Retrieves a value from the cache by key.
    ///
    /// Returns `None` if the key doesn't exist or the entry has expired.
    /// Updates access metadata for LRU/LFU eviction policies.
    pub async fn get(&self, key: &K) -> Option<V> {
        // First check if expired without holding write lock
        {
            let storage = self.storage.read().await;
            if let Some(entry) = storage.data.get(key) {
                if self.is_expired(entry) {
                    drop(storage);
                    let mut storage = self.storage.write().await;
                    storage.data.remove(key);
                    self.metrics.record_expiration();
                    self.metrics.record_miss();
                    return None;
                }
            } else {
                self.metrics.record_miss();
                return None;
            }
        }

        // Now update access metadata
        let now = self.clock.now();
        let mut storage = self.storage.write().await;
        if let Some(entry) = storage.data.get_mut(key) {
            entry.last_accessed = now;
            entry.access_count += 1;
            let value = entry.value.clone();
            self.metrics.record_hit();
            Some(value)
        } else {
            self.metrics.record_miss();
            None
        }
    }

    /// Gets a value or inserts it using the provided function if not present.
    ///
    /// This is useful for lazy initialization of cache values.
    pub async fn get_or_insert_with<F>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> V,
    {
        if let Some(value) = self.get(&key).await {
            return value;
        }

        let value = f();
        self.insert(key, value.clone()).await;
        value
    }

    /// Gets a value or inserts it using the provided async function if not
    /// present.
    ///
    /// This is useful for lazy initialization of cache values with async
    /// operations.
    pub async fn get_or_insert_with_async<F, Fut>(&self, key: K, f: F) -> V
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = V>,
    {
        if let Some(value) = self.get(&key).await {
            return value;
        }

        let value = f().await;
        self.insert(key, value.clone()).await;
        value
    }

    /// Removes and returns a value from the cache.
    pub async fn remove(&self, key: &K) -> Option<V> {
        let mut storage = self.storage.write().await;
        storage.data.remove(key).map(|entry| entry.value)
    }

    /// Checks if a key exists in the cache and is not expired.
    pub async fn contains_key(&self, key: &K) -> bool {
        let storage = self.storage.read().await;
        if let Some(entry) = storage.data.get(key) {
            !self.is_expired(entry)
        } else {
            false
        }
    }

    /// Returns the current number of entries in the cache.
    pub async fn len(&self) -> usize {
        let storage = self.storage.read().await;
        storage.data.len()
    }

    /// Returns `true` if the cache is empty.
    pub async fn is_empty(&self) -> bool {
        let storage = self.storage.read().await;
        storage.data.is_empty()
    }

    /// Clears all entries from the cache.
    pub async fn clear(&self) {
        let mut storage = self.storage.write().await;
        storage.data.clear();
    }

    /// Removes all expired entries and returns the count of removed entries.
    pub async fn cleanup_expired(&self) -> usize {
        let mut storage = self.storage.write().await;

        let keys_to_remove: Vec<K> = storage
            .data
            .iter()
            .filter(|(_, entry)| self.is_expired(entry))
            .map(|(k, _)| k.clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            storage.data.remove(&key);
            self.metrics.record_expiration();
        }

        count
    }

    /// Returns current cache statistics.
    ///
    /// Note: This method uses a non-blocking read. If the lock is currently
    /// held, the size will be reported as 0 in the snapshot.
    pub fn stats(&self) -> CacheStats {
        let size = self.storage.try_read().map(|s| s.data.len()).unwrap_or(0);
        self.metrics.snapshot(size, self.config.max_size)
    }

    /// Checks if an entry has expired based on TTL configuration.
    fn is_expired(&self, entry: &CacheEntry<V>) -> bool {
        if let Some(ttl) = self.config.ttl {
            let now = self.clock.now();
            let age = now.duration_since(entry.inserted_at);
            age > ttl
        } else {
            false
        }
    }

    /// Evicts a single entry based on the configured eviction policy.
    async fn evict_one(&self, storage: &mut CacheStorage<K, V>) {
        if storage.data.is_empty() {
            return;
        }

        let key_to_evict = match self.config.eviction_policy {
            EvictionPolicy::LRU => {
                // Find least recently used
                storage
                    .data
                    .iter()
                    .min_by_key(|(_, entry)| entry.last_accessed)
                    .map(|(k, _)| k.clone())
            }
            EvictionPolicy::LFU => {
                // Find least frequently used
                storage
                    .data
                    .iter()
                    .min_by_key(|(_, entry)| entry.access_count)
                    .map(|(k, _)| k.clone())
            }
            EvictionPolicy::FIFO => {
                // Find oldest insertion
                storage
                    .data
                    .iter()
                    .min_by_key(|(_, entry)| entry.insertion_order)
                    .map(|(k, _)| k.clone())
            }
            EvictionPolicy::Random => {
                // Pick random key
                use rand::seq::IteratorRandom;
                let mut rng = rand::thread_rng();
                storage.data.keys().choose(&mut rng).cloned()
            }
            EvictionPolicy::None => None,
        };

        if let Some(key) = key_to_evict {
            storage.data.remove(&key);
            self.metrics.record_eviction();
        }
    }
}

impl<K, V, C> Clone for AsyncCache<K, V, C>
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
    //! Unit tests for cache::async_core.
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::resilience::MockClock;

    /// Validates `AsyncCache::new` behavior for the basic insert and get
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key1".to_string()).await` equals `Some(42)`.
    /// - Confirms `cache.get(&"nonexistent".to_string()).await` equals `None`.
    #[tokio::test]
    async fn test_basic_insert_and_get() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::default());

        cache.insert("key1".to_string(), 42).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some(42));
        assert_eq!(cache.get(&"nonexistent".to_string()).await, None);
    }

    /// Validates `AsyncCache::new` behavior for the lru eviction scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key1".to_string()).await` equals `Some(1)`.
    /// - Confirms `cache.get(&"key2".to_string()).await` equals `None`.
    /// - Confirms `cache.get(&"key3".to_string()).await` equals `Some(3)`.
    #[tokio::test]
    async fn test_lru_eviction() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(2));

        cache.insert("key1".to_string(), 1).await;
        cache.insert("key2".to_string(), 2).await;

        // Access key1 to make it recently used
        cache.get(&"key1".to_string()).await;

        // Insert key3, should evict key2 (least recently used)
        cache.insert("key3".to_string(), 3).await;

        assert_eq!(cache.get(&"key1".to_string()).await, Some(1));
        assert_eq!(cache.get(&"key2".to_string()).await, None);
        assert_eq!(cache.get(&"key3".to_string()).await, Some(3));
    }

    /// Validates `CacheConfig::builder` behavior for the lfu eviction scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a".to_string()).await` equals `Some(1)`.
    /// - Confirms `cache.get(&"b".to_string()).await` equals `None`.
    /// - Confirms `cache.get(&"c".to_string()).await` equals `Some(3)`.
    #[tokio::test]
    async fn test_lfu_eviction() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::LFU).build();
        let cache: AsyncCache<String, i32> = AsyncCache::new(config);

        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;

        // Increase frequency for "a"
        let _ = cache.get(&"a".to_string()).await;
        let _ = cache.get(&"a".to_string()).await;
        let _ = cache.get(&"b".to_string()).await;

        cache.insert("c".to_string(), 3).await;

        assert_eq!(cache.get(&"a".to_string()).await, Some(1));
        assert_eq!(cache.get(&"b".to_string()).await, None);
        assert_eq!(cache.get(&"c".to_string()).await, Some(3));
    }

    /// Validates `CacheConfig::builder` behavior for the fifo eviction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"first".to_string()).await` equals `None`.
    /// - Confirms `cache.get(&"second".to_string()).await` equals `Some(2)`.
    /// - Confirms `cache.get(&"third".to_string()).await` equals `Some(3)`.
    #[tokio::test]
    async fn test_fifo_eviction() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::FIFO).build();
        let cache: AsyncCache<String, i32> = AsyncCache::new(config);

        cache.insert("first".to_string(), 1).await;
        cache.insert("second".to_string(), 2).await;
        cache.insert("third".to_string(), 3).await;

        assert_eq!(cache.get(&"first".to_string()).await, None);
        assert_eq!(cache.get(&"second".to_string()).await, Some(2));
        assert_eq!(cache.get(&"third".to_string()).await, Some(3));
    }

    /// Validates `CacheConfig::builder` behavior for the random eviction
    /// preserves new entry scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len().await` equals `2`.
    /// - Confirms `cache.get(&"c".to_string()).await` equals `Some(3)`.
    #[tokio::test]
    async fn test_random_eviction_preserves_new_entry() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::Random).build();
        let cache: AsyncCache<String, i32> = AsyncCache::new(config);

        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;
        cache.insert("c".to_string(), 3).await;

        assert_eq!(cache.len().await, 2);
        assert_eq!(cache.get(&"c".to_string()).await, Some(3));
    }

    /// Validates `CacheConfig::builder` behavior for the no eviction allows
    /// growth scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len().await` equals `3`.
    #[tokio::test]
    async fn test_no_eviction_allows_growth() {
        let config =
            CacheConfig::builder().max_size(2).eviction_policy(EvictionPolicy::None).build();
        let cache: AsyncCache<String, i32> = AsyncCache::new(config);

        cache.insert("a".to_string(), 1).await;
        cache.insert("b".to_string(), 2).await;
        cache.insert("c".to_string(), 3).await;

        assert_eq!(cache.len().await, 3);
    }

    /// Validates `MockClock::new` behavior for the ttl expiration scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key1".to_string()).await` equals `Some(42)`.
    /// - Confirms `cache.get(&"key1".to_string()).await` equals `None`.
    #[tokio::test]
    async fn test_ttl_expiration() {
        let mock_clock = MockClock::new();
        let cache =
            AsyncCache::with_clock(CacheConfig::ttl(Duration::from_secs(60)), mock_clock.clone());

        cache.insert("key1".to_string(), 42).await;
        assert_eq!(cache.get(&"key1".to_string()).await, Some(42));

        // Advance time beyond TTL
        mock_clock.advance(Duration::from_secs(61));

        // Entry should be expired
        assert_eq!(cache.get(&"key1".to_string()).await, None);
    }

    /// Validates `MockClock::new` behavior for the cleanup expired scenario.
    ///
    /// Assertions:
    /// - Confirms `removed` equals `2`.
    /// - Confirms `cache.len().await` equals `0`.
    /// - Confirms `stats.expirations` equals `2`.
    #[tokio::test]
    async fn test_cleanup_expired() {
        let mock_clock = MockClock::new();
        let cache =
            AsyncCache::with_clock(CacheConfig::ttl(Duration::from_secs(60)), mock_clock.clone());

        cache.insert("key1".to_string(), 1).await;
        cache.insert("key2".to_string(), 2).await;

        mock_clock.advance(Duration::from_secs(61));

        let removed = cache.cleanup_expired().await;
        assert_eq!(removed, 2);
        assert_eq!(cache.len().await, 0);
        let stats = cache.stats();
        assert_eq!(stats.expirations, 2);
    }

    /// Validates `MockClock::new` behavior for the contains key respects ttl
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.contains_key(&"key".to_string()).await` evaluates to
    ///   true.
    /// - Ensures `!cache.contains_key(&"key".to_string()).await` evaluates to
    ///   true.
    #[tokio::test]
    async fn test_contains_key_respects_ttl() {
        let mock_clock = MockClock::new();
        let cache =
            AsyncCache::with_clock(CacheConfig::ttl(Duration::from_secs(5)), mock_clock.clone());

        cache.insert("key".to_string(), 1).await;
        assert!(cache.contains_key(&"key".to_string()).await);

        mock_clock.advance(Duration::from_secs(6));
        assert!(!cache.contains_key(&"key".to_string()).await);
    }

    /// Validates `AsyncCache::new` behavior for the get or insert with
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `value` equals `42`.
    /// - Confirms `value` equals `42`.
    #[tokio::test]
    async fn test_get_or_insert_with() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::default());

        let value = cache.get_or_insert_with("key1".to_string(), || 42).await;
        assert_eq!(value, 42);

        let value = cache.get_or_insert_with("key1".to_string(), || 999).await;
        assert_eq!(value, 42); // Should return cached value
    }

    /// Validates `AsyncCache::new` behavior for the get or insert with async
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `value` equals `42`.
    /// - Confirms `value` equals `42`.
    #[tokio::test]
    async fn test_get_or_insert_with_async() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::default());

        let value = cache.get_or_insert_with_async("key1".to_string(), async || 42).await;
        assert_eq!(value, 42);

        let value = cache.get_or_insert_with_async("key1".to_string(), async || 999).await;
        assert_eq!(value, 42); // Should return cached value
    }

    /// Validates `AsyncCache::new` behavior for the len and is empty scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.is_empty().await` evaluates to true.
    /// - Confirms `cache.len().await` equals `0`.
    /// - Ensures `!cache.is_empty().await` evaluates to true.
    /// - Confirms `cache.len().await` equals `1`.
    /// - Ensures `cache.is_empty().await` evaluates to true.
    /// - Confirms `cache.len().await` equals `0`.
    #[tokio::test]
    async fn test_len_and_is_empty() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::default());

        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);

        cache.insert("key".to_string(), 1).await;

        assert!(!cache.is_empty().await);
        assert_eq!(cache.len().await, 1);

        cache.clear().await;

        assert!(cache.is_empty().await);
        assert_eq!(cache.len().await, 0);
    }

    /// Validates `AsyncCache::new` behavior for the remove returns value
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `removed` equals `Some(1)`.
    /// - Confirms `cache.get(&"key".to_string()).await` equals `None`.
    /// - Confirms `cache.len().await` equals `0`.
    #[tokio::test]
    async fn test_remove_returns_value() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::default());

        cache.insert("key".to_string(), 1).await;

        let removed = cache.remove(&"key".to_string()).await;
        assert_eq!(removed, Some(1));
        assert_eq!(cache.get(&"key".to_string()).await, None);
        assert_eq!(cache.len().await, 0);
    }

    /// Validates `Arc::new` behavior for the concurrent access scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len().await` equals `100`.
    #[tokio::test]
    async fn test_concurrent_access() {
        let cache = Arc::new(AsyncCache::new(CacheConfig::lru(100)));
        let mut handles = vec![];

        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = tokio::spawn(async move {
                for j in 0..10 {
                    let key = format!("key_{}", i * 10 + j);
                    cache_clone.insert(key.clone(), i * 10 + j).await;
                    cache_clone.get(&key).await;
                }
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.unwrap();
        }

        assert_eq!(cache.len().await, 100);
    }

    /// Validates `AsyncCache::new` behavior for the stats scenario.
    ///
    /// Assertions:
    /// - Confirms `stats.hits` equals `1`.
    /// - Confirms `stats.misses` equals `1`.
    /// - Confirms `stats.inserts` equals `1`.
    #[tokio::test]
    async fn test_stats() {
        let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(10));

        cache.insert("key1".to_string(), 1).await;
        cache.get(&"key1".to_string()).await; // hit
        cache.get(&"nonexistent".to_string()).await; // miss

        let stats = cache.stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.inserts, 1);
    }
}
