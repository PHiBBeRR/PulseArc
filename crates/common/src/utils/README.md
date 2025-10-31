# Utility Functions and Helpers

Common utility functions and helper macros for reducing boilerplate and standardizing serialization across the agent codebase.

## Overview

This module provides reusable utilities that simplify common patterns throughout the application. It includes declarative macros for reducing enum conversion boilerplate and serde serialization helpers for complex data types like `Duration`.

## Features

- **Status Conversion Macros**: Eliminate boilerplate for Display and FromStr implementations
- **Serde Helpers**: Custom serialization for Duration as milliseconds
- **Case-Insensitive Parsing**: Automatic case normalization in string conversions
- **Consistent Patterns**: Standardized implementations across the codebase

## Architecture

```text
┌─────────────────────────────────┐
│       Utils Module              │
├─────────────────────────────────┤
│                                 │
│  ┌───────────────────────────┐ │
│  │   Macros (macros.rs)      │ │
│  │  • impl_status_conversions│ │
│  └───────────────────────────┘ │
│                                 │
│  ┌───────────────────────────┐ │
│  │   Serde (serde.rs)        │ │
│  │  • duration_millis        │ │
│  └───────────────────────────┘ │
│                                 │
└─────────────────────────────────┘
```

## Components

### 1. Macros (`macros.rs`)

Declarative macros for eliminating repetitive implementations.

#### impl_status_conversions!

Implements Display and FromStr traits for status enums with:
- Lowercase string output via Display
- Case-insensitive parsing via FromStr
- Descriptive error messages with enum name
- Zero boilerplate for common status patterns

**Generated Implementations:**
- `Display` - Converts enum variant to lowercase string
- `FromStr` - Parses case-insensitive strings to enum variants

**Features:**
- Accepts any variant mapping (PascalCase to snake_case)
- Handles all string case variations (UPPER, lower, Mixed)
- Returns clear error messages on invalid input

### 2. Serde Helpers (`serde.rs`)

Custom serialization utilities for common data types.

#### duration_millis

Serde serialization module for Duration:
- Serializes Duration as milliseconds (u64)
- Deserializes milliseconds back to Duration
- JSON-compatible integer representation
- Handles zero and large duration values

## Usage Examples

### Status Conversion Macro

```rust
use agent::impl_status_conversions;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Failed,
}

// Generate Display and FromStr implementations
impl_status_conversions!(BatchStatus {
    Pending => "pending",
    Processing => "processing",
    Completed => "completed",
    Failed => "failed",
});

// Display implementation (lowercase)
let status = BatchStatus::Processing;
assert_eq!(status.to_string(), "processing");
println!("Status: {}", status);  // Output: "Status: processing"

// FromStr implementation (case-insensitive)
let status = BatchStatus::from_str("COMPLETED").unwrap();
assert_eq!(status, BatchStatus::Completed);

let status = BatchStatus::from_str("pending").unwrap();
assert_eq!(status, BatchStatus::Pending);

let status = BatchStatus::from_str("FaILeD").unwrap();
assert_eq!(status, BatchStatus::Failed);

// Error handling
match BatchStatus::from_str("invalid") {
    Ok(_) => println!("Parsed successfully"),
    Err(e) => println!("Error: {}", e),  // "Invalid BatchStatus: invalid"
}
```

### Duration Serialization

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;
use agent::common::utils::duration_millis;

#[derive(Serialize, Deserialize)]
struct Config {
    #[serde(with = "duration_millis")]
    timeout: Duration,

    #[serde(with = "duration_millis")]
    retry_delay: Duration,

    name: String,
}

// Create config
let config = Config {
    timeout: Duration::from_secs(30),
    retry_delay: Duration::from_millis(1500),
    name: "my_config".to_string(),
};

// Serialize to JSON
let json = serde_json::to_string(&config).unwrap();
println!("{}", json);
// Output: {"timeout":30000,"retry_delay":1500,"name":"my_config"}

// Deserialize from JSON
let json = r#"{"timeout":5000,"retry_delay":2500,"name":"restored"}"#;
let config: Config = serde_json::from_str(json).unwrap();

assert_eq!(config.timeout, Duration::from_secs(5));
assert_eq!(config.retry_delay, Duration::from_millis(2500));
```

### Multiple Duration Fields

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;
use agent::common::utils::duration_millis;

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct NetworkConfig {
    #[serde(with = "duration_millis")]
    connect_timeout: Duration,

    #[serde(with = "duration_millis")]
    read_timeout: Duration,

    #[serde(with = "duration_millis")]
    write_timeout: Duration,

    retries: u32,
}

let config = NetworkConfig {
    connect_timeout: Duration::from_secs(5),
    read_timeout: Duration::from_secs(30),
    write_timeout: Duration::from_secs(15),
    retries: 3,
};

// Serialize and deserialize
let json = serde_json::to_string(&config).unwrap();
let restored: NetworkConfig = serde_json::from_str(&json).unwrap();

assert_eq!(config, restored);
```

### Zero and Large Duration Values

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;
use agent::common::utils::duration_millis;

#[derive(Serialize, Deserialize)]
struct Timings {
    #[serde(with = "duration_millis")]
    zero: Duration,

    #[serde(with = "duration_millis")]
    one_hour: Duration,

    #[serde(with = "duration_millis")]
    one_day: Duration,
}

let timings = Timings {
    zero: Duration::ZERO,
    one_hour: Duration::from_secs(3600),
    one_day: Duration::from_secs(86400),
};

let json = serde_json::to_string(&timings).unwrap();
// {"zero":0,"one_hour":3600000,"one_day":86400000}

let restored: Timings = serde_json::from_str(&json).unwrap();
assert_eq!(restored.zero, Duration::ZERO);
assert_eq!(restored.one_hour, Duration::from_secs(3600));
```

### Complex Enum with Custom Strings

```rust
use agent::impl_status_conversions;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    NotStarted,
    InProgress,
    PartiallyCompleted,
    FullyCompleted,
    FailedRetryable,
    FailedPermanent,
}

impl_status_conversions!(SyncStatus {
    NotStarted => "not_started",
    InProgress => "in_progress",
    PartiallyCompleted => "partially_completed",
    FullyCompleted => "fully_completed",
    FailedRetryable => "failed_retryable",
    FailedPermanent => "failed_permanent",
});

// All variants work with any case
assert_eq!(SyncStatus::from_str("NOT_STARTED").unwrap(), SyncStatus::NotStarted);
assert_eq!(SyncStatus::from_str("in_progress").unwrap(), SyncStatus::InProgress);
assert_eq!(SyncStatus::from_str("PARTIALLY_COMPLETED").unwrap(), SyncStatus::PartiallyCompleted);
```

### Roundtrip Conversion

```rust
use agent::impl_status_conversions;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Active,
    Inactive,
    Suspended,
}

impl_status_conversions!(State {
    Active => "active",
    Inactive => "inactive",
    Suspended => "suspended",
});

// Test roundtrip conversion
let states = vec![State::Active, State::Inactive, State::Suspended];

for state in states {
    let string = state.to_string();
    let parsed = State::from_str(&string).unwrap();
    assert_eq!(state, parsed);
}
```

## API Reference

### Macros

**impl_status_conversions!**

Signature:
```rust
impl_status_conversions!(EnumName {
    Variant1 => "string1",
    Variant2 => "string2",
    // ... more variants
});
```

Parameters:
- `EnumName` - The enum type name
- `Variant` - Enum variant name (PascalCase)
- `"string"` - String representation (typically snake_case)

Generated:
- `Display` trait - Converts enum to string
- `FromStr` trait - Parses string to enum (case-insensitive)

### Serde Modules

**duration_millis**

Functions:
- `serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>`
- `deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>`

Usage:
```rust
#[serde(with = "duration_millis")]
field_name: Duration
```

## Testing

### Unit Tests

```bash
# Run all utils tests
cargo test --package agent --lib common::utils

# Run specific module tests
cargo test --package agent --lib common::utils::macros
cargo test --package agent --lib common::utils::serde
```

### Test Examples

```rust
use agent::impl_status_conversions;
use std::str::FromStr;

#[test]
fn test_status_display() {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TestStatus { Pending, Done }

    impl_status_conversions!(TestStatus {
        Pending => "pending",
        Done => "done",
    });

    assert_eq!(TestStatus::Pending.to_string(), "pending");
    assert_eq!(TestStatus::Done.to_string(), "done");
}

#[test]
fn test_status_fromstr() {
    assert_eq!(TestStatus::from_str("pending").unwrap(), TestStatus::Pending);
    assert_eq!(TestStatus::from_str("DONE").unwrap(), TestStatus::Done);
    assert!(TestStatus::from_str("invalid").is_err());
}
```

```rust
use serde::{Deserialize, Serialize};
use std::time::Duration;
use agent::common::utils::duration_millis;

#[test]
fn test_duration_serialization() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Test {
        #[serde(with = "duration_millis")]
        duration: Duration,
    }

    let test = Test {
        duration: Duration::from_millis(1500),
    };

    let json = serde_json::to_string(&test).unwrap();
    assert!(json.contains("1500"));

    let restored: Test = serde_json::from_str(&json).unwrap();
    assert_eq!(test, restored);
}
```

## Best Practices

### Macros

1. **Use for Status Enums**: Apply to enums representing states, statuses, or modes
2. **Consistent Naming**: Use snake_case for string representations
3. **Document Variants**: Add doc comments to explain each variant
4. **Test Roundtrips**: Verify Display and FromStr work together
5. **Handle Errors**: Use proper error handling for FromStr failures

### Serde

1. **Use duration_millis**: Prefer milliseconds for JSON compatibility
2. **Document Units**: Clearly document time units in comments
3. **Test Roundtrips**: Verify serialization and deserialization work correctly
4. **Handle Edge Cases**: Test with zero, very small, and very large durations
5. **Consistent Format**: Use same time unit across related fields

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

## Related Modules

- **agent/common/observability**: Error types using status conversions
- **agent/common/validation**: Field validation utilities
- **agent/storage**: Duration fields in database models

## Roadmap

- [ ] Add more serde helpers (datetime formatting, etc.)
- [ ] Create macros for other common patterns
- [ ] Add utility functions for common transformations
- [ ] Implement builder pattern macros

## License

See the root LICENSE file for licensing information.
