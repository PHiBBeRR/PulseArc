# ADR-001: PulseArc Architecture Overview

## Status
**Accepted** (Current Implementation)

**Last Updated:** October 2025

---

## Context

PulseArc is a macOS-native time tracking and productivity analytics application that requires:

- **Real-time activity capture** from the operating system (applications, windows, URLs)
- **Secure local data storage** with encryption for sensitive information
- **Responsive desktop UI** with native platform integration
- **Extensibility** for future integrations (SAP, calendars, web APIs)
- **High-quality codebase** adhering to enterprise standards and best practices

The architecture needed to balance:
- **Performance:** Minimal overhead on system resources
- **Security:** Encrypted storage, secure credential management
- **Maintainability:** Clean separation of concerns, testable components
- **Platform integration:** Deep macOS system integration
- **Scalability:** Support for future features without architectural refactoring

---

## Decision

PulseArc implements a **multi-layered architecture** combining:

1. **Tauri 2.0 Desktop Framework** for cross-language (Rust + TypeScript) desktop development
2. **Hexagonal Architecture (Ports & Adapters)** for the backend
3. **Domain-Driven Design (DDD)** for business logic organization
4. **Feature-Based Architecture** for the frontend
5. **Rust Cargo Workspace** for backend modularity
6. **React + TypeScript** for the user interface

---

## Architecture Overview

### System Layers

```
┌─────────────────────────────────────────────────────────────┐
│                   Frontend (React/TS)                        │
│  Features: Timer, Entries, Settings, Analytics, Timeline    │
│  Shared: IPC Client, State Management, UI Components        │
└───────────────────────────┬─────────────────────────────────┘
                            │ Tauri IPC (Commands + Events)
                            ▼
┌─────────────────────────────────────────────────────────────┐
│                    Backend (Rust)                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  API Layer: Tauri Commands, Context/DI               │  │
│  └─────────────────────┬─────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────┐  │
│  │  Core Layer: Business Logic + Port Definitions       │  │
│  └─────────────────────┬─────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────┐  │
│  │  Infrastructure Layer: Concrete Implementations      │  │
│  └─────────────────────┬─────────────────────────────────┘  │
│  ┌─────────────────────▼─────────────────────────────────┐  │
│  │  Domain Layer: Pure Business Models                  │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │  Common Layer: Shared Infrastructure                 │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

---

## Backend Architecture (Rust)

### Cargo Workspace Structure

The backend is organized as a **Cargo workspace** with 5 primary crates, each with clear responsibilities:

```
crates/
├── common/      # Cross-cutting concerns (auth, cache, security, observability)
├── domain/      # Pure domain models (zero infrastructure dependencies)
├── core/        # Business logic + port definitions (hexagonal architecture)
├── infra/       # Infrastructure implementations (database, platform APIs)
└── api/         # Tauri application entry point + command handlers
```

### 1. `pulsearc-common` (Shared Infrastructure)

**Purpose:** Cross-cutting concerns and reusable utilities

**Key Modules:**
- **auth**: OAuth 2.0 + PKCE authentication primitives
- **cache**: TTL-based caching (sync & async)
- **compliance**: Audit logging, feature flags, configuration
- **error**: Comprehensive error handling framework with classification
- **lifecycle**: Async component lifecycle management
- **observability**: Metrics, monitoring, distributed tracing
- **privacy**: Secure hashing, pattern matching, sanitization
- **resilience**: Circuit breakers, retry logic with backoff
- **security**: RBAC, keychain integration, encryption
- **storage**: Encrypted database infrastructure (SQLCipher)
- **sync**: Synchronization primitives, retry logic, queue management
- **validation**: Enterprise-grade validation framework

**Key Dependencies:**
```toml
# Security
blake3, hex, keyring, aes-gcm, argon2, zeroize

# Database
rusqlite (with SQLCipher), r2d2, r2d2_sqlite

# Caching
moka (TTL + size-based eviction)

# HTTP/OAuth
reqwest, oauth2

# Async
tokio, async-trait, futures
```

**Design Principles:**
- Pure infrastructure concerns only
- No business logic
- Highly reusable across features
- Comprehensive test coverage

---

### 2. `pulsearc-domain` (Pure Domain Layer)

**Purpose:** Pure business domain models with **zero infrastructure dependencies**

**Core Types:**

```rust
// Core activity representation
pub struct ActivityContext {
    pub app_name: String,
    pub window_title: Option<String>,
    pub url: Option<String>,
    pub document_path: Option<String>,
    pub captured_at: DateTime<Utc>,
}

// Timestamped activity snapshot
pub struct ActivitySnapshot {
    pub id: Uuid,
    pub context: ActivityContext,
    pub timestamp: DateTime<Utc>,
}

// Classified work period
pub struct TimeEntry {
    pub id: Uuid,
    pub project_id: Option<Uuid>,
    pub description: String,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub wbs_code: Option<String>,
}

// Application configuration
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub sync: SyncConfig,
    pub tracking: TrackingConfig,
}
```

**Error Types:**
```rust
#[derive(Debug, thiserror::Error)]
pub enum PulseArcError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Platform error: {0}")]
    Platform(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Security error: {0}")]
    Security(String),

    // ... etc
}

pub type Result<T> = std::result::Result<T, PulseArcError>;
```

**Philosophy:**
- No dependencies on other PulseArc crates
- Only foundational external crates (serde, chrono, uuid, thiserror)
- Can be understood and tested in complete isolation
- Forms the ubiquitous language of the system

---

### 3. `pulsearc-core` (Business Logic Layer)

**Purpose:** Pure business logic with **port definitions** (no implementations)

**Architecture Pattern:** **Hexagonal Architecture (Ports & Adapters)**

#### Tracking Module

**Ports (Trait Definitions):**

```rust
#[async_trait]
pub trait ActivityProvider: Send + Sync {
    async fn capture_activity(&self) -> Result<ActivitySnapshot>;
    async fn pause(&self) -> Result<()>;
    async fn resume(&self) -> Result<()>;
    fn is_paused(&self) -> bool;
}

#[async_trait]
pub trait ActivityRepository: Send + Sync {
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()>;
    async fn find_by_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>
    ) -> Result<Vec<ActivitySnapshot>>;
}

#[async_trait]
pub trait ActivityEnricher: Send + Sync {
    async fn enrich(&self, context: ActivityContext) -> Result<ActivityContext>;
}
```

**Service Implementation:**

```rust
pub struct TrackingService {
    provider: Arc<dyn ActivityProvider>,
    repository: Arc<dyn ActivityRepository>,
    enrichers: Vec<Arc<dyn ActivityEnricher>>,
}

impl TrackingService {
    pub async fn capture_and_save(&self) -> Result<ActivitySnapshot> {
        // 1. Capture from OS
        let mut snapshot = self.provider.capture_activity().await?;

        // 2. Enrich with additional context
        for enricher in &self.enrichers {
            snapshot.context = enricher.enrich(snapshot.context).await?;
        }

        // 3. Persist
        self.repository.save(&snapshot).await?;

        Ok(snapshot)
    }
}
```

#### Classification Module

**Ports:**

```rust
#[async_trait]
pub trait Classifier: Send + Sync {
    async fn classify(&self, snapshots: Vec<ActivitySnapshot>) -> Result<Vec<TimeEntry>>;
}

#[async_trait]
pub trait TimeEntryRepository: Send + Sync {
    async fn save(&self, entry: &TimeEntry) -> Result<()>;
    async fn find_by_date(&self, date: NaiveDate) -> Result<Vec<TimeEntry>>;
    async fn update(&self, entry: &TimeEntry) -> Result<()>;
    async fn delete(&self, id: Uuid) -> Result<()>;
}
```

**Service:**

```rust
pub struct ClassificationService {
    classifier: Arc<dyn Classifier>,
    repository: Arc<dyn TimeEntryRepository>,
}

impl ClassificationService {
    pub async fn classify_activities(
        &self,
        snapshots: Vec<ActivitySnapshot>
    ) -> Result<Vec<TimeEntry>> {
        let entries = self.classifier.classify(snapshots).await?;

        for entry in &entries {
            self.repository.save(entry).await?;
        }

        Ok(entries)
    }
}
```

**Benefits of Hexagonal Architecture:**
- **Testability:** Core logic tested with mock implementations
- **Flexibility:** Swap implementations without changing business logic
- **Platform Independence:** Core is platform-agnostic
- **Clear boundaries:** Port traits define explicit contracts

**Dependencies:**
- Only depends on `pulsearc-common` and `pulsearc-domain`
- No infrastructure concerns
- No platform-specific code

---

### 4. `pulsearc-infra` (Infrastructure Layer)

**Purpose:** Concrete implementations of core ports

**Key Implementations:**

#### Database (`database/`)

```rust
pub struct DbManager {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}

impl DbManager {
    pub fn new(config: &DatabaseConfig) -> Result<Self> {
        // Initialize SQLite with SQLCipher encryption
        let manager = r2d2_sqlite::SqliteConnectionManager::file(&config.path)
            .with_init(|conn| {
                // Set encryption key if provided
                if let Some(key) = &config.encryption_key {
                    conn.pragma_update(None, "key", key)?;
                }
                conn.pragma_update(None, "journal_mode", "WAL")?;
                Ok(())
            });

        let pool = r2d2::Pool::builder()
            .max_size(config.pool_size)
            .build(manager)?;

        Ok(Self { pool })
    }

    fn run_migrations(&self) -> Result<()> {
        // Embedded SQL migrations
        let conn = self.pool.get()?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS activity_snapshots (
                id TEXT PRIMARY KEY,
                app_name TEXT NOT NULL,
                window_title TEXT,
                url TEXT,
                document_path TEXT,
                timestamp TEXT NOT NULL
            )",
            [],
        )?;

        // ... more migrations
        Ok(())
    }
}
```

**Repository Implementations:**

```rust
pub struct SqliteActivityRepository {
    db: Arc<DbManager>,
}

#[async_trait]
impl ActivityRepository for SqliteActivityRepository {
    async fn save(&self, snapshot: &ActivitySnapshot) -> Result<()> {
        let conn = self.db.pool.get()?;
        conn.execute(
            "INSERT INTO activity_snapshots
             (id, app_name, window_title, url, document_path, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                snapshot.id.to_string(),
                snapshot.context.app_name,
                snapshot.context.window_title,
                snapshot.context.url,
                snapshot.context.document_path,
                snapshot.timestamp.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    // ... other methods
}
```

#### Platform (`platform/`)

**macOS Activity Provider:**

```rust
use objc2::runtime::NSObject;
use objc2_foundation::{NSString, NSArray};
use objc2_app_kit::NSWorkspace;

pub struct MacOsActivityProvider {
    paused: AtomicBool,
}

#[async_trait]
impl ActivityProvider for MacOsActivityProvider {
    async fn capture_activity(&self) -> Result<ActivitySnapshot> {
        if self.is_paused() {
            return Err(PulseArcError::Platform("Tracking paused".into()));
        }

        // Get active application via macOS APIs
        let workspace = unsafe { NSWorkspace::sharedWorkspace() };
        let active_app = unsafe { workspace.frontmostApplication() };

        let app_name = unsafe {
            active_app.localizedName()
                .to_string()
        };

        // Capture window title via Accessibility API
        let window_title = self.capture_window_title()?;

        // Capture URL from browser (if applicable)
        let url = self.capture_browser_url(&app_name)?;

        Ok(ActivitySnapshot {
            id: Uuid::new_v7(),
            context: ActivityContext {
                app_name,
                window_title,
                url,
                document_path: None,
                captured_at: Utc::now(),
            },
            timestamp: Utc::now(),
        })
    }

    // ... pause/resume implementation
}
```

**Platform-Specific Dependencies:**
```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2 = { workspace = true }
objc2-foundation = { workspace = true }
objc2-app-kit = { workspace = true }
cocoa = "0.25"
core-foundation = "0.10"
core-graphics = "0.24"
io-kit-sys = "0.4"
```

#### Key Manager (`key_manager.rs`)

```rust
use keyring::Entry;

pub struct KeyManager {
    service_name: String,
}

impl KeyManager {
    pub fn new(service_name: impl Into<String>) -> Self {
        Self {
            service_name: service_name.into(),
        }
    }

    pub fn store_key(&self, key_name: &str, value: &str) -> Result<()> {
        let entry = Entry::new(&self.service_name, key_name)?;
        entry.set_password(value)?;
        Ok(())
    }

    pub fn retrieve_key(&self, key_name: &str) -> Result<String> {
        let entry = Entry::new(&self.service_name, key_name)?;
        let password = entry.get_password()?;
        Ok(password)
    }

    pub fn delete_key(&self, key_name: &str) -> Result<()> {
        let entry = Entry::new(&self.service_name, key_name)?;
        entry.delete_credential()?;
        Ok(())
    }
}
```

#### Instance Lock (`instance_lock.rs`)

```rust
use std::fs;
use std::io::Write;

pub struct InstanceLock {
    lock_file: PathBuf,
}

impl InstanceLock {
    pub fn acquire(app_name: &str) -> Result<Self> {
        let lock_file = std::env::temp_dir().join(format!("{}.lock", app_name));

        if lock_file.exists() {
            // Check if process is still running
            let pid_str = fs::read_to_string(&lock_file)?;
            let pid: i32 = pid_str.trim().parse()?;

            if Self::is_process_running(pid) {
                return Err(PulseArcError::Internal(
                    format!("Another instance is already running (PID: {})", pid)
                ));
            }

            // Stale lock file, remove it
            fs::remove_file(&lock_file)?;
        }

        // Create lock file with current PID
        let mut file = fs::File::create(&lock_file)?;
        writeln!(file, "{}", std::process::id())?;

        Ok(Self { lock_file })
    }

    fn is_process_running(pid: i32) -> bool {
        // Platform-specific process check
        #[cfg(target_os = "macos")]
        {
            use libc::{kill, ESRCH};
            unsafe {
                kill(pid, 0) == 0 || *libc::__error() != ESRCH
            }
        }
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.lock_file);
    }
}
```

---

### 5. `pulsearc` (API/Application Layer)

**Purpose:** Tauri application entry point and command handlers

**Main Entry Point (`main.rs`):**

```rust
use tauri::{Manager, State};
use std::sync::Arc;

mod commands;
mod context;

use context::AppContext;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize logging
    env_logger::init();

    // Initialize application context (DI container)
    let app_context = Arc::new(AppContext::new().await?);

    tauri::Builder::default()
        .manage(app_context)
        .setup(|app| {
            #[cfg(target_os = "macos")]
            {
                // Apply macOS-specific window effects
                use cocoa::appkit::{NSWindow, NSWindowStyleMask};
                let window = app.get_window("main").unwrap();
                let ns_window = window.ns_window().unwrap() as cocoa::base::id;

                unsafe {
                    // Enable native blur/vibrancy
                    ns_window.setTitlebarAppearsTransparent_(cocoa::base::YES);
                    ns_window.setStyleMask_(
                        NSWindowStyleMask::NSFullSizeContentViewWindowMask
                    );
                }
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::tracking::get_activity,
            commands::tracking::pause_tracker,
            commands::tracking::resume_tracker,
            commands::projects::get_user_projects,
            commands::suggestions::get_dismissed_suggestions,
            commands::calendar::get_calendar_events_for_timeline,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}
```

**Dependency Injection Container (`context/mod.rs`):**

```rust
use std::sync::Arc;
use pulsearc_domain::config::AppConfig;
use pulsearc_core::tracking::TrackingService;
use pulsearc_infra::database::DbManager;
use pulsearc_infra::platform::MacOsActivityProvider;
use pulsearc_infra::database::SqliteActivityRepository;

pub struct AppContext {
    pub config: AppConfig,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    _instance_lock: InstanceLock,
}

impl AppContext {
    pub async fn new() -> Result<Self> {
        // Load configuration
        let config = AppConfig::load_from_env()?;

        // Acquire instance lock
        let instance_lock = InstanceLock::acquire("pulsearc")?;

        // Initialize database
        let db = Arc::new(DbManager::new(&config.database)?);
        db.run_migrations()?;

        // Wire up hexagonal architecture
        let activity_provider = Arc::new(MacOsActivityProvider::new());
        let activity_repository = Arc::new(SqliteActivityRepository::new(Arc::clone(&db)));

        let tracking_service = Arc::new(TrackingService::new(
            activity_provider,
            activity_repository,
            vec![], // enrichers
        ));

        Ok(Self {
            config,
            db,
            tracking_service,
            _instance_lock: instance_lock,
        })
    }
}
```

**Tauri Commands (`commands/tracking.rs`):**

```rust
use tauri::State;
use std::sync::Arc;
use crate::context::AppContext;
use pulsearc_domain::{ActivitySnapshot, Result};

#[tauri::command]
pub async fn get_activity(
    ctx: State<'_, Arc<AppContext>>
) -> Result<ActivitySnapshot> {
    ctx.tracking_service.capture_and_save().await
}

#[tauri::command]
pub async fn pause_tracker(
    ctx: State<'_, Arc<AppContext>>
) -> Result<()> {
    ctx.tracking_service.pause().await
}

#[tauri::command]
pub async fn resume_tracker(
    ctx: State<'_, Arc<AppContext>>
) -> Result<()> {
    ctx.tracking_service.resume().await
}
```

**Tauri Configuration (`tauri.conf.json`):**

```json
{
  "productName": "PulseArc",
  "version": "0.1.0",
  "build": {
    "beforeDevCommand": "pnpm dev",
    "devUrl": "http://localhost:5173",
    "frontendDist": "../../frontend/dist"
  },
  "app": {
    "windows": [
      {
        "title": "PulseArc",
        "width": 420,
        "height": 300,
        "resizable": false,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true
      }
    ],
    "security": {
      "csp": null
    },
    "macOSPrivateApi": true
  }
}
```

---

## Frontend Architecture (React/TypeScript)

### Structure

```
frontend/
├── App.tsx                 # Main entry point, view routing
├── main.tsx               # React bootstrap
├── globals.css            # TailwindCSS globals
├── components/            # Shared UI components
│   └── ui/               # shadcn/ui components (40+)
├── features/             # Feature modules (see below)
└── shared/               # Shared infrastructure
    ├── components/       # Reusable UI components
    ├── services/         # Core services (IPC, audio, cache)
    ├── state/            # State derivation
    ├── events/           # Event bus
    ├── hooks/            # Custom hooks
    ├── types/            # Shared types
    ├── utils/            # Utilities
    └── test/             # Test utilities
```

### Feature-Based Organization

Each feature is a **vertical slice** with all layers:

```
feature/
├── components/           # React components
├── services/            # Backend integration (Tauri commands)
├── stores/              # State management (Zustand)
├── hooks/               # Custom React hooks
├── types/               # TypeScript type definitions
└── index.ts             # Barrel export (public API)
```

**Benefits:**
- **High cohesion:** Related code stays together
- **Low coupling:** Features are independent
- **Scalability:** Add features without touching existing code
- **Team autonomy:** Teams can own entire features
- **Easier testing:** Test entire feature in isolation

### Core Features

#### 1. **timer** - Main Timer Widget

**Components:**
- `MainTimer` - Primary UI (500+ lines)
- `ActivityBreakdownTooltip` - Visual activity breakdown
- `OutboxStatus` - Sync status indicator
- `FilterSortModal` / `FilterSortPopover` - Entry filtering
- `SuggestedEntries` - AI-powered suggestions
- `WbsAutocomplete` - Work breakdown structure autocomplete

**Services:**
- `wbsUsageService` - WBS code usage tracking

**State:**
- Timer state (running/paused/stopped)
- Current activity
- Suggestions

**Testing:**
- 7+ test files including integration and SAP tests

#### 2. **time-entry** - Time Entry Management

**Components:**
- `EntriesView` / `EntriesPanel` - Entry list views
- `DayView` - Day-based entry view
- `ClassifyEntryModal` - Activity classification
- `EditEntryModal` / `SaveEntryModal` - Entry CRUD
- `CompactEntries` / `CompactQuickEntry` - Compact views
- `IdleTimeReview` - Idle time classification
- `DismissFeedbackModal` - Feedback collection

**Services:**
- `entryService` - Entry CRUD operations

**Stores:**
- `entryStore` (Zustand) - Entry state management

#### 3. **timeline** - Visual Activity Timeline

**Components:**
- `TimelineView` - Main timeline view
- `TimelineDayView` - Day-based timeline
- `MarqueeText` - Scrolling text for long titles

**Services:**
- `timelineService` - Timeline data fetching (with calendar integration)

**Features:**
- Zoomable timeline
- Calendar event overlay
- Activity clustering

#### 4. **analytics** - Productivity Analytics

**Components:**
- `AnalyticsView` - Main analytics dashboard
- `IdlePeriodDetail` - Idle time breakdown
- `IdleTimeChart` - Visualization

**Services:**
- `analyticsService` - Analytics data aggregation
- `idleAnalyticsService` - Idle time analysis

**Metrics:**
- Productive vs idle time
- Application breakdown
- Time of day patterns

#### 5. **settings** - Application Configuration

**Components:**
- `SettingsView` / `SettingsPanel` - Main settings UI
- `MainApiSettings` - Main API configuration
- `SapSettings` - SAP integration settings
- `IdleDetectionSettings` / `IdleSettings` - Idle detection config
- `CalendarProviderCard` - Calendar provider setup (Google, Microsoft)
- `SyncStatus` - Sync status display
- `AccountMenu` - User account management

**Services:**
- `WebApiService` - Main API integration
- `sapService` - SAP time entry sync
- `calendarService` - Calendar provider integration
- `settingsService` - Settings persistence
- `adminService` - Admin operations

**Testing:**
- 14+ test files (highest coverage in codebase)

#### 6. **project** - Project Management

**Components:**
- `QuickProjectSwitcher` - Quick project switching

**Services:**
- `projectService` - Project CRUD

**Stores:**
- `projectStore` (Zustand) - Project state

**Utilities:**
- `projectColors` - Project color management

**Types:**
- `Project`, `RecentProject`, `ProjectColor`

#### 7. **activity-tracker** - AI Activity Capture

**Components:**
- `ActivityTrackerView` - Main tracker UI
- `SuggestionChip` - Suggestion display

**Hooks:**
- `useSuggestionManager` - Suggestion lifecycle management

**Features:**
- AI-powered activity suggestions
- Smart time block proposals
- Context-aware categorization

#### 8. **build-my-day** - Day Planning

**Components:**
- `BuildMyDayView` - Day planning interface
- `BuildMyDayFilterPopover` - Filter controls

**Features:**
- Calendar integration
- Time block scheduling
- AI-powered planning assistance

#### 9. **idle-detection** - Idle Time Tracking

**Components:**
- `IdleDetectionModal` - Idle period classification

**Services:**
- `idleDetectionService` - Idle detection logic

**Features:**
- Automatic idle detection
- Configurable thresholds
- Classification interface

### Shared Infrastructure

#### IPC Client (`shared/services/ipc/`)

**Purpose:** Abstraction over Tauri window management and IPC

```typescript
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';

export class TauriAPI {
    private static window = getCurrentWindow();

    static async setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
        await this.window.setAlwaysOnTop(alwaysOnTop);
    }

    static async setSize(width: number, height: number): Promise<void> {
        await this.window.setSize({ width, height });
    }

    static async center(): Promise<void> {
        await this.window.center();
    }

    static async invokeCommand<T>(
        command: string,
        args?: Record<string, unknown>
    ): Promise<T> {
        return invoke<T>(command, args);
    }
}

// Usage in services:
export async function getActivity(): Promise<ActivitySnapshot> {
    return TauriAPI.invokeCommand('get_activity');
}
```

#### State Management

**Pattern:** Zustand for global state, React hooks for local state

**Example Store:**

```typescript
import { create } from 'zustand';

interface ProjectStore {
    projects: Project[];
    currentProject: Project | null;
    setProjects: (projects: Project[]) => void;
    setCurrentProject: (project: Project | null) => void;
}

export const useProjectStore = create<ProjectStore>((set) => ({
    projects: [],
    currentProject: null,
    setProjects: (projects) => set({ projects }),
    setCurrentProject: (project) => set({ currentProject: project }),
}));
```

#### Event System (`shared/events/`)

**Purpose:** Decoupled event communication

```typescript
export class TimerEvents {
    private static listeners = new Map<string, Set<Function>>();

    static on(event: string, callback: Function): () => void {
        if (!this.listeners.has(event)) {
            this.listeners.set(event, new Set());
        }
        this.listeners.get(event)!.add(callback);

        // Return unsubscribe function
        return () => {
            this.listeners.get(event)?.delete(callback);
        };
    }

    static emit(event: string, data?: unknown): void {
        this.listeners.get(event)?.forEach((callback) => {
            callback(data);
        });
    }
}

// Usage:
TimerEvents.emit('timer:started', { timestamp: Date.now() });

const unsubscribe = TimerEvents.on('timer:started', (data) => {
    console.log('Timer started:', data);
});
```

#### Error Handling (`shared/services/errorToastService.ts`)

**Purpose:** Centralized error display

```typescript
import { toast } from 'sonner';

export class ErrorToastService {
    static show(error: unknown, context?: string): void {
        const message = this.extractMessage(error);
        const title = context ? `${context} failed` : 'Error';

        toast.error(title, {
            description: message,
            duration: 5000,
        });
    }

    private static extractMessage(error: unknown): string {
        if (error instanceof Error) {
            return error.message;
        }
        if (typeof error === 'string') {
            return error;
        }
        return 'An unexpected error occurred';
    }
}
```

#### Audio Service (`shared/services/audio/`)

**Purpose:** Timer sound effects

```typescript
export class AudioService {
    private audio: HTMLAudioElement | null = null;

    async playTimerStart(): Promise<void> {
        this.audio = new Audio('/sounds/timer-start.mp3');
        await this.audio.play();
    }

    async playTimerStop(): Promise<void> {
        this.audio = new Audio('/sounds/timer-stop.mp3');
        await this.audio.play();
    }
}
```

---

## Integration Points

### 1. Tauri IPC (Frontend ↔ Backend)

**Mechanism:** Command-based RPC

**Flow:**
```
Frontend (TS)          Tauri IPC           Backend (Rust)
     │                     │                     │
     │  invoke('cmd')      │                     │
     ├────────────────────>│                     │
     │                     │  #[tauri::command]  │
     │                     ├────────────────────>│
     │                     │                     │
     │                     │  Result<T>          │
     │                     │<────────────────────┤
     │  Promise<T>         │                     │
     │<────────────────────┤                     │
```

**Example:**

```typescript
// Frontend
const activity = await invoke<ActivitySnapshot>('get_activity');

// Backend
#[tauri::command]
async fn get_activity(
    ctx: State<'_, Arc<AppContext>>
) -> Result<ActivitySnapshot> {
    ctx.tracking_service.capture_and_save().await
}
```

**All Commands:**
- `get_activity` - Capture current activity
- `pause_tracker` / `resume_tracker` - Control tracking
- `get_user_projects` - Fetch projects
- `get_dismissed_suggestions` / `get_proposed_blocks` - Suggestions
- `get_calendar_events_for_timeline` - Calendar integration
- `get_outbox_status` - Sync status
- `animate_window_resize` - Native window animation
- `send_db_metrics_to_datadog` / `send_activity_metrics_to_datadog` - Metrics

### 2. Event System (Backend → Frontend)

**Tauri Events:** Pub/sub for backend-initiated communication

```rust
// Backend (emit event)
app.emit("initialization-status", "ready")?;

// Frontend (listen)
import { listen } from '@tauri-apps/api/event';

const unlisten = await listen<string>('initialization-status', (event) => {
    console.log('Status:', event.payload); // "ready"
});
```

**Key Events:**
- `initialization-status` - Backend readiness (initializing → ready → error)
- `cached-data-loaded` - Cached data available for instant display
- System tray events:
  - `start-timer` / `pause-timer` / `stop-timer`
  - `show-window` / `hide-window`
  - `open-ai-entry`

### 3. Window Management

**Dynamic Resizing:** Frontend controls window size based on view

```typescript
// Window size configurations
const WINDOW_SIZES = {
    timer: { width: 420, height: 300 },
    entries_day: { width: 680, height: 620 },
    entries_week: { width: 790, height: 410 },
    settings: { width: 580, height: 450 },
    analytics: { width: 580, height: 1025 },
    timeline_day: { width: 680, height: 720 },
    timeline_week: { width: 1450, height: 720 },
    build_my_day: { width: 680, height: 620 },
};

// Apply size
await TauriAPI.setSize(WINDOW_SIZES.timer.width, WINDOW_SIZES.timer.height);
await TauriAPI.setResizable(false);
await TauriAPI.center();
```

**Native Animations:**

```typescript
// Smooth native animation (uses CoreAnimation on macOS)
await invoke('animate_window_resize', {
    width: 680,
    height: 620,
    duration: 0.3,
});
```

### 4. Database

**Backend:** SQLite with SQLCipher encryption

```rust
// Encryption key from environment
let encryption_key = std::env::var("DATABASE_ENCRYPTION_KEY")
    .unwrap_or_else(|_| "default-dev-key".to_string());

// Initialize with encryption
conn.pragma_update(None, "key", &encryption_key)?;
conn.pragma_update(None, "journal_mode", "WAL")?;
```

**Schema Migration:**

```rust
impl DbManager {
    pub fn run_migrations(&self) -> Result<()> {
        let conn = self.pool.get()?;

        // Version tracking
        conn.execute(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER PRIMARY KEY,
                applied_at TEXT NOT NULL
            )",
            [],
        )?;

        // Apply migrations incrementally
        let current_version = self.get_schema_version(&conn)?;

        for migration in MIGRATIONS.iter().skip(current_version) {
            conn.execute_batch(migration.sql)?;
            self.set_schema_version(&conn, migration.version)?;
        }

        Ok(())
    }
}
```

### 5. State Synchronization

**Pattern:** Frontend periodically fetches + backend emits events

**Initialization Flow:**

```
Backend Startup:
1. Load cached data from DB
2. Emit "cached-data-loaded" event
3. Initialize services (async)
4. Emit "initialization-status: ready"

Frontend Startup:
1. Display loading spinner
2. Listen for "cached-data-loaded" → display cached data instantly
3. Listen for "initialization-status: ready" → fetch live data
4. Start periodic refresh (e.g., every 30s)
```

**Periodic Refresh:**

```typescript
useEffect(() => {
    const interval = setInterval(async () => {
        const activity = await getActivity();
        setCurrentActivity(activity);
    }, 30000); // 30 seconds

    return () => clearInterval(interval);
}, []);
```

---

## Technology Stack

### Backend (Rust)

| Category | Technologies |
|----------|-------------|
| **Framework** | Tauri 2.9, Tokio 1.x (multi-thread) |
| **Language** | Rust 1.77 (stable, pinned) |
| **Database** | rusqlite 0.37 + SQLCipher (encrypted) |
| **Connection Pool** | r2d2 0.8 |
| **Security** | blake3, keyring, aes-gcm, argon2, zeroize |
| **HTTP/OAuth** | reqwest 0.12 (rustls), oauth2 5.0, axum 0.8 |
| **Observability** | tracing 0.1, metrics 0.23, prometheus 0.14 |
| **Serialization** | serde 1.0, serde_json 1.0 |
| **Time** | chrono 0.4, chrono-tz 0.10 |
| **Identifiers** | uuid 1.0 (v4, v7) |
| **Caching** | moka 0.12 (TTL, LRU) |
| **Platform (macOS)** | objc2, cocoa, core-foundation, io-kit-sys |
| **Error Handling** | thiserror 2.0, anyhow 1.0 |

### Frontend (React/TypeScript)

| Category | Technologies |
|----------|-------------|
| **Framework** | React 19.2, TypeScript 5.9 |
| **Build Tool** | Vite 7.1 |
| **UI Components** | Radix UI (40+ components), shadcn/ui |
| **Styling** | TailwindCSS 4.1, tailwindcss-animate |
| **Animation** | Framer Motion 12.x |
| **Icons** | Lucide React 0.548 |
| **State** | Zustand 5.0, React Hook Form 7.65 |
| **Tauri** | @tauri-apps/api 2.9 |
| **Charts** | Recharts 3.3 |
| **Calendar** | react-day-picker 9.11 |
| **Dates** | date-fns 4.1 |
| **Notifications** | sonner 2.0 |
| **Testing** | Vitest 4.0, @testing-library/react 16.3 |
| **Linting** | ESLint 9.38, Prettier 3.6 |
| **Git Hooks** | Husky 9.1, lint-staged 16.2 |

### Development Tools

| Category | Tools |
|----------|-------|
| **Rust** | cargo-audit, cargo-deny, cargo +nightly fmt |
| **Frontend** | pnpm (package manager), Prisma 6.17 (dev DB) |
| **Automation** | Makefile (45+ commands), xtask (Rust CLI) |
| **CI/CD** | GitHub Actions (macOS self-hosted runner) |
| **Version Control** | Git, Conventional Commits |

---

## Design Principles & Standards

### 1. Rust Standards (CLAUDE.md)

**Toolchain:**
- Rust 1.77 (stable, pinned by `rust-toolchain.toml`)
- Nightly rustfmt for formatting

**Logging:**
- `tracing` exclusively (no `println!`, no `log` macros)
- Structured logging with fields
- JSON output in production

**Error Handling:**
- `thiserror` in libraries
- `anyhow` at application boundaries
- NO `unwrap()`, `expect()`, `panic!()` (except tests)
- Explicit error propagation

**Async:**
- Tokio multi-thread runtime
- No blocking in async contexts
- Use `spawn_blocking` for CPU-heavy work
- Timeouts and cancellation for external calls

**Lints:**
- `cargo clippy -- -D warnings -D clippy::all -D clippy::pedantic -D clippy::nursery`
- `unsafe` denied by default

**Testing:**
- Unit + integration tests
- Deterministic (no network, clock, or randomness)
- `#[tokio::test(flavor = "multi_thread")]` for async tests

### 2. Dependency Policy

**Workspace Dependencies:**
- Centralized in root `Cargo.toml`
- Crates use `.workspace = true`

**Supply Chain Security:**
- `cargo deny check` required
- `cargo audit` required
- Licenses allow-listed in `deny.toml`
- No wildcards (`"*"`)
- No yanked crates

### 3. Git Hygiene

**Commits:**
- Conventional Commits (`feat:`, `fix:`, `perf:`, `refactor:`)
- Clear, descriptive messages

**PRs:**
- Small, focused changes
- Risk assessment + rollback plan
- "How I tested this" section

**Hooks:**
- Pre-commit: formatting, linting
- Pre-push: tests (optional)

### 4. CI Pipeline

**Required Checks:**
1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test --workspace`
4. `cargo deny check`
5. `cargo audit`

**Quick Local Check:**
```bash
cargo ci  # or: cargo xtask ci
```

---

## Consequences

### Positive

1. **Maintainability**
   - Clean separation of concerns
   - Testable components (hexagonal architecture)
   - Clear dependency flow (layered architecture)
   - Feature-based frontend (high cohesion, low coupling)

2. **Security**
   - Encrypted database (SQLCipher)
   - Secure credential storage (system keychain)
   - No secrets in code
   - Supply chain auditing (cargo-deny, cargo-audit)
   - Strict lints (no unsafe, no unwrap, no panics)

3. **Performance**
   - Rust backend (zero-cost abstractions, memory safety)
   - Async I/O (Tokio)
   - Connection pooling (r2d2)
   - Efficient caching (moka with TTL)
   - Minimal frontend bundle (Vite tree-shaking)

4. **Platform Integration**
   - Native macOS APIs (Accessibility, Keychain)
   - Native window effects (blur, vibrancy)
   - System tray integration
   - Native animations (CoreAnimation)

5. **Developer Experience**
   - Single command CI (`cargo ci`)
   - Comprehensive tooling (Makefile, xtask)
   - Fast builds (incremental compilation, Vite HMR)
   - Strong type safety (Rust + TypeScript)
   - Excellent IDE support (rust-analyzer, TypeScript LSP)

6. **Testability**
   - Hexagonal architecture enables easy mocking
   - Feature-based organization isolates test scope
   - Comprehensive test utilities (shared/test/)
   - High coverage in critical paths (settings, timer)

### Negative

1. **Complexity**
   - Multi-layer architecture requires understanding of patterns
   - Onboarding time for new developers
   - More boilerplate (trait definitions, implementations)

2. **Platform Lock-in**
   - macOS-only (no Linux/Windows support)
   - Heavy reliance on macOS-specific APIs
   - Porting would require significant refactoring

3. **Build Times**
   - Rust compilation can be slow (incremental builds help)
   - Large dependency tree (Tauri, Tokio, etc.)

4. **Learning Curve**
   - Rust ownership/borrowing model
   - Async programming (tokio)
   - Hexagonal architecture pattern
   - Tauri IPC model

### Mitigations

1. **Documentation**
   - Comprehensive ADRs (like this document)
   - Inline code comments
   - Examples in tests
   - Architecture diagrams

2. **Tooling**
   - Automated CI pipeline
   - Developer CLI tools (xtask, Makefile)
   - Pre-commit hooks

3. **Training**
   - Onboarding guides
   - Architecture walkthroughs
   - Code review guidelines

4. **Incremental Adoption**
   - Start with simple features
   - Gradually introduce advanced patterns
   - Pair programming for complex areas

---

## Future Considerations

### 1. Cross-Platform Support

**Challenge:** Current architecture is macOS-only

**Options:**
- **Option A:** Abstract platform layer further
  - Define `PlatformProvider` trait
  - Implement `WindowsActivityProvider`, `LinuxActivityProvider`
  - Conditional compilation (`#[cfg(target_os = "...")]`)

- **Option B:** Use cross-platform libraries
  - Replace Accessibility API with [accesskit](https://github.com/AccessKit/accesskit)
  - Use [rdev](https://github.com/Narsil/rdev) for input monitoring

**Recommendation:** Option A for control, Option B for speed

### 2. Cloud Sync

**Challenge:** Currently local-only storage

**Design:**
- Add `SyncService` in core
- Define `CloudSyncProvider` trait
- Implement providers (S3, Azure, custom backend)
- Conflict resolution strategy (last-write-wins, CRDTs)
- End-to-end encryption

**Architecture Impact:**
- New crate: `pulsearc-sync`
- Backend API (axum + tonic for gRPC)
- Frontend sync status UI

### 3. Plugin System

**Challenge:** Hard to extend without modifying core

**Design:**
- WebAssembly (WASM) plugins
- Plugin manifest (`plugin.toml`)
- Sandboxed execution
- Plugin API via trait objects

**Example:**
```rust
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    fn on_activity_captured(&self, snapshot: &ActivitySnapshot) -> Result<()>;
    fn on_time_entry_saved(&self, entry: &TimeEntry) -> Result<()>;
}
```

### 4. Distributed Tracing

**Challenge:** Limited observability in production

**Design:**
- OpenTelemetry integration
- Jaeger/Tempo backend
- Span correlation (frontend → backend)
- Performance monitoring

**Implementation:**
```rust
use tracing_opentelemetry::OpenTelemetryLayer;
use opentelemetry::global;

let tracer = global::tracer("pulsearc");
let telemetry = OpenTelemetryLayer::new(tracer);
```

### 5. Offline-First Sync

**Challenge:** Network failures should not block usage

**Design:**
- Outbox pattern (queue pending changes)
- Background sync worker
- Conflict-free replicated data types (CRDTs)
- Progressive sync (prioritize recent data)

**Architecture:**
```
Client (SQLite)                    Server (PostgreSQL)
       │                                  │
       │  1. Write to local DB            │
       ├───────────────────>              │
       │  2. Queue in outbox              │
       ├───────────────────>              │
       │                                  │
       │  3. Background: Sync outbox      │
       ├─────────────────────────────────>│
       │                                  │
       │  4. Receive remote changes       │
       │<─────────────────────────────────┤
       │  5. Merge with local DB          │
       ├───────────────────>              │
```

### 6. Real-Time Collaboration

**Challenge:** Multiple users working on shared projects

**Design:**
- WebSocket connection (tokio-tungstenite)
- Operational Transformation (OT) or CRDTs
- Presence indicators
- Live cursors/selections

**Tech Stack:**
- `yrs` (Yjs CRDT implementation in Rust)
- `y-websocket` for sync protocol

---

## Related Documents

- [MACOS_ARCHITECTURE.md](../MACOS_ARCHITECTURE.md) - macOS-specific implementation details
- [TRACKER_REFACTOR_PLAN.md](../TRACKER_REFACTOR_PLAN.md) - Tracker refactoring plan
- [FILE_MAPPING.md](../FILE_MAPPING.md) - File structure and mappings
- [CLAUDE.md](../../CLAUDE.md) - Development standards and rules

---

## Revision History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| 1.0 | 2025-10-30 | Lewis Catapang | Initial comprehensive architecture document |

---

## Appendix A: Glossary

| Term | Definition |
|------|------------|
| **Hexagonal Architecture** | Architectural pattern isolating business logic from infrastructure via ports (interfaces) and adapters (implementations) |
| **Port** | Interface defining a contract (trait in Rust) |
| **Adapter** | Concrete implementation of a port |
| **DDD** | Domain-Driven Design - approach focusing on core domain and domain logic |
| **DI** | Dependency Injection - pattern for providing dependencies |
| **IPC** | Inter-Process Communication - mechanism for frontend-backend communication in Tauri |
| **CRDT** | Conflict-Free Replicated Data Type - data structure for eventual consistency |
| **WAL** | Write-Ahead Logging - database journaling mode for SQLite |
| **PKCE** | Proof Key for Code Exchange - OAuth 2.0 extension for public clients |

## Appendix B: Key File Locations

| Category | Path |
|----------|------|
| **Workspace Config** | `/Cargo.toml` |
| **Tauri Config** | `/crates/api/tauri.conf.json` |
| **Frontend Entry** | `/frontend/main.tsx` |
| **Backend Entry** | `/crates/api/src/main.rs` |
| **DI Container** | `/crates/api/src/context/mod.rs` |
| **Domain Types** | `/crates/domain/src/types.rs` |
| **Core Services** | `/crates/core/src/{tracking,classification}/service.rs` |
| **Database Manager** | `/crates/infra/src/database/manager.rs` |
| **macOS Provider** | `/crates/infra/src/platform/macos.rs` |
| **IPC Client** | `/frontend/shared/services/ipc/ipcClient.ts` |
| **Main Timer** | `/frontend/features/timer/components/MainTimer.tsx` |
| **Settings** | `/frontend/features/settings/components/SettingsView.tsx` |

## Appendix C: Command Reference

| Command | Description |
|---------|-------------|
| `make ci` | Run full CI pipeline locally |
| `cargo ci` | Alias for `cargo xtask ci` |
| `cargo xtask fmt` | Check formatting |
| `cargo xtask clippy` | Run lints |
| `cargo xtask test` | Run tests |
| `cargo xtask deny` | Check dependencies |
| `cargo xtask audit` | Security audit |
| `pnpm dev` | Start Vite dev server |
| `pnpm tauri:dev` | Start Tauri dev mode |
| `pnpm test` | Run frontend tests |
| `pnpm format:check` | Check frontend formatting |

---

**End of ADR-001**
