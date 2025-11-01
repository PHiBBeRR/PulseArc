# Testing Utilities

Comprehensive test helpers, mocks, and fixtures for writing robust tests.

## Overview

The testing module provides a complete suite of utilities for writing tests:

- **Assertions**: Custom assertion macros for complex scenarios
- **Fixtures**: Random test data generators
- **Builders**: Fluent API for constructing test objects
- **Matchers**: Reusable assertion logic
- **Mocks**: Mock implementations of common traits
- **Temp Files**: RAII temporary file/directory management
- **Time Mocking**: Control time in tests (re-exported from sync)

## Features

- ✅ Custom assertion macros
- ✅ Random data generation
- ✅ Builder pattern for test data
- ✅ Mock HTTP client and storage
- ✅ Temporary file management with auto-cleanup
- ✅ Time mocking for deterministic tests
- ✅ Pattern matching utilities
- ✅ Thread-safe mocks

## Quick Start

### Custom Assertions

```rust
use agent::common::testing::{assert_error_contains, assert_eventually};
use std::time::Duration;

#[test]
fn test_error_message() {
    let result: Result<(), String> = Err("Connection timeout".to_string());
    assert_error_contains!(result, "timeout");
}

#[test]
fn test_eventually() {
    let mut counter = 0;
    assert_eventually!(Duration::from_secs(1), || {
        counter += 1;
        counter >= 5
    });
}
```

### Random Test Data

```rust
use agent::common::testing::fixtures::{random_string, random_email, random_u64};

#[test]
fn test_with_random_data() {
    let user_id = random_string(10);
    let email = random_email();
    let timestamp = random_u64();
    
    assert_eq!(user_id.len(), 10);
    assert!(email.contains('@'));
}
```

### Test Builders

```rust
use agent::common::testing::builders::{TestBuilder, StringBuilder, UserBuilder};

#[test]
fn test_with_builder() {
    let user = UserBuilder::new()
        .id("123")
        .name("Alice")
        .email("alice@example.com")
        .age(30)
        .build();
    
    assert_eq!(user.get("name"), Some(&"Alice".to_string()));
}

#[test]
fn test_string_builder() {
    let sql = StringBuilder::new()
        .append("SELECT * FROM users")
        .append(" WHERE ")
        .append("active = true")
        .build();
    
    assert!(sql.contains("SELECT"));
}
```

### Matchers

```rust
use agent::common::testing::matchers::{contains_string, matches_pattern, in_range};

#[test]
fn test_matchers() {
    assert!(contains_string("Hello World", "World").is_ok());
    assert!(matches_pattern("test@example.com", r"^\w+@\w+\.\w+$").is_ok());
    assert!(in_range(5, 1, 10).is_ok());
}
```

### Mock HTTP Client

```rust
use agent::common::testing::mocks::MockHttpClient;

#[test]
fn test_http_client() {
    let client = MockHttpClient::new();
    client.add_response("https://api.example.com/users", 200, r#"{"users": []}"#);
    
    let response = client.get("https://api.example.com/users").unwrap();
    assert_eq!(response.status, 200);
    assert_eq!(client.request_count("https://api.example.com/users"), 1);
}
```

### Mock Storage

```rust
use agent::common::testing::mocks::MockStorage;

#[test]
fn test_storage() {
    let storage = MockStorage::new();
    
    storage.set("key1", "value1").unwrap();
    assert_eq!(storage.get("key1").unwrap(), Some("value1".to_string()));
    
    storage.delete("key1").unwrap();
    assert!(storage.is_empty());
}
```

### Temporary Files

```rust
use agent::common::testing::temp::{TempFile, TempDir};

#[test]
fn test_with_temp_file() {
    let temp_file = TempFile::with_contents("test", "txt", "hello world").unwrap();
    
    let contents = temp_file.read().unwrap();
    assert_eq!(contents, "hello world");
    
    // File automatically deleted when temp_file goes out of scope
}

#[test]
fn test_with_temp_dir() {
    let temp_dir = TempDir::new("test").unwrap();
    
    let file_path = temp_dir.create_file("test.txt", "data").unwrap();
    assert!(file_path.exists());
    
    // Directory and contents automatically deleted when temp_dir goes out of scope
}
```

### Time Mocking

```rust
use agent::common::testing::time::MockClock;
use std::time::Duration;

#[test]
fn test_with_mock_time() {
    let clock = MockClock::new();
    let start = clock.now();
    
    clock.advance(Duration::from_secs(5));
    let end = clock.now();
    
    assert_eq!(end.duration_since(start), Duration::from_secs(5));
}
```

### Async Utilities

```rust
use agent::common::testing::async_utils::{timeout_ok, retry_async, poll_until};
use std::time::Duration;

#[tokio::test]
async fn test_with_timeout() {
    let result = timeout_ok(Duration::from_secs(1), async {
        // Some async operation
        tokio::time::sleep(Duration::from_millis(100)).await;
        42
    }).await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_with_retry() {
    let mut attempts = 0;
    let result = retry_async(3, Duration::from_millis(10), || async {
        attempts += 1;
        if attempts < 3 {
            Err("Not yet")
        } else {
            Ok(42)
        }
    }).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_poll_until() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    let flag = Arc::new(AtomicBool::new(false));
    let flag_clone = flag.clone();

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        flag_clone.store(true, Ordering::SeqCst);
    });

    let result = poll_until(
        Duration::from_secs(1),
        Duration::from_millis(10),
        || async { flag.load(Ordering::SeqCst) }
    ).await;

    assert!(result);
}
```

## API Reference

### Assertions Module

**Macros:**
- `assert_error_contains!(result, substring)` - Assert error message contains substring
- `assert_error_kind!(result, kind)` - Assert error is of specific kind
- `assert_eventually!(timeout, condition)` - Assert condition becomes true within timeout
- `assert_retry_count!(actual, expected)` - Assert retry count matches

**Functions:**
- `assert_approx_eq(actual, expected, epsilon)` - Assert floats are approximately equal
- `assert_duration_in_range(actual, expected, tolerance)` - Assert duration within range
- `assert_contains_all(haystack, needles)` - Assert collection contains all items
- `assert_sorted(items)` - Assert collection is sorted

### Async Utilities Module

**Macros:**
- `assert_eventually_async!(timeout, future)` - Async variant of `assert_eventually!`

**Functions:**
- `timeout_ok(duration, future)` - Run future with timeout, return Result
- `retry_async(max_attempts, initial_delay, operation)` - Retry async operation with exponential backoff
- `poll_until(timeout, interval, condition)` - Poll async condition until true or timeout

### Fixtures Module

**Functions:**
- `random_string(len)` - Generate random alphanumeric string
- `random_email()` - Generate random email address
- `random_u64()` - Generate random u64
- `random_u32()` - Generate random u32
- `random_bool()` - Generate random boolean
- `random_u64_range(min, max)` - Generate random u64 in range
- `random_numeric(len)` - Generate random numeric string
- `random_hex(len)` - Generate random hex string
- `random_bytes(len)` - Generate random bytes

### Builders Module

**TestBuilder\<T\>:**
- `new()` - Create new builder
- `with(key, value)` - Add key-value pair
- `build()` - Build HashMap

**StringBuilder:**
- `new()` - Create new builder
- `append(s)` - Append string
- `append_with_sep(s, sep)` - Append with separator
- `build()` - Build final string
- `build_with_sep(sep)` - Build with custom separator

**UserBuilder:**
- `new()` - Create new user builder
- `id(id)` - Set user ID
- `name(name)` - Set name
- `email(email)` - Set email
- `age(age)` - Set age
- `active(bool)` - Set active status
- `build()` - Build HashMap representation

### Matchers Module

**Functions:**
- `contains_string(haystack, needle)` - Check if string contains substring
- `is_ok(result)` - Check if result is Ok
- `is_error(result)` - Check if result is Err
- `matches_pattern(text, pattern)` - Check if string matches regex
- `in_range(value, min, max)` - Check if value in range
- `is_empty(collection)` - Check if collection is empty
- `is_not_empty(collection)` - Check if collection is not empty
- `has_length(collection, expected)` - Check collection length

### Mocks Module

**MockHttpClient:**
- `new()` - Create new mock client
- `add_response(url, status, body)` - Add mock response
- `get(url)` - Simulate GET request
- `requests()` - Get all requests made
- `request_count(url)` - Count requests to URL
- `clear_requests()` - Clear request history

**MockStorage:**
- `new()` - Create new mock storage
- `set(key, value)` - Set key-value pair
- `get(key)` - Get value by key
- `delete(key)` - Delete key
- `exists(key)` - Check if key exists
- `keys()` - Get all keys
- `clear()` - Clear all data
- `len()` - Get item count
- `is_empty()` - Check if empty

### Temp Module

**TempDir:**
- `new(prefix)` - Create temporary directory
- `path()` - Get directory path
- `create_file(name, contents)` - Create file in directory
- `create_dir(name)` - Create subdirectory
- `keep()` - Keep directory after drop

**TempFile:**
- `new(prefix, extension)` - Create temporary file
- `with_contents(prefix, extension, contents)` - Create with contents
- `path()` - Get file path
- `write(contents)` - Write to file
- `read()` - Read from file
- `keep()` - Keep file after drop

### Time Module

Re-exports from `common::sync`:
- `Clock` - Time abstraction trait
- `SystemClock` - Real system clock
- `MockClock` - Mock clock for testing

## Best Practices

### 1. Use Assertions for Clarity

```rust
// Good: Clear intent
assert_error_contains!(result, "timeout");

// Less clear
assert!(result.is_err());
assert!(format!("{:?}", result.unwrap_err()).contains("timeout"));
```

### 2. Leverage Builders for Complex Objects

```rust
// Good: Readable and maintainable
let user = UserBuilder::new()
    .name("Alice")
    .email("alice@example.com")
    .active(true)
    .build();

// Less readable
let mut user = HashMap::new();
user.insert("name".to_string(), "Alice".to_string());
user.insert("email".to_string(), "alice@example.com".to_string());
user.insert("active".to_string(), "true".to_string());
```

### 3. Always Use Temp Files for File System Tests

```rust
// Good: Automatic cleanup
let temp_file = TempFile::new("test", "txt").unwrap();
// Test code...
// File automatically deleted

// Bad: Manual cleanup, easy to forget
let path = "/tmp/test-file.txt";
std::fs::write(path, "data").unwrap();
// Test code...
std::fs::remove_file(path).unwrap(); // Easy to forget or fail
```

### 4. Use Mock Clock for Time-Dependent Tests

```rust
// Good: Deterministic, fast
let clock = MockClock::new();
clock.advance(Duration::from_secs(5));
// Test with known time

// Bad: Non-deterministic, slow
tokio::time::sleep(Duration::from_secs(5)).await;
```

### 5. Leverage Matchers for Reusable Logic

```rust
// Good: Reusable, composable
fn assert_valid_email(email: &str) {
    assert!(matches_pattern(email, r"^\w+@\w+\.\w+$").is_ok());
}

// Less reusable
assert!(email.contains('@') && email.contains('.'));
```

## Testing the Testing Module

```bash
# Run all tests
cargo test --lib common::testing

# Run specific test module
cargo test --lib common::testing::assertions
cargo test --lib common::testing::fixtures

# Run with all features
cargo test --all-features --lib common::testing
```

## Examples

### Integration Test Helper

```rust
use agent::common::testing::{TempDir, MockStorage, random_string};

pub struct TestEnvironment {
    pub temp_dir: TempDir,
    pub storage: MockStorage,
    pub user_id: String,
}

impl TestEnvironment {
    pub fn new() -> Self {
        Self {
            temp_dir: TempDir::new("test-env").unwrap(),
            storage: MockStorage::new(),
            user_id: random_string(16),
        }
    }
}

#[test]
fn test_with_environment() {
    let env = TestEnvironment::new();
    
    env.storage.set(&env.user_id, "test-data").unwrap();
    let data = env.storage.get(&env.user_id).unwrap();
    
    assert_eq!(data, Some("test-data".to_string()));
}
```

### Property-Based Test Helper

```rust
use agent::common::testing::fixtures::random_u64_range;

#[test]
fn property_test_addition() {
    for _ in 0..100 {
        let a = random_u64_range(0, 1000);
        let b = random_u64_range(0, 1000);
        
        let sum = a + b;
        assert!(sum >= a);
        assert!(sum >= b);
    }
}
```

## Related Modules

- **common::sync** - Time abstractions (Clock, MockClock)
- **common::error** - Error types for testing
- **common::resilience** - Retry and circuit breaker testing

## Dependencies

```toml
[dependencies]
uuid = "1.0"
regex = "1.10"
rand = "0.8"
```

## License

See the root LICENSE file for licensing information.

