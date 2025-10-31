# Tracker Module Refactoring Plan

**Status:** Planning Phase
**Priority:** P0 - Critical Path
**Estimated Effort:** 16-24 hours
**Owner:** TBD
**Created:** 2025-01-30

---

## Executive Summary

This document outlines the comprehensive plan to refactor the tracker module (~5,290 LOC) from `legacy/api/src/tracker` into the new modular crates architecture. The tracker is **mission-critical** as it provides the core activity tracking functionality that the entire application depends on.

**Why Start Here:**
- The current `crates/infra/src/platform/macos.rs` is a placeholder returning "Unknown"
- The real implementation exists in legacy with 943 lines of working macOS-specific code
- All other features (classification, sync, integrations) depend on activity data
- Clear boundaries with well-defined trait interfaces make it a good first refactor

**Platform-Agnostic Design Goal:**
- Keep core business logic platform-independent
- Create thin platform wrappers for macOS and Windows
- Enable easy addition of new platforms (Linux, etc.)
- Share enrichment, caching, and state management logic across platforms

---

## Platform-Agnostic Design Principles

> **Goal:** Design for Windows from day one while refactoring macOS implementation.

### Architecture Overview

**Three-Layer Design:**

```
┌─────────────────────────────────────────────────┐
│         Application Layer (crates/api)          │
│    Platform-independent business logic          │
└─────────────────────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────┐
│          Core Layer (crates/core)               │
│    Traits, domain types, orchestration          │
│    NO platform-specific code                    │
└─────────────────────────────────────────────────┘
                     ▼
┌─────────────────────────────────────────────────┐
│      Infrastructure (crates/infra)              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐      │
│  │  macOS   │  │ Windows  │  │  Linux   │      │
│  │ Provider │  │ Provider │  │ Provider │      │
│  └──────────┘  └──────────┘  └──────────┘      │
│         Thin platform wrappers                   │
└─────────────────────────────────────────────────┘
```

### Design Rules

#### Rule 1: Core is Platform-Agnostic

**Core crate MUST NOT:**
- Import platform-specific crates (cocoa, windows, x11)
- Use `#[cfg(target_os = "...")]` (except for tests)
- Call OS-specific APIs directly
- Know about Accessibility API, Win32, or X11

**Core crate SHOULD:**
- Define traits for platform capabilities
- Contain domain types and business logic
- Orchestrate activity tracking workflow
- Handle caching, enrichment coordination, event routing

**Example:**

```rust
// ✅ crates/core/src/tracking/ports.rs - Platform agnostic

#[async_trait]
pub trait ActivityProvider: Send + Sync {
    /// Get current foreground application info
    async fn get_foreground_app(&self) -> Result<AppInfo>;

    /// Get recent application list
    async fn get_recent_apps(&self, limit: usize) -> Result<Vec<AppInfo>>;

    /// Check if provider has necessary permissions
    fn has_permissions(&self) -> bool;

    /// Request permissions from user (if applicable)
    async fn request_permissions(&self) -> Result<()>;
}

// Platform-independent domain types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppInfo {
    pub name: String,
    pub bundle_id: Option<String>,  // macOS
    pub process_name: Option<String>,  // Windows/Linux
    pub window_title: String,
    pub process_id: u32,
}
```

#### Rule 2: Shared Logic in Common Modules

**Move reusable code out of platform implementations:**

```rust
// crates/infra/src/platform/shared/cache.rs
// Used by BOTH macOS and Windows

pub struct EnrichmentCache {
    cache: Cache<String, EnrichmentData>,
}

impl EnrichmentCache {
    pub fn new(ttl: Duration, max_size: u64) -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(ttl)
                .max_capacity(max_size)
                .build(),
        }
    }

    pub async fn get_or_insert<F, Fut>(&self, key: String, fetch: F) -> EnrichmentData
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<EnrichmentData>>,
    {
        // Shared caching logic for all platforms
    }
}
```

#### Rule 3: Platform Selection at Compile Time

```rust
// crates/infra/src/platform/mod.rs

pub mod shared;  // Common utilities

#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "macos")]
pub use macos::MacOsActivityProvider as PlatformProvider;

#[cfg(target_os = "windows")]
pub mod windows;
#[cfg(target_os = "windows")]
pub use windows::WindowsActivityProvider as PlatformProvider;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub mod fallback;
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use fallback::FallbackProvider as PlatformProvider;
```

#### Rule 4: Conditional Dependencies

```toml
# crates/infra/Cargo.toml

[dependencies]
# Platform-agnostic (all platforms)
tokio = { version = "1", features = ["rt-multi-thread", "sync", "time"] }
moka = { version = "0.12", features = ["future"] }  # Shared cache
metrics = "0.21"

# macOS-only
[target.'cfg(target_os = "macos")'.dependencies]
core-foundation = "0.9"
cocoa = "0.25"
objc = "0.2"

# Windows-only
[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_UI_WindowsAndMessaging",
    "Win32_System_Threading",
    "Win32_UI_Accessibility",
] }
```

### Platform Feature Parity Matrix

| Feature | macOS | Windows | Linux | Notes |
|---------|-------|---------|-------|-------|
| **Core** | | | | |
| Foreground app | ✅ | ⏳ Phase 7 | 🚧 Stub | All platforms |
| Window title | ✅ | ⏳ Phase 7 | 🚧 Stub | All platforms |
| Process ID | ✅ | ⏳ Phase 7 | 🚧 Stub | All platforms |
| Recent apps | ✅ | ⏳ Phase 7 | ❌ | macOS: NSWorkspace, Win: EnumWindows |
| **Identifiers** | | | | |
| Bundle ID | ✅ | ❌ | ❌ | macOS-specific |
| Process path | ✅ | ⏳ Phase 7 | 🚧 Stub | Win: QueryFullProcessImageName |
| **Enrichment** | | | | |
| Browser URL | ✅ | ⏳ Phase 7 | ❌ | macOS: AX, Win: UIA |
| Office doc name | ✅ | ⏳ Phase 7 | ❌ | macOS: AX, Win: COM |
| **Events** | | | | |
| App switch | ✅ | ⏳ Phase 7 | ❌ | macOS: NSWorkspace, Win: SetWinEventHook |
| Sleep/Wake | ✅ | ⏳ Phase 7 | ❌ | macOS: IOKit, Win: PowerSetting |
| Lock/Unlock | ✅ | ⏳ Phase 7 | ❌ | macOS: DistributedNotificationCenter, Win: WTS |
| **Idle** | | | | |
| System idle time | ✅ | ⏳ Phase 7 | ❌ | macOS: IOHIDGetIdleTime, Win: GetLastInputInfo |

**Legend:** ✅ Done | ⏳ Planned | 🚧 Stub | ❌ Not planned

### Windows Implementation Preview

**Phase 7 will add:**

```rust
// crates/infra/src/platform/windows/provider.rs

use windows::Win32::UI::WindowsAndMessaging::*;
use super::shared::EnrichmentCache;

pub struct WindowsActivityProvider {
    cache: Arc<EnrichmentCache>,  // Shared with macOS!
    paused: AtomicBool,
}

#[async_trait]
impl ActivityProvider for WindowsActivityProvider {
    async fn get_foreground_app(&self) -> Result<AppInfo> {
        unsafe {
            let hwnd = GetForegroundWindow();
            let window_title = self.get_window_title(hwnd)?;
            let process_id = self.get_process_id(hwnd)?;
            let process_path = self.get_process_path(process_id)?;

            Ok(AppInfo {
                name: extract_app_name(&process_path),
                bundle_id: None,  // Not applicable
                process_name: Some(process_path.file_name()?.to_string()),
                window_title,
                process_id,
            })
        }
    }

    fn has_permissions(&self) -> bool {
        true  // No special permissions needed
    }
}
```

### Cross-Platform Testing

**Test platform-agnostic code once:**

```rust
// crates/core/tests/tracking_tests.rs - Runs on ALL platforms

#[tokio::test]
async fn test_tracking_service() {
    let provider = MockActivityProvider::new();
    let service = TrackingService::new(provider);
    // ... test works on macOS, Windows, Linux
}
```

**Test platform-specific code separately:**

```rust
// crates/infra/tests/platform_tests.rs

#[cfg(target_os = "macos")]
mod macos_tests { /* ... */ }

#[cfg(target_os = "windows")]
mod windows_tests { /* ... */ }
```

### Documentation Standards

**Document platform behavior in traits:**

```rust
/// Get the foreground application information.
///
/// # Platform Behavior
///
/// - **macOS**: Accessibility API (requires permission)
/// - **Windows**: Win32 API (no permission needed)
/// - **Linux**: Stub (returns placeholder)
///
/// # Errors
///
/// - `ActivityError::PermissionDenied` - macOS only
/// - `ActivityError::NoForegroundWindow` - All platforms
async fn get_foreground_app(&self) -> Result<AppInfo>;
```

---

## Rust Best Practices & Design Improvements

### 1. Generics Over Trait Objects

**Problem:** `Box<dyn ActivityProvider>` adds dynamic dispatch overhead in hot paths.

**Solution:** Make `TrackingService` generic:

```rust
pub struct TrackingService<P: ActivityProvider> {
    provider: Arc<P>,
    event_source: Option<Box<dyn OsEventSource>>,
    // ...
}
```

**For Dynamic Dispatch:** Create an object-safe adapter:

```rust
use std::{future::Future, pin::Pin};

pub trait ActivityProviderDyn: Send + Sync {
    fn get_activity(&self) -> Pin<Box<dyn Future<Output = Result<ActivityContext>> + Send + '_>>;
    fn is_paused(&self) -> bool;
    fn pause(&self) -> Result<()>;
    fn resume(&self) -> Result<()>;
}

impl<T: ActivityProvider + ?Sized> ActivityProviderDyn for T {
    fn get_activity(&self) -> Pin<Box<dyn Future<Output = _> + Send + '_>> {
        Box::pin(ActivityProvider::get_activity(self))
    }
    // ... forward other methods
}
```

### 2. Stream-Based Events

**Problem:** Callbacks (`Box<dyn Fn()>`) are hard to test and compose.

**Solution:** Expose events as `Stream`:

```rust
use futures_core::Stream;

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum OsEvent {
    AppActivated { bundle_id: String, app_name: String },
    Sleep,
    Wake,
    Locked,
    Unlocked,
}

pub trait OsEventSource: Send + Sync {
    type Stream<'a>: Stream<Item = OsEvent> + Send
    where
        Self: 'a;

    fn events(&self) -> Self::Stream<'_>;
}
```

**Benefits:**
- Composable with `select!`, `StreamExt::filter`, etc.
- Natural backpressure
- Easier to test with mock streams

**Implementation:** Use `tokio::sync::broadcast` or `mpsc` internally.

### 3. Cancellation Token

**Problem:** `Arc<AtomicBool>` for shutdown is manual and error-prone.

**Solution:** Use `tokio_util::sync::CancellationToken`:

```rust
use tokio_util::sync::CancellationToken;
use tokio::task::JoinSet;

pub struct TrackingService<P: ActivityProvider> {
    provider: Arc<P>,
    cancel_token: CancellationToken,
    tasks: JoinSet<Result<()>>,
}

impl<P: ActivityProvider> TrackingService<P> {
    pub async fn stop(&mut self) -> Result<()> {
        self.cancel_token.cancel();

        // Wait for all tasks to complete
        while let Some(result) = self.tasks.join_next().await {
            result??;  // Propagate panics and errors
        }

        Ok(())
    }

    async fn snapshot_writer_task(&self) {
        let mut interval = tokio::time::interval(Duration::from_secs(30));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => break,
                _ = interval.tick() => {
                    // Save snapshot
                }
            }
        }
    }
}
```

### 4. Structured Error Handling

**Problem:** `ActivityTracking(String)` loses type information.

**Solution:** Use `thiserror` for structured errors:

```rust
// In crates/infra/src/platform/error.rs
#[derive(thiserror::Error, Debug)]
pub enum ActivityError {
    #[error("Accessibility permission denied")]
    AccessibilityDenied,

    #[error("Accessibility API error: {0}")]
    Accessibility(#[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("OS event listener error: {0}")]
    OsEvents(#[from] OsEventError),

    #[error("Enrichment timeout after {timeout_ms}ms")]
    EnrichmentTimeout { timeout_ms: u64 },

    #[error("Cache error: {0}")]
    Cache(String),
}

// In crates/shared/src/errors.rs
#[derive(thiserror::Error, Debug)]
pub enum PulseArcError {
    #[error("Activity tracking: {0}")]
    Activity(#[from] infra::ActivityError),

    // ... other variants
}

// For Tauri commands
#[derive(serde::Serialize)]
pub struct ApiError {
    code: &'static str,
    message: String,
}

impl From<PulseArcError> for ApiError {
    fn from(err: PulseArcError) -> Self {
        Self {
            code: err.code(),
            message: err.to_string(),
        }
    }
}

#[tauri::command]
pub async fn get_activity(
    ctx: State<'_, Arc<AppContext>>,
) -> Result<ActivityContext, ApiError> {
    ctx.tracking_service
        .get_activity()
        .await
        .map_err(Into::into)
}
```

### 5. Advanced Caching

**Problem:** `RwLock<HashMap>` has contention under heavy load.

**Solutions:**

**Option A - Sharded Map:**
```rust
use dashmap::DashMap;

pub struct EnrichmentCache {
    cache: DashMap<String, CacheEntry>,
    ttl: Duration,
}

impl EnrichmentCache {
    pub fn get(&self, key: &str) -> Option<EnrichmentData> {
        self.cache.get(key)
            .filter(|entry| entry.is_fresh(self.ttl))
            .map(|entry| entry.data.clone())
    }
}
```

**Option B - TTL Cache (Recommended):**
```rust
use moka::future::Cache;

pub struct EnrichmentCache {
    cache: Cache<String, EnrichmentData>,
}

impl EnrichmentCache {
    pub fn new() -> Self {
        Self {
            cache: Cache::builder()
                .time_to_live(Duration::from_millis(750))
                .max_capacity(100)
                .build(),
        }
    }

    pub async fn get_or_insert_with<F, Fut>(
        &self,
        key: String,
        f: F,
    ) -> EnrichmentData
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = EnrichmentData>,
    {
        self.cache
            .get_or_insert_with(key, f)
            .await
    }
}
```

**Metrics:** Moka provides built-in hit rate, eviction counts, etc.

### 6. Minimal Tokio in Core

**Keep `core` runtime-agnostic:**

```rust
// crates/core/Cargo.toml
[dependencies]
futures-core = "0.3"
futures-util = "0.3"
# NO tokio here

// Use traits for time/sync
pub trait Clock: Send + Sync {
    fn now(&self) -> Instant;
}

pub struct SystemClock;
impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}
```

**Tokio only in `infra`:**

```rust
// crates/infra/Cargo.toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "time", "sync", "signal"] }
# NOT "full"
```

### 7. Compile-Time Platform Gating

**Prefer `#[cfg]` over runtime checks:**

```rust
// ❌ Bad: Runtime check
pub trait OsEventSource {
    fn is_supported() -> bool;
}

// ✅ Good: Compile-time selection
#[cfg(target_os = "macos")]
pub type PlatformEventSource = MacOsEventSource;

#[cfg(not(target_os = "macos"))]
pub type PlatformEventSource = FallbackEventSource;

// For runtime permission checks, be explicit:
pub trait AccessibilityProvider {
    fn has_permission(&self) -> bool;
    fn request_permission(&self) -> Result<()>;
}
```

### 8. Modern macOS FFI

**Consider `objc2`/`icrate` instead of `cocoa`/`objc`:**

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = "0.5"
icrate = { version = "0.1", features = ["Foundation", "AppKit"] }
```

**Benefits:**
- Better Send/Sync safety
- More type-safe bindings
- Active maintenance

**If staying with `objc`:**
- Keep `unsafe` blocks minimal and well-documented
- Wrap in safe interfaces immediately
- Add SAFETY comments for each unsafe block

### 9. API Surface Hardening

```rust
// Make enums non-exhaustive for forward compatibility
#[non_exhaustive]
pub enum PauseReason {
    Manual,
    Idle,
    ScreenLocked,
}

#[non_exhaustive]
pub enum OsEvent {
    AppActivated { bundle_id: String },
    Sleep,
    Wake,
}

// Mark important returns as must_use
#[must_use = "Dropping the guard will resume tracking"]
pub struct PauseGuard<'a> {
    service: &'a TrackingService,
}

// Workspace-level lints in Cargo.toml:
[workspace.lints.rust]
missing_docs = "warn"
unsafe_code = "forbid"  # Except in FFI crates

[workspace.lints.clippy]
pedantic = "warn"
unwrap_used = "deny"
expect_used = "deny"
```

### 10. Interior Mutability for Pause/Resume

**Problem:** `pause(&mut self)` requires exclusive access across threads.

**Solution:** Use interior mutability:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

pub struct MacOsActivityProvider {
    paused: AtomicBool,
    // ... other fields
}

impl ActivityProvider for MacOsActivityProvider {
    fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Acquire)
    }

    fn pause(&self) -> Result<()> {  // Note: &self, not &mut self
        self.paused.store(true, Ordering::Release);
        Ok(())
    }

    fn resume(&self) -> Result<()> {
        self.paused.store(false, Ordering::Release);
        Ok(())
    }
}
```

### 11. Snapshot Writer with Backpressure

```rust
use tokio::sync::watch;

pub struct TrackingService<P: ActivityProvider> {
    // Latest activity always available without blocking
    current_activity: watch::Receiver<ActivityContext>,
    activity_sender: watch::Sender<ActivityContext>,
}

impl<P: ActivityProvider> TrackingService<P> {
    pub async fn get_activity(&self) -> ActivityContext {
        // O(1), just borrow the latest value
        self.current_activity.borrow().clone()
    }

    async fn refresh_loop(&self) {
        let mut event_stream = self.event_source.events();

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => break,
                Some(event) = event_stream.next() => {
                    match event {
                        OsEvent::AppActivated { .. } => {
                            if let Ok(ctx) = self.provider.get_activity().await {
                                // Send to watcher (overwrites, no queue)
                                let _ = self.activity_sender.send(ctx);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
```

### 12. Domain Newtypes

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BundleId(String);

#[derive(Debug, Clone)]
pub struct WindowTitle(String);

#[derive(Debug, Clone, Copy)]
pub struct Milliseconds(u64);

impl Milliseconds {
    pub const fn from_millis(ms: u64) -> Self {
        Self(ms)
    }

    pub fn as_duration(self) -> Duration {
        Duration::from_millis(self.0)
    }
}

// Prevents mixing up parameters:
fn enrich(&self, bundle: BundleId, title: WindowTitle) { }
// vs error-prone:
fn enrich(&self, bundle: String, title: String) { }
```

---

## Current State Analysis

### Legacy Structure (~5,290 LOC)

```
legacy/api/src/tracker/
├── core.rs (773 LOC)              # Tracker & RefresherState
├── provider.rs (41 LOC)           # ActivityProvider trait
├── mod.rs (22 LOC)                # Module exports
├── idle/                          # Idle detection (1,957 LOC)
│   ├── detector.rs (440 LOC)      # MacOsIdleDetector
│   ├── period_tracker.rs (476)    # IdlePeriodTracker
│   ├── recovery.rs (603 LOC)      # SleepWakeListener
│   ├── lock_detection.rs (350)    # LockScreenListener
│   ├── config.rs (189 LOC)        # IdleConfig, RecoveryConfig
│   ├── types.rs (269 LOC)         # IdleError, PauseReason, ActivityEvent
│   └── mod.rs (20 LOC)
├── providers/                     # Platform implementations (1,122 LOC)
│   ├── macos.rs (943 LOC)         # MacOsProvider with enrichment
│   ├── dummy.rs (168 LOC)         # Windows/Linux fallback
│   └── mod.rs (11 LOC)
└── os_events/                     # Event abstraction (985 LOC)
    ├── macos.rs (400 LOC)         # NSWorkspace listener
    ├── macos_ax.rs (372 LOC)      # Accessibility API helpers
    ├── traits.rs (64 LOC)         # DI traits
    ├── fallback.rs (61 LOC)       # Non-macOS stub
    ├── mod.rs (88 LOC)
    └── README.md
```

### New Structure (Current - Placeholders)

```
crates/
├── core/src/tracking/
│   ├── ports.rs (51 LOC)          # ActivityProvider trait (async)
│   ├── service.rs (placeholder)   # TrackingService (empty)
│   └── mod.rs
├── infra/src/platform/
│   ├── macos.rs (47 LOC)          # Placeholder returning "Unknown"
│   └── mod.rs
└── shared/src/
    └── types.rs                   # ActivityContext, ActivitySnapshot
```

### Key Differences

| Aspect | Legacy | New Crates |
|--------|--------|------------|
| **API Style** | Synchronous | Async (async-trait) |
| **Error Handling** | Custom ActivityError | Unified PulseArcError |
| **Provider Trait** | `fn fetch()` | `async fn get_activity()` |
| **Dependencies** | Monolithic | Modular crates |
| **Testing** | Some mocks | Full DI with traits |
| **Pause State** | External management | Built into provider |

---

## Refactoring Strategy

### Phase 1: Foundation (4-6 hours)

**Goal:** Establish core traits, types, and error handling

#### 1.1 Update Core Traits

**File:** `crates/core/src/tracking/ports.rs`

**Actions:**
- ✅ Keep existing async `ActivityProvider` trait
- ✅ `ActivityRepository` and `ActivityEnricher` are good as-is
- ⚠️ Add missing traits from legacy:
  ```rust
  /// OS event listener abstraction
  #[async_trait]
  pub trait OsEventListener: Send + Sync {
      async fn start(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()>;
      async fn stop(&mut self) -> Result<()>;
      fn is_supported() -> bool where Self: Sized;
  }
  ```

#### 1.2 Migrate Types

**File:** `crates/shared/src/types.rs`

**Actions:**
- Port `TrackerState` struct
- Port `PauseReason` enum from `idle/types.rs`
- Port `WindowContext` if missing
- Ensure `ActivityContext` matches legacy structure
- Add `RefresherState` concept (or replace with new design)

#### 1.3 Update Error Types

**File:** `crates/shared/src/errors.rs`

**Actions:**
- Add `ActivityError` variant to `PulseArcError` if missing:
  ```rust
  #[error("Activity tracking error: {0}")]
  ActivityTracking(String),
  ```

**Acceptance Criteria:**
- [ ] All traits compile without errors
- [ ] Types match legacy structure
- [ ] Error conversions work seamlessly
- [ ] Run `cargo check --all` successfully

---

### Phase 2: Idle Detection Module (4-6 hours)

**Goal:** Port idle detection, sleep/wake recovery, and lock screen detection

#### 2.1 Create Idle Module Structure

**New Files:**
```
crates/infra/src/idle/
├── mod.rs
├── config.rs          # IdleConfig, RecoveryConfig
├── types.rs           # ActivityEvent, IdleError
├── detector.rs        # MacOsIdleDetector
├── period_tracker.rs  # IdlePeriodTracker
├── recovery.rs        # SleepWakeListener
└── lock_detection.rs  # LockScreenListener
```

#### 2.2 Port Implementation

**Source:** `legacy/api/src/tracker/idle/*`

**Key Changes:**
- Convert synchronous APIs to async where appropriate
- Replace custom error types with `PulseArcError`
- Update imports to use new crate structure
- Keep macOS-specific code in `#[cfg(target_os = "macos")]` blocks

**Dependencies:**
- `IOKit-sys` (for macOS idle detection)
- `core-foundation` (for sleep/wake notifications)

#### 2.3 Testing

**Actions:**
- Port existing unit tests to new structure
- Add integration tests for idle detection
- Mock sleep/wake events for testing

**Acceptance Criteria:**
- [ ] Idle detector correctly identifies system idle time
- [ ] Sleep/wake recovery prevents false idle detection
- [ ] Lock screen detection works on macOS
- [ ] All tests pass: `cargo test --package pulsearc-infra idle`
- [ ] No clippy warnings

---

### Phase 3: OS Event Abstraction (3-4 hours)

**Goal:** Port event-driven architecture for macOS NSWorkspace

#### 3.1 Create OS Events Module

**New Files:**
```
crates/infra/src/os_events/
├── mod.rs              # Exports, platform selection
├── traits.rs           # DI traits (WorkspaceNotifications, AxProvider)
├── macos.rs            # MacOsEventListener
├── macos_ax.rs         # Accessibility API helpers
└── fallback.rs         # Non-macOS polling fallback
```

#### 3.2 Port Implementation

**Source:** `legacy/api/src/tracker/os_events/*`

**Key Changes:**
- Implement `OsEventListener` trait from Phase 1
- Convert callback-based API to async channels
- Add proper error propagation
- Keep test mocks for DI

**macOS NSWorkspace Integration:**
```rust
impl OsEventListener for MacOsEventListener {
    async fn start(&mut self, callback: Box<dyn Fn() + Send + Sync>) -> Result<()> {
        // Register NSWorkspace.didActivateApplicationNotification
        // Invoke callback on each app activation
    }
}
```

#### 3.3 Testing

**Actions:**
- Port DI test mocks to new structure
- Test NSWorkspace observer lifecycle
- Verify fallback behavior on non-macOS

**Acceptance Criteria:**
- [ ] NSWorkspace notifications trigger callbacks on macOS
- [ ] Fallback returns error on non-macOS platforms
- [ ] Observer cleanup works properly
- [ ] Tests pass: `cargo test --package pulsearc-infra os_events`
- [ ] No memory leaks (verify with instruments)

---

### Phase 4: macOS Provider (5-7 hours)

**Goal:** Port full-featured macOS provider with enrichment

#### 4.1 Port MacOsProvider

**File:** `crates/infra/src/platform/macos.rs`

**Source:** `legacy/api/src/tracker/providers/macos.rs` (943 LOC)

**Key Components:**
1. **Base Provider:**
   - Accessibility API integration
   - App/window info fetching
   - Recent apps list

2. **Enrichment System:**
   - Browser URL extraction (Chrome, Safari, Firefox, Arc, Edge, Brave)
   - Office document metadata (Excel, Word, PowerPoint)
   - PDF document names (Acrobat, Preview, PDF Expert)
   - TTL-based caching (750ms)

3. **Background Worker:**
   - Async enrichment queue
   - Throttling (750ms per app+title)
   - Bounded channel (size: 10)

**Key Changes:**
- Convert `fetch()` to `async fn get_activity()`
- Replace `Arc<Mutex<HashMap>>` cache with `tokio::sync::RwLock`
- Use `tokio::spawn` for background worker
- Replace `std::sync::mpsc` with `tokio::sync::mpsc`
- Add structured logging with `tracing`

#### 4.2 Enrichment Architecture

**Current (Legacy):**
```rust
// Synchronous cache-first with optional background worker
let enrichment = match self.cache.lock().unwrap().get(&bundle_id) {
    Some(entry) if entry.is_fresh() => entry.data.clone(),
    _ => self.fetch_enrichment_sync(bundle_id, window_title),
};
```

**Proposed (Async):**
```rust
// Async cache-first with background enrichment
let cache = self.cache.read().await;
let enrichment = match cache.get(&bundle_id) {
    Some(entry) if entry.is_fresh() => entry.data.clone(),
    _ => {
        drop(cache);  // Release read lock
        self.enqueue_enrichment(bundle_id, window_title).await;
        EnrichmentData::default()  // Return placeholder, cache updates in background
    }
};
```

#### 4.3 Dependencies

**Cargo.toml additions:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"
objc = "0.2"
core-foundation = "0.9"
core-graphics = "0.23"
```

#### 4.4 Testing

**Actions:**
- Port MacOsProvider unit tests
- Test enrichment cache hit/miss scenarios
- Test background worker lifecycle
- Mock Accessibility API for CI/CD

**Acceptance Criteria:**
- [ ] Basic activity fetching works (app name, window title)
- [ ] Browser URL enrichment works for major browsers
- [ ] Office document enrichment works
- [ ] Cache TTL expiration works correctly
- [ ] Background worker processes jobs without blocking
- [ ] Tests pass: `cargo test --package pulsearc-infra macos`
- [ ] Clippy passes with strict lints

---

### Phase 5: Tracker Core Service (4-6 hours)

**Goal:** Port central orchestration logic

#### 5.1 Create TrackingService

**File:** `crates/core/src/tracking/service.rs`

**Source:** `legacy/api/src/tracker/core.rs` (773 LOC)

**Key Components:**
1. **Service Structure:**
   ```rust
   pub struct TrackingService {
       provider: Arc<dyn ActivityProvider>,
       event_listener: Option<Box<dyn OsEventListener>>,
       snapshot_interval: Duration,
       stop_signal: Arc<AtomicBool>,
       // ... other fields
   }
   ```

2. **Lifecycle Methods:**
   - `new()` - Create service with provider
   - `start()` - Start event listener and snapshot writer
   - `stop()` - Graceful shutdown
   - `get_activity()` - Get current activity (cache-aware)

3. **Event-Driven Refresher:**
   - Use tokio task instead of thread
   - Handle app switch events
   - Smart change detection
   - Circuit breaker for event emission failures

4. **Snapshot Writer:**
   - Periodic persistence (30s interval)
   - Use `ActivityRepository` trait
   - Handle errors gracefully

#### 5.2 Refactoring from Legacy

**Legacy Pattern (Thread-based):**
```rust
fn start_refresher(&self) -> std::thread::JoinHandle<()> {
    let provider = self.provider.clone();
    std::thread::spawn(move || {
        loop {
            let context = provider.fetch().ok();
            // ... emit event
            std::thread::sleep(Duration::from_millis(250));
        }
    })
}
```

**New Pattern (Async task-based):**
```rust
pub async fn start(&mut self) -> Result<()> {
    // Start event listener
    if let Some(listener) = &mut self.event_listener {
        listener.start(Box::new(move || {
            // Trigger activity fetch via channel
        })).await?;
    }

    // Start snapshot writer task
    let snapshot_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(30)).await;
            // ... save snapshot
        }
    });

    Ok(())
}
```

#### 5.3 Testing

**Actions:**
- Port core integration tests
- Test event-driven vs polling mode
- Test snapshot persistence
- Test graceful shutdown

**Acceptance Criteria:**
- [ ] Service starts and stops cleanly
- [ ] Activity context updates on app switch
- [ ] Snapshots persist every 30 seconds
- [ ] Circuit breaker prevents event spam
- [ ] Tests pass: `cargo test --package pulsearc-core tracking`
- [ ] No data races (run with `--features tokio-console`)

---

### Phase 6: Integration & Commands (2-3 hours)

**Goal:** Wire up refactored tracker to Tauri commands

#### 6.1 Update AppContext

**File:** `crates/api/src/context/mod.rs`

**Actions:**
- Replace legacy tracker with new `TrackingService`
- Initialize macOS provider
- Start tracking service

**Before:**
```rust
// Legacy (not currently used in new structure)
let provider = legacy::MacOsProvider::new(false);
let tracker = legacy::Tracker::new(provider, metrics, config);
```

**After:**
```rust
let provider = Arc::new(MacOsActivityProvider::new());
let tracking_service = TrackingService::new(provider);
tracking_service.start().await?;
```

#### 6.2 Update Commands

**File:** `crates/api/src/commands/tracking.rs`

**Actions:**
- Update `get_activity` command to use new service
- Update `pause_tracker` / `resume_tracker` commands
- Add proper error handling

**Command Signature:**
```rust
#[tauri::command]
pub async fn get_activity(
    ctx: State<'_, Arc<AppContext>>,
) -> Result<ActivityContext, String> {
    ctx.tracking_service
        .get_activity()
        .await
        .map_err(|e| e.to_string())
}
```

#### 6.3 Testing

**Actions:**
- Test Tauri commands manually
- Verify frontend receives activity updates
- Test pause/resume functionality

**Acceptance Criteria:**
- [ ] `get_activity` command returns valid data
- [ ] `pause_tracker` stops activity tracking
- [ ] `resume_tracker` restarts activity tracking
- [ ] Frontend UI shows live activity updates
- [ ] Manual testing: `pnpm tauri:dev`

---

### Phase 7: Windows/Linux Support (3-4 hours)

**Goal:** Port cross-platform fallback provider

#### 7.1 Port DummyProvider

**File:** `crates/infra/src/platform/dummy.rs`

**Source:** `legacy/api/src/tracker/providers/dummy.rs` (168 LOC)

**Actions:**
- Port Windows Win32 API integration
- Port Linux placeholder
- Convert to async API

**Windows Implementation:**
```rust
#[cfg(target_os = "windows")]
impl ActivityProvider for DummyProvider {
    async fn get_activity(&self) -> Result<ActivityContext> {
        let hwnd = unsafe { GetForegroundWindow() };
        let window_title = get_window_title(hwnd)?;
        let process_path = get_process_path(hwnd)?;
        // ... build ActivityContext
    }
}
```

#### 7.2 Platform Selection

**File:** `crates/infra/src/platform/mod.rs`

```rust
#[cfg(target_os = "macos")]
pub use macos::MacOsActivityProvider;

#[cfg(target_os = "windows")]
pub use dummy::WindowsActivityProvider;

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub use dummy::DummyActivityProvider;
```

**Acceptance Criteria:**
- [ ] Windows provider fetches app name and window title
- [ ] Linux provider returns placeholder context
- [ ] Platform selection works at compile time
- [ ] Tests pass on all platforms

---

### Phase 8: Cleanup & Documentation (2-3 hours)

**Goal:** Remove legacy code and update documentation

#### 8.1 Remove Legacy Code

**Actions:**
- Delete `legacy/api/src/tracker/` entirely
- Remove legacy tracker dependencies from `legacy/api/Cargo.toml`
- Update imports across codebase

**Verification:**
```bash
# Search for legacy tracker imports
rg "legacy::tracker" --type rust
rg "legacy.*tracker" --type rust

# Should return no results
```

#### 8.2 Update Documentation

**Files to Update:**
- `docs/MACOS_ARCHITECTURE.md` - Update tracker references
- `docs/FILE_MAPPING.md` - Update file locations
- `README.md` - Update architecture diagram
- Add `crates/core/src/tracking/README.md` (port from legacy)
- Add `crates/infra/src/platform/README.md`

#### 8.3 Changelog

**File:** `CHANGELOG.md` (if exists)

```markdown
## [Unreleased]

### Changed
- Refactored tracker module to new modular architecture
- Migrated from sync to async activity tracking API
- Improved error handling with unified PulseArcError

### Improved
- Async enrichment prevents UI blocking
- Better platform abstraction for Windows/Linux support
- Enhanced testability with dependency injection
```

**Acceptance Criteria:**
- [ ] No references to `legacy/tracker` in codebase
- [ ] All documentation updated
- [ ] Architecture diagrams accurate
- [ ] `cargo build --all` succeeds
- [ ] `cargo test --all` passes

---

## Testing Strategy

### Unit Tests

**Scope:** Individual modules and functions

**Coverage Targets:**
- `crates/core/tracking`: 80%+
- `crates/infra/platform`: 70%+
- `crates/infra/idle`: 75%+
- `crates/infra/os_events`: 70%+

**Run:**
```bash
cargo test --all --lib
cargo tarpaulin --exclude-files "legacy/*" --out Html
```

### Integration Tests

**Scope:** Cross-module interactions

**Test Cases:**
1. **Activity Tracking Flow:**
   - Provider → Service → Command → Frontend
   - Verify end-to-end data flow

2. **Event-Driven Updates:**
   - App switch triggers activity update
   - Frontend receives event
   - Cache updated correctly

3. **Idle Detection:**
   - System idle triggers pause
   - Wake from sleep resumes tracking
   - Lock screen stops tracking

4. **Enrichment:**
   - Browser URL extracted correctly
   - Office document metadata captured
   - Cache hits/misses work as expected

**Run:**
```bash
cargo test --all --test '*'
```

### Manual Testing

**Checklist:**
- [ ] macOS: Activity tracking works
- [ ] macOS: Browser URL enrichment works (Chrome, Safari, Firefox)
- [ ] macOS: Office document enrichment works (Excel, Word, PowerPoint)
- [ ] macOS: Idle detection works correctly
- [ ] macOS: Sleep/wake recovery works
- [ ] macOS: Lock screen detection works
- [ ] Windows: Basic activity tracking works
- [ ] Linux: Placeholder returns "Unknown"
- [ ] Frontend: Live activity updates appear
- [ ] Frontend: Pause/resume buttons work
- [ ] No memory leaks (check with Activity Monitor)
- [ ] No excessive CPU usage (<5% idle)

**Tools:**
```bash
# Memory profiling
cargo instruments --template "Leaks" --bin pulsearc

# Performance profiling
cargo instruments --template "Time Profiler" --bin pulsearc

# Tokio console (async debugging)
RUSTFLAGS="--cfg tokio_unstable" cargo run --features tokio-console
```

### Advanced Testing

#### Loom for Concurrency

**Test cache and worker for race conditions:**

```rust
#[cfg(loom)]
mod loom_tests {
    use loom::sync::Arc;
    use loom::thread;

    #[test]
    fn cache_concurrent_access() {
        loom::model(|| {
            let cache = Arc::new(EnrichmentCache::new());

            let c1 = cache.clone();
            let h1 = thread::spawn(move || {
                c1.insert("key".into(), data1);
            });

            let c2 = cache.clone();
            let h2 = thread::spawn(move || {
                c2.get("key")
            });

            h1.join().unwrap();
            h2.join().unwrap();
            // Verify no races, no deadlocks
        });
    }
}
```

**Run:**
```bash
RUSTFLAGS="--cfg loom" cargo test --release --test loom_tests
```

#### Miri for Undefined Behavior

**Catch unsafe code issues:**

```bash
# Install miri
rustup +nightly component add miri

# Run tests under miri
cargo +nightly miri test --package pulsearc-infra
```

**Miri catches:**
- Use-after-free
- Invalid pointer dereference
- Data races
- Uninitialized memory reads

#### Proptest for State Machines

**Test idle detection state machine:**

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn idle_period_tracker_invariants(
        idle_threshold_ms in 1000u64..60000,
        activity_events in prop::collection::vec(
            (any::<bool>(), 0u64..5000),  // (is_idle, duration_ms)
            1..100
        )
    ) {
        let mut tracker = IdlePeriodTracker::new(
            Duration::from_millis(idle_threshold_ms)
        );

        for (is_idle, duration_ms) in activity_events {
            tracker.update(is_idle);
            std::thread::sleep(Duration::from_millis(duration_ms));

            // Invariant: current_idle_duration never exceeds idle_threshold
            // when activity is detected
            if !is_idle {
                assert!(tracker.current_idle_duration() == Duration::ZERO);
            }
        }
    }
}
```

#### Cargo Nextest

**Faster, better test runner:**

```bash
# Install
cargo install cargo-nextest

# Run tests
cargo nextest run --workspace

# Retry flaky tests
cargo nextest run --retries 3

# Run with coverage
cargo nextest run --no-fail-fast
cargo tarpaulin --engine llvm --follow-exec
```

**Benefits:**
- Parallel test execution
- Flaky test detection and quarantine
- Better failure output
- Per-test timing

#### Permission-Gated Tests

**Isolate macOS Accessibility tests:**

```rust
// In crates/infra/tests/macos_integration.rs

#[cfg(all(
    target_os = "macos",
    feature = "integration-tests-macos"
))]
mod macos_ax_tests {
    use pulsearc_infra::platform::MacOsActivityProvider;

    #[tokio::test]
    async fn test_real_accessibility_api() {
        // Requires Accessibility permission
        let provider = MacOsActivityProvider::new();
        let ctx = provider.get_activity().await.unwrap();

        assert!(!ctx.app_name.is_empty());
    }
}
```

**Run:**
```bash
# CI: Skip permission-gated tests
cargo test --workspace

# Local: Run with permissions
cargo test --workspace --features integration-tests-macos
```

---

## Metrics & Observability

### Structured Metrics

**Add counters for key operations:**

```rust
use metrics::{counter, histogram, gauge};

impl<P: ActivityProvider> TrackingService<P> {
    async fn refresh_loop(&self) {
        let mut event_stream = self.event_source.events();

        loop {
            tokio::select! {
                _ = self.cancel_token.cancelled() => {
                    counter!("tracking.shutdown_clean").increment(1);
                    break;
                }
                Some(event) = event_stream.next() => {
                    counter!("tracking.event_received").increment(1);

                    let start = Instant::now();
                    match self.handle_event(event).await {
                        Ok(_) => {
                            counter!("tracking.event_handled").increment(1);
                            histogram!("tracking.event_latency_ms")
                                .record(start.elapsed().as_millis() as f64);
                        }
                        Err(e) => {
                            counter!("tracking.event_error").increment(1);
                            tracing::error!(?e, "Event handling failed");
                        }
                    }
                }
            }
        }
    }
}
```

**Key Metrics:**

| Metric | Type | Purpose |
|--------|------|---------|
| `tracking.event_received` | Counter | Total events from OS |
| `tracking.event_handled` | Counter | Successfully processed events |
| `tracking.event_error` | Counter | Event handling failures |
| `tracking.event_latency_ms` | Histogram | Event→UI update latency |
| `enrichment.cache_hit` | Counter | Cache hit rate |
| `enrichment.cache_miss` | Counter | Cache miss rate |
| `enrichment.timeout` | Counter | Enrichment timeouts |
| `enrichment.enqueued` | Counter | Jobs queued |
| `enrichment.dropped` | Counter | Queue full, job dropped |
| `snapshot.saved` | Counter | Snapshots persisted |
| `snapshot.failed` | Counter | Snapshot persistence failures |

### Tracing with `#[instrument]`

```rust
use tracing::{instrument, info, warn, error};

impl MacOsActivityProvider {
    #[instrument(skip(self), fields(bundle_id = %bundle_id))]
    async fn enrich_activity(
        &self,
        bundle_id: &BundleId,
        window_title: &WindowTitle,
    ) -> Result<EnrichmentData> {
        info!("Starting enrichment");

        // Check cache
        if let Some(cached) = self.cache.get(bundle_id).await {
            info!("Cache hit");
            return Ok(cached);
        }

        // Fetch from API
        let result = self.fetch_enrichment(bundle_id, window_title).await?;

        info!(url = ?result.url, "Enrichment complete");
        Ok(result)
    }
}
```

**Tracing Subscribers:**

```rust
// In main.rs
use tracing_subscriber::{
    fmt, prelude::*, EnvFilter, Registry,
};

fn init_tracing() {
    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().with_target(true).with_line_number(true))
        .init();
}

// Set log level:
// RUST_LOG=pulsearc_infra=debug,pulsearc_core=trace cargo run
```

### Exporting Metrics

**For production monitoring:**

```rust
use metrics_exporter_prometheus::PrometheusBuilder;

fn setup_metrics() {
    PrometheusBuilder::new()
        .with_http_listener(([127, 0, 0, 1], 9090))
        .install()
        .expect("Failed to install Prometheus exporter");
}

// Metrics available at http://localhost:9090/metrics
```

---

## CI & Supply Chain Hygiene

### Dependency Auditing

**`cargo-deny`** - Comprehensive supply chain checks:

```toml
# deny.toml
[advisories]
vulnerability = "deny"
unmaintained = "warn"
yanked = "deny"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "ISC",
]
deny = [
    "GPL-3.0",
]

[bans]
multiple-versions = "warn"
wildcards = "deny"

[sources]
unknown-registry = "deny"
unknown-git = "deny"
```

**Run:**
```bash
cargo install cargo-deny
cargo deny check
```

**`cargo-audit`** - Security vulnerabilities:

```bash
cargo install cargo-audit
cargo audit
```

**`cargo-udeps`** - Unused dependencies:

```bash
cargo +nightly install cargo-udeps
cargo +nightly udeps --all-targets
```

### MSRV Policy

**Set Minimum Supported Rust Version:**

```toml
# In each Cargo.toml
[package]
name = "pulsearc-core"
rust-version = "1.75.0"  # MSRV
```

**Check in rust-toolchain.toml:**

```toml
[toolchain]
channel = "1.75.0"
components = ["rustfmt", "clippy", "rust-src"]
targets = ["x86_64-apple-darwin", "aarch64-apple-darwin"]
```

### CI Pipeline

**`.github/workflows/ci.yml`:**

```yaml
name: CI

on: [push, pull_request]

env:
  RUSTFLAGS: "-D warnings"

jobs:
  test:
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, beta]

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - name: Check formatting
        run: cargo fmt --all -- --check

      - name: Clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

      - name: Build
        run: cargo build --workspace --all-features

      - name: Test
        run: cargo nextest run --workspace --all-features

      - name: Doc tests
        run: cargo test --doc --workspace

  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Audit dependencies
        run: |
          cargo install cargo-audit
          cargo audit

      - name: Check licenses
        run: |
          cargo install cargo-deny
          cargo deny check

      - name: Unused deps
        run: |
          cargo +nightly install cargo-udeps
          cargo +nightly udeps --all-targets

  coverage:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage
        run: cargo tarpaulin --out Xml --all-features

      - name: Upload to codecov
        uses: codecov/codecov-action@v3

  miri:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@nightly
        with:
          components: miri

      - name: Run miri
        run: cargo +nightly miri test --package pulsearc-infra
```

### Pre-commit Hooks

**`.husky/pre-commit`** (already exists, enhance it):

```bash
#!/bin/sh

# Format check
cargo fmt --all -- --check || {
    echo "❌ Code not formatted. Run: cargo fmt --all"
    exit 1
}

# Clippy
cargo clippy --all-targets --all-features -- -D warnings || {
    echo "❌ Clippy warnings found. Fix them first."
    exit 1
}

# Quick tests
cargo nextest run --workspace --no-fail-fast || {
    echo "❌ Tests failed"
    exit 1
}

# Audit (optional, can be slow)
if command -v cargo-deny &> /dev/null; then
    cargo deny check advisories || {
        echo "⚠️  Security advisories found"
        exit 1
    }
fi

echo "✅ Pre-commit checks passed"
```

### Dependency Update Policy

**Renovate or Dependabot:**

```json
// renovate.json
{
  "extends": ["config:base"],
  "rust": {
    "enabled": true,
    "rangeStrategy": "bump"
  },
  "schedule": ["before 3am on Monday"],
  "automerge": false,
  "platformAutomerge": false
}
```

---

## Risk Mitigation

### High-Risk Areas

#### 1. Async Conversion

**Risk:** Converting sync APIs to async may introduce deadlocks or race conditions

**Mitigation:**
- Use `tokio::sync::RwLock` instead of `std::sync::Mutex`
- Avoid holding locks across `.await` points
- Use structured concurrency with `tokio::task`
- Test with `tokio-console` for deadlock detection

#### 2. macOS Accessibility API

**Risk:** Accessibility permission prompts may break in new structure

**Mitigation:**
- Keep existing permission check logic
- Test on fresh macOS installation
- Document permission requirements clearly
- Graceful degradation if permission denied

#### 3. Enrichment Performance

**Risk:** Async enrichment may cause cache staleness issues

**Mitigation:**
- Keep TTL at 750ms (proven value from legacy)
- Monitor enrichment latency with metrics
- Add circuit breaker for slow enrichment
- Test with high-frequency app switching

#### 4. Event Listener Lifecycle

**Risk:** NSWorkspace observer may not clean up properly

**Mitigation:**
- Test observer cleanup thoroughly
- Use RAII pattern with Drop trait
- Verify no memory leaks with instruments
- Add integration test for start/stop cycles

### Medium-Risk Areas

#### 5. Type Mismatches

**Risk:** ActivityContext structure may differ between legacy and new

**Mitigation:**
- Compare structures side-by-side before porting
- Add compile-time assertions for field compatibility
- Write conversion tests

#### 6. Error Handling

**Risk:** Custom ActivityError may not map cleanly to PulseArcError

**Mitigation:**
- Add comprehensive error conversion tests
- Ensure all error paths propagate correctly
- Test error messages in frontend

### Low-Risk Areas

#### 7. Platform Abstraction

**Risk:** Windows/Linux providers may have different behavior

**Mitigation:**
- Use conditional compilation extensively
- Test on each platform in CI/CD
- Document platform-specific limitations

---

## Dependencies

### New Crate Dependencies

**crates/infra/Cargo.toml:**
```toml
[dependencies]
pulsearc-core = { path = "../core" }
pulsearc-shared = { path = "../shared" }
async-trait = "0.1"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"
objc = "0.2"
core-foundation = "0.9"
core-graphics = "0.23"

[target.'cfg(target_os = "windows")'.dependencies]
windows = { version = "0.52", features = ["Win32_Foundation", "Win32_UI_WindowsAndMessaging"] }
```

**crates/core/Cargo.toml:**
```toml
[dependencies]
pulsearc-shared = { path = "../shared" }
async-trait = "0.1"
tokio = { version = "1", features = ["sync", "time"] }
```

### Version Compatibility

**Tokio Runtime:**
- Ensure all crates use same tokio version (currently "1.x")
- Use `tokio::runtime::Handle::current()` for nested contexts
- Document required tokio features

**Async-trait:**
- Pin to "0.1" for stability
- Consistent usage across all traits

---

## Performance Benchmarks

### Baseline (Legacy)

**Activity Fetch (macOS, no enrichment):**
- Mean: 12ms
- P50: 10ms
- P99: 25ms

**Enrichment (Browser URL):**
- Mean: 85ms
- P50: 75ms
- P99: 180ms

**Event Latency (App Switch → UI Update):**
- Mean: 45ms
- P50: 40ms
- P99: 95ms

### Target (New Implementation)

**Same or Better:**
- Activity Fetch: ≤ 15ms (P50)
- Enrichment: ≤ 100ms (P50)
- Event Latency: ≤ 50ms (P50)

**Measurement:**
```bash
cargo bench --package pulsearc-infra --bench activity_tracking
```

**Regression Criteria:**
- If P50 increases >20%, investigate before merging
- If P99 increases >50%, investigate before merging

---

## Rollout Plan

### Phase-by-Phase Deployment

**Goal:** Incremental rollout with quick rollback capability

#### Stage 1: Development Branch

- Create `feature/tracker-refactor` branch
- Complete Phases 1-8
- All tests passing locally
- Code review with team

**Criteria:**
- [ ] All tests pass
- [ ] No clippy warnings
- [ ] Documentation complete
- [ ] Manual testing on macOS complete

#### Stage 2: Internal Testing

- Merge to `develop` branch
- Deploy to internal test devices
- Collect feedback for 2-3 days

**Criteria:**
- [ ] No crashes
- [ ] No data loss
- [ ] Performance acceptable
- [ ] No major bugs reported

#### Stage 3: Canary Release

- Merge to `main` branch
- Release to 10% of users (if applicable)
- Monitor metrics for 1 week

**Criteria:**
- [ ] Crash rate < 0.1%
- [ ] Performance within targets
- [ ] No P0 bugs reported

#### Stage 4: Full Rollout

- Increase to 100% of users
- Monitor for 2 weeks
- Mark refactor complete

**Rollback Plan:**
- If critical bugs found, revert to legacy implementation
- Keep legacy code in separate branch for 2 release cycles
- Document rollback procedure

---

## Success Criteria

### Functional Requirements

- [x] All existing tracker functionality preserved
- [ ] Activity tracking works on macOS
- [ ] Idle detection works correctly
- [ ] Browser URL enrichment works
- [ ] Office document enrichment works
- [ ] Event-driven updates work
- [ ] Pause/resume functionality works
- [ ] Windows basic tracking works
- [ ] Linux placeholder works

### Non-Functional Requirements

- [ ] Code follows new architecture patterns
- [ ] All tests pass (unit + integration)
- [ ] Test coverage ≥ 70%
- [ ] No clippy warnings
- [ ] Documentation complete and accurate
- [ ] Performance meets or exceeds legacy
- [ ] No memory leaks
- [ ] CPU usage < 5% idle

### Code Quality

- [ ] All public APIs documented
- [ ] Error handling comprehensive
- [ ] No `unwrap()` in production code
- [ ] All TODO comments resolved
- [ ] No `#[allow(clippy::...)]` without justification

---

## Timeline & Milestones

**Total Estimated Time:** 16-24 hours

| Phase | Description | Hours | Dependencies | Milestone |
|-------|-------------|-------|--------------|-----------|
| 1 | Foundation | 4-6 | None | Core traits defined |
| 2 | Idle Detection | 4-6 | Phase 1 | Idle module complete |
| 3 | OS Events | 3-4 | Phase 1 | Event abstraction done |
| 4 | macOS Provider | 5-7 | Phase 1, 3 | Full macOS support |
| 5 | Tracker Core | 4-6 | Phase 1-4 | Service layer complete |
| 6 | Integration | 2-3 | Phase 5 | Commands wired up |
| 7 | Cross-Platform | 3-4 | Phase 1 | Windows/Linux support |
| 8 | Cleanup | 2-3 | Phase 1-7 | Legacy code removed |

**Suggested Schedule (1 person, full-time):**
- **Week 1:** Phases 1-3 (Foundation, Idle, OS Events)
- **Week 2:** Phases 4-5 (macOS Provider, Tracker Core)
- **Week 3:** Phases 6-8 (Integration, Cross-Platform, Cleanup)

**Suggested Schedule (2 people, parallel work):**
- **Developer 1:** Phases 1, 2, 5, 8 (Core, Idle, Service, Cleanup)
- **Developer 2:** Phases 3, 4, 6, 7 (OS Events, macOS, Integration, Cross-Platform)
- **Timeline:** 8-12 days

---

## Appendix A: File Mapping

### Before (Legacy)

```
legacy/api/src/tracker/
├── core.rs → crates/core/src/tracking/service.rs
├── provider.rs → crates/core/src/tracking/ports.rs
├── idle/ → crates/infra/src/idle/
├── providers/macos.rs → crates/infra/src/platform/macos.rs
├── providers/dummy.rs → crates/infra/src/platform/dummy.rs
└── os_events/ → crates/infra/src/os_events/
```

### After (New Structure)

```
crates/
├── core/src/tracking/
│   ├── ports.rs (traits)
│   ├── service.rs (orchestration)
│   ├── mod.rs
│   └── README.md
├── infra/src/
│   ├── platform/
│   │   ├── macos.rs (MacOsActivityProvider)
│   │   ├── dummy.rs (Windows/Linux)
│   │   ├── mod.rs
│   │   └── README.md
│   ├── idle/
│   │   ├── detector.rs
│   │   ├── period_tracker.rs
│   │   ├── recovery.rs
│   │   ├── lock_detection.rs
│   │   ├── config.rs
│   │   ├── types.rs
│   │   └── mod.rs
│   └── os_events/
│       ├── macos.rs
│       ├── macos_ax.rs
│       ├── fallback.rs
│       ├── traits.rs
│       └── mod.rs
└── shared/src/
    ├── types.rs (ActivityContext, etc.)
    └── errors.rs (PulseArcError)
```

---

## Appendix B: API Changes

### ActivityProvider Trait

**Before (Legacy):**
```rust
pub trait ActivityProvider {
    fn fetch(&self) -> Result<ActivityContext, ActivityError>;
}
```

**After (New):**
```rust
#[async_trait]
pub trait ActivityProvider: Send + Sync {
    async fn get_activity(&self) -> Result<ActivityContext>;
    fn is_paused(&self) -> bool;
    fn pause(&mut self) -> Result<()>;
    fn resume(&mut self) -> Result<()>;
}
```

### Usage Example

**Before (Legacy):**
```rust
let provider = MacOsProvider::new(false);
let context = provider.fetch()?;
```

**After (New):**
```rust
let provider = MacOsActivityProvider::new();
let context = provider.get_activity().await?;
```

### Tauri Command

**Before:**
```rust
#[tauri::command]
pub fn get_activity(tracker: State<Tracker>) -> Result<ActivityContext, String> {
    tracker.get_activity_context()
        .map_err(|e| e.to_string())
}
```

**After:**
```rust
#[tauri::command]
pub async fn get_activity(ctx: State<'_, Arc<AppContext>>) -> Result<ActivityContext, String> {
    ctx.tracking_service
        .get_activity()
        .await
        .map_err(|e| e.to_string())
}
```

---

## Appendix C: Testing Checklist

### Pre-Refactor

- [ ] All legacy tests passing
- [ ] Manual testing baseline established
- [ ] Performance benchmarks recorded

### During Refactor

- [ ] Unit tests written for each module
- [ ] Integration tests cover cross-module flows
- [ ] Mock providers work in tests
- [ ] Error cases tested

### Post-Refactor

- [ ] All new tests passing
- [ ] Legacy tests removed
- [ ] Manual testing complete (see Phase 8)
- [ ] Performance benchmarks meet targets
- [ ] Memory profiling clean
- [ ] No regressions found

---

## Appendix D: References

- [Legacy Tracker README](../legacy/api/src/tracker/README.md)
- [CLAUDE.md](../CLAUDE.md) - Development guidelines
- [MACOS_ARCHITECTURE.md](./MACOS_ARCHITECTURE.md) - System architecture
- [Clippy Configuration](../clippy.toml) - Linting rules
- [Tokio Async Book](https://tokio.rs/tokio/tutorial) - Async patterns

---

## Questions & Decisions Log

### Q1: Should we keep synchronous enrichment option?

**Decision:** No, async-only for consistency. Background enrichment is opt-in via config.

**Rationale:** Simplifies codebase, async is more idiomatic for IO-bound operations.

### Q2: Should idle detection be in core or infra?

**Decision:** Infra - it's platform-specific implementation

**Rationale:** Core should only have traits, infra has implementations.

### Q3: Cache implementation - std::sync or tokio::sync?

**Decision:** tokio::sync::RwLock for all async code

**Rationale:** Avoid blocking tokio executor, better performance for async workloads.

### Q4: Should we keep RefresherState concept?

**Decision:** TBD - may replace with simpler task management

**Rationale:** Needs further design discussion. RefresherState may be over-engineered for async context.

---

**Last Updated:** 2025-01-30
**Next Review:** After Phase 2 completion
**Status:** 🟡 Planning → Ready to Start Phase 1