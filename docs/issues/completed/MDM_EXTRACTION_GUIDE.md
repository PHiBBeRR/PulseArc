# MDM Extraction Guide

This document identifies all files necessary to extract the MDM (Mobile Device Management) functionality from the PulseArc platform.

## Overview

MDM is a self-contained enterprise policy management system located in `legacy/api/mdm.rs`. It provides policy enforcement, compliance checking, and remote configuration management.

## Files Required for Extraction

### ✅ Core MDM Module

**Location:** `legacy/api/src/mdm.rs` (≈ 950 LOC, including unit tests)

**Contains:**
- `MdmConfig` – top-level configuration object
- `MdmConfigBuilder` – fluent builder for `MdmConfig`
- `ComplianceRule`, `ComplianceResult`, `ComplianceReport`
- `PolicySetting` / `PolicyValue` types
- `MdmError`, `MdmResult`
- Optional compliance API (`ComplianceContext`, etc.) behind the `audit-compliance` feature flag
- Inline unit tests (no separate test module)

### 🔗 Current Integration

- `legacy/api/src/lib.rs` does **not** currently re-export the MDM module.
- No other legacy modules reference `MdmConfig`; the code is dormant aside from its unit tests.
- Extraction can therefore be treated as a pure code move with no call-site updates required. If you decide to expose the module again from legacy, add:

```rust
// legacy/api/src/lib.rs
pub mod mdm;
```

and re-export the specific types you want to keep public:

```rust
pub use mdm::{MdmConfig, MdmConfigBuilder, ComplianceRule, ComplianceReport, PolicySetting};
```

(This re-export is optional and only needed if downstream code outside the legacy crate should keep using the module after extraction.)

## Dependencies

### External Crates (Required)

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
url = "2.5"

[dev-dependencies]
serde_json = "1.0"  # For testing
```

### Internal Dependencies

**None!** MDM is completely self-contained with:
- ✅ No dependencies on other legacy modules
- ✅ No dependencies on common utilities
- ✅ Standard library only (HashMap, fmt, etc.)
- ✅ Only uses serde and url crates

## Extraction Options

### Option 1: Create a New Crate in `crates/`

**What to Extract:**
1. Copy `legacy/api/src/mdm.rs` → `crates/mdm/src/lib.rs`
2. Create `crates/mdm/Cargo.toml` with the minimal dependencies listed above
3. Move/rename the inline tests as desired (e.g., keep them in the same file or split into `tests/`)
4. Add a README explaining the API and feature flags

**Result:** A clean, reusable library crate that can be consumed by both the new infra layer and any future services.

### Option 2: Fold into `crates/infra`

**What to Do:**
1. Create a new module tree, e.g. `crates/infra/src/mdm/mod.rs`
2. Copy the contents of `legacy/api/src/mdm.rs` into that module
3. Export it from `crates/infra/src/lib.rs` (e.g., `pub mod mdm;`)
4. Update `crates/infra/Cargo.toml` to include the `audit-compliance` feature flag (if still needed)
5. Remove the legacy copy once Phase 4 rewiring is complete

**Result:** MDM lives alongside the rest of the new infrastructure adapters and participates in the existing feature flag matrix.

### Option 3: Keep a Legacy Copy but Publish a Stub

**What to Do:**
1. Leave `legacy/api/src/mdm.rs` untouched (for archival purposes)
2. Extract only the types you need into `crates/mdm` or `crates/infra/src/mdm`
3. Add `pub use` statements in legacy to point to the new implementation if the legacy Tauri app still needs it

**Result:** Lets the new codebase evolve independently while keeping legacy compilable until Phase 4 deletes the old tree.

## What Makes MDM Extraction-Friendly

### ✅ Clean Architecture
- **Self-contained**: No dependencies on other legacy modules
- **Well-tested**: 20+ unit tests covering all functionality
- **Builder pattern**: Easy to use API
- **Serde integration**: JSON serialization out of the box

### ✅ Clear Boundaries
- **Error type**: Dedicated `MdmError` enum
- **Result type**: `MdmResult<T>` alias
- **Feature gating**: Compliance checks behind `audit-compliance` feature
- **No global state**: All functions are methods on structs

### ✅ Enterprise-Ready Features
- Remote configuration fetching
- Policy enforcement
- Compliance checking with severity levels
- Merge strategy for local/remote configs
- Validation at every step

## Usage After Extraction

### Basic Usage

```rust
use mdm::{MdmConfig, PolicySetting, PolicyValue, ComplianceRule, ValidationType};

// Create MDM configuration
let config = MdmConfig::builder()
    .policy_enforcement(true)
    .remote_config_url("https://config.example.com/mdm")
    .update_interval_secs(3600)
    .add_compliance_check(ComplianceRule::new(
        "encryption_required",
        ValidationType::FieldExists("encryption".to_string()),
    ))
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

### Compliance Checking

```rust
#[cfg(feature = "audit-compliance")]
{
    use mdm::ComplianceContext;

    let context = ComplianceContext::new()
        .with_field("encryption", "enabled")
        .with_field("version", "2.0");

    let report = config.check_compliance(&context)?;

    if !report.is_compliant() {
        eprintln!("Compliance check failed:");
        eprintln!("  Critical failures: {}", report.critical_failures);
        eprintln!("  Warnings: {}", report.warnings);
    }
}
```

## Migration Steps

### Step 1: Prepare the Destination

```bash
# Option 1: new crate
mkdir -p crates/mdm/src

# Option 2: module inside infra
mkdir -p crates/infra/src/mdm
```

### Step 2: Copy the Source

```bash
cp legacy/api/src/mdm.rs crates/mdm/src/lib.rs           # Option 1
# or
cp legacy/api/src/mdm.rs crates/infra/src/mdm/mod.rs     # Option 2
```

### Step 3: Wire Up the Build

- **If using a new crate (`crates/mdm`):**
  - Add the crate to the workspace member list in the root `Cargo.toml`
  - Create `crates/mdm/Cargo.toml` with the dependency snippet shown above

- **If embedding in infra:**
  - Add `pub mod mdm;` to `crates/infra/src/lib.rs`
  - Add `"audit-compliance"` to `crates/infra/Cargo.toml` under `[features]` if you plan to keep the flag

### Step 4: Update Call Sites (Only If Needed)

Search for `mdm::` or `MdmConfig` references. For new code under `crates/`, update imports to point at the new crate/module, e.g.

```rust
use pulsearc_mdm::MdmConfig;        // if you name the new crate `pulsearc-mdm`
// or
use pulsearc_infra::mdm::MdmConfig; // if tucked into infra
```

(`legacy/api` currently has no call sites, so this step is optional unless you add new integrations.)

### Step 5: Run Tests

```bash
cargo test -p pulsearc-mdm              # new crate
# or
cargo test -p pulsearc-infra mdm::tests # module inside infra
```

### Step 6: Sunset the Legacy Copy (Optional)

Once Phase 4 rewiring removes the legacy dependency, delete `legacy/api/src/mdm.rs` and drop any `pub mod mdm;` re-exports you added earlier.

## Files Summary

| File | Lines | Status | Action |
|------|-------|--------|--------|
| `legacy/api/src/mdm.rs` | 950 | ✅ Required | Copy to new location |
| `legacy/api/src/lib.rs` | 200+ | ⚠️ Update | Add/remove `pub mod mdm;` as needed |
| (none) | - | 🔗 Integration | No active call sites – optional |
| _Inline tests_ | - | ✅ Tests | Already inside `mdm.rs`; move if desired |

**Total Core Code:** ~978 lines (self-contained, production-ready)

## Recommendations

### For Standalone Library (Recommended)

**Pros:**
- ✅ Fully independent and reusable
- ✅ Can be published to crates.io
- ✅ Versioned separately
- ✅ Easy to maintain
- ✅ No legacy dependencies

**Cons:**
- ❌ Need to create new project structure
- ❌ Need separate CI/CD if desired

**Best for:** Reusing MDM in other projects, open-sourcing

### For `crates/infra`

**Pros:**
- ✅ Available to all new infrastructure adapters
- ✅ Shares existing feature-flag and CI matrix
- ✅ Easier to wire into current Phase 3 work
- ✅ Shared testing infrastructure

**Cons:**
- ❌ Still tied to the infra crate release cadence
- ❌ Harder to reuse outside the workspace without depending on infra

**Best for:** Keeping MDM within PulseArc while migrating away from the legacy tree

## Feature Flags

MDM uses one feature flag:

```toml
[features]
audit-compliance = []  # Enables ComplianceContext and check_compliance()
```

**Used for:**
- `ComplianceContext` struct
- `check_compliance()` method
- Compliance checking tests

**Without this feature:** Basic policy management still works, just no runtime compliance checking.

## Next Steps

1. **Decide on extraction location** (new crate, infra module, or hybrid)
2. **Create destination structure** (directories, Cargo.toml)
3. **Copy mdm.rs** to new location
4. **Update imports** in existing code
5. **Run tests** to verify everything works
6. **Update documentation** to reflect new location
7. **Consider adding README** with usage examples

## SSL/TLS Certificates for MDM Testing

### Certificate Requirements

MDM remote configuration fetching requires HTTPS, which needs SSL/TLS certificates.

**For Testing (Recommended):**
- ✅ Use **self-signed certificates** for local/CI testing
- ✅ Quick setup with included script
- ✅ No CA approval required
- ✅ Free and automated

**For Production:**
- 🔴 Obtain **proper CA-signed certificates** (Let's Encrypt, DigiCert, etc.)
- 🔴 Required for Apple Push Notification Service (APNs)
- 🔴 Needed for public-facing MDM servers
- 🔴 Essential for compliance requirements

### Generating Test Certificates

We've included a script to generate self-signed certificates for testing:

```bash
cd scripts/mdm
./generate-test-certs.sh
```

**Generated files** (in `.mdm-certs/`):
- `ca-cert.pem` - Root CA certificate (trust this in your system)
- `server-cert.pem` / `server-key.pem` - Server certificate and key
- `client-cert.pem` / `client-key.pem` - Client certificate for mutual TLS
- `*.p12` - PKCS#12 bundles for keychain import

See [`scripts/mdm/README.md`](../../scripts/mdm/README.md) for detailed usage instructions.

### Using Certificates in MDM Code

#### Basic HTTPS Client (Testing)

```rust
use reqwest;

let client = reqwest::Client::builder()
    .danger_accept_invalid_certs(true)  // ⚠️ TEST ONLY!
    .build()?;

// Fetch MDM config
let config = client
    .get("https://localhost:8080/mdm/config")
    .send()
    .await?
    .json::<MdmConfig>()
    .await?;
```

#### Production-Like Setup

```rust
use reqwest;
use std::fs;

// Load CA certificate
let ca_cert = fs::read(".mdm-certs/ca-cert.pem")?;
let ca_cert = reqwest::Certificate::from_pem(&ca_cert)?;

let client = reqwest::Client::builder()
    .add_root_certificate(ca_cert)
    .build()?;

let config = client
    .get("https://mdm.yourcompany.com/config")
    .send()
    .await?
    .json::<MdmConfig>()
    .await?;
```

### CI/CD Integration

The certificates are gitignored and need to be generated in CI:

```yaml
# .github/workflows/mdm-tests.yml
- name: Generate MDM test certificates
  run: |
    cd scripts/mdm
    ./generate-test-certs.sh

- name: Run MDM integration tests
  run: cargo test --features mdm-integration
  env:
    MDM_CA_CERT: ${{ github.workspace }}/.mdm-certs/ca-cert.pem
    MDM_SERVER_CERT: ${{ github.workspace }}/.mdm-certs/server-cert.pem
    MDM_SERVER_KEY: ${{ github.workspace }}/.mdm-certs/server-key.pem
```

### Self-Hosted Runner Setup

For your **self-hosted macOS runner**:

```bash
# One-time setup on the runner machine
cd /path/to/PulseArc/scripts/mdm
./generate-test-certs.sh

# Export environment variables (add to runner config)
export MDM_CA_CERT=/path/to/PulseArc/.mdm-certs/ca-cert.pem
export MDM_SERVER_CERT=/path/to/PulseArc/.mdm-certs/server-cert.pem
export MDM_SERVER_KEY=/path/to/PulseArc/.mdm-certs/server-key.pem
```

The certificates are valid for 365 days by default. Regenerate annually or as needed.

---

## Questions?

- **Is MDM used elsewhere?** Only in `validate_config.rs` as optional validation
- **Does it depend on other modules?** No, completely self-contained
- **Can it be extracted safely?** Yes, very clean boundaries
- **Will it break anything?** Only if you remove integration in `validate_config.rs`
- **Is it production-ready?** Yes, fully tested with 20+ unit tests
- **Do I need certificates for MDM?** Only if testing remote config fetching over HTTPS
- **Can I use self-signed certs in production?** No - get proper CA-signed certificates

---

**Estimated Extraction Time:** 30-60 minutes for standalone crate, 15-30 minutes for common module

**Risk Level:** ⬜ Low - self-contained with clear boundaries

**Testing Required:** ⬜ Minimal - existing tests should pass without changes

**Certificate Setup:** ⬜ 5 minutes with included script
