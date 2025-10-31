//! Comprehensive collections benchmarks
//!
//! Benchmarks for specialized data structures including BloomFilter,
//! BoundedQueue, LruCache, PriorityQueue, RingBuffer, and Trie.
//!
//! Run with: `cargo bench --bench collections_bench -p pulsearc-common
//! --features foundation`

use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::collections::{
    BloomFilter, BoundedQueue, LruCache, MaxHeap, MinHeap, PriorityQueue, RingBuffer, Trie,
};

// ============================================================================
// BloomFilter Benchmarks
// ============================================================================

fn bench_bloom_filter_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter_insert");

    for size in [1000, 10_000, 100_000] {
        for fpr in [0.01, 0.001] {
            group.throughput(Throughput::Elements(1));
            group.bench_with_input(
                BenchmarkId::new(format!("size_{}", size), format!("fpr_{}", fpr)),
                &(size, fpr),
                |b, &(size, fpr)| {
                    let mut filter = BloomFilter::new(size, fpr).unwrap();
                    let mut counter = 0u64;
                    b.iter(|| {
                        filter.insert(&black_box(counter));
                        counter = counter.wrapping_add(1);
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_bloom_filter_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter_contains");

    for size in [1000, 10_000, 100_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut filter = BloomFilter::new(size, 0.01).unwrap();
            // Pre-populate with half the expected items
            for i in 0..(size / 2) {
                filter.insert(&i);
            }
            let mut counter = 0u64;
            b.iter(|| {
                let key = counter % (size as u64);
                let result = black_box(filter.contains(&black_box(key)));
                black_box(result);
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_bloom_filter_clear(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter_clear");

    for size in [1000, 10_000, 100_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut filter = BloomFilter::new(size, 0.01).unwrap();
                    for i in 0..size {
                        filter.insert(&i);
                    }
                    filter
                },
                |mut filter| {
                    filter.clear();
                    black_box(filter);
                },
                criterion::BatchSize::SmallInput,
            );
        });
    }

    group.finish();
}

fn bench_bloom_filter_false_positive_rate(c: &mut Criterion) {
    let mut group = c.benchmark_group("bloom_filter_false_positive_rate");

    for size in [1000, 10_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let mut filter = BloomFilter::new(size, 0.01).unwrap();
            for i in 0..size {
                filter.insert(&i);
            }
            b.iter(|| {
                let fpr = black_box(filter.estimated_false_positive_rate());
                black_box(fpr);
            });
        });
    }

    group.finish();
}

// ============================================================================
// BoundedQueue Benchmarks
// ============================================================================

fn bench_bounded_queue_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounded_queue_push");

    for capacity in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let queue = BoundedQueue::new(capacity);
            let mut counter = 0u64;
            b.iter(|| {
                // Pop occasionally to avoid filling up
                if counter.is_multiple_of(capacity as u64) {
                    let _ = queue.try_pop();
                }
                let _ = queue.try_push(black_box(counter));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_bounded_queue_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounded_queue_pop");

    for capacity in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let queue = BoundedQueue::new(capacity);
            // Pre-fill queue
            for i in 0..capacity {
                let _ = queue.try_push(i as u64);
            }
            b.iter(|| {
                if let Some(value) = queue.try_pop() {
                    black_box(value);
                    // Refill to maintain queue size
                    let _ = queue.try_push(value + 1);
                }
            });
        });
    }

    group.finish();
}

fn bench_bounded_queue_try_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounded_queue_try_push");

    for capacity in [10, 100] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let queue = BoundedQueue::new(capacity);
            let mut counter = 0u64;
            b.iter(|| {
                let result = queue.try_push(black_box(counter));
                let _ = black_box(result);
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_bounded_queue_push_timeout(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounded_queue_push_timeout");

    group.throughput(Throughput::Elements(1));
    group.bench_function("timeout_immediate", |b| {
        let queue = BoundedQueue::new(10);
        // Fill queue
        for i in 0..10 {
            let _ = queue.try_push(i);
        }
        let mut counter = 10u64;
        b.iter(|| {
            let result = queue.push_timeout(black_box(counter), Duration::from_millis(1));
            let _ = black_box(result);
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_bounded_queue_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("bounded_queue_concurrent");

    for thread_count in [2, 4] {
        group.throughput(Throughput::Elements(100));
        group.bench_with_input(
            BenchmarkId::from_parameter(thread_count),
            &thread_count,
            |b, &thread_count| {
                let queue = Arc::new(BoundedQueue::new(100));

                b.iter(|| {
                    let mut handles = vec![];
                    for _ in 0..thread_count {
                        let q = Arc::clone(&queue);
                        let handle = std::thread::spawn(move || {
                            for i in 0..50 {
                                let _ = q.try_push(i);
                            }
                        });
                        handles.push(handle);
                    }
                    for handle in handles {
                        handle.join().unwrap();
                    }
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// LruCache Benchmarks
// ============================================================================

fn bench_lru_cache_put(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_cache_put");

    for capacity in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
            let mut counter = 0u64;
            b.iter(|| {
                cache.put(black_box(counter), black_box(format!("value_{}", counter)));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_lru_cache_get_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_cache_get_hit");

    for capacity in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
            // Pre-populate
            for i in 0..capacity {
                cache.put(i as u64, format!("value_{}", i));
            }
            let mut counter = 0u64;
            b.iter(|| {
                let key = counter % (capacity as u64);
                let result = black_box(cache.get(&black_box(key)));
                black_box(result);
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_lru_cache_get_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_cache_get_miss");

    for capacity in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
            // Pre-populate with keys 0..capacity
            for i in 0..capacity {
                cache.put(i as u64, format!("value_{}", i));
            }
            let mut counter = 0u64;
            b.iter(|| {
                // Query keys that don't exist
                let key = (capacity as u64) + counter;
                let result = black_box(cache.get(&black_box(key)));
                black_box(result);
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_lru_cache_peek(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_cache_peek");

    for capacity in [100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
            for i in 0..capacity {
                cache.put(i as u64, format!("value_{}", i));
            }
            let mut counter = 0u64;
            b.iter(|| {
                let key = counter % (capacity as u64);
                let result = black_box(cache.peek(&black_box(key)));
                black_box(result);
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_lru_cache_eviction(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_cache_eviction");

    for capacity in [100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut cache = LruCache::new(NonZeroUsize::new(capacity).unwrap());
            // Pre-fill to capacity
            for i in 0..capacity {
                cache.put(i as u64, format!("value_{}", i));
            }
            let mut counter = capacity as u64;
            b.iter(|| {
                // This will trigger eviction
                cache.put(black_box(counter), black_box(format!("value_{}", counter)));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_lru_cache_resize(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_cache_resize");

    group.bench_function("resize_grow", |b| {
        b.iter_batched(
            || {
                let mut cache = LruCache::new(NonZeroUsize::new(100).unwrap());
                for i in 0..100 {
                    cache.put(i, format!("value_{}", i));
                }
                cache
            },
            |mut cache| {
                cache.resize(NonZeroUsize::new(1000).unwrap());
                black_box(cache);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("resize_shrink", |b| {
        b.iter_batched(
            || {
                let mut cache = LruCache::new(NonZeroUsize::new(1000).unwrap());
                for i in 0..1000 {
                    cache.put(i, format!("value_{}", i));
                }
                cache
            },
            |mut cache| {
                cache.resize(NonZeroUsize::new(100).unwrap());
                black_box(cache);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// PriorityQueue (MinHeap/MaxHeap) Benchmarks
// ============================================================================

fn bench_priority_queue_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue_push");

    group.throughput(Throughput::Elements(1));
    group.bench_function("min_heap", |b| {
        let mut heap = MinHeap::new();
        let mut counter = 0u64;
        b.iter(|| {
            heap.push(black_box(counter));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("max_heap", |b| {
        let mut heap = MaxHeap::new();
        let mut counter = 0u64;
        b.iter(|| {
            heap.push(black_box(counter));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_priority_queue_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue_pop");

    group.throughput(Throughput::Elements(1));
    group.bench_function("min_heap", |b| {
        b.iter_batched(
            || {
                let mut heap = MinHeap::new();
                for i in 0..1000 {
                    heap.push(i);
                }
                heap
            },
            |mut heap| {
                while let Some(val) = heap.pop() {
                    black_box(val);
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.bench_function("max_heap", |b| {
        b.iter_batched(
            || {
                let mut heap = MaxHeap::new();
                for i in 0..1000 {
                    heap.push(i);
                }
                heap
            },
            |mut heap| {
                while let Some(val) = heap.pop() {
                    black_box(val);
                }
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_priority_queue_peek(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue_peek");

    group.throughput(Throughput::Elements(1));
    group.bench_function("min_heap", |b| {
        let mut heap = MinHeap::new();
        for i in 0..1000 {
            heap.push(i);
        }
        b.iter(|| {
            let result = black_box(heap.peek());
            black_box(result);
        });
    });

    group.bench_function("max_heap", |b| {
        let mut heap = MaxHeap::new();
        for i in 0..1000 {
            heap.push(i);
        }
        b.iter(|| {
            let result = black_box(heap.peek());
            black_box(result);
        });
    });

    group.finish();
}

fn bench_priority_queue_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue_mixed");

    group.throughput(Throughput::Elements(2)); // push + pop
    group.bench_function("min_heap", |b| {
        let mut heap = MinHeap::new();
        // Pre-populate
        for i in 0..500 {
            heap.push(i);
        }
        let mut counter = 500u64;
        b.iter(|| {
            heap.push(black_box(counter));
            if let Some(val) = heap.pop() {
                black_box(val);
            }
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("max_heap", |b| {
        let mut heap = MaxHeap::new();
        // Pre-populate
        for i in 0..500 {
            heap.push(i);
        }
        let mut counter = 500u64;
        b.iter(|| {
            heap.push(black_box(counter));
            if let Some(val) = heap.pop() {
                black_box(val);
            }
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_priority_queue_trait(c: &mut Criterion) {
    let mut group = c.benchmark_group("priority_queue_trait");

    // Benchmark using the PriorityQueue trait interface
    group.throughput(Throughput::Elements(1));
    group.bench_function("trait_push_min_heap", |b| {
        let mut heap: Box<dyn PriorityQueue<u64>> = Box::new(MinHeap::new());
        let mut counter = 0u64;
        b.iter(|| {
            heap.push(black_box(counter));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("trait_push_max_heap", |b| {
        let mut heap: Box<dyn PriorityQueue<u64>> = Box::new(MaxHeap::new());
        let mut counter = 0u64;
        b.iter(|| {
            heap.push(black_box(counter));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("trait_operations_min_heap", |b| {
        let mut heap: Box<dyn PriorityQueue<u64>> = Box::new(MinHeap::new());
        // Pre-populate
        for i in 0..100 {
            heap.push(i);
        }
        let mut counter = 100u64;
        b.iter(|| {
            heap.push(black_box(counter));
            let _ = heap.peek();
            if let Some(val) = heap.pop() {
                black_box(val);
            }
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

// ============================================================================
// RingBuffer Benchmarks
// ============================================================================

fn bench_ring_buffer_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_push");

    for capacity in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut buffer = RingBuffer::new(capacity);
            let mut counter = 0u64;
            b.iter(|| {
                buffer.push(black_box(counter));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_ring_buffer_pop(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_pop");

    for capacity in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut buffer = RingBuffer::new(capacity);
            // Pre-fill
            for i in 0..capacity {
                buffer.push(i as u64);
            }
            b.iter(|| {
                if let Some(val) = buffer.pop() {
                    black_box(val);
                    // Refill to maintain size
                    buffer.push(val + 1);
                }
            });
        });
    }

    group.finish();
}

fn bench_ring_buffer_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_get");

    for capacity in [10, 100, 1000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut buffer = RingBuffer::new(capacity);
            for i in 0..capacity {
                buffer.push(i as u64);
            }
            let mut counter = 0;
            b.iter(|| {
                let idx = counter % capacity;
                let result = black_box(buffer.get(black_box(idx)));
                black_box(result);
                counter += 1;
            });
        });
    }

    group.finish();
}

fn bench_ring_buffer_wraparound(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_wraparound");

    for capacity in [10, 100] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut buffer = RingBuffer::new(capacity);
            let mut counter = 0u64;
            b.iter(|| {
                // This will cause wraparound when full
                buffer.push(black_box(counter));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_ring_buffer_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("ring_buffer_iter");

    for capacity in [10, 100, 1000] {
        group.throughput(Throughput::Elements(capacity as u64));
        group.bench_with_input(BenchmarkId::from_parameter(capacity), &capacity, |b, &capacity| {
            let mut buffer = RingBuffer::new(capacity);
            for i in 0..capacity {
                buffer.push(i as u64);
            }
            b.iter(|| {
                for val in buffer.iter() {
                    black_box(val);
                }
            });
        });
    }

    group.finish();
}

// ============================================================================
// Trie Benchmarks
// ============================================================================

fn bench_trie_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_insert");

    let words = generate_words(1000);

    group.throughput(Throughput::Elements(1));
    group.bench_function("insert", |b| {
        let mut trie = Trie::new();
        let mut idx = 0;
        b.iter(|| {
            let word = &words[idx % words.len()];
            trie.insert(black_box(word));
            idx += 1;
        });
    });

    group.finish();
}

fn bench_trie_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_contains");

    let words = generate_words(1000);
    let mut trie = Trie::new();
    for word in &words {
        trie.insert(word);
    }

    group.throughput(Throughput::Elements(1));
    group.bench_function("contains_hit", |b| {
        let mut idx = 0;
        b.iter(|| {
            let word = &words[idx % words.len()];
            let result = black_box(trie.contains(black_box(word)));
            black_box(result);
            idx += 1;
        });
    });

    group.bench_function("contains_miss", |b| {
        b.iter(|| {
            let result = black_box(trie.contains(black_box("nonexistent_word_xyz123")));
            black_box(result);
        });
    });

    group.finish();
}

fn bench_trie_starts_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_starts_with");

    let words = generate_words(1000);
    let mut trie = Trie::new();
    for word in &words {
        trie.insert(word);
    }

    let prefixes: Vec<String> = words.iter().map(|w| w.chars().take(3).collect()).collect();

    group.throughput(Throughput::Elements(1));
    group.bench_function("starts_with", |b| {
        let mut idx = 0;
        b.iter(|| {
            let prefix = &prefixes[idx % prefixes.len()];
            let result = black_box(trie.starts_with(black_box(prefix)));
            black_box(result);
            idx += 1;
        });
    });

    group.finish();
}

fn bench_trie_find_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_find_prefix");

    let words = generate_words(1000);
    let mut trie = Trie::new();
    for word in &words {
        trie.insert(word);
    }

    let prefixes: Vec<String> = words.iter().map(|w| w.chars().take(3).collect()).collect();

    group.throughput(Throughput::Elements(1));
    group.bench_function("find_prefix", |b| {
        let mut idx = 0;
        b.iter(|| {
            let prefix = &prefixes[idx % prefixes.len()];
            let results = black_box(trie.find_prefix(black_box(prefix)));
            black_box(results);
            idx += 1;
        });
    });

    group.finish();
}

fn bench_trie_remove(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_remove");

    let words = generate_words(1000);

    group.throughput(Throughput::Elements(1));
    group.bench_function("remove", |b| {
        b.iter_batched(
            || {
                let mut trie = Trie::new();
                for word in &words {
                    trie.insert(word);
                }
                (trie, 0)
            },
            |(mut trie, mut idx)| {
                let word = &words[idx % words.len()];
                let result = black_box(trie.remove(black_box(word)));
                black_box(result);
                idx += 1;
                black_box((trie, idx));
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

fn bench_trie_iter_prefix(c: &mut Criterion) {
    let mut group = c.benchmark_group("trie_iter_prefix");

    let words = generate_words(1000);
    let mut trie = Trie::new();
    for word in &words {
        trie.insert(word);
    }

    let prefixes: Vec<String> = words.iter().map(|w| w.chars().take(3).collect()).collect();

    group.throughput(Throughput::Elements(1));
    group.bench_function("iter_prefix", |b| {
        let mut idx = 0;
        b.iter(|| {
            let prefix = &prefixes[idx % prefixes.len()];
            for word in trie.iter_prefix(black_box(prefix)) {
                black_box(word);
            }
            idx += 1;
        });
    });

    group.finish();
}

// ============================================================================
// Real-World Scenario Benchmarks
// ============================================================================

fn bench_url_deduplication_bloom_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_url_deduplication");

    // Simulates web crawler URL deduplication
    group.throughput(Throughput::Elements(1));
    group.bench_function("bloom_filter", |b| {
        let mut filter = BloomFilter::new(100_000, 0.001).unwrap();
        let mut counter = 0u64;
        b.iter(|| {
            let url = format!("https://example.com/page{}", counter);
            if !filter.contains(&url) {
                filter.insert(&url);
            }
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_task_queue_bounded_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_task_queue");

    // Simulates producer-consumer task queue
    group.throughput(Throughput::Elements(100));
    group.bench_function("bounded_queue", |b| {
        let queue = Arc::new(BoundedQueue::new(100));

        b.iter(|| {
            let producer_queue = Arc::clone(&queue);
            let consumer_queue = Arc::clone(&queue);

            let producer = std::thread::spawn(move || {
                for i in 0..100 {
                    let _ = producer_queue.try_push(i);
                }
            });

            let consumer = std::thread::spawn(move || {
                for _ in 0..100 {
                    while consumer_queue.try_pop().is_none() {
                        std::hint::spin_loop();
                    }
                }
            });

            producer.join().unwrap();
            consumer.join().unwrap();
        });
    });

    group.finish();
}

fn bench_page_cache_lru(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_page_cache");

    // Simulates browser page cache with 90% hit rate
    group.throughput(Throughput::Elements(1));
    group.bench_function("lru_cache", |b| {
        let mut cache = LruCache::new(NonZeroUsize::new(100).unwrap());
        // Pre-populate with popular pages
        for i in 0..90 {
            cache.put(format!("page_{}", i), vec![0u8; 1000]);
        }
        let mut counter = 0u64;
        b.iter(|| {
            let is_hit = (counter % 100) < 90;
            let key = if is_hit {
                format!("page_{}", counter % 90)
            } else {
                format!("page_{}", 90 + counter)
            };
            let _ = cache.get(&black_box(key));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_event_stream_priority_queue(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_event_stream");

    // Simulates event processing with priority
    group.throughput(Throughput::Elements(2)); // push + pop
    group.bench_function("min_heap", |b| {
        let mut heap = MinHeap::new();
        let mut counter = 0u64;
        b.iter(|| {
            // Add event with timestamp
            heap.push(black_box(counter));
            // Process oldest event
            if let Some(timestamp) = heap.pop() {
                black_box(timestamp);
            }
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_sensor_data_ring_buffer(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_sensor_data");

    // Simulates sensor data buffering
    group.throughput(Throughput::Elements(1));
    group.bench_function("ring_buffer", |b| {
        let mut buffer = RingBuffer::new(100);
        let mut reading = 0u64;
        b.iter(|| {
            buffer.push(black_box(reading));
            reading = reading.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_autocomplete_trie(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_autocomplete");

    // Simulates autocomplete with dictionary
    let dictionary = generate_words(10_000);
    let mut trie = Trie::new();
    for word in &dictionary {
        trie.insert(word);
    }

    let queries: Vec<String> = dictionary.iter().map(|w| w.chars().take(3).collect()).collect();

    group.throughput(Throughput::Elements(1));
    group.bench_function("trie", |b| {
        let mut idx = 0;
        b.iter(|| {
            let query = &queries[idx % queries.len()];
            let suggestions = black_box(trie.find_prefix(black_box(query)));
            black_box(suggestions);
            idx += 1;
        });
    });

    group.finish();
}

// ============================================================================
// Helper Functions
// ============================================================================

fn generate_words(count: usize) -> Vec<String> {
    let prefixes = ["app", "test", "data", "user", "sys", "config", "cache", "temp"];
    let suffixes = ["tion", "ing", "ed", "er", "ly", "ness", "ment", "ful"];

    (0..count)
        .map(|i| {
            let prefix = prefixes[i % prefixes.len()];
            let suffix = suffixes[(i / prefixes.len()) % suffixes.len()];
            format!("{}{}{}", prefix, i, suffix)
        })
        .collect()
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    bloom_filter,
    bench_bloom_filter_insert,
    bench_bloom_filter_contains,
    bench_bloom_filter_clear,
    bench_bloom_filter_false_positive_rate,
);

criterion_group!(
    bounded_queue,
    bench_bounded_queue_push,
    bench_bounded_queue_pop,
    bench_bounded_queue_try_push,
    bench_bounded_queue_push_timeout,
    bench_bounded_queue_concurrent,
);

criterion_group!(
    lru_cache,
    bench_lru_cache_put,
    bench_lru_cache_get_hit,
    bench_lru_cache_get_miss,
    bench_lru_cache_peek,
    bench_lru_cache_eviction,
    bench_lru_cache_resize,
);

criterion_group!(
    priority_queue,
    bench_priority_queue_push,
    bench_priority_queue_pop,
    bench_priority_queue_peek,
    bench_priority_queue_mixed,
    bench_priority_queue_trait,
);

criterion_group!(
    ring_buffer,
    bench_ring_buffer_push,
    bench_ring_buffer_pop,
    bench_ring_buffer_get,
    bench_ring_buffer_wraparound,
    bench_ring_buffer_iter,
);

criterion_group!(
    trie,
    bench_trie_insert,
    bench_trie_contains,
    bench_trie_starts_with,
    bench_trie_find_prefix,
    bench_trie_remove,
    bench_trie_iter_prefix,
);

criterion_group!(
    real_world,
    bench_url_deduplication_bloom_filter,
    bench_task_queue_bounded_queue,
    bench_page_cache_lru,
    bench_event_stream_priority_queue,
    bench_sensor_data_ring_buffer,
    bench_autocomplete_trie,
);

criterion_main!(
    bloom_filter,
    bounded_queue,
    lru_cache,
    priority_queue,
    ring_buffer,
    trie,
    real_world,
);
