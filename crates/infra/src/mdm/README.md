# MDM Infrastructure Module

Enterprise policy management with remote configuration support.

## Overview

The MDM (Mobile Device Management) module provides:
- Policy enforcement and compliance checking
- Remote configuration fetching over HTTPS
- Certificate-based security (CA certificates, mutual TLS)
- Configuration validation and merging
- Feature-gated compliance auditing

## Architecture

```
┌───────────────────────────────────────────────────────────┐
│                      MDM Infrastructure                   │
├───────────────────────────────────────────────────────────┤
│                                                           │
│  ┌──────────────┐         ┌──────────────┐                │
│  │  MdmConfig   │◄────────│  MdmClient   │                │
│  │              │         │              │                │
│  │ - policies   │         │ - reqwest    │                │
│  │ - compliance │         │ - TLS/certs  │                │
│  │ - validation │         │ - HTTPS      │                │
│  └──────────────┘         └──────────────┘                │
│         │                        │                        │
│         ▼                        ▼                        │
│  ┌──────────────┐         ┌──────────────┐                │
│  │  Local       │         │  Remote      │                │
│  │  Config      │◄────────│  Config      │                │
│  │              │  merge  │  Server      │                │
│  └──────────────┘         └──────────────┘                │
│                                  │                        │
│                                  ▼                        │
│                           ┌──────────────┐                │
│                           │  SSL/TLS     │                │
│                           │  Certificates│                │
│                           └──────────────┘                │
│                                                           │
└───────────────────────────────────────────────────────────┘
```

## Features

### Core Features (Always Available)

- **Policy Management** - Define and enforce policies
- **Configuration Validation** - Validate URLs, rules, and policy values
- **Local Configuration** - Fluent builder pattern for config creation
- **Remote Fetching** - HTTPS-based config retrieval
- **Configuration Merging** - Merge remote config with local overrides

### Feature-Gated (`audit-compliance`)

- **Compliance Checking** - Runtime validation of compliance rules
- **Compliance Context** - Field and metadata tracking
- **Compliance Reports** - Detailed failure reporting with severity levels

## Quick Start

### 1. Basic Configuration

```rust
use pulsearc_infra::mdm::{MdmConfig, PolicySetting, PolicyValue};

let config = MdmConfig::builder()
    .policy_enforcement(true)
    .remote_config_url("https://mdm.example.com/config")
    .update_interval_secs(3600)
    .add_policy(
        "max_idle_time",
        PolicySetting::new(PolicyValue::Number(900.0))
    )
    .build()?;

// Validate configuration
config.validate()?;

// Check policy
if config.is_policy_enabled("max_idle_time") {
    let value = config.get_policy_value("max_idle_time");
    println!("Max idle time: {:?}", value);
}
```

### 2. Remote Configuration Fetching

#### Production (with CA certificate)

```rust
use pulsearc_infra::mdm::MdmClient;

// Load your organization's CA certificate
let client = MdmClient::with_ca_cert(
    "https://mdm.yourcompany.com/config",
    "/etc/ssl/certs/company-ca.pem"
)?;

// Fetch configuration
let config = client.fetch_config().await?;
println!("Fetched config with {} policies", config.policies.len());
```

#### Testing (with self-signed certificates)

```rust
use pulsearc_infra::mdm::MdmClient;

// Use test certificates from scripts/mdm/generate-test-certs.sh
let ca_cert = std::env::var("MDM_CA_CERT").unwrap();

let client = MdmClient::with_ca_cert(
    "https://localhost:8080/mdm/config",
    &ca_cert
)?;

let config = client.fetch_config().await?;
```

### 3. Compliance Checking (Feature: `audit-compliance`)

```rust
#[cfg(feature = "audit-compliance")]
{
    use pulsearc_infra::mdm::{ComplianceContext, ComplianceRule, ValidationType};

    // Add compliance rules
    let config = MdmConfig::builder()
        .add_compliance_check(ComplianceRule::new(
            "encryption_required",
            ValidationType::FieldExists("encryption".to_string())
        ))
        .build()?;

    // Create compliance context
    let context = ComplianceContext::new()
        .with_field("encryption", "enabled")
        .with_field("version", "2.0");

    // Check compliance
    let report = config.check_compliance(&context)?;

    if !report.is_compliant() {
        eprintln!("Compliance check failed!");
        eprintln!("  Critical failures: {}", report.critical_failures);
        eprintln!("  Warnings: {}", report.warnings);
    }
}
```

### 4. Merging Remote Configuration

```rust
use pulsearc_infra::mdm::MdmClient;

// Start with local config
let mut local_config = MdmConfig::builder()
    .allow_local_override(true)  // Allow merging
    .build()?;

// Fetch and merge remote config
let client = MdmClient::new("https://mdm.example.com/config")?;
let merged_config = client.fetch_and_merge(local_config).await?;

println!("Merged configuration with remote policies");
```

## Certificates Setup

MDM requires SSL/TLS certificates for secure HTTPS communication.

### For Testing (Self-Signed Certificates)

```bash
# 1. Generate test certificates
cd scripts/mdm
./generate-test-certs.sh

# 2. Export environment variables
export MDM_CA_CERT=$(pwd)/../../.mdm-certs/ca-cert.pem
export MDM_SERVER_CERT=$(pwd)/../../.mdm-certs/server-cert.pem
export MDM_SERVER_KEY=$(pwd)/../../.mdm-certs/server-key.pem

# 3. Use in code
let client = MdmClient::with_ca_cert(
    "https://localhost:8080/config",
    &std::env::var("MDM_CA_CERT")?
)?;
```

See [`scripts/mdm/README.md`](../../../../scripts/mdm/README.md) for detailed certificate documentation.

### For Production

Use proper CA-signed certificates from:
- **Let's Encrypt** (free, automated)
- **DigiCert, GlobalSign, etc.** (commercial)
- **Your organization's PKI** (enterprise)

Required for:
- 🔴 Production deployments
- 🔴 Apple Push Notification Service (APNs)
- 🔴 Public-facing MDM servers
- 🔴 Compliance requirements (SOC2, HIPAA, etc.)

## CI/CD Integration

### Self-Hosted Runner Setup

```bash
# One-time setup on your CI runner
cd /path/to/PulseArc/scripts/mdm
./generate-test-certs.sh

# Add to runner environment
echo 'export MDM_CA_CERT=/path/to/PulseArc/.mdm-certs/ca-cert.pem' >> ~/.bashrc
echo 'export MDM_SERVER_CERT=/path/to/PulseArc/.mdm-certs/server-cert.pem' >> ~/.bashrc
echo 'export MDM_SERVER_KEY=/path/to/PulseArc/.mdm-certs/server-key.pem' >> ~/.bashrc
```

### GitHub Actions

```yaml
- name: Generate MDM test certificates
  run: |
    cd scripts/mdm
    ./generate-test-certs.sh

- name: Run MDM integration tests
  run: cargo test --features audit-compliance
  env:
    MDM_CA_CERT: ${{ github.workspace }}/.mdm-certs/ca-cert.pem
    MDM_SERVER_CERT: ${{ github.workspace }}/.mdm-certs/server-cert.pem
    MDM_SERVER_KEY: ${{ github.workspace }}/.mdm-certs/server-key.pem
```

## API Reference

### `MdmConfig`

Main configuration structure with builder pattern.

**Methods:**
- `new()` - Create default config
- `builder()` - Get fluent builder
- `validate()` - Validate configuration
- `is_policy_enabled(name)` - Check if policy is enabled
- `get_policy_value(name)` - Get policy value
- `check_compliance(context)` - Run compliance checks (requires `audit-compliance` feature)
- `merge_remote(remote)` - Merge with remote config

### `MdmClient`

HTTP client for fetching remote configuration.

**Constructors:**
- `new(url)` - Create with default TLS validation
- `with_ca_cert(url, ca_path)` - Create with custom CA certificate
- `with_insecure_tls(url)` - Create for testing (disables validation) - **`#[cfg(test)]` only**

**Methods:**
- `with_timeout(duration)` - Set custom timeout
- `fetch_config()` - Fetch configuration from remote server
- `fetch_and_merge(local)` - Fetch and merge with local config

### `ComplianceRule` (Feature: `audit-compliance`)

Individual compliance validation rule.

**Constructors:**
- `new(name, validation_type)` - Create new rule

**Methods:**
- `validate()` - Validate rule structure
- `check(context)` - Execute compliance check

### `ComplianceContext` (Feature: `audit-compliance`)

Context for compliance checking with fields and metadata.

**Methods:**
- `new()` - Create empty context
- `with_field(key, value)` - Add field
- `with_metadata(key, value)` - Add metadata
- `has_field(field)` - Check if field exists
- `get_field(field)` - Get field value

## Examples

Run the provided examples:

```bash
# Basic MDM configuration
cargo run --example mdm_remote_config --features audit-compliance

# Set up certificates first
cd scripts/mdm && ./generate-test-certs.sh
export MDM_CA_CERT=$(pwd)/../../.mdm-certs/ca-cert.pem

# Then run the example
cargo run --example mdm_remote_config
```

## Testing

```bash
# Run all tests
cargo test -p pulsearc-infra mdm

# Run with compliance feature
cargo test -p pulsearc-infra mdm --features audit-compliance

# Run specific test
cargo test -p pulsearc-infra mdm::tests::test_mdm_config_builder
```

## Troubleshooting

### Certificate Errors

**Problem:** "certificate verify failed" or similar errors

**Solutions:**
1. Make sure CA certificate path is correct
2. Verify certificate hasn't expired: `openssl x509 -in ca-cert.pem -noout -dates`
3. For testing, regenerate certificates: `cd scripts/mdm && ./generate-test-certs.sh`
4. Check that server certificate matches the hostname

### Network Errors

**Problem:** Connection timeout or refused

**Solutions:**
1. Verify MDM server is running
2. Check firewall rules allow HTTPS (port 443 or custom)
3. Increase timeout: `client.with_timeout(Duration::from_secs(60))`
4. Test with curl: `curl --cacert ca-cert.pem https://localhost:8080/config`

### Configuration Validation Errors

**Problem:** `MdmError::ValidationError`

**Solutions:**
1. Check that all required fields are present
2. Verify URL format: must be valid HTTPS URL
3. Ensure policy values are valid (no empty strings, no NaN numbers)
4. Run validation explicitly: `config.validate()?`

## Security Considerations

### ⚠️ Do NOT

- ❌ Use `with_insecure_tls()` in production (it's `#[cfg(test)]` only anyway)
- ❌ Commit certificates or private keys to git (already in `.gitignore`)
- ❌ Use self-signed certificates in production
- ❌ Disable certificate validation for real deployments
- ❌ Store encryption keys in code or config files

### ✅ Do

- ✅ Use proper CA-signed certificates for production
- ✅ Rotate certificates regularly (annually minimum)
- ✅ Store private keys with restrictive permissions (`chmod 600`)
- ✅ Use environment variables for certificate paths
- ✅ Validate all configuration before using
- ✅ Enable compliance checking in production (`audit-compliance` feature)

## Related Documentation

- [MDM Extraction Guide](../../../../docs/issues/MDM_EXTRACTION_GUIDE.md) - Original migration documentation
- [Certificate Generation Script](../../../../scripts/mdm/README.md) - SSL/TLS certificate setup
- [Phase 3 Infra Tracking](../../../../docs/issues/PHASE-3-INFRA-TRACKING.md) - Migration tracking

## License

See root `LICENSE` file for details.
