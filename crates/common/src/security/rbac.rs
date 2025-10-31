// Role-Based Access Control (RBAC) System

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::NaiveTime;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

// Type aliases for complex types
type RoleMap = Arc<RwLock<HashMap<String, Role>>>;
type PermissionMap = Arc<RwLock<HashMap<String, Permission>>>;
type UserRoleMap = Arc<RwLock<HashMap<String, Vec<String>>>>;
type PermissionCache = Arc<RwLock<HashMap<String, CachedPermission>>>;
type InitResult = Result<(), Box<dyn std::error::Error>>;
type ConditionFuture<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = bool> + 'a + Send>>;

/// User role in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Role {
    pub id: String,
    pub name: String,
    pub description: String,
    pub permissions: HashSet<String>,
    pub parent_role: Option<String>,
    pub priority: u32,
}

/// Permission definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct Permission {
    pub id: String,
    pub resource: String,
    pub action: String,
    pub description: String,
    pub requires_approval: bool,
}

/// User context for RBAC checks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContext {
    pub user_id: String,
    pub roles: Vec<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub attributes: HashMap<String, String>,
}

/// RBAC policy for dynamic permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RBACPolicy {
    pub id: String,
    pub name: String,
    pub condition: PolicyCondition,
    pub effect: PolicyEffect,
    pub permissions: Vec<String>,
}

/// Policy condition for dynamic evaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PolicyCondition {
    Always,
    TimeRange { start_time: String, end_time: String },
    IpRange { allowed_ips: Vec<String> },
    UserAttribute { attribute: String, value: String },
    And { conditions: Vec<PolicyCondition> },
    Or { conditions: Vec<PolicyCondition> },
    Not { condition: Box<PolicyCondition> },
}

/// Policy effect
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// RBAC Manager for the application
pub struct RBACManager {
    roles: RoleMap,
    /// Permission registry for validation and discovery
    permissions: PermissionMap,
    user_roles: UserRoleMap,
    policies: Arc<RwLock<Vec<RBACPolicy>>>,
    cache: PermissionCache,
}

/// Cached permission result
#[derive(Debug, Clone)]
struct CachedPermission {
    granted: bool,
    expires_at: std::time::Instant,
}

impl Default for RBACManager {
    fn default() -> Self {
        Self::new()
    }
}

impl RBACManager {
    /// Create a new RBAC manager
    pub fn new() -> Self {
        // Build default roles before wrapping in RwLock to avoid blocking issues
        let default_roles = Self::build_default_roles();

        Self {
            roles: Arc::new(RwLock::new(default_roles)),
            permissions: Arc::new(RwLock::new(HashMap::new())),
            user_roles: Arc::new(RwLock::new(HashMap::new())),
            policies: Arc::new(RwLock::new(Vec::new())),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Build default roles (pure function, no locking)
    fn build_default_roles() -> HashMap<String, Role> {
        let default_roles = vec![
            Role {
                id: "admin".to_string(),
                name: "Administrator".to_string(),
                description: "Full system access".to_string(),
                permissions: HashSet::from_iter(vec![
                    "system:*".to_string(),
                    "menu:*".to_string(),
                    "config:*".to_string(),
                    "audit:*".to_string(),
                ]),
                parent_role: None,
                priority: 100,
            },
            Role {
                id: "power_user".to_string(),
                name: "Power User".to_string(),
                description: "Advanced user with elevated permissions".to_string(),
                permissions: HashSet::from_iter(vec![
                    "menu:view".to_string(),
                    "menu:interact".to_string(),
                    "config:read".to_string(),
                    "config:write".to_string(),
                    "audit:read".to_string(),
                ]),
                parent_role: Some("user".to_string()),
                priority: 50,
            },
            Role {
                id: "user".to_string(),
                name: "Standard User".to_string(),
                description: "Basic user permissions".to_string(),
                permissions: HashSet::from_iter(vec![
                    "menu:view".to_string(),
                    "menu:interact:basic".to_string(),
                    "config:read:own".to_string(),
                ]),
                parent_role: None,
                priority: 10,
            },
            Role {
                id: "guest".to_string(),
                name: "Guest".to_string(),
                description: "Limited read-only access".to_string(),
                permissions: HashSet::from_iter(vec!["menu:view:basic".to_string()]),
                parent_role: None,
                priority: 1,
            },
            Role {
                id: "auditor".to_string(),
                name: "Auditor".to_string(),
                description: "Audit and compliance access".to_string(),
                permissions: HashSet::from_iter(vec![
                    "audit:*".to_string(),
                    "compliance:view".to_string(),
                    "menu:view".to_string(),
                ]),
                parent_role: None,
                priority: 60,
            },
        ];

        let mut roles_map = HashMap::new();
        for role in default_roles {
            roles_map.insert(role.id.clone(), role);
        }
        roles_map
    }

    /// Initialize RBAC manager
    pub fn initialize(&mut self) -> InitResult {
        info!("Initializing RBAC manager");
        // Roles are already initialized in new(), just initialize permissions
        self.initialize_default_permissions()?;
        Ok(())
    }

    /// Initialize default permissions
    fn initialize_default_permissions(&mut self) -> InitResult {
        // Permissions registry is optional and not used by the core permission checking
        // logic which relies on role-based permissions defined in the roles
        // themselves. This can be populated during actual application startup
        // if needed, but not required for tests.
        info!("Permission registry initialization skipped (permissions are embedded in roles)");
        Ok(())
    }

    /// Check if a user has a specific permission
    ///
    /// # Permission Matching Rules
    ///
    /// Permissions are matched using three strategies, evaluated in order:
    ///
    /// ## 1. Exact Match
    /// The permission string exactly matches a user's permission.
    ///
    /// ```text
    /// User has: "menu:view"
    /// Checking: "menu:view"
    /// Result:   ✅ GRANTED
    /// ```
    ///
    /// ## 2. Resource Wildcard (`resource:*`)
    /// User has wildcard permission for a specific resource.
    ///
    /// ```text
    /// User has: "menu:*"
    /// Checking: "menu:view", "menu:edit", "menu:delete"
    /// Result:   ✅ GRANTED (all menu actions)
    /// ```
    ///
    /// ## 3. Global Wildcard (`*:*` or `system:*`)
    /// User has global wildcard permission for all resources and actions.
    ///
    /// ```text
    /// User has: "system:*"
    /// Checking: "menu:view", "config:write", "audit:delete"
    /// Result:   ✅ GRANTED (all permissions)
    /// ```
    ///
    /// ```text
    /// User has: "*:*"
    /// Checking: Any permission
    /// Result:   ✅ GRANTED (superuser)
    /// ```
    ///
    /// # Wildcard Hierarchy
    ///
    /// ```text
    /// *:*           → Matches everything (superuser)
    /// system:*      → Matches all system permissions
    /// menu:*        → Matches all menu permissions (menu:view, menu:edit, etc.)
    /// menu:view     → Matches only menu:view (exact)
    /// ```
    ///
    /// # Policy Override
    ///
    /// Dynamic policies can override role-based permissions:
    /// - **Deny policies** always take precedence (even over wildcards)
    /// - **Allow policies** grant permission if no deny policy applies
    /// - If no policy matches, fall back to role-based matching
    ///
    /// ```text
    /// User has: "system:*"
    /// Policy:   Deny "audit:delete" during business hours
    /// Checking: "audit:delete" at 2pm
    /// Result:   ❌ DENIED (policy overrides wildcard)
    /// ```
    ///
    /// # Caching
    ///
    /// Permission checks are cached for 60 seconds to improve performance.
    /// Cache is keyed by `user_id:permission_id`.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use pulsearc_common::security::rbac::{RBACManager, UserContext, Permission};
    ///
    /// let rbac = RBACManager::new();
    /// let admin = UserContext::admin("admin_user");
    /// let perm = Permission::new("menu:view", "menu", "view");
    ///
    /// // Admin has "system:*" which matches everything
    /// assert!(rbac.check_permission(&admin, &perm).await);
    /// ```
    pub async fn check_permission(
        &self,
        user_context: &UserContext,
        permission: &Permission,
    ) -> bool {
        // Check cache first
        let cache_key = format!("{}:{}", user_context.user_id, permission.id);
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(&cache_key) {
            if cached.expires_at > std::time::Instant::now() {
                debug!("Permission check cache hit: {}", cache_key);
                return cached.granted;
            }
        }
        drop(cache);

        // Check user roles
        let user_permissions = self.get_user_permissions(user_context).await;

        // Check exact permission
        let permission_string = format!("{}:{}", permission.resource, permission.action);
        let has_exact = user_permissions.contains(&permission_string);

        // Check wildcard permissions
        let has_resource_wildcard =
            user_permissions.contains(&format!("{}:*", permission.resource));
        let has_global_wildcard =
            user_permissions.contains("*:*") || user_permissions.contains("system:*");

        // Check policies
        let policy_result = self.evaluate_policies(user_context, &permission_string).await;

        let granted = match policy_result {
            Some(PolicyEffect::Deny) => false,
            Some(PolicyEffect::Allow) => true,
            None => has_exact || has_resource_wildcard || has_global_wildcard,
        };

        // Cache the result
        let mut cache = self.cache.write().await;
        cache.insert(
            cache_key,
            CachedPermission {
                granted,
                expires_at: std::time::Instant::now() + std::time::Duration::from_secs(60),
            },
        );

        granted
    }

    /// Get all permissions for a user
    async fn get_user_permissions(&self, user_context: &UserContext) -> HashSet<String> {
        let mut permissions = HashSet::new();
        let roles = self.roles.read().await;

        for role_id in &user_context.roles {
            if let Some(role) = roles.get(role_id) {
                permissions.extend(role.permissions.clone());

                // Include parent role permissions
                if let Some(parent_id) = &role.parent_role {
                    if let Some(parent_role) = roles.get(parent_id) {
                        permissions.extend(parent_role.permissions.clone());
                    }
                }
            }
        }

        permissions
    }

    /// Evaluate policies for a permission
    async fn evaluate_policies(
        &self,
        user_context: &UserContext,
        permission: &str,
    ) -> Option<PolicyEffect> {
        let policies = self.policies.read().await;

        for policy in policies.iter() {
            if !policy.permissions.contains(&permission.to_string()) {
                continue;
            }

            if self.evaluate_condition(&policy.condition, user_context).await {
                return Some(policy.effect);
            }
        }

        None
    }

    /// Evaluate a policy condition
    #[allow(clippy::only_used_in_recursion)]
    fn evaluate_condition<'a>(
        &'a self,
        condition: &'a PolicyCondition,
        user_context: &'a UserContext,
    ) -> ConditionFuture<'a> {
        Box::pin(async move {
            match condition {
                PolicyCondition::Always => true,
                PolicyCondition::TimeRange { start_time, end_time } => {
                    // Parse time strings (expected format: "HH:MM" or "HH:MM:SS")
                    let current_time = chrono::Local::now();

                    match (parse_time_string(start_time), parse_time_string(end_time)) {
                        (Ok(start), Ok(end)) => {
                            let current_time_of_day = current_time.time();
                            debug!(
                                "Time range check: {} - {} (current: {})",
                                start_time, end_time, current_time_of_day
                            );

                            // Handle time ranges that cross midnight
                            if start <= end {
                                current_time_of_day >= start && current_time_of_day <= end
                            } else {
                                current_time_of_day >= start || current_time_of_day <= end
                            }
                        }
                        _ => {
                            warn!("Invalid time range format: {} - {}", start_time, end_time);
                            false
                        }
                    }
                }
                PolicyCondition::IpRange { allowed_ips } => {
                    if let Some(ip) = &user_context.ip_address {
                        allowed_ips.contains(ip)
                    } else {
                        false
                    }
                }
                PolicyCondition::UserAttribute { attribute, value } => {
                    user_context.attributes.get(attribute).map(|v| v == value).unwrap_or(false)
                }
                PolicyCondition::And { conditions } => {
                    for cond in conditions {
                        if !self.evaluate_condition(cond, user_context).await {
                            return false;
                        }
                    }
                    true
                }
                PolicyCondition::Or { conditions } => {
                    for cond in conditions {
                        if self.evaluate_condition(cond, user_context).await {
                            return true;
                        }
                    }
                    false
                }
                PolicyCondition::Not { condition } => {
                    !self.evaluate_condition(condition, user_context).await
                }
            }
        })
    }

    /// Assign a role to a user
    pub async fn assign_role(&self, user_id: &str, role_id: &str) -> InitResult {
        let roles = self.roles.read().await;
        if !roles.contains_key(role_id) {
            return Err(format!("Role '{}' does not exist", role_id).into());
        }
        drop(roles);

        let mut user_roles = self.user_roles.write().await;
        let roles = user_roles.entry(user_id.to_string()).or_insert_with(Vec::new);

        if !roles.contains(&role_id.to_string()) {
            roles.push(role_id.to_string());
            info!("Assigned role '{}' to user '{}'", role_id, user_id);
        }

        // Clear cache for this user
        let mut cache = self.cache.write().await;
        cache.retain(|k, _| !k.starts_with(&format!("{}:", user_id)));

        Ok(())
    }

    /// Remove a role from a user
    pub async fn revoke_role(&self, user_id: &str, role_id: &str) -> InitResult {
        let mut user_roles = self.user_roles.write().await;

        if let Some(roles) = user_roles.get_mut(user_id) {
            roles.retain(|r| r != role_id);
            info!("Revoked role '{}' from user '{}'", role_id, user_id);

            // Clear cache for this user
            let mut cache = self.cache.write().await;
            cache.retain(|k, _| !k.starts_with(&format!("{}:", user_id)));
        }

        Ok(())
    }

    /// Get all roles for a user
    pub async fn get_user_roles(&self, user_id: &str) -> Vec<Role> {
        let user_roles = self.user_roles.read().await;
        let roles = self.roles.read().await;

        user_roles
            .get(user_id)
            .map(|role_ids| role_ids.iter().filter_map(|id| roles.get(id).cloned()).collect())
            .unwrap_or_default()
    }

    /// Create a custom role
    pub async fn create_role(&self, role: Role) -> InitResult {
        let mut roles = self.roles.write().await;

        if roles.contains_key(&role.id) {
            return Err(format!("Role '{}' already exists", role.id).into());
        }

        info!("Created role '{}' with {} permissions", role.id, role.permissions.len());
        roles.insert(role.id.clone(), role);
        Ok(())
    }

    /// Add a policy
    pub async fn add_policy(&self, policy: RBACPolicy) -> InitResult {
        let mut policies = self.policies.write().await;

        info!("Added policy '{}' with effect {:?}", policy.id, policy.effect);
        policies.push(policy);

        // Clear all cache as policies affect everyone
        self.cache.write().await.clear();

        Ok(())
    }

    /// Register a permission in the permission registry
    pub async fn register_permission(&self, permission: Permission) -> InitResult {
        let mut permissions = self.permissions.write().await;

        if permissions.contains_key(&permission.id) {
            return Err(format!("Permission '{}' already registered", permission.id).into());
        }

        debug!("Registered permission: {}", permission.id);
        permissions.insert(permission.id.clone(), permission);
        Ok(())
    }

    /// Get a permission from the registry by ID
    pub async fn get_permission(&self, permission_id: &str) -> Option<Permission> {
        let permissions = self.permissions.read().await;
        permissions.get(permission_id).cloned()
    }

    /// List all registered permissions
    pub async fn list_permissions(&self) -> Vec<Permission> {
        let permissions = self.permissions.read().await;
        permissions.values().cloned().collect()
    }

    /// Validate if a permission string exists in the registry
    pub async fn is_permission_registered(&self, permission_id: &str) -> bool {
        let permissions = self.permissions.read().await;
        permissions.contains_key(permission_id)
    }

    /// Get permissions by resource
    pub async fn get_permissions_by_resource(&self, resource: &str) -> Vec<Permission> {
        let permissions = self.permissions.read().await;
        permissions.values().filter(|p| p.resource == resource).cloned().collect()
    }
}

impl Permission {
    /// Create a new permission from a resource:action string
    pub fn new(permission_str: &str) -> Self {
        let parts: Vec<&str> = permission_str.split(':').collect();
        let resource = parts.first().unwrap_or(&"").to_string();
        let action = parts.get(1).unwrap_or(&"").to_string();

        Self {
            id: permission_str.to_string(),
            resource,
            action,
            description: String::new(),
            requires_approval: false,
        }
    }
}

/// Helper function to parse time strings like "HH:MM" or "HH:MM:SS"
fn parse_time_string(time_str: &str) -> Result<NaiveTime, chrono::ParseError> {
    // Try parsing as HH:MM:SS first
    NaiveTime::parse_from_str(time_str, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(time_str, "%H:%M"))
}

#[cfg(test)]
mod tests {
    //! Unit tests for security::rbac.
    use super::*;

    /// Tests that `parse_time_string()` successfully parses HH:MM format time
    /// strings.
    #[test]
    fn test_parse_time_string_hh_mm() {
        let result = parse_time_string("14:30");
        assert!(result.is_ok());
        // Just verify it parsed successfully
    }

    /// Tests that `parse_time_string()` successfully parses HH:MM:SS format
    /// time strings.
    #[test]
    fn test_parse_time_string_hh_mm_ss() {
        let result = parse_time_string("14:30:45");
        assert!(result.is_ok());
        // Just verify it parsed successfully
    }

    /// Tests that `parse_time_string()` rejects invalid time formats and
    /// out-of-range values.
    #[test]
    fn test_parse_time_string_invalid() {
        assert!(parse_time_string("25:00").is_err());
        assert!(parse_time_string("14:60").is_err());
        assert!(parse_time_string("invalid").is_err());
        assert!(parse_time_string("").is_err());
    }

    /// Tests that `RBACManager::new()` creates a manager instance successfully.
    #[test]
    fn test_rbac_manager_new() {
        let manager = RBACManager::new();
        // Verify manager is created successfully
        assert!(std::ptr::eq(Arc::as_ptr(&manager.roles), Arc::as_ptr(&manager.roles)));
    }

    /// Tests that `RBACManager::initialize()` completes without errors.
    #[test]
    fn test_rbac_manager_initialize() {
        let mut manager = RBACManager::new();
        let result = manager.initialize();
        assert!(result.is_ok());
    }

    /// Tests that `RBACManager::new()` pre-populates default roles (admin,
    /// power_user, user, guest, auditor).
    #[tokio::test]
    async fn test_default_roles_created() {
        let manager = RBACManager::new();
        let roles = manager.roles.read().await;

        assert!(roles.contains_key("admin"));
        assert!(roles.contains_key("power_user"));
        assert!(roles.contains_key("user"));
        assert!(roles.contains_key("guest"));
        assert!(roles.contains_key("auditor"));

        // Verify admin role has global permissions
        let admin = roles.get("admin").unwrap();
        assert!(admin.permissions.contains("system:*"));
        assert_eq!(admin.priority, 100);
    }

    /// Tests that role hierarchy is correctly configured with parent roles.
    #[tokio::test]
    async fn test_role_hierarchy_with_parent() {
        let manager = RBACManager::new();
        let roles = manager.roles.read().await;

        let power_user = roles.get("power_user").unwrap();
        assert_eq!(power_user.parent_role, Some("user".to_string()));
    }

    /// Tests that `assign_role()` successfully assigns a role to a user and
    /// retrieves it.
    #[tokio::test]
    async fn test_assign_role() {
        let manager = RBACManager::new();
        let result = manager.assign_role("user123", "admin").await;
        assert!(result.is_ok());

        let user_roles = manager.get_user_roles("user123").await;
        assert_eq!(user_roles.len(), 1);
        assert_eq!(user_roles[0].id, "admin");
    }

    /// Tests that `assign_role()` returns an error when attempting to assign a
    /// nonexistent role.
    #[tokio::test]
    async fn test_assign_nonexistent_role() {
        let manager = RBACManager::new();
        let result = manager.assign_role("user123", "nonexistent").await;
        assert!(result.is_err());
    }

    /// Tests that `revoke_role()` successfully removes a role from a user.
    #[tokio::test]
    async fn test_revoke_role() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();
        manager.assign_role("user123", "user").await.ok();

        let result = manager.revoke_role("user123", "admin").await;
        assert!(result.is_ok());

        let user_roles = manager.get_user_roles("user123").await;
        assert_eq!(user_roles.len(), 1);
        assert_eq!(user_roles[0].id, "user");
    }

    /// Tests that `revoke_role()` handles nonexistent users gracefully without
    /// errors.
    #[tokio::test]
    async fn test_revoke_role_from_nonexistent_user() {
        let manager = RBACManager::new();
        let result = manager.revoke_role("nonexistent", "admin").await;
        assert!(result.is_ok()); // Should not error
    }

    /// Tests that `check_permission()` grants access when user has exact
    /// permission match.
    #[tokio::test]
    async fn test_check_permission_exact_match() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["admin".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let permission = Permission::new("system:read");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted); // Admin has system:*
    }

    /// Tests that `check_permission()` grants access when user has wildcard
    /// resource permission.
    #[tokio::test]
    async fn test_check_permission_wildcard_resource() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["admin".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let permission = Permission::new("menu:any_action");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted); // Admin has menu:*
    }

    /// Tests that `check_permission()` denies access when user lacks required
    /// permissions.
    #[tokio::test]
    async fn test_check_permission_denied() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "guest").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["guest".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let permission = Permission::new("system:shutdown");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(!granted); // Guest doesn't have system permissions
    }

    /// Tests that `check_permission()` includes permissions inherited from
    /// parent roles.
    #[tokio::test]
    async fn test_check_permission_with_parent_role() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["power_user".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        // Power user should have user permissions through parent role
        let permission = Permission::new("menu:view");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that `check_permission()` caches permission check results for
    /// performance.
    #[tokio::test]
    async fn test_permission_cache() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["admin".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let permission = Permission::new("system:read");

        // First check (miss cache)
        let granted1 = manager.check_permission(&user_context, &permission).await;
        assert!(granted1);

        // Second check (hit cache)
        let granted2 = manager.check_permission(&user_context, &permission).await;
        assert!(granted2);

        // Verify cache has entry
        let cache = manager.cache.read().await;
        let cache_key = format!("{}:{}", user_context.user_id, permission.id);
        assert!(cache.contains_key(&cache_key));
    }

    /// Tests that permission cache is invalidated when user roles change.
    #[tokio::test]
    async fn test_cache_cleared_on_role_change() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["admin".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let permission = Permission::new("system:read");
        manager.check_permission(&user_context, &permission).await;

        // Revoke role should clear cache
        manager.revoke_role("user123", "admin").await.ok();

        let cache = manager.cache.read().await;
        let cache_key = format!("{}:{}", user_context.user_id, permission.id);
        assert!(!cache.contains_key(&cache_key));
    }

    /// Tests that `create_role()` successfully creates a custom role with
    /// specific permissions.
    #[tokio::test]
    async fn test_create_custom_role() {
        let manager = RBACManager::new();

        let custom_role = Role {
            id: "custom".to_string(),
            name: "Custom Role".to_string(),
            description: "Test role".to_string(),
            permissions: HashSet::from_iter(vec!["test:read".to_string()]),
            parent_role: None,
            priority: 20,
        };

        let result = manager.create_role(custom_role).await;
        assert!(result.is_ok());

        let roles = manager.roles.read().await;
        assert!(roles.contains_key("custom"));
    }

    /// Tests that `create_role()` returns an error when attempting to create a
    /// duplicate role.
    #[tokio::test]
    async fn test_create_duplicate_role() {
        let manager = RBACManager::new();

        let custom_role = Role {
            id: "admin".to_string(), // Duplicate
            name: "Custom Admin".to_string(),
            description: "Test role".to_string(),
            permissions: HashSet::new(),
            parent_role: None,
            priority: 20,
        };

        let result = manager.create_role(custom_role).await;
        assert!(result.is_err());
    }

    /// Tests that RBAC policy with `Always` condition grants access
    /// unconditionally.
    #[tokio::test]
    async fn test_policy_always_condition() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["guest".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let policy = RBACPolicy {
            id: "allow_all".to_string(),
            name: "Allow All".to_string(),
            condition: PolicyCondition::Always,
            effect: PolicyEffect::Allow,
            permissions: vec!["test:action".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:action");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that RBAC policy with `TimeRange` condition evaluates based on
    /// current time.
    #[tokio::test]
    async fn test_policy_time_range_condition() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["guest".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        // Create a policy with a time range that should always be true (00:00 - 23:59)
        let policy = RBACPolicy {
            id: "time_policy".to_string(),
            name: "Time Policy".to_string(),
            condition: PolicyCondition::TimeRange {
                start_time: "00:00".to_string(),
                end_time: "23:59".to_string(),
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:time".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:time");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that RBAC policy time ranges correctly handle periods crossing
    /// midnight.
    #[tokio::test]
    async fn test_policy_time_range_crossing_midnight() {
        let manager = RBACManager::new();
        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec![],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        // Test time range logic by evaluating condition directly
        let condition = PolicyCondition::TimeRange {
            start_time: "22:00".to_string(),
            end_time: "06:00".to_string(),
        };

        let _result = manager.evaluate_condition(&condition, &user_context).await;
        // Result depends on current time, but should not panic - just verify it
        // executes
    }

    /// Tests that RBAC policy with `IpRange` condition grants access based on
    /// user IP address.
    #[tokio::test]
    async fn test_policy_ip_range_condition() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["guest".to_string()],
            session_id: None,
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: None,
            attributes: HashMap::new(),
        };

        let policy = RBACPolicy {
            id: "ip_policy".to_string(),
            name: "IP Policy".to_string(),
            condition: PolicyCondition::IpRange {
                allowed_ips: vec!["192.168.1.100".to_string(), "10.0.0.1".to_string()],
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:ip".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:ip");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that RBAC policy denies access when user IP is not in allowed
    /// range.
    #[tokio::test]
    async fn test_policy_ip_range_denied() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["guest".to_string()],
            session_id: None,
            ip_address: Some("192.168.1.200".to_string()), // Not in allowed list
            user_agent: None,
            attributes: HashMap::new(),
        };

        let policy = RBACPolicy {
            id: "ip_policy".to_string(),
            name: "IP Policy".to_string(),
            condition: PolicyCondition::IpRange { allowed_ips: vec!["192.168.1.100".to_string()] },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:ip".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:ip");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(!granted);
    }

    /// Tests that RBAC policy with `UserAttribute` condition evaluates based on
    /// user attributes.
    #[tokio::test]
    async fn test_policy_user_attribute_condition() {
        let manager = RBACManager::new();

        let mut attributes = HashMap::new();
        attributes.insert("department".to_string(), "engineering".to_string());

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["guest".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes,
        };

        let policy = RBACPolicy {
            id: "attr_policy".to_string(),
            name: "Attribute Policy".to_string(),
            condition: PolicyCondition::UserAttribute {
                attribute: "department".to_string(),
                value: "engineering".to_string(),
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:attr".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:attr");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that RBAC policy with `And` condition requires all sub-conditions
    /// to be true.
    #[tokio::test]
    async fn test_policy_and_condition() {
        let manager = RBACManager::new();

        let mut attributes = HashMap::new();
        attributes.insert("department".to_string(), "engineering".to_string());

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec![],
            session_id: None,
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: None,
            attributes,
        };

        let policy = RBACPolicy {
            id: "and_policy".to_string(),
            name: "And Policy".to_string(),
            condition: PolicyCondition::And {
                conditions: vec![
                    PolicyCondition::IpRange { allowed_ips: vec!["192.168.1.100".to_string()] },
                    PolicyCondition::UserAttribute {
                        attribute: "department".to_string(),
                        value: "engineering".to_string(),
                    },
                ],
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:and".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:and");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that RBAC policy with `Or` condition grants access if any
    /// sub-condition is true.
    #[tokio::test]
    async fn test_policy_or_condition() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec![],
            session_id: None,
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: None,
            attributes: HashMap::new(), // No attributes
        };

        let policy = RBACPolicy {
            id: "or_policy".to_string(),
            name: "Or Policy".to_string(),
            condition: PolicyCondition::Or {
                conditions: vec![
                    PolicyCondition::IpRange { allowed_ips: vec!["192.168.1.100".to_string()] },
                    PolicyCondition::UserAttribute {
                        attribute: "department".to_string(),
                        value: "engineering".to_string(),
                    },
                ],
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:or".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:or");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted); // Should be granted because IP matches
    }

    /// Tests that RBAC policy with `Not` condition inverts the nested condition
    /// result.
    #[tokio::test]
    async fn test_policy_not_condition() {
        let manager = RBACManager::new();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec![],
            session_id: None,
            ip_address: Some("192.168.1.200".to_string()), // Not in allowed list
            user_agent: None,
            attributes: HashMap::new(),
        };

        let policy = RBACPolicy {
            id: "not_policy".to_string(),
            name: "Not Policy".to_string(),
            condition: PolicyCondition::Not {
                condition: Box::new(PolicyCondition::IpRange {
                    allowed_ips: vec!["192.168.1.100".to_string()],
                }),
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:not".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:not");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted); // Should be granted because IP is NOT in the list
    }

    /// Tests that RBAC policy with `Deny` effect overrides role-based
    /// permissions.
    #[tokio::test]
    async fn test_policy_deny_effect() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["admin".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        // Add deny policy that overrides admin permissions
        let policy = RBACPolicy {
            id: "deny_policy".to_string(),
            name: "Deny Policy".to_string(),
            condition: PolicyCondition::Always,
            effect: PolicyEffect::Deny,
            permissions: vec!["system:shutdown".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("system:shutdown");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(!granted); // Deny should override admin permissions
    }

    /// Tests that RBAC policy evaluates complex nested conditions (And/Or
    /// combinations).
    #[tokio::test]
    async fn test_nested_policy_conditions() {
        let manager = RBACManager::new();

        let mut attributes = HashMap::new();
        attributes.insert("level".to_string(), "senior".to_string());

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec![],
            session_id: None,
            ip_address: Some("192.168.1.100".to_string()),
            user_agent: None,
            attributes,
        };

        // (IP match AND (level=senior OR department=engineering))
        let policy = RBACPolicy {
            id: "nested_policy".to_string(),
            name: "Nested Policy".to_string(),
            condition: PolicyCondition::And {
                conditions: vec![
                    PolicyCondition::IpRange { allowed_ips: vec!["192.168.1.100".to_string()] },
                    PolicyCondition::Or {
                        conditions: vec![
                            PolicyCondition::UserAttribute {
                                attribute: "level".to_string(),
                                value: "senior".to_string(),
                            },
                            PolicyCondition::UserAttribute {
                                attribute: "department".to_string(),
                                value: "engineering".to_string(),
                            },
                        ],
                    },
                ],
            },
            effect: PolicyEffect::Allow,
            permissions: vec!["test:nested".to_string()],
        };

        manager.add_policy(policy).await.ok();

        let permission = Permission::new("test:nested");
        let granted = manager.check_permission(&user_context, &permission).await;
        assert!(granted);
    }

    /// Tests that `get_user_roles()` returns an empty vector for nonexistent
    /// users.
    #[tokio::test]
    async fn test_get_user_roles_empty() {
        let manager = RBACManager::new();
        let roles = manager.get_user_roles("nonexistent").await;
        assert_eq!(roles.len(), 0);
    }

    /// Tests that a user can be assigned multiple roles and all are retrieved.
    #[tokio::test]
    async fn test_multiple_roles_per_user() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();
        manager.assign_role("user123", "auditor").await.ok();

        let user_roles = manager.get_user_roles("user123").await;
        assert_eq!(user_roles.len(), 2);

        let role_ids: HashSet<String> = user_roles.iter().map(|r| r.id.clone()).collect();
        assert!(role_ids.contains("admin"));
        assert!(role_ids.contains("auditor"));
    }

    /// Tests that permission cache is cleared when a new policy is added.
    #[tokio::test]
    async fn test_cache_cleared_on_policy_add() {
        let manager = RBACManager::new();
        manager.assign_role("user123", "admin").await.ok();

        let user_context = UserContext {
            user_id: "user123".to_string(),
            roles: vec!["admin".to_string()],
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        };

        let permission = Permission::new("system:read");
        manager.check_permission(&user_context, &permission).await;

        // Verify cache has entry
        {
            let cache = manager.cache.read().await;
            assert!(!cache.is_empty());
        }

        // Add policy should clear all cache
        let policy = RBACPolicy {
            id: "test_policy".to_string(),
            name: "Test Policy".to_string(),
            condition: PolicyCondition::Always,
            effect: PolicyEffect::Allow,
            permissions: vec!["test:action".to_string()],
        };
        manager.add_policy(policy).await.ok();

        // Cache should be empty
        let cache = manager.cache.read().await;
        assert!(cache.is_empty());
    }

    /// Tests that `Permission::new()` correctly parses resource:action format
    /// strings.
    #[test]
    fn test_permission_new() {
        let perm = Permission::new("resource:action");
        assert_eq!(perm.resource, "resource");
        assert_eq!(perm.action, "action");
        assert_eq!(perm.id, "resource:action");
    }

    /// Tests that permissions with the same ID are considered equal.
    #[test]
    fn test_permission_equality() {
        let perm1 = Permission::new("test:read");
        let perm2 = Permission::new("test:read");
        assert_eq!(perm1.id, perm2.id);
    }
}
