#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, clippy::perf, clippy::complexity, clippy::suspicious)]

//! An in-memory Least Recently Used (LRU) cache with predictable `O(1)` APIs.
//!
//! # Complexity
//! - `new`, `cap`, `len`, `is_empty`, `clear`: `O(1)` (amortized for `clear`
//!   only due to drops).
//! - `put`, `get`, `get_mut`, `peek`, `contains`, `pop`, `resize`, `iter`:
//!   `O(1)` amortized.
//!
//! # Panic Safety
//! - Construction requires a `NonZeroUsize`; resizing uses the same guard.
//! - All other operations avoid panics when used with valid indices and
//!   maintain internal invariants with debug assertions.
//!
//! # Thread Safety
//! - `LruCache` is not `Sync` or `Send`; wrap it in synchronization primitives
//!   to share across threads.
//!
//! # Eviction Policy
//! - A successful `put`, `get`, or `get_mut` promotes the corresponding entry
//!   to the most recently used (MRU) position.
//! - When the cache reaches capacity, inserting a new key evicts the least
//!   recently used (LRU) entry.

use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;
use std::iter::FusedIterator;
use std::num::NonZeroUsize;
use std::rc::Rc;

type NodeSlot<K, V> = Option<Node<K, V>>;

/// LRU cache backed by an indexable doubly linked list stored in a `Vec`.
///
/// The cache promotes entries to MRU on successful `put`, `get`, and `get_mut`
/// calls while `peek` inspects without promotion.
///
/// # Examples
///
/// ```
/// use std::num::NonZeroUsize;
///
/// use pulsearc_common::collections::LruCache;
///
/// let mut cache = LruCache::new(NonZeroUsize::new(2).unwrap());
/// cache.put("k1", "v1");
/// cache.put("k2", "v2");
/// assert_eq!(cache.get(&"k1"), Some(&"v1"));
/// cache.put("k3", "v3"); // Evicts "k2"
/// assert!(cache.get(&"k2").is_none());
/// ```
pub struct LruCache<K, V>
where
    K: Eq + Hash,
{
    capacity: NonZeroUsize,
    map: HashMap<Rc<K>, usize>,
    nodes: Vec<NodeSlot<K, V>>,
    free_list: Vec<usize>,
    head: Option<usize>,
    tail: Option<usize>,
    len: usize,
}

impl<K, V> LruCache<K, V>
where
    K: Eq + Hash,
{
    /// Creates a cache with the provided non-zero capacity.
    pub fn new(capacity: NonZeroUsize) -> Self {
        Self {
            map: HashMap::with_capacity(capacity.get()),
            nodes: Vec::with_capacity(capacity.get()),
            free_list: Vec::new(),
            capacity,
            head: None,
            tail: None,
            len: 0,
        }
    }

    /// Attempts to construct a cache, returning `None` when `capacity` is zero.
    pub fn try_new(capacity: usize) -> Option<Self> {
        NonZeroUsize::new(capacity).map(Self::new)
    }

    /// Returns the maximum number of elements stored without evicting.
    #[must_use]
    pub fn cap(&self) -> usize {
        self.capacity.get()
    }

    /// Returns the number of elements currently stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` when the cache has no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Removes all entries, keeping the configured capacity.
    pub fn clear(&mut self) {
        self.map.clear();
        self.nodes.clear();
        self.free_list.clear();
        self.head = None;
        self.tail = None;
        self.len = 0;
    }

    /// Inserts or updates a key-value pair, returning the previous value when
    /// present.
    ///
    /// Successful inserts promote the entry to MRU. When the cache is full, the
    /// LRU entry is evicted first.
    pub fn put(&mut self, key: K, value: V) -> Option<V> {
        if let Some(&index) = self.map.get(&key) {
            let node = self.nodes[index].as_mut().expect("index mapped to vacant slot");
            let previous = std::mem::replace(&mut node.value, value);
            self.promote(index);
            return Some(previous);
        }

        if self.len == self.capacity.get() {
            let _ = self.evict_lru();
        }

        let key_ptr = Rc::new(key);
        let index = self.allocate_slot(Rc::clone(&key_ptr), value);
        self.attach_front(index);
        self.map.insert(key_ptr, index);
        self.len += 1;
        None
    }

    /// Retrieves a value by key, promoting the entry to MRU when found.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let &index = self.map.get(key)?;
        self.promote(index);
        self.nodes[index].as_ref().map(|node| &node.value)
    }

    /// Retrieves a mutable value reference by key, promoting the entry to MRU
    /// when found.
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        let &index = self.map.get(key)?;
        self.promote(index);
        self.nodes[index].as_mut().map(|node| &mut node.value)
    }

    /// Reads a value by key without altering the recency order.
    #[must_use]
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.map.get(key).and_then(|&index| self.nodes[index].as_ref().map(|node| &node.value))
    }

    /// Returns `true` when a value associated with `key` exists.
    #[must_use]
    pub fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }

    /// Removes and returns the value associated with `key`, if present.
    pub fn pop(&mut self, key: &K) -> Option<V> {
        let index = self.map.remove(key)?;
        self.detach(index);
        self.len = self.len.saturating_sub(1);
        let node = self.nodes[index].take().expect("index mapped to vacant slot");
        self.free_list.push(index);
        Some(node.value)
    }

    /// Resizes the cache, evicting LRU entries when shrinking.
    pub fn resize(&mut self, capacity: NonZeroUsize) {
        self.capacity = capacity;
        while self.len > self.capacity.get() {
            let _ = self.evict_lru();
        }
    }

    /// Tries to resize the cache, returning `false` when `capacity` is zero.
    pub fn try_resize(&mut self, capacity: usize) -> bool {
        match NonZeroUsize::new(capacity) {
            Some(capacity) => {
                self.resize(capacity);
                true
            }
            None => false,
        }
    }

    /// Returns an iterator that yields entries from MRU to LRU.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter { cache: self, current: self.head, remaining: self.len }
    }

    fn allocate_slot(&mut self, key: Rc<K>, value: V) -> usize {
        if let Some(index) = self.free_list.pop() {
            self.nodes[index] = Some(Node::new(key, value));
            index
        } else {
            self.nodes.push(Some(Node::new(key, value)));
            self.nodes.len() - 1
        }
    }

    fn promote(&mut self, index: usize) {
        if self.head == Some(index) {
            return;
        }
        self.detach(index);
        self.attach_front(index);
    }

    fn evict_lru(&mut self) -> Option<Node<K, V>> {
        let index = self.tail?;
        self.detach(index);
        self.len = self.len.saturating_sub(1);
        let node = self.nodes[index].take().expect("tail mapped to vacant slot");
        self.map.remove(&node.key);
        self.free_list.push(index);
        Some(node)
    }

    fn detach(&mut self, index: usize) {
        let (prev, next) = match self.nodes.get(index).and_then(Option::as_ref) {
            Some(node) => (node.prev, node.next),
            None => return,
        };

        match prev {
            Some(prev_index) => {
                if let Some(prev_node) = self.nodes.get_mut(prev_index).and_then(Option::as_mut) {
                    prev_node.next = next;
                }
            }
            None => {
                self.head = next;
            }
        }

        match next {
            Some(next_index) => {
                if let Some(next_node) = self.nodes.get_mut(next_index).and_then(Option::as_mut) {
                    next_node.prev = prev;
                }
            }
            None => {
                self.tail = prev;
            }
        }

        if let Some(node) = self.nodes.get_mut(index).and_then(Option::as_mut) {
            node.prev = None;
            node.next = None;
        }
    }

    fn attach_front(&mut self, index: usize) {
        if let Some(node) = self.nodes.get_mut(index).and_then(Option::as_mut) {
            node.prev = None;
            node.next = self.head;
        }

        if let Some(head_index) = self.head {
            if let Some(head_node) = self.nodes.get_mut(head_index).and_then(Option::as_mut) {
                head_node.prev = Some(index);
            }
        } else {
            self.tail = Some(index);
        }

        self.head = Some(index);
    }
}

impl<K, V> fmt::Debug for LruCache<K, V>
where
    K: Eq + Hash,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("LruCache")
            .field("len", &self.len)
            .field("capacity", &self.capacity)
            .finish()
    }
}

#[derive(Debug)]
struct Node<K, V> {
    key: Rc<K>,
    value: V,
    prev: Option<usize>,
    next: Option<usize>,
}

impl<K, V> Node<K, V> {
    fn new(key: Rc<K>, value: V) -> Self {
        Self { key, value, prev: None, next: None }
    }
}

/// Iterator over cache entries from MRU to LRU.
pub struct Iter<'a, K, V>
where
    K: Eq + Hash,
{
    cache: &'a LruCache<K, V>,
    current: Option<usize>,
    remaining: usize,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: Eq + Hash,
{
    type Item = (&'a K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.current?;
        let node = self.cache.nodes[index].as_ref().expect("iterator visited vacant slot");
        self.current = node.next;
        self.remaining = self.remaining.saturating_sub(1);
        Some((node.key.as_ref(), &node.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.remaining, Some(self.remaining))
    }
}

impl<'a, K, V> ExactSizeIterator for Iter<'a, K, V> where K: Eq + Hash {}

impl<'a, K, V> FusedIterator for Iter<'a, K, V> where K: Eq + Hash {}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::lru_cache.
    use std::num::NonZeroUsize;

    use super::LruCache;

    fn cache(capacity: usize) -> LruCache<&'static str, i32> {
        LruCache::new(NonZeroUsize::new(capacity).expect("capacity must be > 0"))
    }

    fn order(cache: &LruCache<&'static str, i32>) -> Vec<&'static str> {
        cache.iter().map(|(key, _)| *key).collect()
    }

    /// Validates the insert and update scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.is_empty()` evaluates to true.
    /// - Confirms `cache.put("a", 1)` equals `None`.
    /// - Ensures `cache.contains(&"a")` evaluates to true.
    /// - Confirms `cache.len()` equals `1`.
    /// - Confirms `cache.put("a", 2)` equals `Some(1)`.
    /// - Confirms `cache.peek(&"a")` equals `Some(&2)`.
    /// - Confirms `cache.get(&"a")` equals `Some(&3)`.
    /// - Confirms `cache.len()` equals `1`.
    #[test]
    fn insert_and_update() {
        let mut cache = cache(2);
        assert!(cache.is_empty());

        assert_eq!(cache.put("a", 1), None);
        assert!(cache.contains(&"a"));
        assert_eq!(cache.len(), 1);

        assert_eq!(cache.put("a", 2), Some(1));
        assert_eq!(cache.peek(&"a"), Some(&2));

        if let Some(value) = cache.get_mut(&"a") {
            *value = 3;
        }

        assert_eq!(cache.get(&"a"), Some(&3));
        assert_eq!(cache.len(), 1);
    }

    /// Validates the get promotes entry scenario.
    ///
    /// Assertions:
    /// - Confirms `order(&cache)` equals `vec!["c", "b", "a"]`.
    /// - Confirms `cache.get(&"a")` equals `Some(&1)`.
    /// - Confirms `order(&cache)` equals `vec!["a", "c", "b"]`.
    #[test]
    fn get_promotes_entry() {
        let mut cache = cache(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        assert_eq!(order(&cache), vec!["c", "b", "a"]);
        assert_eq!(cache.get(&"a"), Some(&1));
        assert_eq!(order(&cache), vec!["a", "c", "b"]);
    }

    /// Validates the peek does not promote scenario.
    ///
    /// Assertions:
    /// - Confirms `order(&cache)` equals `vec!["b", "a"]`.
    /// - Confirms `cache.peek(&"a")` equals `Some(&1)`.
    /// - Confirms `order(&cache)` equals `vec!["b", "a"]`.
    #[test]
    fn peek_does_not_promote() {
        let mut cache = cache(2);
        cache.put("a", 1);
        cache.put("b", 2);

        assert_eq!(order(&cache), vec!["b", "a"]);
        assert_eq!(cache.peek(&"a"), Some(&1));
        assert_eq!(order(&cache), vec!["b", "a"]);
    }

    /// Validates the evicts lru at capacity scenario.
    ///
    /// Assertions:
    /// - Ensures `!cache.contains(&"a")` evaluates to true.
    /// - Confirms `cache.len()` equals `2`.
    /// - Confirms `order(&cache)` equals `vec!["c", "b"]`.
    #[test]
    fn evicts_lru_at_capacity() {
        let mut cache = cache(2);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3); // Evicts "a"

        assert!(!cache.contains(&"a"));
        assert_eq!(cache.len(), 2);
        assert_eq!(order(&cache), vec!["c", "b"]);
    }

    /// Validates `NonZeroUsize::new` behavior for the resize shrinking eviction
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.cap()` equals `2`.
    /// - Confirms `cache.len()` equals `2`.
    /// - Ensures `!cache.contains(&"a")` evaluates to true.
    /// - Confirms `order(&cache)` equals `vec!["c", "b"]`.
    #[test]
    fn resize_shrinking_eviction() {
        let mut cache = cache(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        cache.resize(NonZeroUsize::new(2).unwrap());
        assert_eq!(cache.cap(), 2);
        assert_eq!(cache.len(), 2);
        assert!(!cache.contains(&"a"));
        assert_eq!(order(&cache), vec!["c", "b"]);
    }

    /// Validates the try resize shrinks and reuses capacity scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.try_resize(2)` evaluates to true.
    /// - Confirms `cache.cap()` equals `2`.
    /// - Confirms `order(&cache)` equals `vec!["c", "b"]`.
    /// - Confirms `cache.len()` equals `2`.
    /// - Ensures `cache.contains(&"d")` evaluates to true.
    /// - Confirms `order(&cache)` equals `vec!["d", "c"]`.
    /// - Ensures `cache.try_resize(1)` evaluates to true.
    /// - Confirms `order(&cache)` equals `vec!["z"]`.
    #[test]
    fn try_resize_shrinks_and_reuses_capacity() {
        let mut cache = cache(3);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.put("c", 3);

        assert!(cache.try_resize(2));
        assert_eq!(cache.cap(), 2);
        assert_eq!(order(&cache), vec!["c", "b"]);

        cache.put("d", 4); // Should evict "b"
        assert_eq!(cache.len(), 2);
        assert!(cache.contains(&"d"));
        assert_eq!(order(&cache), vec!["d", "c"]);

        cache.clear();
        assert!(cache.try_resize(1));
        cache.put("z", 26);
        assert_eq!(order(&cache), vec!["z"]);
    }

    /// Validates the pop removes entry scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.pop(&"a")` equals `Some(1)`.
    /// - Ensures `!cache.contains(&"a")` evaluates to true.
    /// - Confirms `cache.len()` equals `1`.
    /// - Confirms `order(&cache)` equals `vec!["b"]`.
    #[test]
    fn pop_removes_entry() {
        let mut cache = cache(2);
        cache.put("a", 1);
        cache.put("b", 2);

        assert_eq!(cache.pop(&"a"), Some(1));
        assert!(!cache.contains(&"a"));
        assert_eq!(cache.len(), 1);
        assert_eq!(order(&cache), vec!["b"]);
    }

    /// Validates the len and capacity invariants scenario.
    ///
    /// Assertions:
    /// - Confirms `cache.cap()` equals `1`.
    /// - Ensures `cache.is_empty()` evaluates to true.
    /// - Confirms `cache.len()` equals `1`.
    /// - Confirms `cache.cap()` equals `1`.
    /// - Confirms `cache.len()` equals `1`.
    /// - Ensures `cache.contains(&"b")` evaluates to true.
    /// - Ensures `cache.is_empty()` evaluates to true.
    /// - Confirms `cache.cap()` equals `1`.
    #[test]
    fn len_and_capacity_invariants() {
        let mut cache = cache(1);
        assert_eq!(cache.cap(), 1);
        assert!(cache.is_empty());

        cache.put("a", 1);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.cap(), 1);

        cache.put("b", 2); // Evicts "a"
        assert_eq!(cache.len(), 1);
        assert!(cache.contains(&"b"));

        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.cap(), 1);
    }
}
