//! Keychain module - re-exports from encryption
//!
//! This module provides a compatibility layer for code that imports
//! keychain types directly from `security::keychain` instead of
//! `security::encryption::keychain`.

pub use super::encryption::keychain::{KeychainError, KeychainProvider};
