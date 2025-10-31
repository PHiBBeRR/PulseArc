#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, clippy::perf, clippy::complexity, clippy::suspicious)]

//! A fixed-capacity circular buffer with overwrite-on-full and constant-time
//! operations.
//!
//! A [`RingBuffer`] stores items in insertion order while keeping its length
//! bounded by the caller-provided capacity. When [`push`](RingBuffer::push)
//! receives a new value and the buffer is full, the oldest element (logical
//! index `0`) is discarded and the new value is appended. Index-based access
//! walks forward from the logical head; after every push or pop the head and
//! tail positions move modulo the fixed capacity.
//!
//!
//! # Complexity
//! - `push`, `pop`, `get`, `get_mut`, `len`, `is_empty`, `is_full`, `capacity`,
//!   `iter`, `iter_mut`, and `as_slices` are all **O(1)** time.
//!
//! # Panic Safety
//! - Public methods avoid panicking; there are no `unwrap`/`expect` calls in
//!   the implementation.
//!
//! # Thread Safety
//! - `RingBuffer<T>` uses no interior mutability and no `unsafe`. It is
//!   `Send`/`Sync` if `T` is `Send`/`Sync`.

use std::collections::VecDeque;

/// A fixed-capacity circular buffer storing elements in first-in-first-out
/// order.
///
/// # Examples
///
/// ```rust
/// use pulsearc_common::collections::RingBuffer;
///
/// let mut buffer = RingBuffer::new(3);
/// buffer.push(1);
/// buffer.push(2);
/// buffer.push(3);
/// buffer.push(4); // overwrites the oldest item (`1`)
///
/// assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4]);
/// assert_eq!(buffer.pop(), Some(2));
/// assert_eq!(buffer.get(0), Some(&3));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RingBuffer<T> {
    buf: VecDeque<T>,
    capacity: usize,
}

/// Contiguous slice pair representing the logical contents of the ring buffer.
type SlicePair<'a, T> = (&'a [T], &'a [T]);

impl<T> RingBuffer<T> {
    /// Creates a new buffer with the provided capacity.
    ///
    /// A capacity of zero is clamped to `1`, ensuring at least one slot without
    /// panicking.
    #[inline]
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { buf: VecDeque::with_capacity(capacity), capacity }
    }

    /// Pushes an item to the buffer, overwriting the oldest item when full.
    #[inline]
    pub fn push(&mut self, item: T) {
        if self.is_full() {
            let _ = self.buf.pop_front();
        }
        self.buf.push_back(item);
    }

    /// Removes and returns the oldest item in the buffer.
    #[inline]
    #[must_use]
    pub fn pop(&mut self) -> Option<T> {
        self.buf.pop_front()
    }

    /// Returns an immutable reference to the value at `idx`, counting from the
    /// oldest element.
    ///
    /// The index maps to `(head + idx) % capacity`, so `idx == 0` always refers
    /// to the most ancient element currently stored.
    #[inline]
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.buf.get(idx)
    }

    /// Returns a mutable reference to the value at `idx`, counting from the
    /// oldest element.
    #[inline]
    #[must_use]
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut T> {
        self.buf.get_mut(idx)
    }

    /// Returns the number of items currently stored.
    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    /// Returns `true` when the buffer has no items.
    #[inline]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Returns `true` when the buffer reached its capacity.
    #[inline]
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    /// Returns the maximum number of items the buffer can hold.
    #[inline]
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Removes all elements, leaving the capacity unchanged.
    #[inline]
    pub fn clear(&mut self) {
        self.buf.clear();
    }

    /// Returns an iterator visiting elements from oldest to newest.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buf.iter()
    }

    /// Returns a mutable iterator visiting elements from oldest to newest.
    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.buf.iter_mut()
    }

    /// Returns two contiguous slices that represent the logical contents of the
    /// buffer.
    #[inline]
    #[must_use]
    pub fn as_slices(&self) -> SlicePair<'_, T> {
        self.buf.as_slices()
    }
}

impl<T> Default for RingBuffer<T> {
    /// Creates a single-slot buffer that overwrites on every push after the
    /// first.
    #[inline]
    fn default() -> Self {
        Self::new(1)
    }
}

impl<T> IntoIterator for RingBuffer<T> {
    type Item = T;
    type IntoIter = std::collections::vec_deque::IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.buf.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a RingBuffer<T> {
    type Item = &'a T;
    type IntoIter = std::collections::vec_deque::Iter<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.buf.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut RingBuffer<T> {
    type Item = &'a mut T;
    type IntoIter = std::collections::vec_deque::IterMut<'a, T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.buf.iter_mut()
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::ring_buffer.
    use super::RingBuffer;

    /// Validates `RingBuffer::new` behavior for the fifo with overwrite
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `buffer.iter().copied().collect::<Vec<_>>()` equals `vec![2,
    ///   3, 4]`.
    /// - Confirms `buffer.pop()` equals `Some(2)`.
    /// - Confirms `buffer.pop()` equals `Some(3)`.
    /// - Confirms `buffer.pop()` equals `Some(4)`.
    /// - Confirms `buffer.pop()` equals `None`.
    #[test]
    fn fifo_with_overwrite() {
        let mut buffer = RingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        buffer.push(4);

        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![2, 3, 4]);
        assert_eq!(buffer.pop(), Some(2));
        assert_eq!(buffer.pop(), Some(3));
        assert_eq!(buffer.pop(), Some(4));
        assert_eq!(buffer.pop(), None);
    }

    /// Validates `RingBuffer::new` behavior for the get and get mut are ordered
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `buffer.get(1).map(String::as_str)` equals `Some("beta")`.
    /// - Confirms `buffer.get(1).map(String::as_str)` equals
    ///   `Some("beta-mutated")`.
    /// - Confirms `buffer.get(3)` equals `None`.
    #[test]
    fn get_and_get_mut_are_ordered() {
        let mut buffer = RingBuffer::new(4);
        buffer.push(String::from("alpha"));
        buffer.push(String::from("beta"));
        buffer.push(String::from("gamma"));

        assert_eq!(buffer.get(1).map(String::as_str), Some("beta"));
        if let Some(value) = buffer.get_mut(1) {
            value.push_str("-mutated");
        }
        assert_eq!(buffer.get(1).map(String::as_str), Some("beta-mutated"));
        assert_eq!(buffer.get(3), None);
    }

    /// Validates `RingBuffer::new` behavior for the iter preserves order after
    /// wraparound scenario.
    ///
    /// Assertions:
    /// - Confirms `collected` equals `vec![2, 3, 4]`.
    /// - Confirms `collected_ref` equals `vec![2, 3, 4]`.
    /// - Confirms `collected_mut` equals `vec![2, 3, 4]`.
    /// - Confirms `collected_owned` equals `vec![2, 3, 4]`.
    #[test]
    fn iter_preserves_order_after_wraparound() {
        let mut buffer = RingBuffer::new(3);
        for value in 0..5 {
            buffer.push(value);
        }

        let collected: Vec<_> = buffer.iter().copied().collect();
        assert_eq!(collected, vec![2, 3, 4]);

        let collected_ref: Vec<_> = (&buffer).into_iter().copied().collect();
        assert_eq!(collected_ref, vec![2, 3, 4]);

        let mut clone_for_mut = buffer.clone();
        let collected_mut: Vec<_> = (&mut clone_for_mut).into_iter().map(|value| *value).collect();
        assert_eq!(collected_mut, vec![2, 3, 4]);

        let collected_owned: Vec<_> = buffer.clone().into_iter().collect();
        assert_eq!(collected_owned, vec![2, 3, 4]);
    }

    /// Validates `RingBuffer::new` behavior for the capacity one edge case
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `buffer.len()` equals `1`.
    /// - Ensures `buffer.is_full()` evaluates to true.
    /// - Confirms `buffer.iter().copied().collect::<Vec<_>>()` equals
    ///   `vec!['b']`.
    /// - Confirms `buffer.pop()` equals `Some('b')`.
    /// - Ensures `buffer.is_empty()` evaluates to true.
    #[test]
    fn capacity_one_edge_case() {
        let mut buffer = RingBuffer::new(1);
        buffer.push('a');
        buffer.push('b');

        assert_eq!(buffer.len(), 1);
        assert!(buffer.is_full());
        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec!['b']);
        assert_eq!(buffer.pop(), Some('b'));
        assert!(buffer.is_empty());
    }

    /// Validates `RingBuffer::new` behavior for the clear resets length but
    /// retains capacity scenario.
    ///
    /// Assertions:
    /// - Ensures `buffer.is_empty()` evaluates to true.
    /// - Confirms `buffer.len()` equals `0`.
    /// - Confirms `buffer.capacity()` equals `2`.
    /// - Confirms `buffer.iter().copied().collect::<Vec<_>>()` equals
    ///   `vec![30]`.
    #[test]
    fn clear_resets_length_but_retains_capacity() {
        let mut buffer = RingBuffer::new(2);
        buffer.push(10);
        buffer.push(20);
        buffer.clear();

        assert!(buffer.is_empty());
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.capacity(), 2);

        buffer.push(30);
        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![30]);
    }

    /// Validates `RingBuffer::new` behavior for the zero capacity is clamped to
    /// one scenario.
    ///
    /// Assertions:
    /// - Confirms `buffer.capacity()` equals `1`.
    /// - Confirms `buffer.iter().copied().collect::<Vec<_>>()` equals
    ///   `vec![43]`.
    #[test]
    fn zero_capacity_is_clamped_to_one() {
        let mut buffer = RingBuffer::new(0);
        assert_eq!(buffer.capacity(), 1);

        buffer.push(42);
        buffer.push(43);

        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![43]);
    }

    /// Validates `RingBuffer::new` behavior for the as slices matches iter
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `combined` equals
    ///   `buffer.iter().copied().collect::<Vec<_>>()`.
    #[test]
    fn as_slices_matches_iter() {
        let mut buffer = RingBuffer::new(4);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        let _ = buffer.pop();
        buffer.push(4);
        buffer.push(5);

        let (first, second) = buffer.as_slices();
        let mut combined = first.to_vec();
        combined.extend_from_slice(second);

        assert_eq!(combined, buffer.iter().copied().collect::<Vec<_>>());
    }

    /// Validates `RingBuffer::new` behavior for the iter mut updates items in
    /// place scenario.
    ///
    /// Assertions:
    /// - Confirms `buffer.iter().copied().collect::<Vec<_>>()` equals `vec![2,
    ///   4, 6]`.
    #[test]
    fn iter_mut_updates_items_in_place() {
        let mut buffer = RingBuffer::new(3);
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);

        for value in buffer.iter_mut() {
            *value *= 2;
        }

        assert_eq!(buffer.iter().copied().collect::<Vec<_>>(), vec![2, 4, 6]);
    }
}
