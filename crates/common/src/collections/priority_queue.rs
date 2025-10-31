#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, clippy::perf, clippy::complexity, clippy::suspicious)]

//! Priority queue utilities backed by [`std::collections::BinaryHeap`].
//!
//! # Complexity
//! - `push`: `O(log n)`
//! - `pop`: `O(log n)`
//! - `peek`: `O(1)`
//!
//! # Panic Safety
//! All provided APIs are panic-free.
//!
//! # Examples
//! ```
//! use pulsearc_common::collections::{MaxHeap, MinHeap};
//!
//! let mut min_heap = MinHeap::new();
//! min_heap.extend([3, 1, 4]);
//! assert_eq!(min_heap.pop(), Some(1));
//!
//! let mut max_heap = MaxHeap::with_capacity(4);
//! max_heap.push(10);
//! max_heap.push(2);
//! assert_eq!(max_heap.peek(), Some(&10));
//! ```

use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::iter::FromIterator;
use std::{fmt, vec};

/// Shared priority-queue operations.
pub trait PriorityQueue<T> {
    /// Adds a value to the queue.
    fn push(&mut self, item: T);
    /// Removes the highest-priority value, returning `None` when empty.
    fn pop(&mut self) -> Option<T>;
    /// Borrows the current highest-priority value without removing it.
    fn peek(&self) -> Option<&T>;
    /// Returns the number of queued items.
    fn len(&self) -> usize;
    /// Returns `true` when the queue contains no items.
    fn is_empty(&self) -> bool;
    /// Removes all items from the queue.
    fn clear(&mut self);
}

/// A min-heap priority queue where the smallest element has the highest
/// priority.
///
/// # Complexity
/// - `push`: `O(log n)`
/// - `pop`: `O(log n)`
/// - `peek`: `O(1)`
///
/// # Panic Safety
/// All operations are panic-free.
///
/// # Examples
/// ```
/// use pulsearc_common::collections::MinHeap;
///
/// let mut heap = MinHeap::new();
/// heap.extend([5, 2, 8]);
/// assert_eq!(heap.pop(), Some(2));
/// assert_eq!(heap.into_sorted_vec(), vec![5, 8]);
/// ```
pub struct MinHeap<T>(BinaryHeap<Reverse<T>>);

impl<T: Ord> MinHeap<T> {
    /// Creates an empty min-heap.
    #[must_use]
    pub fn new() -> Self {
        Self(BinaryHeap::new())
    }

    /// Creates an empty min-heap with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(BinaryHeap::with_capacity(capacity))
    }

    /// Adds a value to the heap.
    pub fn push(&mut self, item: T) {
        self.0.push(Reverse(item));
    }

    /// Removes and returns the smallest value in the heap.
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop().map(|Reverse(item)| item)
    }

    /// Returns a reference to the smallest value without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<&T> {
        self.0.peek().map(|reverse| &reverse.0)
    }

    /// Returns the number of elements currently in the heap.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the heap contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes all elements from the heap.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Consumes the heap and returns the elements in ascending order.
    #[must_use]
    pub fn into_sorted_vec(self) -> Vec<T> {
        let mut data = self.0.into_sorted_vec();
        data.reverse();
        data.into_iter().map(|Reverse(item)| item).collect()
    }
}

impl<T: Ord> Default for MinHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> fmt::Debug for MinHeap<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let elements: Vec<&T> = self.0.iter().map(|reverse| &reverse.0).collect();
        f.debug_struct("MinHeap").field("len", &self.len()).field("elements", &elements).finish()
    }
}

impl<T: Ord> FromIterator<T> for MinHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut heap = Self::new();
        heap.extend(iter);
        heap
    }
}

impl<T: Ord> Extend<T> for MinHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter.into_iter().map(Reverse));
    }
}

impl<T: Ord> IntoIterator for MinHeap<T> {
    type Item = T;
    type IntoIter = vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_sorted_vec().into_iter()
    }
}

impl<T: Ord> PriorityQueue<T> for MinHeap<T> {
    fn push(&mut self, item: T) {
        Self::push(self, item);
    }

    fn pop(&mut self) -> Option<T> {
        Self::pop(self)
    }

    fn peek(&self) -> Option<&T> {
        Self::peek(self)
    }

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn clear(&mut self) {
        Self::clear(self);
    }
}

/// A max-heap priority queue where the largest element has the highest
/// priority.
///
/// # Complexity
/// - `push`: `O(log n)`
/// - `pop`: `O(log n)`
/// - `peek`: `O(1)`
///
/// # Panic Safety
/// All operations are panic-free.
///
/// # Examples
/// ```
/// use pulsearc_common::collections::MaxHeap;
///
/// let mut heap = MaxHeap::from_iter([5, 2, 8]);
/// assert_eq!(heap.peek(), Some(&8));
/// assert_eq!(heap.into_sorted_vec(), vec![8, 5, 2]);
/// ```
pub struct MaxHeap<T>(BinaryHeap<T>);

impl<T: Ord> MaxHeap<T> {
    /// Creates an empty max-heap.
    #[must_use]
    pub fn new() -> Self {
        Self(BinaryHeap::new())
    }

    /// Creates an empty max-heap with the specified capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self(BinaryHeap::with_capacity(capacity))
    }

    /// Adds a value to the heap.
    pub fn push(&mut self, item: T) {
        self.0.push(item);
    }

    /// Removes and returns the largest value in the heap.
    pub fn pop(&mut self) -> Option<T> {
        self.0.pop()
    }

    /// Returns a reference to the largest value without removing it.
    #[must_use]
    pub fn peek(&self) -> Option<&T> {
        self.0.peek()
    }

    /// Returns the number of elements currently in the heap.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the heap contains no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Removes all elements from the heap.
    pub fn clear(&mut self) {
        self.0.clear();
    }

    /// Consumes the heap and returns the elements in descending order.
    #[must_use]
    pub fn into_sorted_vec(self) -> Vec<T> {
        let mut data = self.0.into_sorted_vec();
        data.reverse();
        data
    }
}

impl<T: Ord> Default for MaxHeap<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Ord> fmt::Debug for MaxHeap<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MaxHeap").field("len", &self.len()).field("elements", &self.0).finish()
    }
}

impl<T: Ord> FromIterator<T> for MaxHeap<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut heap = Self::new();
        heap.extend(iter);
        heap
    }
}

impl<T: Ord> Extend<T> for MaxHeap<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        self.0.extend(iter);
    }
}

impl<T: Ord> IntoIterator for MaxHeap<T> {
    type Item = T;
    type IntoIter = vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.into_sorted_vec().into_iter()
    }
}

impl<T: Ord> PriorityQueue<T> for MaxHeap<T> {
    fn push(&mut self, item: T) {
        Self::push(self, item);
    }

    fn pop(&mut self) -> Option<T> {
        Self::pop(self)
    }

    fn peek(&self) -> Option<&T> {
        Self::peek(self)
    }

    fn len(&self) -> usize {
        Self::len(self)
    }

    fn is_empty(&self) -> bool {
        Self::is_empty(self)
    }

    fn clear(&mut self) {
        Self::clear(self);
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::priority_queue.
    use super::{MaxHeap, MinHeap};

    /// Validates `MinHeap::new` behavior for the min heap orders ascending
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `popped` equals `vec![1, 3, 5, 7]`.
    /// - Ensures `heap.is_empty()` evaluates to true.
    #[test]
    fn min_heap_orders_ascending() {
        let mut heap = MinHeap::new();
        heap.extend([5, 3, 7, 1]);
        let mut popped = Vec::new();

        while let Some(value) = heap.pop() {
            popped.push(value);
        }

        assert_eq!(popped, vec![1, 3, 5, 7]);
        assert!(heap.is_empty());
    }

    /// Validates `MaxHeap::default` behavior for the max heap orders descending
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `popped` equals `vec![7, 5, 3, 1]`.
    /// - Ensures `heap.is_empty()` evaluates to true.
    #[test]
    fn max_heap_orders_descending() {
        let mut heap = MaxHeap::default();
        heap.extend([5, 3, 7, 1]);
        let mut popped = Vec::new();

        while let Some(value) = heap.pop() {
            popped.push(value);
        }

        assert_eq!(popped, vec![7, 5, 3, 1]);
        assert!(heap.is_empty());
    }

    /// Validates `MinHeap::from_iter` behavior for the peek does not remove
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `min_heap.peek()` equals `Some(&2)`.
    /// - Confirms `min_heap.len()` equals `3`.
    /// - Confirms `max_heap.peek()` equals `Some(&9)`.
    /// - Confirms `max_heap.len()` equals `3`.
    #[test]
    fn peek_does_not_remove() {
        let min_heap = MinHeap::from_iter([5, 2, 9]);
        assert_eq!(min_heap.peek(), Some(&2));
        assert_eq!(min_heap.len(), 3);

        let max_heap = MaxHeap::from_iter([5, 2, 9]);
        assert_eq!(max_heap.peek(), Some(&9));
        assert_eq!(max_heap.len(), 3);
    }

    /// Validates `MinHeap::from_iter` behavior for the into sorted vec orders
    /// correctly scenario.
    ///
    /// Assertions:
    /// - Confirms `min_vec` equals `vec![1, 2, 3]`.
    /// - Confirms `max_vec` equals `vec![3, 2, 1]`.
    #[test]
    fn into_sorted_vec_orders_correctly() {
        let min_vec = MinHeap::from_iter([3, 1, 2]).into_sorted_vec();
        assert_eq!(min_vec, vec![1, 2, 3]);

        let max_vec = MaxHeap::from_iter([3, 1, 2]).into_sorted_vec();
        assert_eq!(max_vec, vec![3, 2, 1]);
    }

    /// Validates `MinHeap::from_iter` behavior for the from iter and extend
    /// match behavior scenario.
    ///
    /// Assertions:
    /// - Confirms `heap_from_iter.into_sorted_vec()` equals
    ///   `heap_from_extend.into_sorted_vec()`.
    #[test]
    fn from_iter_and_extend_match_behavior() {
        let heap_from_iter = MinHeap::from_iter([4, 1, 7, 3]);

        let mut heap_from_extend = MinHeap::new();
        heap_from_extend.extend([4, 1, 7, 3]);

        assert_eq!(heap_from_iter.into_sorted_vec(), heap_from_extend.into_sorted_vec());
    }

    /// Validates `MaxHeap::from_iter` behavior for the clear resets state
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `heap.is_empty()` evaluates to true.
    /// - Confirms `heap.len()` equals `0`.
    /// - Confirms `heap.pop()` equals `None`.
    #[test]
    fn clear_resets_state() {
        let mut heap = MaxHeap::from_iter([1, 2, 3]);
        heap.clear();

        assert!(heap.is_empty());
        assert_eq!(heap.len(), 0);
        assert_eq!(heap.pop(), None);
    }
}
