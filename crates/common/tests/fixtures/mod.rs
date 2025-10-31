//! Test fixtures for integration tests
#![allow(unused_imports, dead_code, clippy::new_ret_no_self)]
//!
//! This module provides reusable test fixtures for security integration tests.
//!
//! ## Fixtures (Test Data Builders)
//! - [`mock_encryption_keys`] - Encryption key fixtures and performance
//!   measurement
//! - [`mock_rbac_users`] - RBAC user context builders and common user types
//! - [`mock_rbac_permissions`] - RBAC permission fixtures
//! - [`mock_rbac_policies`] - RBAC policy builders and RBAC manager fixtures
//!
//! ## Note on Mocks
//! Mock implementations (MockClock, MockKeychainProvider, MockOAuthClient) are
//! available from `pulsearc_common::testing` module.

// ============================================================================
// FIXTURES - Test data builders and factories
// ============================================================================

/// Encryption key fixtures and performance measurement
pub mod mock_encryption_keys;

/// RBAC user context builders and common user types
pub mod mock_rbac_users;

/// RBAC permission fixtures
pub mod mock_rbac_permissions;

/// RBAC policy builders and RBAC manager fixtures
pub mod mock_rbac_policies;

// ============================================================================
// Re-exports for convenience
// ============================================================================

// Encryption fixtures
pub use mock_encryption_keys::{EncryptionKeyFixture, PerformanceMeasurement};
// RBAC permission fixtures
pub use mock_rbac_permissions::{
    generate_test_permissions, PermissionBuilder, PermissionFixture, TraitPermissionFixture,
};
// RBAC policy fixtures
pub use mock_rbac_policies::{PolicyBuilder, PolicyFixture, RBACFixture};
// RBAC user fixtures
pub use mock_rbac_users::{
    generate_test_users, TraitUserContextFixture, UserContextBuilder, UserContextFixture,
};
