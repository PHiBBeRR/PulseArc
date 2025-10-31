//! Sample Entities and Data Generators
//!
//! **What this file contains:**
//! This module provides sample data structures and batch generators for
//! testing. These are NOT mocks - they are actual data instances used in tests.
//!
//! ## Sample Entities
//! - `TestUser` - Sample user entities with fields: id, email, name, age,
//!   active
//!   - `sample()` - Single active user
//!   - `inactive()` - Single inactive user
//!   - `batch(n)` - Generate n users
//!
//! - `TestProject` - Sample project entities with fields: id, name,
//!   description, owner_id, tags
//!   - `sample()` - Single sample project
//!   - `batch(n)` - Generate n projects
//!
//! - `TestConfig` - Sample configuration structures
//!   - `default()` - Default config
//!   - `with_overrides()` - Config with custom values
//!
//! ## Data Generators
//! - `sample_emails(n)` - Generate n valid email addresses
//! - `sample_urls(n)` - Generate n valid URLs
//! - `sample_ip_addresses(n)` - Generate n IP addresses
//!
//! ## Usage Example
//! ```rust
//! use data::sample_entities::*;
//!
//! let users = TestUser::batch(10); // 10 test users
//! let emails = sample_emails(5); // 5 email addresses
//! ```

use std::collections::HashMap;

use pulsearc_common::testing::fixtures::{
    random_email_seeded, random_hex_seeded, random_string_seeded,
};
use serde::{Deserialize, Serialize};

/// Sample user data structure for testing.
///
/// Represents a typical user entity with common fields used across integration
/// tests. Serializable for storage/API testing and cloneable for test data
/// reuse.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestUser {
    pub id: String,
    pub email: String,
    pub name: String,
    pub age: u32,
    pub active: bool,
}

#[allow(dead_code)]
impl TestUser {
    /// Creates a new TestUser with specified fields and active=true by default.
    pub fn new(id: String, email: String, name: String, age: u32) -> Self {
        Self { id, email, name, age, active: true }
    }

    /// Returns a sample active user with preset values for quick testing.
    pub fn sample() -> Self {
        Self {
            id: "user_123".to_string(),
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
            age: 30,
            active: true,
        }
    }

    /// Returns a sample inactive user for testing inactive user scenarios.
    pub fn inactive() -> Self {
        Self {
            id: "inactive_456".to_string(),
            email: "inactive@example.com".to_string(),
            name: "Inactive User".to_string(),
            age: 25,
            active: false,
        }
    }

    pub fn batch(count: usize) -> Vec<Self> {
        (0..count)
            .map(|i| Self {
                id: format!("user_{:04}", i),
                email: random_email_seeded(i as u64 + 1),
                name: format!("User {}", i),
                age: 20 + (i % 50) as u32,
                active: i % 2 == 0,
            })
            .collect()
    }
}

/// Sample project data structure for testing.
///
/// Represents a project entity with typical fields including owner reference
/// and tags. Useful for testing multi-entity relationships and nested data
/// structures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestProject {
    pub id: String,
    pub name: String,
    pub description: String,
    pub owner_id: String,
    pub tags: Vec<String>,
}

impl TestProject {
    /// Returns a sample project with preset values for quick testing.
    pub fn sample() -> Self {
        Self {
            id: "proj_001".to_string(),
            name: "Sample Project".to_string(),
            description: "A test project for integration testing".to_string(),
            owner_id: "user_123".to_string(),
            tags: vec!["test".to_string(), "integration".to_string()],
        }
    }

    pub fn batch(count: usize) -> Vec<Self> {
        (0..count)
            .map(|i| Self {
                id: format!("proj_{:04}", i),
                name: format!("Project {}", i),
                description: random_string_seeded(32, i as u64 + 1_000),
                owner_id: format!("user_{}", i % 10),
                tags: vec![random_string_seeded(6, i as u64 + 2_000)],
            })
            .collect()
    }
}

/// Sample configuration structure for testing config validation and
/// serialization.
///
/// Contains nested limits structure and feature flags for testing complex
/// configuration scenarios.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TestConfig {
    pub app_name: String,
    pub version: String,
    pub features: HashMap<String, bool>,
    pub limits: ConfigLimits,
}

/// Configuration limits for testing nested configuration structures.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigLimits {
    pub max_users: usize,
    pub max_requests_per_second: u32,
    pub timeout_seconds: u64,
}

impl TestConfig {
    /// Returns default configuration with preset feature flags and limits.
    pub fn default_config() -> Self {
        let mut features = HashMap::new();
        features.insert("feature_a".to_string(), true);
        features.insert("feature_b".to_string(), false);
        features.insert("feature_c".to_string(), true);

        Self {
            app_name: "PulseArcTest".to_string(),
            version: "1.0.0".to_string(),
            features,
            limits: ConfigLimits {
                max_users: 1000,
                max_requests_per_second: 100,
                timeout_seconds: 30,
            },
        }
    }
}

/// Generates sample email addresses for testing email validation and PII
/// detection.
///
/// # Arguments
/// * `count` - Number of email addresses to generate
///
/// # Returns
/// Vector of valid email addresses in format: user{N}@example.com
pub fn sample_emails(count: usize) -> Vec<String> {
    (0..count).map(|i| random_email_seeded(i as u64 + 42)).collect()
}

/// Generates sample URLs for testing URL validation and parsing.
///
/// # Arguments
/// * `count` - Number of URLs to generate
///
/// # Returns
/// Vector of HTTPS URLs in format: https://example{N}.com/path
#[allow(dead_code)]
pub fn sample_urls(count: usize) -> Vec<String> {
    (0..count)
        .map(|i| format!("https://example.com/{}", random_string_seeded(12, i as u64 + 84)))
        .collect()
}

/// Generates sample IP addresses for testing IP validation and PII detection.
///
/// # Arguments
/// * `count` - Number of IP addresses to generate
///
/// # Returns
/// Vector of IPv4 addresses in 192.168.1.{N} range
#[allow(dead_code)]
pub fn sample_ips(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("192.168.1.{}", i % 255)).collect()
}

/// Generates sample phone numbers for testing phone validation and PII
/// detection.
///
/// # Arguments
/// * `count` - Number of phone numbers to generate
///
/// # Returns
/// Vector of US-formatted phone numbers: +1-555-{NNNN}
#[allow(dead_code)]
pub fn sample_phone_numbers(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("+1-555-{:04}", i % 10000)).collect()
}

/// Generates cryptographically random sample API keys for testing
/// authentication.
///
/// # Arguments
/// * `count` - Number of API keys to generate
///
/// # Returns
/// Vector of 64-character hex-encoded API keys (32 random bytes each)
pub fn sample_api_keys(count: usize) -> Vec<String> {
    (0..count).map(|i| random_hex_seeded(64, i as u64 + 128)).collect()
}

/// Generates sample JSON objects for testing JSON serialization and parsing.
///
/// # Arguments
/// * `count` - Number of JSON objects to generate
///
/// # Returns
/// Vector of JSON objects with id, name, value, active status, and metadata
/// fields
pub fn sample_json_objects(count: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            serde_json::json!({
                "id": i,
                "name": format!("Item {}", i),
                "value": i * 100,
                "active": i % 2 == 0,
                "metadata": {
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z"
                }
            })
        })
        .collect()
}

/// Generates common error messages for testing error handling and logging.
///
/// # Returns
/// Vector of typical error messages covering various failure scenarios
#[allow(dead_code)]
pub fn sample_error_messages() -> Vec<String> {
    vec![
        "Invalid input parameter".to_string(),
        "Resource not found".to_string(),
        "Authentication failed".to_string(),
        "Rate limit exceeded".to_string(),
        "Internal server error".to_string(),
        "Connection timeout".to_string(),
        "Insufficient permissions".to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates `TestUser::sample` behavior for the sample user scenario.
    ///
    /// Assertions:
    /// - Confirms `user.id` equals `"user_123"`.
    /// - Confirms `user.email` equals `"test@example.com"`.
    /// - Ensures `user.active` evaluates to true.
    #[test]
    fn test_sample_user() {
        let user = TestUser::sample();
        assert_eq!(user.id, "user_123");
        assert_eq!(user.email, "test@example.com");
        assert!(user.active);
    }

    /// Validates `TestUser::batch` behavior for the user batch scenario.
    ///
    /// Assertions:
    /// - Confirms `users.len()` equals `5`.
    /// - Confirms `users[0].id` equals `"user_0000"`.
    /// - Confirms `users[4].id` equals `"user_0004"`.
    #[test]
    fn test_user_batch() {
        let users = TestUser::batch(5);
        assert_eq!(users.len(), 5);
        assert_eq!(users[0].id, "user_0000");
        assert_eq!(users[4].id, "user_0004");
    }

    /// Validates `TestProject::sample` behavior for the sample project
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `project.id` equals `"proj_001"`.
    /// - Ensures `!project.tags.is_empty()` evaluates to true.
    #[test]
    fn test_sample_project() {
        let project = TestProject::sample();
        assert_eq!(project.id, "proj_001");
        assert!(!project.tags.is_empty());
    }

    /// Validates `TestProject::batch` behavior for the project batch scenario.
    ///
    /// Assertions:
    /// - Confirms `projects.len()` equals `3`.
    #[test]
    fn test_project_batch() {
        let projects = TestProject::batch(3);
        assert_eq!(projects.len(), 3);
    }

    /// Validates `TestConfig::default_config` behavior for the default config
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.app_name` equals `"PulseArcTest"`.
    /// - Confirms `config.version` equals `"1.0.0"`.
    /// - Ensures `config.features.contains_key("feature_a")` evaluates to true.
    #[test]
    fn test_default_config() {
        let config = TestConfig::default_config();
        assert_eq!(config.app_name, "PulseArcTest");
        assert_eq!(config.version, "1.0.0");
        assert!(config.features.contains_key("feature_a"));
    }

    /// Validates the sample emails scenario.
    ///
    /// Assertions:
    /// - Confirms `emails.len()` equals `3`.
    /// - Ensures `emails[0].contains("@example.com")` evaluates to true.
    #[test]
    fn test_sample_emails() {
        let emails = sample_emails(3);
        assert_eq!(emails.len(), 3);
        assert!(emails[0].contains("@example.com"));
    }

    /// Validates the sample api keys scenario.
    ///
    /// Assertions:
    /// - Confirms `keys.len()` equals `2`.
    /// - Confirms `keys[0].len()` equals `64`.
    #[test]
    fn test_sample_api_keys() {
        let keys = sample_api_keys(2);
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0].len(), 64); // 32 bytes = 64 hex chars
    }

    /// Validates the sample json objects scenario.
    ///
    /// Assertions:
    /// - Confirms `objects.len()` equals `2`.
    /// - Ensures `objects[0].is_object()` evaluates to true.
    #[test]
    fn test_sample_json_objects() {
        let objects = sample_json_objects(2);
        assert_eq!(objects.len(), 2);
        assert!(objects[0].is_object());
    }
}
