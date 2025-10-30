/**
 * FEATURE-009: Sorting utilities for backend data
 *
 * CRITICAL: Backend does NOT guarantee sorted order
 * Always use reduce() to find latest items, never assume [0] is latest
 */

import type { ActivitySnapshot } from '@/shared/types/generated';

/**
 * Find the latest snapshot using reduce()
 * This is the CORRECT pattern - do NOT use array[0]
 *
 * @param snapshots - Array of activity snapshots (may be in any order)
 * @returns Latest snapshot by timestamp, or null if array is empty
 *
 * @example
 * ```ts
 * const snapshots = await invoke<ActivitySnapshot[]>('get_recent_snapshots');
 * const latest = findLatestSnapshot(snapshots);
 * if (latest) {
 *   console.log('Current activity:', latest.detected_activity);
 * }
 * ```
 */
export function findLatestSnapshot(snapshots: ActivitySnapshot[]): ActivitySnapshot | null {
  if (snapshots.length === 0) return null;

  return snapshots.reduce((latest, current) => {
    return current.timestamp > latest.timestamp ? current : latest;
  });
}

/**
 * Generic helper to find the latest item by any timestamp field using reduce()
 *
 * @param items - Array of items with timestamp fields
 * @param timestampField - Which timestamp field to compare (default: 'timestamp')
 * @returns Latest item by specified timestamp field, or null if array is empty
 *
 * @example
 * ```ts
 * const outbox = await invoke<TimeEntryOutbox[]>('get_outbox_status');
 * const latest = findLatestByTimestamp(outbox, 'created_at');
 * ```
 */
export function findLatestByTimestamp<T extends { timestamp?: number; created_at?: number }>(
  items: T[],
  timestampField: keyof T = 'timestamp' as keyof T
): T | null {
  if (items.length === 0) return null;

  return items.reduce((latest, current) => {
    const latestTime = (latest[timestampField] as number) ?? 0;
    const currentTime = (current[timestampField] as number) ?? 0;
    return currentTime > latestTime ? current : latest;
  });
}

/**
 * Sort items by timestamp in descending order (latest first)
 * Returns a NEW array - does not mutate original
 *
 * @param items - Array of items with timestamp field
 * @param timestampField - Which timestamp field to sort by (default: 'timestamp')
 * @returns New sorted array (latest first)
 *
 * @example
 * ```ts
 * const snapshots = await invoke<ActivitySnapshot[]>('get_recent_snapshots');
 * const sorted = sortByTimestampDesc(snapshots);
 * // sorted[0] is guaranteed to be latest
 * ```
 */
export function sortByTimestampDesc<T extends Record<string, unknown>>(
  items: T[],
  timestampField: keyof T = 'timestamp'
): T[] {
  return [...items].sort((a, b) => {
    const aTime = (a[timestampField] as number) ?? 0;
    const bTime = (b[timestampField] as number) ?? 0;
    return bTime - aTime; // Descending order (latest first)
  });
}
