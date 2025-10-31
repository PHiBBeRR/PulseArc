# Security Module - Generic Security Infrastructure

Comprehensive security primitives including encryption, key management, Role-Based Access Control (RBAC), and access control abstractions.

## Overview

This module provides generic security components that can be used across different domains and applications. It includes a complete encryption infrastructure with keychain integration, a flexible RBAC system with dynamic policies, and trait abstractions for access control and validation.

## Features

- **Encryption Infrastructure**: Key generation, rotation, secure storage, and caching
- **Keychain Integration**: Platform-specific secure credential storage
- **RBAC System**: Role-based access control with hierarchies and dynamic policies
- **Policy Engine**: Time-based, IP-based, and attribute-based access policies
- **Trait Abstractions**: Pluggable access control and validation implementations
- **No-Op Implementations**: Test doubles for development and testing
- **Secure Memory**: Zero-on-drop strings for sensitive data

## Architecture

```text
┌───────────────────────────────────────────┐
│         Security Module                   │
├───────────────────────────────────────────┤
│                                           │
│  ┌─────────────────────────────────┐     │
│  │   Encryption (encryption/)      │     │
│  │  • Key generation & rotation    │     │
│  │  • SecureString (zero-on-drop)  │     │
│  │  • KeychainProvider             │     │
│  │  • Key caching                  │     │
│  │  • StorageKeyManager            │     │
│  └─────────────────────────────────┘     │
│                                           │
│  ┌─────────────────────────────────┐     │
│  │   RBAC (rbac.rs)                │     │
│  │  • RBACManager                  │     │
│  │  • Roles & Permissions          │     │
│  │  • Policy engine                │     │
│  │  • Permission caching           │     │
│  │  • Policy conditions            │     │
│  └─────────────────────────────────┘     │
│                                           │
│  ┌─────────────────────────────────┐     │
│  │   Traits (traits.rs)            │     │
│  │  • AccessControl trait          │     │
│  │  • Validator trait              │     │
│  │  • No-op implementations        │     │
│  └─────────────────────────────────┘     │
│                                           │
└───────────────────────────────────────────┘
```

## Components

### 1. Encryption Infrastructure (`encryption/`)

Comprehensive encryption utilities for secure key management, storage, and rotation.

**Key Generation** (`keys.rs`)
- Generate random encryption keys
- Configurable key sizes
- Cryptographically secure randomness

**KeychainProvider** (`keychain.rs`)
- Platform-specific credential storage
- macOS: Keychain Access
- Windows: Credential Manager
- Linux: Secret Service API

**Key Caching** (`cache.rs`)
- In-memory key caching for performance
- Thread-safe access with RwLock
- Automatic cache invalidation

**Key Rotation** (`rotation.rs`)
- Automatic key rotation schedules
- Multi-version key support
- Graceful key transition

**SecureString** (`secure_string.rs`)
- Zero-on-drop sensitive strings
- Memory protection
- Safe string operations

**StorageKeyManager** (`rotation.rs`)
- Unified key management interface
- Database integration
- Rotation tracking

### 2. RBAC System (`rbac.rs`)

Flexible role-based access control with hierarchies and dynamic policies.

**RBACManager** - Central RBAC orchestrator:
- Role assignment and revocation
- Permission checking with wildcards
- Policy evaluation
- Permission caching
- Parent role inheritance

**Role** - User role definition:
- Unique ID and name
- Permission set
- Parent role (inheritance)
- Priority level

**Permission** - Permission definition:
- Resource and action
- Description
- Approval requirement

**RBACPolicy** - Dynamic access policies:
- Conditional permissions
- Allow/Deny effects
- Time-based access
- IP-based access
- Attribute-based access
- Complex condition combinations (AND/OR/NOT)

**PolicyCondition** - Policy evaluation conditions:
- `Always` - Unconditional
- `TimeRange` - Time-based access
- `IpRange` - IP-based access
- `UserAttribute` - Attribute-based access
- `And` - All conditions must match
- `Or` - Any condition must match
- `Not` - Inverted condition

### 3. Trait Abstractions (`traits.rs`)

Pluggable implementations for access control and validation.

**AccessControl** - RBAC interface:
- Permission checking
- Role verification
- Batch permission checks
- User permission listing

**Validator** - Generic validation:
- Value validation with type safety
- Field-level validation
- Error collection

**No-Op Implementations**:
- `NoOpAccessControl` - Allow all
- `DenyAllAccessControl` - Deny all
- `NoOpValidator` - Accept all

## Usage Examples

### Encryption Key Management

```rust
use agent::common::security::encryption::{
    generate_encryption_key,
    KeychainProvider,
    get_or_create_key,
    get_or_create_key_cached
};

// Generate a new encryption key
let key = generate_encryption_key(32)?;  // 32-byte key
println!("Key length: {}", key.len());

// Store key in keychain
let keychain = create_keychain_provider();
keychain.set("my_app", "encryption_key", &hex::encode(&key))?;

// Retrieve key from keychain
let stored_key = keychain.get("my_app", "encryption_key")?;

// Get or create with caching
let key = get_or_create_key_cached(
    &keychain,
    "my_app",
    "db_key",
    32
)?;
```

### Secure String Usage

```rust
use agent::common::security::encryption::SecureString;

// Create secure string
let password = SecureString::new("my_secret_password");

// Access value
let value = password.expose_secret();
println!("Length: {}", value.len());

// Memory is zeroed when dropped
drop(password);  // Secure cleanup
```

### Basic RBAC Setup

```rust
use agent::common::security::{
    RBACManager,
    UserContext,
    Permission
};

// Create RBAC manager with default roles
let mut manager = RBACManager::new();
manager.initialize()?;

// Assign role to user
manager.assign_role("user123", "admin").await?;

// Create user context
let user_context = UserContext::new(
    "user123",
    vec!["admin".to_string()]
);

// Check permission
let permission = Permission::new("system:shutdown", "system", "shutdown");
let granted = manager.check_permission(&user_context, &permission).await;

if granted {
    println!("User has permission to shutdown system");
}
```

### Custom Roles

```rust
use agent::common::security::{RBACManager, Role, Permission};
use std::collections::HashSet;

let manager = RBACManager::new();

// Create custom role
let role = Role {
    id: "data_scientist".to_string(),
    name: "Data Scientist".to_string(),
    description: "Access to data analysis tools".to_string(),
    permissions: HashSet::from_iter(vec![
        "data:read".to_string(),
        "data:analyze".to_string(),
        "models:train".to_string(),
    ]),
    parent_role: Some("user".to_string()),
    priority: 30,
};

// Register role
manager.create_role(role).await?;

// Assign to user
manager.assign_role("user123", "data_scientist").await?;
```

### Dynamic Policies

```rust
use agent::common::security::{
    RBACManager,
    RBACPolicy,
    PolicyCondition,
    PolicyEffect
};

let manager = RBACManager::new();

// Time-based access policy
let policy = RBACPolicy {
    id: "business_hours".to_string(),
    name: "Business Hours Access".to_string(),
    condition: PolicyCondition::TimeRange {
        start_time: "09:00".to_string(),
        end_time: "17:00".to_string(),
    },
    effect: PolicyEffect::Allow,
    permissions: vec!["data:write".to_string()],
};

manager.add_policy(policy).await?;

// Check permission (evaluated with policies)
let granted = manager.check_permission(&user_context, &permission).await;
```

### IP-Based Access Control

```rust
use agent::common::security::{
    RBACPolicy,
    PolicyCondition,
    PolicyEffect,
    UserContext
};
use std::collections::HashMap;

// Create user context with IP
let user_context = UserContext {
    user_id: "user123".to_string(),
    roles: vec!["admin".to_string()],
    session_id: Some("session456".to_string()),
    ip_address: Some("192.168.1.100".to_string()),
    user_agent: None,
    attributes: HashMap::new(),
};

// IP-based policy
let policy = RBACPolicy {
    id: "office_network".to_string(),
    name: "Office Network Only".to_string(),
    condition: PolicyCondition::IpRange {
        allowed_ips: vec![
            "192.168.1.0/24".to_string(),
            "10.0.0.0/8".to_string(),
        ],
    },
    effect: PolicyEffect::Allow,
    permissions: vec!["admin:*".to_string()],
};

manager.add_policy(policy).await?;
```

### Complex Policy Conditions

```rust
use agent::common::security::{PolicyCondition, RBACPolicy, PolicyEffect};

// Complex: (IP match AND (senior OR engineering))
let policy = RBACPolicy {
    id: "complex_access".to_string(),
    name: "Complex Access Rule".to_string(),
    condition: PolicyCondition::And {
        conditions: vec![
            PolicyCondition::IpRange {
                allowed_ips: vec!["192.168.1.0/24".to_string()],
            },
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
    permissions: vec!["sensitive:read".to_string()],
};

manager.add_policy(policy).await?;
```

### Deny Policies

```rust
use agent::common::security::{PolicyCondition, PolicyEffect, RBACPolicy};

// Deny policy overrides role permissions
let deny_policy = RBACPolicy {
    id: "deny_shutdown".to_string(),
    name: "Deny System Shutdown".to_string(),
    condition: PolicyCondition::Always,
    effect: PolicyEffect::Deny,
    permissions: vec!["system:shutdown".to_string()],
};

manager.add_policy(deny_policy).await?;

// Even admins cannot shutdown now
let permission = Permission::new("system:shutdown", "system", "shutdown");
let granted = manager.check_permission(&admin_context, &permission).await;
assert!(!granted);  // Denied by policy
```

### Using Access Control Traits

```rust
use agent::common::security::traits::{
    AccessControl,
    NoOpAccessControl,
    DenyAllAccessControl,
    UserContext,
    Permission
};

// Allow-all for testing
let allow_all = NoOpAccessControl;
assert!(allow_all.check_permission(&user, &permission).await);

// Deny-all for lockdown
let deny_all = DenyAllAccessControl;
assert!(!deny_all.check_permission(&user, &permission).await);

// Use in production with real implementation
async fn perform_action(
    ac: &impl AccessControl,
    user: &UserContext,
    permission: &Permission
) -> Result<()> {
    if !ac.check_permission(user, permission).await {
        return Err("Permission denied");
    }
    // Perform action
    Ok(())
}
```

### Validation Trait

```rust
use agent::common::security::traits::{Validator, ValidationError};

struct EmailValidator;

impl Validator for EmailValidator {
    fn validate<T: ?Sized>(&self, value: &T, field: &str) -> Result<(), Vec<ValidationError>> {
        // Validation logic
        if !is_valid_email(value) {
            return Err(vec![ValidationError::new(field, "Invalid email")]);
        }
        Ok(())
    }
}

// Use validator
let validator = EmailValidator;
validator.validate(&"user@example.com", "email")?;
```

## API Reference

### Encryption Functions

**generate_encryption_key**
- `generate_encryption_key(size: usize) -> Result<Vec<u8>>` - Generate key

**get_or_create_key**
- `get_or_create_key(keychain, service, account, size) -> Result<Vec<u8>>` - Get or generate

**get_or_create_key_cached**
- `get_or_create_key_cached(keychain, service, account, size) -> Result<Vec<u8>>` - With caching

### KeychainProvider Methods

- `get(service, account) -> Result<String>` - Get credential
- `set(service, account, password) -> Result<()>` - Set credential
- `delete(service, account) -> Result<()>` - Delete credential

### RBACManager Methods

**Role Management**
- `create_role(role: Role) -> Result<()>` - Create custom role
- `assign_role(user_id, role_id) -> Result<()>` - Assign role to user
- `revoke_role(user_id, role_id) -> Result<()>` - Remove role from user
- `get_user_roles(user_id) -> Vec<Role>` - Get user's roles

**Permission Checking**
- `check_permission(user, permission) -> bool` - Check single permission
- `get_user_permissions(user) -> HashSet<String>` - Get all permissions

**Policy Management**
- `add_policy(policy: RBACPolicy) -> Result<()>` - Add dynamic policy

### AccessControl Trait

- `check_permission(user, permission) -> bool` - Check permission
- `has_role(user, role) -> bool` - Check role
- `get_user_permissions(user) -> Vec<Permission>` - List permissions
- `check_permissions(user, permissions) -> HashMap<String, bool>` - Batch check

## Testing

### Unit Tests

```bash
# Run all security tests
cargo test --package agent --lib common::security

# Run specific module tests
cargo test --package agent --lib common::security::encryption
cargo test --package agent --lib common::security::rbac
cargo test --package agent --lib common::security::traits
```

### Integration Tests

```bash
# Run security integration tests
cargo test --package agent --test security_integration
```

### Example Test

```rust
use agent::common::security::{RBACManager, UserContext, Permission};

#[tokio::test]
async fn test_rbac_permission_check() {
    let manager = RBACManager::new();
    manager.assign_role("user1", "admin").await.unwrap();

    let user = UserContext::new("user1", vec!["admin".to_string()]);
    let permission = Permission::new("system:read", "system", "read");

    assert!(manager.check_permission(&user, &permission).await);
}
```

## Best Practices

### Encryption

1. **Use Strong Keys**: Generate keys with at least 32 bytes (256 bits)
2. **Store in Keychain**: Never hardcode keys, always use platform keychain
3. **Rotate Keys**: Implement regular key rotation for long-lived applications
4. **Use SecureString**: Wrap sensitive strings in SecureString for automatic cleanup
5. **Cache Wisely**: Use caching for performance but consider security implications

### RBAC

1. **Principle of Least Privilege**: Assign minimal permissions needed
2. **Use Role Hierarchies**: Leverage parent roles to simplify management
3. **Implement Policies**: Use dynamic policies for time/location-based access
4. **Cache Permissions**: Enable caching for high-traffic permission checks
5. **Audit Access**: Log permission checks for security auditing
6. **Test Deny Policies**: Ensure deny policies override role permissions

### Access Control

1. **Check Early**: Verify permissions before expensive operations
2. **Use Batch Checks**: Check multiple permissions at once for efficiency
3. **Handle Denials**: Provide clear error messages for permission failures
4. **Implement Timeouts**: Set timeouts for permission checks
5. **Test Edge Cases**: Test with no-op and deny-all implementations

## Dependencies

```toml
[dependencies]
rand = "0.8"
sha2 = "0.10"
hex = "0.4"
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["sync"] }
chrono = "0.4"
regex = "1.10"
tracing = "0.1"
```

## Related Modules

- **agent/common/privacy**: Hashing and PII detection
- **agent/common/observability**: Error handling and metrics
- **agent/common/validation**: Input validation
- **agent/storage**: Encrypted storage

## Roadmap

- [ ] Add OAuth 2.0 integration for RBAC
- [ ] Implement attribute-based access control (ABAC)
- [ ] Add hardware security module (HSM) support
- [ ] Support for external policy engines (OPA)
- [ ] Add biometric authentication support

## References

- [NIST Cryptographic Standards](https://csrc.nist.gov/projects/cryptographic-standards-and-guidelines)
- [OWASP Access Control Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Access_Control_Cheat_Sheet.html)
- [RBAC on Wikipedia](https://en.wikipedia.org/wiki/Role-based_access_control)

## License

See the root LICENSE file for licensing information.
