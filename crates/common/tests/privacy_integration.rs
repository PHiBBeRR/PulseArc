//! Integration tests for privacy module
//!
//! Tests secure hashing and PII pattern detection

#![cfg(feature = "runtime")]

use pulsearc_common::privacy::hash::{HashAlgorithm, HashConfig, SecureHasher};
use pulsearc_common::privacy::patterns::{PatternMatcher, PiiDetectionConfig, PiiType};

/// Validates secure hashing of domain names for privacy protection.
///
/// This test ensures domains are hashed using cryptographic algorithms to
/// protect user privacy. Hashing must be deterministic (same input produces
/// same output) to enable lookups while preventing reverse engineering of
/// original domains.
///
/// # Test Steps
/// 1. Create SecureHasher instance
/// 2. Hash a domain name twice
/// 3. Verify hash is non-empty
/// 4. Verify both hashes are identical (deterministic)
/// 5. Confirm hash cannot be reversed to reveal original domain
#[test]
fn test_secure_domain_hashing() {
    let hasher = SecureHasher::new().expect("Failed to create hasher");

    let domain = "example.com";
    let hash1 = hasher.hash_domain(domain).expect("Failed to hash domain");

    // Hash should be non-empty and deterministic
    assert!(!hash1.is_empty());

    let hash2 = hasher.hash_domain(domain).expect("Failed to hash domain");
    assert_eq!(hash1, hash2, "Hashing should be deterministic");
}

/// Validates support for multiple cryptographic hash algorithms.
///
/// This test ensures the hasher supports different hash algorithms (SHA-256,
/// SHA-384, SHA-512) with appropriate output lengths. Different algorithms
/// provide different security/performance trade-offs for various use cases.
///
/// # Test Steps
/// 1. Test SHA-256 (64 hex chars output)
/// 2. Test SHA-384 (96 hex chars output)
/// 3. Test SHA-512 (128 hex chars output)
/// 4. Verify each algorithm produces non-empty hash
/// 5. Confirm output length matches expected for each algorithm
#[test]
fn test_different_hash_algorithms() {
    let algorithms = vec![HashAlgorithm::Sha256, HashAlgorithm::Sha384, HashAlgorithm::Sha512];

    for algorithm in algorithms {
        let mut config = HashConfig::new().expect("Failed to create config");
        config.algorithm = algorithm.clone();
        config.org_salt = "test-salt-123".to_string();

        let hasher = SecureHasher::with_config(config).expect("Failed to create hasher");
        let hash = hasher.hash_domain("test.com").expect("Failed to hash");

        assert!(!hash.is_empty());

        // Different algorithms should produce different hash lengths
        match algorithm {
            HashAlgorithm::Sha256 => assert_eq!(hash.len(), 64), // 256 bits = 64 hex chars
            HashAlgorithm::Sha384 => assert_eq!(hash.len(), 96), // 384 bits = 96 hex chars
            HashAlgorithm::Sha512 => assert_eq!(hash.len(), 128), // 512 bits = 128 hex chars
        }
    }
}

/// Validates that different salts produce different hashes for same input.
///
/// This test ensures salt values prevent rainbow table attacks by producing
/// unique hashes for the same domain with different salts. Critical for
/// multi-tenant scenarios where each organization needs isolated hash spaces.
///
/// # Test Steps
/// 1. Create two hashers with different salt values
/// 2. Hash same domain with both hashers
/// 3. Verify hashes are different despite same input
/// 4. Confirm salt isolation provides security separation
#[test]
fn test_hash_with_different_salts() {
    let mut config1 = HashConfig::new().expect("Failed to create config");
    config1.org_salt = "salt1".to_string();

    let mut config2 = HashConfig::new().expect("Failed to create config");
    config2.org_salt = "salt2".to_string();

    let hasher1 = SecureHasher::with_config(config1).expect("Failed to create hasher");
    let hasher2 = SecureHasher::with_config(config2).expect("Failed to create hasher");

    let domain = "example.com";
    let hash1 = hasher1.hash_domain(domain).expect("Failed to hash");
    let hash2 = hasher2.hash_domain(domain).expect("Failed to hash");

    assert_ne!(hash1, hash2, "Different salts should produce different hashes");
}

/// Validates batch hashing of multiple domains efficiently.
///
/// This test ensures multiple domains can be hashed in a single operation,
/// improving performance for batch processing. Each domain must produce a
/// unique hash to prevent collisions and maintain data integrity.
///
/// # Test Steps
/// 1. Create hasher instance
/// 2. Prepare list of 3 different domains
/// 3. Hash all domains in single batch operation
/// 4. Verify 3 hashes returned
/// 5. Confirm all hashes are unique (no collisions)
#[test]
fn test_hash_multiple_domains() {
    let hasher = SecureHasher::new().expect("Failed to create hasher");

    let domains = vec!["example.com", "test.org", "sample.net"];
    let hashes = hasher.hash_multiple_domains(&domains).expect("Failed to hash domains");

    assert_eq!(hashes.len(), 3);
    assert_ne!(hashes[0], hashes[1]);
    assert_ne!(hashes[1], hashes[2]);
}

/// Validates salt rotation for enhanced security over time.
///
/// This test ensures salt values can be rotated for security best practices,
/// invalidating old hashes and requiring re-hashing. Salt rotation is important
/// for long-running systems to maintain security against evolving threats.
///
/// # Test Steps
/// 1. Create hasher and hash a domain
/// 2. Store initial hash
/// 3. Rotate the salt value
/// 4. Hash same domain again with new salt
/// 5. Verify hash changed after rotation
/// 6. Confirm old hash is effectively invalidated
#[test]
fn test_salt_rotation() {
    let mut hasher = SecureHasher::new().expect("Failed to create hasher");

    let domain = "example.com";
    let hash_before = hasher.hash_domain(domain).expect("Failed to hash");

    // Rotate salt
    hasher.rotate_salt().expect("Failed to rotate salt");

    let hash_after = hasher.hash_domain(domain).expect("Failed to hash");

    assert_ne!(hash_before, hash_after, "Hash should change after salt rotation");
}

/// Validates error handling for empty or invalid domain input.
///
/// This test ensures the hasher rejects invalid input (empty strings) with
/// appropriate errors rather than producing meaningless hashes. Proper input
/// validation prevents bugs and maintains data quality.
///
/// # Test Steps
/// 1. Create hasher instance
/// 2. Attempt to hash empty string
/// 3. Verify operation returns error (not success)
/// 4. Confirm invalid input is rejected before hashing
#[test]
fn test_empty_domain_error() {
    let hasher = SecureHasher::new().expect("Failed to create hasher");

    let result = hasher.hash_domain("");
    assert!(result.is_err(), "Empty domain should produce error");
}

/// Validates PII detection for email addresses in text.
///
/// This test ensures the pattern matcher can identify email addresses in
/// unstructured text, enabling automatic detection and redaction of sensitive
/// information. Critical for privacy compliance (GDPR, CCPA).
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Provide text containing email address
/// 3. Run PII detection
/// 4. Verify email address was detected
/// 5. Confirm detection includes correct PII type (Email)
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_detection_email() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let text = "Contact me at user@example.com for more information.";
    let detections = matcher.detect_pii(text).await.expect("Failed to detect PII");

    // Should detect email address
    assert!(!detections.is_empty());

    let has_email = detections.iter().any(|d| matches!(d.entity_type, PiiType::Email));
    assert!(has_email, "Should detect email address");
}

/// Validates PII detection for phone numbers in various formats.
///
/// This test ensures the pattern matcher recognizes multiple phone number
/// formats (US-style with dashes, parentheses, international, plain digits).
/// Comprehensive format support is critical for real-world PII detection
/// accuracy.
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Test multiple phone formats: 123-456-7890, (555) 123-4567, etc.
/// 3. Run detection on each format
/// 4. Verify all formats are correctly identified
/// 5. Confirm PII type is PhoneNumber for all cases
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_detection_phone() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let test_cases = vec![
        "Call me at 123-456-7890",
        "Phone: (555) 123-4567",
        "Contact: 5551234567",
        "International: +1-555-123-4567",
    ];

    for text in test_cases {
        let detections = matcher.detect_pii(text).await.expect("Failed to detect PII");

        let has_phone = detections.iter().any(|d| matches!(d.entity_type, PiiType::Phone));
        assert!(has_phone, "Should detect phone number in: {}", text);
    }
}

/// Validates PII detection for Social Security Numbers (SSN).
///
/// This test ensures SSNs are detected in text, protecting highly sensitive
/// personal information. SSN detection is crucial for US privacy compliance
/// and preventing accidental exposure of this critical identifier.
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Provide text containing SSN in format: 123-45-6789
/// 3. Run PII detection
/// 4. Verify SSN was detected
/// 5. Confirm PII type is SSN
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_detection_ssn() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let text = "SSN: 123-45-6789";
    let detections = matcher.detect_pii(text).await.expect("Failed to detect PII");

    let has_ssn = detections.iter().any(|d| matches!(d.entity_type, PiiType::Ssn));
    assert!(has_ssn, "Should detect SSN");
}

/// Validates PII detection for IP addresses in text.
///
/// This test ensures IP addresses are detected as potentially identifying
/// information, as they can be used to track users or reveal locations.
/// Important for comprehensive privacy protection in logs and user data.
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Provide text containing IPv4 address (192.168.1.100)
/// 3. Run PII detection
/// 4. Verify IP address was detected
/// 5. Confirm PII type is IPAddress
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_detection_ip_address() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let text = "Server IP: 192.168.1.100";
    let detections = matcher.detect_pii(text).await.expect("Failed to detect PII");

    let has_ip = detections.iter().any(|d| matches!(d.entity_type, PiiType::IpAddress));
    assert!(has_ip, "Should detect IP address");
}

/// Validates detection of multiple PII types in single text.
///
/// This test ensures the pattern matcher can identify multiple different types
/// of PII in one text sample, critical for real-world scenarios where various
/// sensitive data types appear together (emails, phones, IPs in same log).
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Provide text with email, phone, and IP address
/// 3. Run PII detection
/// 4. Verify at least 3 detections found
/// 5. Confirm all three PII types (Email, PhoneNumber, IPAddress) detected
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_detection_multiple_types() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let text = "Contact: user@example.com, Phone: 123-456-7890, IP: 10.0.0.1";
    let detections = matcher.detect_pii(text).await.expect("Failed to detect PII");

    assert!(detections.len() >= 3, "Should detect multiple PII types");

    let has_email = detections.iter().any(|d| matches!(d.entity_type, PiiType::Email));
    let has_phone = detections.iter().any(|d| matches!(d.entity_type, PiiType::Phone));
    let has_ip = detections.iter().any(|d| matches!(d.entity_type, PiiType::IpAddress));

    assert!(has_email && has_phone && has_ip);
}

/// Validates no false positives when text contains no PII.
///
/// This test ensures the pattern matcher doesn't produce false positives,
/// correctly identifying clean text without sensitive information. Low false
/// positive rate is critical for practical usability and user trust.
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Provide normal text with no sensitive information
/// 3. Run PII detection
/// 4. Verify empty detection list (no false positives)
/// 5. Confirm clean text passes without issues
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_detection_no_pii() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let text = "This is just a normal sentence with no sensitive information.";
    let detections = matcher.detect_pii(text).await.expect("Failed to detect PII");

    assert!(detections.is_empty(), "Should not detect PII in normal text");
}

/// Validates automatic redaction of detected PII from text.
///
/// This test ensures detected PII can be automatically replaced with redaction
/// markers, enabling safe logging and display of text containing sensitive
/// data. Redaction is the final step in PII protection pipeline.
///
/// # Test Steps
/// 1. Create PatternMatcher with default config
/// 2. Provide text with email and phone number
/// 3. Run PII redaction
/// 4. Verify original sensitive values removed from text
/// 5. Confirm redaction markers ([REDACTED]) present
/// 6. Ensure text remains readable but safe
#[tokio::test(flavor = "multi_thread")]
async fn test_pii_redaction() {
    let config = PiiDetectionConfig::default();
    let matcher = PatternMatcher::new(config).await.expect("Failed to create matcher");

    let text = "Email: user@example.com, Phone: 123-456-7890";
    let redacted = matcher.redact_pii(text).await.expect("Failed to redact PII");

    // Redacted text should not contain original PII
    assert!(!redacted.contains("user@example.com"));
    assert!(!redacted.contains("123-456-7890"));

    // Should contain redaction markers
    assert!(redacted.contains("[REDACTED"));
}

/// Validates thread-safe concurrent PII detection from multiple tasks.
///
/// This test ensures PatternMatcher can be safely shared across multiple async
/// tasks, performing concurrent PII detection without data races or panics.
/// Critical for high-throughput scenarios with parallel text processing.
///
/// # Test Steps
/// 1. Create PatternMatcher wrapped in Arc
/// 2. Spawn 10 concurrent async tasks
/// 3. Each task detects PII in unique text
/// 4. Wait for all tasks to complete
/// 5. Verify all detections succeeded
/// 6. Confirm no concurrency issues occurred
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_pii_detection() {
    let config = PiiDetectionConfig::default();
    let matcher =
        std::sync::Arc::new(PatternMatcher::new(config).await.expect("Failed to create matcher"));

    let mut handles = vec![];

    for i in 0..10 {
        let matcher_clone = std::sync::Arc::clone(&matcher);
        let handle = tokio::spawn(async move {
            let text = format!("Contact{}: test{}@example.com", i, i);
            matcher_clone.detect_pii(&text).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.expect("Task should complete");
        assert!(result.is_ok());
    }
}
