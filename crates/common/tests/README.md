# Integration Tests for pulsearc-common

This directory contains comprehensive integration tests for the `pulsearc-common` crate.

## Test Structure

```
tests/
├── README.md                          # This file
├── fixtures/                          # MOCKS (simulated implementations) and FIXTURES (test data builders)
│   ├── mod.rs                         # Clearly categorizes and re-exports all fixtures
│   ├── mock_clock.rs                  # MOCK: Simulated clock (no real time delays)
│   ├── mock_keychain.rs               # MOCK: Simulated keychain (no macOS prompts)
│   ├── mock_oauth_client.rs           # MOCK: Simulated OAuth client (no HTTP requests)
│   ├── mock_encryption_keys.rs        # FIXTURE: Encryption key builders + performance helpers
│   ├── mock_rbac_users.rs             # FIXTURE: RBAC user context builders (admin, user, guest, etc.)
│   ├── mock_rbac_permissions.rs       # FIXTURE: RBAC permission builders (menu, audit, config, etc.)
│   └── mock_rbac_policies.rs          # FIXTURE: RBAC policy builders + RBAC manager fixtures
├── data/                              # Sample entities and data generators (NOT mocks)
│   ├── mod.rs                         # Data module
│   └── sample_entities.rs             # Sample users, projects, configs, email/URL generators
├── helpers/                           # General-purpose test utilities
│   └── mod.rs                         # TempDirFixture, unique_test_id, retry helpers, macros
├── auth_integration.rs                # OAuth 2.0 + PKCE integration tests
├── cache_integration.rs               # Cache with various eviction policies
├── resilience_integration.rs          # Circuit breaker and retry logic
├── validation_integration.rs          # Validation framework tests
├── privacy_integration.rs             # Secure hashing tests
├── lifecycle_integration.rs           # State management tests
├── collections_integration.rs         # Collection types tests
├── security_integration.rs            # Security (encryption, RBAC) tests (28 tests, 11 use fixtures)
└── cross_module_integration.rs        # Cross-module interaction tests
```

## Understanding the Organization

### fixtures/ - Mocks and Test Fixtures

**MOCKS** (files prefixed with `mock_*` that simulate behavior) - Simulated trait implementations:
- `mock_clock.rs` - Simulated clock for testing time-dependent code without delays
- `mock_keychain.rs` - Simulated keychain to avoid macOS Keychain permission prompts
- `mock_oauth_client.rs` - Simulated OAuth client to test auth flows without HTTP requests

**FIXTURES** (files prefixed with `mock_*` that generate test data) - Test data builders and factories:
- `mock_encryption_keys.rs` - Encryption key builders and performance measurement
  - `EncryptionKeyFixture::generate()`, `::fixed(length)`, `::direct_source()`
  - `PerformanceMeasurement::start()`, `::assert_below(duration)`

- `mock_rbac_users.rs` - RBAC user context builders
  - `UserContextFixture::admin()`, `::user()`, `::guest()`, `::empty()`
  - `UserContextBuilder` - Fluent builder with `.with_role()`, `.with_ip()`, etc.
  - `generate_test_users(count, role)` - Batch generation

- `mock_rbac_permissions.rs` - RBAC permission builders
  - `PermissionFixture::menu_view()`, `::audit_delete()`, `::custom(resource, action)`
  - `generate_test_permissions(count)` - Batch generation

- `mock_rbac_policies.rs` - RBAC policy builders and manager fixtures
  - `PolicyFixture::deny_audit_delete()`, `::ip_restricted()`, `::business_hours_only()`
  - `PolicyBuilder` - Fluent builder with `.with_condition()`, `.with_effect()`
  - `RBACFixture::new()` - Initialized RBAC manager for testing
  - Assertion macros: `assert_permission_granted!`, `assert_permission_denied!`

### data/ - Sample Entities

**Sample data structures** (NOT mocks) for testing:
- `TestUser`, `TestProject`, `TestConfig` with `::sample()` and `::batch(n)` methods
- Email/URL/IP address generators: `sample_emails(n)`, `sample_urls(n)`

### helpers/ - General Utilities

**Common test utilities**:
- `TempDirFixture` - Temporary directory with auto-cleanup
- `unique_test_id()` - Generate unique test identifiers
- `retry_test()` - Retry flaky tests
- `assert_error_contains!` macro

## Test Coverage

### auth_integration.rs
- PKCE challenge generation and validation
- OAuth state generation and validation
- Token management with keychain integration
- Token expiration checking
- Concurrent token access

### cache_integration.rs
- LRU, LFU, FIFO, and Random eviction policies
- TTL-based expiration
- Combined TTL + eviction strategies
- Cache statistics tracking
- Concurrent cache access
- Async cache operations

### resilience_integration.rs
- Retry with exponential/linear/fixed backoff
- Custom retry policies
- Circuit breaker state transitions
- Circuit breaker metrics
- Combined circuit breaker + retry patterns
- Different jitter strategies

### validation_integration.rs
- Email, URL, and IP address validation
- String length and pattern validation
- Range and collection validation
- Nested validation contexts
- Batch validation
- Validation error handling

### privacy_integration.rs
- Secure domain hashing with SHA-256/384/512
- Salt rotation
- Multiple domain hashing
- Hash determinism verification

### lifecycle_integration.rs
- State manager operations
- Managed state with Arc<RwLock<T>>
- Concurrent state modifications
- Complex type state management

### cross_module_integration.rs
- Cache + Validation workflows
- Resilience + Auth patterns
- Cache + Privacy integration
- Complete multi-module workflows
- Concurrent cross-module operations

### security_integration.rs
- Encryption key lifecycle and rotation
- Key caching and security event handling
- RBAC with wildcard permissions
- Dynamic policy evaluation
- Role hierarchy and assignment
- Encryption with RBAC-controlled access
- Trait implementations (AccessControl)
- Performance testing (28 tests total, 11 using fixtures)

## Known Issues & TODO

### ~~API Mismatches~~ ✅ **ALL RESOLVED**

All API issues have been verified and resolved:

1. **RetryConfig API** ✅ **RESOLVED**
   - Tests correctly use `RetryConfig::new()` which returns a builder
   - Builder pattern verified working in `resilience/retry.rs`
   - Affected files: `resilience_integration.rs`, `cross_module_integration.rs`

2. **CircuitBreakerConfig API** ✅ **RESOLVED**
   - Tests correctly use `CircuitBreakerConfig::builder()` pattern
   - Implementation verified in `resilience/circuit_breaker.rs`
   - Affected files: `resilience_integration.rs`, `cross_module_integration.rs`

3. **TokenManager API** ✅ **RESOLVED**
   - Methods correctly named: `store_tokens()`, `get_tokens()`, `get_access_token()`
   - `TokenSet` structure is properly defined in `auth/types.rs`
   - Affected files: `auth_integration.rs`, `cross_module_integration.rs`

4. **OAuthConfig API** ✅ **RESOLVED**
   - Constructor correctly uses: `domain`, `client_id`, `redirect_uri`, `scopes`, `audience`
   - Implementation verified in `auth/types.rs`
   - Affected files: `auth_integration.rs`

5. **AsyncCache API** ✅ **RESOLVED**
   - API correctly uses `insert()`, `get()`, `remove()`, `clear()`
   - Implementation verified in `cache/mod.rs`
   - Affected files: `cache_integration.rs`

6. **KeychainProvider API** ✅ **RESOLVED**
   - Methods correctly use: `store_tokens()`, `retrieve_tokens()`, `delete_tokens()`, `has_tokens()`
   - Works properly with `Arc<KeychainProvider>`
   - Affected files: Multiple test files

## API Verification Status

✅ **ALL APIS VERIFIED** - Integration tests use correct APIs:

1. **RetryConfig** - Builder pattern working correctly
2. **CircuitBreakerConfig** - Builder pattern working correctly
3. **TokenManager** - All methods correctly named and used
4. **OAuthConfig** - Constructor and fields match implementation
5. **AsyncCache** - All async methods working as expected
6. **KeychainProvider** - All trait methods working with Arc
7. **ManagedState** - Used correctly in lifecycle tests (no StateRegistry)
8. **CollectionValidator** - Now supports per-item validation (✅ P1 complete)

## Notes on Non-Existent APIs

The following APIs were referenced in earlier documentation but do not exist:

- **StateRegistry**: Tests correctly use `ManagedState` instead
- **PatternMatcher** (in privacy module): Privacy module provides `SecureHasher` for domain hashing, not pattern matching

## Running Tests

Once fixed, run tests with:

```bash
# Run all integration tests
cargo test -p pulsearc-common --tests

# Run specific test file
cargo test -p pulsearc-common --test auth_integration

# Run specific test
cargo test -p pulsearc-common --test auth_integration test_pkce_challenge_generation

# Run with output
cargo test -p pulsearc-common --tests -- --nocapture
```

## Usage Examples

### Using Mocks (Simulated Implementations)

```rust
// Import mocks from fixtures
mod fixtures;
use fixtures::{MockClock, MockKeychain, MockOAuthClient};

// Mock clock - test time-dependent code without delays
let clock = MockClock::new();
clock.advance_seconds(100);  // Skip forward 100 seconds
let time = clock.now();

// Mock keychain - test credential storage without macOS prompts
let keychain = MockKeychain::new();
keychain.store_tokens("user@example.com", &tokens).await?;

// Mock OAuth client - test auth flows without HTTP requests
let oauth = MockOAuthClient::new();
oauth.set_next_token_response(Ok(tokens));
```

### Using Fixtures (Test Data Builders)

```rust
// All fixtures are re-exported from fixtures::mod, so you can import everything at once
mod fixtures;
use fixtures::*;

// Or import specific fixture modules
use fixtures::mock_encryption_keys::*;
use fixtures::mock_rbac_users::*;
use fixtures::mock_rbac_permissions::*;
use fixtures::mock_rbac_policies::*;

// Encryption keys
let key = EncryptionKeyFixture::generate();
let key_source = EncryptionKeyFixture::direct_source("test");

// Pre-configured users
let admin = UserContextFixture::admin("admin1");
let user = UserContextFixture::user("user1");
let guest = UserContextFixture::guest();

// Custom users with builder
let custom = UserContextBuilder::new("user2")
    .with_role("auditor")
    .with_ip("192.168.1.100")
    .with_attribute("department", "security")
    .build();

// RBAC permissions
let view_perm = PermissionFixture::menu_view();
let delete_perm = PermissionFixture::audit_delete();
let custom_perm = PermissionFixture::custom("resource", "action");

// RBAC policies
let deny_policy = PolicyFixture::deny_audit_delete();
let ip_policy = PolicyFixture::ip_restricted(
    vec!["192.168.1.100".to_string()],
    vec!["admin:access".to_string()]
);

// RBAC manager
let mut rbac_fixture = RBACFixture::new();
let rbac = rbac_fixture.manager_mut();

// Performance measurement
let perf = PerformanceMeasurement::start("my operation");
// ... perform operation ...
perf.assert_below(Duration::from_millis(100));

// Batch generation
let users = generate_test_users(10, "user");
let perms = generate_test_permissions(100);
```

### Using Sample Data

```rust
// Import sample entities
mod data;
use data::sample_entities::*;

// Generate batch data
let users = TestUser::batch(10);
let projects = TestProject::batch(5);
let emails = sample_emails(20);
```

### Using Test Utilities

```rust
// Import helpers
mod helpers;
use helpers::*;

// Generate unique test ID
let test_id = unique_test_id("mytest");

// Create temp directory
let temp = TempDirFixture::new();
let file_path = temp.file_path("test.db");
```

## Contributing

When adding new integration tests:

1. Follow the existing test structure and naming conventions
2. Use the test helpers and fixtures from `helpers/`
3. Test both success and failure paths
4. Test concurrent access where applicable
5. Clean up resources (keychain entries, temp files) in tests
6. Add descriptive doc comments to test functions
7. Group related tests in the same file

## Integration with CI

These tests should run in CI as part of the standard test suite:

```yaml
# .github/workflows/ci.yml
- name: Run integration tests
  run: cargo test -p pulsearc-common --tests
```

## Performance Considerations

Some tests involve:
- Real keychain operations (may be slow on some platforms)
- Sleep/delay operations for TTL testing
- Concurrent task spawning

Use `#[tokio::test(flavor = "multi_thread")]` for async tests that benefit from parallelism.

## Next Steps

1. Fix API mismatches listed above
2. Ensure all tests compile without errors
3. Run tests and verify they pass
4. Add any missing test coverage
5. Integrate into CI pipeline
6. Consider adding property-based tests with `proptest` crate
