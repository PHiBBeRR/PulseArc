//! RBAC User Context Test Fixtures
//!
//! **What this file contains:**
//! Test fixtures for creating RBAC user contexts with different roles and
//! attributes.
//!
//! ## UserContextBuilder
//! Fluent builder for creating custom user contexts:
//! - `with_role()` - Add a role
//! - `with_roles()` - Add multiple roles
//! - `with_session()` - Set session ID
//! - `with_ip()` - Set IP address
//! - `with_user_agent()` - Set user agent
//! - `with_attribute()` - Add custom attribute
//! - `build()` - Create the UserContext
//!
//! ## UserContextFixture
//! Pre-configured users for common test scenarios:
//! - `admin()` - Admin user
//! - `power_user()` - Power user
//! - `user()` - Regular user
//! - `guest()` - Guest user
//! - `empty()` - User with no roles
//! - `auditor()` - Auditor user
//! - `with_department()` - User with department attribute
//! - `from_ip()` - User with IP address
//!
//! ## TraitUserContextFixture
//! Trait-based user contexts for trait tests.
//!
//! ## Helpers
//! - `generate_test_users(count, role)` - Generate multiple users for batch
//!   testing
//!
//! ## Usage
//! ```rust
//! use fixtures::mock_rbac_users::*;
//!
//! // Pre-configured users
//! let admin = UserContextFixture::admin("admin1");
//! let user = UserContextFixture::user("user1");
//!
//! // Custom user with builder
//! let custom = UserContextBuilder::new("user2")
//!     .with_role("auditor")
//!     .with_ip("192.168.1.100")
//!     .with_attribute("department", "security")
//!     .build();
//!
//! // Batch generation
//! let users = generate_test_users(10, "user");
//! ```

use std::collections::HashMap;

use pulsearc_common::security::rbac::UserContext;
use pulsearc_common::security::traits::UserContext as TraitUserContext;

// ============================================================================
// RBAC User Context Fixtures
// ============================================================================

/// Builder for UserContext test fixtures
pub struct UserContextBuilder {
    user_id: String,
    roles: Vec<String>,
    session_id: Option<String>,
    ip_address: Option<String>,
    user_agent: Option<String>,
    attributes: HashMap<String, String>,
}

impl UserContextBuilder {
    /// Create a new user context builder
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            roles: vec![],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Add a role
    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.roles.push(role.into());
        self
    }

    /// Add multiple roles
    pub fn with_roles(mut self, roles: Vec<String>) -> Self {
        self.roles.extend(roles);
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

    /// Set user agent
    pub fn with_user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    /// Add an attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Build the UserContext
    pub fn build(self) -> UserContext {
        UserContext {
            user_id: self.user_id,
            roles: self.roles,
            session_id: self.session_id,
            ip_address: self.ip_address,
            user_agent: self.user_agent,
            attributes: self.attributes,
        }
    }
}

/// Pre-configured user context fixtures
pub struct UserContextFixture;

impl UserContextFixture {
    /// Create an admin user
    pub fn admin(user_id: impl Into<String>) -> UserContext {
        UserContextBuilder::new(user_id).with_role("admin").build()
    }

    /// Create a power user
    pub fn power_user(user_id: impl Into<String>) -> UserContext {
        UserContextBuilder::new(user_id).with_role("power_user").build()
    }

    /// Create a regular user
    pub fn user(user_id: impl Into<String>) -> UserContext {
        UserContextBuilder::new(user_id).with_role("user").build()
    }

    /// Create a guest user
    pub fn guest() -> UserContext {
        UserContextBuilder::new("guest").with_role("guest").build()
    }

    /// Create a user with no roles
    pub fn empty(user_id: impl Into<String>) -> UserContext {
        UserContextBuilder::new(user_id).build()
    }

    /// Create an auditor user
    pub fn auditor(user_id: impl Into<String>) -> UserContext {
        UserContextBuilder::new(user_id).with_role("auditor").build()
    }

    /// Create a user with custom attributes (e.g., for policy testing)
    pub fn with_department(user_id: impl Into<String>, department: &str) -> UserContext {
        UserContextBuilder::new(user_id)
            .with_role("user")
            .with_attribute("department", department)
            .build()
    }

    /// Create a user from a specific IP (for IP-based policy testing)
    pub fn from_ip(user_id: impl Into<String>, ip: &str) -> UserContext {
        UserContextBuilder::new(user_id).with_role("user").with_ip(ip).build()
    }
}

/// Trait user context fixtures (for trait-based tests)
pub struct TraitUserContextFixture;

impl TraitUserContextFixture {
    /// Create an admin user (trait version)
    pub fn admin(user_id: impl Into<String>) -> TraitUserContext {
        TraitUserContext::admin(user_id)
    }

    /// Create a regular user (trait version)
    pub fn user(user_id: impl Into<String>, roles: Vec<String>) -> TraitUserContext {
        TraitUserContext::new(user_id, roles)
    }

    /// Create a guest user (trait version)
    pub fn guest() -> TraitUserContext {
        TraitUserContext::guest()
    }
}

// ============================================================================
// User Generation Helpers
// ============================================================================

/// Generate multiple user contexts for testing
pub fn generate_test_users(count: usize, role: &str) -> Vec<UserContext> {
    (0..count)
        .map(|i| UserContextBuilder::new(format!("user_{}", i)).with_role(role).build())
        .collect()
}
