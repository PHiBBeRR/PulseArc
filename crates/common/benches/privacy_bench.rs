//! Privacy benchmarks for hashing and pattern detection pipelines.
//!
//! These benches focus on the hot paths exercised by the privacy module:
//! secure hashing, batch hashing, configuration rotation, pattern detection,
//! cached lookups, and redaction throughput.
//!
//! Run with: `cargo bench --bench privacy_bench -p pulsearc-common --features
//! runtime`

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::privacy::hash::{HashAlgorithm, HashConfig, SecureHasher};
use pulsearc_common::privacy::patterns::PatternMatcher;
use tokio::runtime::Runtime;

const BENCH_SALT: &str = "privacy-bench-salt-0123456789abcdef";
const BASE_LOG_LINE: &str = "\
    [2024-03-17T12:01:45Z] customer=John Doe email=john.doe@example.com \
    phone=+1-415-555-2671 ssn=123-45-6789 card=4111-1111-1111-1111 \
    ip=192.168.42.17 agent=\"PulseArc Service\" notes=\"Follow-up required\"\n";

type DomainBatch = (&'static str, Vec<String>);
type DetectionCorpusEntry = (&'static str, Arc<str>);

fn algorithm_name(algo: &HashAlgorithm) -> &'static str {
    match algo {
        HashAlgorithm::Sha256 => "sha256",
        HashAlgorithm::Sha384 => "sha384",
        HashAlgorithm::Sha512 => "sha512",
    }
}

fn build_hasher(algo: &HashAlgorithm) -> SecureHasher {
    let mut config = HashConfig::with_algorithm(algo.clone())
        .expect("failed to create hash config for benchmark");
    config.set_org_salt(BENCH_SALT.to_string()).expect("failed to configure deterministic salt");
    SecureHasher::with_config(config).expect("failed to construct secure hasher for benchmark")
}

fn generate_domain_batches() -> Vec<DomainBatch> {
    vec![
        ("batch_16", (0..16).map(|idx| format!("svc-{idx}.privacy-bench.pulsearc.dev")).collect()),
        (
            "batch_256",
            (0..256).map(|idx| format!("svc-{idx}.privacy-bench.pulsearc.dev")).collect(),
        ),
        (
            "batch_1024",
            (0..1024).map(|idx| format!("svc-{idx}.privacy-bench.pulsearc.dev")).collect(),
        ),
    ]
}

fn generate_detection_corpus() -> Vec<DetectionCorpusEntry> {
    vec![
        ("short_log", Arc::<str>::from(BASE_LOG_LINE)),
        ("medium_log", Arc::<str>::from(BASE_LOG_LINE.repeat(8))),
        ("long_log", Arc::<str>::from(BASE_LOG_LINE.repeat(32))),
    ]
}

fn bench_hash_single_domain(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_hash_single_domain");
    let inputs = vec![
        ("short", "alpha.pulsearc.dev".to_string()),
        ("typical", "customer-events.service.pulsearc.internal".to_string()),
        ("long", "deeply.nested.subdomain.analytics.prod.pulsearc-enterprise.internal".to_string()),
    ];

    let algorithms = [HashAlgorithm::Sha256, HashAlgorithm::Sha384, HashAlgorithm::Sha512];

    for (label, domain) in &inputs {
        group.throughput(Throughput::Elements(1));
        for algorithm in algorithms.iter() {
            let hasher = build_hasher(algorithm);
            group.bench_with_input(
                BenchmarkId::new(algorithm_name(algorithm), label),
                domain,
                |b, domain| {
                    b.iter(|| {
                        let hash =
                            hasher.hash_domain(black_box(domain.as_str())).expect("hashing failed");
                        black_box(hash);
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_hash_multiple_domains(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_hash_multiple_domains");
    let datasets = generate_domain_batches();
    let algorithms = [HashAlgorithm::Sha256, HashAlgorithm::Sha384, HashAlgorithm::Sha512];

    for (label, domains) in &datasets {
        for algorithm in algorithms.iter() {
            let hasher = build_hasher(algorithm);
            group.throughput(Throughput::Elements(domains.len() as u64));
            group.bench_with_input(
                BenchmarkId::new(algorithm_name(algorithm), label),
                domains,
                |b, domains| {
                    let mut domain_refs = Vec::with_capacity(domains.len());
                    b.iter(|| {
                        domain_refs.clear();
                        domain_refs.extend(domains.iter().map(|s| s.as_str()));
                        let hashes = hasher
                            .hash_multiple_domains(black_box(domain_refs.as_slice()))
                            .expect("batch hashing failed");
                        black_box(hashes);
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_hash_salt_rotation(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_hash_salt_rotation");

    group.bench_function("rotate_salt", |b| {
        let mut hasher = build_hasher(&HashAlgorithm::Sha256);
        b.iter(|| {
            hasher.rotate_salt().expect("salt rotation failed");
        });
    });

    group.finish();
}

fn bench_pattern_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_pattern_detection");
    group.sample_size(20);

    let rt = Runtime::new().expect("failed to build tokio runtime for benchmarks");
    let matcher =
        rt.block_on(PatternMatcher::with_defaults()).expect("failed to build pattern matcher");
    let corpus = generate_detection_corpus();

    for (label, text) in &corpus {
        let matcher_clone = matcher.clone();
        let text_arc = Arc::clone(text);
        group.throughput(Throughput::Bytes(text_arc.len() as u64));
        group.bench_function(format!("detect_{label}"), |b| {
            let matcher = matcher_clone.clone();
            let text = Arc::clone(&text_arc);
            b.to_async(&rt).iter(move || {
                let matcher = matcher.clone();
                let text = Arc::clone(&text);
                async move {
                    let entities = matcher
                        .detect_pii(black_box(text.as_ref()))
                        .await
                        .expect("pattern detection failed");
                    black_box(entities);
                }
            });
        });
    }

    group.finish();
}

fn bench_pattern_redaction(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_pattern_redaction");
    group.sample_size(20);

    let rt = Runtime::new().expect("failed to build tokio runtime for benchmarks");
    let matcher =
        rt.block_on(PatternMatcher::with_defaults()).expect("failed to build pattern matcher");
    let corpus = generate_detection_corpus();

    for (label, text) in &corpus {
        let matcher_clone = matcher.clone();
        let text_arc = Arc::clone(text);
        group.throughput(Throughput::Bytes(text_arc.len() as u64));
        group.bench_function(format!("redact_{label}"), |b| {
            let matcher = matcher_clone.clone();
            let text = Arc::clone(&text_arc);
            b.to_async(&rt).iter(move || {
                let matcher = matcher.clone();
                let text = Arc::clone(&text);
                async move {
                    let redacted = matcher
                        .redact_pii(black_box(text.as_ref()))
                        .await
                        .expect("pattern redaction failed");
                    black_box(redacted);
                }
            });
        });
    }

    group.finish();
}

fn bench_pattern_cached_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_pattern_cached_detection");
    group.sample_size(20);

    let rt = Runtime::new().expect("failed to build tokio runtime for benchmarks");
    let matcher =
        rt.block_on(PatternMatcher::with_defaults()).expect("failed to build pattern matcher");
    let cached_text = Arc::<str>::from(BASE_LOG_LINE.repeat(48));

    // Warm cache before measuring repeated lookups.
    rt.block_on(async {
        matcher.detect_pii(cached_text.as_ref()).await.expect("cache warm-up failed");
    });

    group.throughput(Throughput::Bytes(cached_text.len() as u64));
    group.bench_function("detect_cached_long_log", |b| {
        let matcher = matcher.clone();
        let text = Arc::clone(&cached_text);
        b.to_async(&rt).iter(move || {
            let matcher = matcher.clone();
            let text = Arc::clone(&text);
            async move {
                let entities = matcher
                    .detect_pii(black_box(text.as_ref()))
                    .await
                    .expect("cached detection failed");
                black_box(entities);
            }
        });
    });

    group.finish();
}

fn bench_pattern_config_updates(c: &mut Criterion) {
    let mut group = c.benchmark_group("privacy_pattern_config_updates");
    group.sample_size(15);

    let rt = Runtime::new().expect("failed to build tokio runtime for benchmarks");
    let matcher =
        rt.block_on(PatternMatcher::with_defaults()).expect("failed to build pattern matcher");
    let counter = Arc::new(AtomicUsize::new(0));

    group.bench_function("update_config_default", |b| {
        let matcher = matcher.clone();
        let counter = Arc::clone(&counter);
        b.to_async(&rt).iter(move || {
            let matcher = matcher.clone();
            let counter = Arc::clone(&counter);
            async move {
                let mut config = matcher.get_config().await;
                let idx = counter.fetch_add(1, Ordering::Relaxed);
                config.version = format!("bench-{idx:06}");
                matcher.update_config(config).await.expect("config update failed");
            }
        });
    });

    group.finish();
}

criterion_group!(
    privacy_benches,
    bench_hash_single_domain,
    bench_hash_multiple_domains,
    bench_hash_salt_rotation,
    bench_pattern_detection,
    bench_pattern_redaction,
    bench_pattern_cached_detection,
    bench_pattern_config_updates,
);
criterion_main!(privacy_benches);
