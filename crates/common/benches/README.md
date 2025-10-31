# Sync Benchmarks

Comprehensive performance benchmarks for the PulseArc sync module covering queue
throughput, batch processing, retry orchestration, and retry budgets.

## Running Benchmarks

```bash
cargo bench --bench sync_bench -p pulsearc-common --features runtime
```

### Run Specific Benchmark Groups

```bash
# Queue enqueue/dequeue paths across configurations
cargo bench --bench sync_bench -p pulsearc-common --features runtime -- sync_queue_enqueue_dequeue

# Batch enqueue/dequeue performance
cargo bench --bench sync_bench -p pulsearc-common --features runtime -- sync_queue_batch_processing

# Failure handling and retry scheduling impact
cargo bench --bench sync_bench -p pulsearc-common --features runtime -- sync_queue_failure_path

# Retry strategy async execution profiles
cargo bench --bench sync_bench -p pulsearc-common --features runtime -- sync_retry_strategy_async

# Retry budget acquisition and refill behavior
cargo bench --bench sync_bench -p pulsearc-common --features runtime -- sync_retry_budget
```

## Benchmark Coverage

### Queue Operations
- High-volume enqueue/dequeue with default and high-performance configs
- Batch enqueue/dequeue throughput at multiple batch sizes
- Failure path handling with metrics updates and cleanup

### Retry Infrastructure
- Immediate, transient, and persistent failure retry scenarios
- Token budget acquisition, refill, and multi-token requests

Results mirror Criterion output patterns used across the workspace and HTML
reports are written to `target/criterion/`.

---

# Lifecycle Benchmarks

Benchmarks for lifecycle management utilities including shared state wrappers,
managed state orchestration, atomic counters, state registries, and controller
flows.

## Running Benchmarks

```bash
cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime
```

### Run Specific Benchmark Groups

```bash
# Mixed read/write contention on SharedState
cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime -- lifecycle_shared_state

# ManagedState access and builder creation patterns
cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime -- lifecycle_managed_state

# Atomic counter mutation paths
cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime -- lifecycle_atomic_counter

# Manager controller initialization/shutdown orchestration
cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime -- lifecycle_manager_controller

# StateRegistry registration and read access
cargo bench --bench lifecycle_bench -p pulsearc-common --features runtime -- lifecycle_state_registry
```

## Benchmark Coverage

- SharedState concurrent read/write paths plus timeout accessors
- ManagedState mutation and cloning, plus builder construction overhead
- AtomicCounter increment/add/reset behaviors under varying contention
- ManagerController startup/shutdown orchestration with mock managers
- StateRegistry registration and concurrent reader workloads

---

# Security Benchmarks

Comprehensive performance benchmarks for the PulseArc security module.

## Running Benchmarks

### Run All Security Benchmarks

```bash
cargo bench --bench security_bench -p pulsearc-common --features pulsearc-common/platform
```

### Run Specific Benchmark Groups

```bash
# Key generation benchmarks
cargo bench --bench security_bench -p pulsearc-common --features pulsearc-common/platform -- key_generation

# Encryption benchmarks
cargo bench --bench security_bench -p pulsearc-common --features pulsearc-common/platform -- encryption

# SecureString benchmarks
cargo bench --bench security_bench -p pulsearc-common --features pulsearc-common/platform -- secure_string

# RBAC benchmarks
cargo bench --bench security_bench -p pulsearc-common --features pulsearc-common/platform -- rbac
```

### Test Benchmarks Without Running Full Suite

```bash
cargo bench --bench security_bench -p pulsearc-common --features pulsearc-common/platform -- --test
```

## Benchmark Coverage

### Key Operations

- **Key Generation**
  - `generate_encryption_key`: 64-character secure key generation
  - `symmetric_key_generation`: 32-byte AES-256 key generation

- **Key Derivation**
  - `argon2_password_derivation`: Password-based key derivation using Argon2

- **Key Rotation**
  - `rotate_encryption_key`: Key rotation performance
  - `key_fingerprint_generation`: SHA-256 fingerprint generation

### Encryption Operations

- **Encrypt/Decrypt** (tested with multiple payload sizes)
  - Small (16 bytes)
  - Medium (1 KB)
  - Large (64 KB)
  - Extra Large (1 MB)

- **String Operations**
  - `encrypt_to_string`: Encrypt and base64 encode
  - `decrypt_from_string`: Decode and decrypt

### SecureString Operations

- `create_secure_string`: Secure string creation
- `constant_time_eq_same`: Constant-time comparison (matching strings)
- `constant_time_eq_different`: Constant-time comparison (different strings)
- `regular_eq_comparison`: Standard equality comparison

### RBAC Operations

- **Manager Operations**
  - `create_rbac_manager`: RBAC manager initialization
  - `create_role`: Role creation
  - `register_permission`: Permission registration

- **Permission Checks**
  - `check_permission_granted`: Permission check (granted)
  - `check_permission_denied`: Permission check (denied)

- **Policy Evaluation**
  - `add_time_based_policy`: Time-based policy creation
  - `add_ip_based_policy`: IP-based policy creation
  - `evaluate_time_policy`: Policy evaluation performance

## Results

Benchmark results are saved to:
- Console output (summary statistics)
- HTML reports in `target/criterion/`
- Detailed data in `target/criterion/<benchmark_name>/`

### Viewing HTML Reports

```bash
open target/criterion/report/index.html
```

## Performance Considerations

### Key Derivation

Argon2 is intentionally slow for security (reduces sample size in benchmarks).
Typical performance: ~100-200ms per key derivation.

### Encryption

- Small payloads (< 1KB): ~1-5 microseconds
- Large payloads (> 1MB): ~100-500 microseconds
- Overhead is primarily from AES-GCM operations and nonce generation

### RBAC

- Manager creation: ~100-500 nanoseconds
- Permission checks: ~1-10 microseconds (with caching)
- Policy evaluation: ~5-50 microseconds (depends on complexity)

## CI Integration

These benchmarks can be run in CI with:

```bash
cargo xtask bench
```

Or directly:

```bash
cargo bench --workspace --features pulsearc-common/platform
```

## Dependencies

- `criterion 0.5`: Benchmarking framework with statistical analysis
- Features: `html_reports`, `async_tokio`

## Notes

- Benchmarks use the `platform` feature flag (required for security module)
- Some benchmarks create fresh managers per iteration for isolation
- RBAC benchmarks use unique identifiers to avoid conflicts
- Results may vary based on system load and hardware

---

# Error Handling Benchmarks

Comprehensive performance benchmarks for the PulseArc error handling system.

## Running Benchmarks

### Run All Error Benchmarks

```bash
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation
```

### Run Specific Benchmark Groups

```bash
# Error construction benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- error_construction

# Error display formatting
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- error_display

# Error classification benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- error_classification

# Error conversion benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- error_conversions

# Structured logging benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- structured_logging

# Error cloning benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- error_cloning

# Fluent API benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- fluent_api

# ErrorSeverity benchmarks
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- error_severity

# Realistic usage patterns
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- realistic_patterns
```

### Test Benchmarks Without Running Full Suite

```bash
cargo bench --bench error_bench -p pulsearc-common --features pulsearc-common/foundation -- --test
```

## Benchmark Coverage

### Error Construction

#### Simple Constructors
- `config`: Configuration error without field
- `lock`: Lock acquisition error without resource
- `serialization`: Serialization error without format
- `persistence`: Persistence error without operation
- `not_found`: Resource not found without identifier
- `unauthorized`: Unauthorized operation without permission
- `internal`: Internal error without context
- `validation`: Validation error without value

#### Complex Constructors
- `config_field`: Configuration error with specific field
- `lock_resource`: Lock error with specific resource
- `circuit_breaker_with_retry`: Circuit breaker with retry delay
- `serialization_format`: Serialization error with format (JSON/TOML)
- `persistence_op`: Persistence error with operation type
- `rate_limit_detailed`: Rate limit with limit, window, and retry delay
- `timeout`: Timeout error with operation name and duration
- `backend`: Backend error with service, message, and retryability
- `validation_with_value`: Validation error with invalid value
- `not_found_with_id`: Not found error with identifier
- `unauthorized_with_perm`: Unauthorized with required permission
- `internal_with_context`: Internal error with context
- `task_cancelled_with_reason`: Task cancellation with reason
- `async_timeout`: Async operation timeout

### Error Display Formatting

Tests formatting performance for all error variants with different complexity levels:
- Simple messages
- Messages with embedded context
- Messages with optional fields
- Messages with durations and numeric values

### ErrorClassification Trait

#### Retryability Checks
- `is_retryable()` for retryable errors (circuit breaker, rate limit, timeout, lock, backend)
- `is_retryable()` for non-retryable errors (config, validation, not found, internal, unauthorized)

#### Severity Assessment
- `severity()` for all error variants
- Returns: Info, Warning, Error, or Critical

#### Critical Status
- `is_critical()` for critical errors (internal errors)
- `is_critical()` for non-critical errors (all others)

#### Retry Delay
- `retry_after()` returning `Some(Duration)` (circuit breaker, rate limit)
- `retry_after()` returning `None` (non-retryable errors)

### Error Conversions

Tests automatic conversion from standard library error types:
- `From<std::io::Error>` → `CommonError::Persistence`
- `From<serde_json::Error>` → `CommonError::Serialization`
- `From<toml::de::Error>` → `CommonError::Serialization` (foundation feature)

### Structured Logging

Tests `as_tracing_fields()` method for all error variants:
- Returns vector of (key, value) pairs for structured logging
- Includes error type, message, and variant-specific fields
- Compatible with `tracing` crate integration

### Error Cloning

Tests clone performance for various error complexities:
- Simple errors (single field)
- Complex errors (multiple fields, optional values)
- Errors with embedded Duration values

### Fluent API

Tests method chaining for adding context:
- `with_additional_context()` single call
- `with_additional_context()` chained multiple times

### ErrorSeverity Operations

- `Display` formatting
- Comparison operators (`>`, `<`, `==`)
- Equality checks

### Realistic Usage Patterns

#### Error Lifecycle (Retryable)
1. Create timeout error
2. Check if retryable
3. Get severity
4. Format for display

#### Error Lifecycle (Permanent)
1. Create validation error
2. Check if retryable
3. Get severity
4. Check if critical
5. Format for display

#### Retry Decision with Delay
1. Create circuit breaker error with retry delay
2. Check retryability
3. Get retry delay

#### Error with Structured Logging
1. Create timeout error
2. Generate structured log fields
3. Get severity

#### Error Conversion Pipeline
1. Create `io::Error`
2. Convert to `CommonError`
3. Check retryability
4. Format for display

## Results

Benchmark results are saved to:
- Console output (summary statistics)
- HTML reports in `target/criterion/`
- Detailed data in `target/criterion/<benchmark_name>/`

### Viewing HTML Reports

```bash
open target/criterion/report/index.html
```

## Performance Characteristics

### Error Construction

- **Simple constructors**: ~5-20 nanoseconds
  - Single string allocation
  - Minimal overhead

- **Complex constructors**: ~10-50 nanoseconds
  - Multiple field allocations
  - Option wrapping overhead

- **With Duration**: Additional ~5-10 nanoseconds
  - Duration struct creation overhead

### Error Display

- **Simple errors**: ~50-200 nanoseconds
  - Basic string formatting
  - No optional field checks

- **Complex errors**: ~100-500 nanoseconds
  - Multiple format! macro invocations
  - Optional field formatting with if-let
  - Duration formatting

### ErrorClassification

- **is_retryable()**: ~1-5 nanoseconds
  - Simple pattern matching
  - No allocations
  - Branch prediction friendly

- **severity()**: ~1-5 nanoseconds
  - Pattern matching returns enum
  - Zero allocations

- **is_critical()**: ~1-5 nanoseconds
  - Pattern matching with boolean return
  - Optimizes to constant in many cases

- **retry_after()**: ~1-5 nanoseconds
  - Option<Duration> return
  - Copy semantics (Duration is Copy)

### Error Conversions

- **From<io::Error>**: ~20-100 nanoseconds
  - String allocation from error message
  - Error variant construction

- **From<serde_json::Error>**: ~30-150 nanoseconds
  - String allocation
  - Format specification ("JSON")
  - Error variant construction

### Structured Logging

- **as_tracing_fields()**: ~50-500 nanoseconds
  - Vec allocation for field pairs
  - Multiple string allocations
  - String conversions (to_string())
  - Duration to milliseconds conversion

### Error Cloning

- **Simple errors**: ~10-30 nanoseconds
  - String cloning (Arc increment typically)
  - Struct copy

- **Complex errors**: ~20-100 nanoseconds
  - Multiple string clones
  - Option handling
  - Duration copy (trivial)

## CI Integration

These benchmarks can be run in CI with:

```bash
cargo xtask bench
```

Or directly:

```bash
cargo bench --workspace --features pulsearc-common/foundation
```

## Dependencies

- `criterion 0.5`: Benchmarking framework with statistical analysis
- Features: `html_reports`

## Notes

- Error benchmarks use the `foundation` feature flag (more lightweight than `platform`)
- All benchmarks use `black_box()` to prevent compiler optimizations
- Realistic patterns simulate common error handling workflows
- Results represent performance in hot paths; cold path performance may vary
- Memory allocation patterns may affect results on systems with memory pressure
