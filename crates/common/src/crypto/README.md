# Crypto Module

Low-level symmetric cryptography primitives that power PulseArc runtime features. The code in this folder focuses on **AES-256-GCM encryption, password-derived keys, key rotation, and encrypted key persistence**. Higher-level key lifecycle concerns are layered on top in other crates/modules (`security::encryption`, queue persistence, etc.).

## Directory Layout
- `mod.rs` – Exposes the `encryption` module and re-exports the primary types.
- `encryption.rs` – Implements the `EncryptionService`, `EncryptedData`, and disk key-storage utilities.

## Feature Flags & Dependencies
The module lives in the `pulsearc-common` crate and requires the `runtime` feature flag because it relies on optional dependencies:
- `aes-gcm` for AES-256-GCM authenticated encryption
- `argon2` for password-based key derivation
- `base64` for string encoding of payloads
- `rand_core` (via `argon2`) for generating keys and nonces

Enable the feature when adding the crate:

```toml
pulsearc-common = { path = "crates/common", features = ["runtime"] }
```

## Data Model: `EncryptedData`
`EncryptedData` is a serde-serializable container shared across the runtime:
- `nonce` (`Vec<u8>`): 12-byte AES-GCM nonce generated via `OsRng`.
- `ciphertext` (`Vec<u8>`): Raw encrypted bytes (96-bit tag included by `aes-gcm`).
- `salt` (`Option<String>`): Base64-encoded salt string only present for password-derived keys.
- `algorithm` (`String`): Currently always `"AES-256-GCM"`; future-proofed for migrations.

When used with `encrypt_to_string`, the struct is serialized to JSON and then base64-encoded, producing an opaque string safe for persistence or transmission.

## `EncryptionService` Workflows
Core helper that wraps key material and the AES-GCM cipher instance. Error paths bubble up `CommonError` variants to callers.

### Creating from random key material
```rust
use pulsearc_common::crypto::encryption::EncryptionService;

let key = EncryptionService::generate_key(); // 32 random bytes from OsRng
let service = EncryptionService::new(key.clone())?;
let fingerprint = service.key_fingerprint(); // SHA-256(first 8 bytes) for logging/telemetry
```

### Deriving keys from passwords (Argon2)
```rust
let password_service = EncryptionService::from_password("correct horse battery staple")?;
let secret = password_service.encrypt(b"persist me")?;
assert!(secret.salt.is_some(), "password workflows must embed the derived salt");

// Persist the salt and reuse it to avoid breaking existing ciphertexts
let salt = secret.salt.clone().expect("salt should exist");
let rehydrated =
    EncryptionService::from_password_with_salt("correct horse battery staple", Some(&salt))?;
```

### Encrypting, decrypting, and string round-trips
```rust
let encrypted = service.encrypt(b"payload bytes")?;
let decrypted = service.decrypt(&encrypted)?;

// Base64 string workflow (JSON payload + base64)
let encoded = service.encrypt_to_string(b"payload bytes")?;
let decoded = service.decrypt_from_string(&encoded)?;
assert_eq!(decoded.as_slice(), b"payload bytes");
```

### Rotating keys & migrating payloads
```rust
let mut active = EncryptionService::new(EncryptionService::generate_key())?;
let rotated = EncryptionService::new(EncryptionService::generate_key())?;

let ciphertext = active.encrypt(b"rotation target")?;

// Move ciphertext to the new key
let migrated = active.reencrypt(&ciphertext, &rotated)?;
let cleartext = rotated.decrypt(&migrated)?;

// Permanently switch to the new key
active.rotate_key(rotated.generate_key())?;
```

`rotate_key` clears any password salt because the service now operates on raw key material. Always rehydrate password-derived services with `from_password_with_salt` instead of calling `rotate_key`.

## Key Persistence Helpers (`key_storage`)
`key_storage::{save_key, load_key}` wrap `EncryptionService` password flows to persist symmetric keys to disk.

```rust
use pulsearc_common::crypto::encryption::key_storage;
use std::path::Path;

let key = EncryptionService::generate_key();
let path = Path::new("~/.pulsearc/encrypted.key");
let master_password = std::env::var("PULSEARC_MASTER_PASSWORD")?;

key_storage::save_key(&key, path, &master_password)?;
let recovered = key_storage::load_key(path, &master_password)?;
assert_eq!(recovered, key);
```

The file contains a base64 string. Decoding the string yields the JSON representation of `EncryptedData`. Attempts to load with the wrong password surface a `CommonError`.

## Integration Points
- `pulsearc_common::security::encryption`: High-level key caching, rotation policies, and keychain adapters built on top of these primitives.
- `pulsearc_common::sync::queue::encryption`: Thin wrapper that adapts the shared service to queue-specific error types.
- Benchmarks (`crates/common/benches/crypto_bench.rs`) and integration tests (`crates/common/tests/crypto_integration.rs`) ensure regressions are caught across performance and correctness boundaries.

## Testing & Benchmarking
- Unit tests live inline in `encryption.rs`. Run them with:
  ```bash
  cargo test -p pulsearc-common --features runtime --lib crypto::encryption
  ```
- Integration tests require the same feature set:
  ```bash
  cargo test -p pulsearc-common --features runtime --test crypto_integration
  ```
- Benchmarks (Criterion) are gated behind the `runtime` feature:
  ```bash
  cargo bench --bench crypto_bench --features runtime
  ```

## Security Notes & Maintenance Tips
- Keys **must** be exactly 32 bytes; `EncryptionService::new` enforces this and returns a `CommonError` otherwise.
- Nonces are randomly generated (12 bytes) for every encryption call; no caller state is required.
- `key_fingerprint` is for diagnostics only and should not be treated as a secure hash for comparison.
- Password-derived flows embed the salt in the payload; always persist the salt alongside the ciphertext or re-serialize using `encrypt_to_string`.
- To tweak Argon2 parameters (memory, iterations), adjust the `argon2::Argon2` initialization in `EncryptionService::from_password_with_salt`.
- Any future algorithms should extend `EncryptedData::algorithm` and add compatibility logic so older payloads continue to decrypt.
