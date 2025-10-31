//! RBAC Policy and Manager Test Fixtures
//!
//! **What this file contains:**
//! Test fixtures for creating RBAC policies and initialized RBAC managers.
//!
//! ## PolicyBuilder
//! Builder for creating custom RBAC policies:
//! - `new(id, name)` - Create policy builder
//! - `with_condition()` - Set policy condition
//! - `with_effect()` - Set policy effect (Allow/Deny)
//! - `with_permission()` - Add permission
//! - `with_permissions()` - Add multiple permissions
//! - `build()` - Build the policy
//!
//! ## PolicyFixture
//! Pre-configured common policies:
//! - `deny_audit_delete()` - Deny audit deletion
//! - `business_hours_only()` - Allow only during business hours
//! - `ip_restricted()` - IP-based access control
//! - `department_only()` - Department-based access
//! - `engineering_from_office()` - Combined department + IP policy
//!
//! ## RBACFixture
//! Initialized RBAC manager for testing:
//! - `new()` - Create initialized RBAC manager
//! - `uninitialized()` - Create uninitialized manager
//! - `manager()` - Get immutable reference
//! - `manager_mut()` - Get mutable reference
//!
//! ## Assertion Macros
//! - `assert_permission_granted!` - Assert permission check succeeds
//! - `assert_permission_denied!` - Assert permission check fails
//!
//! ## Usage
//! ```rust
//! use fixtures::mock_rbac_policies::*;
//!
//! // Pre-configured policies
//! let deny_policy = PolicyFixture::deny_audit_delete();
//! let ip_policy = PolicyFixture::ip_restricted(
//!     vec!["192.168.1.100".to_string()],
//!     vec!["admin:access".to_string()],
//! );
//!
//! // RBAC manager
//! let mut rbac_fixture = RBACFixture::new();
//! let rbac = rbac_fixture.manager_mut();
//! rbac.add_policy(deny_policy).await?;
//!
//! // Assertions
//! assert_permission_granted!(rbac, &admin_user, &view_perm);
//! assert_permission_denied!(rbac, &guest_user, &delete_perm);
//! ```

#![allow(dead_code)]

use pulsearc_common::security::rbac::{PolicyCondition, PolicyEffect, RBACManager, RBACPolicy};

// ============================================================================
// Policy Fixtures
// ============================================================================

/// Builder for RBAC policy fixtures
pub struct PolicyBuilder {
    id: String,
    name: String,
    condition: PolicyCondition,
    effect: PolicyEffect,
    permissions: Vec<String>,
}

impl PolicyBuilder {
    /// Create a new policy builder
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            condition: PolicyCondition::Always,
            effect: PolicyEffect::Allow,
            permissions: vec![],
        }
    }

    /// Set the condition
    pub fn with_condition(mut self, condition: PolicyCondition) -> Self {
        self.condition = condition;
        self
    }

    /// Set the effect
    pub fn with_effect(mut self, effect: PolicyEffect) -> Self {
        self.effect = effect;
        self
    }

    /// Add a permission
    pub fn with_permission(mut self, permission: impl Into<String>) -> Self {
        self.permissions.push(permission.into());
        self
    }

    /// Add multiple permissions
    pub fn with_permissions(mut self, permissions: Vec<String>) -> Self {
        self.permissions.extend(permissions);
        self
    }

    /// Build the policy
    pub fn build(self) -> RBACPolicy {
        RBACPolicy {
            id: self.id,
            name: self.name,
            condition: self.condition,
            effect: self.effect,
            permissions: self.permissions,
        }
    }
}

/// Pre-configured policy fixtures
pub struct PolicyFixture;

impl PolicyFixture {
    /// Create a deny-all audit delete policy
    pub fn deny_audit_delete() -> RBACPolicy {
        PolicyBuilder::new("deny_audit_delete", "Deny Audit Deletion")
            .with_effect(PolicyEffect::Deny)
            .with_permission("audit:delete")
            .build()
    }

    /// Create a business hours only policy
    pub fn business_hours_only(permissions: Vec<String>) -> RBACPolicy {
        PolicyBuilder::new("business_hours", "Business Hours Only")
            .with_condition(PolicyCondition::TimeRange {
                start_time: "09:00".to_string(),
                end_time: "17:00".to_string(),
            })
            .with_effect(PolicyEffect::Allow)
            .with_permissions(permissions)
            .build()
    }

    /// Create an IP-restricted policy
    pub fn ip_restricted(allowed_ips: Vec<String>, permissions: Vec<String>) -> RBACPolicy {
        PolicyBuilder::new("ip_restricted", "IP-Restricted Access")
            .with_condition(PolicyCondition::IpRange { allowed_ips })
            .with_effect(PolicyEffect::Allow)
            .with_permissions(permissions)
            .build()
    }

    /// Create a department-based policy
    pub fn department_only(department: &str, permissions: Vec<String>) -> RBACPolicy {
        PolicyBuilder::new(
            format!("dept_{}", department),
            format!("{} Department Only", department),
        )
        .with_condition(PolicyCondition::UserAttribute {
            attribute: "department".to_string(),
            value: department.to_string(),
        })
        .with_effect(PolicyEffect::Allow)
        .with_permissions(permissions)
        .build()
    }

    /// Create an AND condition policy
    pub fn engineering_from_office(permissions: Vec<String>) -> RBACPolicy {
        PolicyBuilder::new("eng_office_only", "Engineering from Office")
            .with_condition(PolicyCondition::And {
                conditions: vec![
                    PolicyCondition::UserAttribute {
                        attribute: "department".to_string(),
                        value: "engineering".to_string(),
                    },
                    PolicyCondition::IpRange { allowed_ips: vec!["192.168.1.0/24".to_string()] },
                ],
            })
            .with_effect(PolicyEffect::Allow)
            .with_permissions(permissions)
            .build()
    }
}

// ============================================================================
// RBAC Manager Fixtures
// ============================================================================

/// RBAC manager fixture with initialization
pub struct RBACFixture {
    pub manager: RBACManager,
}

impl RBACFixture {
    /// Create a new RBAC fixture with default roles
    pub fn new() -> Self {
        let mut manager = RBACManager::new();
        manager.initialize().expect("Failed to initialize RBAC");
        Self { manager }
    }

    /// Create an RBAC fixture without initialization
    pub fn uninitialized() -> Self {
        Self { manager: RBACManager::new() }
    }

    /// Get the manager
    pub fn manager(&self) -> &RBACManager {
        &self.manager
    }

    /// Get mutable manager
    pub fn manager_mut(&mut self) -> &mut RBACManager {
        &mut self.manager
    }
}

impl Default for RBACFixture {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that a permission check succeeds
#[macro_export]
macro_rules! assert_permission_granted {
    ($rbac:expr, $user:expr, $perm:expr) => {{
        let result = $rbac.check_permission($user, $perm).await;
        assert!(
            result,
            "Expected permission to be granted for user '{}' and permission '{}'",
            $user.user_id, $perm.id
        );
    }};
}

/// Assert that a permission check fails
#[macro_export]
macro_rules! assert_permission_denied {
    ($rbac:expr, $user:expr, $perm:expr) => {{
        let result = $rbac.check_permission($user, $perm).await;
        assert!(
            !result,
            "Expected permission to be denied for user '{}' and permission '{}'",
            $user.user_id, $perm.id
        );
    }};
}
