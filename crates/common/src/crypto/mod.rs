//! Shared cryptographic primitives used across runtime and platform features.

pub mod encryption;

pub use encryption::{EncryptedData, EncryptionService};
