//! Encryption module for secure key management
//!
//! This module provides **high-level key management infrastructure** built on
//! top of the cryptographic primitives in `crypto::encryption`.
//!
//! ## Features
//!
//! - **Key Caching**: Thread-safe in-memory caching of encryption keys
//! - **Key Rotation**: Scheduled key rotation with configurable policies
//! - **Keychain Integration**: Platform keychain storage for persistent keys
//! - **Secure Strings**: Memory-safe string handling with automatic zeroing
//! - **Key Providers**: Unified interface for key retrieval and creation
//!
//! ## Module Relationships
//!
//! - **`crypto::encryption`**: Low-level AES-256-GCM encryption primitives
//! - **`security::encryption`** (this module): High-level key management
//!   infrastructure
//!
//! This separation allows the crypto module to remain focused on cryptographic
//! operations while this module handles the operational aspects of key
//! lifecycle management.

pub mod cache;
pub mod key_provider;
pub mod key_rotation;
pub mod keychain;
pub mod keys;
pub mod rotation;
pub mod secure_string;

// Re-export commonly used types
pub use cache::{
    clear_cache, clear_cache_on_security_event, get_cache_stats, get_or_create_key_cached,
    is_cached, CacheStats,
};
pub use key_provider::get_or_create_key;
pub use key_rotation::KeyRotationSchedule;
pub use keychain::{KeychainError, KeychainProvider};
pub use keys::generate_encryption_key;
pub use rotation::StorageKeyManager;
pub use secure_string::SecureString;

pub use crate::crypto::encryption::{
    EncryptedData, EncryptionService as SymmetricEncryptionService,
};
