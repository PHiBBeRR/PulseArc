# PulseArc Tauri Commands Reference

**Last Updated:** 2025-01-10
**Total Commands:** 42

This document provides a comprehensive reference of all Tauri commands exposed by the PulseArc backend to the frontend.

---

## Command Categories

- [Activity Tracking](#activity-tracking) (4 commands)
- [Projects](#projects) (1 command)
- [Suggestions & Proposed Blocks](#suggestions--proposed-blocks) (6 commands)
- [Block Management](#block-management) (3 commands)
- [Calendar Integration](#calendar-integration) (6 commands)
- [Database Management](#database-management) (5 commands)
- [Feature Flags](#feature-flags) (3 commands)
- [Health Check](#health-check) (1 command)
- [User Profile](#user-profile) (2 commands)
- [Window Management](#window-management) (1 command)
- [Idle Period Management](#idle-period-management) (5 commands)
- [Idle Sync Telemetry](#idle-sync-telemetry) (7 commands)
- [Debug Commands](#debug-commands) (1 command)

---

## Activity Tracking

### `get_activity`
**Phase:** Core tracking
**Returns:** `ActivityContext`
**Description:** Captures and returns the current activity context including active window, application, and timing information.

**Frontend Usage:** Used by activity monitoring UI

---

### `pause_tracker`
**Phase:** Core tracking
**Returns:** `()`
**Description:** Pauses the activity tracking service, stopping capture of new activity data.

**Frontend Usage:** ✅ 4 uses - Pause/stop buttons in UI

---

### `resume_tracker`
**Phase:** Core tracking
**Returns:** `()`
**Description:** Resumes activity tracking after being paused.

**Frontend Usage:** ✅ 5 uses - Resume/start buttons in UI

---

### `save_time_entry`
**Phase:** Core tracking
**Parameters:**
- `ctx: State<Arc<AppContext>>`
- `entry: TimeEntry`

**Returns:** `Result<()>`
**Description:** Manually saves a time entry (e.g., from user input or edit).

**Frontend Usage:** Manual time entry forms

---

## Projects

### `get_user_projects`
**Phase:** Core
**Returns:** `Vec<Project>`
**Description:** Retrieves all projects associated with the current user.

**Frontend Usage:** ❌ Not yet invoked - Ready for project picker UI

---

## Suggestions & Proposed Blocks

### `get_dismissed_suggestions`
**Phase:** Phase 4B.1
**Parameters:**
- `day_epoch: Option<i64>` - Unix timestamp for start of day
- `status: Option<String>` - Filter by status

**Returns:** `Vec<ProposedBlock>`
**Description:** Retrieves dismissed block suggestions for review or restoration.

**Frontend Usage:** ❌ Not yet invoked - Ready for dismissed items view

---

### `get_proposed_blocks`
**Phase:** Phase 4B.1
**Parameters:**
- `day_epoch: i64` - Unix timestamp for start of day
- `status: Option<String>` - Filter by status (suggested, pending_classification, etc.)

**Returns:** `Vec<ProposedBlock>`
**Description:** Fetches proposed time blocks for a given day, optionally filtered by status.

**Frontend Usage:** ❌ Not yet invoked - Ready for block timeline UI

---

### `get_outbox_status`
**Phase:** Phase 4B.1
**Parameters:**
- `status: Option<OutboxStatus>` - Filter by outbox status

**Returns:** `Vec<TimeEntryOutbox>`
**Description:** Retrieves time entries in the outbox (pending sync to external systems).

**Frontend Usage:** ❌ Not yet invoked - Ready for sync status UI

---

### `clear_suggestions`
**Phase:** Phase 4B.1
**Returns:** `Result<()>`
**Description:** Clears all pending suggestions (destructive operation).

**Frontend Usage:** Clear/reset functionality

---

### `delete_suggestion`
**Phase:** Phase 4B.1
**Parameters:**
- `suggestion_id: String`

**Returns:** `Result<()>`
**Description:** Permanently deletes a specific suggestion by ID.

**Frontend Usage:** Delete action in suggestion UI

---

### `dismiss_suggestion`
**Phase:** Phase 4B.1
**Parameters:**
- `suggestion_id: String`

**Returns:** `Result<()>`
**Description:** Dismisses a suggestion (soft delete - can be restored).

**Frontend Usage:** Dismiss/reject action in suggestion UI

---

### `restore_suggestion`
**Phase:** Phase 4B.1
**Parameters:**
- `suggestion_id: String`

**Returns:** `Result<()>`
**Description:** Restores a previously dismissed suggestion.

**Frontend Usage:** Undo dismiss / restore from trash

---

### `update_suggestion`
**Phase:** Phase 4B.1
**Parameters:**
- `suggestion_id: String`
- `updated_data: ProposedBlock`

**Returns:** `Result<()>`
**Description:** Updates an existing suggestion with new data.

**Frontend Usage:** Edit suggestion in UI

---

## Block Management

### `build_my_day`
**Phase:** Phase 4B.1
**Parameters:**
- `day_epoch: Option<i64>` - Unix timestamp for start of day (defaults to today)

**Returns:** `Vec<ProposedBlock>`
**Description:** Builds time blocks for a specific day from activity segments. Idempotent - returns existing blocks if already built.

**Frontend Usage:** ❌ Not yet invoked - Ready for "Build My Day" button

---

### `accept_proposed_block`
**Phase:** Phase 4B.1
**Parameters:**
- `block_id: String`

**Returns:** `Result<()>`
**Description:** Accepts a proposed block and enqueues it for sync to external systems (SAP, etc.).

**Frontend Usage:** ✅ 1 use - Accept/approve block action

---

### `dismiss_proposed_block`
**Phase:** Phase 4B.1
**Parameters:**
- `block_id: String`

**Returns:** `Result<()>`
**Description:** Rejects/dismisses a proposed block.

**Frontend Usage:** ✅ 1 use - Reject/dismiss block action

---

## Calendar Integration

### `initiate_calendar_auth`
**Phase:** Phase 4B.2
**Returns:** `Result<String>` - OAuth authorization URL
**Description:** Initiates OAuth flow for Google Calendar integration. Returns URL for user to authorize.

**Frontend Usage:** ✅ 1 use - Connect calendar button

---

### `disconnect_calendar`
**Phase:** Phase 4B.2
**Returns:** `Result<()>`
**Description:** Disconnects the linked calendar account and clears stored credentials.

**Frontend Usage:** ✅ 3 uses - Disconnect/unlink calendar

---

### `get_calendar_connection_status`
**Phase:** Phase 4B.2
**Returns:** `CalendarConnectionStatus`
**Description:** Checks if a calendar is connected and returns connection metadata.

**Frontend Usage:** ✅ 4 uses - Display connection status in settings

---

### `sync_calendar_events`
**Phase:** Phase 4B.2
**Returns:** `Result<u32>` - Number of events synced
**Description:** Manually triggers a sync of calendar events from the connected account.

**Frontend Usage:** ✅ 2 uses - Manual sync button

---

### `get_calendar_events_for_timeline`
**Phase:** Phase 4B.2
**Parameters:**
- `start_ts: i64` - Start timestamp
- `end_ts: i64` - End timestamp

**Returns:** `Vec<CalendarEvent>`
**Description:** Retrieves calendar events within a time range for timeline display.

**Frontend Usage:** ✅ 5 uses - Timeline/calendar view

---

### `get_calendar_sync_settings`
**Phase:** Phase 4B.2
**Returns:** `CalendarSyncSettings`
**Description:** Retrieves current calendar sync configuration (auto-sync interval, filters, etc.).

**Frontend Usage:** ✅ 3 uses - Calendar settings UI

---

### `update_calendar_sync_settings`
**Phase:** Phase 4B.2
**Parameters:**
- `settings: CalendarSyncSettings`

**Returns:** `Result<()>`
**Description:** Updates calendar sync settings with new configuration.

**Frontend Usage:** ✅ 5 uses - Save settings in calendar config

---

## Database Management

### `get_database_stats`
**Phase:** Phase 4A.1
**Returns:** `DatabaseStats`
**Description:** Returns database statistics including size, row counts, and performance metrics.

**Frontend Usage:** ❌ Not yet invoked - Ready for admin/debug UI

---

### `get_recent_snapshots`
**Phase:** Phase 4A.1
**Parameters:**
- `limit: Option<u32>` - Max number of snapshots to return

**Returns:** `Vec<ActivitySnapshot>`
**Description:** Retrieves recent activity snapshots for debugging or review.

**Frontend Usage:** ❌ Not yet invoked - Ready for activity history view

---

### `vacuum_database`
**Phase:** Phase 4A.1
**Returns:** `Result<()>`
**Description:** Runs VACUUM on the SQLCipher database to reclaim space and optimize performance.

**Frontend Usage:** ❌ Not yet invoked - Ready for maintenance UI

---

### `get_database_health`
**Phase:** Phase 4A.1
**Returns:** `DatabaseHealth`
**Description:** Checks database integrity and returns health status.

**Frontend Usage:** ❌ Not yet invoked - Ready for admin dashboard

---

### `clear_snapshots`
**Phase:** Phase 4A.1
**Parameters:**
- `before_ts: Option<i64>` - Clear snapshots before this timestamp

**Returns:** `Result<u32>` - Number of snapshots cleared
**Description:** Clears old activity snapshots to free up space.

**Frontend Usage:** ❌ Not yet invoked - Ready for data cleanup UI

---

## Feature Flags

### `is_feature_enabled`
**Phase:** Phase 4
**Parameters:**
- `flag_name: String`

**Returns:** `bool`
**Description:** Checks if a specific feature flag is enabled.

**Frontend Usage:** ❌ Not yet invoked - Ready for feature gating

---

### `toggle_feature_flag`
**Phase:** Phase 4
**Parameters:**
- `flag_name: String`
- `enabled: bool`

**Returns:** `Result<()>`
**Description:** Enables or disables a feature flag.

**Frontend Usage:** ❌ Not yet invoked - Ready for admin/debug settings

---

### `list_feature_flags`
**Phase:** Phase 4
**Returns:** `Vec<FeatureFlag>`
**Description:** Lists all available feature flags with their current state.

**Frontend Usage:** ❌ Not yet invoked - Ready for feature flag UI

---

## Health Check

### `get_app_health`
**Phase:** Phase 4.1.6
**Returns:** `AppHealth`
**Description:** Returns comprehensive application health status including service states, database connection, and system metrics.

**Frontend Usage:** ❌ Not yet invoked - Ready for system health dashboard

---

## User Profile

### `get_user_profile`
**Phase:** Phase 4A.2
**Returns:** `Option<UserProfile>`
**Description:** Retrieves the current user's profile information.

**Frontend Usage:** ❌ Not yet invoked - Ready for user settings

---

### `upsert_user_profile`
**Phase:** Phase 4A.2
**Parameters:**
- `profile: UserProfile`

**Returns:** `Result<()>`
**Description:** Creates or updates user profile information.

**Frontend Usage:** ❌ Not yet invoked - Ready for profile edit UI

---

## Window Management

### `animate_window_resize`
**Phase:** Phase 4A.3
**Parameters:**
- `target_height: f64` - Target window height in pixels
- `duration_ms: u64` - Animation duration in milliseconds

**Returns:** `Result<()>`
**Description:** Smoothly animates the application window to a new height (macOS only).

**Frontend Usage:** ✅ 1 use - Expand/collapse animations

---

## Idle Period Management

### `get_idle_periods`
**Phase:** Phase 4B.3
**Parameters:**
- `start_ts: Option<i64>` - Start timestamp filter
- `end_ts: Option<i64>` - End timestamp filter

**Returns:** `Vec<IdlePeriod>`
**Description:** Retrieves detected idle periods within a time range.

**Frontend Usage:** ❌ Not yet invoked - Ready for idle period review UI

---

### `update_idle_period_action`
**Phase:** Phase 4B.3
**Parameters:**
- `idle_period_id: String`
- `action: IdlePeriodAction` - (keep_tracking, discard_time, etc.)

**Returns:** `Result<()>`
**Description:** Updates the user's chosen action for a detected idle period.

**Frontend Usage:** ❌ Not yet invoked - Ready for idle resolution UI

---

### `get_idle_summary`
**Phase:** Phase 4B.3
**Parameters:**
- `day_epoch: Option<i64>` - Unix timestamp for start of day

**Returns:** `IdleSummary`
**Description:** Returns summary statistics of idle time for a given day.

**Frontend Usage:** ❌ Not yet invoked - Ready for daily summary UI

---

### `get_idle_settings`
**Phase:** Phase 4B.3 (Configuration)
**Returns:** `IdleSettings`
**Description:** Retrieves current idle detection settings (threshold, enabled state).

**Frontend Usage:** Idle settings configuration

---

### `set_idle_enabled`
**Phase:** Phase 4B.3 (Configuration)
**Parameters:**
- `enabled: bool`

**Returns:** `Result<()>`
**Description:** Enables or disables idle detection globally.

**Frontend Usage:** Toggle idle detection in settings

---

### `set_idle_threshold`
**Phase:** Phase 4B.3 (Configuration)
**Parameters:**
- `threshold_seconds: u32`

**Returns:** `Result<()>`
**Description:** Sets the idle threshold (number of seconds of inactivity before marking as idle).

**Frontend Usage:** Idle threshold slider/input in settings

---

## Idle Sync Telemetry

These commands track idle detection and timer synchronization for debugging and metrics.

### `record_idle_detection`
**Phase:** Phase 4C.2
**Parameters:**
- `idle_start: i64` - Timestamp when idle started
- `idle_duration_secs: u32` - Duration of idle period

**Returns:** `Result<()>`
**Description:** Records an idle detection event for telemetry.

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

### `record_activity_wake`
**Phase:** Phase 4C.2
**Parameters:**
- `wake_ts: i64` - Timestamp when activity resumed

**Returns:** `Result<()>`
**Description:** Records when user activity resumes after idle.

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

### `record_timer_event_emission`
**Phase:** Phase 4C.2
**Parameters:**
- `event_type: String` - Type of timer event
- `payload: serde_json::Value` - Event data

**Returns:** `Result<()>`
**Description:** Records when a timer event is emitted from backend.

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

### `record_timer_event_reception`
**Phase:** Phase 4C.2
**Parameters:**
- `event_type: String`
- `received_ts: i64`

**Returns:** `Result<()>`
**Description:** Records when frontend receives a timer event (measures latency).

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

### `record_invalid_payload`
**Phase:** Phase 4C.2
**Parameters:**
- `error_details: String`

**Returns:** `Result<()>`
**Description:** Records parsing/validation errors in event payloads.

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

### `record_state_transition`
**Phase:** Phase 4C.2
**Parameters:**
- `from_state: String`
- `to_state: String`
- `transition_ts: i64`

**Returns:** `Result<()>`
**Description:** Records application state transitions for debugging.

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

### `record_auto_start_tracker_rule`
**Phase:** Phase 4C.2
**Parameters:**
- `rule_name: String`
- `triggered: bool`

**Returns:** `Result<()>`
**Description:** Records when auto-start rules are evaluated and triggered.

**Frontend Usage:** ✅ 1 use - Automatic telemetry

---

## Debug Commands

### `seed_activity_snapshots`
**Phase:** Phase 4E.1
**Availability:** `#[cfg(debug_assertions)]` - **DEBUG BUILDS ONLY**
**Parameters:**
- `count: u32` - Number of snapshots to seed
- `start_ts: Option<i64>` - Starting timestamp

**Returns:** `Result<u32>` - Number of snapshots created
**Description:** Seeds the database with synthetic activity snapshots for development and testing.

**Frontend Usage:** ❌ Not yet invoked - Available in debug builds for testing

---

## Deprecated Commands

The following commands were referenced in old documentation but are **NOT** implemented in the current codebase:

| Deprecated Command | Replacement |
|-------------------|-------------|
| `disconnect_google_calendar` | ✅ Use `disconnect_calendar` |
| `initiate_google_calendar_auth` | ✅ Use `initiate_calendar_auth` |
| `open_ai_entry` | ❌ Removed - functionality deprecated |
| `save_manual_activity` | ✅ Use `save_time_entry` |
| `clear_local_activities` | ✅ Use `clear_snapshots` |
| `clear_outbox` | ✅ Use `clear_suggestions` |
| `delete_outbox_entry` | ✅ Use `delete_suggestion` |
| `get_cost_summary` | ❌ Removed - functionality deprecated |

---

## Frontend Integration Status

### ✅ **Active Commands** (18 commands in use)
Commands actively called by the frontend:
- Activity tracking: `pause_tracker`, `resume_tracker`
- Block management: `accept_proposed_block`, `dismiss_proposed_block`
- Calendar: All 6 calendar commands
- Window: `animate_window_resize`
- Idle telemetry: All 7 telemetry commands

### 🔶 **Ready But Unused** (24 commands)
Fully implemented backend commands not yet integrated in frontend:
- `get_activity`, `save_time_entry`
- `get_user_projects`
- All suggestion commands
- `build_my_day`, `get_proposed_blocks`
- All database management commands
- All feature flag commands
- `get_app_health`
- All user profile commands
- All idle period management commands

### 📊 **Usage Summary**
- **Total Commands:** 42
- **Frontend Active:** 18 (43%)
- **Ready for Integration:** 24 (57%)
- **Debug Only:** 1

---

## Architecture Notes

All commands follow the **hexagonal architecture** pattern:
- Commands reside in `crates/api/src/commands/`
- Business logic in `crates/core/`
- Infrastructure in `crates/infra/`
- Domain types in `crates/domain/`

Commands use:
- **Dependency Injection** via `AppContext`
- **Structured Logging** with `tracing`
- **Metrics Recording** for observability
- **Error Handling** via `Result<T, PulseArcError>`

---

## Related Documentation

- [Architecture Decision Records](../docs/adr/)
- [Phase 4 Migration Guide](../docs/migration/)
- [API Type Definitions](../frontend/shared/types/generated/)
- [Development Setup](../README.md)

---

**Note:** This documentation is automatically verified against `crates/api/src/main.rs`. Any discrepancies indicate the codebase has changed and this document needs updating.
