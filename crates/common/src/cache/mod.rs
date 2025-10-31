//! Generic cache implementations with configurable eviction policies
//!
//! This module provides a unified cache framework that consolidates multiple
//! cache implementations across the codebase. It supports various eviction
//! policies (LRU, LFU, FIFO, Random, None) and TTL-based expiration.
//!
//! # Features
//!
//! - **Thread-safe**: Uses `Arc<RwLock<>>` for safe concurrent access
//! - **Generic**: Works with any `K: Eq + Hash + Clone` and `V: Clone`
//! - **Configurable eviction**: LRU, LFU, FIFO, Random, or no eviction
//! - **TTL support**: Automatic expiration based on time-to-live
//! - **Metrics tracking**: Optional hit/miss/eviction statistics
//! - **Testable**: Clock abstraction for deterministic time-based testing
//!
//! # Examples
//!
//! ## Simple LRU Cache
//! ```
//! use pulsearc_common::cache::{Cache, CacheConfig};
//!
//! let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));
//! cache.insert("key".to_string(), 42);
//! assert_eq!(cache.get(&"key".to_string()), Some(42));
//! ```
//!
//! ## TTL-based Cache
//! ```
//! use std::time::Duration;
//!
//! use pulsearc_common::cache::{Cache, CacheConfig};
//!
//! let cache: Cache<String, String> = Cache::new(CacheConfig::ttl(Duration::from_secs(3600)));
//! cache.insert("session".to_string(), "data".to_string());
//! ```
//!
//! ## Combined TTL + LRU
//! ```
//! use std::time::Duration;
//!
//! use pulsearc_common::cache::{Cache, CacheConfig};
//!
//! let cache: Cache<String, Vec<u8>> =
//!     Cache::new(CacheConfig::ttl_lru(Duration::from_secs(300), 1000));
//! ```
//!
//! ## Custom Configuration with Builder
//! ```
//! use std::time::Duration;
//!
//! use pulsearc_common::cache::{Cache, CacheConfig, EvictionPolicy};
//!
//! let config = CacheConfig::builder()
//!     .max_size(500)
//!     .ttl(Duration::from_secs(1800))
//!     .eviction_policy(EvictionPolicy::LFU)
//!     .track_metrics(true)
//!     .build();
//!
//! let cache: Cache<String, i32> = Cache::new(config);
//! ```
//!
//! ## Get or Insert with Lazy Computation
//! ```
//! use pulsearc_common::cache::{Cache, CacheConfig};
//!
//! let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(100));
//!
//! let value = cache.get_or_insert_with("key".to_string(), || {
//!     // Expensive computation only runs if key doesn't exist
//!     expensive_computation()
//! });
//! # fn expensive_computation() -> i32 { 42 }
//! ```
//!
//! ## Cache Statistics
//! ```
//! use pulsearc_common::cache::{Cache, CacheConfig};
//!
//! let config = CacheConfig::builder().max_size(100).track_metrics(true).build();
//!
//! let cache: Cache<String, i32> = Cache::new(config);
//!
//! cache.insert("key1".to_string(), 1);
//! let _ = cache.get(&"key1".to_string());
//!
//! let stats = cache.stats();
//! println!("Hit rate: {:.2}%", stats.hit_rate() * 100.0);
//! println!("Cache size: {}/{:?}", stats.size, stats.max_size);
//! ```
//!
//! # Eviction Policies
//!
//! - **LRU (Least Recently Used)**: Evicts entries that haven't been accessed
//!   recently
//! - **LFU (Least Frequently Used)**: Evicts entries with the lowest access
//!   count
//! - **FIFO (First In First Out)**: Evicts the oldest entries by insertion time
//! - **Random**: Evicts random entries
//! - **None**: No automatic eviction (useful for TTL-only caches)
//!
//! # Thread Safety
//!
//! The cache is thread-safe and can be shared across threads using `Arc`:
//!
//! ```
//! use std::sync::Arc;
//! use std::thread;
//!
//! use pulsearc_common::cache::{Cache, CacheConfig};
//!
//! let cache = Arc::new(Cache::new(CacheConfig::lru(100)));
//!
//! let mut handles = vec![];
//! for i in 0..10 {
//!     let cache_clone = Arc::clone(&cache);
//!     let handle = thread::spawn(move || {
//!         cache_clone.insert(format!("key-{}", i), i);
//!     });
//!     handles.push(handle);
//! }
//!
//! for handle in handles {
//!     handle.join().unwrap();
//! }
//! ```

mod async_core;
mod config;
mod core;
pub mod examples;
mod stats;
pub mod utils;

// Re-export public API
pub use core::Cache;

pub use async_core::AsyncCache;
pub use config::{CacheConfig, CacheConfigBuilder, EvictionPolicy};
pub use stats::CacheStats;
