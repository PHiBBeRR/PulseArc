//! RBAC Permission Test Fixtures
//!
//! **What this file contains:**
//! Test fixtures for creating RBAC permissions.
//!
//! ## PermissionBuilder
//! Builder for creating custom permissions:
//! - `new(resource, action)` - Create builder for "resource:action" permission
//! - `build()` - Build the permission
//!
//! ## PermissionFixture
//! Pre-configured common permissions:
//! - `menu_view()`, `menu_edit()`, `menu_delete()` - Menu permissions
//! - `config_read()`, `config_write()` - Config permissions
//! - `audit_read()`, `audit_delete()` - Audit permissions
//! - `system_config()` - System configuration
//! - `encryption_access()` - Encryption key access
//! - `custom(resource, action)` - Custom permission
//!
//! ## TraitPermissionFixture
//! Trait-based permissions for trait tests.
//!
//! ## Helpers
//! - `generate_test_permissions(count)` - Generate multiple permissions for
//!   batch testing
//!
//! ## Usage
//! ```rust
//! use fixtures::mock_rbac_permissions::*;
//!
//! // Pre-configured permissions
//! let view = PermissionFixture::menu_view();
//! let delete = PermissionFixture::audit_delete();
//!
//! // Custom permission
//! let custom = PermissionFixture::custom("resource", "action");
//!
//! // Batch generation
//! let perms = generate_test_permissions(100);
//! ```

#![allow(dead_code, clippy::new_ret_no_self)]

use pulsearc_common::security::rbac::Permission;
use pulsearc_common::security::traits::Permission as TraitPermission;

// ============================================================================
// Permission Fixtures
// ============================================================================

/// Builder for Permission test fixtures
pub struct PermissionBuilder {
    permission_str: String,
}

impl PermissionBuilder {
    /// Create a new permission builder
    pub fn new(resource: &str, action: &str) -> Self {
        Self { permission_str: format!("{}:{}", resource, action) }
    }

    /// Build the Permission
    pub fn build(self) -> Permission {
        Permission::new(&self.permission_str)
    }
}

/// Pre-configured permission fixtures
pub struct PermissionFixture;

impl PermissionFixture {
    /// Create a menu:view permission
    pub fn menu_view() -> Permission {
        Permission::new("menu:view")
    }

    /// Create a menu:edit permission
    pub fn menu_edit() -> Permission {
        Permission::new("menu:edit")
    }

    /// Create a menu:delete permission
    pub fn menu_delete() -> Permission {
        Permission::new("menu:delete")
    }

    /// Create a config:read permission
    pub fn config_read() -> Permission {
        Permission::new("config:read")
    }

    /// Create a config:write permission
    pub fn config_write() -> Permission {
        Permission::new("config:write")
    }

    /// Create an audit:read permission
    pub fn audit_read() -> Permission {
        Permission::new("audit:read")
    }

    /// Create an audit:delete permission
    pub fn audit_delete() -> Permission {
        Permission::new("audit:delete")
    }

    /// Create a system:config permission
    pub fn system_config() -> Permission {
        Permission::new("system:config")
    }

    /// Create a custom permission
    pub fn custom(resource: &str, action: &str) -> Permission {
        Permission::new(&format!("{}:{}", resource, action))
    }

    /// Create an encryption:access_key permission
    pub fn encryption_access() -> Permission {
        Permission::new("encryption:access_key")
    }
}

/// Trait permission fixtures
pub struct TraitPermissionFixture;

impl TraitPermissionFixture {
    /// Create a permission (trait version)
    pub fn new(id: &str, resource: &str, action: &str) -> TraitPermission {
        TraitPermission::new(id, resource, action)
    }

    /// Create a menu:view permission
    pub fn menu_view() -> TraitPermission {
        TraitPermission::new("menu:view", "menu", "view")
    }

    /// Create a system admin permission
    pub fn system_admin() -> TraitPermission {
        TraitPermission::new("system:admin", "system", "admin")
    }
}

// ============================================================================
// Permission Generation Helpers
// ============================================================================

/// Generate multiple permissions for performance testing
pub fn generate_test_permissions(count: usize) -> Vec<Permission> {
    (0..count).map(|i| Permission::new(&format!("resource{}:action", i))).collect()
}
