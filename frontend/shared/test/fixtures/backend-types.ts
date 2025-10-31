/**
 * Test fixture factories for backend types
 * FEATURE-010: Auto-generated type support
 *
 * These factories provide sensible defaults for all generated backend types,
 * making tests more maintainable and reducing boilerplate.
 *
 * Usage:
 *   const snapshot = createMockActivitySnapshot({ id: 'custom-id' });
 */

import type {
  ActivityCategory,
  ActivityContext,
  ActivityMetadata,
  ActivitySegment,
  ActivitySnapshot,
  BatchQueue,
  BatchStatus,
  ConfidenceEvidence,
  DatabaseStats,
  OutboxStatus,
  PrismaTimeEntryDto,
  SyncStats,
  TimeEntryOutbox,
  WindowContext,
} from '@/shared/types/generated';

/* -------------------------------------------------------------------------- */
/* Activity Types                                                             */
/* -------------------------------------------------------------------------- */

/**
 * Factory for ActivitySnapshot with sensible defaults
 *
 * Optional fields (processed_at) are undefined by default
 */
export function createMockActivitySnapshot(
  overrides: Partial<ActivitySnapshot> = {}
): ActivitySnapshot {
  return {
    id: `snap-${Date.now()}`,
    timestamp: Date.now(),
    activity_context_json: JSON.stringify({
      detected_activity: 'Test Activity',
      active_app: { app_name: 'Test App', window_title: 'Test' },
    }),
    detected_activity: 'Test Activity',
    work_type: null,
    activity_category: 'internal',
    primary_app: 'Test App',
    processed: false,
    batch_id: null,
    created_at: Date.now(),
    is_idle: false, // Default to active
    idle_duration_secs: 0, // No idle time by default
    // processed_at is optional, omit by default
    ...overrides,
  };
}

/**
 * Factory for ActivitySegment with sensible defaults
 */
export function createMockActivitySegment(
  overrides: Partial<ActivitySegment> = {}
): ActivitySegment {
  const now = Date.now();
  return {
    id: `seg-${now}`,
    start_ts: now - 300000, // 5 minutes ago
    end_ts: now,
    primary_app: 'Test App',
    normalized_label: 'Test Activity (PII redacted)',
    sample_count: 10,
    dictionary_keys: null,
    created_at: now,
    processed: false,
    snapshot_ids: [],
    work_type: null,
    activity_category: 'unknown',
    detected_activity: 'Test Activity',
    extracted_signals_json: null,
    project_match_json: null,
    idle_time_secs: 0, // No idle time by default
    active_time_secs: 300, // 5 minutes active (300 seconds)
    user_action: null, // No user action by default
    ...overrides,
  };
}

/**
 * Factory for WindowContext with sensible defaults
 */
export function createMockWindowContext(overrides: Partial<WindowContext> = {}): WindowContext {
  return {
    app_name: 'Test App',
    window_title: 'Test Window',
    bundle_id: null,
    url: null,
    url_host: null,
    document_name: null,
    file_path: null,
    ...overrides,
  };
}

/**
 * Factory for ActivityContext with sensible defaults
 */
export function createMockActivityContext(
  overrides: Partial<ActivityContext> = {}
): ActivityContext {
  return {
    active_app: createMockWindowContext(),
    recent_apps: [],
    detected_activity: 'Test Activity',
    work_type: null,
    activity_category: 'internal' as ActivityCategory,
    billable_confidence: 0.5,
    suggested_client: null,
    suggested_matter: null,
    suggested_task_code: null,
    extracted_metadata: createMockActivityMetadata(),
    evidence: createMockConfidenceEvidence(),
    ...overrides,
  };
}

/**
 * Factory for ConfidenceEvidence with sensible defaults
 */
export function createMockConfidenceEvidence(
  overrides: Partial<ConfidenceEvidence> = {}
): ConfidenceEvidence {
  return {
    reasons: ['test_evidence'],
    ...overrides,
  };
}

/**
 * Factory for ActivityMetadata with sensible defaults
 */
export function createMockActivityMetadata(
  overrides: Partial<ActivityMetadata> = {}
): ActivityMetadata {
  return {
    document_name: null,
    file_path: null,
    project_code: null,
    client_identifier: null,
    matter_number: null,
    email_subject: null,
    ...overrides,
  };
}

/* -------------------------------------------------------------------------- */
/* Batch & Queue Types                                                        */
/* -------------------------------------------------------------------------- */

/**
 * Factory for BatchQueue with sensible defaults
 *
 * Optional fields (processed_at, processing_started_at, lease_expires_at) are undefined by default
 */
export function createMockBatchQueue(overrides: Partial<BatchQueue> = {}): BatchQueue {
  return {
    batch_id: `batch-${Date.now()}`,
    activity_count: 10,
    status: 'pending' as BatchStatus,
    created_at: Date.now(),
    // processed_at is optional, omit by default
    error_message: null,
    // processing_started_at is optional, omit by default
    worker_id: null,
    // lease_expires_at is optional, omit by default
    time_entries_created: 0,
    openai_cost: 0,
    ...overrides,
  };
}

/* -------------------------------------------------------------------------- */
/* Outbox Types                                                               */
/* -------------------------------------------------------------------------- */

/**
 * Factory for TimeEntryOutbox with sensible defaults
 *
 * Optional fields (retry_after, sent_at) are undefined by default
 */
export function createMockTimeEntryOutbox(
  overrides: Partial<TimeEntryOutbox> = {}
): TimeEntryOutbox {
  const now = Date.now();
  return {
    id: `outbox-${now}`,
    idempotency_key: `idem-${now}`,
    user_id: 'user-test-123',
    payload_json: JSON.stringify(createMockPrismaTimeEntryDto()),
    backend_cuid: null,
    status: 'pending' as OutboxStatus,
    attempts: 0,
    last_error: null,
    // retry_after is optional, omit by default
    created_at: now,
    // sent_at is optional, omit by default
    // : SAP Integration fields (all null by default in Phase 0)
    correlation_id: null,
    local_status: null,
    remote_status: null,
    sap_entry_id: null,
    // next_attempt_at is optional, omit by default
    error_code: null,
    // last_forwarded_at is optional, omit by default
    wbs_code: null,
    target: 'api', // FEATURE-016: Required field for multi-target support
    description: null,
    auto_applied: false,
    version: 1,
    last_modified_by: 'system',
    // last_modified_at is optional, omit by default
    ...overrides,
  };
}

/**
 * Factory for PrismaTimeEntryDto with sensible defaults
 */
export function createMockPrismaTimeEntryDto(
  overrides: Partial<PrismaTimeEntryDto> = {}
): PrismaTimeEntryDto {
  const defaults = {
    id: null,
    orgId: 'org-test-123',
    projectId: 'proj-test-456',
    taskId: null,
    userId: 'user-test-789',
    entryDate: new Date().toISOString().split('T')[0], // YYYY-MM-DD
    durationMinutes: 60,
    notes: null,
    billable: null,
    source: 'ai',
    status: null,
    startTime: null,
    endTime: null,
    durationSec: null,
  };

  return { ...defaults, ...overrides } as PrismaTimeEntryDto;
}

/* -------------------------------------------------------------------------- */
/* Stats Types                                                                */
/* -------------------------------------------------------------------------- */

/**
 * Factory for SyncStats with sensible defaults
 *
 * Optional fields (last_sync_time) are undefined by default
 */
export function createMockSyncStats(overrides: Partial<SyncStats> = {}): SyncStats {
  return {
    local_activity_count: 0,
    pending_batches: 0,
    failed_batches: 0,
    // last_sync_time is optional, omit by default
    ...overrides,
  };
}

/**
 * Factory for DatabaseStats with sensible defaults
 */
export function createMockDatabaseStats(overrides: Partial<DatabaseStats> = {}): DatabaseStats {
  return {
    snapshot_count: 0,
    unprocessed_count: 0,
    segment_count: 0,
    batch_stats: {
      pending: 0,
      processing: 0,
      completed: 0,
      failed: 0,
    },
    ...overrides,
  };
}

/* -------------------------------------------------------------------------- */
/* Helper Functions                                                           */
/* -------------------------------------------------------------------------- */

/**
 * Create an array of mock ActivitySnapshots
 *
 * @param count - Number of snapshots to create
 * @param overrides - Overrides to apply to all snapshots
 */
export function createMockActivitySnapshots(
  count: number,
  overrides: Partial<ActivitySnapshot> = {}
): ActivitySnapshot[] {
  return Array.from({ length: count }, (_, i) =>
    createMockActivitySnapshot({
      id: `snap-${i}`,
      timestamp: Date.now() - i * 30000, // 30 seconds apart
      ...overrides,
    })
  );
}

/**
 * Create an array of mock TimeEntryOutbox items
 *
 * @param count - Number of outbox entries to create
 * @param overrides - Overrides to apply to all entries
 */
export function createMockTimeEntryOutboxes(
  count: number,
  overrides: Partial<TimeEntryOutbox> = {}
): TimeEntryOutbox[] {
  return Array.from({ length: count }, (_, i) =>
    createMockTimeEntryOutbox({
      id: `outbox-${i}`,
      ...overrides,
    })
  );
}

/* -------------------------------------------------------------------------- */
/* FEATURE-015: Calendar Types                                                */
/* -------------------------------------------------------------------------- */

/**
 * Factory for CalendarEvent with sensible defaults
 * FEATURE-015: Google Calendar integration
 */
export function createMockCalendarEvent(
  overrides: Record<string, unknown> = {}
): Record<string, unknown> {
  const now = Date.now();
  return {
    id: `cal-event-${now}`,
    summary: 'Test Event',
    description: null,
    start: Math.floor(now / 1000),
    end: Math.floor((now + 3600000) / 1000), // +1 hour
    calendar_id: 'primary',
    is_all_day: false,
    recurring_event_id: null,
    original_start_time: null,
    parsed_project: null,
    parsed_workstream: null,
    parsed_task: 'Test Event',
    ...overrides,
  };
}

/**
 * Factory for CalendarConnectionStatus
 * FEATURE-015: OAuth connection status
 */
export function createMockCalendarConnectionStatus(
  overrides: Record<string, unknown> = {}
): Record<string, unknown> {
  return {
    connected: false,
    email: null,
    last_sync: null,
    sync_enabled: false,
    ...overrides,
  };
}

/**
 * Factory for CalendarSyncSettings
 * FEATURE-015: Sync configuration
 */
export function createMockCalendarSyncSettings(
  overrides: Record<string, unknown> = {}
): Record<string, unknown> {
  return {
    enabled: true,
    sync_interval_minutes: 30,
    include_all_day_events: false,
    min_event_duration_minutes: 15,
    lookback_hours: 4,
    lookahead_hours: 1,
    excluded_calendar_ids: [],
    sync_token: null,
    last_sync_epoch: null,
    ...overrides,
  };
}

/**
 * Factory for TimelineCalendarEvent
 * FEATURE-015: Timeline visualization
 */
export function createMockTimelineCalendarEvent(
  overrides: Record<string, unknown> = {}
): Record<string, unknown> {
  const now = Date.now();
  return {
    id: `timeline-cal-${now}`,
    project: 'Test Project',
    task: 'Test Task',
    start_time: '10:00',
    start_epoch: Math.floor(now / 1000),
    duration: 60,
    status: 'suggested',
    is_calendar_event: true,
    is_all_day: false,
    original_summary: 'Test Event',
    ...overrides,
  };
}
