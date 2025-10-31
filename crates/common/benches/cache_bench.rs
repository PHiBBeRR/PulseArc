//! Comprehensive cache benchmarks
//!
//! Benchmarks for cache operations including insert, get, eviction policies,
//! TTL expiration, and concurrent access patterns.
//!
//! Run with: `cargo bench --bench cache_bench -p pulsearc-common --features
//! runtime`

use std::sync::Arc;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::cache::{AsyncCache, Cache, CacheConfig, EvictionPolicy};

type JsonVecCache = Cache<String, Arc<Vec<serde_json::Value>>>;
type ByteVecCache = Cache<String, Arc<Vec<u8>>>;
type U64VecCache = Cache<String, Arc<Vec<u64>>>;

// ============================================================================
// Basic Operations Benchmarks
// ============================================================================

fn bench_cache_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_insert");

    for size in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("lru", size), &size, |b, &size| {
            let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(size));
            let mut counter = 0u64;
            b.iter(|| {
                cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_cache_get_hit(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_get_hit");

    for size in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("lru", size), &size, |b, &size| {
            let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(size));
            // Pre-populate cache
            for i in 0..size as u64 {
                cache.insert(i, format!("value_{}", i));
            }
            let mut counter = 0u64;
            b.iter(|| {
                let key = counter % (size as u64);
                let _ = black_box(cache.get(&black_box(key)));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_cache_get_miss(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_get_miss");

    for size in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("lru", size), &size, |b, &size| {
            let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(size));
            // Pre-populate with keys 0..size
            for i in 0..size as u64 {
                cache.insert(i, format!("value_{}", i));
            }
            let mut counter = 0u64;
            b.iter(|| {
                // Query keys that don't exist (size + counter)
                let key = (size as u64) + counter;
                let _ = black_box(cache.get(&black_box(key)));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_cache_mixed_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_mixed_operations");

    for size in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(3)); // insert + get + remove per iteration
        group.bench_with_input(BenchmarkId::new("lru", size), &size, |b, &size| {
            let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(size));
            let mut counter = 0u64;
            b.iter(|| {
                let key = counter % (size as u64);
                cache.insert(black_box(key), black_box(format!("value_{}", counter)));
                let _ = black_box(cache.get(&black_box(key)));
                cache.remove(&black_box(key));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Eviction Policy Benchmarks
// ============================================================================

fn bench_eviction_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("eviction_policies");
    let size = 1000;

    let policies = [
        ("lru", EvictionPolicy::LRU),
        ("lfu", EvictionPolicy::LFU),
        ("fifo", EvictionPolicy::FIFO),
        ("random", EvictionPolicy::Random),
        ("none", EvictionPolicy::None),
    ];

    for (name, policy) in policies {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::new("insert", name), &policy, |b, &policy| {
            let config = CacheConfig::builder().max_size(size).eviction_policy(policy).build();
            let cache: Cache<u64, String> = Cache::new(config);

            let mut counter = 0u64;
            b.iter(|| {
                cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
                counter = counter.wrapping_add(1);
            });
        });

        group.bench_with_input(BenchmarkId::new("get", name), &policy, |b, &policy| {
            let config = CacheConfig::builder().max_size(size).eviction_policy(policy).build();
            let cache: Cache<u64, String> = Cache::new(config);

            // Pre-populate
            for i in 0..size as u64 {
                cache.insert(i, format!("value_{}", i));
            }

            let mut counter = 0u64;
            b.iter(|| {
                let key = counter % (size as u64);
                let _ = black_box(cache.get(&black_box(key)));
                counter = counter.wrapping_add(1);
            });
        });
    }

    group.finish();
}

fn bench_lru_eviction_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("lru_eviction_overhead");

    for size in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("insert_beyond_capacity", size),
            &size,
            |b, &size| {
                let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(size));

                // Pre-fill to capacity
                for i in 0..size as u64 {
                    cache.insert(i, format!("value_{}", i));
                }

                let mut counter = size as u64;
                b.iter(|| {
                    // This will trigger eviction
                    cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
                    counter = counter.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Hit/Miss Ratio Benchmarks
// ============================================================================

fn bench_cache_hit_ratios(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_hit_ratios");
    let size = 1000;

    let hit_ratios = [0.0, 0.25, 0.5, 0.75, 0.95, 1.0];

    for &hit_ratio in &hit_ratios {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("lru", format!("{}%", (hit_ratio * 100.0) as u32)),
            &hit_ratio,
            |b, &hit_ratio| {
                let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(size));

                // Pre-populate with keys 0..size
                for i in 0..size as u64 {
                    cache.insert(i, format!("value_{}", i));
                }

                let mut counter = 0u64;
                b.iter(|| {
                    // Determine if this access should be a hit or miss
                    let is_hit = (counter % 100) < (hit_ratio * 100.0) as u64;
                    let key = if is_hit {
                        counter % (size as u64) // Access existing key
                    } else {
                        (size as u64) + counter // Access non-existent key
                    };
                    let _ = black_box(cache.get(&black_box(key)));
                    counter = counter.wrapping_add(1);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// TTL Benchmarks
// ============================================================================

fn bench_cache_ttl_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_ttl_operations");

    group.throughput(Throughput::Elements(1));
    group.bench_function("insert_with_ttl", |b| {
        let cache: Cache<u64, String> = Cache::new(CacheConfig::ttl(Duration::from_secs(3600)));

        let mut counter = 0u64;
        b.iter(|| {
            cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("get_with_ttl_check", |b| {
        let cache: Cache<u64, String> = Cache::new(CacheConfig::ttl(Duration::from_secs(3600)));

        // Pre-populate
        for i in 0..1000u64 {
            cache.insert(i, format!("value_{}", i));
        }

        let mut counter = 0u64;
        b.iter(|| {
            let key = counter % 1000;
            let _ = black_box(cache.get(&black_box(key)));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("ttl_lru_combined", |b| {
        let cache: Cache<u64, String> =
            Cache::new(CacheConfig::ttl_lru(Duration::from_secs(3600), 1000));

        let mut counter = 0u64;
        b.iter(|| {
            cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
            let key = counter % 500;
            let _ = black_box(cache.get(&black_box(key)));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

// ============================================================================
// Metrics Tracking Benchmarks
// ============================================================================

fn bench_cache_with_metrics(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_with_metrics");

    group.throughput(Throughput::Elements(1));
    group.bench_function("insert_with_metrics", |b| {
        let config = CacheConfig::builder().max_size(1000).track_metrics(true).build();
        let cache: Cache<u64, String> = Cache::new(config);

        let mut counter = 0u64;
        b.iter(|| {
            cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("insert_without_metrics", |b| {
        let config = CacheConfig::builder().max_size(1000).track_metrics(false).build();
        let cache: Cache<u64, String> = Cache::new(config);

        let mut counter = 0u64;
        b.iter(|| {
            cache.insert(black_box(counter), black_box(format!("value_{}", counter)));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("get_with_metrics", |b| {
        let config = CacheConfig::builder().max_size(1000).track_metrics(true).build();
        let cache: Cache<u64, String> = Cache::new(config);

        // Pre-populate
        for i in 0..1000u64 {
            cache.insert(i, format!("value_{}", i));
        }

        let mut counter = 0u64;
        b.iter(|| {
            let key = counter % 1000;
            let _ = black_box(cache.get(&black_box(key)));
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("stats_collection", |b| {
        let config = CacheConfig::builder().max_size(1000).track_metrics(true).build();
        let cache: Cache<u64, String> = Cache::new(config);

        // Pre-populate and generate some stats
        for i in 0..1000u64 {
            cache.insert(i, format!("value_{}", i));
        }
        for i in 0..500u64 {
            let _ = cache.get(&i);
        }

        b.iter(|| {
            let stats = black_box(cache.stats());
            black_box(stats);
        });
    });

    group.finish();
}

// ============================================================================
// Concurrent Access Benchmarks
// ============================================================================

fn bench_cache_concurrent_reads(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_concurrent_reads");

    for thread_count in [2, 4, 8] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            &thread_count,
            |b, &thread_count| {
                let cache = Arc::new(Cache::new(CacheConfig::lru(1000)));

                // Pre-populate
                for i in 0..1000u64 {
                    cache.insert(i, format!("value_{}", i));
                }

                b.iter(|| {
                    let mut handles = vec![];
                    for _ in 0..thread_count {
                        let cache_clone = Arc::clone(&cache);
                        let handle = std::thread::spawn(move || {
                            for i in 0..100u64 {
                                let _ = black_box(cache_clone.get(&black_box(i)));
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

fn bench_cache_concurrent_mixed(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_concurrent_mixed");

    for thread_count in [2, 4, 8] {
        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            &thread_count,
            |b, &thread_count| {
                let cache = Arc::new(Cache::new(CacheConfig::lru(1000)));

                // Pre-populate
                for i in 0..1000u64 {
                    cache.insert(i, format!("value_{}", i));
                }

                b.iter(|| {
                    let mut handles = vec![];
                    for t in 0..thread_count {
                        let cache_clone = Arc::clone(&cache);
                        let handle = std::thread::spawn(move || {
                            for i in 0..100u64 {
                                let key = (t as u64 * 100) + i;
                                cache_clone.insert(key, format!("value_{}", key));
                                let _ = black_box(cache_clone.get(&black_box(key)));
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
// Async Cache Benchmarks
// ============================================================================

fn bench_async_cache_basic(c: &mut Criterion) {
    let mut group = c.benchmark_group("async_cache_basic");

    let rt = tokio::runtime::Runtime::new().unwrap();

    group.throughput(Throughput::Elements(1));
    group.bench_function("async_insert", |b| {
        let cache = Arc::new(AsyncCache::new(CacheConfig::lru(1000)));
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        b.to_async(&rt).iter(|| {
            let cache = Arc::clone(&cache);
            let counter = Arc::clone(&counter);
            async move {
                let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                cache.insert(black_box(count), black_box(format!("value_{}", count))).await;
            }
        });
    });

    group.bench_function("async_get_hit", |b| {
        let cache = Arc::new(AsyncCache::new(CacheConfig::lru(1000)));

        // Pre-populate
        rt.block_on(async {
            for i in 0..1000u64 {
                cache.insert(i, format!("value_{}", i)).await;
            }
        });

        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));
        b.to_async(&rt).iter(|| {
            let cache = Arc::clone(&cache);
            let counter = Arc::clone(&counter);
            async move {
                let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let key = count % 1000;
                let _ = black_box(cache.get(&black_box(key)).await);
            }
        });
    });

    group.bench_function("async_get_or_insert", |b| {
        let cache = Arc::new(AsyncCache::new(CacheConfig::lru(1000)));
        let counter = Arc::new(std::sync::atomic::AtomicU64::new(0));

        b.to_async(&rt).iter(|| {
            let cache = Arc::clone(&cache);
            let counter = Arc::clone(&counter);
            async move {
                let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let value =
                    cache.get_or_insert_with(black_box(count), || format!("value_{}", count)).await;
                black_box(value);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Memory Overhead Benchmarks
// ============================================================================

fn bench_cache_memory_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_memory_overhead");

    for size in [100, 1000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("populate_cache", size), &size, |b, &size| {
            b.iter(|| {
                let cache: Cache<u64, Vec<u8>> = Cache::new(CacheConfig::lru(size));
                for i in 0..size as u64 {
                    // Each value is 100 bytes
                    cache.insert(i, vec![0u8; 100]);
                }
                black_box(cache);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Get or Insert Benchmarks
// ============================================================================

fn bench_get_or_insert_with(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_or_insert_with");

    group.throughput(Throughput::Elements(1));
    group.bench_function("always_compute", |b| {
        let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(1000));
        let mut counter = 0u64;
        b.iter(|| {
            let value =
                cache.get_or_insert_with(black_box(counter), || format!("value_{}", counter));
            black_box(value);
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("always_cached", |b| {
        let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(1000));
        // Pre-populate
        for i in 0..1000u64 {
            cache.insert(i, format!("value_{}", i));
        }
        let mut counter = 0u64;
        b.iter(|| {
            let key = counter % 1000;
            let value = cache.get_or_insert_with(black_box(key), || format!("value_{}", key));
            black_box(value);
            counter = counter.wrapping_add(1);
        });
    });

    group.bench_function("mixed_50_50", |b| {
        let cache: Cache<u64, String> = Cache::new(CacheConfig::lru(1000));
        // Pre-populate half the keys
        for i in 0..500u64 {
            cache.insert(i, format!("value_{}", i));
        }
        let mut counter = 0u64;
        b.iter(|| {
            let key = counter % 1000;
            let value = cache.get_or_insert_with(black_box(key), || format!("value_{}", key));
            black_box(value);
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    basic_operations,
    bench_cache_insert,
    bench_cache_get_hit,
    bench_cache_get_miss,
    bench_cache_mixed_operations,
);

criterion_group!(eviction, bench_eviction_policies, bench_lru_eviction_overhead,);

criterion_group!(hit_ratios, bench_cache_hit_ratios,);

criterion_group!(ttl, bench_cache_ttl_operations,);

criterion_group!(metrics, bench_cache_with_metrics,);

criterion_group!(concurrent, bench_cache_concurrent_reads, bench_cache_concurrent_mixed,);

criterion_group!(async_cache, bench_async_cache_basic,);

criterion_group!(memory, bench_cache_memory_overhead,);

criterion_group!(get_or_insert, bench_get_or_insert_with,);

// ============================================================================
// Real-World Scenario Benchmarks
// ============================================================================

fn bench_api_gateway_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_api_gateway");

    // Simulates API gateway with 95% hit rate, 1000 endpoint cache
    group.throughput(Throughput::Elements(1));
    group.bench_function("endpoint_routing", |b| {
        let cache: ByteVecCache = Cache::new(CacheConfig::ttl_lru(Duration::from_secs(300), 1000));

        // Pre-populate popular endpoints
        for i in 0..950 {
            cache.insert(
                format!("/api/endpoint{}", i),
                Arc::new(format!("response{}", i).into_bytes()),
            );
        }

        let mut counter = 0u64;
        b.iter(|| {
            let is_hit = (counter % 100) < 95;
            let key = if is_hit {
                format!("/api/endpoint{}", counter % 950)
            } else {
                format!("/api/new{}", counter)
            };
            let _ = black_box(cache.get(&black_box(key)));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_session_store_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_session_store");

    // Simulates user session storage with 30min TTL
    group.throughput(Throughput::Elements(1));
    group.bench_function("session_access", |b| {
        let cache: Cache<String, Arc<serde_json::Value>> =
            Cache::new(CacheConfig::ttl_lru(Duration::from_secs(1800), 10_000));

        // Pre-populate with active sessions
        for i in 0..5000 {
            let session = Arc::new(serde_json::json!({
                "user_id": i,
                "authenticated_at": "2024-01-01T00:00:00Z",
                "permissions": ["read", "write"]
            }));
            cache.insert(format!("session_{}", i), session);
        }

        let mut counter = 0u64;
        b.iter(|| {
            let key = format!("session_{}", counter % 5000);
            let _ = black_box(cache.get(&black_box(key)));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_database_query_cache_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_db_query_cache");

    // Simulates database query result caching with 60s TTL
    group.throughput(Throughput::Elements(1));
    group.bench_function("query_caching", |b| {
        let cache: JsonVecCache = Cache::new(CacheConfig::ttl_lru(Duration::from_secs(60), 500));

        // Pre-populate with query results
        for i in 0..250 {
            let results =
                Arc::new(vec![serde_json::json!({"id": i, "name": format!("user{}", i)})]);
            cache.insert(format!("SELECT * FROM users WHERE id = {}", i), results);
        }

        let mut counter = 0u64;
        b.iter(|| {
            // 80% hit rate (realistic for query caching)
            let is_hit = (counter % 10) < 8;
            let query = if is_hit {
                format!("SELECT * FROM users WHERE id = {}", counter % 250)
            } else {
                format!("SELECT * FROM users WHERE id = {}", 500 + counter)
            };

            let _ = cache.get_or_insert_with(black_box(query), || {
                // Simulate query execution cost
                Arc::new(vec![serde_json::json!({"id": counter})])
            });
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_cdn_asset_cache_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_cdn_cache");

    // Simulates CDN edge cache for static assets
    group.throughput(Throughput::Elements(1));
    group.bench_function("asset_serving", |b| {
        let cache: ByteVecCache = Cache::new(CacheConfig::lru(1000));

        // Pre-populate with popular assets (large values)
        for i in 0..800 {
            cache.insert(
                format!("/static/asset{}.jpg", i),
                Arc::new(vec![0u8; 10_000]), // 10KB assets
            );
        }

        let mut counter = 0u64;
        b.iter(|| {
            // 98% hit rate (typical for CDN)
            let is_hit = (counter % 100) < 98;
            let key = if is_hit {
                format!("/static/asset{}.jpg", counter % 800)
            } else {
                format!("/static/new_asset{}.jpg", counter)
            };
            let _ = black_box(cache.get(&black_box(key)));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_rate_limiter_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_rate_limiter");

    // Simulates rate limiting with short TTL windows
    group.throughput(Throughput::Elements(1));
    group.bench_function("rate_limit_check", |b| {
        let cache: U64VecCache = Cache::new(CacheConfig::ttl(Duration::from_secs(60)));

        let mut counter = 0u64;
        b.iter(|| {
            let ip = format!("192.168.1.{}", counter % 255);
            let timestamps = cache.get_or_insert_with(black_box(ip), || Arc::new(vec![counter]));
            black_box(timestamps);
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

fn bench_microservice_config_cache_scenario(c: &mut Criterion) {
    let mut group = c.benchmark_group("real_world_config_cache");

    // Simulates microservice configuration caching
    group.throughput(Throughput::Elements(1));
    group.bench_function("config_lookup", |b| {
        let cache: Cache<String, Arc<serde_json::Value>> =
            Cache::new(CacheConfig::ttl_lru(Duration::from_secs(300), 100));

        // Pre-populate with config keys
        let config_keys = vec![
            "database.connection_string",
            "feature_flags.new_ui",
            "rate_limits.api",
            "cache.ttl",
        ];

        for key in &config_keys {
            cache.insert((*key).to_string(), Arc::new(serde_json::json!({"enabled": true})));
        }

        let mut counter = 0u64;
        b.iter(|| {
            let key = config_keys[(counter as usize) % config_keys.len()];
            let _ = black_box(cache.get(&black_box(key.to_string())));
            counter = counter.wrapping_add(1);
        });
    });

    group.finish();
}

criterion_group!(
    real_world,
    bench_api_gateway_scenario,
    bench_session_store_scenario,
    bench_database_query_cache_scenario,
    bench_cdn_asset_cache_scenario,
    bench_rate_limiter_scenario,
    bench_microservice_config_cache_scenario,
);

criterion_main!(
    basic_operations,
    eviction,
    hit_ratios,
    ttl,
    metrics,
    concurrent,
    async_cache,
    memory,
    get_or_insert,
    real_world,
);
