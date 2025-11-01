# Test Factory Guide for PulseArc

**Purpose**: This guide provides everything you need to create test factories and test harnesses for the PulseArc application.

---

## Table of Contents

1. [Existing Test Infrastructure](#existing-test-infrastructure)
2. [AppContext Structure](#appcontext-structure)
3. [Test Database Setup](#test-database-setup)
4. [Creating a Test Factory](#creating-a-test-factory)
5. [Available Test Utilities](#available-test-utilities)
6. [Best Practices](#best-practices)
7. [Complete Examples](#complete-examples)

---

## Existing Test Infrastructure

### Files and Locations

**Infra Test Support** (`crates/infra/tests/support.rs`):
- `TestDatabase` - Temporary SQLCipher database wrapper
- `setup_outbox_db()` - Creates outbox table schema
- `setup_segment_db()` - Creates activity segments table
- `setup_snapshot_db()` - Creates activity snapshots table
- `make_outbox_entry()` - Helper for creating test outbox entries
- `init_test_logger()` - Test logger with log capture

**Common Testing Module** (`crates/common/src/testing/`):
- **fixtures.rs** - Random data generators (seeded and non-seeded)
- **builders.rs** - Fluent test data builders
- **mocks.rs** - Mock implementations (HTTP, Storage, OAuth, Keychain)
- **assertions.rs** - Custom assertions
- **async_utils.rs** - Async test helpers
- **temp.rs** - Temporary file/directory utilities
- **time.rs** - MockClock for time-based testing

**API Test Examples** (`crates/api/tests/`):
- `context_lifecycle.rs` - AppContext creation and shutdown tests
- `database_commands.rs` - Integration tests with database
- `user_profile_commands.rs` - User profile command tests

---

## AppContext Structure

The `AppContext` is your application's dependency injection container. Here's what it contains:

```rust
pub struct AppContext {
    // Core services
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<DynFeatureFlagsPort>,           // Trait object
    pub database_stats: Arc<DynDatabaseStatsPort>,         // Trait object
    pub command_metrics: Arc<DynCommandMetricsPort>,       // Trait object
    pub snapshots: Arc<DynSnapshotRepositoryPort>,         // Trait object
    pub user_profile: Arc<DynUserProfileRepositoryPort>,   // Trait object

    // Schedulers
    pub block_scheduler: Arc<BlockScheduler>,
    pub classification_scheduler: Arc<ClassificationScheduler>,
    pub sync_scheduler: Arc<SyncScheduler>,

    #[cfg(feature = "calendar")]
    pub calendar_scheduler: Arc<CalendarScheduler>,

    // Instance lock (keeps app singleton)
    _instance_lock: InstanceLock,
}
```

### Key Dependencies for Testing

**Required for Minimal Test Context:**
1. **Database** (`DbManager`) - SQLCipher encrypted database
2. **Config** - Application configuration
3. **Services** - Trait objects implementing ports
4. **Schedulers** - Background job schedulers

---

## Test Database Setup

### Basic Test Database

```rust
use tempfile::TempDir;
use pulsearc_infra::database::DbManager;
use std::sync::Arc;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

pub struct TestDatabase {
    pub manager: Arc<DbManager>,
    _temp_dir: TempDir,
}

impl TestDatabase {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("temp dir creation");
        let db_path = temp_dir.path().join("test.db");

        let manager = DbManager::new(&db_path, 4, Some(TEST_KEY))
            .expect("db manager creation");

        Self {
            manager: Arc::new(manager),
            _temp_dir: temp_dir
        }
    }

    pub fn execute_batch(&self, sql: &str) {
        let conn = self.manager.get_connection()
            .expect("connection");
        conn.execute_batch(sql)
            .expect("SQL execution");
    }
}
```

### Schema Setup Helpers

```rust
pub fn setup_outbox_db() -> TestDatabase {
    let db = TestDatabase::new();
    db.execute_batch("
        CREATE TABLE time_entry_outbox (
            id TEXT PRIMARY KEY,
            idempotency_key TEXT NOT NULL,
            user_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            status TEXT NOT NULL,
            -- ... other columns
        );
        CREATE INDEX idx_outbox_status_retry
        ON time_entry_outbox(status, retry_after);
    ");
    db
}
```

---

## Creating a Test Factory

### Pattern 1: Simple Helper Function (Recommended for Quick Tests)

```rust
// In your test file or tests/support.rs

use pulsearc_domain::Config;
use pulsearc_lib::context::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

async fn create_test_context() -> pulsearc_domain::Result<AppContext> {
    // Set encryption key to avoid keychain access
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

    // Create temporary database path
    let temp_dir = std::env::temp_dir();
    let test_db_path = temp_dir.join(
        format!("pulsearc_test_{}.db", uuid::Uuid::new_v4())
    );

    // Custom config with test database
    let config = Config {
        database: pulsearc_domain::DatabaseConfig {
            path: test_db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None, // Uses env var
        },
        ..Config::default()
    };

    AppContext::new_with_config(config).await
}

// Usage in tests:
#[tokio::test(flavor = "multi_thread")]
async fn test_something() {
    let context = create_test_context().await.unwrap();
    // ... your test
    context.shutdown().await.unwrap();
}
```

### Pattern 2: Builder Pattern (Recommended for Complex Factories)

```rust
// In tests/support.rs or a dedicated test_factory.rs module

use std::sync::Arc;
use pulsearc_domain::{Config, DatabaseConfig, Result};
use pulsearc_lib::context::AppContext;
use tempfile::TempDir;

pub struct AppContextBuilder {
    pool_size: usize,
    encryption_key: Option<String>,
    custom_db_path: Option<String>,
    _temp_dir: Option<TempDir>,
}

impl AppContextBuilder {
    pub fn new() -> Self {
        Self {
            pool_size: 5,
            encryption_key: Some(Self::test_key().to_string()),
            custom_db_path: None,
            _temp_dir: None,
        }
    }

    fn test_key() -> &'static str {
        "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }

    pub fn with_pool_size(mut self, size: usize) -> Self {
        self.pool_size = size;
        self
    }

    pub fn with_custom_db_path(mut self, path: String) -> Self {
        self.custom_db_path = Some(path);
        self
    }

    pub async fn build(self) -> Result<AppContext> {
        // Set environment variable for encryption key
        if let Some(key) = &self.encryption_key {
            std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", key);
        }

        // Determine database path
        let db_path = if let Some(path) = self.custom_db_path {
            path
        } else {
            let temp_dir = std::env::temp_dir();
            temp_dir.join(format!("test_{}.db", uuid::Uuid::new_v4()))
                .to_string_lossy()
                .to_string()
        };

        let config = Config {
            database: DatabaseConfig {
                path: db_path,
                pool_size: self.pool_size,
                encryption_key: None,
            },
            ..Config::default()
        };

        AppContext::new_with_config(config).await
    }
}

// Usage:
#[tokio::test(flavor = "multi_thread")]
async fn test_with_builder() {
    let context = AppContextBuilder::new()
        .with_pool_size(10)
        .build()
        .await
        .unwrap();

    // ... test

    context.shutdown().await.unwrap();
}
```

### Pattern 3: Test Harness with Lifecycle Management

```rust
// In tests/support.rs

pub struct TestHarness {
    pub context: AppContext,
    pub db: Arc<DbManager>,
    _temp_dir: TempDir,
}

impl TestHarness {
    pub async fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        let db_path = temp_dir.path().join("test.db");

        std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", Self::test_key());

        let config = Config {
            database: DatabaseConfig {
                path: db_path.to_string_lossy().to_string(),
                pool_size: 5,
                encryption_key: None,
            },
            ..Config::default()
        };

        let context = AppContext::new_with_config(config).await?;
        let db = Arc::clone(&context.db);

        Ok(Self { context, db, _temp_dir: temp_dir })
    }

    fn test_key() -> &'static str {
        "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    }

    pub async fn shutdown(self) -> Result<()> {
        self.context.shutdown().await
    }

    // Helper methods for common operations
    pub fn execute_sql(&self, sql: &str) -> Result<()> {
        let conn = self.db.get_connection()?;
        conn.execute_batch(sql)?;
        Ok(())
    }
}

// Usage:
#[tokio::test(flavor = "multi_thread")]
async fn test_with_harness() {
    let harness = TestHarness::new().await.unwrap();

    // Setup test data
    harness.execute_sql("INSERT INTO ...").unwrap();

    // Run test
    // ...

    // Cleanup
    harness.shutdown().await.unwrap();
}
```

---

## Available Test Utilities

### Random Data Generation

```rust
use pulsearc_common::testing::fixtures::*;

// Non-deterministic (different each run)
let email = random_email();
let name = random_string(20);
let id = random_u64();

// Deterministic (same result for same seed)
let email = random_email_seeded(42);
let name = random_string_seeded(20, 42);
let id = random_u64_seeded(42);
```

### Mock Objects

```rust
use pulsearc_common::testing::mocks::*;

// Mock HTTP client
let http = MockHttpClient::new();
http.add_response("https://api.example.com", 200, "OK");
let response = http.get("https://api.example.com").unwrap();

// Response sequences (different response each time)
http.add_response_sequence(
    "https://api.example.com",
    vec![(200, "First"), (200, "Second"), (404, "Not Found")]
);

// Mock storage
let storage = MockStorage::new();
storage.set("key", "value");
assert_eq!(storage.get("key").unwrap(), "value");

// Mock OAuth client (with platform feature)
#[cfg(feature = "platform")]
{
    let oauth = MockOAuthClient::new();
    oauth.add_token_response(token_set);
}
```

### Time Mocking

```rust
use pulsearc_common::testing::{MockClock, SystemClock, Clock};
use std::time::Duration;

let clock = MockClock::new();
clock.advance(Duration::from_secs(60));

let now = clock.now(); // Returns fixed time
```

### Assertions

```rust
use pulsearc_common::testing::assertions::*;

// Approximate equality (for floating point)
assert_approx_eq!(3.14, 3.141, 0.01);

// Duration in range
assert_duration_in_range!(
    elapsed,
    Duration::from_secs(1),
    Duration::from_secs(2)
);

// Contains all items
assert_contains_all!(vec![1, 2, 3, 4], &[2, 4]);

// Is sorted
assert_sorted!(&[1, 2, 3, 4, 5]);

// Error contains message
assert_error_contains!(result, "connection timeout");
```

### Async Utilities

```rust
use pulsearc_common::testing::async_utils::*;
use std::time::Duration;

// Retry until condition met
retry_async(5, Duration::from_millis(100), || async {
    // your async operation
    Ok(())
}).await.unwrap();

// Poll until condition
poll_until(Duration::from_secs(5), Duration::from_millis(100), || async {
    // Check condition
    Ok(true)
}).await.unwrap();

// Timeout wrapper
timeout_ok(Duration::from_secs(1), async {
    // your operation
}).await.unwrap();
```

### Temporary Files

```rust
use pulsearc_common::testing::temp::*;

// Temporary directory (auto-cleanup)
let temp_dir = TempDir::new().unwrap();
let path = temp_dir.path();

// Temporary file
let temp_file = TempFile::new().unwrap();
temp_file.write_str("test content").unwrap();
```

---

## Best Practices

### 1. **Use Deterministic Data**

```rust
// ❌ BAD - flaky tests
let user_id = random_u64();

// ✅ GOOD - deterministic
let user_id = random_u64_seeded(42);
```

### 2. **Clean Up Resources**

```rust
// ✅ GOOD - explicit cleanup
#[tokio::test]
async fn test_app() {
    let context = create_test_context().await.unwrap();

    // ... test

    context.shutdown().await.unwrap();
}

// ✅ GOOD - RAII cleanup via Drop
{
    let _context = create_test_context().await.unwrap();
    // Cleaned up when scope ends
}
```

### 3. **Isolate Tests with Unique Databases**

```rust
// ✅ GOOD - each test gets unique database
async fn create_test_context() -> Result<AppContext> {
    let test_db_path = std::env::temp_dir()
        .join(format!("test_{}.db", uuid::Uuid::new_v4()));
    // ...
}
```

### 4. **Use TEST_DATABASE_ENCRYPTION_KEY Environment Variable**

```rust
// ✅ GOOD - avoids keychain access in tests
std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);
```

### 5. **Set Timeouts for Async Tests**

```rust
#[tokio::test]
async fn test_shutdown() {
    let context = create_test_context().await.unwrap();

    // ✅ GOOD - prevent hanging tests
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        context.shutdown()
    ).await;

    assert!(result.is_ok(), "shutdown timed out");
}
```

### 6. **Use Multi-Thread Tokio Runtime**

```rust
// ✅ GOOD - matches production runtime
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_operations() {
    // ...
}
```

---

## Complete Examples

### Example 1: Simple Integration Test

```rust
use pulsearc_domain::{Config, DatabaseConfig};
use pulsearc_lib::context::AppContext;

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

#[tokio::test(flavor = "multi_thread")]
async fn test_feature_flags() {
    // Setup
    std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);
    let db_path = std::env::temp_dir()
        .join(format!("test_{}.db", uuid::Uuid::new_v4()));

    let config = Config {
        database: DatabaseConfig {
            path: db_path.to_string_lossy().to_string(),
            pool_size: 5,
            encryption_key: None,
        },
        ..Config::default()
    };

    let context = AppContext::new_with_config(config)
        .await
        .expect("context creation");

    // Test
    let is_enabled = context.feature_flags
        .is_enabled("test_flag", false)
        .await
        .expect("feature flag check");

    assert!(!is_enabled, "test_flag should default to false");

    // Cleanup
    context.shutdown().await.expect("shutdown");
}
```

### Example 2: Test with Custom Database Schema

```rust
use pulsearc_infra::tests::support::TestDatabase;

#[tokio::test]
async fn test_custom_schema() {
    let db = TestDatabase::new();

    // Setup custom schema
    db.execute_batch("
        CREATE TABLE test_users (
            id TEXT PRIMARY KEY,
            email TEXT NOT NULL,
            created_at INTEGER NOT NULL
        );
    ");

    // Insert test data
    let conn = db.manager.get_connection().unwrap();
    conn.execute(
        "INSERT INTO test_users (id, email, created_at) VALUES (?, ?, ?)",
        rusqlite::params!["user1", "test@example.com", 1234567890]
    ).unwrap();

    // Query and verify
    let email: String = conn.query_row(
        "SELECT email FROM test_users WHERE id = ?",
        rusqlite::params!["user1"],
        |row| row.get(0)
    ).unwrap();

    assert_eq!(email, "test@example.com");
}
```

### Example 3: Test Factory with Builder Pattern

```rust
pub struct TestContextFactory {
    pool_size: usize,
    with_calendar: bool,
}

impl TestContextFactory {
    pub fn new() -> Self {
        Self {
            pool_size: 5,
            with_calendar: false,
        }
    }

    pub fn with_large_pool(mut self) -> Self {
        self.pool_size = 20;
        self
    }

    #[cfg(feature = "calendar")]
    pub fn with_calendar(mut self) -> Self {
        self.with_calendar = true;
        self
    }

    pub async fn create(self) -> Result<AppContext> {
        let test_key = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", test_key);

        let db_path = std::env::temp_dir()
            .join(format!("test_{}.db", uuid::Uuid::new_v4()));

        let mut config = Config {
            database: DatabaseConfig {
                path: db_path.to_string_lossy().to_string(),
                pool_size: self.pool_size,
                encryption_key: None,
            },
            ..Config::default()
        };

        AppContext::new_with_config(config).await
    }
}

// Usage:
#[tokio::test(flavor = "multi_thread")]
async fn test_with_factory() {
    let context = TestContextFactory::new()
        .with_large_pool()
        .create()
        .await
        .unwrap();

    // ... test

    context.shutdown().await.unwrap();
}
```

---

## Key Takeaways

1. **Use `TestDatabase` from `crates/infra/tests/support.rs`** for database-heavy tests
2. **Use `AppContextBuilder` pattern** for complex AppContext setup
3. **Always set `TEST_DATABASE_ENCRYPTION_KEY`** to avoid keychain prompts
4. **Use deterministic random data** (`*_seeded` variants) for reproducible tests
5. **Leverage existing mock objects** in `pulsearc_common::testing::mocks`
6. **Clean up resources** with explicit `shutdown()` or RAII patterns
7. **Use `#[tokio::test(flavor = "multi_thread")]`** for realistic async testing
8. **Set timeouts** to prevent hanging tests

---

## Additional Resources

- [crates/infra/tests/support.rs](../crates/infra/tests/support.rs) - Infrastructure test helpers
- [crates/common/src/testing/](../crates/common/src/testing/) - Common testing utilities
- [crates/api/tests/context_lifecycle.rs](../crates/api/tests/context_lifecycle.rs) - AppContext test examples
- [crates/common/docs/API_GUIDE.md](../crates/common/docs/API_GUIDE.md) - Common crate API guide
