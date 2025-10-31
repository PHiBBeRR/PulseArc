# Privacy Module

Portable privacy primitives shared by the `pulsearc-common` crate. The module bundles secure domain hashing, a configurable PII detection pipeline, and the metrics/compliance wiring that other crates import without application-specific baggage.

## Module Layout

```
privacy/
├── README.md
├── hash/
│   ├── config.rs        # algorithm + salt configuration helpers
│   ├── error.rs         # HashError + HashResult
│   ├── hasher.rs        # SecureHasher implementation + tests
│   └── metrics.rs       # Prometheus-backed metrics collector
├── patterns/
│   ├── config.rs        # PiiDetectionConfig and related knobs
│   ├── core.rs          # async PatternMatcher pipeline
│   ├── error.rs         # PiiError + PiiResult
│   ├── metrics.rs       # in-memory metrics collector utilities
│   └── types.rs         # domain types used across detection
└── mod.rs               # public re-exports consumed elsewhere
```

## Feature Highlights

- Deterministic, per-tenant hashing for domains and similar identifiers using SHA-256/384/512 with organization salts and rotation helpers.
- Async PII detection pipeline with configurable regex patterns, contextual heuristics, and optional false-positive reduction.
- Built-in string redaction helper that preserves context while obfuscating sensitive values.
- Metrics collectors for hash and pattern operations that expose Prometheus-friendly counters, gauges, and histograms.
- Rich configuration types covering compliance frameworks, retention policies, performance tuning, and security limits.
- Safe sharing across tasks via `Arc<RwLock<...>>`, allowing hot configuration updates without interrupting in-flight requests.

## Crate Features & Dependencies

The privacy module lives inside `pulsearc-common`. Because the crate heavily uses feature flags, make sure the consumer enables the right set:

- `foundation`: required for hashing (pulls in `serde`, `rand`, `regex`, `sha2`, `hex`, etc.).
- `runtime`: required for the pattern pipeline, async primitives, Prometheus metrics, and integration tests (enables `tokio`, `prometheus`, `lazy_static`, `tracing`, ...).

Example dependency stanza for a downstream crate:

```toml
[dependencies]
pulsearc-common = { path = "crates/common", features = ["runtime"] }
```

> When only the hashing utilities are needed you can depend on `pulsearc-common` with the `"foundation"` feature instead.

## Quick Start

### Secure Domain Hashing

```rust
use pulsearc_common::privacy::{HashAlgorithm, HashConfig, SecureHasher};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start from the built-in config so org-specific settings stay together.
    let mut config = HashConfig::with_algorithm(HashAlgorithm::Sha384)?;
    config.set_org_salt("acme::2024".to_owned())?;

    let hasher = SecureHasher::with_config(config)?;
    let digest = hasher.hash_domain("analytics.example")?;

    println!("Hashed domain: {digest}");
    Ok(())
}
```

Key points:

- `HashConfig::default()` already seeds a cryptographically secure salt; persist it if you need deterministic hashes across restarts.
- `SecureHasher::hash_multiple_domains` processes a slice and fails fast if any element is invalid (empty string, etc.).
- `SecureHasher::rotate_salt` regenerates the organizational salt in place.

### PII Detection & Redaction

```rust
use pulsearc_common::privacy::patterns::{AnalysisContext, PatternMatcher, PiiDetectionConfig, PiiResult};

#[tokio::main]
async fn main() -> PiiResult<()> {
    // Defaults cover common PII types (email, phone, SSN, IP, credit card…)
    let matcher = PatternMatcher::with_defaults().await?;

    let text = "Contact ops@example.io or +1-555-0100 for support.";
    let report = matcher
        .detect_pii_comprehensive(text, AnalysisContext::default())
        .await?;

    for entity in &report.entities {
        println!(
            "Found {:?} with confidence {} at {}..{}",
            entity.entity_type,
            entity.confidence,
            entity.start_position,
            entity.end_position
        );
    }

    let redacted = matcher.redact_pii(text).await?;
    println!("Redacted text: {redacted}");

    Ok(())
}
```

Highlights:

- `PiiDetectionConfig::default()` enables regex, contextual analysis, and checksum validation for the bundled patterns.
- `AnalysisContext` lets you annotate compliance zone, user, and session metadata; helpers offer `::default()`, `::minimal()`, and user-focused constructors.
- `PatternMatcher::update_config` hot-swaps configs without forcing a restart.

## Hash Subsystem (`hash/`)

### Key Types

- `SecureHasher`: main hashing facade with deterministic outputs per organization salt.
- `HashConfig`: stores `org_salt`, `salt_length`, and `HashAlgorithm`.
- `HashAlgorithm`: `Sha256`, `Sha384`, and `Sha512`.
- `HashMetricsCollector`: tracks performance/security/compliance counters via Prometheus.
- `HashError` / `HashResult`: strongly typed error channel for callers.

### Capabilities

- Deterministic hashing via `hash_domain` and batch hashing via `hash_multiple_domains`.
- Security helpers: `rotate_salt`, `generate_org_salt`, `set_org_salt`.
- Output lengths match algorithm bit-size (64, 96, 128 hex chars).
- Validation ensures salts and domains are non-empty before hashing.

### Metrics Integration

`hash/metrics.rs` registers a set of Prometheus collectors (histograms for duration, counters for successes/failures, gauges for salt age, etc.). Use it when you need richer telemetry:

```rust
use pulsearc_common::privacy::hash::{HashMetricsCollector, HashOperationParams, HashResult};
use std::time::{Duration, Instant};

fn record_example(metrics: &mut HashMetricsCollector) -> HashResult<()> {
    let start = Instant::now();
    // ... invoke the hasher ...
    metrics.record_operation(HashOperationParams {
        operation_id: "hash#1".into(),
        algorithm: "sha256".into(),
        compliance_mode: "gdpr".into(),
        duration: start.elapsed(),
        input_size: 24,
    });
    Ok(())
}
```

`HashMetricsCollector::export_prometheus_metrics` emits a ready-to-serve metrics buffer that you can expose from a background task or Tauri command.

## Pattern Subsystem (`patterns/`)

### Key Types

- `PatternMatcher`: async detection and redaction pipeline.
- `PiiDetectionConfig`: top-level configuration for detection methods, thresholds, ML hooks, retention policies, and auditing preferences.
- `PiiEntity`: result item with context window, confidence, sensitivity level, detection method, and compliance tags.
- `RedactionStrategy`, `ComplianceFramework`, `SensitivityLevel`, and other enum types exposed via `patterns::types`.

### Detection Pipeline

1. Input validation based on `SecurityConfig` (size limits, anomaly checks).
2. Cache lookup (configurable LRU capped at 512 entries and roughly 5 MiB).
3. Method execution for each enabled `DetectionMethod` (`Regex`, `ContextualAnalysis`, `ChecksumValidation`, plus placeholders for dictionaries and ML).
4. False-positive reduction if enabled (regex overrides and heuristics).
5. Compliance evaluation across configured frameworks.
6. Result aggregation with processing time, sensitivity roll-up, and optional metadata/audit hooks.

### Built-in Coverage

The default config ships regexes for email, phone, SSN, credit card, IP address, and several other identifiers. Additional `PiiType` variants include driver's licenses, tax IDs, financial records, biometric markers, usernames, API keys, and a `Custom(String)` escape hatch for bespoke patterns.

To add a new pattern dynamically:

```rust
use pulsearc_common::privacy::patterns::{
    PatternConfig, PiiDetectionConfig, PiiType, RedactionStrategy, SensitivityLevel, ConfidenceScore,
    ComplianceFramework,
};

let mut config = PiiDetectionConfig::default();
config.pattern_configs.insert(
    PiiType::Custom("order_number".into()),
    PatternConfig {
        pattern_type: PiiType::Custom("order_number".into()),
        regex_patterns: vec![r"\bORD-\d{6}\b".into()],
        context_patterns: vec!["order".into()],
        exclusion_patterns: vec![],
        sensitivity_level: SensitivityLevel::Internal,
        redaction_strategy: RedactionStrategy::PartialMasking,
        minimum_confidence: ConfidenceScore::new(0.75),
        enabled: true,
        compliance_frameworks: vec![ComplianceFramework::Ccpa],
        custom_validators: vec![],
    },
);
```

### Metrics & Quality Tracking

`patterns/metrics.rs` exposes `PiiMetricsCollector`, which records detection operations (`DetectionOperationParams`), quality statistics, and compliance snapshots. The collector keeps in-memory aggregates that you can serialize via `MetricsSnapshot` for dashboards or audits.

## Testing & Benchmarking

- Unit tests live alongside implementations (`hash/hasher.rs`, `patterns/core.rs`). Run them with:

  ```bash
  cargo test -p pulsearc-common hash::hasher
  cargo test -p pulsearc-common patterns::core
  ```

- Integration coverage is under `crates/common/tests/privacy_integration.rs` (guarded by `cfg(feature = "runtime")`):

  ```bash
  cargo test -p pulsearc-common --features runtime --test privacy_integration
  ```

- Benchmarks (`privacy_bench`) require the `"runtime"` feature:

  ```bash
  cargo bench -p pulsearc-common --features runtime privacy_bench
  ```

Running `make test` or `make ci` from the repository root will also execute the privacy suites.

## Extending & Maintenance Tips

- When introducing new hash algorithms, update `HashAlgorithm`, extend the match arms in `SecureHasher::hash_domain`, and adjust the integration tests for expected output lengths.
- To adjust default detection coverage, modify `PiiDetectionConfig::default` and update fixtures in `privacy_integration.rs`.
- Keep salt values outside source control; `HashConfig::set_org_salt` expects callers to load secrets from a secure store.
- Prometheus collectors are registered globally. Duplicate metric names will panic, so reuse the shared collectors instead of constructing ad-hoc ones.
- The pattern cache intentionally caps memory usage. Inspect `PatternMatcher::get_performance_metrics()` when profiling large workloads before tweaking cache limits.

## Related Modules

- `privacy::hash::metrics` integrates with the broader observability stack (`crates/common/src/observability`).
- `privacy::patterns` pairs with infra components that persist audit trails and compliance reports.
- Storage and redaction policies defined here are referenced by domain and API crates when constructing compliance workflows.
