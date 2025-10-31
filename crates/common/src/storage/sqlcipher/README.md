# Storage SQLCipher

SQLCipher integration for encrypted database storage.

## Overview

Provides encrypted SQLite database using SQLCipher, ensuring all data is encrypted at rest. Handles key management, pragmas, and secure connection pooling.

## Features

- **Transparent Encryption**: All data encrypted at rest
- **Key Management**: Secure key derivation and storage
- **Performance Tuning**: Optimized SQLCipher pragmas
- **Connection Pooling**: Encrypted connection pool management
- **Key Rotation**: Support for rotating encryption keys

## Components

### SQLCipher Config (`config.rs`)

Configuration for encrypted database:

```rust
use agent::storage::sqlcipher::{SqlCipherConfig, CipherVersion};

let config = SqlCipherConfig {
    database_path: PathBuf::from("data.db"),
    cipher_version: CipherVersion::V4,
    kdf_iter: 256000,
    page_size: 4096,
    cipher_page_size: 4096,
    hmac_algorithm: HmacAlgorithm::Sha512,
    kdf_algorithm: KdfAlgorithm::Sha512,
    compatibility_mode: false,
};
```

### Connection Manager (`connection.rs`)

Manage encrypted database connections:

```rust
use agent::storage::sqlcipher::ConnectionManager;

let manager = ConnectionManager::new(config)?;

// Open encrypted connection
let conn = manager.connect(encryption_key)?;

// Execute query on encrypted database
conn.execute("CREATE TABLE secrets (data BLOB)", [])?;
```

### Connection Pool (`pool.rs`)

Pooled encrypted connections:

```rust
use agent::storage::sqlcipher::EncryptedPool;

let pool = EncryptedPool::new(config, encryption_key)?
    .max_connections(10)
    .min_idle(2)
    .connection_timeout(Duration::from_secs(30))
    .build()?;

// Get connection from pool
let conn = pool.get()?;

// Use connection
conn.execute("INSERT INTO secrets VALUES (?)", params![data])?;

// Automatically returned to pool on drop
```

### Pragmas Configuration (`pragmas.rs`)

SQLCipher-specific pragmas:

```rust
use agent::storage::sqlcipher::Pragmas;

let pragmas = Pragmas::new()
    .cipher_version(4)
    .kdf_iter(256000)
    .cipher_page_size(4096)
    .cipher_hmac_algorithm("HMAC_SHA512")
    .cipher_kdf_algorithm("PBKDF2_HMAC_SHA512")
    .cipher_memory_security(true)
    .build();

conn.apply_pragmas(&pragmas)?;
```

## Usage

### Basic Encrypted Database

```rust
use agent::storage::sqlcipher::{EncryptedDatabase, SqlCipherConfig};
use keyring::Entry;

// Get encryption key from keyring
let entry = Entry::new("pulsearc-agent", "db-key")?;
let key = entry.get_password()?;

// Open encrypted database
let db = EncryptedDatabase::open("data.db", &key)?;

// Use normally
db.execute("CREATE TABLE IF NOT EXISTS activities (...)", [])?;
```

### Key Derivation

Derive database key from master password:

```rust
use agent::storage::sqlcipher::KeyDerivation;

let kdf = KeyDerivation::new()
    .algorithm(KdfAlgorithm::Argon2id)
    .iterations(3)
    .memory_cost(65536)
    .parallelism(4)
    .build();

let db_key = kdf.derive_key(
    master_password.as_bytes(),
    salt,
    32  // 256-bit key
)?;
```

### Key Storage

Securely store encryption keys:

```rust
use keyring::Entry;

// Store in OS keychain
let entry = Entry::new("pulsearc-agent", "db-encryption-key")?;
entry.set_password(&encryption_key)?;

// Retrieve later
let key = entry.get_password()?;
```

### Key Rotation

Rotate encryption keys:

```rust
use agent::storage::sqlcipher::KeyRotation;

let rotation = KeyRotation::new(db_path)?;

// Rekey database with new key
rotation.rekey(old_key, new_key)?;

// Or change KDF parameters
rotation.change_kdf_params(old_key, new_kdf_params)?;
```

## SQLCipher Versions

### Version 4 (Recommended)

Latest version with best security:

```rust
config.cipher_version = CipherVersion::V4;
config.kdf_iter = 256000;
config.cipher_page_size = 4096;
```

**Features**:
- PBKDF2-HMAC-SHA512 KDF
- SHA512 HMAC
- 256,000 PBKDF2 iterations
- 4096-byte pages

### Version 3

For compatibility:

```rust
config.cipher_version = CipherVersion::V3;
config.kdf_iter = 64000;
```

### Migration Between Versions

```rust
let migrator = SqlCipherMigrator::new();

// Migrate from v3 to v4
migrator.migrate_version(
    "data.db",
    key,
    CipherVersion::V3,
    CipherVersion::V4
)?;
```

## Performance Tuning

### Page Size

Larger pages = fewer pages = better performance:

```rust
// Default: 4096
config.cipher_page_size = 8192;  // Better for larger databases
```

### Cache Size

More cache = fewer disk reads:

```sql
PRAGMA cache_size = -64000;  // 64MB cache
```

### Memory Security

Trade performance for security:

```sql
PRAGMA cipher_memory_security = ON;   -- Secure but slower
PRAGMA cipher_memory_security = OFF;  -- Faster but less secure
```

### KDF Iterations

More iterations = slower but more secure:

```rust
// Default: 256,000
config.kdf_iter = 512000;  // More secure but slower to open
```

## Security Considerations

### Key Management

- **Never hardcode keys** in source code
- **Use OS keychain** for key storage
- **Derive keys** from user passwords using strong KDF
- **Rotate keys** periodically
- **Secure key memory** with zeroize

### Secure Connection Setup

```rust
use zeroize::Zeroizing;

let key = Zeroizing::new(get_encryption_key()?);

let conn = EncryptedDatabase::open("data.db", &key)?;

// Key is zeroed when dropped
```

### Prevent Key Leakage

```rust
use agent::storage::sqlcipher::SecureConnection;

let conn = SecureConnection::open(db_path, key)?;

// Prevents key from appearing in:
// - Debug output
// - Error messages
// - Logs
// - Core dumps
```

## Error Handling

Handle SQLCipher-specific errors:

```rust
use agent::storage::sqlcipher::SqlCipherError;

match EncryptedDatabase::open("data.db", wrong_key) {
    Err(SqlCipherError::WrongKey) => {
        eprintln!("Incorrect encryption key");
        // Prompt for correct key
    }
    Err(SqlCipherError::CorruptedDatabase) => {
        eprintln!("Database file corrupted");
        // Restore from backup
    }
    Err(SqlCipherError::IncompatibleVersion { expected, found }) => {
        eprintln!("Database version mismatch: {} vs {}", expected, found);
        // Run migration
    }
    Err(e) => eprintln!("Database error: {}", e),
    Ok(db) => { /* ... */ }
}
```

## Testing

### Test with Encryption

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypted_storage() {
        let temp_dir = tempfile::tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let key = "test-key-32-bytes-long-string!";

        let db = EncryptedDatabase::open(&db_path, key).unwrap();
        db.execute("CREATE TABLE test (data TEXT)", []).unwrap();
        db.execute("INSERT INTO test VALUES ('secret')", []).unwrap();

        // Verify data is encrypted on disk
        let raw_bytes = std::fs::read(&db_path).unwrap();
        assert!(!raw_bytes.windows(6).any(|w| w == b"secret"));

        // Verify can read with correct key
        let db2 = EncryptedDatabase::open(&db_path, key).unwrap();
        let data: String = db2.query_row(
            "SELECT data FROM test",
            [],
            |row| row.get(0)
        ).unwrap();
        assert_eq!(data, "secret");

        // Verify cannot read with wrong key
        assert!(EncryptedDatabase::open(&db_path, "wrong-key").is_err());
    }
}
```

## Benchmarking

Compare encrypted vs unencrypted:

```rust
#[bench]
fn bench_encrypted_inserts(b: &mut Bencher) {
    let db = EncryptedDatabase::open(":memory:", "key").unwrap();
    db.execute("CREATE TABLE test (data INTEGER)", []).unwrap();

    b.iter(|| {
        db.execute("INSERT INTO test VALUES (?)", params![42]).unwrap();
    });
}
```

## Troubleshooting

### Database Won't Open

- Check key is correct
- Verify SQLCipher version compatibility
- Check file permissions
- Ensure SQLCipher is compiled correctly

### Performance Issues

- Increase cache size
- Adjust page size
- Reduce KDF iterations (carefully)
- Use connection pooling
- Consider disabling memory security for non-sensitive data

### Migration Issues

- Backup database first
- Use correct source/dest versions
- Verify key before migration
- Test on copy first

## See Also

- [storage/](../) - Main storage module
- [storage/encryption/](../encryption/) - Additional encryption features
- [SQLCipher Documentation](https://www.zetetic.net/sqlcipher/documentation/)
