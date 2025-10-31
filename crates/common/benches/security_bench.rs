//! Comprehensive security benchmarks
//!
//! Benchmarks for encryption, key management, RBAC, and secure memory
//! operations.
//!
//! Run with: `cargo bench --bench security_bench`

use std::collections::HashMap;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
// Import security modules
use pulsearc_common::crypto::encryption::EncryptionService;
use pulsearc_common::security::encryption::{generate_encryption_key, SecureString};
use pulsearc_common::security::rbac::{
    Permission, PolicyCondition, PolicyEffect, RBACManager, RBACPolicy, Role, UserContext,
};

// ============================================================================
// Key Generation Benchmarks
// ============================================================================

fn bench_key_generation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_generation");

    group.bench_function("generate_encryption_key", |b| {
        b.iter(|| {
            let key = generate_encryption_key();
            black_box(key);
        });
    });

    group.bench_function("symmetric_key_generation", |b| {
        b.iter(|| {
            let key = EncryptionService::generate_key();
            black_box(key);
        });
    });

    group.finish();
}

// ============================================================================
// Encryption Benchmarks
// ============================================================================

fn bench_encryption(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption");

    // Test different payload sizes
    let sizes = vec![
        ("small_16B", 16),
        ("medium_1KB", 1024),
        ("large_64KB", 64 * 1024),
        ("xlarge_1MB", 1024 * 1024),
    ];

    for (name, size) in sizes {
        let data = vec![0u8; size];
        let service = EncryptionService::new(EncryptionService::generate_key())
            .expect("Failed to create encryption service");

        group.throughput(Throughput::Bytes(size as u64));

        group.bench_with_input(BenchmarkId::new("encrypt", name), &data, |b, data| {
            b.iter(|| {
                let encrypted = service.encrypt(black_box(data)).expect("Encryption failed");
                black_box(encrypted);
            });
        });

        // Prepare encrypted data for decryption benchmark
        let encrypted = service.encrypt(&data).expect("Failed to encrypt test data");

        group.bench_with_input(BenchmarkId::new("decrypt", name), &encrypted, |b, encrypted| {
            b.iter(|| {
                let decrypted = service.decrypt(black_box(encrypted)).expect("Decryption failed");
                black_box(decrypted);
            });
        });
    }

    group.finish();
}

fn bench_encryption_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("encryption_string");

    let service = EncryptionService::new(EncryptionService::generate_key())
        .expect("Failed to create encryption service");

    let test_data = b"Hello, World! This is a test message for encryption benchmarking.";

    group.bench_function("encrypt_to_string", |b| {
        b.iter(|| {
            let encrypted =
                service.encrypt_to_string(black_box(test_data)).expect("Encryption failed");
            black_box(encrypted);
        });
    });

    let encrypted_string =
        service.encrypt_to_string(test_data).expect("Failed to encrypt test data");

    group.bench_function("decrypt_from_string", |b| {
        b.iter(|| {
            let decrypted = service
                .decrypt_from_string(black_box(&encrypted_string))
                .expect("Decryption failed");
            black_box(decrypted);
        });
    });

    group.finish();
}

// ============================================================================
// Key Derivation Benchmarks
// ============================================================================

fn bench_key_derivation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_derivation");

    // Argon2 is intentionally slow for security - reduce sample size
    group.sample_size(10);

    group.bench_function("argon2_password_derivation", |b| {
        b.iter(|| {
            let service = EncryptionService::from_password(black_box("test_password_123"))
                .expect("Key derivation failed");
            black_box(service);
        });
    });

    group.finish();
}

// ============================================================================
// Key Rotation Benchmarks
// ============================================================================

fn bench_key_rotation(c: &mut Criterion) {
    let mut group = c.benchmark_group("key_rotation");

    group.bench_function("rotate_encryption_key", |b| {
        let mut service = EncryptionService::new(EncryptionService::generate_key())
            .expect("Failed to create encryption service");

        b.iter(|| {
            let new_key = EncryptionService::generate_key();
            service.rotate_key(black_box(new_key)).expect("Key rotation failed");
        });
    });

    group.bench_function("key_fingerprint_generation", |b| {
        let service = EncryptionService::new(EncryptionService::generate_key())
            .expect("Failed to create encryption service");

        b.iter(|| {
            let fingerprint = service.key_fingerprint();
            black_box(fingerprint);
        });
    });

    group.finish();
}

// ============================================================================
// SecureString Benchmarks
// ============================================================================

fn bench_secure_string(c: &mut Criterion) {
    let mut group = c.benchmark_group("secure_string");

    group.bench_function("create_secure_string", |b| {
        b.iter(|| {
            let s = SecureString::new(black_box("test_secret_value_123".to_string()));
            black_box(s);
        });
    });

    let s1 = SecureString::new("test_secret_value_123".to_string());
    let s2 = SecureString::new("test_secret_value_123".to_string());

    group.bench_function("constant_time_eq_same", |b| {
        b.iter(|| {
            let result = s1.constant_time_eq(black_box(&s2));
            black_box(result);
        });
    });

    let s3 = SecureString::new("different_secret_value".to_string());

    group.bench_function("constant_time_eq_different", |b| {
        b.iter(|| {
            let result = s1.constant_time_eq(black_box(&s3));
            black_box(result);
        });
    });

    group.bench_function("regular_eq_comparison", |b| {
        b.iter(|| {
            let result = &s1 == black_box(&s2);
            black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// RBAC Benchmarks
// ============================================================================

fn bench_rbac_manager(c: &mut Criterion) {
    let mut group = c.benchmark_group("rbac");

    // Setup RBAC manager with test data
    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    group.bench_function("create_rbac_manager", |b| {
        b.iter(|| {
            let manager = RBACManager::new();
            black_box(manager);
        });
    });

    group.bench_function("create_role", |b| {
        b.to_async(&rt).iter(|| async {
            // Create fresh manager for each iteration
            let manager = RBACManager::new();
            let role = Role {
                id: "test_role".to_string(),
                name: "Test Role".to_string(),
                description: "A test role".to_string(),
                permissions: std::collections::HashSet::new(),
                parent_role: None,
                priority: 100,
            };

            manager.create_role(black_box(role)).await.expect("Failed to create role");
        });
    });

    group.bench_function("register_permission", |b| {
        b.to_async(&rt).iter(|| async {
            // Create fresh manager for each iteration
            let manager = RBACManager::new();
            let permission = Permission {
                id: "test_perm".to_string(),
                resource: "test_resource".to_string(),
                action: "read".to_string(),
                description: "Test permission".to_string(),
                requires_approval: false,
            };

            manager
                .register_permission(black_box(permission))
                .await
                .expect("Failed to register permission");
        });
    });

    group.finish();
}

fn bench_rbac_permission_checks(c: &mut Criterion) {
    let mut group = c.benchmark_group("rbac_permission_checks");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    // Setup manager with realistic data for permission grant check
    let manager_granted = rt.block_on(async {
        let manager = RBACManager::new();

        // Add test permissions
        for i in 0..10 {
            let permission = Permission {
                id: format!("perm_granted_{i}"),
                resource: format!("resource_{}", i % 3),
                action: if i % 2 == 0 { "read" } else { "write" }.to_string(),
                description: format!("Permission {i}"),
                requires_approval: i % 3 == 0,
            };
            manager.register_permission(permission).await.expect("Failed to register permission");
        }

        // Add test roles
        let mut permissions = std::collections::HashSet::new();
        permissions.insert("perm_granted_0".to_string());
        permissions.insert("perm_granted_1".to_string());

        let role = Role {
            id: "admin_granted".to_string(),
            name: "Administrator".to_string(),
            description: "Admin role".to_string(),
            permissions,
            parent_role: None,
            priority: 100,
        };
        manager.create_role(role).await.expect("Failed to create role");

        // Assign role to user
        manager.assign_role("user_granted", "admin_granted").await.expect("Failed to assign role");

        manager
    });

    // Setup manager for permission denied check
    let manager_denied = RBACManager::new();

    let user_context_granted = UserContext {
        user_id: "user_granted".to_string(),
        roles: vec!["admin_granted".to_string()],
        session_id: Some("session_123".to_string()),
        ip_address: Some("192.168.1.1".to_string()),
        user_agent: Some("test_agent".to_string()),
        attributes: HashMap::new(),
    };

    let user_context_denied = UserContext {
        user_id: "user_denied".to_string(),
        roles: vec![],
        session_id: Some("session_456".to_string()),
        ip_address: Some("192.168.1.2".to_string()),
        user_agent: Some("test_agent".to_string()),
        attributes: HashMap::new(),
    };

    let perm_granted = Permission::new("resource_0:read");
    let perm_denied = Permission::new("resource_999:delete");

    group.bench_function("check_permission_granted", |b| {
        b.to_async(&rt).iter(|| async {
            let result = manager_granted
                .check_permission(black_box(&user_context_granted), black_box(&perm_granted))
                .await;
            black_box(result);
        });
    });

    group.bench_function("check_permission_denied", |b| {
        b.to_async(&rt).iter(|| async {
            let result = manager_denied
                .check_permission(black_box(&user_context_denied), black_box(&perm_denied))
                .await;
            black_box(result);
        });
    });

    group.finish();
}

fn bench_rbac_policies(c: &mut Criterion) {
    let mut group = c.benchmark_group("rbac_policies");

    let rt = tokio::runtime::Runtime::new().expect("Failed to create runtime");

    let manager = RBACManager::new();

    let time_policy = RBACPolicy {
        id: "time_policy".to_string(),
        name: "Business Hours Only".to_string(),
        condition: PolicyCondition::TimeRange {
            start_time: "09:00".to_string(),
            end_time: "17:00".to_string(),
        },
        effect: PolicyEffect::Allow,
        permissions: vec!["perm_1".to_string()],
    };

    let ip_policy = RBACPolicy {
        id: "ip_policy".to_string(),
        name: "Internal Network Only".to_string(),
        condition: PolicyCondition::IpRange {
            allowed_ips: vec!["192.168.1.0/24".to_string(), "10.0.0.0/8".to_string()],
        },
        effect: PolicyEffect::Allow,
        permissions: vec!["perm_2".to_string()],
    };

    group.bench_function("add_time_based_policy", |b| {
        b.to_async(&rt).iter(|| async {
            manager.add_policy(black_box(time_policy.clone())).await.expect("Failed to add policy");
        });
    });

    group.bench_function("add_ip_based_policy", |b| {
        b.to_async(&rt).iter(|| async {
            manager.add_policy(black_box(ip_policy.clone())).await.expect("Failed to add policy");
        });
    });

    let user_context = UserContext {
        user_id: "user_1".to_string(),
        roles: vec![],
        session_id: Some("session_123".to_string()),
        ip_address: Some("192.168.1.100".to_string()),
        user_agent: Some("test_agent".to_string()),
        attributes: HashMap::new(),
    };

    let test_perm = Permission::new("resource:read");

    group.bench_function("evaluate_time_policy", |b| {
        b.to_async(&rt).iter(|| async {
            manager.add_policy(time_policy.clone()).await.expect("Failed to add policy");
            let result =
                manager.check_permission(black_box(&user_context), black_box(&test_perm)).await;
            black_box(result);
        });
    });

    group.finish();
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(key_benches, bench_key_generation, bench_key_derivation, bench_key_rotation,);

criterion_group!(encryption_benches, bench_encryption, bench_encryption_string_operations,);

criterion_group!(secure_string_benches, bench_secure_string,);

criterion_group!(
    rbac_benches,
    bench_rbac_manager,
    bench_rbac_permission_checks,
    bench_rbac_policies,
);

criterion_main!(key_benches, encryption_benches, secure_string_benches, rbac_benches,);
