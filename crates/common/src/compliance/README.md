# PulseArc Compliance Module

`crates/common/src/compliance` contains the compliance primitives that power
PulseArc's enterprise features. The module lives inside the `pulsearc-common`
crate and is gated behind the `platform` feature tier (which pulls in the
`runtime` tier). It provides three building blocks:

- **Audit logging** – capture and route security-relevant events.
- **Configuration management** – manage remote configuration with local
  overrides and version safety.
- **Feature flags** – gate capability rollouts with deterministic targeting.

All components are designed to operate in async contexts, integrate with the
security stack, and avoid unsafe code.

## Location & Build Requirements
- Path: `crates/common/src/compliance`
- Crate: `pulsearc-common`
- Minimum feature set: enable the `platform` feature when compiling or testing
  (`cargo test -p pulsearc-common --features "platform"`).
- Tests and examples use `tokio`, `reqwest`, `serde`, `chrono`, and `tracing`
  through the workspace dependencies brought in by the feature tiers.
- Optional streaming uses the `AUDIT_WEBHOOK_URL` environment variable when a
  webhook URL is not supplied directly in configuration.

## Directory Map
- `mod.rs` – public surface and re-exports.
- `audit.rs` – event taxonomy, logger implementation, streaming, and metrics.
- `config.rs` – remote configuration fetcher plus local overrides.
- `feature_flags.rs` – deterministic rollout engine with default flags.
- `README.md` – this document.

## Quick Start
```rust
use pulsearc_common::compliance::audit::{
    AuditConfig, AuditContext, AuditEvent, AuditSeverity,
};
use pulsearc_common::compliance::{ConfigManager, FeatureFlagManager, GlobalAuditLogger};
use pulsearc_common::security::rbac::UserContext;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = GlobalAuditLogger::new();

    logger
        .configure(AuditConfig {
            log_file_path: Some("logs/audit.log".into()),
            min_severity: AuditSeverity::Warning,
            enable_streaming: true,
            streaming_url: Some("https://internal.example.com/audit-webhook".into()),
            ..Default::default()
        })
        .await;
    logger.initialize_with_path().await?;

    let user_ctx = UserContext {
        user_id: "user-42".into(),
        roles: vec!["admin".into()],
        session_id: Some("session-123".into()),
        ip_address: Some("10.0.0.42".into()),
        user_agent: Some("tauri-client".into()),
        attributes: Default::default(),
    };

    logger
        .log_event(
            AuditEvent::UnauthorizedAccess {
                resource: "billing".into(),
                user_id: Some(user_ctx.user_id.clone()),
            },
            AuditContext {
                user_id: Some(user_ctx.user_id.clone()),
                session_id: user_ctx.session_id.clone(),
                ip_address: user_ctx.ip_address.clone(),
                user_agent: user_ctx.user_agent.clone(),
            },
            AuditSeverity::Security,
        )
        .await;

    let mut config = ConfigManager::new();
    config.set_override("feature.mode".into(), json!("canary"));

    let flags = FeatureFlagManager::new();
    if flags.is_enabled("enterprise_menu", Some(&user_ctx)).await {
        // Activate gated UI behaviour here.
    }

    Ok(())
}
```

---

## Audit Logging (`audit.rs`)

### Capabilities
- Ring-buffer backed store (`VecDeque`) with configurable retention.
- File persistence and optional webhook streaming (`reqwest` + async tasks).
- Severity gating (`AuditSeverity`) to drop low-value events.
- In-memory querying and aggregate statistics.
- Thread-safe, cloneable `GlobalAuditLogger` backed by `Arc<RwLock<...>>`.

### Event Taxonomy
`AuditEvent` is a tagged enum grouped by use case:
- **Menu events**: `MenuItemClicked`, `MenuStateChanged`
- **Permissions**: `PermissionCheck`, `RoleAssigned`
- **Configuration**: `ConfigurationChanged`, `RemoteConfigSync`
- **Feature flags**: `FeatureFlagToggled`
- **Security**: `UnauthorizedAccess`, `SuspiciousActivity`, `ComplianceViolation`
- **Data access**: `DataAccessed`, `DataModified`
- **System lifecycle**: `ApplicationStarted`, `ApplicationStopped`, `ErrorOccurred`
- **Custom**: arbitrary JSON payloads under `Custom`

`AuditEvent::get_type()` returns a stable identifier for downstream consumers.

### Configuration
`AuditConfig` fields (set via `GlobalAuditLogger::configure`):
- `max_memory_entries` – in-memory retention (default 10,000).
- `log_file_path` – optional append-only JSONL file path (created lazily).
- `enable_streaming` / `streaming_url` / `streaming_timeout_secs` – controls webhook dispatch.
- `min_severity` – drop events below the configured level.
- `encrypt_sensitive` – reserved for downstream integrations.

Call `initialize_with_path()` after configuration to ensure directories exist
before logging begins.

### Logging Workflow
1. Construct context with `AuditContext` helpers (`new`, `empty`, `with_component`,
   etc.).
2. Create an event variant.
3. Call `log_event(event, context, severity).await`.
4. Optional: query or export later with `query`, `get_statistics`, or `export`.

`log_event` keeps the buffer within `max_memory_entries`, appends to any file,
and schedules asynchronous webhook delivery. Failures are reported through
`tracing` (`warn!`/`error!`) but never panic.

### Querying & Metrics
- `query(fn filter, Option<usize>)` – in-process filtering with optional limit.
- `export(PathBuf)` – writes pretty-printed JSON to disk.
- `clear(reason, authorized_by)` – wipe the in-memory buffer.
- `clear_with_external_audit(reason, authorized_by)` – append a JSON entry to
  the configured log file before clearing.
- `get_statistics()` – aggregate counts by severity and event type plus oldest /
  newest timestamps.

### Streaming
When `enable_streaming` is true, events are posted asynchronously to:
- `AuditConfig.streaming_url`, if present.
- Else `AUDIT_WEBHOOK_URL` environment variable, if provided.

The webhook call happens in a detached `tokio::spawn`, so the caller is not
blocked during HTTP I/O. Handle secrets via configuration, never by logging
sensitive payloads.

---

## Configuration Management (`config.rs`)

### Core Types
- `RemoteConfig` – versioned document with environment label, settings map, and
  sync metadata.
- `ConfigManager` – owns a `RemoteConfig` plus local overrides layered on top.

### Version Safety
`ConfigManager::sync_from_remote` performs a major-version compatibility check
(`1.x` only accepts `1.*`). Rejecting incompatible versions prevents silently
loading breaking changes. You can override `RemoteConfig::version` in tests to
exercise boundary conditions.

### Data Sources
1. **Remote sync**: `sync_from_remote(url).await` fetches JSON from an HTTP
   endpoint (requires an async runtime and `reqwest`). Successful sync stamps
   `last_sync` and stores the `sync_url`.
2. **Local files**: `load_from_file(path)` reads JSON from disk. Useful for
   bootstrapping or offline operation.
3. **Overrides**: `set_override(key, value)` stores a `serde_json::Value`
   override, which always wins over remote settings.

`get_all_settings()` returns the merged map (remote config + overrides), while
`get(key)` automatically resolves overrides first.

### Example
```rust
let mut manager = ConfigManager::new();

// Load a base file bundled with the app.
manager.load_from_file("config/base.json")?;

// Point at staging and override a sensitive toggle locally.
if std::env::var("PULSEARC_ENV").as_deref() == Ok("staging") {
    manager.sync_from_remote("https://config.example.com/staging.json").await?;
}
manager.set_override("feature.experimental", serde_json::json!(true));

let rollout: bool = manager
    .get("feature.experimental")
    .and_then(|v| v.as_bool())
    .unwrap_or(false);
```

Clear overrides with `clear_overrides()` when the local layer should be reset.

---

## Feature Flags (`feature_flags.rs`)

### Model
- `FeatureFlag` – id, name, description, global enabled bit, rollout percentage,
  targeted roles/users, and arbitrary string metadata.
- `FeatureFlagManager` – manages the registry. `new()` seeds defaults for
  `"enterprise_menu"` (enabled for admins/power users) and
  `"advanced_telemetry"` (disabled canary).

### Evaluation Order
`is_enabled(flag_id, Option<&UserContext>)` checks:
1. Global `enabled` bit — disabled flags short-circuit to `false`.
2. Targeted users (`target_users`) via exact match.
3. Targeted roles (`target_roles`) against `UserContext.roles`.
4. Rollout percentage – deterministic FNV-1a hash on `user_id` + `flag_id`
   produces a stable bucket in `[0, 10000)`. With no `UserContext`, only `0%`
   / `100%` can pass.

Use `toggle_flag`, `add_flag`, and `set_rollout_percentage` during runtime or
administrative flows. All mutations log via `tracing::info`.

### Example
```rust
use pulsearc_common::compliance::feature_flags::{FeatureFlag, FeatureFlagManager};
use pulsearc_common::security::rbac::UserContext;

let mut manager = FeatureFlagManager::new();

manager.add_flag(FeatureFlag {
    id: "search.canary".into(),
    name: "Search Canary".into(),
    description: "Enable the new search pipeline".into(),
    enabled: true,
    rollout_percentage: 25.0,
    target_roles: vec!["beta_tester".into()],
    target_users: vec!["user-123".into()],
    metadata: std::collections::HashMap::new(),
})?;

let context = UserContext {
    user_id: "user-987".into(),
    roles: vec!["beta_tester".into()],
    session_id: None,
    ip_address: None,
    user_agent: None,
    attributes: Default::default(),
};

if manager.is_enabled("search.canary", Some(&context)).await {
    // Serve the new experience.
}
```

Add metadata (for example, `"owner" -> "growth-team"`) to integrate with ops
dashboards.

---

## Integration Points & Best Practices
- **Security coupling**: Audit events depend on `crate::security::rbac::Permission`
  and feature flags rely on `security::rbac::UserContext`. Keep those structures
  current when introducing new roles or permissions.
- **Sensitive data**: Avoid logging credentials or tokens. Use high-level
  descriptors (e.g., redact IDs or count records instead of embedding payloads).
- **Retention**: Periodically export or archive audits via `export` or
  downstream ingestion. Align retention with organisational policies
  (e.g., 90 days for SOC 2 evidence).
- **Streaming failures**: Configure alerting on webhook failures by scraping
  `tracing` output or forwarding to observability pipelines.
- **Configuration hygiene**: Clear overrides before persisting snapshots to
  prevent leaking environment-specific values.
- **Feature lifecycle**: Remove flags once rollout reaches 100% and code paths
  stabilise to minimize configuration debt.

---

## Testing & Tooling
- Full module suite:
  - `cargo test -p pulsearc-common --features "platform test-utils" compliance::audit`
  - `cargo test -p pulsearc-common --features "platform test-utils" compliance::config`
  - `cargo test -p pulsearc-common --features "platform test-utils" compliance::feature_flags`
- Integration tests: `cargo test --workspace --all-features compliance_integration`
  (ensures cross-module behaviour).
- Lints: `cargo clippy --workspace --all-targets --all-features`.

Tests rely on temporary directories (`tempfile`) and mocked HTTP via `wiremock`
in higher-level suites. Network calls in unit tests are avoided; remote sync
tests should inject fixtures instead of hitting live services.

---

## Extending the Module
1. **New audit event** – add a variant to `AuditEvent`, update the `match` in
   `get_type`, and include unit coverage in `audit.rs`.
2. **New configuration setting** – no schema enforcement exists today; consider
   adding validation before relying on critical keys.
3. **Additional feature flag targeting** – extend `FeatureFlag` with new fields
   (remember to update serialization and evaluation order).
4. **Streaming destinations** – follow the pattern in `stream_to_service`,
   spawning async tasks that log (not panic) on failure.

Document any behavioural changes here and in `docs/` so downstream teams can
follow the compliance trail.

---

## Related Documentation
- `docs/security/` – policies that drive audit requirements.
- `docs/runtime/` – async runtime guidelines shared across platform modules.
- `crates/common/src/security/` – RBAC and permission primitives referenced by
  compliance.

Keep this README in sync with code changes to maintain accurate onboarding for
new contributors and auditors.
