# Common Storage Infrastructure

Enterprise-grade encrypted storage infrastructure with SQLCipher integration, connection pooling, and comprehensive security features.

## Overview

The `agent/common/storage` module provides platform-agnostic storage primitives for encrypted SQLite databases. It serves as the foundation for all database operations in the PulseArc agent, offering secure-by-default encrypted storage with enterprise features like connection pooling, circuit breakers, and comprehensive metrics.

## Features

✅ **SQLCipher Encryption** - Industry-standard 256-bit AES encryption at rest
✅ **Connection Pooling** - r2d2-based pool with configurable size and timeouts
✅ **Circuit Breaker Protection** - Automatic fault isolation using `agent/common/resilience`
✅ **Platform-Agnostic Traits** - Swap backends without changing application code
✅ **Key Management** - Secure key storage via platform keychain integration
✅ **Comprehensive Metrics** - Connection pool, query, and performance tracking
✅ **WAL Mode** - Write-Ahead Logging for improved concurrency
✅ **Type Safety** - Strongly-typed errors with retry semantics
✅ **Structured Logging** - Full tracing integration with correlation IDs

## Architecture

### Module Structure

```
agent/common/storage/
├── mod.rs                  # Public API and re-exports
├── config.rs               # StorageConfig and KeySource
├── error.rs                # StorageError with ErrorClassification
├── types.rs                # Platform-agnostic traits
├── sqlcipher/              # SQLCipher backend implementation
│   ├── mod.rs
│   ├── cipher.rs           # SQLCipher encryption configuration
│   ├── config.rs           # Pool configuration
│   ├── connection.rs       # Connection wrapper
│   ├── pool.rs             # r2d2 connection pool
│   └── pragmas.rs          # SQLite pragma management
└── encryption/             # → agent/common/security/encryption
    ├── keys.rs             # Cryptographic key generation
    ├── keychain.rs         # Platform keychain integration
    ├── rotation.rs         # StorageKeyManager for key rotation
    ├── secure_string.rs    # Auto-zeroizing SecureString
    └── cache.rs            # Thread-safe key caching
```

### Design Philosophy

1. **Portable by Default** - Platform-agnostic traits allow backend flexibility
2. **Secure by Default** - Encryption is mandatory, not optional
3. **Enterprise-Ready** - Connection pooling, circuit breakers, metrics built-in
4. **Type-Safe Errors** - Rich error types with retry semantics and severity levels
5. **Zero-Copy Where Possible** - Efficient memory usage with minimal allocations

## Usage

### Basic Setup

```rust
use agent::common::storage::{
    StorageConfig, StorageConfigBuilder, KeySource
};
use agent::storage::{StorageManager, SqlCipherPool};
use std::path::PathBuf;

// Production: Use platform keychain (recommended)
let config = StorageConfig::new(PathBuf::from("data/app.db"))
    .with_key_source(KeySource::Keychain {
        service: "PulseArc".to_string(),
        username: "db_encryption_key".to_string(),
    })
    .with_pool_size(10)
    .with_connection_timeout(Duration::from_secs(5));

// Initialize storage manager
let storage = StorageManager::with_sqlcipher(
    Path::new("data/app.db"),
    config
)?;

// Get a connection and execute queries
let conn = storage.get_connection()?;
conn.execute("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT)", &[])?;
```

### Configuration Builder Pattern

```rust
use agent::common::storage::{StorageConfig, KeySource};
use std::time::Duration;

// Use builder for validation
let config = StorageConfig::builder(PathBuf::from("data/app.db"))
    .pool_size(20)
    .connection_timeout(Duration::from_secs(10))
    .busy_timeout(Duration::from_millis(10000))
    .key_source(KeySource::keychain("PulseArc", "db_key"))
    .build()?; // Validates all settings

// Builder catches invalid configurations
let invalid = StorageConfig::builder(PathBuf::from("data/app.db"))
    .pool_size(0)  // Invalid!
    .build();      // Returns Err(InvalidConfig)
```

### Environment-Specific Key Sources

```rust
// Development: Direct key (NOT for production)
let dev_config = StorageConfig::new(db_path)
    .with_key_source(KeySource::direct(
        "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    ));

// CI/Testing: Environment variable
let ci_config = StorageConfig::new(db_path)
    .with_key_source(KeySource::environment("DB_ENCRYPTION_KEY"));

// Production: Platform keychain (macOS/Windows/Linux)
let prod_config = StorageConfig::new(db_path)
    .with_key_source(KeySource::keychain("PulseArc", "db_key"));
```

### Direct Pool Usage

```rust
use agent::common::storage::sqlcipher::{SqlCipherPool, SqlCipherPoolConfig};
use agent::security::encryption::keychain::get_or_create_key;

// Get encryption key from keychain
let key_source = KeySource::Keychain {
    service: "PulseArc".to_string(),
    username: "db_encryption_key".to_string(),
};
let key = get_or_create_key(&key_source)?;

// Create connection pool
let pool_config = SqlCipherPoolConfig::default();
let pool = SqlCipherPool::new(
    Path::new("data/app.db"),
    key.expose().to_string(),
    pool_config
)?;

// Get connection with circuit breaker protection
let conn = pool.get_sqlcipher_connection()?;

// Use connection for database operations
conn.execute("INSERT INTO users (name) VALUES (?)", &[&"Alice"])?;
```

### Transaction Management

```rust
use agent::common::storage::types::Transaction;

let conn = storage.get_sqlcipher_connection()?;

// Begin transaction
let mut tx = conn.transaction()?;

// Execute operations
tx.execute("INSERT INTO users (name) VALUES (?)", &[&"Bob"])?;
tx.execute("INSERT INTO logs (action) VALUES (?)", &[&"user_created"])?;

// Commit or rollback
tx.commit()?; // Or tx.rollback()? for explicit rollback
// Auto-rollback on drop if not committed
```

### Prepared Statements

```rust
let conn = storage.get_sqlcipher_connection()?;

// Prepare statement
let mut stmt = conn.prepare("INSERT INTO users (name) VALUES (?)")?;

// Execute multiple times efficiently
for name in &["Alice", "Bob", "Charlie"] {
    stmt.execute(&[name])?;
}

// Query with parameters
let mut stmt = conn.prepare("SELECT id, name FROM users WHERE name LIKE ?")?;
let users = stmt.query_map(&[&"%Al%"], |row| {
    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
})?;

for user in users {
    println!("User: {:?}", user);
}
```

### Query Methods

```rust
let conn = storage.get_sqlcipher_connection()?;

// Query single row
let user_count: i64 = conn.query_row(
    "SELECT COUNT(*) FROM users",
    &[],
    |row| row.get(0)
)?;

// Query with error handling
let name: Option<String> = conn.query_row(
    "SELECT name FROM users WHERE id = ?",
    &[&42],
    |row| row.get(0)
).ok(); // Convert error to None

// Execute non-query
let rows_affected = conn.execute(
    "DELETE FROM users WHERE last_login < ?",
    &[&old_date]
)?;
```

## Configuration

### StorageConfig

Complete configuration reference:

```rust
pub struct StorageConfig {
    /// Database file path
    pub path: PathBuf,

    /// Connection pool size (default: 10, max: 100)
    pub pool_size: u32,

    /// Connection timeout in seconds (default: 5)
    pub connection_timeout_secs: u64,

    /// Busy timeout in milliseconds (default: 5000)
    /// How long to wait for locked database before failing
    pub busy_timeout_ms: u64,

    /// Enable WAL mode (default: true)
    /// Recommended for better concurrency
    pub enable_wal: bool,

    /// Enable foreign keys (default: true)
    /// Recommended for referential integrity
    pub enable_foreign_keys: bool,

    /// Encryption key source
    pub key_source: KeySource,
}
```

### KeySource Options

```rust
pub enum KeySource {
    /// Platform keychain (RECOMMENDED for production)
    /// - macOS: Keychain Access
    /// - Windows: Credential Manager
    /// - Linux: Secret Service API
    Keychain {
        service: String,    // e.g., "PulseArc"
        username: String,   // e.g., "db_encryption_key"
    },

    /// Environment variable (for CI/testing)
    Environment {
        var_name: String,   // e.g., "DB_ENCRYPTION_KEY"
    },

    /// Direct key (DANGEROUS - testing only)
    /// ⚠️ Never use in production code
    Direct {
        key: String,        // 64-character encryption key
    },
}
```

### SqlCipherPoolConfig

Pool-specific configuration:

```rust
pub struct SqlCipherPoolConfig {
    /// Maximum connections (default: 10)
    pub max_size: u32,

    /// Connection timeout (default: 5 seconds)
    pub connection_timeout: Duration,

    /// SQLite busy timeout (default: 5000ms)
    pub busy_timeout: Duration,

    /// Enable WAL journal mode (default: true)
    pub enable_wal: bool,

    /// Enable foreign key constraints (default: true)
    pub enable_foreign_keys: bool,
}
```

## Security

### Encryption Configuration

SQLCipher 4.x with strong defaults:

```rust
use agent::common::storage::sqlcipher::{SqlCipherConfig, configure_sqlcipher};

let cipher_config = SqlCipherConfig::new("your-64-char-key".to_string())
    .with_cipher_compatibility(4)           // SQLCipher 4.x
    .with_kdf_iter(256000)                  // Strong PBKDF2 iterations
    .without_memory_security();             // Not recommended

// Applied automatically by SqlCipherPool
configure_sqlcipher(&conn, &cipher_config)?;
```

**Default SQLCipher Settings:**
- **Algorithm**: AES-256-CBC
- **KDF**: PBKDF2-HMAC-SHA512
- **Iterations**: 256,000 (NIST recommended minimum)
- **Page Size**: 4096 bytes
- **HMAC**: SHA512
- **Memory Security**: ON (prevents key leakage)

### Key Management

```rust
use agent::security::encryption::{
    generate_encryption_key,
    get_or_create_key,
    get_or_create_key_cached,
};

// Generate new cryptographically secure key
let new_key = generate_encryption_key(); // SecureString (auto-zeroing)

// Get or create from keychain (creates if missing)
let key = get_or_create_key(&KeySource::keychain("PulseArc", "db_key"))?;

// Cached version (fast subsequent access)
let cached_key = get_or_create_key_cached(&KeySource::keychain("PulseArc", "db_key"))?;
```

### Key Rotation

```rust
use agent::security::encryption::{StorageKeyManager, KeyRotationSchedule};

// Create key manager with schedule
let mut schedule = KeyRotationSchedule::default();
schedule.set_rotation_days(90); // Rotate every 90 days

let mut key_manager = StorageKeyManager::with_schedule(initial_key, schedule);

// Check and rotate if needed
if key_manager.should_rotate() {
    key_manager.rotate_key(&conn)?; // Uses SQLCipher PRAGMA rekey
}

// Force rotation (e.g., security incident)
key_manager.force_rotate(&conn)?;

// Monitor rotation status
println!("Days since last rotation: {}",
         key_manager.days_since_last_rotation());
```

### Memory Safety

All keys use `SecureString` with automatic zeroization:

```rust
use agent::security::encryption::SecureString;

{
    let key = get_or_create_key(&key_source)?;
    // Use key.expose() only when needed
    configure_sqlcipher(&conn, &SqlCipherConfig::from_secure_key(key))?;
} // <-- Memory is securely zeroed here
```

### Verification

```rust
use agent::common::storage::sqlcipher::verify_encryption;

// Verify encryption is working correctly
verify_encryption(&conn)?; // Fails if wrong key or not encrypted
```

## Connection Pooling

### Pool Configuration

```rust
use agent::common::storage::sqlcipher::{SqlCipherPool, SqlCipherPoolConfig};

let pool_config = SqlCipherPoolConfig {
    max_size: 20,                                    // Max connections
    connection_timeout: Duration::from_secs(10),     // Wait time
    busy_timeout: Duration::from_millis(5000),       // Lock wait time
    enable_wal: true,                                // Concurrency
    enable_foreign_keys: true,                       // Integrity
};

let pool = SqlCipherPool::new(db_path, encryption_key, pool_config)?;
```

### Health Checks

```rust
use agent::common::storage::types::ConnectionPool;

let health = pool.health_check()?;

println!("Healthy: {}", health.healthy);
println!("Active connections: {}", health.active_connections);
println!("Idle connections: {}", health.idle_connections);
println!("Max connections: {}", health.max_connections);

if let Some(msg) = health.message {
    eprintln!("Health issue: {}", msg);
}
```

### Pool Metrics

```rust
let metrics = pool.metrics();

println!("Total connections acquired: {}", metrics.connections_acquired);
println!("Connection timeouts: {}", metrics.connections_timeout);
println!("Connection errors: {}", metrics.connections_error);
println!("Average acquisition time: {}ms", metrics.avg_acquisition_time_ms);
println!("Queries executed: {}", metrics.queries_executed);
println!("Query failures: {}", metrics.queries_failed);
```

### Circuit Breaker Protection

Built-in circuit breaker prevents cascading failures:

```rust
// Circuit breaker automatically opens after 5 consecutive failures
// Prevents further connection attempts for 30 seconds
// Allows 3 test requests in half-open state
// Closes after 2 successful requests

// If circuit is open, get_connection() returns:
Err(StorageError::Connection(
    "Circuit breaker open - connection pool temporarily unavailable"
))
```

**Circuit Breaker States:**
1. **Closed** (Normal) - All requests go through
2. **Open** (Failing) - All requests rejected immediately
3. **Half-Open** (Testing) - Limited requests allowed to test recovery

## Error Handling

### Error Types

```rust
use agent::common::storage::error::{StorageError, StorageResult};
use agent::common::error::{ErrorClassification, ErrorSeverity};

pub enum StorageError {
    Connection(String),              // Connection failures
    Query(String),                   // Query execution errors
    DatabaseError(String),           // General database errors
    Encryption(String),              // Encryption configuration
    Migration(String),               // Schema migration errors
    Keychain(String),                // Keychain access issues
    WrongKeyOrNotEncrypted,          // Wrong key or unencrypted DB
    PoolExhausted,                   // No connections available
    Timeout(u64),                    // Connection timeout
    InvalidConfig(String),           // Configuration validation
    SchemaVersionMismatch { expected: i32, found: i32 },
    // ... plus std errors (Io, Rusqlite, R2d2, SerdeJson)
}
```

### Error Classification

```rust
use agent::common::error::ErrorClassification;

let err = StorageError::PoolExhausted;

// Check if retryable
if err.is_retryable() {
    println!("Can retry this operation");
}

// Get severity
match err.severity() {
    ErrorSeverity::Critical => { /* Alert immediately */ },
    ErrorSeverity::Error => { /* Log and monitor */ },
    ErrorSeverity::Warning => { /* Track metrics */ },
    _ => {},
}

// Check criticality
if err.is_critical() {
    // Page on-call engineer
}

// Get retry delay
if let Some(delay) = err.retry_after() {
    sleep(delay);
    retry();
}
```

### Retryable Errors

The following errors are marked as retryable:
- `PoolExhausted` - Pool may free up connections
- `Timeout(_)` - Temporary resource constraint
- `Connection(_)` - May be transient network issue
- SQLite `BUSY` or `LOCKED` errors - Database lock contention

### Error Context

```rust
use agent::common::storage::error::StorageError;

// Add operation context for debugging
let err = StorageError::Query("SELECT failed".to_string())
    .with_operation("fetch_user_by_id");

// Converts to CommonError with operation field
match err {
    StorageError::Common(common_err) => {
        println!("Failed during: {}", common_err.operation);
    },
    _ => {},
}
```

### Best Practices

```rust
use agent::common::storage::error::StorageResult;

fn fetch_user(id: i64) -> StorageResult<User> {
    let conn = storage.get_connection()?;

    conn.query_row(
        "SELECT id, name FROM users WHERE id = ?",
        &[&id],
        |row| Ok(User {
            id: row.get(0)?,
            name: row.get(1)?,
        })
    ).map_err(|e| {
        // Add context for better debugging
        StorageError::Query(format!("Failed to fetch user {}: {}", id, e))
            .with_operation("fetch_user")
    })
}

// Error handling in application code
match fetch_user(42) {
    Ok(user) => println!("Found user: {}", user.name),
    Err(StorageError::WrongKeyOrNotEncrypted) => {
        eprintln!("Database encryption key is incorrect");
    },
    Err(StorageError::PoolExhausted) => {
        eprintln!("Too many concurrent database connections");
        // Maybe retry after delay
    },
    Err(e) if e.is_retryable() => {
        eprintln!("Retryable error: {}", e);
        // Implement retry logic
    },
    Err(e) => {
        eprintln!("Database error: {}", e);
    },
}
```

## Performance

### Connection Pool Sizing

```rust
// Formula: pool_size = (core_count * 2) + effective_spindle_count
// For SSDs: effective_spindle_count = 1
// For 4-core machine with SSD: pool_size = (4 * 2) + 1 = 9

let optimal_pool_size = num_cpus::get() * 2 + 1;
let config = StorageConfig::new(db_path)
    .with_pool_size(optimal_pool_size as u32);
```

### WAL Mode Benefits

Write-Ahead Logging improves concurrency:

```rust
// WAL mode (enabled by default)
// - Readers don't block writers
// - Writers don't block readers
// - Up to 100x faster for writes
// - Automatic checkpointing

let config = StorageConfig::new(db_path)
    .with_enable_wal(true); // Default
```

### Pragma Tuning

Applied automatically by the pool:

```sql
PRAGMA journal_mode=WAL;              -- Concurrency
PRAGMA synchronous=NORMAL;            -- Balance safety/performance
PRAGMA wal_autocheckpoint=1000;       -- Auto-checkpoint every 1000 pages
PRAGMA foreign_keys=ON;               -- Referential integrity
PRAGMA busy_timeout=5000;             -- Wait 5s for locks
```

### Query Optimization

```rust
// Use prepared statements for repeated queries
let mut stmt = conn.prepare("SELECT * FROM users WHERE id = ?")?;
for id in user_ids {
    stmt.query_map(&[&id], |row| { /* ... */ })?;
}

// Use transactions for bulk operations
let mut tx = conn.transaction()?;
for item in items {
    tx.execute("INSERT INTO items VALUES (?)", &[&item])?;
}
tx.commit()?;

// Use indices for filtered queries
conn.execute("CREATE INDEX idx_users_email ON users(email)", &[])?;
```

### Performance Metrics

| Operation | Without Pool | With Pool | Improvement |
|-----------|-------------|-----------|-------------|
| Connection acquisition | ~50-100ms | ~0-5ms | **10-20x faster** |
| Key retrieval (cached) | ~50-100ms | ~0ms | **∞ faster** |
| Concurrent reads (WAL) | Serialized | Parallel | **N-way speedup** |
| Bulk inserts (tx) | ~100ms/row | ~1ms/row | **100x faster** |

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_encrypted_storage() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let config = StorageConfig::new(db_path.clone())
            .with_key_source(KeySource::direct(
                "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            ));

        let storage = StorageManager::with_sqlcipher(&db_path, config).unwrap();

        let conn = storage.get_connection().unwrap();
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", &[]).unwrap();

        // Verify data is encrypted on disk
        let raw_bytes = std::fs::read(&db_path).unwrap();
        assert!(!raw_bytes.starts_with(b"SQLite format"));
    }

    #[test]
    fn test_wrong_key_detection() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // Create with one key
        {
            let config = StorageConfig::new(db_path.clone())
                .with_key_source(KeySource::direct("key1_64_chars_aaa..."));
            let storage = StorageManager::with_sqlcipher(&db_path, config).unwrap();
            let conn = storage.get_connection().unwrap();
            conn.execute("CREATE TABLE test (id INTEGER)", &[]).unwrap();
        }

        // Try to open with wrong key
        let config = StorageConfig::new(db_path)
            .with_key_source(KeySource::direct("key2_64_chars_bbb..."));
        let result = StorageManager::with_sqlcipher(&db_path, config);

        assert!(matches!(result, Err(StorageError::WrongKeyOrNotEncrypted)));
    }
}
```

### Integration Tests

Place in `agent/tests/common/storage.rs`:

```rust
use agent::common::storage::*;
use agent::storage::StorageManager;
use tempfile::TempDir;

#[test]
fn test_concurrent_access() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let config = StorageConfig::new(db_path.clone())
        .with_pool_size(10)
        .with_key_source(KeySource::direct("test_key_64_chars_aaa..."));

    let storage = Arc::new(StorageManager::with_sqlcipher(&db_path, config).unwrap());

    // Create table
    {
        let conn = storage.get_connection().unwrap();
        conn.execute("CREATE TABLE counter (value INTEGER)", &[]).unwrap();
        conn.execute("INSERT INTO counter VALUES (0)", &[]).unwrap();
    }

    // Spawn concurrent threads
    let mut handles = vec![];
    for _ in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let handle = std::thread::spawn(move || {
            let conn = storage_clone.get_connection().unwrap();
            conn.execute("UPDATE counter SET value = value + 1", &[]).unwrap();
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Verify counter
    let conn = storage.get_connection().unwrap();
    let value: i64 = conn.query_row("SELECT value FROM counter", &[], |row| row.get(0)).unwrap();
    assert_eq!(value, 10);
}
```

### Test Utilities

```rust
// Helper for creating test storage
pub fn create_test_storage() -> (TempDir, StorageManager) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let config = StorageConfig::new(db_path.clone())
        .with_key_source(KeySource::direct(
            "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        ));

    let storage = StorageManager::with_sqlcipher(&db_path, config).unwrap();

    (temp_dir, storage)
}

// Usage in tests
#[test]
fn my_test() {
    let (_temp_dir, storage) = create_test_storage();
    // _temp_dir ensures directory lives for duration of test
    let conn = storage.get_connection().unwrap();
    // ... test logic
}
```

## Troubleshooting

### Issue: "Wrong key or database not encrypted"

```rust
// Cause: Database encrypted with different key
// Solution: Ensure you're using the correct key source

// Check key in keychain
let key = get_or_create_key(&KeySource::keychain("PulseArc", "db_key"))?;
println!("Key length: {}", key.len()); // Should be 64

// Verify encryption
verify_encryption(&conn)?;

// If key is lost, database cannot be recovered
// Always backup encryption keys securely!
```

### Issue: "Pool exhausted" / "Connection timeout"

```rust
// Cause: All connections in use or leaked
// Solution: Increase pool size or fix connection leaks

// 1. Check for leaked connections
// Ensure connections are dropped after use
{
    let conn = storage.get_connection()?;
    // Use conn...
} // <-- Connection returned to pool here

// 2. Increase pool size
let config = StorageConfig::new(db_path)
    .with_pool_size(20); // Increase from default 10

// 3. Increase timeout
let config = StorageConfig::new(db_path)
    .with_connection_timeout(Duration::from_secs(10));

// 4. Check metrics
let metrics = pool.metrics();
println!("Timeouts: {}", metrics.connections_timeout);
println!("Errors: {}", metrics.connections_error);
```

### Issue: "Circuit breaker open"

```rust
// Cause: Multiple consecutive connection failures
// Solution: Wait for recovery period or investigate root cause

// Check health status
let health = pool.health_check()?;
if !health.healthy {
    println!("Unhealthy: {:?}", health.message);
}

// Circuit breaker will automatically close after:
// 1. Timeout period (30 seconds)
// 2. Successful test requests in half-open state

// Or restart the pool
drop(pool);
let pool = SqlCipherPool::new(db_path, key, config)?;
```

### Issue: Database locked errors

```rust
// Cause: Long-running transactions or high contention
// Solution: Increase busy timeout or use WAL mode

let config = StorageConfig::new(db_path)
    .with_busy_timeout(Duration::from_millis(10000)) // Wait 10s
    .with_enable_wal(true); // Enable WAL for concurrency

// Also: Keep transactions short
let mut tx = conn.transaction()?;
// Do minimal work here
tx.commit()?;
```

### Issue: Performance degradation

```rust
// Check and optimize:

// 1. Pool metrics
let metrics = pool.metrics();
println!("Avg acquisition time: {}ms", metrics.avg_acquisition_time_ms);

// 2. Increase cache size
conn.execute("PRAGMA cache_size = -64000", &[])?; // 64MB

// 3. Analyze queries
conn.execute("EXPLAIN QUERY PLAN SELECT ...", &[])?;

// 4. Add indices
conn.execute("CREATE INDEX idx_name ON table(column)", &[])?;

// 5. Use transactions for bulk operations
let mut tx = conn.transaction()?;
for item in items {
    tx.execute("INSERT INTO table VALUES (?)", &[&item])?;
}
tx.commit()?;
```

## Migration from Legacy Code

### From macos/db to agent/common/storage

```rust
// Before (macos/db/manager.rs)
use crate::db::manager::DbManager;

let db = DbManager::new(db_path).await?;
let conn = db.get_connection().await?;

// After (agent/common/storage)
use agent::common::storage::{StorageConfig, KeySource};
use agent::storage::StorageManager;

let config = StorageConfig::new(db_path)
    .with_key_source(KeySource::keychain("PulseArc", "db_key"));

let storage = StorageManager::with_sqlcipher(&db_path, config)?;
let conn = storage.get_connection()?;
```

### Key Changes

1. **No async** - Storage is synchronous (SQLite is sync)
2. **Explicit configuration** - Use `StorageConfig` instead of implicit defaults
3. **Key sources** - Explicitly specify keychain, environment, or direct
4. **Error types** - Use `StorageError` instead of generic errors
5. **Pool management** - Connection pool is built-in, not manual

## API Reference

### Core Types

- **`StorageConfig`** - Storage configuration with builder pattern
- **`KeySource`** - Encryption key source (Keychain/Environment/Direct)
- **`StorageError`** - Rich error type with classification
- **`SqlCipherPool`** - Connection pool with circuit breaker
- **`SqlCipherConnection`** - Pooled encrypted connection
- **`Transaction`** - ACID transaction wrapper

### Key Functions

- `StorageConfig::new(path)` → `StorageConfig` - Create configuration
- `StorageConfig::builder(path)` → `StorageConfigBuilder` - Builder pattern
- `SqlCipherPool::new(path, key, config)` → `Result<SqlCipherPool>` - Create pool
- `pool.get_sqlcipher_connection()` → `Result<SqlCipherConnection>` - Get connection
- `conn.execute(sql, params)` → `Result<usize>` - Execute statement
- `conn.query_row(sql, params, f)` → `Result<T>` - Query single row
- `conn.prepare(sql)` → `Result<Statement>` - Prepare statement
- `conn.transaction()` → `Result<Transaction>` - Begin transaction

### Traits

- **`ConnectionPool`** - Platform-agnostic pool interface
- **`Connection`** - Platform-agnostic connection interface
- **`Statement`** - Prepared statement interface
- **`ErrorClassification`** - Error severity and retry semantics

## Security Best Practices

1. **Always use keychain in production** - Never hardcode keys
2. **Rotate keys periodically** - Use `StorageKeyManager`
3. **Validate configuration** - Use builder pattern with `.build()?`
4. **Monitor critical errors** - Check `err.is_critical()`
5. **Verify encryption** - Call `verify_encryption()` after setup
6. **Secure key memory** - Keys are `SecureString` with auto-zeroization
7. **Audit key operations** - All key operations are logged via tracing
8. **Backup encryption keys** - Lost keys = unrecoverable data

## Contributing

When contributing to this module:

1. Run tests: `cargo test -p agent --lib common::storage`
2. Add tests for new functionality
3. Update this README for new features
4. Follow existing patterns (builder, error classification)
5. Use `SecureString` for all key material
6. Add tracing spans for observable operations
7. Maintain backward compatibility or document breaking changes

## License

Part of the PulseArc platform - see main LICENSE file.

## See Also

- [`agent/common/security/encryption/`](../security/encryption/README.md) - Encryption and key management
- [`agent/storage/`](../../storage/README.md) - High-level storage manager
- [`agent/common/resilience/`](../resilience/) - Circuit breaker patterns
- [`agent/common/error/`](../error/README.md) - Error classification system
- [SQLCipher Documentation](https://www.zetetic.net/sqlcipher/documentation/)
- [rusqlite Documentation](https://docs.rs/rusqlite/)
- [r2d2 Documentation](https://docs.rs/r2d2/)
