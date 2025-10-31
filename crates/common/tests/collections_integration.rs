//! Integration tests for `pulsearc_common::collections`.
//!
//! These tests stress the bounded queue, trie, ring buffer, and min-heap
//! adapters together to validate concurrent ingestion and lookup scenarios.

#![allow(clippy::doc_lazy_continuation)]

use std::sync::{Arc, Mutex};
use std::thread;

use pulsearc_common::collections::{BoundedQueue, MinHeap, RingBuffer, Trie};

/// Validates `Arc::new` behavior for the bounded queue drives shared
/// collections scenario.
///
/// Assertions:
/// - Ensures `trie.contains("alpha")` evaluates to true.
/// - Ensures `trie.starts_with("del")` evaluates to true.
/// - Confirms `trie.find_prefix("e")` equals `vec!["epsilon"]`.
/// - Confirms `recent` equals `vec!["gamma".to_string(), "delta".to_string(),
///   "epsilon".to_string()]`.
/// - Confirms `min_heap.pop()` equals `Some("delta".to_string())`.
/// - Confirms `queue.pop().unwrap()` equals `None`.
/// Validates that a bounded queue fan-out correctly feeds the trie, ring
/// buffer, and min-heap helpers under concurrent production/consumption.
#[test]
fn bounded_queue_drives_shared_collections() {
    let queue = Arc::new(BoundedQueue::<String>::new(4));
    let trie = Arc::new(Mutex::new(Trie::new()));
    let buffer = Arc::new(Mutex::new(RingBuffer::<String>::new(3)));

    let consumer_queue = Arc::clone(&queue);
    let consumer_trie = Arc::clone(&trie);
    let consumer_buffer = Arc::clone(&buffer);

    let consumer = thread::spawn(move || {
        while let Some(word) = consumer_queue.pop().unwrap() {
            consumer_trie.lock().unwrap().insert(&word);
            consumer_buffer.lock().unwrap().push(word);
        }
    });

    let words = ["alpha", "beta", "gamma", "delta", "epsilon"];
    for word in words {
        queue.push(word.to_string()).unwrap();
    }
    queue.close();

    consumer.join().unwrap();

    let trie = Arc::try_unwrap(trie).unwrap().into_inner().unwrap();
    let buffer = Arc::try_unwrap(buffer).unwrap().into_inner().unwrap();

    assert!(trie.contains("alpha"));
    assert!(trie.starts_with("del"));
    assert_eq!(trie.find_prefix("e"), vec!["epsilon"]);

    let recent: Vec<_> = buffer.iter().cloned().collect();
    assert_eq!(recent, vec!["gamma".to_string(), "delta".to_string(), "epsilon".to_string()]);

    let mut min_heap = MinHeap::from_iter(recent.clone());
    assert_eq!(min_heap.pop(), Some("delta".to_string()));

    // Queue drained and closed.
    assert_eq!(queue.pop().unwrap(), None);
}
