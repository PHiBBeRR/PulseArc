//! Security primitives and utilities
//!
//! This module provides generic security components that can be used
//! across different domains and applications.

pub mod encryption;
pub mod keychain;
pub mod rbac;
pub mod traits;

// Re-export keychain types from encryption
pub use encryption::{KeychainError, KeychainProvider};
// Re-export commonly used types from rbac
pub use rbac::{
    Permission, PolicyCondition, PolicyEffect, RBACManager, RBACPolicy, Role, UserContext,
};
// Re-export trait abstractions
pub use traits::{
    AccessControl, DenyAllAccessControl, NoOpAccessControl, NoOpValidator,
    Permission as TraitPermission, UserContext as TraitUserContext, ValidationError,
    ValidationResult, Validator,
};
