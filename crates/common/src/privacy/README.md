# Privacy Module - Portable Core

Portable, domain-independent privacy functionality for secure hashing, PII detection, pattern matching, and data sanitization.

## Overview

This module provides comprehensive privacy-preserving operations that can be used across different domains and applications. It includes cryptographic hashing for anonymization, sophisticated PII (Personally Identifiable Information) detection, and data sanitization capabilities designed for regulatory compliance (GDPR, CCPA, HIPAA).

## Features

- **Secure Hashing**: Cryptographic hashing with salt management and algorithm flexibility
- **PII Detection**: Multi-method PII detection (regex, ML models, heuristics)
- **Pattern Matching**: Sophisticated pattern matching for sensitive data
- **Performance Metrics**: Comprehensive metrics for hash operations and detection
- **Compliance Support**: GDPR, CCPA, HIPAA compliance frameworks
- **Data Sanitization**: Redaction strategies for sensitive information

## Architecture

```text
┌───────────────────────────────────┐
│       Privacy Module              │
├───────────────────────────────────┤
│                                   │
│  ┌───────────────────────────┐   │
│  │   Hash (hash/)            │   │
│  │  • SecureHasher           │   │
│  │  • HashAlgorithm          │   │
│  │  • HashConfig             │   │
│  │  • HashMetrics            │   │
│  │  • ComplianceMetrics      │   │
│  └───────────────────────────┘   │
│                                   │
│  ┌───────────────────────────┐   │
│  │   Patterns (patterns/)    │   │
│  │  • PatternMatcher         │   │
│  │  • PiiDetectionConfig     │   │
│  │  • PiiType                │   │
│  │  • DetectionResult        │   │
│  │  • RedactionStrategy      │   │
│  └───────────────────────────┘   │
│                                   │
└───────────────────────────────────┘
```

## Components

### 1. Secure Hashing (`hash/`)

Cryptographic hashing infrastructure for privacy-preserving data anonymization.

**SecureHasher** - Main hashing interface with:
- Multiple algorithm support (SHA-256, SHA-512, Blake3)
- Automatic salt generation and management
- HMAC-based hashing for additional security
- Performance and compliance metrics tracking

**HashAlgorithm** - Supported algorithms:
- `Sha256` - SHA-256 (default, good balance)
- `Sha512` - SHA-512 (higher security)
- `Blake3` - Blake3 (fastest, modern)

**HashConfig** - Configuration for hash operations:
- Algorithm selection
- Salt management
- Encoding preferences
- Pepper (additional secret key) support

**HashMetricsCollector** - Comprehensive metrics:
- Performance metrics (duration, throughput)
- Salt metrics (generation, reuse)
- Security metrics (algorithm usage)
- Compliance metrics (salt age, rotation status)

### 2. PII Detection and Pattern Matching (`patterns/`)

Sophisticated PII detection with multiple detection methods and compliance support.

**PatternMatcher** - Main PII detection interface:
- Regex-based pattern matching
- ML model integration
- Heuristic detection
- Confidence scoring
- Context-aware detection

**PiiType** - Detectable PII types:
- Email addresses
- Phone numbers
- Social Security Numbers (SSN)
- Credit card numbers
- IP addresses
- Physical addresses
- Names (with context)
- Dates of birth
- Medical record numbers
- Custom patterns

**DetectionResult** - Detection output:
- PiiEntity with type and location
- Confidence score
- Detection method used
- Context information
- Redaction suggestions

**PiiDetectionConfig** - Configuration:
- Enabled PII types
- Confidence thresholds
- Detection methods
- ML model settings
- Custom patterns

**RedactionStrategy** - Sanitization methods:
- Full redaction (complete removal)
- Partial redaction (show last 4 digits)
- Masking (replace with asterisks)
- Hashing (one-way anonymization)
- Encryption (reversible protection)

## Usage Examples

### Secure Hashing

```rust
use agent::common::privacy::{SecureHasher, HashAlgorithm, HashConfig};

// Create hasher with default configuration
let hasher = SecureHasher::new(HashConfig::default())?;

// Hash a string
let hashed = hasher.hash_string("user@example.com")?;
println!("Hashed: {}", hashed);

// Hash with custom salt
let salt = hasher.generate_salt()?;
let hashed_with_salt = hasher.hash_with_salt("user@example.com", &salt)?;

// Hash multiple values with same salt (for consistency)
let values = vec!["user1@example.com", "user2@example.com"];
let hashed_values = hasher.hash_batch(&values)?;

// Verify hash
let is_match = hasher.verify("user@example.com", &hashed)?;
assert!(is_match);
```

### Custom Hash Configuration

```rust
use agent::common::privacy::{SecureHasher, HashAlgorithm, HashConfig};

// Create configuration
let config = HashConfig::builder()
    .algorithm(HashAlgorithm::Blake3)
    .pepper("my_application_secret")
    .encoding("base64")
    .build()?;

// Create hasher with custom config
let hasher = SecureHasher::new(config)?;

// Hash with custom configuration
let hashed = hasher.hash_string("sensitive_data")?;
```

### PII Detection

```rust
use agent::common::privacy::{PatternMatcher, PiiDetectionConfig, PiiType};

// Create pattern matcher with default config
let config = PiiDetectionConfig::default()
    .enable_type(PiiType::Email)
    .enable_type(PiiType::PhoneNumber)
    .enable_type(PiiType::CreditCard)
    .set_confidence_threshold(0.8);

let matcher = PatternMatcher::new(config)?;

// Detect PII in text
let text = "Contact me at john@example.com or call 555-1234";
let results = matcher.detect(text)?;

for entity in results {
    println!("Found: {:?} at position {} with confidence {}",
        entity.pii_type,
        entity.location.start,
        entity.confidence
    );
}
```

### PII Redaction

```rust
use agent::common::privacy::{
    PatternMatcher,
    PiiDetectionConfig,
    RedactionStrategy
};

// Configure matcher with redaction
let config = PiiDetectionConfig::default()
    .enable_type(PiiType::Email)
    .enable_type(PiiType::PhoneNumber)
    .set_redaction_strategy(RedactionStrategy::Partial);

let matcher = PatternMatcher::new(config)?;

// Detect and redact PII
let text = "My email is john@example.com and phone is 555-1234";
let redacted = matcher.redact(text)?;

println!("Redacted: {}", redacted);
// Output: "My email is j***@example.com and phone is ***-1234"
```

### Advanced PII Detection with Context

```rust
use agent::common::privacy::{
    PatternMatcher,
    PiiDetectionConfig,
    DetectionMethod,
    AnalysisContext
};

// Configure with multiple detection methods
let config = PiiDetectionConfig::builder()
    .add_method(DetectionMethod::Regex)
    .add_method(DetectionMethod::Heuristic)
    .add_method(DetectionMethod::MachineLearning)
    .enable_context_analysis(true)
    .confidence_threshold(0.75)
    .build()?;

let matcher = PatternMatcher::new(config)?;

// Provide context for better detection
let context = AnalysisContext::builder()
    .domain("healthcare")
    .language("en")
    .add_custom_pattern("medical_id", r"\d{8}-\d{4}")
    .build();

// Detect with context
let text = "Patient ID: 12345678-9012, SSN: 123-45-6789";
let results = matcher.detect_with_context(text, &context)?;

for entity in results {
    println!("Detected: {:?} via {:?} method",
        entity.pii_type,
        entity.detection_method
    );
}
```

### Hash Performance Metrics

```rust
use agent::common::privacy::{SecureHasher, HashConfig};

let hasher = SecureHasher::new(HashConfig::default())?;

// Perform operations
for i in 0..1000 {
    hasher.hash_string(&format!("data_{}", i))?;
}

// Get metrics snapshot
let metrics = hasher.metrics_snapshot()?;

println!("Hash operations: {}", metrics.performance.total_operations);
println!("Average duration: {:?}", metrics.performance.average_duration);
println!("Salts generated: {}", metrics.salt.total_generated);
println!("Compliance status: {:?}", metrics.compliance.status);
```

### PII Detection Metrics

```rust
use agent::common::privacy::{PatternMatcher, PiiDetectionConfig};

let matcher = PatternMatcher::new(PiiDetectionConfig::default())?;

// Perform detections
let texts = vec![
    "Email: user@example.com",
    "Phone: 555-1234",
    "SSN: 123-45-6789"
];

for text in texts {
    matcher.detect(text)?;
}

// Get metrics
let metrics = matcher.metrics_snapshot()?;

println!("Total detections: {}", metrics.operational.total_detections);
println!("Detection rate: {:.2}%", metrics.quality.detection_accuracy);
println!("Average confidence: {:.2}", metrics.quality.average_confidence);
```

### Compliance Validation

```rust
use agent::common::privacy::{
    PatternMatcher,
    PiiDetectionConfig,
    ComplianceFramework
};

// Configure for GDPR compliance
let config = PiiDetectionConfig::builder()
    .compliance_framework(ComplianceFramework::GDPR)
    .enable_audit_logging(true)
    .require_consent_tracking(true)
    .data_retention_days(30)
    .build()?;

let matcher = PatternMatcher::new(config)?;

// Validate compliance
let violations = matcher.check_compliance(text)?;

if !violations.is_empty() {
    for violation in violations {
        println!("Violation: {:?} - {}",
            violation.severity,
            violation.message
        );
    }
}
```

## API Reference

### SecureHasher Methods

**Core Operations**
- `new(config: HashConfig) -> Result<Self>` - Create hasher
- `hash_string(data: &str) -> Result<String>` - Hash string
- `hash_bytes(data: &[u8]) -> Result<Vec<u8>>` - Hash bytes
- `hash_with_salt(data: &str, salt: &[u8]) -> Result<String>` - Hash with custom salt
- `hash_batch(data: &[&str]) -> Result<Vec<String>>` - Batch hash
- `verify(data: &str, hash: &str) -> Result<bool>` - Verify hash

**Salt Management**
- `generate_salt() -> Result<Vec<u8>>` - Generate random salt
- `rotate_salt() -> Result<()>` - Rotate current salt

**Metrics**
- `metrics_snapshot() -> Result<HashMetricsSnapshot>` - Get metrics

### PatternMatcher Methods

**Detection**
- `new(config: PiiDetectionConfig) -> Result<Self>` - Create matcher
- `detect(text: &str) -> Result<Vec<PiiEntity>>` - Detect PII
- `detect_with_context(text: &str, context: &AnalysisContext) -> Result<Vec<PiiEntity>>` - Detect with context

**Redaction**
- `redact(text: &str) -> Result<String>` - Redact detected PII
- `redact_with_strategy(text: &str, strategy: RedactionStrategy) -> Result<String>` - Custom redaction

**Compliance**
- `check_compliance(text: &str) -> Result<Vec<ComplianceViolation>>` - Check compliance
- `validate_data_handling() -> Result<ComplianceStatus>` - Validate practices

**Metrics**
- `metrics_snapshot() -> Result<MetricsSnapshot>` - Get metrics

## Testing

### Unit Tests

```bash
# Run all privacy tests
cargo test --package agent --lib common::privacy

# Run specific module tests
cargo test --package agent --lib common::privacy::hash
cargo test --package agent --lib common::privacy::patterns
```

### Integration Tests

```bash
# Run privacy integration tests
cargo test --package agent --test privacy_integration
```

### Example Test

```rust
use agent::common::privacy::{SecureHasher, HashConfig};

#[test]
fn test_hash_consistency() {
    let hasher = SecureHasher::new(HashConfig::default()).unwrap();

    let data = "test@example.com";
    let hash1 = hasher.hash_string(data).unwrap();
    let hash2 = hasher.hash_string(data).unwrap();

    // Same input should produce same hash with same salt
    assert_eq!(hash1, hash2);

    // Verify should succeed
    assert!(hasher.verify(data, &hash1).unwrap());
}
```

## Best Practices

### Hashing

1. **Use Strong Algorithms**: Prefer SHA-256 or Blake3 for new implementations
2. **Always Use Salt**: Never hash without salt for privacy-sensitive data
3. **Rotate Salts**: Implement salt rotation for long-lived applications
4. **Use Pepper**: Add application-level pepper for additional security
5. **Monitor Performance**: Track metrics to detect performance degradation

### PII Detection

1. **Configure Appropriately**: Enable only needed PII types to reduce false positives
2. **Set Confidence Thresholds**: Balance detection accuracy vs false positives
3. **Use Context**: Provide context for better detection accuracy
4. **Implement Redaction**: Always redact PII in logs and non-essential storage
5. **Audit Regularly**: Review detection metrics and compliance status

### Compliance

1. **Choose Framework**: Configure for your compliance requirements (GDPR, CCPA, HIPAA)
2. **Enable Auditing**: Track all PII operations for compliance reports
3. **Implement Retention**: Configure appropriate data retention periods
4. **Validate Regularly**: Run compliance checks periodically
5. **Document Violations**: Log and address compliance violations promptly

## Dependencies

```toml
[dependencies]
sha2 = "0.10"
blake3 = "1.5"
base64 = "0.22"
regex = "1.10"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
```

## Related Modules

- **agent/common/security**: Encryption and RBAC
- **agent/common/observability**: Error handling and metrics
- **agent/common/validation**: Input validation
- **agent/storage**: Encrypted storage with privacy features

## Roadmap

- [ ] Add differential privacy support
- [ ] Implement k-anonymity algorithms
- [ ] Add more ML models for PII detection
- [ ] Support additional compliance frameworks (PIPEDA, LGPD)
- [ ] Add synthetic data generation for testing

## References

- [GDPR](https://gdpr.eu/) - General Data Protection Regulation
- [CCPA](https://oag.ca.gov/privacy/ccpa) - California Consumer Privacy Act
- [HIPAA](https://www.hhs.gov/hipaa/) - Health Insurance Portability and Accountability Act

## License

See the root LICENSE file for licensing information.
