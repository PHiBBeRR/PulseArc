/**
 * FEATURE-010: Tauri Backend API Wrappers
 *
 * This file provides API wrappers with timestamp normalization for Tauri commands.
 * The actual type definitions are auto-generated from Rust via ts-rs.
 *
 * Timestamp Normalization:
 * - Backend sends Unix timestamps in SECONDS
 * - Frontend expects timestamps in MILLISECONDS
 * - Heuristic: values < 10 billion are seconds, >= 10 billion are already ms
 */

import { invoke } from '@tauri-apps/api/core';

// Re-export all generated types
export type {
  ActivitySnapshot,
  ActivitySegment,
  ActivityContext,
  WindowContext,
  WorkType,
  ActivityCategory,
  ConfidenceEvidence,
  ActivityMetadata,
  BatchQueue,
  BatchStatus,
  BatchStats,
  TimeEntryOutbox,
  OutboxStatus,
  PrismaTimeEntryDto,
  IdMapping,
  DatabaseStats,
  SyncStats,
  OutboxStats,
  DlqBatch,
} from './generated';

import type {
  ActivitySnapshot,
  ActivitySegment,
  BatchQueue,
  TimeEntryOutbox,
  SyncStats,
  DatabaseStats,
} from './generated';

/* -------------------------------------------------------------------------- */
/* Timestamp Normalization Helpers                                           */
/* -------------------------------------------------------------------------- */

/**
 * Convert epoch timestamp to milliseconds using heuristic:
 * - If value < 10 billion, assume seconds and multiply by 1000
 * - If value >= 10 billion, assume already in milliseconds
 *
 * @param epoch - Unix timestamp (may be in seconds or milliseconds)
 * @returns Timestamp in milliseconds
 */
function toMs(epoch: number): number {
  return epoch < 10_000_000_000 ? epoch * 1000 : epoch;
}

/**
 * Normalize timestamp fields in an object
 *
 * @param obj - Object with timestamp fields
 * @param timestampFields - Array of field names to normalize
 * @returns Object with normalized timestamps
 */
function normalizeTimestamps<T extends Record<string, unknown>>(
  obj: T,
  timestampFields: string[]
): T {
  const normalized = { ...obj } as Record<string, unknown>;
  for (const field of timestampFields) {
    const value = normalized[field];
    if (value !== null && value !== undefined && typeof value === 'number') {
      normalized[field] = toMs(value);
    }
  }
  return normalized as T;
}

/* -------------------------------------------------------------------------- */
/* Activity Snapshot API Wrappers                                            */
/* -------------------------------------------------------------------------- */

/**
 * Get recent activity snapshots with normalized timestamps
 *
 * @returns Array of activity snapshots with timestamps in milliseconds
 */
export async function getRecentSnapshots(): Promise<ActivitySnapshot[]> {
  const data = await invoke<ActivitySnapshot[]>('get_recent_snapshots');
  return data.map((s) =>
    normalizeTimestamps(s, ['timestamp', 'created_at', 'processed_at'])
  );
}

/**
 * Get recent activities with optional limit and normalized timestamps
 *
 * @param limit - Maximum number of activities to return (optional)
 * @returns Array of activity snapshots with timestamps in milliseconds
 */
export async function getRecentActivities(limit?: number): Promise<ActivitySnapshot[]> {
  const data = await invoke<ActivitySnapshot[]>('get_recent_activities', { limit });
  return data.map((s) =>
    normalizeTimestamps(s, ['timestamp', 'created_at', 'processed_at'])
  );
}

/* -------------------------------------------------------------------------- */
/* Activity Segment API Wrappers                                             */
/* -------------------------------------------------------------------------- */

/**
 * Get recent activity segments with normalized timestamps
 *
 * @returns Array of activity segments with timestamps in milliseconds
 */
export async function getRecentSegments(): Promise<ActivitySegment[]> {
  const data = await invoke<ActivitySegment[]>('get_recent_segments');
  return data.map((s) =>
    normalizeTimestamps(s, ['start_ts', 'end_ts', 'created_at'])
  );
}

/* -------------------------------------------------------------------------- */
/* Batch Queue API Wrappers                                                  */
/* -------------------------------------------------------------------------- */

/**
 * Get batch queue status with normalized timestamps
 *
 * @returns Array of batch queue entries with timestamps in milliseconds
 */
export async function getBatchStatus(): Promise<BatchQueue[]> {
  const data = await invoke<BatchQueue[]>('get_batch_status');
  return data.map((b) =>
    normalizeTimestamps(b, [
      'created_at',
      'processed_at',
      'processing_started_at',
      'lease_expires_at',
    ])
  );
}

/* -------------------------------------------------------------------------- */
/* Outbox API Wrappers                                                       */
/* -------------------------------------------------------------------------- */

/**
 * Get outbox status with normalized timestamps
 *
 * @returns Array of outbox entries with timestamps in milliseconds
 */
export async function getOutboxStatus(): Promise<TimeEntryOutbox[]> {
  const data = await invoke<TimeEntryOutbox[]>('get_outbox_status');
  return data.map((o) =>
    normalizeTimestamps(o, ['created_at', 'sent_at', 'retry_after'])
  );
}

/* -------------------------------------------------------------------------- */
/* Stats API Wrappers                                                        */
/* -------------------------------------------------------------------------- */

/**
 * Get sync statistics with normalized timestamps
 *
 * @returns Sync stats with timestamps in milliseconds
 */
export async function getSyncStats(): Promise<SyncStats> {
  const data = await invoke<SyncStats>('get_sync_stats');
  return normalizeTimestamps(data, ['last_sync_time']);
}

/**
 * Get database statistics (no timestamp normalization needed)
 *
 * @returns Database statistics
 */
export async function getDatabaseStats(): Promise<DatabaseStats> {
  return await invoke<DatabaseStats>('get_database_stats');
}

/**
 * Get cost summary (placeholder - may need to be implemented on backend)
 *
 * @returns Cost summary data
 */
export async function getCostSummary(): Promise<unknown> {
  return await invoke('get_cost_summary');
}
