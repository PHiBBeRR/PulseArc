//! Trait abstractions for security components
//!
//! This module defines traits for Role-Based Access Control (RBAC)
//! and other security primitives, allowing components to use security
//! features without depending on specific implementations.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ============================================================================
// RBAC Traits
// ============================================================================

/// Generic user context for authorization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub attributes: HashMap<String, String>,
}

/// Generic permission definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: String,
    pub resource: String,
    pub action: String,
    pub description: String,
    pub requires_approval: bool,
}

/// Trait for Role-Based Access Control implementations
#[async_trait]
pub trait AccessControl: Send + Sync {
    /// Check if a user has permission to perform an action
    async fn check_permission(&self, user: &UserContext, permission: &Permission) -> bool;

    /// Check if a user has a specific role
    async fn has_role(&self, user: &UserContext, role: &str) -> bool {
        user.roles.iter().any(|r| r == role)
    }

    /// Get all permissions for a user
    async fn get_user_permissions(&self, user: &UserContext) -> Vec<Permission>;

    /// Check multiple permissions at once
    async fn check_permissions(
        &self,
        user: &UserContext,
        permissions: &[Permission],
    ) -> HashMap<String, bool> {
        let mut results = HashMap::new();
        for permission in permissions {
            let granted = self.check_permission(user, permission).await;
            results.insert(permission.id.clone(), granted);
        }
        results
    }
}

// ============================================================================
// Validation Traits
// ============================================================================

/// Generic validation error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub code: Option<String>,
}

/// Generic validation result
pub type ValidationResult<T> = Result<T, Vec<ValidationError>>;

/// Trait for validation implementations
pub trait Validator: Send + Sync {
    /// Validate a value and return errors if invalid
    fn validate<T>(&self, value: &T, field: &str) -> ValidationResult<()>
    where
        T: ?Sized;

    /// Check if a value is valid without returning errors
    fn is_valid<T>(&self, value: &T) -> bool
    where
        T: ?Sized,
    {
        self.validate(value, "value").is_ok()
    }
}

// ============================================================================
// No-Op Implementations
// ============================================================================

/// No-op access control that allows everything (for testing)
#[derive(Debug, Clone)]
pub struct NoOpAccessControl;

#[async_trait]
impl AccessControl for NoOpAccessControl {
    async fn check_permission(&self, _user: &UserContext, _permission: &Permission) -> bool {
        true // Allow everything
    }

    async fn get_user_permissions(&self, _user: &UserContext) -> Vec<Permission> {
        vec![]
    }
}

/// No-op validator that accepts everything (for testing)
#[derive(Debug, Clone)]
pub struct NoOpValidator;

impl Validator for NoOpValidator {
    fn validate<T>(&self, _value: &T, _field: &str) -> ValidationResult<()>
    where
        T: ?Sized,
    {
        Ok(())
    }
}

// ============================================================================
// Strict Implementations (for testing edge cases)
// ============================================================================

/// Strict access control that denies everything (for testing)
#[derive(Debug, Clone)]
pub struct DenyAllAccessControl;

#[async_trait]
impl AccessControl for DenyAllAccessControl {
    async fn check_permission(&self, _user: &UserContext, _permission: &Permission) -> bool {
        false // Deny everything
    }

    async fn get_user_permissions(&self, _user: &UserContext) -> Vec<Permission> {
        vec![]
    }
}

// ============================================================================
// Helper Builders
// ============================================================================

impl UserContext {
    /// Create a new user context
    pub fn new(user_id: impl Into<String>, roles: Vec<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles,
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Create an admin user context
    pub fn admin(user_id: impl Into<String>) -> Self {
        Self::new(user_id, vec!["admin".to_string()])
    }

    /// Create a guest user context
    pub fn guest() -> Self {
        Self::new("guest", vec!["guest".to_string()])
    }

    /// Add a role
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Set session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set IP address
    pub fn with_ip(mut self, ip: impl Into<String>) -> Self {
        self.ip_address = Some(ip.into());
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

impl Permission {
    /// Create a new permission
    pub fn new(
        id: impl Into<String>,
        resource: impl Into<String>,
        action: impl Into<String>,
    ) -> Self {
        let id_str = id.into();
        Self {
            id: id_str.clone(),
            resource: resource.into(),
            action: action.into(),
            description: format!("Permission: {}", id_str),
            requires_approval: false,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Require approval
    pub fn requires_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }
}

impl ValidationError {
    /// Create a new validation error
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self { field: field.into(), message: message.into(), code: None }
    }

    /// Set error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::traits.
    use super::*;

    /// Validates `UserContext::new` behavior for the noop access control
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `ac.check_permission(&user, &permission).await` evaluates to
    ///   true.
    /// - Ensures `ac.has_role(&user, "user").await` evaluates to true.
    /// - Confirms `ac.get_user_permissions(&user).await.len()` equals `0`.
    #[tokio::test]
    async fn test_noop_access_control() {
        let ac = NoOpAccessControl;
        let user = UserContext::new("user1", vec!["user".to_string()]);
        let permission = Permission::new("read", "resource", "read");

        assert!(ac.check_permission(&user, &permission).await);
        assert!(ac.has_role(&user, "user").await);
        assert_eq!(ac.get_user_permissions(&user).await.len(), 0);
    }

    /// Validates `UserContext::admin` behavior for the deny all access control
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `!ac.check_permission(&user, &permission).await` evaluates to
    ///   true.
    /// - Ensures `ac.has_role(&user, "admin").await` evaluates to true.
    #[tokio::test]
    async fn test_deny_all_access_control() {
        let ac = DenyAllAccessControl;
        let user = UserContext::admin("admin1");
        let permission = Permission::new("read", "resource", "read");

        assert!(!ac.check_permission(&user, &permission).await);
        assert!(ac.has_role(&user, "admin").await); // Role check works
    }

    /// Validates the noop validator scenario.
    ///
    /// Assertions:
    /// - Ensures `validator.validate(&"anything", "field").is_ok()` evaluates
    ///   to true.
    /// - Ensures `validator.is_valid(&42)` evaluates to true.
    #[test]
    fn test_noop_validator() {
        let validator = NoOpValidator;
        assert!(validator.validate(&"anything", "field").is_ok());
        assert!(validator.is_valid(&42));
    }

    /// Validates `UserContext::new` behavior for the user context builder
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `user.user_id` equals `"user1"`.
    /// - Confirms `user.roles.len()` equals `2`.
    /// - Ensures `user.roles.contains(&"user".to_string())` evaluates to true.
    /// - Ensures `user.roles.contains(&"editor".to_string())` evaluates to
    ///   true.
    /// - Confirms `user.session_id` equals `Some("session123".to_string())`.
    /// - Confirms `user.ip_address` equals `Some("192.168.1.1".to_string())`.
    /// - Confirms `user.attributes.get("department")` equals
    ///   `Some(&"engineering".to_string())`.
    #[test]
    fn test_user_context_builder() {
        let user = UserContext::new("user1", vec!["user".to_string()])
            .with_role("editor")
            .with_session("session123")
            .with_ip("192.168.1.1")
            .with_attribute("department", "engineering");

        assert_eq!(user.user_id, "user1");
        assert_eq!(user.roles.len(), 2);
        assert!(user.roles.contains(&"user".to_string()));
        assert!(user.roles.contains(&"editor".to_string()));
        assert_eq!(user.session_id, Some("session123".to_string()));
        assert_eq!(user.ip_address, Some("192.168.1.1".to_string()));
        assert_eq!(user.attributes.get("department"), Some(&"engineering".to_string()));
    }

    /// Validates `Permission::new` behavior for the permission builder
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `permission.id` equals `"admin:delete"`.
    /// - Confirms `permission.resource` equals `"users"`.
    /// - Confirms `permission.action` equals `"delete"`.
    /// - Confirms `permission.description` equals `"Delete users"`.
    /// - Ensures `permission.requires_approval` evaluates to true.
    #[test]
    fn test_permission_builder() {
        let permission = Permission::new("admin:delete", "users", "delete")
            .with_description("Delete users")
            .requires_approval();

        assert_eq!(permission.id, "admin:delete");
        assert_eq!(permission.resource, "users");
        assert_eq!(permission.action, "delete");
        assert_eq!(permission.description, "Delete users");
        assert!(permission.requires_approval);
    }

    /// Validates `ValidationError::new` behavior for the validation error
    /// builder scenario.
    ///
    /// Assertions:
    /// - Confirms `error.field` equals `"email"`.
    /// - Confirms `error.message` equals `"Invalid email format"`.
    /// - Confirms `error.code` equals `Some("EMAIL_001".to_string())`.
    #[test]
    fn test_validation_error_builder() {
        let error = ValidationError::new("email", "Invalid email format").with_code("EMAIL_001");

        assert_eq!(error.field, "email");
        assert_eq!(error.message, "Invalid email format");
        assert_eq!(error.code, Some("EMAIL_001".to_string()));
    }
}
