# Compliance Primitives

Generic compliance infrastructure for enterprise applications including audit logging, configuration management, and feature flag systems.

## Overview

This module provides foundational compliance tools that help applications meet regulatory requirements, track user actions, manage configuration across environments, and control feature rollouts.

## Features

- **📝 Audit Logging**: Comprehensive event tracking for compliance and security
- **⚙️ Configuration Management**: Centralized config with remote updates
- **🚩 Feature Flags**: Runtime feature toggles for gradual rollouts and A/B testing

## Components

### 1. Audit Logging (`audit.rs`)

Track user actions and system events for compliance, security investigations, and debugging.

#### Key Types

```rust
pub struct AuditEvent {
    pub timestamp: SystemTime,
    pub severity: AuditSeverity,
    pub actor: String,           // User ID or system component
    pub action: String,          // What happened
    pub resource: String,        // What was affected
    pub outcome: String,         // Success, Failure, etc.
    pub context: AuditContext,   // Additional metadata
}

pub enum AuditSeverity {
    Info,      // Normal operations
    Warning,   // Unexpected but handled
    Critical,  // Security-relevant events
}

pub struct AuditContext {
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub session_id: Option<String>,
    pub metadata: HashMap<String, String>,
}
```

#### Usage Example

```rust
use agent::common::compliance::{GlobalAuditLogger, AuditEvent, AuditSeverity, AuditContext};

// Log a security-relevant event
GlobalAuditLogger::log(AuditEvent {
    timestamp: SystemTime::now(),
    severity: AuditSeverity::Critical,
    actor: "user_123".to_string(),
    action: "delete_data".to_string(),
    resource: "project_456".to_string(),
    outcome: "success".to_string(),
    context: AuditContext {
        ip_address: Some("192.168.1.100".to_string()),
        session_id: Some("session_abc".to_string()),
        ..Default::default()
    },
});

// Query audit logs
let events = GlobalAuditLogger::query(
    start_time,
    end_time,
    Some("user_123".to_string()),  // Filter by actor
);
```

#### Audit Best Practices

**What to Log:**
- ✅ Authentication events (login, logout, failed attempts)
- ✅ Authorization changes (role assignments, permission updates)
- ✅ Data access (read, write, delete sensitive data)
- ✅ Configuration changes (feature flags, settings)
- ✅ Security events (token generation, key rotation)

**What NOT to Log:**
- ❌ Sensitive data (passwords, tokens, PII)
- ❌ High-frequency events (every API call)
- ❌ Debugging information (use tracing instead)

### 2. Configuration Management (`config.rs`)

Centralized configuration management with support for remote updates and environment-specific overrides.

#### Key Types

```rust
pub struct ConfigManager {
    local_config: HashMap<String, String>,
    remote_config: Option<RemoteConfig>,
}

pub struct RemoteConfig {
    pub endpoint: String,
    pub api_key: String,
    pub refresh_interval: Duration,
}

impl ConfigManager {
    pub fn new() -> Self;
    pub fn set(&mut self, key: String, value: String);
    pub fn get(&self, key: &str) -> Option<&String>;
    pub fn get_or_default(&self, key: &str, default: &str) -> String;
    pub async fn sync_remote(&mut self) -> Result<(), ConfigError>;
}
```

#### Usage Example

```rust
use agent::common::compliance::{ConfigManager, RemoteConfig};
use std::time::Duration;

// Initialize with local config
let mut config = ConfigManager::new();
config.set("api_endpoint".to_string(), "https://api.example.com".to_string());
config.set("max_retries".to_string(), "3".to_string());

// Enable remote config updates
config.set_remote(RemoteConfig {
    endpoint: "https://config.example.com".to_string(),
    api_key: "config_key_123".to_string(),
    refresh_interval: Duration::from_secs(300),  // Refresh every 5 minutes
});

// Get configuration values
let api_endpoint = config.get_or_default("api_endpoint", "http://localhost");
let max_retries: u32 = config.get_or_default("max_retries", "3").parse().unwrap();

// Sync with remote config server
config.sync_remote().await?;
```

#### Configuration Hierarchy

The configuration system follows a precedence hierarchy:

1. **Environment Variables** (highest priority)
2. **Remote Configuration** (synced from server)
3. **Local Configuration** (set programmatically)
4. **Default Values** (fallback)

```rust
// Example: Environment variables override remote config
std::env::set_var("API_ENDPOINT", "https://staging.example.com");

let endpoint = config.get_or_default("api_endpoint", "http://localhost");
// Returns: "https://staging.example.com" (from env var)
```

### 3. Feature Flags (`feature_flags.rs`)

Runtime feature toggles for gradual rollouts, A/B testing, and emergency kill switches.

#### Key Types

```rust
pub struct FeatureFlagManager {
    flags: HashMap<String, FeatureFlag>,
}

pub struct FeatureFlag {
    pub key: String,
    pub enabled: bool,
    pub rollout_percentage: Option<f64>,  // 0.0 - 1.0
    pub allowed_users: Option<Vec<String>>,
    pub metadata: HashMap<String, String>,
}

impl FeatureFlagManager {
    pub fn new() -> Self;
    pub fn register_flag(&mut self, flag: FeatureFlag);
    pub fn is_enabled(&self, key: &str) -> bool;
    pub fn is_enabled_for_user(&self, key: &str, user_id: &str) -> bool;
    pub fn set_enabled(&mut self, key: &str, enabled: bool);
}
```

#### Usage Example

```rust
use agent::common::compliance::{FeatureFlagManager, FeatureFlag};

let mut flags = FeatureFlagManager::new();

// Register a new feature flag
flags.register_flag(FeatureFlag {
    key: "new_ui".to_string(),
    enabled: true,
    rollout_percentage: Some(0.1),  // 10% of users
    allowed_users: Some(vec!["beta_user_1".to_string()]),
    metadata: HashMap::new(),
});

// Check if feature is enabled globally
if flags.is_enabled("new_ui") {
    // Show new UI
}

// Check if feature is enabled for specific user
if flags.is_enabled_for_user("new_ui", &user_id) {
    // User is in the 10% rollout or is a beta user
    show_new_ui();
} else {
    show_old_ui();
}

// Emergency kill switch
flags.set_enabled("new_ui", false);
```

#### Feature Flag Strategies

**1. Boolean Flag (Simple On/Off)**
```rust
FeatureFlag {
    key: "maintenance_mode".to_string(),
    enabled: true,
    rollout_percentage: None,
    allowed_users: None,
    metadata: HashMap::new(),
}
```

**2. Percentage Rollout (Gradual Release)**
```rust
FeatureFlag {
    key: "new_algorithm".to_string(),
    enabled: true,
    rollout_percentage: Some(0.25),  // 25% of users
    allowed_users: None,
    metadata: HashMap::new(),
}
```

**3. User Allowlist (Beta Testing)**
```rust
FeatureFlag {
    key: "experimental_feature".to_string(),
    enabled: true,
    rollout_percentage: None,
    allowed_users: Some(vec![
        "internal_user_1".to_string(),
        "beta_tester_2".to_string(),
    ]),
    metadata: HashMap::new(),
}
```

**4. Combined Strategy (Allowlist + Rollout)**
```rust
FeatureFlag {
    key: "premium_feature".to_string(),
    enabled: true,
    rollout_percentage: Some(0.05),  // 5% of general users
    allowed_users: Some(vec!["vip_user_1".to_string()]),  // Always enabled for VIPs
    metadata: {
        let mut m = HashMap::new();
        m.insert("description".to_string(), "Premium analytics dashboard".to_string());
        m
    },
}
```

## Integration Examples

### Complete Compliance Setup

```rust
use agent::common::compliance::{
    GlobalAuditLogger, ConfigManager, FeatureFlagManager,
    AuditEvent, AuditSeverity, AuditContext, FeatureFlag
};
use std::sync::Arc;
use parking_lot::RwLock;

// Initialize compliance infrastructure
pub struct ComplianceInfra {
    pub config: Arc<RwLock<ConfigManager>>,
    pub flags: Arc<RwLock<FeatureFlagManager>>,
}

impl ComplianceInfra {
    pub fn new() -> Self {
        let mut config = ConfigManager::new();
        config.set("app_version".to_string(), "1.0.0".to_string());
        config.set("environment".to_string(), "production".to_string());

        let mut flags = FeatureFlagManager::new();
        flags.register_flag(FeatureFlag {
            key: "analytics".to_string(),
            enabled: true,
            rollout_percentage: Some(1.0),  // 100% rollout
            allowed_users: None,
            metadata: HashMap::new(),
        });

        Self {
            config: Arc::new(RwLock::new(config)),
            flags: Arc::new(RwLock::new(flags)),
        }
    }

    pub fn log_action(&self, user_id: &str, action: &str, resource: &str) {
        GlobalAuditLogger::log(AuditEvent {
            timestamp: SystemTime::now(),
            severity: AuditSeverity::Info,
            actor: user_id.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
            outcome: "success".to_string(),
            context: AuditContext::default(),
        });
    }

    pub fn check_feature(&self, feature_key: &str, user_id: &str) -> bool {
        let flags = self.flags.read();
        flags.is_enabled_for_user(feature_key, user_id)
    }
}
```

### Compliance Middleware (Web Server)

```rust
use axum::{
    middleware::Next,
    response::Response,
    extract::Request,
};

pub async fn audit_middleware(
    request: Request,
    next: Next,
) -> Response {
    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let user_id = extract_user_id(&request);  // Your auth logic

    // Log the request
    GlobalAuditLogger::log(AuditEvent {
        timestamp: SystemTime::now(),
        severity: AuditSeverity::Info,
        actor: user_id.clone(),
        action: format!("{} {}", method, uri),
        resource: uri.clone(),
        outcome: "initiated".to_string(),
        context: AuditContext::default(),
    });

    let response = next.run(request).await;

    // Log the response
    let outcome = if response.status().is_success() {
        "success"
    } else {
        "failure"
    };

    GlobalAuditLogger::log(AuditEvent {
        timestamp: SystemTime::now(),
        severity: if outcome == "failure" {
            AuditSeverity::Warning
        } else {
            AuditSeverity::Info
        },
        actor: user_id,
        action: format!("{} {}", method, uri),
        resource: uri,
        outcome: outcome.to_string(),
        context: AuditContext::default(),
    });

    response
}
```

## Testing

### Unit Tests

```bash
# Run compliance tests
cargo test --package agent --lib common::compliance

# Test specific module
cargo test --package agent --lib common::compliance::audit
cargo test --package agent --lib common::compliance::feature_flags
```

### Testing Feature Flags

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_rollout() {
        let mut flags = FeatureFlagManager::new();
        flags.register_flag(FeatureFlag {
            key: "test_feature".to_string(),
            enabled: true,
            rollout_percentage: Some(0.5),
            allowed_users: None,
            metadata: HashMap::new(),
        });

        // Test with deterministic user IDs
        let enabled_count = (0..1000)
            .filter(|i| flags.is_enabled_for_user("test_feature", &format!("user_{}", i)))
            .count();

        // Should be approximately 50% (500 ± tolerance)
        assert!((450..550).contains(&enabled_count));
    }

    #[test]
    fn test_user_allowlist() {
        let mut flags = FeatureFlagManager::new();
        flags.register_flag(FeatureFlag {
            key: "beta_feature".to_string(),
            enabled: true,
            rollout_percentage: None,
            allowed_users: Some(vec!["user_1".to_string()]),
            metadata: HashMap::new(),
        });

        assert!(flags.is_enabled_for_user("beta_feature", "user_1"));
        assert!(!flags.is_enabled_for_user("beta_feature", "user_2"));
    }
}
```

## Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
parking_lot = "0.12"
tokio = { version = "1.0", features = ["full"] }
```

## Compliance Standards

This module helps meet requirements for:

- **GDPR**: Audit logs for data access and deletion
- **HIPAA**: Access logs for protected health information
- **SOC 2**: Change management and access control
- **ISO 27001**: Information security event management

## Best Practices

### 1. Audit Log Retention

```rust
// Implement log rotation and archival
impl GlobalAuditLogger {
    pub async fn archive_old_logs(&self, older_than: Duration) {
        // Move logs older than retention period to archive storage
        // Example: 90 days for compliance
    }
}
```

### 2. Configuration Security

```rust
// Never log sensitive configuration values
let api_key = config.get("api_key").unwrap();
// ❌ Don't: log!("API Key: {}", api_key);
// ✅ Do: log!("API Key configured: {}", api_key.is_some());
```

### 3. Feature Flag Hygiene

```rust
// Remove feature flags once fully rolled out
if flags.is_enabled("new_feature") && rollout_percentage == 1.0 {
    // Time to remove the flag and make this the default behavior
}
```

## Roadmap

- [ ] Audit log export to external SIEM systems
- [ ] Configuration schema validation
- [ ] Feature flag analytics and dashboards
- [ ] Multi-tenancy support for feature flags
- [ ] Integration with external feature flag services (LaunchDarkly, Split.io)

## License

See the root LICENSE file for licensing information.
