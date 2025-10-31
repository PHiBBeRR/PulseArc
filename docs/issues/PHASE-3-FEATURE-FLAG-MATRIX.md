# Phase 3 Feature Flag Compile Matrix

**Created:** November 1, 2025
**Status:** 🟢 READY FOR CI IMPLEMENTATION
**Purpose:** Ensure all feature combinations compile independently before Phase 3 PRs merge

---

## Feature Flags in Infra Crate

From [crates/infra/Cargo.toml](../../crates/infra/Cargo.toml):

```toml
[features]
default = []
calendar = []
sap = []
tree-classifier = []
ml = ["tree-classifier"]
graphql = []
```

---

## Compile Matrix (6 flags = 64 combinations)

### Critical Combinations (Must Test)

| # | Features | Purpose | Priority |
|---|----------|---------|----------|
| 1 | `[]` (default) | Minimal build, no optional features | 🔴 CRITICAL |
| 2 | `calendar` | Calendar integration only | 🔴 CRITICAL |
| 3 | `sap` | SAP integration only | 🔴 CRITICAL |
| 4 | `tree-classifier` | Tree classifier only | 🟡 HIGH |
| 5 | `ml` | ML features (includes tree-classifier) | 🟡 HIGH |
| 6 | `graphql` | GraphQL client only | 🟡 HIGH |
| 7 | `calendar,sap` | Both integrations | 🔴 CRITICAL |
| 8 | `calendar,sap,ml` | Full enterprise build | 🔴 CRITICAL |
| 9 | `sap,ml,graphql` | SAP + ML + GraphQL | 🟡 HIGH |
| 10 | `calendar,sap,ml,graphql` | All features enabled | 🔴 CRITICAL |

**Total critical combinations:** 6
**Total high-priority combinations:** 4
**Total recommended test coverage:** 10 combinations

---

## Feature Dependencies

### Linear Dependencies
```
ml → tree-classifier
```

### Independent Features
- `calendar` (no dependencies)
- `sap` (no dependencies)
- `graphql` (no dependencies)

### Implied by Default
- None (default = `[]` is minimal)

---

## CI Test Commands

### Minimal Build (Default)
```bash
cargo check -p pulsearc-infra
cargo test -p pulsearc-infra
cargo clippy -p pulsearc-infra -- -D warnings
```

### Single Feature Tests
```bash
# Calendar
cargo check -p pulsearc-infra --features calendar
cargo test -p pulsearc-infra --features calendar

# SAP
cargo check -p pulsearc-infra --features sap
cargo test -p pulsearc-infra --features sap

# ML (includes tree-classifier)
cargo check -p pulsearc-infra --features ml
cargo test -p pulsearc-infra --features ml

# GraphQL
cargo check -p pulsearc-infra --features graphql
cargo test -p pulsearc-infra --features graphql
```

### Common Combinations
```bash
# Both integrations
cargo check -p pulsearc-infra --features calendar,sap

# Full enterprise
cargo check -p pulsearc-infra --features calendar,sap,ml,graphql
cargo test -p pulsearc-infra --features calendar,sap,ml,graphql
```

### All Features
```bash
cargo check -p pulsearc-infra --all-features
cargo test -p pulsearc-infra --all-features
cargo clippy -p pulsearc-infra --all-features -- -D warnings
```

---

## Automated Testing with xtask

### Recommended: Add Feature Matrix Testing

Create `xtask/src/features.rs`:

```rust
use anyhow::Result;
use std::process::Command;

const FEATURE_COMBINATIONS: &[&[&str]] = &[
    // Critical
    &[],                                    // default
    &["calendar"],
    &["sap"],
    &["calendar", "sap"],
    &["ml"],
    &["calendar", "sap", "ml", "graphql"], // all features

    // High priority
    &["tree-classifier"],
    &["graphql"],
    &["sap", "ml", "graphql"],
    &["calendar", "sap", "ml"],
];

pub fn test_feature_matrix() -> Result<()> {
    println!("Testing {} feature combinations...", FEATURE_COMBINATIONS.len());

    for (i, features) in FEATURE_COMBINATIONS.iter().enumerate() {
        let feature_str = if features.is_empty() {
            "default".to_string()
        } else {
            features.join(",")
        };

        println!("\n[{}/{}] Testing features: {}", i + 1, FEATURE_COMBINATIONS.len(), feature_str);

        // Check compilation
        let mut cmd = Command::new("cargo");
        cmd.arg("check").arg("-p").arg("pulsearc-infra");

        if !features.is_empty() {
            cmd.arg("--features").arg(features.join(","));
        }

        let status = cmd.status()?;
        if !status.success() {
            anyhow::bail!("Feature combination '{}' failed to compile", feature_str);
        }

        println!("✅ Features '{}' compiled successfully", feature_str);
    }

    println!("\n✅ All {} feature combinations compile successfully!", FEATURE_COMBINATIONS.len());
    Ok(())
}
```

### Usage

```bash
# Test all feature combinations
cargo xtask test-features

# Or via make
make test-features
```

---

## CI Pipeline Configuration

### GitHub Actions Example

```yaml
name: Feature Flag Matrix

on: [push, pull_request]

jobs:
  feature-matrix:
    name: Test Feature Combinations
    runs-on: ubuntu-latest
    strategy:
      matrix:
        features:
          - ""                                      # default
          - "calendar"
          - "sap"
          - "calendar,sap"
          - "ml"
          - "graphql"
          - "tree-classifier"
          - "sap,ml,graphql"
          - "calendar,sap,ml"
          - "calendar,sap,ml,graphql"              # all features

    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: 1.77
          override: true

      - name: Check compilation (${{ matrix.features || 'default' }})
        run: |
          if [ -z "${{ matrix.features }}" ]; then
            cargo check -p pulsearc-infra
          else
            cargo check -p pulsearc-infra --features "${{ matrix.features }}"
          fi

      - name: Run tests (${{ matrix.features || 'default' }})
        run: |
          if [ -z "${{ matrix.features }}" ]; then
            cargo test -p pulsearc-infra
          else
            cargo test -p pulsearc-infra --features "${{ matrix.features }}"
          fi
```

---

## Local Testing Script

Create `scripts/test-features.sh`:

```bash
#!/bin/bash
set -e

FEATURES=(
    ""                                    # default
    "calendar"
    "sap"
    "calendar,sap"
    "ml"
    "graphql"
    "tree-classifier"
    "sap,ml,graphql"
    "calendar,sap,ml"
    "calendar,sap,ml,graphql"            # all features
)

echo "Testing ${#FEATURES[@]} feature combinations..."

for i in "${!FEATURES[@]}"; do
    FEATURE="${FEATURES[$i]}"
    if [ -z "$FEATURE" ]; then
        DISPLAY="default"
        CMD="cargo check -p pulsearc-infra"
    else
        DISPLAY="$FEATURE"
        CMD="cargo check -p pulsearc-infra --features $FEATURE"
    fi

    echo ""
    echo "[$((i+1))/${#FEATURES[@]}] Testing features: $DISPLAY"

    if $CMD; then
        echo "✅ Features '$DISPLAY' compiled successfully"
    else
        echo "❌ Features '$DISPLAY' failed to compile"
        exit 1
    fi
done

echo ""
echo "✅ All ${#FEATURES[@]} feature combinations compile successfully!"
```

### Usage

```bash
chmod +x scripts/test-features.sh
./scripts/test-features.sh
```

---

## Phase 3 PR Requirements

### Before Merging ANY Phase 3 PR

1. ✅ All critical feature combinations compile:
   - `[]` (default)
   - `calendar`
   - `sap`
   - `calendar,sap`
   - `ml`
   - `calendar,sap,ml,graphql` (all features)

2. ✅ Tests pass for applicable features:
   - If PR adds calendar code: `cargo test --features calendar`
   - If PR adds SAP code: `cargo test --features sap`

3. ✅ Clippy passes for applicable features:
   - If PR adds calendar code: `cargo clippy --features calendar -- -D warnings`

4. ✅ Feature-gated code properly isolated:
   - Use `#[cfg(feature = "...")]` on modules/functions
   - Use `#[cfg_attr(feature = "...", ...)]` on types

---

## Common Feature Flag Issues

### ❌ Issue 1: Missing Feature Gates

```rust
// ❌ WRONG - SAP code not gated
use crate::integrations::sap::SapClient;

pub fn do_something() {
    // Uses SAP without feature gate
}
```

```rust
// ✅ CORRECT - Properly gated
#[cfg(feature = "sap")]
use crate::integrations::sap::SapClient;

#[cfg(feature = "sap")]
pub fn do_something() {
    // Only compiles with sap feature
}
```

### ❌ Issue 2: Transitive Feature Dependencies

```rust
// ❌ WRONG - ml feature not propagated
[dependencies]
pulsearc-infra = { path = "../infra" }  // Missing features = ["ml"]
```

```rust
// ✅ CORRECT - Feature propagated
[dependencies]
pulsearc-infra = { path = "../infra", features = ["ml"] }
```

### ❌ Issue 3: Conditional Compilation in Tests

```rust
// ❌ WRONG - Test always compiles, but SapClient doesn't
#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::sap::SapClient;  // Error if 'sap' feature not enabled
}
```

```rust
// ✅ CORRECT - Test properly gated
#[cfg(all(test, feature = "sap"))]
mod tests {
    use super::*;
    use crate::integrations::sap::SapClient;  // Only compiles with sap feature
}
```

---

## Validation Checklist

Before starting Phase 3:

- [ ] CI pipeline includes feature matrix testing (10 combinations)
- [ ] Local test script available (`scripts/test-features.sh`)
- [ ] xtask command added (`cargo xtask test-features`)
- [ ] All Phase 3 contributors trained on feature flag patterns
- [ ] PR template includes feature flag checklist

During Phase 3:

- [ ] Each PR tests applicable feature combinations
- [ ] Code review verifies proper `#[cfg(feature = "...")]` usage
- [ ] Integration tests properly feature-gated
- [ ] No compilation errors in any of the 10 critical combinations

---

## Success Criteria

Phase 3 feature flag testing is successful when:

1. ✅ All 10 critical combinations compile without errors
2. ✅ Tests pass for each applicable feature
3. ✅ Clippy clean for all feature combinations
4. ✅ CI automatically tests matrix on every PR
5. ✅ No feature flag regressions introduced during Phase 3

---

## Related Documents

- [PHASE-3-INFRA-TRACKING.md](PHASE-3-INFRA-TRACKING.md) - Main migration plan
- [PHASE-3-REGRESSION-TEST-SUMMARY.md](PHASE-3-REGRESSION-TEST-SUMMARY.md) - Regression test guide

---

**Document Status:** 🟢 READY FOR CI IMPLEMENTATION
**Next Steps:** Implement CI matrix testing before Phase 3A.1
**Contact:** @infra-squad for questions
