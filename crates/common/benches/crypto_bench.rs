//! Comprehensive cryptographic operations benchmarks
//!
//! This benchmark suite provides detailed performance metrics for all
//! cryptographic operations in the crypto module:
//!
//! - **Key Generation**: Random key generation and password-based derivation
//! - **Encryption/Decryption**: AES-256-GCM with various payload sizes
//! - **String Operations**: Base64-encoded encrypt/decrypt workflows
//! - **Key Management**: Rotation, fingerprinting, and reencryption
//! - **Key Storage**: File-based encrypted key persistence
//!
//! ## Running Benchmarks
//!
//! ```bash
//! # Run all crypto benchmarks
//! cargo bench --bench crypto_bench
//!
//! # Run specific benchmark group
//! cargo bench --bench crypto_bench -- key_generation
//! cargo bench --bench crypto_bench -- encryption_throughput
//! cargo bench --bench crypto_bench -- key_storage
//!
//! # Save baseline for comparison
//! cargo bench --bench crypto_bench -- --save-baseline master
//!
//! # Compare against baseline
//! cargo bench --bench crypto_bench -- --baseline master
//! ```
//!
//! ## Expected Performance Characteristics
//!
//! - **Key Generation**: ~50-100μs (random), ~100-200ms (Argon2)
//! - **Encryption (1KB)**: ~5-10μs
//! - **Encryption (1MB)**: ~2-5ms
//! - **Decryption**: Similar to encryption
//! - **Key Rotation**: ~1-2μs
//! - **Reencryption**: decrypt + encrypt overhead

use std::path::PathBuf;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulsearc_common::crypto::encryption::{EncryptedData, EncryptionService};
use tempfile::TempDir;

// ============================================================================
// Constants for benchmarking
// ============================================================================

const SMALL_DATA: usize = 16; // 16 bytes - small metadata
const MEDIUM_DATA: usize = 1024; // 1 KB - typical config/token
const LARGE_DATA: usize = 64 * 1024; // 64 KB - large document
const XLARGE_DATA: usize = 1024 * 1024; // 1 MB - file attachment
const XXLARGE_DATA: usize = 10 * 1024 * 1024; // 10 MB - large file

const TEST_PASSWORD: &str = "test_password_for_benchmarking_purposes";

// ============================================================================
// Key Generation Benchmarks
// ============================================================================

fn bench_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_generation");

    group.bench_function("generate_random_key", |b| {
        b.iter(|| {
            let key = EncryptionService::generate_key();
            black_box(key);
        });
    });

    // Test key generation from fresh service
    group.bench_function("new_service_from_key", |b| {
        let key = EncryptionService::generate_key();
        b.iter(|| {
            let service =
                EncryptionService::new(black_box(key.clone())).expect("Failed to create service");
            black_box(service);
        });
    });

    group.finish();
}

// ============================================================================
// Password-Based Key Derivation Benchmarks (Argon2)
// ============================================================================

fn bench_password_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("password_derivation");

    // Argon2 is intentionally slow - reduce sample size
    group.sample_size(10);

    group.bench_function("argon2_new_salt", |b| {
        b.iter(|| {
            let service = EncryptionService::from_password(black_box(TEST_PASSWORD))
                .expect("Derivation failed");
            black_box(service);
        });
    });

    // Test with existing salt (faster - no salt generation)
    let service =
        EncryptionService::from_password(TEST_PASSWORD).expect("Failed to create service");
    let encrypted_test = service.encrypt(b"test").expect("Failed to encrypt");
    let salt = encrypted_test.salt.as_ref().expect("No salt found");

    group.bench_function("argon2_existing_salt", |b| {
        b.iter(|| {
            let service = EncryptionService::from_password_with_salt(
                black_box(TEST_PASSWORD),
                Some(black_box(salt)),
            )
            .expect("Derivation failed");
            black_box(service);
        });
    });

    group.finish();
}

// ============================================================================
// Encryption Throughput Benchmarks
// ============================================================================

fn bench_encryption_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption_throughput");

    let service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create encryption service");

    let sizes = vec![
        ("16B", SMALL_DATA),
        ("1KB", MEDIUM_DATA),
        ("64KB", LARGE_DATA),
        ("1MB", XLARGE_DATA),
        ("10MB", XXLARGE_DATA),
    ];

    for (name, size) in sizes {
        let data = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark encryption
        group.bench_with_input(BenchmarkId::new("encrypt", name), &data, |b, data| {
            b.iter(|| {
                let encrypted = service.encrypt(black_box(data)).expect("Encryption failed");
                black_box(encrypted);
            });
        });

        // Prepare encrypted data for decryption benchmark
        let encrypted = service.encrypt(&data).expect("Failed to prepare encrypted data");

        // Benchmark decryption
        group.bench_with_input(BenchmarkId::new("decrypt", name), &encrypted, |b, encrypted| {
            b.iter(|| {
                let decrypted = service.decrypt(black_box(encrypted)).expect("Decryption failed");
                black_box(decrypted);
            });
        });

        // Benchmark round-trip (encrypt + decrypt)
        group.bench_with_input(BenchmarkId::new("round_trip", name), &data, |b, data| {
            b.iter(|| {
                let encrypted = service.encrypt(black_box(data)).expect("Encryption failed");
                let decrypted = service.decrypt(&encrypted).expect("Decryption failed");
                black_box(decrypted);
            });
        });
    }

    group.finish();
}

// ============================================================================
// String-Based Encryption Benchmarks (Base64 encoding/decoding)
// ============================================================================

fn bench_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");

    let service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create encryption service");

    let sizes = vec![("small_16B", 16), ("medium_256B", 256), ("large_4KB", 4096)];

    for (name, size) in sizes {
        let data = vec![0u8; size];

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark encrypt_to_string
        group.bench_with_input(BenchmarkId::new("encrypt_to_string", name), &data, |b, data| {
            b.iter(|| {
                let encrypted =
                    service.encrypt_to_string(black_box(data)).expect("Encryption failed");
                black_box(encrypted);
            });
        });

        // Prepare encrypted string for decryption benchmark
        let encrypted_str =
            service.encrypt_to_string(&data).expect("Failed to prepare encrypted string");

        // Benchmark decrypt_from_string
        group.bench_with_input(
            BenchmarkId::new("decrypt_from_string", name),
            &encrypted_str,
            |b, encrypted_str| {
                b.iter(|| {
                    let decrypted = service
                        .decrypt_from_string(black_box(encrypted_str))
                        .expect("Decryption failed");
                    black_box(decrypted);
                });
            },
        );

        // Benchmark string round-trip
        group.bench_with_input(BenchmarkId::new("string_round_trip", name), &data, |b, data| {
            b.iter(|| {
                let encrypted =
                    service.encrypt_to_string(black_box(data)).expect("Encryption failed");
                let decrypted = service.decrypt_from_string(&encrypted).expect("Decryption failed");
                black_box(decrypted);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Key Management Benchmarks
// ============================================================================

fn bench_key_management(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_management");

    // Benchmark key fingerprint generation
    let service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create encryption service");

    group.bench_function("key_fingerprint", |b| {
        b.iter(|| {
            let fingerprint = service.key_fingerprint();
            black_box(fingerprint);
        });
    });

    // Benchmark key rotation
    group.bench_function("rotate_key", |b| {
        let mut service = EncryptionService::new(EncryptionService::generate_key())
            .expect("Failed to create encryption service");

        b.iter(|| {
            let new_key = EncryptionService::generate_key();
            service.rotate_key(black_box(new_key)).expect("Key rotation failed");
        });
    });

    group.finish();
}

// ============================================================================
// Reencryption Benchmarks
// ============================================================================

fn bench_reencryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("reencryption");

    let service1 = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create first service");
    let service2 = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create second service");

    let sizes = vec![("small_16B", 16), ("medium_1KB", 1024), ("large_64KB", 64 * 1024)];

    for (name, size) in sizes {
        let data = vec![0u8; size];
        let encrypted = service1.encrypt(&data).expect("Failed to prepare encrypted data");

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("reencrypt", name), &encrypted, |b, encrypted| {
            b.iter(|| {
                let reencrypted = service1
                    .reencrypt(black_box(encrypted), &service2)
                    .expect("Reencryption failed");
                black_box(reencrypted);
            });
        });
    }

    group.finish();
}

// ============================================================================
// Key Storage Benchmarks (File I/O)
// ============================================================================

fn bench_key_storage(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_storage");

    // Reduce sample size due to file I/O
    group.sample_size(20);

    let key = EncryptionService::generate_key();

    // Benchmark save_key
    group.bench_function("save_key_to_file", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let key_path: PathBuf = temp_dir.path().join("test_key.enc");
                (temp_dir, key_path)
            },
            |(_temp_dir, key_path)| {
                pulsearc_common::crypto::encryption::key_storage::save_key(
                    black_box(&key),
                    &key_path,
                    TEST_PASSWORD,
                )
                .expect("Failed to save key");
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark load_key
    group.bench_function("load_key_from_file", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let key_path: PathBuf = temp_dir.path().join("test_key.enc");
                pulsearc_common::crypto::encryption::key_storage::save_key(
                    &key,
                    &key_path,
                    TEST_PASSWORD,
                )
                .expect("Failed to save key");
                (temp_dir, key_path)
            },
            |(_temp_dir, key_path)| {
                let loaded_key = pulsearc_common::crypto::encryption::key_storage::load_key(
                    &key_path,
                    black_box(TEST_PASSWORD),
                )
                .expect("Failed to load key");
                black_box(loaded_key);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    // Benchmark round-trip (save + load)
    group.bench_function("key_storage_round_trip", |b| {
        b.iter_batched(
            || {
                let temp_dir = TempDir::new().expect("Failed to create temp dir");
                let key_path: PathBuf = temp_dir.path().join("test_key.enc");
                (temp_dir, key_path)
            },
            |(_temp_dir, key_path)| {
                pulsearc_common::crypto::encryption::key_storage::save_key(
                    black_box(&key),
                    &key_path,
                    TEST_PASSWORD,
                )
                .expect("Failed to save key");

                let loaded_key = pulsearc_common::crypto::encryption::key_storage::load_key(
                    &key_path,
                    TEST_PASSWORD,
                )
                .expect("Failed to load key");
                black_box(loaded_key);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

// ============================================================================
// EncryptedData Serialization Benchmarks
// ============================================================================

fn bench_encrypted_data_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("encrypted_data_serialization");

    let service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create encryption service");

    let data = vec![0u8; 1024];
    let encrypted = service.encrypt(&data).expect("Failed to encrypt data");

    // Benchmark JSON serialization
    group.bench_function("serialize_to_json", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&encrypted)).expect("Serialization failed");
            black_box(json);
        });
    });

    let json = serde_json::to_string(&encrypted).expect("Failed to serialize");

    // Benchmark JSON deserialization
    group.bench_function("deserialize_from_json", |b| {
        b.iter(|| {
            let data: EncryptedData =
                serde_json::from_str(black_box(&json)).expect("Deserialization failed");
            black_box(data);
        });
    });

    // Benchmark JSON round-trip
    group.bench_function("json_round_trip", |b| {
        b.iter(|| {
            let json = serde_json::to_string(black_box(&encrypted)).expect("Serialization failed");
            let data: EncryptedData = serde_json::from_str(&json).expect("Deserialization failed");
            black_box(data);
        });
    });

    group.finish();
}

// ============================================================================
// Comparative Benchmarks (Different Key Sources)
// ============================================================================

fn bench_key_source_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_source_comparison");

    // Reduce sample size for password-based operations
    group.sample_size(10);

    let test_data = vec![0u8; 1024];

    // Random key service
    let random_service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create random key service");

    // Password-based service
    let password_service = EncryptionService::from_password(TEST_PASSWORD)
        .expect("Failed to create password-based service");

    // Benchmark encryption with random key
    group.bench_function("encrypt_with_random_key", |b| {
        b.iter(|| {
            let encrypted =
                random_service.encrypt(black_box(&test_data)).expect("Encryption failed");
            black_box(encrypted);
        });
    });

    // Benchmark encryption with password-derived key
    group.bench_function("encrypt_with_password_key", |b| {
        b.iter(|| {
            let encrypted =
                password_service.encrypt(black_box(&test_data)).expect("Encryption failed");
            black_box(encrypted);
        });
    });

    group.finish();
}

// ============================================================================
// Concurrent Operations Benchmarks
// ============================================================================

fn bench_concurrent_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_operations");

    let service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create encryption service");

    let test_data = vec![0u8; 1024];

    // Benchmark multiple sequential encryptions (simulates batch processing)
    group.bench_function("batch_encrypt_10_items", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let encrypted = service.encrypt(black_box(&test_data)).expect("Encryption failed");
                black_box(encrypted);
            }
        });
    });

    // Prepare encrypted data for batch decryption
    let encrypted_items: Vec<EncryptedData> =
        (0..10).map(|_| service.encrypt(&test_data).expect("Failed to encrypt")).collect();

    group.bench_function("batch_decrypt_10_items", |b| {
        b.iter(|| {
            for encrypted in &encrypted_items {
                let decrypted = service.decrypt(black_box(encrypted)).expect("Decryption failed");
                black_box(decrypted);
            }
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups Registration
// ============================================================================

criterion_group!(
    key_benches,
    bench_key_generation,
    bench_password_derivation,
    bench_key_management
);

criterion_group!(
    encryption_benches,
    bench_encryption_throughput,
    bench_string_operations,
    bench_reencryption
);

criterion_group!(storage_benches, bench_key_storage, bench_encrypted_data_serialization);

criterion_group!(advanced_benches, bench_key_source_comparison, bench_concurrent_operations);

criterion_main!(key_benches, encryption_benches, storage_benches, advanced_benches);
