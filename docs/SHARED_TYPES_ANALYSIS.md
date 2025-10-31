# Shared Types Analysis - Migration Priority

**Generated**: October 31, 2025
**Purpose**: Identify types shared across multiple modules for coordinated migration
**Related**: LEGACY_STRUCT_MAPPING.md, LEGACY_MIGRATION_INVENTORY.md

---

## Executive Summary

**200+ total structs** analyzed across `legacy/api/src/`

**15 types are heavily shared** (used in 3+ modules) and require coordinated migration to the domain layer to avoid circular dependencies.

---

## Critical Shared Types (Used in 5+ Modules)

### 1. `ActivityContext` — **9 references**

**Priority**: 🔴 CRITICAL (Week 1)

**Used By**:
1. `tracker/core.rs` — Activity capture
2. `tracker/providers/macos.rs` — Platform-specific extraction
3. `preprocess/segmenter.rs` — Segmentation
4. `preprocess/redact.rs` — PII redaction
5. `inference/signals.rs` — Signal extraction
6. `inference/block_builder.rs` — Block construction
7. `db/activity/snapshots.rs` — Persistence
8. `commands/blocks.rs` — API retrieval
9. `commands/database.rs` — Stats

**Migration Target**: `domain/types/activity.rs`

**Fields** (13 total):
```rust
pub struct ActivityContext {
    pub active_app: WindowContext,
    pub recent_apps: Vec<WindowContext>,
    pub detected_activity: String,
    pub work_type: Option<WorkType>,
    pub activity_category: ActivityCategory,
    pub billable_confidence: f32,
    pub suggested_client: Option<String>,
    pub suggested_matter: Option<String>,
    pub suggested_task_code: Option<String>,
    pub extracted_metadata: ActivityMetadata,
    pub evidence: ConfidenceEvidence,
    pub calendar_event: Option<CalendarEventContext>,
    pub location: Option<LocationContext>,
    pub temporal_context: Option<TemporalContext>,
    pub classification: Option<ClassificationContext>,
}
```

**Migration Impact**:
- ✅ Already TS-exported
- ⚠️ JSON-serialized in `ActivitySnapshot.activity_context_json`
- ⚠️ Contains nested types that must migrate together: `WindowContext`, `ActivityMetadata`, `ConfidenceEvidence`, all optional context types

**Dependencies**:
- `WindowContext` (5 refs)
- `WorkType` enum
- `ActivityCategory` enum
- `ActivityMetadata`
- `ConfidenceEvidence`
- `CalendarEventContext`
- `LocationContext`
- `TemporalContext`
- `ClassificationContext`

---

### 2. `ActivitySegment` — **7 references**

**Priority**: 🔴 CRITICAL (Week 1)

**Used By**:
1. `preprocess/segmenter.rs` — Creation
2. `db/activity/segments.rs` — Persistence
3. `inference/signals.rs` — Signal extraction from segments
4. `inference/block_builder.rs` — Block aggregation
5. `inference/batch_classifier.rs` — Batch classification
6. `commands/blocks.rs` — API retrieval
7. `commands/database.rs` — Stats

**Migration Target**: `domain/types/database.rs`

**Fields** (14 total):
```rust
pub struct ActivitySegment {
    // Core
    pub id: String,
    pub start_ts: i64,
    pub end_ts: i64,
    pub primary_app: String,
    pub normalized_label: String,
    pub sample_count: i32,
    pub dictionary_keys: Option<String>,
    pub created_at: i64,
    pub processed: bool,
    pub snapshot_ids: Vec<String>,
    
    // REFACTOR-003: Classification fields
    pub work_type: Option<String>,
    pub activity_category: String,
    pub detected_activity: String,
    pub extracted_signals_json: Option<String>, // Serialized ContextSignals
    pub project_match_json: Option<String>,     // Serialized ProjectMatch
    
    // FEATURE-028: Idle tracking
    pub idle_time_secs: i32,
    pub active_time_secs: i32,
    pub user_action: Option<String>,
}
```

**Migration Impact**:
- ✅ TS-exported
- ⚠️ Stores serialized `ContextSignals` and `ProjectMatch` — needs versioned serialization wrappers
- ⚠️ Links snapshots to blocks (traceability)
- ✅ Has `estimated_token_count()` method for OpenAI batching

**Dependencies**:
- `ContextSignals` (serialized)
- `ProjectMatch` (serialized)

---

### 3. `ProposedBlock` — **6 references**

**Priority**: 🔴 CRITICAL (Week 1)

**Used By**:
1. `inference/block_builder.rs` — Creation
2. `inference/batch_classifier.rs` — Classification
3. `inference/project_matcher.rs` — Project assignment
4. `db/blocks/operations.rs` — Persistence
5. `commands/blocks.rs` — API retrieval
6. `integrations/sap/forwarder.rs` — Time entry conversion

**Migration Target**: `domain/types/classification.rs`

**Fields** (23 total — see LEGACY_STRUCT_MAPPING.md for full details)

**Key Features**:
- Extensive metadata (project, workstream, classification)
- Idle handling (`total_idle_secs`, `idle_handling`)
- Location context (`timezone`, `work_location`, `is_travel`)
- Calendar overlap detection (`has_calendar_overlap`, `overlapping_event_ids`)
- Traceability (`snapshot_ids`, `segment_ids`)

**Migration Impact**:
- ✅ TS-exported
- ⚠️ Central to entire classification workflow
- ⚠️ Contains `activities: Vec<ActivityBreakdown>` (embedded type)
- ✅ Has `estimated_token_count()` method

**Dependencies**:
- `ActivityBreakdown`
- `WorkLocation` enum

---

### 4. `TimeEntryOutbox` — **6 references**

**Priority**: 🔴 CRITICAL (Week 1)

**Used By**:
1. `db/outbox/outbox.rs` — Persistence
2. `sync/outbox_worker.rs` — Background sync to Main API
3. `integrations/sap/forwarder.rs` — SAP sync
4. `domain/api/forwarder.rs` — Main API forwarder
5. `commands/idle_sync.rs` — API commands
6. `commands/database.rs` — Stats

**Migration Target**: `domain/types/database.rs`

**Fields** (22 total):
```rust
pub struct TimeEntryOutbox {
    pub id: String,
    pub idempotency_key: String,
    pub user_id: String,
    pub payload_json: String, // Serialized PrismaTimeEntryDto
    pub backend_cuid: Option<String>,
    pub status: OutboxStatus,
    pub attempts: i32,
    pub last_error: Option<String>,
    pub retry_after: Option<i64>,
    pub created_at: i64,
    pub sent_at: Option<i64>,
    
    // FEATURE-020: SAP Integration
    pub correlation_id: Option<String>,
    pub local_status: Option<String>,
    pub remote_status: Option<String>,
    pub sap_entry_id: Option<String>,
    pub next_attempt_at: Option<i64>,
    pub error_code: Option<String>,
    pub last_forwarded_at: Option<i64>,
    pub wbs_code: Option<String>,
    
    // FEATURE-016: Multi-target outbox
    pub target: String, // 'sap' or 'main_api'
    pub description: Option<String>,
    pub auto_applied: bool,
    pub version: i32,
    pub last_modified_by: String,
    pub last_modified_at: Option<i64>,
}
```

**Migration Impact**:
- ✅ TS-exported
- ⚠️ **Dual-target** outbox pattern (Main API + SAP)
- ⚠️ Complex retry logic with exponential backoff
- ⚠️ Stores serialized `PrismaTimeEntryDto`

**Dependencies**:
- `OutboxStatus` enum
- `PrismaTimeEntryDto` (serialized)

---

### 5. `ContextSignals` — **5 references**

**Priority**: 🔴 CRITICAL (Week 1)

**Used By**:
1. `inference/signals.rs` — Extraction from ActivityContext
2. `inference/project_matcher.rs` — Project scoring
3. `inference/block_builder.rs` — Block reasoning
4. `inference/hybrid_classifier.rs` — ML features
5. `db/activity/segments.rs` — Serialized in `extracted_signals_json`

**Migration Target**: `domain/types/classification.rs`

**Fields** (18 total):
```rust
pub struct ContextSignals {
    // Core signals
    pub title_keywords: Vec<String>,
    pub url_domain: Option<String>,
    pub file_path: Option<String>,
    pub project_folder: Option<String>,
    pub calendar_event_id: Option<String>,
    pub attendee_domains: Vec<String>,
    pub app_category: AppCategory,
    pub is_vdr_provider: bool,
    
    // FEATURE-030: Comprehensive classification
    pub timestamp: i64,
    pub project_id: Option<String>,
    pub organizer_domain: Option<String>,
    pub is_screen_locked: bool,
    pub has_personal_event: bool,
    pub is_internal_training: bool,
    pub is_personal_browsing: bool,
    pub email_direction: Option<String>,
    
    // FEATURE-033: Meeting context
    pub has_external_meeting_attendees: bool,
}
```

**Migration Impact**:
- ❌ **NOT TS-exported** (internal type)
- ⚠️ Serialized to `ActivitySegment.extracted_signals_json`
- ⚠️ Requires **versioned wrapper** (`SerializedSignals`)

**Dependencies**:
- `AppCategory` enum
- `SerializedSignals` wrapper

---

### 6. `WindowContext` — **5 references**

**Priority**: 🔴 CRITICAL (Week 1)

**Used By**:
1. `tracker/core.rs` — Capture
2. `tracker/providers/macos.rs` — Platform extraction
3. `detection/enrichers/browser.rs` — URL enrichment
4. `detection/enrichers/office.rs` — File path enrichment
5. `shared/types/mod.rs` — Embedded in `ActivityContext`

**Migration Target**: `domain/types/activity.rs`

**Fields** (6 total):
```rust
pub struct WindowContext {
    pub app_name: String,
    pub window_title: String,
    pub bundle_id: Option<String>,
    
    // Enrichment (Phase 0)
    pub url: Option<String>,
    pub url_host: Option<String>,
    pub document_name: Option<String>,
    pub file_path: Option<String>,
}
```

**Migration Impact**:
- ✅ TS-exported
- ⚠️ Embedded in `ActivityContext` (migrate together)
- ⚠️ Enrichment happens **asynchronously** in background

---

### 7. `BatchStats` — **5 references**

**Priority**: 🟡 MEDIUM (Week 1, simple type)

**Used By**:
1. `db/batch/operations.rs` — Query
2. `commands/database.rs` — Stats API
3. `sync/scheduler.rs` — Monitoring
4. `shared/types/stats.rs` — Definition
5. `db/models.rs` — Re-export

**Migration Target**: `domain/types/stats.rs`

**Fields** (4 total):
```rust
pub struct BatchStats {
    pub pending: i64,
    pub processing: i64,
    pub completed: i64,
    pub failed: i64,
}
```

**Migration Impact**:
- ✅ TS-exported
- ✅ Simple aggregation type (low risk)

---

## Moderately Shared Types (Used in 2-4 Modules)

| Type | Refs | Priority | Target | Notes |
|------|------|----------|--------|-------|
| `IdlePeriod` | 3 | 🟡 Medium | `domain/types/idle.rs` | FEATURE-028 |
| `CalendarEvent` | 3 | 🟢 Low | `domain/types/calendar.rs` | Feature-gated |
| `WbsElement` | 3 | 🟢 Low | `domain/types/sap.rs` | Feature-gated |
| `UserProfile` | 2 | 🟢 Low | `domain/types/user_profile.rs` | Simple CRUD |
| `PrismaTimeEntryDto` | 3 | 🟡 Medium | `domain/types/api.rs` | API contract |
| `OutboxStats` | 2 | 🟢 Low | `domain/types/stats.rs` | Simple stats |
| `DatabaseStats` | 2 | 🟢 Low | `domain/types/stats.rs` | Simple stats |
| `ActivityMetadata` | 3 | 🟡 Medium | `domain/types/activity.rs` | Embedded type |
| `ProjectMatch` | 4 | 🟡 Medium | `domain/types/classification.rs` | Serialized to DB |

---

## Shared Field Analysis

### `wbs_code` — Appears in 5 types
- `WbsElement` (primary key)
- `ProjectMatch` (optional)
- `ProposedBlock` (via ProjectMatch)
- `TimeEntryOutbox` (for SAP sync)
- `AcceptPatch` (for editing)

**Migration Strategy**: Define in domain layer, shared across classification and SAP modules.

---

### `project_id` — Appears in 6 types
- `PrismaTimeEntryDto` (required)
- `ProjectMatch` (optional)
- `ProposedBlock` (optional, as `inferred_project_id`)
- `ContextSignals` (optional)
- `Project`, `ProjectWithWbs` (primary key)

**Migration Strategy**: Standardize ID format (CUID from backend, UUID for local).

---

### `confidence` — Appears in 6 types
- `ProposedBlock.confidence` (classification confidence)
- `ProjectMatch.confidence` (match confidence)
- `ActivityContext.billable_confidence`
- `PrismaTimeEntryDto.confidence` (display only)
- `CalendarEventRow.confidence_score` (parsing confidence)
- `ParsedEventTitle.confidence`

**Migration Strategy**: Document confidence semantics (0.0-1.0, higher = more certain).

---

### Timestamp Fields (Multiple Formats)

**Unix epoch `i64`** (most common):
- `ActivitySnapshot.timestamp`, `created_at`, `processed_at`
- `ActivitySegment.start_ts`, `end_ts`, `created_at`
- `ProposedBlock.start_ts`, `end_ts`, `created_at`, `reviewed_at`
- `IdlePeriod.start_ts`, `end_ts`, `created_at`, `reviewed_at`
- `TimeEntryOutbox.created_at`, `sent_at`, `retry_after`, etc.

**ISO 8601 strings** (for API interop):
- `PrismaTimeEntryDto.entry_date`, `start_time`, `end_time`
- `TimeEntryResponse.created_at`, `updated_at`

**Chrono types** (internal use):
- Tracker, IdleDetector internals

**Migration Strategy**: Standardize on Unix epoch `i64` for storage, ISO 8601 strings for API boundaries.

---

## Migration Dependencies (Critical Path)

### Week 1: Domain Layer Foundation

**Step 1**: Migrate pure data types (zero business logic)
1. `WindowContext`
2. `ActivityMetadata`
3. `ConfidenceEvidence`
4. `CalendarEventContext`
5. `LocationContext`
6. `TemporalContext`
7. `ClassificationContext`

**Step 2**: Migrate enums
1. `WorkType`
2. `ActivityCategory`
3. `BatchStatus`
4. `OutboxStatus`
5. `WorkLocation`
6. `AppCategory`
7. `PauseReason`

**Step 3**: Migrate composite types
1. `ActivityContext` (depends on Step 1 + Step 2)
2. `ActivitySegment`
3. `ActivitySnapshot`
4. `ContextSignals`
5. `ProjectMatch`
6. `ProposedBlock` (depends on `ContextSignals`, `ProjectMatch`)
7. `TimeEntryOutbox`

---

### Week 2: Core Layer Ports

**Define traits** (no impls yet):
1. `ActivityProvider` (used by Tracker)
2. `ActivityRepository` (used by Tracker, Segmenter)
3. `SegmentRepository` (used by Segmenter, BlockBuilder)
4. `BlockRepository` (used by BlockBuilder, Classifier)
5. `Classifier` (used by BatchClassifier)
6. `ProjectMatcher` (used by BlockBuilder)
7. `OutboxQueue` (used by OutboxWorker)
8. `EventProvider` (used by Tracker)

---

### Week 3-4: Infrastructure Implementations

**Implement adapters**:
1. `MacOsProvider implements ActivityProvider`
2. Database repositories (SQLite implementations)
3. Platform-specific event monitors
4. HTTP clients (SAP, Calendar, Main API)

---

### Week 5: API Layer

**Wire everything together**:
1. Tauri commands → Core services
2. DI container (AppContext)
3. Domain ↔ Frontend type mapping

---

## Circular Dependency Risks

### Identified Risks:

1. **ActivityContext ↔ ContextSignals**
   - `ActivityContext` → (serialized in) → `ActivitySnapshot`
   - `ActivitySegment` → (extracts) → `ContextSignals`
   - **Mitigation**: Both live in domain, no circular dep if extraction logic in core

2. **ProposedBlock ↔ ProjectMatch**
   - `ProjectMatch` → (embedded in) → `ProposedBlock`
   - **Mitigation**: Both in domain, ProjectMatch is value type

3. **TimeEntryOutbox ↔ PrismaTimeEntryDto**
   - `TimeEntryOutbox.payload_json` contains serialized `PrismaTimeEntryDto`
   - **Mitigation**: Both in domain, JSON serialization via serde

4. **ActivitySegment → ContextSignals/ProjectMatch (serialized)**
   - `ActivitySegment` stores JSON strings
   - **Mitigation**: Use versioned wrappers (`SerializedSignals`, `SerializedProjectMatch`)

---

## Action Items

### Immediate (Before Migration Starts)

- [x] ✅ Complete struct inventory
- [x] ✅ Identify shared types
- [ ] 🔲 Map circular dependencies
- [ ] 🔲 Validate TS export annotations
- [ ] 🔲 Document serialization formats

### Week 1 Prep

- [ ] 🔲 Create domain crate module structure
- [ ] 🔲 Set up feature flags (calendar, sap, ml)
- [ ] 🔲 Create migration test matrix
- [ ] 🔲 Establish versioned serialization pattern

---

**Document Status**: ✅ COMPLETE

