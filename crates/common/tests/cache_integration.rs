//! Integration tests for cache module
//!
//! Tests different eviction policies, TTL support, and concurrent access
//! patterns

#![cfg(feature = "runtime")]

use std::sync::Arc;
use std::thread;
use std::time::Duration;

use pulsearc_common::cache::{AsyncCache, Cache, CacheConfig, EvictionPolicy};

/// Verifies basic cache operations (insert, get) with LRU eviction policy.
///
/// This test ensures that when a cache reaches its maximum capacity, the least
/// recently used item is evicted when a new item is inserted. It validates that
/// accessing an item updates its "recently used" status and prevents eviction.
///
/// # Test Steps
/// 1. Insert 3 items into a cache with max size of 3
/// 2. Access key1 to mark it as recently used
/// 3. Insert a 4th item, triggering eviction of key2 (least recently used)
/// 4. Verify key1 and key3 remain, key2 is evicted, key4 is present
#[test]
fn test_lru_cache_basic_operations() {
    let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(3));

    // Insert items
    cache.insert("key1".to_string(), 100);
    cache.insert("key2".to_string(), 200);
    cache.insert("key3".to_string(), 300);

    // Verify all items exist
    assert_eq!(cache.get(&"key1".to_string()), Some(100));
    assert_eq!(cache.get(&"key2".to_string()), Some(200));
    assert_eq!(cache.get(&"key3".to_string()), Some(300));

    // Access key1 to make it recently used
    let _ = cache.get(&"key1".to_string());

    // Insert new item - should evict key2 (least recently used)
    cache.insert("key4".to_string(), 400);

    assert_eq!(cache.get(&"key1".to_string()), Some(100)); // Still exists
    assert_eq!(cache.get(&"key2".to_string()), None); // Evicted
    assert_eq!(cache.get(&"key3".to_string()), Some(300)); // Still exists
    assert_eq!(cache.get(&"key4".to_string()), Some(400)); // New item
}

/// Validates LFU (Least Frequently Used) eviction policy behavior.
///
/// This test verifies that the cache tracks access frequency and evicts items
/// with the lowest access count when capacity is reached. Items accessed more
/// frequently are retained over less frequently accessed items.
///
/// # Test Steps
/// 1. Insert 3 items into a cache with max size of 3 and LFU policy
/// 2. Access key1 five times (make it frequently used)
/// 3. Access key2 once (make it less frequently used)
/// 4. Insert key4, triggering eviction of key3 (never accessed, least frequent)
/// 5. Verify key1 and key2 remain, key3 is evicted, key4 is present
#[test]
fn test_lfu_cache_eviction() {
    let config = CacheConfig::builder().max_size(3).eviction_policy(EvictionPolicy::LFU).build();

    let cache: Cache<String, i32> = Cache::new(config);

    // Insert items
    cache.insert("key1".to_string(), 100);
    cache.insert("key2".to_string(), 200);
    cache.insert("key3".to_string(), 300);

    // Access key1 multiple times (make it frequently used)
    for _ in 0..5 {
        let _ = cache.get(&"key1".to_string());
    }

    // Access key2 once
    let _ = cache.get(&"key2".to_string());

    // Insert new item - should evict key3 (least frequently used)
    cache.insert("key4".to_string(), 400);

    assert_eq!(cache.get(&"key1".to_string()), Some(100)); // Frequently used
    assert_eq!(cache.get(&"key2".to_string()), Some(200)); // Used once
    assert_eq!(cache.get(&"key3".to_string()), None); // Evicted
    assert_eq!(cache.get(&"key4".to_string()), Some(400)); // New item
}

/// Validates FIFO (First In First Out) eviction policy behavior.
///
/// This test ensures that the cache evicts items in the order they were
/// inserted, regardless of access patterns. The oldest inserted item is always
/// evicted first when the cache reaches capacity.
///
/// # Test Steps
/// 1. Insert 3 items in order: "first", "second", "third"
/// 2. Insert "fourth", triggering eviction of "first" (oldest)
/// 3. Verify "first" is evicted, others remain in order
#[test]
fn test_fifo_cache_eviction() {
    let config = CacheConfig::builder().max_size(3).eviction_policy(EvictionPolicy::FIFO).build();

    let cache: Cache<String, i32> = Cache::new(config);

    // Insert items in order
    cache.insert("first".to_string(), 1);
    cache.insert("second".to_string(), 2);
    cache.insert("third".to_string(), 3);

    // Insert new item - should evict "first" (oldest)
    cache.insert("fourth".to_string(), 4);

    assert_eq!(cache.get(&"first".to_string()), None); // Evicted
    assert_eq!(cache.get(&"second".to_string()), Some(2));
    assert_eq!(cache.get(&"third".to_string()), Some(3));
    assert_eq!(cache.get(&"fourth".to_string()), Some(4));
}

/// Validates time-to-live (TTL) based cache entry expiration.
///
/// This test verifies that cache entries are automatically expired after their
/// TTL duration elapses, ensuring stale data is not served. Uses sleep to wait
/// for expiration and validates the entry becomes inaccessible.
///
/// # Test Steps
/// 1. Create cache with 100ms TTL
/// 2. Insert an item and verify immediate availability
/// 3. Sleep for 150ms (past TTL)
/// 4. Verify item is expired and no longer retrievable
#[test]
fn test_ttl_expiration() {
    let cache: Cache<String, String> = Cache::new(CacheConfig::ttl(Duration::from_millis(100)));

    // Insert item
    cache.insert("expiring".to_string(), "value".to_string());

    // Item should exist immediately
    assert_eq!(cache.get(&"expiring".to_string()), Some("value".to_string()));

    // Wait for TTL to expire
    thread::sleep(Duration::from_millis(150));

    // Item should be expired
    assert_eq!(cache.get(&"expiring".to_string()), None);
}

/// Validates combined TTL and LRU eviction policies working together.
///
/// This test ensures that both TTL-based expiration and LRU-based eviction
/// operate correctly when combined. Items can be evicted by either reaching
/// capacity (LRU) or exceeding their TTL duration.
///
/// # Test Steps
/// 1. Create cache with 200ms TTL and max size of 2
/// 2. Insert 2 items, filling the cache
/// 3. Insert a 3rd item, triggering LRU eviction of key1
/// 4. Sleep for 250ms (past TTL)
/// 5. Verify remaining items are expired by TTL
#[test]
fn test_ttl_with_lru() {
    let cache: Cache<String, i32> = Cache::new(CacheConfig::ttl_lru(Duration::from_millis(200), 2));

    // Insert items
    cache.insert("key1".to_string(), 100);
    cache.insert("key2".to_string(), 200);

    // Cache is full, inserting new item should evict LRU
    cache.insert("key3".to_string(), 300);

    assert_eq!(cache.get(&"key1".to_string()), None); // Evicted by LRU
    assert_eq!(cache.get(&"key2".to_string()), Some(200));
    assert_eq!(cache.get(&"key3".to_string()), Some(300));

    // Wait for TTL to expire
    thread::sleep(Duration::from_millis(250));

    // All should be expired
    assert_eq!(cache.get(&"key2".to_string()), None);
    assert_eq!(cache.get(&"key3".to_string()), None);
}

/// Validates lazy value computation with `get_or_insert_with`.
///
/// This test ensures that the cache can lazily compute and insert values only
/// when they don't exist, avoiding redundant computation for cache hits. The
/// computation function should only execute once for the same key.
///
/// # Test Steps
/// 1. First call to `get_or_insert_with` computes value (increment counter)
/// 2. Second call with same key uses cached value (counter unchanged)
/// 3. Verify computation executed exactly once
#[test]
fn test_get_or_insert_with() {
    let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

    let mut computation_count = 0;

    // First call should compute
    let value1 = cache.get_or_insert_with("key".to_string(), || {
        computation_count += 1;
        42
    });
    assert_eq!(value1, 42);
    assert_eq!(computation_count, 1);

    // Second call should use cached value
    let value2 = cache.get_or_insert_with("key".to_string(), || {
        computation_count += 1;
        99 // This should not be executed
    });
    assert_eq!(value2, 42);
    assert_eq!(computation_count, 1); // Should not have incremented
}

/// Validates cache statistics tracking for hits, misses, and size.
///
/// This test ensures that the cache correctly tracks metrics when statistics
/// are enabled, including cache hits, misses, current size, and calculated
/// hit rate. These metrics are essential for cache performance monitoring.
///
/// # Test Steps
/// 1. Create cache with metrics tracking enabled
/// 2. Insert 2 items
/// 3. Perform a hit (existing key) and a miss (non-existent key)
/// 4. Verify statistics: size=2, hits=1, misses=1, hit_rate between 0 and 1
#[test]
fn test_cache_statistics() {
    let config = CacheConfig::builder().max_size(10).track_metrics(true).build();

    let cache: Cache<String, i32> = Cache::new(config);

    // Insert items
    cache.insert("key1".to_string(), 100);
    cache.insert("key2".to_string(), 200);

    // Hit
    let _ = cache.get(&"key1".to_string());

    // Miss
    let _ = cache.get(&"nonexistent".to_string());

    // Get statistics
    let stats = cache.stats();

    assert_eq!(stats.size, 2);
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
    assert!(stats.hit_rate() > 0.0);
    assert!(stats.hit_rate() < 1.0);
}

/// Validates thread-safe concurrent cache access from multiple threads.
///
/// This test ensures the cache is safe for concurrent use by multiple threads,
/// verifying that simultaneous insertions and reads don't cause data races,
/// corruption, or panics. Tests with 10 threads each inserting/reading 10
/// items.
///
/// # Test Steps
/// 1. Create shared cache wrapped in Arc
/// 2. Spawn 10 threads, each inserting 10 unique items
/// 3. Each thread reads back its own items and verifies values
/// 4. Wait for all threads to complete successfully
/// 5. Verify cache contains data (size > 0)
#[test]
fn test_concurrent_cache_access() {
    let cache = Arc::new(Cache::new(CacheConfig::lru(100)));
    let mut handles = vec![];

    // Spawn multiple threads that insert and read concurrently
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = thread::spawn(move || {
            // Insert items
            for j in 0..10 {
                cache_clone.insert(format!("key-{}-{}", i, j), i * 10 + j);
            }

            // Read items
            for j in 0..10 {
                let value = cache_clone.get(&format!("key-{}-{}", i, j));
                assert_eq!(value, Some(i * 10 + j));
            }
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // Verify cache contains all items (or subset if eviction occurred)
    let stats = cache.stats();
    assert!(stats.size > 0);
}

/// Validates cache clear operation removes all entries.
///
/// This test ensures the `clear()` method removes all cached entries and resets
/// the cache to an empty state, with size returning to zero and all keys
/// becoming inaccessible.
///
/// # Test Steps
/// 1. Insert 3 items into the cache
/// 2. Verify size is 3
/// 3. Call clear()
/// 4. Verify size is 0 and all keys return None
#[test]
fn test_cache_clear() {
    let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

    // Insert items
    cache.insert("key1".to_string(), 100);
    cache.insert("key2".to_string(), 200);
    cache.insert("key3".to_string(), 300);

    assert_eq!(cache.stats().size, 3);

    // Clear cache
    cache.clear();

    assert_eq!(cache.stats().size, 0);
    assert_eq!(cache.get(&"key1".to_string()), None);
    assert_eq!(cache.get(&"key2".to_string()), None);
}

/// Validates selective removal of individual cache entries.
///
/// This test ensures the `remove()` method can selectively delete specific
/// entries while preserving others, and correctly updates the cache size.
///
/// # Test Steps
/// 1. Insert 2 items
/// 2. Remove one specific item (key1)
/// 3. Verify removed item returns None
/// 4. Verify other item still exists
/// 5. Verify size decremented to 1
#[test]
fn test_cache_remove() {
    let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(10));

    cache.insert("key1".to_string(), 100);
    cache.insert("key2".to_string(), 200);

    // Remove item
    cache.remove(&"key1".to_string());

    assert_eq!(cache.get(&"key1".to_string()), None);
    assert_eq!(cache.get(&"key2".to_string()), Some(200));
    assert_eq!(cache.stats().size, 1);
}

/// Validates cache with no eviction policy retains all entries.
///
/// This test ensures that when the eviction policy is set to `None`, the cache
/// grows without bound and retains all inserted entries regardless of size.
/// This is useful for scenarios where memory is not a constraint.
///
/// # Test Steps
/// 1. Create cache with EvictionPolicy::None
/// 2. Insert 100 items without size limit
/// 3. Verify all 100 items remain in cache
#[test]
fn test_cache_no_eviction() {
    let config = CacheConfig::builder().eviction_policy(EvictionPolicy::None).build();

    let cache: Cache<String, i32> = Cache::new(config);

    // Insert many items without size limit
    for i in 0..100 {
        cache.insert(format!("key-{}", i), i);
    }

    // All items should still be present
    assert_eq!(cache.stats().size, 100);

    for i in 0..100 {
        assert_eq!(cache.get(&format!("key-{}", i)), Some(i));
    }
}

/// Validates basic async cache operations (insert, get, remove, clear).
///
/// This test ensures the async cache interface works correctly with Tokio,
/// providing non-blocking cache operations suitable for async contexts. Tests
/// all primary async methods including lazy insertion.
///
/// # Test Steps
/// 1. Create async cache with LRU policy
/// 2. Insert items using async methods
/// 3. Get items and verify values
/// 4. Test async `get_or_insert_with` with async closure
/// 5. Remove an item asynchronously
/// 6. Clear cache and verify empty
#[tokio::test(flavor = "multi_thread")]
async fn test_async_cache_operations() {
    let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(10));

    // Insert items
    cache.insert("key1".to_string(), 100).await;
    cache.insert("key2".to_string(), 200).await;

    // Get items
    assert_eq!(cache.get(&"key1".to_string()).await, Some(100));
    assert_eq!(cache.get(&"key2".to_string()).await, Some(200));

    // Test get_or_insert_with
    let value = cache.get_or_insert_with("key3".to_string(), || 300).await;
    assert_eq!(value, 300);

    // Remove item
    cache.remove(&"key1".to_string()).await;
    assert_eq!(cache.get(&"key1".to_string()).await, None);

    // Clear cache
    cache.clear().await;
    assert_eq!(cache.stats().size, 0);
}

/// Validates concurrent async cache access from multiple Tokio tasks.
///
/// This test ensures the async cache is safe for concurrent use by multiple
/// async tasks, verifying that simultaneous insertions and reads don't cause
/// data races or corruption in an async context. Tests with 10 concurrent
/// tasks.
///
/// # Test Steps
/// 1. Create shared async cache wrapped in Arc
/// 2. Spawn 10 Tokio tasks, each inserting 10 unique items
/// 3. Each task reads back its own items and verifies values
/// 4. Await all tasks to complete successfully
/// 5. Verify cache contains data (size > 0)
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_async_cache_access() {
    let cache = Arc::new(AsyncCache::new(CacheConfig::lru(100)));
    let mut handles = vec![];

    // Spawn multiple tasks that access cache concurrently
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let handle = tokio::spawn(async move {
            // Insert items
            for j in 0..10 {
                cache_clone.insert(format!("key-{}-{}", i, j), i * 10 + j).await;
            }

            // Read items
            for j in 0..10 {
                let value = cache_clone.get(&format!("key-{}-{}", i, j)).await;
                assert_eq!(value, Some(i * 10 + j));
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    for handle in handles {
        handle.await.expect("Task should complete");
    }

    let stats = cache.stats();
    assert!(stats.size > 0);
}

/// Validates cache builder pattern with custom configuration options.
///
/// This test ensures the builder pattern allows flexible cache configuration
/// with custom max_size, TTL, eviction policy, and metrics tracking. Verifies
/// that all specified options are correctly applied to the created cache.
///
/// # Test Steps
/// 1. Build cache with custom options: size=50, TTL=300s, LFU policy, metrics
///    enabled
/// 2. Insert a test item
/// 3. Verify item is accessible
/// 4. Verify configuration was applied (check max_size from stats)
#[test]
fn test_cache_builder_custom_config() {
    let config = CacheConfig::builder()
        .max_size(50)
        .ttl(Duration::from_secs(300))
        .eviction_policy(EvictionPolicy::LFU)
        .track_metrics(true)
        .build();

    let cache: Cache<String, String> = Cache::new(config);

    cache.insert("test".to_string(), "value".to_string());

    assert_eq!(cache.get(&"test".to_string()), Some("value".to_string()));
    assert_eq!(cache.stats().max_size, Some(50));
}
