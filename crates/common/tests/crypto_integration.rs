//! Integration tests for crypto module
//!
//! Validates symmetric encryption primitives, password-derived workflows,
//! re-encryption flows, and persistent key storage helpers.

#![cfg(feature = "runtime")]

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use pulsearc_common::crypto::encryption::key_storage;
use pulsearc_common::{EncryptedData, SymmetricEncryptionService};
use tempfile::NamedTempFile;

/// End-to-end round-trip using generated symmetric keys across distinct service
/// instances.
#[test]
fn symmetric_encryption_round_trip_across_instances() {
    let key = SymmetricEncryptionService::generate_key();
    let encryptor = SymmetricEncryptionService::new(key.clone()).expect("failed to init encryptor");
    let decryptor =
        SymmetricEncryptionService::new(key).expect("failed to init decryptor with same key");

    let plaintext = b"crypto integration payload";
    let encrypted = encryptor.encrypt(plaintext).expect("encrypt should succeed");

    assert_eq!(encrypted.algorithm, "AES-256-GCM");
    assert_eq!(encrypted.nonce.len(), 12, "AES-GCM nonce should be 96 bits");
    assert!(encrypted.salt.is_none(), "raw symmetric keys should not embed a password salt");
    assert!(!encrypted.ciphertext.is_empty());

    let decrypted = decryptor.decrypt(&encrypted).expect("decrypt should succeed");
    assert_eq!(decrypted.as_slice(), plaintext);
}

/// Verifies password-derived encryption embeds the salt and supports
/// string-based workflows.
#[test]
fn password_derived_encryption_preserves_salt_in_payload() {
    let service = SymmetricEncryptionService::from_password("correct horse battery staple")
        .expect("password-based service should initialize");

    let secret = b"persist-by-password";
    let encoded = service.encrypt_to_string(secret).expect("string encryption should succeed");
    let decoded = BASE64.decode(&encoded).expect("base64 decode should succeed");
    let payload: EncryptedData =
        serde_json::from_slice(&decoded).expect("serialized payload should deserialize");

    assert!(payload.salt.is_some(), "password-based encryption must embed its salt");
    assert_eq!(payload.algorithm, "AES-256-GCM");

    let round_trip = service
        .decrypt_from_string(&encoded)
        .expect("string decryption should succeed with same service instance");
    assert_eq!(round_trip.as_slice(), secret);
}

/// Ensures encrypted payloads can be re-encrypted with a new symmetric key.
#[test]
fn reencrypts_payload_between_services() {
    let key_a = SymmetricEncryptionService::generate_key();
    let key_b = SymmetricEncryptionService::generate_key();

    let service_a = SymmetricEncryptionService::new(key_a).expect("service A should initialize");
    let service_b = SymmetricEncryptionService::new(key_b).expect("service B should initialize");

    let plaintext = b"rotate-me";
    let encrypted = service_a.encrypt(plaintext).expect("service A encrypts");
    let migrated =
        service_a.reencrypt(&encrypted, &service_b).expect("re-encryption should succeed");

    assert!(migrated.salt.is_none(), "raw key rotation should not embed a salt");

    let decrypted = service_b.decrypt(&migrated).expect("service B decrypts");
    assert_eq!(decrypted.as_slice(), plaintext);
}

/// Validates persistent key storage end-to-end with password rehydration and
/// error handling.
#[test]
fn key_storage_round_trip_with_password() {
    let file = NamedTempFile::new().expect("temp file should be created");
    let path = file.path();
    let key = SymmetricEncryptionService::generate_key();
    let master_password = "crypto-integration-master-password";

    key_storage::save_key(&key, path, master_password).expect("key should persist to disk");

    let loaded =
        key_storage::load_key(path, master_password).expect("stored key should be recoverable");
    assert_eq!(loaded, key, "round-trip recovery should yield the original key material");

    let wrong_password = key_storage::load_key(path, "definitely-wrong");
    assert!(wrong_password.is_err(), "incorrect password should not decrypt stored key");
}
