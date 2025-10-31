// Feature Flags System for A/B Testing and Gradual Rollouts

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tracing::info;

use crate::security::rbac::UserContext;

/// Feature flag definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub rollout_percentage: f32,
    pub target_roles: Vec<String>,
    pub target_users: Vec<String>,
    pub metadata: HashMap<String, String>,
}

/// Feature flag manager
#[derive(Debug)]
pub struct FeatureFlagManager {
    flags: HashMap<String, FeatureFlag>,
}

impl Default for FeatureFlagManager {
    fn default() -> Self {
        Self::new()
    }
}

impl FeatureFlagManager {
    pub fn new() -> Self {
        let mut manager = Self { flags: HashMap::new() };
        manager.initialize_default_flags();
        manager
    }

    pub fn initialize(&mut self) -> Result<(), String> {
        info!("Initializing feature flags");
        self.initialize_default_flags();
        Ok(())
    }

    fn initialize_default_flags(&mut self) {
        self.flags.insert(
            "enterprise_menu".to_string(),
            FeatureFlag {
                id: "enterprise_menu".to_string(),
                name: "Enterprise Menu".to_string(),
                description: "Enable enterprise menu features".to_string(),
                enabled: true,
                rollout_percentage: 100.0,
                target_roles: vec!["admin".to_string(), "power_user".to_string()],
                target_users: vec![],
                metadata: HashMap::new(),
            },
        );

        self.flags.insert(
            "advanced_telemetry".to_string(),
            FeatureFlag {
                id: "advanced_telemetry".to_string(),
                name: "Advanced Telemetry".to_string(),
                description: "Enable advanced telemetry collection".to_string(),
                enabled: false,
                rollout_percentage: 0.0,
                target_roles: vec![],
                target_users: vec![],
                metadata: HashMap::new(),
            },
        );
    }

    pub async fn is_enabled(&self, flag_id: &str, context: Option<&UserContext>) -> bool {
        if let Some(flag) = self.flags.get(flag_id) {
            if !flag.enabled {
                return false;
            }

            if let Some(ctx) = context {
                // Check target users
                if flag.target_users.contains(&ctx.user_id) {
                    return true;
                }

                // Check target roles
                for role in &ctx.roles {
                    if flag.target_roles.contains(role) {
                        return true;
                    }
                }

                // Check rollout percentage
                if flag.rollout_percentage >= 100.0 {
                    return true;
                }

                if flag.rollout_percentage <= 0.0 {
                    return false;
                }

                // FNV-1a hash-based rollout (deterministic per user + flag_id)
                // This ensures same user gets different rollouts for different flags
                let hash = Self::fnv1a_hash(&format!("{}:{}", ctx.user_id, flag_id));
                let threshold = (flag.rollout_percentage * 100.0) as u64;
                return (hash % 10000) < threshold;
            }

            flag.rollout_percentage >= 100.0
        } else {
            false
        }
    }

    /// FNV-1a hash function for deterministic rollout
    /// Returns a 64-bit hash value with good distribution properties
    fn fnv1a_hash(s: &str) -> u64 {
        const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
        const FNV_PRIME: u64 = 1099511628211;

        let mut hash = FNV_OFFSET_BASIS;
        for byte in s.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(FNV_PRIME);
        }
        hash
    }

    pub fn toggle_flag(&mut self, flag_id: &str, enabled: bool) -> Result<(), String> {
        if let Some(flag) = self.flags.get_mut(flag_id) {
            flag.enabled = enabled;
            info!("Feature flag '{}' set to {}", flag_id, enabled);
            Ok(())
        } else {
            Err(format!("Feature flag '{}' not found", flag_id))
        }
    }

    pub fn get_flag(&self, flag_id: &str) -> Option<&FeatureFlag> {
        self.flags.get(flag_id)
    }

    pub fn add_flag(&mut self, flag: FeatureFlag) -> Result<(), String> {
        let flag_id = flag.id.clone();
        if self.flags.contains_key(&flag_id) {
            return Err(format!("Feature flag '{}' already exists", flag_id));
        }
        self.flags.insert(flag_id.clone(), flag);
        info!("Added feature flag '{}'", flag_id);
        Ok(())
    }

    pub fn set_rollout_percentage(&mut self, flag_id: &str, percentage: f32) -> Result<(), String> {
        if !(0.0..=100.0).contains(&percentage) {
            return Err("Rollout percentage must be between 0 and 100".to_string());
        }

        if let Some(flag) = self.flags.get_mut(flag_id) {
            flag.rollout_percentage = percentage;
            info!("Feature flag '{}' rollout set to {}%", flag_id, percentage);
            Ok(())
        } else {
            Err(format!("Feature flag '{}' not found", flag_id))
        }
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for compliance::feature_flags.
    use std::collections::HashMap;

    use super::*;

    fn create_test_user_context(user_id: &str, roles: Vec<&str>) -> UserContext {
        UserContext {
            user_id: user_id.to_string(),
            roles: roles.iter().map(|r| r.to_string()).collect(),
            session_id: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Tests that `FeatureFlagManager::new()` initializes with default flags.
    #[test]
    fn test_feature_flag_manager_new() {
        let manager = FeatureFlagManager::new();
        assert!(!manager.flags.is_empty());
    }

    /// Tests that default feature flags (enterprise_menu, advanced_telemetry)
    /// are initialized correctly.
    #[test]
    fn test_default_flags_initialized() {
        let manager = FeatureFlagManager::new();
        assert!(manager.get_flag("enterprise_menu").is_some());
        assert!(manager.get_flag("advanced_telemetry").is_some());

        let enterprise_flag = manager.get_flag("enterprise_menu").unwrap();
        assert!(enterprise_flag.enabled);
        assert_eq!(enterprise_flag.rollout_percentage, 100.0);

        let telemetry_flag = manager.get_flag("advanced_telemetry").unwrap();
        assert!(!telemetry_flag.enabled);
        assert_eq!(telemetry_flag.rollout_percentage, 0.0);
    }

    /// Tests that `is_enabled()` returns false for disabled feature flags.
    #[tokio::test]
    async fn test_is_enabled_flag_disabled() {
        let manager = FeatureFlagManager::new();
        let context = create_test_user_context("user123", vec!["admin"]);

        // advanced_telemetry is disabled by default
        let enabled = manager.is_enabled("advanced_telemetry", Some(&context)).await;
        assert!(!enabled);
    }

    /// Tests that `is_enabled()` returns false for nonexistent feature flags.
    #[tokio::test]
    async fn test_is_enabled_nonexistent_flag() {
        let manager = FeatureFlagManager::new();
        let context = create_test_user_context("user123", vec!["admin"]);

        let enabled = manager.is_enabled("nonexistent", Some(&context)).await;
        assert!(!enabled);
    }

    /// Tests that `is_enabled()` grants access to specifically targeted users.
    #[tokio::test]
    async fn test_is_enabled_target_user() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "test_flag".to_string(),
            name: "Test Flag".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 0.0, // 0% rollout
            target_roles: vec![],
            target_users: vec!["user123".to_string()],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        let context = create_test_user_context("user123", vec![]);
        let enabled = manager.is_enabled("test_flag", Some(&context)).await;
        assert!(enabled); // Should be enabled for targeted user
    }

    /// Tests that `is_enabled()` grants access based on user roles.
    #[tokio::test]
    async fn test_is_enabled_target_role() {
        let manager = FeatureFlagManager::new();
        let context = create_test_user_context("user123", vec!["admin"]);

        // enterprise_menu targets admin and power_user roles
        let enabled = manager.is_enabled("enterprise_menu", Some(&context)).await;
        assert!(enabled);
    }

    /// Tests that `is_enabled()` uses rollout percentage when user is not
    /// specifically targeted.
    #[tokio::test]
    async fn test_is_enabled_not_targeted() {
        let manager = FeatureFlagManager::new();
        let context = create_test_user_context("user123", vec!["guest"]);

        // enterprise_menu targets admin and power_user roles only
        // But rollout is 100%, so it should still be enabled
        let enabled = manager.is_enabled("enterprise_menu", Some(&context)).await;
        assert!(enabled);
    }

    /// Tests that 100% rollout percentage enables flag for all users.
    #[tokio::test]
    async fn test_rollout_percentage_100() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "full_rollout".to_string(),
            name: "Full Rollout".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 100.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        let context = create_test_user_context("any_user", vec![]);
        let enabled = manager.is_enabled("full_rollout", Some(&context)).await;
        assert!(enabled);
    }

    /// Tests that 0% rollout percentage disables flag for all users.
    #[tokio::test]
    async fn test_rollout_percentage_0() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "no_rollout".to_string(),
            name: "No Rollout".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 0.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        let context = create_test_user_context("user123", vec![]);
        let enabled = manager.is_enabled("no_rollout", Some(&context)).await;
        assert!(!enabled);
    }

    /// Tests that hash-based rollout produces deterministic results for the
    /// same user.
    #[tokio::test]
    async fn test_hash_based_rollout_deterministic() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "partial_rollout".to_string(),
            name: "Partial Rollout".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 50.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        let context = create_test_user_context("testuser", vec![]);

        // Check multiple times - should be consistent
        let result1 = manager.is_enabled("partial_rollout", Some(&context)).await;
        let result2 = manager.is_enabled("partial_rollout", Some(&context)).await;
        let result3 = manager.is_enabled("partial_rollout", Some(&context)).await;

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    /// Tests that 0% and 100% rollout percentages apply consistently to all
    /// users.
    #[tokio::test]
    async fn test_hash_based_rollout_different_users() {
        let mut manager = FeatureFlagManager::new();

        // Test with 0% rollout - should exclude everyone except targeted users
        let flag_0_percent = FeatureFlag {
            id: "no_rollout".to_string(),
            name: "No Rollout".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 0.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag_0_percent).ok();

        let test_users = vec!["user1", "user2", "user3"];
        for user_id in &test_users {
            let context = create_test_user_context(user_id, vec![]);
            let enabled = manager.is_enabled("no_rollout", Some(&context)).await;
            assert!(!enabled, "Expected 0% rollout to exclude all users");
        }

        // Test with 100% rollout - should include everyone
        let flag_100_percent = FeatureFlag {
            id: "full_rollout".to_string(),
            name: "Full Rollout".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 100.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag_100_percent).ok();

        for user_id in &test_users {
            let context = create_test_user_context(user_id, vec![]);
            let enabled = manager.is_enabled("full_rollout", Some(&context)).await;
            assert!(enabled, "Expected 100% rollout to include all users");
        }
    }

    /// Tests that `is_enabled()` uses only rollout percentage when no user
    /// context is provided.
    #[tokio::test]
    async fn test_is_enabled_no_context() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "no_context".to_string(),
            name: "No Context".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 100.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        // Without context, should use rollout percentage only
        let enabled = manager.is_enabled("no_context", None).await;
        assert!(enabled);
    }

    /// Tests that partial rollout without context returns false (no user to
    /// hash).
    #[tokio::test]
    async fn test_is_enabled_no_context_partial_rollout() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "partial_no_context".to_string(),
            name: "Partial No Context".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 50.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        // Without context and partial rollout, should be disabled
        let enabled = manager.is_enabled("partial_no_context", None).await;
        assert!(!enabled);
    }

    /// Tests that `toggle_flag()` successfully enables and disables feature
    /// flags.
    #[test]
    fn test_toggle_flag() {
        let mut manager = FeatureFlagManager::new();

        // Toggle enterprise_menu off
        let result = manager.toggle_flag("enterprise_menu", false);
        assert!(result.is_ok());

        let flag = manager.get_flag("enterprise_menu").unwrap();
        assert!(!flag.enabled);

        // Toggle back on
        let result = manager.toggle_flag("enterprise_menu", true);
        assert!(result.is_ok());

        let flag = manager.get_flag("enterprise_menu").unwrap();
        assert!(flag.enabled);
    }

    /// Tests that `toggle_flag()` returns an error for nonexistent flags.
    #[test]
    fn test_toggle_nonexistent_flag() {
        let mut manager = FeatureFlagManager::new();
        let result = manager.toggle_flag("nonexistent", true);
        assert!(result.is_err());
    }

    /// Tests that `add_flag()` successfully creates a new feature flag.
    #[test]
    fn test_add_flag() {
        let mut manager = FeatureFlagManager::new();

        let new_flag = FeatureFlag {
            id: "new_feature".to_string(),
            name: "New Feature".to_string(),
            description: "A new feature".to_string(),
            enabled: true,
            rollout_percentage: 100.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        let result = manager.add_flag(new_flag);
        assert!(result.is_ok());
        assert!(manager.get_flag("new_feature").is_some());
    }

    /// Tests that `add_flag()` returns an error when adding a duplicate flag.
    #[test]
    fn test_add_duplicate_flag() {
        let mut manager = FeatureFlagManager::new();

        let duplicate_flag = FeatureFlag {
            id: "enterprise_menu".to_string(), // Already exists
            name: "Duplicate".to_string(),
            description: "Duplicate".to_string(),
            enabled: true,
            rollout_percentage: 100.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: HashMap::new(),
        };

        let result = manager.add_flag(duplicate_flag);
        assert!(result.is_err());
    }

    /// Tests that `set_rollout_percentage()` successfully updates rollout
    /// percentage.
    #[test]
    fn test_set_rollout_percentage() {
        let mut manager = FeatureFlagManager::new();

        let result = manager.set_rollout_percentage("enterprise_menu", 75.0);
        assert!(result.is_ok());

        let flag = manager.get_flag("enterprise_menu").unwrap();
        assert_eq!(flag.rollout_percentage, 75.0);
    }

    /// Tests that `set_rollout_percentage()` rejects invalid percentage values.
    #[test]
    fn test_set_rollout_percentage_invalid() {
        let mut manager = FeatureFlagManager::new();

        let result = manager.set_rollout_percentage("enterprise_menu", 150.0);
        assert!(result.is_err());

        let result = manager.set_rollout_percentage("enterprise_menu", -10.0);
        assert!(result.is_err());
    }

    /// Tests that `set_rollout_percentage()` returns an error for nonexistent
    /// flags.
    #[test]
    fn test_set_rollout_percentage_nonexistent_flag() {
        let mut manager = FeatureFlagManager::new();
        let result = manager.set_rollout_percentage("nonexistent", 50.0);
        assert!(result.is_err());
    }

    /// Tests that `is_enabled()` matches if user has any of the targeted roles.
    #[tokio::test]
    async fn test_multiple_roles_matching() {
        let manager = FeatureFlagManager::new();
        let context = create_test_user_context("user123", vec!["user", "power_user"]);

        // enterprise_menu targets admin and power_user
        let enabled = manager.is_enabled("enterprise_menu", Some(&context)).await;
        assert!(enabled); // Should match power_user role
    }

    /// Tests that flag enabled status takes precedence over targeting (disabled
    /// flag stays disabled).
    #[tokio::test]
    async fn test_target_user_overrides_disabled_flag() {
        let mut manager = FeatureFlagManager::new();

        let flag = FeatureFlag {
            id: "disabled_flag".to_string(),
            name: "Disabled Flag".to_string(),
            description: "Test".to_string(),
            enabled: false, // Disabled globally
            rollout_percentage: 0.0,
            target_roles: vec![],
            target_users: vec!["vip_user".to_string()],
            metadata: HashMap::new(),
        };

        manager.add_flag(flag).ok();

        let context = create_test_user_context("vip_user", vec![]);
        let enabled = manager.is_enabled("disabled_flag", Some(&context)).await;
        assert!(!enabled); // Should be disabled because flag.enabled = false
    }

    /// Tests that `initialize()` populates default flags.
    #[test]
    fn test_initialize() {
        let mut manager = FeatureFlagManager { flags: HashMap::new() };

        assert!(manager.flags.is_empty());

        let result = manager.initialize();
        assert!(result.is_ok());
        assert!(!manager.flags.is_empty());
    }

    /// Tests that feature flags can store custom metadata.
    #[test]
    fn test_feature_flag_metadata() {
        let mut metadata = HashMap::new();
        metadata.insert("environment".to_string(), "production".to_string());

        let flag = FeatureFlag {
            id: "with_metadata".to_string(),
            name: "With Metadata".to_string(),
            description: "Test".to_string(),
            enabled: true,
            rollout_percentage: 100.0,
            target_roles: vec![],
            target_users: vec![],
            metadata: metadata.clone(),
        };

        assert_eq!(flag.metadata.get("environment"), Some(&"production".to_string()));
    }
}
