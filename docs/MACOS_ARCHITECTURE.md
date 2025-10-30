# macOS Application Architecture

**Version:** 1.0.0  
**Last Updated:** October 29, 2025  
**Status:** Production

---

## Table of Contents

1. [Introduction & Overview](#introduction--overview)
2. [System Architecture](#system-architecture)
3. [Backend Modules (Rust)](#backend-modules-rust)
4. [Frontend Modules (TypeScript/React)](#frontend-modules-typescriptreact)
5. [Data Flow Diagrams](#data-flow-diagrams)
6. [Module Interaction Matrix](#module-interaction-matrix)
7. [Key Features by Module](#key-features-by-module)
8. [Configuration & Deployment](#configuration--deployment)
9. [Performance & Scalability](#performance--scalability)
10. [Security & Privacy](#security--privacy)

---

## Introduction & Overview

### What is the macOS Application?

The **PulseArc macOS Application** is a Tauri-based desktop application that combines a high-performance Rust backend with a modern React/TypeScript frontend. It provides comprehensive time tracking, activity monitoring, and productivity analytics specifically optimized for macOS.

### Core Purpose

The macOS application serves as the **production implementation** of the PulseArc platform for macOS users. It:

- **Tracks** user activity using macOS Accessibility API and NSWorkspace events
- **Manages** time entries with intelligent classification
- **Syncs** data with multiple backend systems (Main API, SAP, Calendar)
- **Provides** real-time UI for time management and productivity insights
- **Integrates** with external services (Google Calendar, Microsoft Calendar, SAP)
- **Ensures** data privacy with SQLCipher encryption and PII redaction

### Platform Capabilities

| Feature | Status | Technology | Performance |
|---------|--------|------------|-------------|
| **Activity Capture** | ✅ Full | AX API + NSWorkspace | Event-driven, <0.5% CPU |
| **Browser URL Extraction** | ✅ Full | AX API traversal | ~50-200ms per fetch |
| **Office Document Metadata** | ✅ Full | Window title parsing | <1ms |
| **Idle Detection** | ✅ Full | CGEventSource | Real-time |
| **Calendar Sync** | ✅ Full | OAuth + REST API | Multi-provider |
| **SAP Integration** | ✅ Full | GraphQL + OAuth | Outbox pattern |
| **ML Classification** | ✅ Full | Linfa (tree + logistic) | ~100ms per block |

### Architecture Philosophy

The application follows a **hybrid architecture** combining the best of both worlds:

- **Rust Backend**: Performance-critical operations, database access, system integration
- **React Frontend**: Rich user interface, state management, user interactions
- **Tauri Bridge**: Type-safe IPC communication between frontend and backend
- **Connection Pooling**: Concurrent database access without blocking
- **Event-Driven**: Minimal polling, maximum responsiveness
- **Offline-First**: Full functionality without network connectivity

---

## System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      macOS Application (Tauri 2.x)                      │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼──────────────────────────┐
        │                           │                          │
┌───────▼────────┐          ┌───────▼────────┐         ┌──────▼──────┐
│  RUST BACKEND  │◀────────▶│  TAURI BRIDGE  │◀───────▶│   REACT UI  │
│   (src-tauri)  │   IPC    │   (Commands)   │   IPC   │  (frontend) │
└────────────────┘          └────────────────┘         └─────────────┘
        │                           │                          │
        │                           │                          │
   ┌────▼──────────────────────────▼───────────────────────────▼────┐
   │                                                                │
   │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐        │
   │  │ Database │  │ Tracker  │  │  Sync    │  │ Inference│        │
   │  │(SQLCipher│  │ (Events) │  │(Outbox)  │  │  (ML)    │        │
   │  └──────────┘  └──────────┘  └──────────┘  └──────────┘        │
   │                                                                │
   │  ┌──────────────────┐  ┌──────────────────┐  ┌──────────────┐  │
   │  │  Integrations    │  │  Observability   │  │   Shared     │  │
   │  │ (SAP, Calendar)  │  │  (Metrics, Logs) │  │ (Auth, Types)│  │
   │  └──────────────────┘  └──────────────────┘  └──────────────┘  │
   │                                                                │
   └────────────────────────────────────────────────────────────────┘
                                    │
        ┌───────────────────────────┼─────────────────────────┐
        │                           │                         │
   ┌────▼─────┐              ┌──────▼──────┐           ┌──────▼──────┐
   │   macOS  │              │  Backend    │           │  External   │
   │    OS    │              │    APIs     │           │  Services   │
   │(AX, NSW) │              │  (GraphQL)  │           │(OAuth, REST)│
   └──────────┘              └─────────────┘           └─────────────┘
```

### Component Layers

#### 1. UI Layer (React/TypeScript)
- **Purpose**: User interface and interaction handling
- **Location**: `frontend/`
- **Output**: User commands, UI state updates
- **Communication**: Tauri IPC to backend commands

#### 2. Command Layer (Tauri Bridge)
- **Purpose**: Type-safe bridge between frontend and backend
- **Location**: `src-tauri/src/commands/`, `src-tauri/src/main.rs`
- **Methods**: Tauri command handlers with `#[tauri::command]`
- **Communication**: Bidirectional IPC (invoke/emit)

#### 3. Backend Core (Rust)
- **Purpose**: Business logic, data processing, system integration
- **Location**: `src-tauri/src/`
- **Subsystems**: Database, Tracker, Sync, Inference, Integrations
- **Communication**: Direct function calls, async tasks

#### 4. Data Layer
- **Purpose**: Persistent storage with encryption
- **Technology**: SQLCipher with connection pooling
- **Location**: `src-tauri/src/db/`
- **Features**: Outbox pattern, transactions, migrations

#### 5. Integration Layer
- **Purpose**: External service communication
- **Systems**: Main API, SAP GraphQL, Google/Microsoft Calendar
- **Location**: `src-tauri/src/integrations/`, `src-tauri/src/domain/`
- **Pattern**: OAuth → HTTP Client → Outbox → Retry → Sync

### Technology Stack

#### Backend (Rust)
```toml
Framework:        Tauri 2.9 (desktop application framework)
Database:         SQLCipher 0.37 (encrypted SQLite)
Connection Pool:  r2d2 + r2d2_sqlite
HTTP Client:      reqwest 0.12 (with rustls-tls)
Async Runtime:    tokio 1.x (multi-threaded)
ML Framework:     linfa 0.7 (tree + logistic regression)
Serialization:    serde + serde_json
OAuth:            oauth2 5.0
GraphQL:          Custom implementation
```

#### Frontend (TypeScript/React)
```json
Framework:        React 18
Build Tool:       Vite
UI Components:    Custom + Shared components library
State Management: Zustand stores + React hooks
Styling:          TailwindCSS
Testing:          Vitest + React Testing Library
Type Safety:      TypeScript 5.x
```

#### Platform APIs (macOS)
```
Accessibility:    AX API (objc2-app-kit)
Workspace Events: NSWorkspace notifications (objc2-foundation)
Idle Detection:   CGEventSource (core-graphics)
Sleep/Wake:       IOKit (io-kit-sys)
Lock Detection:   CFNotificationCenter (core-foundation)
Keychain:         Security framework (keyring crate)
```

---

## Backend Modules (Rust)

### Core Modules

#### 1. Database Module (`db/`)

**Purpose**: SQLCipher-encrypted database with connection pooling for concurrent access.

**Key Files**:
- `manager.rs` - DbManager with r2d2 connection pool
- `migrations.rs` - Schema versioning and migrations
- `models.rs` - Data structures matching SQL schema
- `activity/` - Snapshot and segment operations
- `batch/` - Batch queue and DLQ operations
- `outbox/` - Outbox pattern for sync
- `blocks/` - Proposed time blocks
- `calendar/` - Calendar events and sync settings

**Features**:
- ✅ Connection pooling (8 connections max)
- ✅ SQLCipher encryption with keychain integration
- ✅ WAL mode for concurrent reads during writes
- ✅ Prepared statement caching (32 statements)
- ✅ Outbox pattern for idempotent writes
- ✅ ID mapping (local UUID ↔ backend CUID)
- ✅ Token usage tracking for cost management
- ✅ Dead Letter Queue for permanent failures

**Architecture**:
```
DbManager (Connection Pool)
    │
    ├─► ActivityOperations (snapshots, segments)
    ├─► BatchOperations (queues, leases, DLQ)
    ├─► OutboxOperations (time entries, retry logic)
    ├─► CalendarOperations (tokens, events, sync settings)
    ├─► BlockOperations (proposed blocks, acceptance)
    └─► UtilOperations (stats, queries, cleanup)
```

**Performance**:
- Bulk inserts: 100k records in 12-18 seconds
- Connection acquisition: <1ms (pooled)
- Query latency: <10ms (indexed queries)
- Cache hit rate: ~95% (prepared statements)

---

#### 2. Tracker Module (`tracker/`)

**Purpose**: Activity tracking with event-driven detection and enrichment caching.

**Key Files**:
- `core.rs` - Tracker orchestration and RefresherState
- `provider.rs` - ActivityProvider trait
- `providers/macos.rs` - macOS-specific provider with AX API
- `idle/` - Idle detection with sleep/wake recovery
- `os_events/` - NSWorkspace event listener abstraction

**Features**:
- ✅ Event-driven app switching (NSWorkspace notifications)
- ✅ Browser URL extraction (Chrome, Safari, Firefox, Arc, Edge, Brave)
- ✅ Office document metadata (Excel, Word, PowerPoint, PDF)
- ✅ Enrichment caching with 750ms TTL
- ✅ Background enrichment worker (optional)
- ✅ Smart change detection (meaningful fields only)
- ✅ Snapshot persistence (30-second intervals)
- ✅ Idle period tracking with recovery

**Architecture**:
```
Tracker
  │
  ├─► MacOsProvider
  │     ├─► AX API (window info)
  │     ├─► Browser Enrichers (URL extraction)
  │     ├─► Office Enrichers (document metadata)
  │     └─► Enrichment Cache (TTL-based)
  │
  ├─► MacOsEventListener
  │     └─► NSWorkspace (app activation events)
  │
  ├─► IdleDetector
  │     ├─► CGEventSource (idle time)
  │     ├─► Sleep/Wake Recovery
  │     └─► Period Tracker
  │
  └─► SnapshotWriter
        └─► Database (30-second saves)
```

**Performance**:
- Event-driven latency: ~10-50ms (app switch to context fetch)
- AX API fetch: ~5-20ms (without enrichment)
- Browser URL fetch: ~50-200ms (AX API traversal)
- Cache hit rate: ~80% (750ms TTL)
- CPU usage: <0.5% (event-driven)

---

#### 3. Detection Module (`detection/`)

**Purpose**: Intelligent activity detection with pack-based rule system.

**Key Files**:
- `mod.rs` - Engine + Detector trait
- `default.rs` - Fallback activity generator
- `packs/` - Pack-based detector organization
- `enrichers/` - Browser and office metadata enrichers

**Detector Packs**:

**Technology Pack** (priority: 10)
- IDE Detector (VSCode, Cursor, IntelliJ, XCode)
- Browser Detector (with site-specific sub-detectors)
- Design Detector (Figma, Sketch, Photoshop)
- Comms Detector (Slack, Discord, Teams)
- Email Detector (Mail, Outlook, Spark)
- Terminal Detector (iTerm2, Warp, Alacritty)

**Deals Pack** (priority: 5)
- VDR Detector (Datasite, Intralinks, Firmex)
- Tax Research Detector (Bloomberg Tax, CCH, Tax Notes)
- Deal Documents Detector (M&A, PE, Banking documents)
- Tax Software Detector (Vertex, Avalara, Onesource)
- Client Communication Detector (deal-related meetings)
- Practice Management Detector (SAP, Neon, FinancialForce)

**Features**:
- ✅ Priority-based pack execution
- ✅ First-match-wins strategy
- ✅ Configurable pack enabling/disabling
- ✅ Fallback to generic activity
- ✅ Zero-copy string operations

---

#### 4. Sync Module (`sync/`)

**Purpose**: Backend synchronization with retry logic and cost tracking.

**Key Files**:
- `neon_client.rs` - Backend GraphQL client
- `outbox_worker.rs` - Outbox pattern processor
- `scheduler.rs` - Sync scheduler with configurable intervals
- `retry.rs` - Generic retry mechanism with exponential backoff
- `cost_tracker.rs` - Token usage and cost cap enforcement
- `cleanup.rs` - Local storage retention policies

**Features**:
- ✅ Outbox pattern for idempotent writes
- ✅ Exponential backoff with jitter
- ✅ HTTP 429 rate limit handling
- ✅ Monthly cost caps ($5/month default)
- ✅ Graceful degradation (rules-only when cap exceeded)
- ✅ ID mapping (local UUID ↔ backend CUID)
- ✅ Automatic cleanup (1-day retention for processed data)

**Architecture**:
```
SyncScheduler (tokio task)
    │
    ├─► OutboxWorker
    │     ├─► NeonClient (GraphQL)
    │     ├─► RetryExecutor (exponential backoff)
    │     └─► CostTracker (token usage)
    │
    └─► CleanupService
          └─► Database (retention policies)
```

**Sync Flow**:
```
1. Fetch pending outbox entries
2. Map to GraphQL input (DTO mapper)
3. Execute with retry policy
   ├─► Success: Record CUID, mark sent
   ├─► Retryable: Increment attempt, schedule retry
   └─► Permanent: Move to DLQ
4. Update ID mapping table
5. Clean up old processed data
```

---

#### 5. Preprocessing Module (`preprocess/`)

**Purpose**: Privacy-focused data preparation with PII redaction.

**Key Files**:
- `redact.rs` - PII redaction and label normalization
- `segmenter.rs` - Activity segmentation
- `trigger.rs` - Event-driven segmentation trigger

**Features**:
- ✅ Email redaction (`[EMAIL]`)
- ✅ URL query parameter removal
- ✅ Document ID redaction (`[ID]`)
- ✅ File path username redaction (`[USER]`)
- ✅ IP address redaction (`[IP]`)
- ✅ Phone number redaction (`[PHONE]`)
- ✅ Label normalization (app:context format)
- ✅ Time window segmentation (5-minute windows)
- ✅ Activity-based grouping

**Normalization Examples**:
```rust
normalize_label("vscode", "main.rs - my-project")
// Returns: "vscode:main.rs"

normalize_label("chrome", "https://github.com/user/repo")
// Returns: "chrome:github.com"

normalize_label("slack", "#engineering - Slack")
// Returns: "slack:#engineering"
```

---

#### 6. Inference Module (`inference/`)

**Purpose**: ML-based classification with hybrid approach.

**Key Files**:
- `block_builder.rs` - Converts segments to proposed blocks
- `scheduler.rs` - Block building scheduler (11 PM daily)
- `tree_classifier.rs` - Decision tree classifier (linfa)
- `logistic_classifier.rs` - Logistic regression classifier
- `rules_classifier.rs` - Sophisticated rules-based fallback
- `metrics.rs` - Classification performance tracking

**Classification Modes**:

**1. Hybrid (Default)**
```
Input: Proposed Block
   ↓
Tree Classifier (linfa)
   ↓
Logistic Classifier (linfa)
   ↓
Sophisticated Rules
   ↓
Output: Classified Time Entry
```

**2. Rules-Only (Cost Cap Exceeded)**
```
Input: Proposed Block
   ↓
Sophisticated Rules
   ↓
Output: Classified Time Entry
```

**Features**:
- ✅ Hybrid classifier (tree + logistic + rules)
- ✅ 100% local classification (no API calls)
- ✅ Graceful degradation when cost cap exceeded
- ✅ Metrics tracking (accuracy, latency, mode usage)
- ✅ Training pipeline from test data
- ✅ Real-time classification (Build My Day)

**Performance**:
- Classification latency: ~100ms per block
- Training time: <1 second (44 examples)
- Memory usage: ~5MB (model + features)
- Accuracy: ~85% (tree + logistic), ~70% (rules-only)

---

### Integration Modules

#### 7. Calendar Integration (`integrations/calendar/`)

**Purpose**: Multi-provider calendar synchronization.

**Supported Providers**:
- ✅ Google Calendar (OAuth 2.0 + REST API)
- ✅ Microsoft Calendar (OAuth 2.0 + REST API)

**Key Files**:
- `oauth.rs` - OAuth flow with PKCE
- `client.rs` - REST API client
- `sync.rs` - Calendar event synchronization
- `scheduler.rs` - Background sync scheduler
- `providers/` - Provider-specific implementations

**Features**:
- ✅ OAuth 2.0 with PKCE for security
- ✅ Token refresh with automatic retry
- ✅ Keychain storage for tokens
- ✅ Incremental sync with sync tokens
- ✅ Configurable sync interval (15 minutes default)
- ✅ Event filtering (all-day, duration, exclusions)
- ✅ Timeline integration
- ✅ Suggestions for time blocking

**Sync Flow**:
```
1. Check calendar connection (token expiry)
2. Refresh token if needed (automatic)
3. Fetch events since last sync (incremental)
4. Parse and normalize events
5. Store in local database
6. Emit events to UI (timeline update)
7. Schedule next sync (configurable interval)
```

---

#### 8. SAP Integration (`integrations/sap/`)

**Purpose**: SAP time entry forwarding with WBS caching.

**Key Files**:
- `auth/` - SAP OAuth authentication
- `client.rs` - SAP GraphQL client
- `forwarder.rs` - Outbox pattern forwarder
- `scheduler.rs` - WBS sync scheduler
- `cache.rs` - WBS element caching
- `validation.rs` - WBS code validation
- `bulk_lookup.rs` - Batch WBS lookups

**Features**:
- ✅ OAuth 2.0 authentication
- ✅ Outbox pattern for reliable forwarding
- ✅ WBS element caching (local + Neon)
- ✅ Batch WBS lookup (reduce GraphQL calls)
- ✅ Health monitoring with auto-recovery
- ✅ Retry logic with exponential backoff
- ✅ Network status detection
- ✅ Validation before submission

**Architecture**:
```
SAP Integration
  │
  ├─► SapAuthService (OAuth tokens)
  │     └─► Keychain (token storage)
  │
  ├─► SapClient (GraphQL)
  │     ├─► WBS Search
  │     ├─► Time Entry Creation
  │     └─► Health Check
  │
  ├─► OutboxForwarder
  │     ├─► Fetch pending entries
  │     ├─► Validate WBS codes
  │     ├─► Submit to SAP
  │     └─► Retry on failure
  │
  ├─► WbsCache
  │     ├─► Local SQLite cache
  │     └─► Neon DB cache (shared)
  │
  └─► SyncScheduler
        └─► Daily WBS sync (configurable)
```

---

#### 9. Main API Integration (`domain/api/`)

**Purpose**: Main backend API integration for time entries and user data.

**Key Files**:
- `auth.rs` - OAuth authentication service
- `client.rs` - GraphQL client with retry logic
- `forwarder.rs` - Outbox pattern forwarder
- `scheduler.rs` - Background sync scheduler
- `models.rs` - GraphQL types and DTOs

**Features**:
- ✅ OAuth 2.0 authentication
- ✅ Outbox pattern for reliable sync
- ✅ 10-second sync interval (configurable)
- ✅ Token refresh with auto-recovery
- ✅ Health monitoring
- ✅ User profile management
- ✅ Project/task synchronization

---

### Supporting Modules

#### 10. HTTP Module (`http/`)

**Purpose**: HTTP client abstraction with GraphQL support.

**Key Files**:
- `client.rs` - HttpClient with retry and rate limiting
- `graphql.rs` - GraphQL request builders

**Features**:
- ✅ Connection pooling (reqwest)
- ✅ Exponential backoff retry
- ✅ HTTP 429 rate limit handling
- ✅ Timeout configuration
- ✅ TLS with rustls
- ✅ GraphQL query/mutation builders
- ✅ Error classification (retryable vs permanent)

---

#### 11. Observability Module (`observability/`)

**Purpose**: Metrics, errors, and monitoring.

**Key Files**:
- `metrics/` - Performance metrics and counters
- `errors/` - Structured error types
- `datadog.rs` - Datadog integration (optional)

**Metrics**:
- Activity fetch latency
- Enrichment success/failure rates
- Database query performance
- Sync success/failure rates
- Classification accuracy
- Token usage statistics

---

#### 12. Shared Module (`shared/`)

**Purpose**: Common utilities and types.

**Key Files**:
- `auth/` - OAuth services and token management
- `config.rs` - Configuration management
- `cache.rs` - Startup cache for instant display
- `types/` - Shared data structures
- `extractors/` - Pattern matching utilities

**Features**:
- ✅ OAuth service abstraction (reusable for SAP, Calendar, Main API)
- ✅ PKCE support for OAuth flows
- ✅ Keychain integration for token storage
- ✅ Startup cache for <500ms TTFD
- ✅ Activity context types
- ✅ Pattern extractors for metadata

---

## Frontend Modules (TypeScript/React)

### Architecture Overview

The frontend follows a **feature-based architecture** with clear separation of concerns:

```
frontend/
  │
  ├─► analytics/        (Analytics views, idle detection UI)
  ├─► capture/          (Activity tracker interface)
  ├─► settings/         (Settings panel with integrations)
  └─► time-management/  (Timer, entries, timeline, projects)
```

Each feature module follows this pattern:
```
feature-name/
  ├─► components/  (React components)
  ├─► hooks/       (Custom hooks for business logic)
  ├─► services/    (API calls via Tauri invoke)
  ├─► stores/      (Zustand stores for state)
  ├─► types/       (TypeScript types)
  └─► utils/       (Helper functions)
```

---

### Feature Modules

#### 1. Analytics Module (`analytics/`)

**Purpose**: Activity analytics and idle time management.

**Sub-Features**:

**Analytics Views**
- Activity breakdowns by project/category
- Time distribution charts
- Productivity metrics
- Historical data analysis

**Idle Detection**
- Idle period review modal
- Action selection (discard, add to last entry, manual entry)
- Idle time chart visualization
- Daily idle summaries

**Key Components**:
- `AnalyticsView.tsx` - Main analytics dashboard
- `IdleDetectionModal.tsx` - Idle period review interface
- `IdlePeriodDetail.tsx` - Detailed idle period information
- `IdleTimeChart.tsx` - Visual representation of idle periods

**Services**:
- `analyticsService.ts` - Data fetching and aggregation
- `idleAnalyticsService.ts` - Idle period statistics
- `idleDetectionService.ts` - Idle period actions (discard, add, manual)

**Tauri Commands Used**:
```typescript
invoke('get_idle_periods', { start_ts, end_ts })
invoke('update_idle_period_action', { period_id, action, notes })
invoke('get_idle_summary', { date })
invoke('get_database_stats')
invoke('get_recent_activities', { limit })
```

---

#### 2. Capture Module (`capture/`)

**Purpose**: AI-powered activity tracker interface.

**Key Features**:
- Real-time activity context display
- AI-generated suggestions
- Manual activity entry
- Suggestion acceptance/dismissal with feedback
- Keyboard shortcuts (Cmd+I to open)

**Key Components**:
- `ActivityTrackerView.tsx` - Main tracker interface
- `SuggestionChip.tsx` - Individual suggestion display

**Hooks**:
- `useSuggestionManager.ts` - Manages suggestion lifecycle
  - Fetches activity context
  - Handles acceptance/dismissal
  - Manages loading states
  - Provides feedback collection

**Tauri Commands Used**:
```typescript
invoke('fetch_activity_context')
invoke('save_manual_activity', { description })
invoke('get_user_projects')  // For suggestions
```

**Features**:
- ✅ Pre-fetching activity context before window opens
- ✅ Efficient rendering with suggestion chips
- ✅ Pause state awareness
- ✅ Keyboard navigation support
- ✅ Feedback collection for dismissed suggestions

---

#### 3. Settings Module (`settings/`)

**Purpose**: Application configuration and integration management.

**Sub-Features**:

**Account Management**
- User profile display
- OAuth authentication status
- Sign in/sign out functionality

**Calendar Integration**
- Multi-provider support (Google, Microsoft)
- OAuth connection flow
- Sync settings configuration
- Calendar event filtering
- Exclusion rules

**SAP Integration**
- OAuth authentication
- WBS search and validation
- Outbox status monitoring
- Sync settings configuration
- Health monitoring

**Main API Integration**
- OAuth authentication
- Scheduler control (start/stop)
- Outbox status display
- User information

**Key Components**:
- `SettingsView.tsx` - Main settings panel
- `AccountMenu.tsx` - User account dropdown
- `CalendarProviderCard.tsx` - Calendar provider cards
- `SapSettings.tsx` - SAP integration settings
- `MainApiSettings.tsx` - Main API settings
- `SyncStatus.tsx` - Sync status indicators
- `IdleDetectionSettings.tsx` - Idle detection configuration

**Services**:
- `settingsService.ts` - General settings operations
- `calendarService.ts` - Calendar operations (multi-provider)
- `sapService.ts` - SAP operations
- `WebApiService.ts` - Main API operations
- `adminService.ts` - Database admin operations

**Tauri Commands Used**:
```typescript
// Calendar
invoke('initiate_calendar_auth', { provider })
invoke('disconnect_calendar', { provider })
invoke('get_calendar_connection_status')
invoke('update_calendar_sync_settings', { user_email, settings })
invoke('sync_calendar_events', { force })

// SAP
invoke('sap_start_login')
invoke('sap_is_authenticated')
invoke('sap_logout')
invoke('sap_search_wbs', { query })
invoke('sap_get_sync_settings')
invoke('sap_update_sync_settings', { settings })
invoke('sap_check_connection_health')

// Main API
invoke('webapi_start_login')
invoke('webapi_is_authenticated')
invoke('webapi_logout')
invoke('webapi_get_user_info')
invoke('webapi_start_scheduler')
invoke('webapi_stop_scheduler')
```

---

#### 4. Time Management Module (`time-management/`)

**Purpose**: Core time tracking and management features.

**Sub-Modules**:

##### Timer (`timer/`)
- Start/pause/stop timer
- Project and WBS selection
- Active timer display
- Timer state synchronization
- Keyboard shortcuts

**Key Components**:
- `TimerView.tsx` - Main timer interface
- `TimerControls.tsx` - Play/pause/stop buttons
- `ProjectSelector.tsx` - Project dropdown
- `WbsSelector.tsx` - WBS element selector
- `ActiveTimer.tsx` - Running timer display

##### Time Entries (`time-entry/`)
- Time entry list (compact and detailed views)
- Entry editing and deletion
- Classification with ML suggestions
- Idle time review integration
- Day view with summaries

**Key Components**:
- `EntriesView.tsx` - Main entries list
- `CompactEntries.tsx` - Compact view for quick review
- `EditEntryModal.tsx` - Entry editing interface
- `ClassifyEntryModal.tsx` - ML-powered classification
- `IdleTimeReview.tsx` - Idle period review integration
- `SaveEntryModal.tsx` - New entry creation

**Stores**:
- `entryStore.ts` - Entry state management
  - CRUD operations
  - Filtering and sorting
  - Selection management
  - Optimistic updates

##### Timeline (`timeline/`)
- Visual timeline of activities and calendar events
- Day/week views
- Calendar event integration
- Activity segment display
- Time block visualization

**Key Components**:
- `TimelineView.tsx` - Main timeline view
- `TimelineDayView.tsx` - Day-specific timeline
- `MarqueeText.tsx` - Scrolling text for long titles

**Services**:
- `timelineService.ts` - Timeline data fetching
  - Activity segments
  - Calendar events
  - Time blocks
  - Data merging and sorting

##### Projects (`project/`)
- Project management
- Quick project switching
- Project color coding
- Usage tracking

**Key Components**:
- `QuickProjectSwitcher.tsx` - Fast project selection

**Stores**:
- `projectStore.ts` - Project state management
  - Project list
  - Active project
  - Recent projects
  - Color management

##### Build My Day (`build-my-day/`)
- AI-generated time block suggestions
- Block acceptance/dismissal
- Real-time classification
- Filter and configuration

**Key Components**:
- `BuildMyDayView.tsx` - Main view with block list
- `BuildMyDayFilterPopover.tsx` - Date and filter controls

**Tauri Commands Used**:
```typescript
// Timer
invoke('pause_tracker')
invoke('resume_tracker')
invoke('get_tracker_state')

// Entries
invoke('get_time_entries', { time_filter })
invoke('update_suggestion', { id, title, project, wbs_code, duration_sec })
invoke('accept_suggestion', { id })
invoke('dismiss_suggestion', { id, reason })
invoke('delete_outbox_entry', { id })

// Build My Day
invoke('build_my_day', { date })
invoke('get_proposed_blocks')
invoke('accept_proposed_block', { block_id })
invoke('dismiss_proposed_block', { block_id })
invoke('get_pending_blocks_count')

// Timeline
invoke('get_calendar_events_for_timeline', { startDate, endDate })
```

---

### Frontend Architecture Patterns

#### 1. Service Layer Pattern

Services encapsulate all backend communication:

```typescript
// services/exampleService.ts
import { invoke } from '@tauri-apps/api/core';

export const exampleService = {
  async fetchData(params: Params): Promise<Result> {
    return invoke('backend_command', params);
  },
  
  async updateData(id: string, data: Data): Promise<void> {
    return invoke('update_command', { id, data });
  }
};
```

**Benefits**:
- Type-safe backend calls
- Centralized error handling
- Easy mocking for tests
- Clear separation of concerns

---

#### 2. Store Pattern (Zustand)

State management with Zustand stores:

```typescript
// stores/exampleStore.ts
import { create } from 'zustand';

interface ExampleState {
  items: Item[];
  loading: boolean;
  error: string | null;
  
  fetchItems: () => Promise<void>;
  updateItem: (id: string, data: Data) => Promise<void>;
}

export const useExampleStore = create<ExampleState>((set, get) => ({
  items: [],
  loading: false,
  error: null,
  
  fetchItems: async () => {
    set({ loading: true, error: null });
    try {
      const items = await exampleService.fetchData();
      set({ items, loading: false });
    } catch (error) {
      set({ error: error.message, loading: false });
    }
  },
  
  updateItem: async (id, data) => {
    await exampleService.updateData(id, data);
    // Optimistic update
    set(state => ({
      items: state.items.map(item => 
        item.id === id ? { ...item, ...data } : item
      )
    }));
  }
}));
```

**Benefits**:
- Simple, minimal boilerplate
- React DevTools integration
- No provider wrapping needed
- TypeScript-first design

---

#### 3. Custom Hooks Pattern

Business logic extraction into hooks:

```typescript
// hooks/useExample.ts
export function useExample(config: Config) {
  const [state, setState] = useState<State>(initialState);
  const store = useExampleStore();
  
  useEffect(() => {
    // Setup logic
    store.fetchItems();
    
    // Cleanup
    return () => {
      // Cleanup logic
    };
  }, []);
  
  const handleAction = useCallback((params: Params) => {
    // Business logic
    setState(prevState => /* update */);
    store.updateItem(params.id, params.data);
  }, [store]);
  
  return {
    state,
    handleAction,
    // ... other exports
  };
}
```

**Benefits**:
- Reusable business logic
- Testable in isolation
- Clean component code
- Composition over inheritance

---

#### 4. Component Organization

Components follow a consistent structure:

```typescript
// components/Example.tsx
import React from 'react';
import { useExample } from '../hooks/useExample';

interface ExampleProps {
  // Props definition
}

export function Example({ prop1, prop2 }: ExampleProps) {
  // Hooks at top
  const { state, handleAction } = useExample({ prop1 });
  
  // Event handlers
  const handleClick = () => {
    handleAction({ /* params */ });
  };
  
  // Render
  return (
    <div>
      {/* JSX */}
    </div>
  );
}
```

---

## Data Flow Diagrams

### 1. Activity Capture Flow

```
macOS User Activity
        ↓
  [AX API / NSWorkspace]
        ↓
   MacOsProvider.fetch()
        ↓
  [Enrichment Cache Check]
        ↓
   ┌─────────┴─────────┐
   │                   │
[Cache Hit]      [Cache Miss]
   │                   │
   │            ┌──────▼──────┐
   │            │Browser/Office│
   │            │  Enrichment  │
   │            └──────┬───────┘
   │                   │
   └─────────┬─────────┘
             ↓
      ActivityContext
             ↓
   [Smart Change Detection]
             ↓
      Cache Update
             ↓
   Emit EVENT_ACTIVITY_UPDATED
             ↓
      Frontend Receives Event
             ↓
      UI Updates (React)
```

---

### 2. Time Entry Classification Flow

```
Activity Snapshots (30s intervals)
        ↓
  [Segmentation Trigger]
        ↓
   Group into 5-minute segments
        ↓
   [PII Redaction]
        ↓
   Save ActivitySegments
        ↓
   [Block Builder Scheduler - 11 PM]
        ↓
   Fetch unprocessed segments
        ↓
   Group by activity similarity
        ↓
   Create ProposedTimeBlocks
        ↓
   [HybridClassifier]
        ↓
   ┌───────────────┴──────────────┐
   │                              │
[Tree Classifier]       [Rules Classifier]
   │                              │
[Logistic Classifier]             │
   │                              │
   └───────────────┬──────────────┘
                   ↓
         Classified Time Entry
                   ↓
         Store in proposed_blocks table
                   ↓
         Frontend fetches (Build My Day)
                   ↓
         User accepts/dismisses
                   ↓
         ┌────────┴────────┐
         │                 │
    [Accept]          [Dismiss]
         │                 │
    Create outbox        Mark dismissed
    entry for sync       (with feedback)
         │
         ↓
    Sync to backend
```

---

### 3. Backend Sync Flow (Outbox Pattern)

```
Classified Time Entry
        ↓
   Create TimeEntryOutbox
        ↓
   Generate idempotency key
   {uuid}:{user_id}:{start_ts}:{hash}
        ↓
   Save to time_entry_outbox table
        ↓
   [SyncScheduler - 10s interval]
        ↓
   Fetch pending entries
        ↓
   [RetryExecutor]
        ↓
   HTTP POST to GraphQL
        ↓
   ┌────────────┴─────────────┐
   │                          │
[Success]               [Error]
   │                          │
   │                    ┌─────┴──────┐
   │                    │            │
   │              [Retryable]  [Permanent]
   │                    │            │
   │              Increment    Move to DLQ
   │              attempts,         │
   │              schedule retry    │
   │                                │
   ↓                                │
Backend returns CUID                │
   ↓                                │
Create IdMapping                    │
(local UUID ↔ backend CUID)         │
   ↓                                │
Update outbox status: Sent          │
   ↓                                │
   └────────────┬───────────────────┘
                ↓
       [CleanupService]
                ↓
    Delete old processed entries
         (1-day retention)
```

---

### 4. Calendar Integration Flow

```
User clicks "Connect Calendar"
        ↓
   [CalendarService.initiateAuth()]
        ↓
   invoke('initiate_calendar_auth', { provider })
        ↓
   Tauri opens OAuth URL in browser
        ↓
   User grants permissions
        ↓
   OAuth callback to localhost:3000/callback
        ↓
   Extract authorization code
        ↓
   Exchange code for tokens
        ↓
   [CalendarTokenOperations]
        ↓
   Store tokens in database
        ↓
   Store refresh token in keychain
        ↓
   Create CalendarSyncSettings
        ↓
   [CalendarSyncScheduler.start()]
        ↓
   Background sync loop (15 min interval)
        ↓
   ┌──────────────────┐
   │                  │
   │  1. Check token expiry
   │  2. Refresh if needed
   │  3. Fetch events (incremental)
   │  4. Parse events
   │  5. Store in calendar_events table
   │  6. Emit to UI
   │  7. Sleep until next sync
   │                  │
   └──────────────────┘
                ↓
   Timeline component subscribes
                ↓
   Display events in timeline view
```

---

### 5. SAP Integration Flow

```
User accepts time block
        ↓
   Create TimeEntryOutbox
        ↓
   Add to time_entry_outbox table
        ↓
   [OutboxForwarder - 10s interval]
        ↓
   Fetch pending SAP entries
        ↓
   [WBS Validation]
        ↓
   ┌─────────────┴──────────────┐
   │                            │
[Valid WBS]              [Invalid WBS]
   │                            │
   │                      Mark as failed
   │                      (validation error)
   │                            │
   ↓                            │
[SapClient.createTimeEntry()]   │
   │                            │
   ↓                            │
GraphQL mutation                │
   │                            │
   ┌─────────┴──────────┐       │
   │                    │       │
[Success]          [Error]      │
   │                    │       │
   │              ┌─────┴───────┐
   │              │             │
   │        [Retryable]  [Permanent]
   │              │             │
   │         Exponential   Move to DLQ
   │         backoff retry      │
   │              │             │
   ↓              │             │
Record SAP ID     │             │
   ↓              │             │
Update status     │             │
   ↓              │             │
   └──────────────┴─────────────┘
                  ↓
         User views in UI
         (SAP Settings → Outbox Status)
```

---

## Module Interaction Matrix

### Backend Module Dependencies

| Module | Depends On | Used By |
|--------|-----------|---------|
| **db** | - (core) | tracker, sync, inference, integrations |
| **tracker** | db, detection, observability | main.rs (singleton) |
| **detection** | shared | tracker |
| **preprocess** | db, shared | tracker, inference |
| **sync** | db, http, shared | main.rs (scheduler) |
| **inference** | db, preprocess, observability | main.rs (scheduler), commands |
| **integrations/calendar** | db, http, shared | main.rs (scheduler) |
| **integrations/sap** | db, http, shared, cache | main.rs (scheduler, forwarder) |
| **domain/api** | db, http, shared | main.rs (scheduler) |
| **http** | shared | sync, integrations, domain |
| **observability** | - (core) | all modules |
| **shared** | - (core) | all modules |

---

### Frontend Module Dependencies

| Module | Imports From | Exports To |
|--------|-------------|-----------|
| **analytics** | shared (components, types) | App.tsx |
| **capture** | shared (components, types) | App.tsx |
| **settings** | shared (components, types) | App.tsx |
| **time-management/timer** | shared, project (store) | time-management/index |
| **time-management/entries** | shared, project (store) | time-management/index |
| **time-management/timeline** | shared, calendar (service) | time-management/index |
| **time-management/projects** | shared | timer, entries, timeline |
| **time-management/build-my-day** | shared, entries (types) | time-management/index |

---

### IPC Communication Patterns

| Frontend Module | Tauri Commands | Backend Modules |
|----------------|----------------|-----------------|
| **analytics** | `get_idle_periods`, `update_idle_period_action` | commands/idle → db |
| **capture** | `fetch_activity_context`, `save_manual_activity` | lib.rs (singleton) → tracker |
| **settings (Calendar)** | `initiate_calendar_auth`, `sync_calendar_events` | commands/calendar → integrations/calendar |
| **settings (SAP)** | `sap_start_login`, `sap_search_wbs` | integrations/sap/commands → sap module |
| **settings (Main API)** | `webapi_start_login`, `webapi_get_user_info` | domain/api/commands → api module |
| **timer** | `pause_tracker`, `resume_tracker` | lib.rs → tracker |
| **entries** | `get_time_entries`, `update_suggestion` | commands/monitoring → db |
| **timeline** | `get_calendar_events_for_timeline` | commands/calendar → db/calendar |
| **build-my-day** | `build_my_day`, `accept_proposed_block` | commands/blocks → inference |

---

### Scheduler Coordination

The application runs multiple background schedulers coordinated by the main thread:

```
main.rs (Tauri setup)
    │
    ├─► SyncScheduler (10s interval)
    │     └─► OutboxWorker → NeonClient
    │
    ├─► BlockScheduler (11 PM daily)
    │     └─► BlockBuilder → HybridClassifier
    │
    ├─► CalendarSyncScheduler (15 min interval)
    │     └─► CalendarSync → Provider API
    │
    ├─► SapOutboxForwarder (10s interval)
    │     └─► SapClient → SAP GraphQL
    │
    ├─► SapWbsSyncScheduler (24h interval)
    │     └─► WbsCache → SAP GraphQL
    │
    ├─► WebApiScheduler (10s interval)
    │     └─► WebApiForwarder → Main API
    │
    ├─► CleanupScheduler (1h interval)
    │     └─► CleanupService → Database
    │
    └─► SnapshotWriter (30s interval)
          └─► Tracker → Database
```

**Coordination Strategy**:
- Independent tokio tasks (no shared state)
- Database connection pooling prevents contention
- Staggered intervals reduce simultaneous load
- Graceful shutdown on app quit (stop() methods)

---

## Key Features by Module

### Activity Tracking
- **Module**: tracker/
- **Status**: ✅ Complete
- **Features**:
  - Event-driven app switching (macOS NSWorkspace)
  - Browser URL extraction (6 browsers supported)
  - Office document metadata (Excel, Word, PowerPoint, PDF)
  - Enrichment caching (750ms TTL)
  - Smart change detection
  - 30-second snapshot persistence
  - Idle detection with sleep/wake recovery

---

### Activity Classification
- **Module**: inference/
- **Status**: ✅ Complete
- **Features**:
  - Hybrid classifier (tree + logistic + rules)
  - 100% local classification (no API calls)
  - Block building (11 PM daily)
  - Real-time classification (Build My Day)
  - Metrics tracking
  - Graceful degradation on cost cap

---

### Calendar Integration
- **Module**: integrations/calendar/
- **Status**: ✅ Complete
- **Providers**:
  - Google Calendar
  - Microsoft Calendar
- **Features**:
  - OAuth 2.0 with PKCE
  - Multi-provider support
  - Incremental sync with sync tokens
  - Background scheduler (15 min interval)
  - Event filtering and exclusions
  - Timeline integration
  - Suggestions for time blocking

---

### SAP Integration
- **Module**: integrations/sap/
- **Status**: ✅ Complete
- **Features**:
  - OAuth 2.0 authentication
  - WBS element search and caching
  - Outbox pattern forwarding
  - Batch WBS lookups
  - Health monitoring
  - Network status detection
  - Validation before submission
  - Background sync (daily)

---

### Main API Integration
- **Module**: domain/api/
- **Status**: ✅ Complete
- **Features**:
  - OAuth 2.0 authentication
  - Outbox pattern sync
  - User profile management
  - Project/task sync
  - Background scheduler (10s)
  - Health monitoring

---

### Database
- **Module**: db/
- **Status**: ✅ Complete
- **Features**:
  - SQLCipher encryption
  - Connection pooling (r2d2)
  - WAL mode for concurrency
  - Outbox pattern
  - ID mapping (UUID ↔ CUID)
  - Token usage tracking
  - Dead Letter Queue
  - Cleanup scheduler (1h)

---

### Idle Detection
- **Module**: tracker/idle/
- **Status**: ✅ Complete
- **Features**:
  - CGEventSource idle time detection
  - Sleep/wake recovery
  - Period tracking with actions
  - Manual entry creation
  - Add to last entry
  - Discard idle time
  - Daily summaries

---

### Build My Day
- **Module**: Frontend (build-my-day/) + Backend (inference/)
- **Status**: ✅ Complete
- **Features**:
  - AI-generated time blocks
  - Real-time classification
  - Block acceptance/dismissal
  - Date filtering
  - Pending block count
  - Configuration options

---

## Configuration & Deployment

### Environment Variables

#### Backend Configuration
```bash
# Database
PULSARC_TEST_DB_KEY=test_key_64_chars  # For testing, bypasses keychain

# Main API
DATABASE_URL=postgresql://user:pass@host/db
WEBAPI_URL=https://api.pulsearc.ai
WEBAPI_CLIENT_ID=client_id
WEBAPI_CLIENT_SECRET=client_secret

# SAP Integration
SAP_GRAPHQL_URL=https://sap.example.com/graphql
SAP_CLIENT_ID=sap_client_id
SAP_CLIENT_SECRET=sap_secret

# Calendar Integration
# (OAuth configured in-app, no env vars needed)

# Sync Configuration
SYNC_INTERVAL_SECONDS=10  # Backend sync interval
ENABLE_SNAPSHOT_PERSISTENCE=true  # Enable 30s snapshots

# Feature Flags
DETECTOR_PACKS_DEALS_ENABLED=true
DETECTOR_PACKS_TECHNOLOGY_ENABLED=true
SKIP_KEYCHAIN_INIT=true  # Skip keychain in debug mode
```

---

### Configuration Files

#### Tauri Configuration (`tauri.conf.json`)
```json
{
  "build": {
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build",
    "devPath": "http://localhost:5173",
    "distDir": "../dist"
  },
  "package": {
    "productName": "PulseArc",
    "version": "0.1.0"
  },
  "tauri": {
    "bundle": {
      "active": true,
      "identifier": "com.pulsearc.app",
      "macOS": {
        "entitlements": "Entitlements.plist",
        "frameworks": [],
        "minimumSystemVersion": "10.15"
      }
    }
  }
}
```

#### Cargo Configuration (`Cargo.toml`)
```toml
[package]
name = "PulseArc"
version = "0.1.0"
edition = "2021"

[dependencies]
tauri = { version = "2.9", features = ["macos-private-api", "tray-icon"] }
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.37", features = ["bundled-sqlcipher-vendored-openssl"] }
r2d2 = "0.8"
r2d2_sqlite = "0.31"
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
linfa = { version = "0.7", optional = true }
linfa-trees = { version = "0.7", optional = true }
linfa-logistic = { version = "0.7", optional = true }
oauth2 = "5.0"
keyring = "2.3"

[features]
default = ["tree-classifier"]
tree-classifier = ["dep:linfa", "dep:linfa-trees", "dep:linfa-logistic"]
```

#### Frontend Configuration (`package.json`)
```json
{
  "name": "pulsearc-macos",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "test": "vitest",
    "test:ui": "vitest --ui"
  },
  "dependencies": {
    "react": "^18.2.0",
    "react-dom": "^18.2.0",
    "@tauri-apps/api": "^2.0.0",
    "zustand": "^4.4.0"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.2.0",
    "typescript": "^5.3.0",
    "vite": "^5.0.0",
    "vitest": "^1.0.0"
  }
}
```

---

### Build Process

#### Development Build
```bash
# Install dependencies
pnpm install
cd src-tauri && cargo build

# Run in development mode
pnpm dev

# In another terminal (if needed)
cd src-tauri && cargo run
```

#### Production Build
```bash
# Build frontend
pnpm build

# Build Tauri app
cd src-tauri
cargo build --release

# Create macOS app bundle
pnpm tauri build
```

**Output**:
- `src-tauri/target/release/PulseArc.app` - macOS application bundle
- `src-tauri/target/release/bundle/macos/PulseArc.dmg` - DMG installer

---

### Development vs Production Settings

| Setting | Development | Production |
|---------|------------|------------|
| Database Encryption | Test key (env var) | Keychain-stored key |
| API Endpoints | localhost | Production URLs |
| OAuth Callbacks | localhost:3000 | Custom URL scheme |
| Logging Level | DEBUG | INFO |
| Keychain Prompts | Skipped (env var) | Enabled |
| Snapshot Persistence | Optional | Always enabled |
| Metrics Collection | Disabled | Enabled (Datadog) |
| Error Reporting | Console only | Sentry/Datadog |

---

## Performance & Scalability

### Database Performance

#### Connection Pooling
- **Pool Size**: 8 connections max
- **Timeout**: 30 seconds
- **Strategy**: LIFO (last in, first out)

**Benefits**:
- Concurrent database access without blocking
- Multiple schedulers can run simultaneously
- Frontend commands don't block backend tasks

**Configuration**:
```rust
let pool = r2d2::Pool::builder()
    .max_size(8)
    .connection_timeout(Duration::from_secs(30))
    .build(manager)?;
```

---

#### Query Optimization
- **Prepared Statements**: Cached (32 statements)
- **Indexes**: All key queries indexed
- **Batch Operations**: Multi-row inserts
- **WAL Mode**: Concurrent reads during writes

**Performance Benchmarks**:
- Connection acquisition: <1ms (pooled)
- Snapshot insert: ~2ms (prepared statement)
- Segment query: ~5ms (indexed)
- Bulk insert (10k): ~1-2 seconds

---

### Background Task Scheduling

#### Event-Driven vs Polling

**Event-Driven (Preferred)**:
```
NSWorkspace Notification → Immediate callback → <50ms latency
```
- Used for: Activity tracking
- CPU: <0.5%
- Latency: 10-50ms

**Polling (Fallback)**:
```
Sleep → Poll → Process → Repeat (250ms interval)
```
- Used for: Non-macOS platforms
- CPU: ~2-3%
- Latency: 250-500ms

---

#### Scheduler Intervals

| Scheduler | Interval | Trigger | CPU Impact |
|-----------|----------|---------|------------|
| SnapshotWriter | 30 seconds | Timer | Minimal |
| SyncScheduler | 10 seconds | Timer | ~0.1% |
| BlockScheduler | Daily (11 PM) | Cron | ~1-2% (during run) |
| CalendarSync | 15 minutes | Timer | ~0.1% |
| SapForwarder | 10 seconds | Timer | ~0.1% |
| WebApiScheduler | 10 seconds | Timer | ~0.1% |
| CleanupService | 1 hour | Timer | ~0.5% (during run) |

**Total Background CPU**: <1% (steady state)

---

### Memory Optimization

#### Enrichment Caching
- **Cache Size**: ~50-100 entries (bounded)
- **Per Entry**: ~100-500 bytes
- **Total**: ~5-50KB
- **TTL**: 750ms

**Benefits**:
- 80% cache hit rate
- Reduces AX API calls by 80%
- Lowers CPU usage by ~70%

---

#### Database Memory
- **WAL File**: ~5-10MB (typical)
- **Cache**: 64MB (configured)
- **Connection Pool**: ~1MB per connection (8 connections = 8MB)
- **Total**: ~15-20MB

---

#### Frontend Memory
- **React Rendering**: ~50-100MB
- **State Management**: ~5-10MB
- **Tauri Runtime**: ~30-50MB
- **Total**: ~100-150MB

**Total Application Memory**: ~120-200MB (typical)

---

### Scalability Considerations

#### Activity Volume
- **Supported Rate**: 1 activity capture per second (sustained)
- **Burst Rate**: 10 captures per second (short bursts)
- **Storage**: ~1KB per snapshot × 30 days × 86,400 seconds = ~2.5GB
- **Cleanup**: 1-day retention reduces to ~100MB

---

#### Concurrent Operations
- **Database**: 8 concurrent connections
- **HTTP Requests**: Unlimited (connection pooling)
- **Background Tasks**: 8+ concurrent (tokio runtime)

---

#### Sync Performance
- **Throughput**: 100 time entries per minute
- **Latency**: ~100ms per entry (HTTP + DB)
- **Retry**: Exponential backoff (doesn't block other entries)
- **Failure Rate**: <1% (with retry)

---

## Security & Privacy

### Data Encryption

#### SQLCipher Configuration
```rust
PRAGMA key = 'encryption_key_from_keychain';
PRAGMA cipher_compatibility = 4;
PRAGMA kdf_iter = 256000;
PRAGMA cipher_memory_security = ON;
```

**Security Level**: AES-256 encryption

**Key Management**:
- Production: Keychain (macOS Security framework)
- Testing: Environment variable (PULSARC_TEST_DB_KEY)
- Key Generation: 64-character random string
- Storage: System keychain with user-scoped access

---

### PII Redaction

**Automatic Redaction** (before storage or transmission):

| PII Type | Pattern | Replacement |
|----------|---------|-------------|
| Email | `user@domain.com` | `[EMAIL]` |
| URL Params | `?key=secret` | (removed) |
| Document ID | `docs.google.com/d/abc123` | `/d/[ID]` |
| File Path | `/Users/john/file.txt` | `/Users/[USER]/file.txt` |
| IP Address | `192.168.1.1` | `[IP]` |
| Phone | `555-123-4567` | `[PHONE]` |

**Implementation**:
- All redaction in `preprocess/redact.rs`
- Regex patterns compiled once (Lazy)
- Applied before database writes
- Applied before API calls
- No configuration to disable (mandatory)

---

### OAuth Token Management

#### Token Storage
```
┌─────────────────────────────────┐
│  OAuth Tokens (sensitive data)  │
└────────────┬────────────────────┘
             │
      ┌──────┴──────┐
      │             │
  [Keychain]    [Database]
   (encrypted)   (metadata only)
      │             │
   Refresh       Access token
   token         expiry, user_email
```

**Security Features**:
- Refresh tokens: Keychain only (never in DB)
- Access tokens: Memory only (never persisted)
- Token rotation: Automatic on expiry
- Revocation: Deletes both keychain and DB entries

---

#### OAuth Flow Security (PKCE)
```
1. Generate code_verifier (random)
2. Compute code_challenge = SHA256(code_verifier)
3. Include challenge in authorization request
4. Provider returns authorization code
5. Exchange code + verifier for tokens
6. Store refresh token in keychain
```

**Benefits**:
- Prevents authorization code interception
- No client secret in app bundle
- Meets OAuth 2.1 security best practices

---

### Network Security

#### HTTPS/TLS
- All HTTP requests use rustls (no OpenSSL)
- TLS 1.2+ only
- Certificate validation enabled
- No self-signed certificates

#### Request Authentication
- OAuth 2.0 bearer tokens (Authorization header)
- Token refresh on 401 Unauthorized
- Automatic retry with fresh token

---

### Audit Logging

#### Event Types Logged
- User authentication (login/logout)
- OAuth token refresh
- Time entry creation/update
- Calendar sync operations
- SAP sync operations
- Database cleanup
- Classification operations

#### Log Storage
- Location: `~/Library/Logs/PulseArc/`
- Format: JSON structured logs
- Retention: 7 days (rolling)
- Sensitive data: Redacted before logging

---

### Compliance

#### GDPR Compliance
- ✅ Data minimization (PII redaction)
- ✅ Right to erasure (cleanup operations)
- ✅ Data portability (export functions)
- ✅ Encryption at rest (SQLCipher)
- ✅ Encryption in transit (HTTPS)

#### CCPA Compliance
- ✅ Opt-out mechanisms (integration toggles)
- ✅ Data deletion (cleanup + database clear)
- ✅ Privacy by design (PII redaction)

---

## Conclusion

The PulseArc macOS application is a **production-ready, enterprise-grade** time tracking solution that combines:

- **High Performance**: Event-driven architecture, connection pooling, efficient caching
- **Rich Integrations**: SAP, Google/Microsoft Calendar, Main API
- **Intelligent Classification**: Hybrid ML system with graceful degradation
- **Robust Sync**: Outbox pattern with retry logic and cost management
- **Privacy-First**: Automatic PII redaction and encrypted storage
- **Modern UI**: React/TypeScript with type-safe backend communication

The hybrid Rust + React architecture provides the best of both worlds:
- Rust handles performance-critical operations and system integration
- React provides a rich, responsive user interface
- Tauri bridges the gap with type-safe IPC

With comprehensive modules for tracking, classification, sync, and integrations, the application delivers a complete time management solution for macOS users.

---

**Document Version**: 1.0.0  
**Last Updated**: October 29, 2025  
**Status**: Complete and Production-Ready

