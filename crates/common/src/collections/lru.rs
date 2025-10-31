//! LRU cache wrapper
//!
//! Wraps the `lru` crate with additional utilities.

use std::borrow::Borrow;
use std::hash::Hash;
use std::num::NonZeroUsize;

use lru::LruCache as ExternalLruCache;

/// LRU (Least Recently Used) cache
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use pulsearc_common::collections::LruCache;
///
/// let mut cache = LruCache::new(NonZeroUsize::new(2).expect("capacity must be > 0"));
/// cache.put("key1", "value1");
/// cache.put("key2", "value2");
///
/// assert_eq!(cache.get(&"key1"), Some(&"value1"));
///
/// cache.put("key3", "value3"); // Evicts key2
/// assert_eq!(cache.get(&"key2"), None);
/// ```
#[derive(Clone, Debug)]
pub struct LruCache<K, V>
where
    K: Hash + Eq,
{
    inner: ExternalLruCache<K, V>,
}

impl<K: Hash + Eq, V> LruCache<K, V> {
    /// Create a new LRU cache with the specified non-zero capacity
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self { inner: ExternalLruCache::new(capacity) }
    }

    /// Try to create a new LRU cache with the specified capacity
    ///
    /// Returns None if capacity is zero
    pub fn try_new(capacity: usize) -> Option<Self> {
        let capacity = NonZeroUsize::new(capacity)?;
        Some(Self { inner: ExternalLruCache::new(capacity) })
    }

    /// Insert a key-value pair into the cache
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        self.inner.put(key, value)
    }

    /// Get a reference to a value in the cache
    pub fn get<Q>(&mut self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get(key)
    }

    /// Get a mutable reference to a value in the cache
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.get_mut(key)
    }

    /// Peek at a value without updating access time
    pub fn peek<Q>(&self, key: &Q) -> Option<&V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.peek(key)
    }

    /// Check if a key exists in the cache
    pub fn contains<Q>(&mut self, key: &Q) -> bool
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.contains(key)
    }

    /// Remove a key from the cache
    pub fn pop<Q>(&mut self, key: &Q) -> Option<V>
    where
        K: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.inner.pop(key)
    }

    /// Get the current number of items in the cache
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Get the capacity of the cache
    pub fn cap(&self) -> usize {
        self.inner.cap().get()
    }

    /// Clear all items from the cache
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Resize the cache with the specified non-zero capacity
    pub fn resize(&mut self, capacity: NonZeroUsize) {
        self.inner.resize(capacity);
    }

    /// Try to resize the cache
    ///
    /// Returns false if capacity is zero
    pub fn try_resize(&mut self, capacity: usize) -> bool {
        if let Some(capacity) = NonZeroUsize::new(capacity) {
            self.inner.resize(capacity);
            true
        } else {
            false
        }
    }

    /// Get an iterator over the cache items (most recent first)
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.inner.iter()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::lru.
    use std::num::NonZeroUsize;

    use super::*;

    fn cache_with_capacity<K: Hash + Eq, V>(capacity: usize) -> LruCache<K, V> {
        LruCache::new(NonZeroUsize::new(capacity).expect("capacity must be non-zero"))
    }

    /// Validates the put get scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key1")` equals `Some(&"value1")`.
    /// - Confirms `cache.get(&"key2")` equals `Some(&"value2")`.
    #[test]
    fn test_put_get() {
        let mut cache = cache_with_capacity::<&str, &str>(2);

        cache.put("key1", "value1");
        cache.put("key2", "value2");

        assert_eq!(cache.get(&"key1"), Some(&"value1"));
        assert_eq!(cache.get(&"key2"), Some(&"value2"));
    }

    /// Validates the eviction scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"key1")` equals `None`.
    /// - Confirms `cache.get(&"key2")` equals `Some(&"value2")`.
    /// - Confirms `cache.get(&"key3")` equals `Some(&"value3")`.
    #[test]
    fn test_eviction() {
        let mut cache = cache_with_capacity::<&str, &str>(2);

        cache.put("key1", "value1");
        cache.put("key2", "value2");
        cache.put("key3", "value3"); // Evicts key1

        assert_eq!(cache.get(&"key1"), None);
        assert_eq!(cache.get(&"key2"), Some(&"value2"));
        assert_eq!(cache.get(&"key3"), Some(&"value3"));
    }

    /// Validates the peek scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.peek(&"key1")` equals `Some(&"value1")`.
    /// - Confirms `cache.get(&"key1")` equals `None`.
    #[test]
    fn test_peek() {
        let mut cache = cache_with_capacity::<&str, &str>(2);

        cache.put("key1", "value1");
        cache.put("key2", "value2");

        // Peek doesn't update access time
        assert_eq!(cache.peek(&"key1"), Some(&"value1"));

        cache.put("key3", "value3"); // Should still evict key1
        assert_eq!(cache.get(&"key1"), None);
    }

    /// Validates the clear scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.len()` equals `0`.
    /// - Ensures `cache.is_empty()` evaluates to true.
    #[test]
    fn test_clear() {
        let mut cache = cache_with_capacity::<&str, &str>(2);

        cache.put("key1", "value1");
        cache.put("key2", "value2");

        cache.clear();

        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    /// Validates `NonZeroUsize::new` behavior for the resize scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.cap()` equals `3`.
    /// - Confirms `cache.len()` equals `3`.
    #[test]
    fn test_resize() {
        let mut cache = cache_with_capacity::<&str, &str>(2);

        cache.put("key1", "value1");
        cache.put("key2", "value2");

        cache.resize(NonZeroUsize::new(3).expect("capacity must be non-zero"));
        assert_eq!(cache.cap(), 3);

        cache.put("key3", "value3");
        assert_eq!(cache.len(), 3);
    }

    /// Validates `String::from` behavior for the borrowed lookup support
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get("key1")` equals `Some(&1)`.
    /// - Ensures `cache.contains("key1")` evaluates to true.
    /// - Ensures `cache .get_mut("key1") .map(|value| { *value = 2; })
    ///   .is_some()` evaluates to true.
    /// - Confirms `cache.peek(&String::from("key1"))` equals `Some(&2)`.
    /// - Confirms `cache.pop("key1")` equals `Some(2)`.
    #[test]
    fn test_borrowed_lookup_support() {
        let mut cache = cache_with_capacity::<String, u32>(2);

        cache.put(String::from("key1"), 1_u32);

        assert_eq!(cache.get("key1"), Some(&1));
        assert!(cache.contains("key1"));

        assert!(cache
            .get_mut("key1")
            .map(|value| {
                *value = 2;
            })
            .is_some());

        assert_eq!(cache.peek(&String::from("key1")), Some(&2));
        assert_eq!(cache.pop("key1"), Some(2));
    }

    /// Validates `LruCache::try_new` behavior for the try methods reject zero
    /// capacity scenario.
    ///
    /// Assertions:
    /// - Ensures `LruCache::<&str` evaluates to true.
    /// - Ensures `!cache.try_resize(0)` evaluates to true.
    /// - Confirms `cache.cap()` equals `2`.
    #[test]
    fn test_try_methods_reject_zero_capacity() {
        assert!(LruCache::<&str, &str>::try_new(0).is_none());

        let mut cache = cache_with_capacity::<&str, &str>(2);
        assert!(!cache.try_resize(0));

        assert_eq!(cache.cap(), 2);
    }

    /// Validates the iter returns most recent first scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.get(&"a")` equals `Some(&1)`.
    /// - Confirms `order` equals `vec!["a", "c", "b"]`.
    #[test]
    fn iter_returns_most_recent_first() {
        let mut cache = cache_with_capacity::<&str, i32>(3);

        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        // Touch "a" to promote it.
        assert_eq!(cache.get(&"a"), Some(&1));

        let order: Vec<_> = cache.iter().map(|(key, _)| *key).collect();
        assert_eq!(order, vec!["a", "c", "b"]);
    }
}
