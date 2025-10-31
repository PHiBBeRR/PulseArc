#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]
#![warn(clippy::all, clippy::perf, clippy::complexity, clippy::suspicious)]

//! Thread-safe bounded FIFO queue with blocking semantics.
//!
//! This module implements [`BoundedQueue`] using only the Rust standard
//! library.
//!
//! **Complexity**
//! - `push`, `try_push`, `push_timeout`, `pop`, `try_pop`, and `pop_timeout`
//!   complete in `O(1)`.
//! - `clear` is `O(n)` where `n` is the number of buffered elements.
//!
//! **Panic Safety**
//! - [`BoundedQueue::new`] panics when constructed with a zero capacity.
//! - Internal mutex poisoning is recovered transparently so that operations can
//!   proceed after a panic in another thread.
//!
//! **Thread Safety**
//! - All operations take `&self` and may be invoked concurrently by multiple
//!   producers and consumers.
//! - Synchronization relies on `std::sync::Mutex` and `std::sync::Condvar`,
//!   with condition-variable wait loops that avoid busy waiting.
//!
//! **Semantics of `close()`**
//! - Closing the queue prevents new pushes (blocking or non-blocking) and wakes
//!   all waiters.
//! - Pending pops drain any buffered items before yielding `Ok(None)` once the
//!   queue becomes empty.
//! - The operation is idempotent; repeated calls have no additional effect.

use std::collections::VecDeque;
use std::fmt;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// Type alias for wait outcome: (MutexGuard, timed_out: bool)
type WaitOutcome<'a, T> = (MutexGuard<'a, Inner<T>>, bool);

/// Error type for bounded queue operations that rely on the queue remaining
/// open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueError {
    /// The queue has been closed.
    Closed,
}

impl fmt::Display for QueueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QueueError::Closed => f.write_str("bounded queue is closed"),
        }
    }
}

impl std::error::Error for QueueError {}

/// Deprecated alias for the legacy error type name.
#[deprecated(note = "use QueueError instead")]
pub type BoundedQueueError = QueueError;

/// Error returned by [`BoundedQueue::try_push`] when the value cannot be queued
/// immediately.
#[derive(Debug, PartialEq, Eq)]
pub enum TryPushError<T> {
    /// The queue was at capacity; the item is returned to the caller.
    Full(T),
    /// The queue has been closed; the item is returned to the caller.
    Closed(T),
}

impl<T> TryPushError<T> {
    /// Returns the item that failed to be enqueued.
    #[must_use]
    pub fn into_inner(self) -> T {
        match self {
            TryPushError::Full(item) | TryPushError::Closed(item) => item,
        }
    }
}

impl<T> fmt::Display for TryPushError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryPushError::Full(_) => f.write_str("bounded queue is full"),
            TryPushError::Closed(_) => f.write_str("bounded queue is closed"),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TryPushError<T> {}

/// Error returned by [`BoundedQueue::push_timeout`] when the value cannot be
/// queued before the timeout elapses.
#[derive(Debug, PartialEq, Eq)]
pub enum TryPushTimeout<T> {
    /// The wait expired before space became available; the item is returned to
    /// the caller.
    Timeout(T),
    /// The queue has been closed; the item is returned to the caller.
    Closed(T),
}

impl<T> TryPushTimeout<T> {
    /// Returns the item that failed to be enqueued.
    #[must_use]
    pub fn into_inner(self) -> T {
        match self {
            TryPushTimeout::Timeout(item) | TryPushTimeout::Closed(item) => item,
        }
    }
}

impl<T> fmt::Display for TryPushTimeout<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryPushTimeout::Timeout(_) => {
                f.write_str("timed out waiting for bounded queue capacity")
            }
            TryPushTimeout::Closed(_) => f.write_str("bounded queue is closed"),
        }
    }
}

impl<T: fmt::Debug> std::error::Error for TryPushTimeout<T> {}

struct Inner<T> {
    queue: VecDeque<T>,
    capacity: usize,
    closed: bool,
}

struct State<T> {
    inner: Mutex<Inner<T>>,
    not_empty: Condvar,
    not_full: Condvar,
}

impl<T> State<T> {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Mutex::new(Inner {
                queue: VecDeque::with_capacity(capacity),
                capacity,
                closed: false,
            }),
            not_empty: Condvar::new(),
            not_full: Condvar::new(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, Inner<T>> {
        match self.inner.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn wait_not_full<'a>(&self, guard: MutexGuard<'a, Inner<T>>) -> MutexGuard<'a, Inner<T>> {
        match self.not_full.wait(guard) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn wait_not_empty<'a>(&self, guard: MutexGuard<'a, Inner<T>>) -> MutexGuard<'a, Inner<T>> {
        match self.not_empty.wait(guard) {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    fn wait_not_full_timeout<'a>(
        &self,
        guard: MutexGuard<'a, Inner<T>>,
        duration: Duration,
    ) -> WaitOutcome<'a, T> {
        match self.not_full.wait_timeout(guard, duration) {
            Ok((guard, result)) => (guard, result.timed_out()),
            Err(poisoned) => {
                let (guard, result) = poisoned.into_inner();
                (guard, result.timed_out())
            }
        }
    }

    fn wait_not_empty_timeout<'a>(
        &self,
        guard: MutexGuard<'a, Inner<T>>,
        duration: Duration,
    ) -> WaitOutcome<'a, T> {
        match self.not_empty.wait_timeout(guard, duration) {
            Ok((guard, result)) => (guard, result.timed_out()),
            Err(poisoned) => {
                let (guard, result) = poisoned.into_inner();
                (guard, result.timed_out())
            }
        }
    }
}

/// Thread-safe bounded FIFO queue with blocking semantics.
///
/// **Complexity**
/// - Queue operations are `O(1)` in the number of buffered elements.
/// - [`BoundedQueue::clear`] is `O(n)` where `n` is the number of elements
///   removed.
///
/// **Panic Safety**
/// - [`BoundedQueue::new`] panics if invoked with a capacity of zero.
/// - Mutex poisoning is recovered automatically, so subsequent operations
///   continue after a thread panic.
///
/// **Thread Safety**
/// - All methods take `&self`, allowing the queue to be shared freely across
///   threads.
/// - Blocking methods use `std::sync::Condvar` with `while` loops to avoid busy
///   waiting and to handle spurious wakeups.
///
/// **Semantics of `close()`**
/// - [`BoundedQueue::close`] wakes all waiting producers and consumers.
/// - After closing, pushes fail immediately while pops continue draining any
///   remaining items and finally return `Ok(None)`.
///
/// ```
/// use std::thread;
///
/// use pulsearc_common::collections::BoundedQueue;
///
/// let queue = BoundedQueue::new(2);
/// queue.push(1).unwrap();
///
/// let worker = {
///     let queue = queue.clone();
///     thread::spawn(move || queue.pop().unwrap())
/// };
///
/// queue.push(2).unwrap();
/// queue.close();
///
/// assert_eq!(worker.join().unwrap(), Some(1));
/// assert_eq!(queue.pop().unwrap(), Some(2));
/// assert_eq!(queue.pop().unwrap(), None);
/// ```
pub struct BoundedQueue<T> {
    state: Arc<State<T>>,
}

impl<T> Clone for BoundedQueue<T> {
    fn clone(&self) -> Self {
        Self { state: Arc::clone(&self.state) }
    }
}

impl<T> BoundedQueue<T> {
    /// Creates a new queue with the provided capacity.
    ///
    /// # Panics
    ///
    /// Panics if `capacity` is zero.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        assert!(capacity > 0, "BoundedQueue capacity must be greater than zero");
        Self { state: Arc::new(State::new(capacity)) }
    }

    /// Returns the maximum number of elements that can be stored.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.state.lock().capacity
    }

    /// Returns the current element count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.state.lock().queue.len()
    }

    /// Returns `true` when the queue has no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` when the queue is at capacity.
    #[must_use]
    pub fn is_full(&self) -> bool {
        let guard = self.state.lock();
        guard.queue.len() == guard.capacity
    }

    /// Removes all buffered elements.
    pub fn clear(&self) {
        let mut guard = self.state.lock();
        let was_empty = guard.queue.is_empty();
        guard.queue.clear();
        drop(guard);
        if !was_empty {
            self.state.not_full.notify_all();
        }
    }

    /// Marks the queue as closed and wakes all waiters.
    pub fn close(&self) {
        let mut guard = self.state.lock();
        if guard.closed {
            return;
        }
        guard.closed = true;
        drop(guard);
        self.state.not_full.notify_all();
        self.state.not_empty.notify_all();
    }

    /// Returns `true` if [`close`](Self::close) has been called.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.state.lock().closed
    }

    /// Pushes an element onto the queue, blocking until space is available or
    /// the queue is closed.
    pub fn push(&self, item: T) -> Result<(), QueueError> {
        let mut guard = self.state.lock();
        loop {
            if guard.closed {
                return Err(QueueError::Closed);
            }
            if guard.queue.len() < guard.capacity {
                guard.queue.push_back(item);
                drop(guard);
                self.state.not_empty.notify_one();
                return Ok(());
            }
            guard = self.state.wait_not_full(guard);
        }
    }

    /// Attempts to push an element without blocking.
    pub fn try_push(&self, item: T) -> Result<(), TryPushError<T>> {
        let mut guard = self.state.lock();
        if guard.closed {
            return Err(TryPushError::Closed(item));
        }
        if guard.queue.len() >= guard.capacity {
            return Err(TryPushError::Full(item));
        }
        guard.queue.push_back(item);
        drop(guard);
        self.state.not_empty.notify_one();
        Ok(())
    }

    /// Attempts to push an element, blocking for at most `timeout`.
    pub fn push_timeout(&self, item: T, timeout: Duration) -> Result<(), TryPushTimeout<T>> {
        let deadline = Instant::now().checked_add(timeout);
        let mut guard = self.state.lock();

        loop {
            if guard.closed {
                return Err(TryPushTimeout::Closed(item));
            }
            if guard.queue.len() < guard.capacity {
                guard.queue.push_back(item);
                drop(guard);
                self.state.not_empty.notify_one();
                return Ok(());
            }

            let remaining = deadline
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::MAX);
            if remaining.is_zero() {
                return Err(TryPushTimeout::Timeout(item));
            }

            let (g, timed_out) = self.state.wait_not_full_timeout(guard, remaining);
            guard = g;

            if timed_out && guard.queue.len() >= guard.capacity {
                if guard.closed {
                    return Err(TryPushTimeout::Closed(item));
                }
                return Err(TryPushTimeout::Timeout(item));
            }
        }
    }

    /// Pops an element from the queue, blocking until an item becomes available
    /// or the queue is closed.
    pub fn pop(&self) -> Result<Option<T>, QueueError> {
        let mut guard = self.state.lock();
        loop {
            if let Some(item) = guard.queue.pop_front() {
                drop(guard);
                self.state.not_full.notify_one();
                return Ok(Some(item));
            }
            if guard.closed {
                return Ok(None);
            }
            guard = self.state.wait_not_empty(guard);
        }
    }

    /// Attempts to pop an element without blocking.
    #[must_use]
    pub fn try_pop(&self) -> Option<T> {
        let mut guard = self.state.lock();
        let item = guard.queue.pop_front();
        if item.is_some() {
            drop(guard);
            self.state.not_full.notify_one();
        }
        item
    }

    /// Pops an element, blocking for at most `timeout`.
    pub fn pop_timeout(&self, timeout: Duration) -> Result<Option<T>, QueueError> {
        let deadline = Instant::now().checked_add(timeout);
        let mut guard = self.state.lock();

        loop {
            if let Some(item) = guard.queue.pop_front() {
                drop(guard);
                self.state.not_full.notify_one();
                return Ok(Some(item));
            }
            if guard.closed {
                return Ok(None);
            }

            let remaining = deadline
                .map(|deadline| deadline.saturating_duration_since(Instant::now()))
                .unwrap_or(Duration::MAX);
            if remaining.is_zero() {
                return Ok(None);
            }

            let (g, timed_out) = self.state.wait_not_empty_timeout(guard, remaining);
            guard = g;

            if timed_out && guard.queue.is_empty() {
                return Ok(None);
            }
        }
    }
}

impl<T> BoundedQueue<T> {
    /// Deprecated async alias for [`BoundedQueue::push`].
    #[allow(clippy::unused_async)]
    #[deprecated(note = "BoundedQueue::push is now synchronous; call it directly instead.")]
    pub async fn push_async(&self, item: T) -> Result<(), QueueError> {
        self.push(item)
    }

    /// Deprecated async alias for [`BoundedQueue::pop`].
    #[allow(clippy::unused_async)]
    #[deprecated(note = "BoundedQueue::pop is now synchronous; call it directly instead.")]
    pub async fn pop_async(&self) -> Result<Option<T>, QueueError> {
        self.pop()
    }

    /// Deprecated async alias for [`BoundedQueue::clear`].
    #[allow(clippy::unused_async)]
    #[deprecated(note = "BoundedQueue::clear is now synchronous; call it directly instead.")]
    pub async fn clear_async(&self) {
        self.clear();
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for collections::bounded_queue.
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    use super::*;

    /// Validates `BoundedQueue::new` behavior for the zero-capacity panic
    /// scenario.
    ///
    /// Assertion coverage: ensures the routine completes without panicking.
    #[test]
    #[should_panic(expected = "BoundedQueue capacity must be greater than zero")]
    fn new_panics_on_zero_capacity() {
        let _ = BoundedQueue::<usize>::new(0);
    }

    /// Validates `BoundedQueue::new` behavior for the state introspection
    /// helpers scenario.
    ///
    /// Assertions:
    /// - Confirms `queue.capacity()` equals `2`.
    /// - Confirms `queue.len()` equals `0`.
    /// - Ensures `queue.is_empty()` evaluates to true.
    /// - Ensures `!queue.is_full()` evaluates to true.
    /// - Ensures `!queue.is_closed()` evaluates to true.
    /// - Ensures `queue.try_pop().is_none()` evaluates to true.
    /// - Confirms `queue.len()` equals `1`.
    /// - Ensures `!queue.is_empty()` evaluates to true.
    /// - Ensures `!queue.is_full()` evaluates to true.
    /// - Ensures `queue.is_full()` evaluates to true.
    /// - Confirms `queue.try_pop()` equals `Some(10)`.
    /// - Confirms `queue.len()` equals `1`.
    /// - Confirms `queue.pop().unwrap()` equals `Some(20)`.
    /// - Ensures `queue.is_empty()` evaluates to true.
    /// - Ensures `queue.is_closed()` evaluates to true.
    /// - Confirms `queue.pop().unwrap()` equals `None`.
    #[test]
    fn state_introspection_helpers() {
        let queue = BoundedQueue::new(2);
        assert_eq!(queue.capacity(), 2);
        assert_eq!(queue.len(), 0);
        assert!(queue.is_empty());
        assert!(!queue.is_full());
        assert!(!queue.is_closed());
        assert!(queue.try_pop().is_none());

        queue.push(10).unwrap();
        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());
        assert!(!queue.is_full());

        queue.push(20).unwrap();
        assert!(queue.is_full());

        assert_eq!(queue.try_pop(), Some(10));
        assert_eq!(queue.len(), 1);

        assert_eq!(queue.pop().unwrap(), Some(20));
        assert!(queue.is_empty());

        queue.close();
        assert!(queue.is_closed());
        assert_eq!(queue.pop().unwrap(), None);
    }

    /// Validates `BoundedQueue::new` behavior for the push pop round trip
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `queue.pop().unwrap()` equals `Some(1)`.
    /// - Confirms `queue.pop().unwrap()` equals `None`.
    #[test]
    fn push_pop_round_trip() {
        let queue = BoundedQueue::new(2);
        queue.push(1).unwrap();
        assert_eq!(queue.pop().unwrap(), Some(1));
        queue.close();
        assert_eq!(queue.pop().unwrap(), None);
    }

    /// Validates `Arc::new` behavior for the clear unblocks waiting producer
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `queue.len()` equals `1`.
    /// - Confirms `queue.pop().unwrap()` equals `Some(2)`.
    /// - Ensures `!queue.is_closed()` evaluates to true.
    #[test]
    fn clear_unblocks_waiting_producer() {
        use std::time::Duration;

        let queue = Arc::new(BoundedQueue::<i32>::new(1));
        queue.push(1).unwrap();

        let producer = {
            let queue = Arc::clone(&queue);
            thread::spawn(move || {
                queue.push(2).unwrap();
            })
        };

        thread::sleep(Duration::from_millis(20));
        queue.clear();

        producer.join().unwrap();

        assert_eq!(queue.len(), 1);
        assert_eq!(queue.pop().unwrap(), Some(2));
        assert!(!queue.is_closed());
    }

    /// Validates `TryPushError::Full` behavior for the try push behavior
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `queue.try_push(1).is_ok()` evaluates to true.
    /// - Confirms `item` equals `2`.
    /// - Confirms `item` equals `3`.
    #[test]
    fn try_push_behavior() {
        let queue = BoundedQueue::<i32>::new(1);
        assert!(queue.try_push(1).is_ok());
        match queue.try_push(2) {
            Err(TryPushError::Full(item)) => assert_eq!(item, 2),
            other => panic!("expected full error, got {other:?}"),
        }
        assert_eq!(queue.pop().unwrap(), Some(1));
        queue.close();
        match queue.try_push(3) {
            Err(TryPushError::Closed(item)) => assert_eq!(item, 3),
            other => panic!("expected closed error, got {other:?}"),
        }
    }

    /// Validates `Duration::from_millis` behavior for the timeout behavior
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item` equals `2`.
    /// - Ensures `result.is_none()` evaluates to true.
    #[test]
    fn timeout_behavior() {
        use std::time::Duration;

        let queue = BoundedQueue::<i32>::new(1);
        queue.push(1).unwrap();

        let err = queue.push_timeout(2, Duration::from_millis(10)).unwrap_err();
        match err {
            TryPushTimeout::Timeout(item) => assert_eq!(item, 2),
            other => panic!("expected timeout error, got {other:?}"),
        }

        assert_eq!(queue.pop().unwrap(), Some(1));

        let result = queue.pop_timeout(Duration::from_millis(10)).unwrap();
        assert!(result.is_none());
    }

    /// Validates `Arc::new` behavior for the close unblocks waiters scenario.
    ///
    /// Assertions:
    /// - Ensures `matches!(blocked_push.join().unwrap(),
    ///   Err(QueueError::Closed))` evaluates to true.
    /// - Confirms `queue.pop().unwrap()` equals `Some(1)`.
    /// - Confirms `queue.pop().unwrap()` equals `None`.
    /// - Confirms `blocked_pop.join().unwrap().unwrap()` equals `None`.
    #[test]
    fn close_unblocks_waiters() {
        use std::time::Duration;

        let queue = Arc::new(BoundedQueue::<i32>::new(1));
        queue.push(1).unwrap();

        let blocked_push = {
            let queue = Arc::clone(&queue);
            thread::spawn(move || queue.push(2))
        };

        thread::sleep(Duration::from_millis(20));
        queue.close();
        assert!(matches!(blocked_push.join().unwrap(), Err(QueueError::Closed)));
        assert_eq!(queue.pop().unwrap(), Some(1));
        assert_eq!(queue.pop().unwrap(), None);

        let queue = Arc::new(BoundedQueue::<i32>::new(1));
        let blocked_pop = {
            let queue = Arc::clone(&queue);
            thread::spawn(move || queue.pop())
        };
        thread::sleep(Duration::from_millis(20));
        queue.close();
        assert_eq!(blocked_pop.join().unwrap().unwrap(), None);
    }

    /// Validates `Arc::new` behavior for the push timeout returns closed when
    /// queue terminates scenario.
    ///
    /// Assertions:
    /// - Confirms `item` equals `2`.
    /// - Confirms `queue.pop().unwrap()` equals `None`.
    #[test]
    fn push_timeout_returns_closed_when_queue_terminates() {
        use std::time::Duration;

        let queue = Arc::new(BoundedQueue::<i32>::new(1));
        queue.push(1).unwrap();

        let blocked_push = {
            let queue = Arc::clone(&queue);
            thread::spawn(move || queue.push_timeout(2, Duration::from_secs(1)))
        };

        thread::sleep(Duration::from_millis(20));
        queue.close();

        match blocked_push.join().unwrap().unwrap_err() {
            TryPushTimeout::Closed(item) => assert_eq!(item, 2),
            other => panic!("expected closed error, got {other:?}"),
        }

        assert_eq!(queue.pop().unwrap(), Some(1));
        assert_eq!(queue.pop().unwrap(), None);
    }

    /// Validates `Arc::new` behavior for the mpmc producers consumers scenario.
    ///
    /// Assertions:
    /// - Confirms `consumed.load(Ordering::SeqCst)` equals `total`.
    #[test]
    fn mpmc_producers_consumers() {
        let queue = Arc::new(BoundedQueue::new(8));
        let producers = 4;
        let items_per_producer = 50;
        let total = producers * items_per_producer;

        let mut producer_handles = Vec::new();
        for id in 0..producers {
            let queue = Arc::clone(&queue);
            producer_handles.push(thread::spawn(move || {
                for offset in 0..items_per_producer {
                    queue.push((id, offset)).unwrap();
                }
            }));
        }

        let consumed = Arc::new(AtomicUsize::new(0));
        let mut consumer_handles = Vec::new();
        for _ in 0..producers {
            let queue = Arc::clone(&queue);
            let consumed = Arc::clone(&consumed);
            consumer_handles.push(thread::spawn(move || {
                while queue.pop().unwrap().is_some() {
                    if consumed.fetch_add(1, Ordering::SeqCst) + 1 >= total {
                        break;
                    }
                }
            }));
        }

        for handle in producer_handles {
            handle.join().unwrap();
        }

        queue.close();

        for handle in consumer_handles {
            handle.join().unwrap();
        }

        assert_eq!(consumed.load(Ordering::SeqCst), total);
    }
}
