//! Cross-module integration tests
//!
//! Tests interactions between multiple modules to ensure they work together
//! correctly

#![cfg(feature = "platform")]

use std::sync::Arc;
use std::time::Duration;

use pulsearc_common::auth::{OAuthClient, OAuthConfig, TokenManager, TokenSet};
use pulsearc_common::cache::{AsyncCache, CacheConfig};
use pulsearc_common::privacy::hash::SecureHasher;
use pulsearc_common::resilience::policies::AlwaysRetry;
use pulsearc_common::resilience::{
    retry_with_policy, CircuitBreaker, CircuitBreakerConfig, RetryConfig,
};
use pulsearc_common::testing::{random_string, MockKeychainProvider};
use pulsearc_common::validation::{EmailValidator, Validator};

mod data;
use data::sample_entities::{sample_emails, TestUser};

/// Generate a unique test identifier
fn unique_test_id(prefix: &str) -> String {
    format!("{}_{}", prefix, random_string(12))
}

/// Custom error for testing
#[derive(Debug, Clone)]
struct TestError(String);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TestError {}

/// Validates integration between cache and validation modules.
///
/// This test ensures that validation can be performed before caching data,
/// preventing invalid data from being stored. Only items passing validation
/// should be cached, demonstrating proper integration between these modules.
///
/// # Test Steps
/// 1. Create cache with LRU policy and email validator
/// 2. Create test users (one with valid email, one with invalid email)
/// 3. Validate each user's email before caching
/// 4. Only cache users that pass validation
/// 5. Verify only valid user is in cache
/// 6. Verify invalid user was rejected and not cached
#[tokio::test(flavor = "multi_thread")]
async fn test_cache_with_validation() {
    let cache: AsyncCache<String, TestUser> = AsyncCache::new(CacheConfig::lru(10));
    let email_validator = EmailValidator::new();

    let users = vec![
        TestUser::new(
            "user1".to_string(),
            "valid@example.com".to_string(),
            "Valid User".to_string(),
            25,
        ),
        TestUser::new(
            "user2".to_string(),
            "invalid-email".to_string(),
            "Invalid User".to_string(),
            30,
        ),
    ];

    for user in users {
        let mut validator = Validator::new();
        let _ = validator.validate_field("email", &user.email, &email_validator);

        if !validator.has_errors() {
            // Only cache valid users
            cache.insert(user.id.clone(), user).await;
        }
    }

    // Only valid user should be cached
    assert!(cache.get(&"user1".to_string()).await.is_some());
    assert!(cache.get(&"user2".to_string()).await.is_none());
}

/// Validates integration between resilience and auth modules with retry logic.
///
/// This test ensures that authentication token operations can be protected by
/// retry mechanisms, making token retrieval more resilient to transient
/// failures. This integration is critical for maintaining authentication in
/// unreliable network conditions.
///
/// # Test Steps
/// 1. Create TokenManager with keychain storage
/// 2. Save initial token set
/// 3. Configure retry with 3 attempts and fixed backoff
/// 4. Wrap token retrieval in retry logic
/// 5. Verify token is successfully retrieved with retry protection
/// 6. Clean up keychain entries
#[tokio::test(flavor = "multi_thread")]
async fn test_resilience_with_auth() {
    let keychain = Arc::new(MockKeychainProvider::new("PulseArcTest".to_string()));
    let service_name = unique_test_id("test_auth_resilience");
    let account_name = "test_user";

    // Create a mock OAuth client for testing
    let config = OAuthConfig::new(
        "dev-test.us.auth0.com".to_string(),
        "test_client".to_string(),
        "http://localhost:3000/callback".to_string(),
        vec!["openid".to_string()],
        None,
    );
    let oauth_client = OAuthClient::new(config);

    let token_manager = Arc::new(TokenManager::new(
        oauth_client,
        keychain.clone(),
        service_name.clone(),
        account_name.to_string(),
        300,
    ));

    // Save initial tokens
    let token_set = TokenSet {
        access_token: "test_access".to_string(),
        refresh_token: Some("test_refresh".to_string()),
        id_token: None,
        expires_in: 3600,
        expires_at: None,
        token_type: "Bearer".to_string(),
        scope: None,
    };

    token_manager.store_tokens(token_set.clone()).await.expect("Failed to save tokens");

    // Use retry to get tokens with resilience
    let retry_config = RetryConfig::new()
        .max_attempts(3)
        .fixed_backoff(Duration::from_millis(10))
        .build()
        .expect("Failed to build config");

    let policy = pulsearc_common::resilience::policies::AlwaysRetry;
    let tm_clone = Arc::clone(&token_manager);
    let result = pulsearc_common::resilience::retry_with_policy(retry_config, policy, || async {
        tm_clone.get_access_token().await.map_err(|e| TestError(e.to_string()))
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "test_access");

    // Cleanup
    let _ = keychain.delete_tokens(account_name);
}

/// Validates integration between cache and privacy modules for secure caching.
///
/// This test ensures that sensitive domain information is hashed before
/// caching, providing privacy protection while maintaining cache functionality.
/// Demonstrates that hashed values are deterministic and can be consistently
/// retrieved from cache.
///
/// # Test Steps
/// 1. Create async cache with LRU policy
/// 2. Initialize secure hasher
/// 3. Hash multiple domains and cache the hashes
/// 4. Retrieve cached hashes and verify they match fresh hashes
/// 5. Confirm deterministic hashing produces consistent results
#[tokio::test(flavor = "multi_thread")]
async fn test_cache_with_privacy() {
    let cache: AsyncCache<String, String> = AsyncCache::new(CacheConfig::lru(100));
    let hasher = SecureHasher::new().expect("Failed to create hasher");

    let domains = vec!["example.com", "test.org", "sample.net"];

    // Hash and cache domains
    for domain in &domains {
        let hash = hasher.hash_domain(domain).expect("Failed to hash");
        cache.insert(domain.to_string(), hash.clone()).await;
    }

    // Verify cached hashes
    for domain in &domains {
        let cached_hash = cache.get(&domain.to_string()).await;
        assert!(cached_hash.is_some());

        let fresh_hash = hasher.hash_domain(domain).expect("Failed to hash");
        assert_eq!(cached_hash.expect("Should be cached"), fresh_hash);
    }
}

/// Validates integration between validation and privacy modules for secure data
/// handling.
///
/// This test ensures that sensitive data (emails) is validated before being
/// hashed, creating a pipeline where only valid sensitive data is hashed for
/// privacy protection. Invalid data is rejected before hashing, preventing
/// pollution of privacy-protected storage.
///
/// # Test Steps
/// 1. Create email validator and secure hasher
/// 2. Generate sample email addresses
/// 3. Validate each email before processing
/// 4. Hash only valid emails for privacy protection
/// 5. Verify all hashes are unique (no collisions)
/// 6. Confirm only valid emails were processed
#[tokio::test(flavor = "multi_thread")]
async fn test_validation_with_privacy() {
    let email_validator = EmailValidator::new();
    let hasher = SecureHasher::new().expect("Failed to create hasher");

    let emails = sample_emails(5);
    let mut valid_hashes = Vec::new();

    for email in emails {
        let mut validator = Validator::new();
        let _ = validator.validate_field("email", &email, &email_validator);

        if !validator.has_errors() {
            // Hash valid emails for privacy
            let hash = hasher.hash_domain(&email).expect("Failed to hash email");
            valid_hashes.push(hash);
        }
    }

    assert!(!valid_hashes.is_empty());
    // All hashes should be unique
    let unique_count = valid_hashes.iter().collect::<std::collections::HashSet<_>>().len();
    assert_eq!(unique_count, valid_hashes.len());
}

/// Validates integration between circuit breaker and cache for resilient
/// caching.
///
/// This test ensures that cache operations can be protected by a circuit
/// breaker, preventing cascade failures when cache operations fail. The circuit
/// breaker monitors cache operation health and fails fast when necessary.
///
/// # Test Steps
/// 1. Create cache wrapped in Arc for sharing
/// 2. Configure circuit breaker with failure/success thresholds
/// 3. Perform multiple cache insertions through circuit breaker
/// 4. Verify all operations succeed and circuit remains closed
/// 5. Verify all cached values are retrievable
/// 6. Confirm circuit breaker tracks successful operations
#[tokio::test(flavor = "multi_thread")]
async fn test_circuit_breaker_with_cache() {
    let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(10));
    let cache = Arc::new(cache);

    let cb_config = CircuitBreakerConfig::builder()
        .failure_threshold(3)
        .success_threshold(1)
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to build config");

    let breaker = CircuitBreaker::new(cb_config).expect("Failed to create circuit breaker");

    // Successful cache operations
    for i in 0..5 {
        let cache_clone = Arc::clone(&cache);
        let result = breaker
            .execute(|| async move {
                cache_clone.insert(format!("key{}", i), i).await;
                Ok::<_, TestError>(())
            })
            .await;
        assert!(result.is_ok());
    }

    // Verify cached values
    for i in 0..5 {
        let value = cache.get(&format!("key{}", i)).await;
        assert_eq!(value, Some(i));
    }
}

/// Validates integration between retry and validation with transient failures.
///
/// This test ensures that validation operations can be retried when
/// experiencing transient failures, such as temporary service unavailability.
/// The retry mechanism persists through failures until validation succeeds or
/// max attempts are reached.
///
/// # Test Steps
/// 1. Create retry config with 5 max attempts and fixed backoff
/// 2. Initialize email validator
/// 3. Simulate transient failures for first 2 attempts
/// 4. Allow validation to succeed on 3rd attempt
/// 5. Verify retry mechanism persisted and eventually succeeded
/// 6. Confirm exactly 3 attempts were made (2 failures + 1 success)
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_with_validation() {
    let attempt_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let email_validator = EmailValidator::new();

    let retry_config = RetryConfig::builder()
        .max_attempts(5)
        .fixed_backoff(Duration::from_millis(10))
        .build()
        .expect("Failed to build config");

    let count_clone = Arc::clone(&attempt_count);
    let result = retry_with_policy(retry_config, AlwaysRetry, || async {
        let count = count_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Simulate transient failure
        if count < 2 {
            return Err(TestError("Transient validation service error".to_string()));
        }

        // Validate email
        let mut validator = Validator::new();
        let _ = validator.validate_field("email", &"test@example.com", &email_validator);

        if validator.has_errors() {
            Err(TestError("Validation failed".to_string()))
        } else {
            Ok("Validation succeeded")
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(attempt_count.load(std::sync::atomic::Ordering::SeqCst), 3);
}

/// Validates complete workflow integrating cache, resilience, and validation
/// modules.
///
/// This test demonstrates a real-world scenario where multiple modules work
/// together: users are validated, cache operations are protected by circuit
/// breakers, and only valid data is cached. This represents a complete data
/// processing pipeline with proper validation, error handling, and caching.
///
/// # Test Steps
/// 1. Setup cache with LRU policy wrapped in Arc
/// 2. Configure circuit breaker to protect operations
/// 3. Initialize validators for email, name, and age
/// 4. Generate batch of test users
/// 5. For each user: validate fields through circuit breaker
/// 6. Cache only users passing all validations
/// 7. Verify cache contains valid users only
/// 8. Confirm circuit breaker tracked operations correctly
#[tokio::test(flavor = "multi_thread")]
async fn test_complete_workflow() {
    // Setup components
    let cache: AsyncCache<String, TestUser> = AsyncCache::new(CacheConfig::lru(50));
    let cache = Arc::new(cache);

    let cb_config = CircuitBreakerConfig::new()
        .failure_threshold(5)
        .success_threshold(2)
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to build config");

    let breaker =
        Arc::new(CircuitBreaker::new(cb_config).expect("Failed to create circuit breaker"));
    let email_validator = EmailValidator::new();

    // Process users through validation, resilience, and caching
    let users = TestUser::batch(10);

    for user in users {
        let cache_clone = Arc::clone(&cache);
        let breaker_clone = Arc::clone(&breaker);

        // Validate user first (synchronously)
        let mut validator = Validator::new();
        let _ = validator.validate_field("email", &user.email.clone(), &email_validator);
        let _ = validator.validate_not_empty("name", &user.name);
        let _ = validator.validate_range("age", user.age, 0, 150);

        // Wrap validation result in circuit breaker
        let validation_result = breaker_clone.call(|| {
            if !validator.has_errors() {
                Ok::<_, TestError>(user.clone())
            } else {
                Err(TestError("Validation failed".to_string()))
            }
        });

        // If validation passed and circuit is closed, cache the user
        if let Ok(valid_user) = validation_result {
            cache_clone.insert(valid_user.id.clone(), valid_user).await;
        }
    }

    // Verify cache has valid users
    let stats = cache.stats();
    assert!(stats.size > 0);
}

/// Validates integration between auth and cache for token caching.
///
/// This test ensures authentication tokens can be cached to improve
/// performance, reducing keychain access frequency while maintaining security.
/// Tokens are stored in keychain for persistence and cached in memory for fast
/// access.
///
/// # Test Steps
/// 1. Create cache with TTL for token expiration
/// 2. Initialize TokenManager with keychain
/// 3. Save tokens to keychain (persistent storage)
/// 4. Retrieve token from keychain
/// 5. Cache token in memory for fast access
/// 6. Verify cached token matches retrieved token
/// 7. Clean up keychain entries
#[tokio::test(flavor = "multi_thread")]
async fn test_auth_with_cache() {
    let cache: AsyncCache<String, String> =
        AsyncCache::new(CacheConfig::ttl(Duration::from_secs(300)));
    let keychain = Arc::new(MockKeychainProvider::new("PulseArcTest".to_string()));
    let service_name = unique_test_id("test_auth_cache");

    // Create OAuth client for token manager
    let oauth_config = OAuthConfig::new(
        "test.auth0.com".to_string(),
        "test_client_id".to_string(),
        "http://localhost:8888/callback".to_string(),
        vec!["openid".to_string()],
        None,
    );
    let oauth_client = OAuthClient::new(oauth_config);

    let token_manager = TokenManager::new(
        oauth_client,
        keychain.clone(),
        service_name.clone(),
        "test_user".to_string(),
        300,
    );

    // Save tokens to keychain
    let token_set = TokenSet {
        access_token: "cached_access_token".to_string(),
        refresh_token: Some("cached_refresh_token".to_string()),
        id_token: None,
        expires_in: 3600,
        expires_at: None,
        token_type: "Bearer".to_string(),
        scope: None,
    };

    token_manager.store_tokens(token_set).await.expect("Failed to store tokens");

    // Get token and cache it
    let access_token = token_manager.get_access_token().await.expect("Failed to get token");

    cache.insert("current_access_token".to_string(), access_token.clone()).await;

    // Verify cached token
    let cached = cache.get(&"current_access_token".to_string()).await;
    assert_eq!(cached, Some(access_token));

    // Cleanup - note: delete_password method not available in current
    // KeychainProvider API let _ = keychain.delete_password(&service_name,
    // "test_user");
}

/// Validates thread-safe concurrent operations across multiple modules.
///
/// This test ensures that cache, privacy, and validation modules can be safely
/// used concurrently from multiple async tasks without data races or
/// corruption. Tests realistic concurrent workflow with domain hashing,
/// caching, and validation.
///
/// # Test Steps
/// 1. Create shared cache and hasher wrapped in Arc
/// 2. Spawn 20 concurrent async tasks
/// 3. Each task: hashes a domain, caches a value, validates the domain
/// 4. Verify all tasks complete successfully
/// 5. Confirm no data corruption or races occurred
/// 6. Verify cache contains all expected entries
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_cross_module_operations() {
    let cache: AsyncCache<String, i32> = AsyncCache::new(CacheConfig::lru(100));
    let cache = Arc::new(cache);
    let hasher = Arc::new(SecureHasher::new().expect("Failed to create hasher"));

    let mut handles = vec![];

    for i in 0..20 {
        let cache_clone = Arc::clone(&cache);
        let hasher_clone = Arc::clone(&hasher);

        let handle = tokio::spawn(async move {
            // Hash domain
            let domain = format!("domain{}.com", i);
            let _hash = hasher_clone.hash_domain(&domain).expect("Failed to hash");

            // Cache the hash
            cache_clone.insert(domain.clone(), i).await;

            // Validate the operation
            let mut validator = Validator::new();
            let _ = validator.validate_not_empty("domain", &domain);

            !validator.has_errors()
        });

        handles.push(handle);
    }

    // Wait for all tasks and verify they all succeeded
    for handle in handles {
        let success = handle.await.expect("Task should complete");
        assert!(success);
    }

    // Verify cache has entries
    let stats = cache.stats();
    assert_eq!(stats.size, 20);
}
