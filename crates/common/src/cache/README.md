# PulseArc Cache Module

Robust synchronous and asynchronous in-memory caches with flexible eviction, TTL, and observability hooks. This module lives under `pulsearc-common::cache` and is part of the workspace-wide runtime utilities.

---

## Runtime Feature Gate

The cache module is compiled when the `runtime` feature is enabled on `pulsearc-common`. The feature pulls in the `foundation` tier (collections, rand, serde, etc.) plus the async stack.

```bash
cargo add pulsearc-common --features runtime

# for logging helpers in `utils`
cargo add pulsearc-common --features "runtime,observability"
```

When working inside the workspace, use:

```bash
cargo test -p pulsearc-common --features runtime cache::
cargo bench -p pulsearc-common --bench cache_bench --features runtime
```

---

## Quick Start (Sync)

```rust
use pulsearc_common::cache::{Cache, CacheConfig};

let cache: Cache<String, i32> = Cache::new(CacheConfig::lru(128));
cache.insert("answer".into(), 42);
assert_eq!(cache.get(&"answer".into()), Some(42));
```

## Quick Start (Async)

```rust
use pulsearc_common::cache::{AsyncCache, CacheConfig};

#[tokio::main]
async fn main() {
    let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(128));

    cache.insert("answer".into(), 42).await;
    assert_eq!(cache.get(&"answer".into()).await, Some(42));
}
```

---

## Capabilities

- **Configurable eviction**: LRU, LFU, FIFO, random, or no eviction (manual only).
- **TTL-aware**: Automatic expiry backed by `resilience::SystemClock`, with deterministic tests via `resilience::MockClock`.
- **Metrics-ready**: Opt-in counters for hits, misses, evictions, expirations, and inserts (`CacheStats`).
- **Sync + async APIs**: `Cache` relies on `std::sync::RwLock`; `AsyncCache` wraps `tokio::sync::RwLock`.
- **Health & reporting utilities**: `utils` exposes `CacheHealthReport`, `MetricsReporter`, and `CacheWarmer`.
- **Examples & benches**: `examples.rs` captures real-world patterns; `benches/cache_bench.rs` measures throughput.

---

## Eviction Policies

| Policy | Description | Typical use |
| --- | --- | --- |
| `LRU` (default) | Removes the least recently accessed entry. | General purpose caches, session stores. |
| `LFU` | Drops entries with the lowest access count. | Memoization of hot computations. |
| `FIFO` | Evicts by insertion order. | Queued workloads and streaming buffers. |
| `Random` | Removes a random entry. | Workloads prone to pathological eviction storms. |
| `None` | Never evicts automatically. | TTL-only caches or manually managed stores. |

Configure policies through `CacheConfig::builder().eviction_policy(...)`.

---

## Configuration Cheatsheet

`CacheConfig` gathers capacity, TTL, policy, and metrics preferences. Convenient presets cover the common cases:

```rust
use std::time::Duration;
use pulsearc_common::cache::{Cache, CacheConfig, EvictionPolicy};

// Pure TTL cache (no eviction) for ephemeral secrets.
let ttl_cache = Cache::<String, String>::new(CacheConfig::ttl(Duration::from_secs(30)));

// LRU with 1k entries.
let lru_cache = Cache::<String, String>::new(CacheConfig::lru(1_000));

// Combined TTL + LRU.
let session_cache =
    Cache::<String, String>::new(CacheConfig::ttl_lru(Duration::from_secs(600), 5_000));

// Fully custom configuration.
let tuned_cache = Cache::<String, String>::new(
    CacheConfig::builder()
        .max_size(10_000)
        .ttl(Duration::from_secs(120))
        .eviction_policy(EvictionPolicy::LFU)
        .track_metrics(true)
        .build(),
);
```

Internally, TTL and eviction decisions operate on monotonic `Instant`s stored alongside each entry.

---

## Operations Overview

Sync API (from `core.rs`):

| Method | Purpose |
| --- | --- |
| `insert(key, value)` | Upserts a value, evicting if `max_size` is reached. |
| `get(&key)` | Returns a clone of the cached value; enforces TTL. |
| `get_or_insert_with(key, f)` | Atomically compute-once, cache thereafter. |
| `remove(&key)` | Manually delete an entry. |
| `cleanup_expired()` | Sweep TTL-expired items; returns removal count. |
| `len() / is_empty()` | Lightweight occupancy checks. |
| `stats()` | Snapshot metrics (requires `track_metrics(true)`). |
| `clear()` | Drop all entries and reset metrics. |

The async API (`async_core.rs`) mirrors the same semantics with `async fn` methods and adds `get_or_insert_with_async`.

---

## Metrics & Observability

Enable metrics via `CacheConfig::builder().track_metrics(true)` to collect `CacheStats` (hits, misses, evictions, expirations, inserts). Utilities in `utils.rs` build on top:

```rust
use pulsearc_common::cache::{Cache, CacheConfig};
use pulsearc_common::cache::utils::{CacheHealthReport, MetricsReporter};

let cache = Cache::<String, String>::new(
    CacheConfig::builder().max_size(500).track_metrics(true).build(),
);

let stats = cache.stats();
println!("Hit rate: {:.1}%", stats.hit_rate() * 100.0);

let report = CacheHealthReport::new(&cache);
for recommendation in &report.recommendations {
    println!("• {recommendation}");
}

let reporter = MetricsReporter::new("edge-config");
let json = reporter.report_json(&cache);
println!("{}", json);
```

- `CacheHealthReport` classifies health (`Healthy`, `LowHitRate`, `NearCapacity`, `Critical`) and suggests remediation steps.
- `MetricsReporter::report` emits tracing logs when the `observability` feature is enabled.
- `CacheWarmer` simplifies pre-loading data or invoking loader callbacks before serving traffic.

---

## Async Patterns

`AsyncCache` is designed for Tokio runtimes and shares the same configuration types:

```rust
use pulsearc_common::cache::{AsyncCache, CacheConfig, EvictionPolicy};

async fn fetch_user(id: String) -> Option<String> {
    // Fetch from the database...
    Some(format!("user:{id}"))
}

async fn cached_user(id: String, cache: &AsyncCache<String, String>) -> Option<String> {
    cache
        .get_or_insert_with_async(id.clone(), || async move {
            fetch_user(id).await.unwrap_or_default()
        })
        .await;
    cache.get(&id).await
}

let cache =
    AsyncCache::new(CacheConfig::builder().max_size(1_000).eviction_policy(EvictionPolicy::LRU).build());
```

- Concurrency is handled by `tokio::sync::RwLock`.
- Eviction and TTL logic matches the synchronous implementation.
- `stats()` is synchronous and non-blocking (uses `try_read` to avoid await in reporting paths).

---

## Utilities & Examples

- **`examples.rs`** documents patterns like API response caching, session stores, memoization, and cache warming with `Arc`-wrapped payloads.
- **`utils.rs`** provides:
  - `CacheHealthReport` / `CacheHealth`
  - `MetricsReporter`
  - `CacheWarmer`
- **Integration tests** (`crates/common/tests/cache_integration.rs`) validate all eviction policies, TTL handling, and `get_or_insert_with` behavior under the `runtime` feature gate.

---

## Benchmarks

Criterion benchmarks live in `crates/common/benches/cache_bench.rs`. They measure core operations (insert/get), policy comparisons, TTL sweeps, and async behavior.

```bash
cargo bench -p pulsearc-common --bench cache_bench --features runtime
```

The harness groups runs by operation type (e.g., `cache_insert`, `cache_get_hit`) so you can quickly compare throughput across capacities.

---

## Testing Matrix

| Target | Command | Notes |
| --- | --- | --- |
| Unit tests | `cargo test -p pulsearc-common --features runtime --lib cache` | Runs module tests (`core.rs`, `async_core.rs`, `config.rs`, `stats.rs`, `utils.rs`). |
| Integration | `cargo test -p pulsearc-common --features runtime --test cache_integration` | Exercises cross-cutting scenarios (TTL + eviction, concurrency). |
| Workspace | `make test` | Invokes the standard repository test suite. |
| CI parity | `make ci` | Runs Rust + frontend checks (preferred before PRs). |

---

## File Layout

| Path | Purpose |
| --- | --- |
| `core.rs` | Synchronous cache implementation (`Cache`). |
| `async_core.rs` | Tokio-based cache (`AsyncCache`). |
| `config.rs` | `CacheConfig`, builder, and eviction policy types. |
| `stats.rs` | `CacheStats` & `MetricsCollector`. |
| `utils.rs` | Health checks, reporters, warmers. |
| `examples.rs` | Curated usage patterns. |
| `../../benches/cache_bench.rs` | Criterion benchmark suite for cache workloads. |

---

## Design Notes & Limitations

- Values must implement `Clone`; consider wrapping large payloads in `Arc<_>` (see `examples::example_arc_pattern`).
- TTL is enforced on access and during manual sweeps (`cleanup_expired`). Long-lived caches should schedule sweeps to remove cold entries proactively.
- `EvictionPolicy::None` allows the cache to outgrow `max_size`; rely on TTL or explicit `remove`/`clear`.
- Random eviction depends on the `rand` crate (brought in through the `foundation` feature).
- Metrics counters accumulate globally; call `clear()` to reset them after load tests.

---

## Contributing

- Follow the repo guidelines (`make fmt`, `make test`, `make ci`).
- Update this document whenever you add new cache utilities, metrics, or behaviors.
- Document new feature flags or environment knobs in `docs/` to keep onboarding smooth.

---

## License

PulseArc is dual-licensed under Apache-2.0 and MIT. Refer to the repository root for the full texts.

