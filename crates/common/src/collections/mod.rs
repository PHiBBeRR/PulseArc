//! Specialized data structures
//!
//! This module provides high-performance, specialized data structures:
//! - **[`bounded_queue`]**: Bounded queue with backpressure
//! - **[`lru_cache`]**: LRU cache
//! - **[`ring_buffer`]**: Fixed-size ring buffer
//! - **[`priority_queue`]**: Min/max heap
//! - **[`trie`]**: Trie for string matching
//! - **[`bloom_filter`]**: Probabilistic membership testing
//!
//! ## Usage
//!
//! ```rust,ignore
//! use pulsearc_common::collections::{BoundedQueue, RingBuffer};
//!
//! # async fn example() {
//! // Bounded queue
//! let queue = BoundedQueue::new(100);
//! queue.push(42).await.unwrap();
//!
//! // Ring buffer
//! let mut buffer = RingBuffer::new(10);
//! buffer.push(1);
//! # }
//! ```

pub mod bloom_filter;
pub mod bounded_queue;
pub mod lru;
pub mod lru_cache;
pub mod priority_queue;
pub mod ring_buffer;
pub mod trie;

// Re-export commonly used types
pub use bloom_filter::BloomFilter;
pub use bounded_queue::{BoundedQueue, QueueError, TryPushError, TryPushTimeout};
pub use lru::LruCache as ExternalLruCache;
pub use lru_cache::LruCache;
pub use priority_queue::{MaxHeap, MinHeap, PriorityQueue};
pub use ring_buffer::RingBuffer;
pub use trie::Trie;
