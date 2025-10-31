# Storage Encryption Module

Unified encryption system for SQLCipher database encryption with enterprise features including key rotation, caching, and comprehensive audit logging.

## Features

✅ **Secure Memory Handling** - Automatic zeroization prevents key leakage
✅ **Key Rotation** - Automatic rotation with SQLCipher rekey support
✅ **Multiple Key Sources** - Keychain, Environment, or Direct
✅ **Performance Caching** - Thread-safe key caching with OnceLock
✅ **Full Audit Trail** - Integration with compliance audit system
✅ **Metrics Tracking** - Prometheus metrics for monitoring
✅ **Cross-Platform** - macOS Keychain, Windows Credential Manager, Linux Secret Service

## Architecture

### Core Components

```
storage/encryption/
├── secure_string.rs      # SecureString with automatic zeroization
├── key_rotation.rs       # KeyRotationSchedule for timing logic
├── keys.rs               # Cryptographic key generation
├── keychain.rs           # Cross-platform keychain integration
├── cipher.rs             # SQLCipher configuration
├── rotation.rs           # StorageKeyManager with rekey support
├── cache.rs              # OnceLock-based key caching
└── audit.rs              # EncryptionAuditLogger
```

## Usage

### Basic Setup

```rust
use agent::storage::encryption::{
    get_or_create_key,
    SqlCipherConfig,
    configure_sqlcipher
};
use agent::storage::config::KeySource;
use rusqlite::Connection;

// Get key from keychain (recommended for production)
let key_source = KeySource::Keychain {
    service: "PulseArc".to_string(),
    username: "db_encryption_key".to_string(),
};

let key = get_or_create_key(&key_source)?;
let config = SqlCipherConfig::from_secure_key(key);

// Configure database
let conn = Connection::open("encrypted.db")?;
configure_sqlcipher(&conn, &config)?;
```

### With Key Rotation

```rust
use agent::storage::encryption::{StorageKeyManager, KeyRotationSchedule};

// Create key manager with custom schedule
let mut schedule = KeyRotationSchedule::default();
schedule.set_rotation_days(30); // Rotate every 30 days

let mut key_manager = StorageKeyManager::with_schedule(initial_key, schedule);

// Check and rotate if needed
if key_manager.should_rotate() {
    key_manager.rotate_key(&conn)?;
}

// Or force immediate rotation
key_manager.force_rotate(&conn)?;
```

### With Caching

```rust
use agent::storage::encryption::cache::get_or_create_key_cached;

// First call - retrieves from keychain
let key1 = get_or_create_key_cached(&key_source)?;

// Second call - instant (cached)
let key2 = get_or_create_key_cached(&key_source)?;

// Both are the same reference
assert!(std::ptr::eq(key1, key2));
```

### With Audit Logging

```rust
use agent::storage::encryption::audit::EncryptionAuditLogger;

let audit_logger = EncryptionAuditLogger::new();

// Log key generation
audit_logger.log_key_generation("keychain").await?;

// Log key rotation
audit_logger.log_key_rotation(true, None, false).await?;

// Log errors
audit_logger.log_encryption_error("config", "Failed to set key").await?;
```

## Security Best Practices

### Key Sources by Environment

| Environment | Recommended Source | Reason |
|-------------|-------------------|---------|
| **Production** | `KeySource::Keychain` | Most secure, OS-managed |
| **CI/Testing** | `KeySource::Environment` | No interactive auth needed |
| **Development** | `KeySource::Direct` ⚠️ | Convenience only, not secure |

### Key Rotation

```rust
// Configure aggressive rotation for high-security scenarios
let mut schedule = KeyRotationSchedule::new(30); // 30 days
key_manager.set_rotation_schedule(schedule);

// Monitor rotation status
println!("Days since last rotation: {}",
         key_manager.days_since_last_rotation());
```

### Memory Safety

The `SecureString` type automatically zeroes memory on drop:

```rust
{
    let key = get_or_create_key(&key_source)?;
    // Use key...
} // <-- Memory is zeroed here automatically
```

## Configuration

### SQLCipher Settings

```rust
let config = SqlCipherConfig::new("your-key-here".to_string())
    .with_cipher_compatibility(4)           // SQLCipher 4.x
    .with_kdf_iter(256000)                  // Strong KDF
    .without_memory_security();             // Not recommended

// Or use from SecureString
let config = SqlCipherConfig::from_secure_key(secure_key);
```

### Custom Rotation Schedule

```rust
let mut schedule = KeyRotationSchedule::new(90);
schedule.record_rotation(); // Mark as rotated now

// Check if rotation needed
if schedule.should_rotate() {
    // Time to rotate!
}
```

## Metrics

The module tracks the following Prometheus metrics:

### Success Metrics
- `sqlcipher_config{status="success"}` - Successful SQLCipher configurations
- `encryption_verify{status="success"}` - Successful encryption verifications
- `storage_key_rotation{status="completed"}` - Successful key rotations
- `storage_key_rotation{status="forced"}` - Forced rotations
- `database_rekey{status="success"}` - Successful database rekeys

### Error Metrics
- `sqlcipher_config{error_type="*"}` - Configuration failures
- `encryption_verify{error_type="failed"}` - Verification failures
- `storage_key_rotation{error_type="failed"}` - Rotation failures
- `database_rekey{error_type="*"}` - Rekey failures

### Performance Metrics
- `sqlcipher_config_duration_ms` - Configuration time
- `storage_key_rotation_duration_ms` - Rotation time

## Error Handling

```rust
use agent::storage::error::StorageError;

match configure_sqlcipher(&conn, &config) {
    Ok(_) => println!("Encryption configured"),
    Err(StorageError::WrongKeyOrNotEncrypted) => {
        eprintln!("Wrong encryption key or database not encrypted");
    }
    Err(StorageError::Encryption(msg)) => {
        eprintln!("Encryption error: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Testing

### Unit Tests

```bash
# Run encryption module tests
cargo test -p agent --lib storage::encryption

# Run specific test
cargo test -p agent --lib storage::encryption::rotation::tests
```

### Integration Tests

```rust
#[test]
fn test_key_rotation_with_data() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create encrypted database
    let initial_key = generate_encryption_key();
    let mut key_manager = StorageKeyManager::new(initial_key.clone());
    let conn = Connection::open(&db_path).unwrap();

    let config = SqlCipherConfig::from_secure_key(initial_key);
    configure_sqlcipher(&conn, &config).unwrap();

    // Insert data
    conn.execute("CREATE TABLE test (id INTEGER, value TEXT)", []).unwrap();
    conn.execute("INSERT INTO test VALUES (1, 'sensitive')", []).unwrap();

    // Rotate key
    key_manager.force_rotate(&conn).unwrap();

    // Verify data still accessible
    let value: String = conn
        .query_row("SELECT value FROM test WHERE id = 1", [], |row| row.get(0))
        .unwrap();

    assert_eq!(value, "sensitive");
}
```

## Performance

### Key Caching Impact

| Operation | Without Cache | With Cache |
|-----------|--------------|-----------|
| First key access | ~50-100ms | ~50-100ms |
| Subsequent access | ~50-100ms | ~0ms |
| **Improvement** | - | **~100x faster** |

### Rotation Performance

Typical key rotation times:
- Small DB (<1MB): ~10-50ms
- Medium DB (1-100MB): ~100-500ms
- Large DB (>100MB): ~500ms-2s

## Troubleshooting

### Issue: "Wrong key or database not encrypted"

```rust
// Ensure you're using the same key that encrypted the database
let key = get_or_create_key(&key_source)?;

// Verify encryption works
verify_encryption(&conn)?;
```

### Issue: "Keychain access denied"

On macOS, ensure your app has keychain access:
```bash
# Grant keychain access
security unlock-keychain
```

### Issue: "Key rotation fails"

```rust
// Check rotation schedule
println!("Should rotate: {}", key_manager.should_rotate());
println!("Days since rotation: {}", key_manager.days_since_last_rotation());

// Force rotation if needed
key_manager.force_rotate(&conn)?;
```

## Migration Guide

### From String to SecureString

**Before:**
```rust
let key: String = get_key();
config.key = key;
```

**After:**
```rust
let key: SecureString = get_key(); // Returns SecureString now
config.key = key; // Already SecureString
```

### Adding Rotation to Existing Code

```rust
// 1. Create key manager
let key = get_or_create_key(&key_source)?;
let mut key_manager = StorageKeyManager::new(key.clone());

// 2. Configure initial encryption
let config = SqlCipherConfig::from_secure_key(key);
configure_sqlcipher(&conn, &config)?;

// 3. Check for rotation periodically
if key_manager.should_rotate() {
    key_manager.rotate_key(&conn)?;
}
```

## API Reference

### Core Types

- **`SecureString`** - Memory-safe string with auto-zeroization
- **`KeyRotationSchedule`** - Manages rotation timing
- **`StorageKeyManager`** - Handles key lifecycle and rotation
- **`SqlCipherConfig`** - SQLCipher configuration
- **`EncryptionAuditLogger`** - Audit logging for compliance

### Key Functions

- `generate_encryption_key()` → `SecureString` - Generate new key
- `get_or_create_key(source)` → `Result<SecureString>` - Get/create key
- `get_or_create_key_cached(source)` → `Result<&'static SecureString>` - Cached version
- `configure_sqlcipher(conn, config)` → `Result<()>` - Configure encryption
- `verify_encryption(conn)` → `Result<()>` - Verify encryption works

## License

Part of the PulseArc platform - see main LICENSE file.

## Contributing

When contributing to this module:
1. Ensure all tests pass: `cargo test -p agent --lib storage::encryption`
2. Add tests for new functionality
3. Update this README if adding new features
4. Follow existing security patterns (use `SecureString`, audit log operations)
5. Add metrics for new operations

## Support

For issues or questions:
- File an issue in the main repository
- Tag with `encryption` or `storage` labels
- Include relevant logs (with sensitive data redacted!)
