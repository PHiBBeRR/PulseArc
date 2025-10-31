/**
 * FEATURE-009: Wire Frontend Timer UI to Tauri Backend Data
 * Tests for sorting patterns - verifying reduce() is used instead of [0] index
 *
 * CRITICAL: Backend does NOT guarantee sorted order
 * Always use reduce() to find latest snapshot, never assume [0] is latest
 */

import type { ActivitySnapshot } from '@/shared/types/generated';
import { describe, expect, it } from 'vitest';

/**
 * Helper function to find the latest snapshot using reduce()
 * This is the CORRECT pattern - do NOT use array[0]
 */
export function findLatestSnapshot(snapshots: ActivitySnapshot[]): ActivitySnapshot | null {
  if (snapshots.length === 0) return null;

  return snapshots.reduce((latest, current) => {
    return current.timestamp > latest.timestamp ? current : latest;
  });
}

/**
 * Helper function to find the latest item by any timestamp field using reduce()
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

describe('Sorting Patterns - FEATURE-009 Acceptance Criteria', () => {
  describe('findLatestSnapshot - reduce() pattern', () => {
    it('should find latest snapshot when snapshots are in reverse chronological order', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-3',
          timestamp: 1729267300000, // Latest (3rd in array)
          activity_context_json: '{}',
          detected_activity: 'Writing code',
          work_type: null,
          activity_category: 'development',
          primary_app: 'VS Code',
          processed: false,
          batch_id: null,
          created_at: 1729267300000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-2',
          timestamp: 1729267200000, // Middle
          activity_context_json: '{}',
          detected_activity: 'Reading docs',
          work_type: null,
          activity_category: 'research',
          primary_app: 'Chrome',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-1',
          timestamp: 1729267100000, // Oldest
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const latest = findLatestSnapshot(snapshots);

      expect(latest?.id).toBe('snap-3');
      expect(latest?.timestamp).toBe(1729267300000);
      expect(latest?.detected_activity).toBe('Writing code');
    });

    it('should find latest snapshot when snapshots are in chronological order', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-1',
          timestamp: 1729267100000, // Oldest (1st in array)
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-2',
          timestamp: 1729267200000, // Middle
          activity_context_json: '{}',
          detected_activity: 'Reading docs',
          work_type: null,
          activity_category: 'research',
          primary_app: 'Chrome',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-3',
          timestamp: 1729267300000, // Latest (last in array)
          activity_context_json: '{}',
          detected_activity: 'Writing code',
          work_type: null,
          activity_category: 'development',
          primary_app: 'VS Code',
          processed: false,
          batch_id: null,
          created_at: 1729267300000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const latest = findLatestSnapshot(snapshots);

      expect(latest?.id).toBe('snap-3');
      expect(latest?.timestamp).toBe(1729267300000);
      expect(latest?.detected_activity).toBe('Writing code');
    });

    it('should find latest snapshot when snapshots are in random order', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-2',
          timestamp: 1729267200000, // Middle (1st in array)
          activity_context_json: '{}',
          detected_activity: 'Reading docs',
          work_type: null,
          activity_category: 'research',
          primary_app: 'Chrome',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-3',
          timestamp: 1729267300000, // Latest (2nd in array) ✅ CORRECT
          activity_context_json: '{}',
          detected_activity: 'Writing code',
          work_type: null,
          activity_category: 'development',
          primary_app: 'VS Code',
          processed: false,
          batch_id: null,
          created_at: 1729267300000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-1',
          timestamp: 1729267100000, // Oldest (3rd in array)
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const latest = findLatestSnapshot(snapshots);

      // ✅ Should find snap-3 even though it's NOT at index [0]
      expect(latest?.id).toBe('snap-3');
      expect(latest?.timestamp).toBe(1729267300000);
      expect(latest?.detected_activity).toBe('Writing code');
    });

    it('should return null for empty array', () => {
      const snapshots: ActivitySnapshot[] = [];
      const latest = findLatestSnapshot(snapshots);
      expect(latest).toBeNull();
    });

    it('should return the only snapshot when array has one item', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-1',
          timestamp: 1729267100000,
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const latest = findLatestSnapshot(snapshots);

      expect(latest?.id).toBe('snap-1');
    });

    it('should handle snapshots with identical timestamps (picks first in reduce)', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-1',
          timestamp: 1729267200000,
          activity_context_json: '{}',
          detected_activity: 'Activity A',
          work_type: null,
          activity_category: 'work',
          primary_app: 'App A',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-2',
          timestamp: 1729267200000, // Same timestamp
          activity_context_json: '{}',
          detected_activity: 'Activity B',
          work_type: null,
          activity_category: 'work',
          primary_app: 'App B',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const latest = findLatestSnapshot(snapshots);

      // When timestamps are equal, reduce keeps the first one encountered
      expect(latest?.id).toBe('snap-1');
    });
  });

  describe('findLatestByTimestamp - generic helper', () => {
    it('should find latest by timestamp field', () => {
      const items = [
        { id: '1', timestamp: 1000 },
        { id: '2', timestamp: 3000 },
        { id: '3', timestamp: 2000 },
      ];

      const latest = findLatestByTimestamp(items, 'timestamp');

      expect(latest?.id).toBe('2');
      expect(latest?.timestamp).toBe(3000);
    });

    it('should find latest by created_at field', () => {
      const items = [
        { id: '1', created_at: 1000, timestamp: 5000 },
        { id: '2', created_at: 3000, timestamp: 1000 },
        { id: '3', created_at: 2000, timestamp: 2000 },
      ];

      const latest = findLatestByTimestamp(items, 'created_at');

      // Should use created_at, NOT timestamp
      expect(latest?.id).toBe('2');
      expect(latest?.created_at).toBe(3000);
    });

    it('should return null for empty array', () => {
      const items: Array<{ timestamp: number }> = [];
      const latest = findLatestByTimestamp(items);
      expect(latest).toBeNull();
    });

    it('should handle missing timestamps gracefully (treats as 0)', () => {
      const items = [
        { id: '1', timestamp: undefined },
        { id: '2', timestamp: 1000 },
        { id: '3', timestamp: undefined },
      ];

      const latest = findLatestByTimestamp(items, 'timestamp');

      expect(latest?.id).toBe('2');
    });
  });

  describe('Anti-Pattern Tests - What NOT to do', () => {
    it('❌ WRONG: Using [0] assumes first element is latest (FAILS)', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-1',
          timestamp: 1729267100000, // Oldest at [0] ❌
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-3',
          timestamp: 1729267300000, // Latest NOT at [0] ✅
          activity_context_json: '{}',
          detected_activity: 'Writing code',
          work_type: null,
          activity_category: 'development',
          primary_app: 'VS Code',
          processed: false,
          batch_id: null,
          created_at: 1729267300000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      // ❌ WRONG PATTERN - assumes [0] is latest
      const wrongLatest = snapshots[0];
      expect(wrongLatest?.id).toBe('snap-1'); // Gets OLDEST, not latest

      // ✅ CORRECT PATTERN - use reduce()
      const correctLatest = findLatestSnapshot(snapshots);
      expect(correctLatest?.id).toBe('snap-3'); // Gets latest
    });

    it('❌ WRONG: Using sort() mutates original array', () => {
      const snapshots: ActivitySnapshot[] = [
        {
          id: 'snap-1',
          timestamp: 1729267100000, // Oldest FIRST in array
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-2',
          timestamp: 1729267200000, // Latest SECOND in array
          activity_context_json: '{}',
          detected_activity: 'Reading docs',
          work_type: null,
          activity_category: 'research',
          primary_app: 'Chrome',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const originalFirstId = snapshots[0]?.id; // 'snap-1'
      expect(originalFirstId).toBe('snap-1');

      // ❌ WRONG - sort() mutates original array
      snapshots.sort((a, b) => b.timestamp - a.timestamp);
      const wrongLatest = snapshots[0];

      expect(wrongLatest?.id).toBe('snap-2'); // Latest is now at [0]
      expect(snapshots[0]?.id).toBe('snap-2'); // Array was mutated - order changed!
      expect(snapshots[0]?.id).not.toBe(originalFirstId); // Originally was 'snap-1', now 'snap-2'

      // ✅ CORRECT - reduce() doesn't mutate
      const unmutatedSnapshots: ActivitySnapshot[] = [
        {
          id: 'snap-1',
          timestamp: 1729267100000,
          activity_context_json: '{}',
          detected_activity: 'Email',
          work_type: null,
          activity_category: 'communication',
          primary_app: 'Gmail',
          processed: false,
          batch_id: null,
          created_at: 1729267100000,
          is_idle: false,
          idle_duration_secs: 0,
        },
        {
          id: 'snap-2',
          timestamp: 1729267200000,
          activity_context_json: '{}',
          detected_activity: 'Reading docs',
          work_type: null,
          activity_category: 'research',
          primary_app: 'Chrome',
          processed: false,
          batch_id: null,
          created_at: 1729267200000,
          is_idle: false,
          idle_duration_secs: 0,
        },
      ];

      const correctLatest = findLatestSnapshot(unmutatedSnapshots);
      expect(correctLatest?.id).toBe('snap-2');
      expect(unmutatedSnapshots[0]?.id).toBe('snap-1'); // Original order preserved!
    });
  });

  describe('Performance Considerations', () => {
    it('should handle large arrays efficiently with reduce()', () => {
      // Create 10,000 snapshots
      const snapshots: ActivitySnapshot[] = Array.from({ length: 10000 }, (_, i) => ({
        id: `snap-${i}`,
        timestamp: 1729267100000 + i * 1000, // Incrementing timestamps
        activity_context_json: '{}',
        detected_activity: `Activity ${i}`,
        work_type: null,
        activity_category: 'work',
        primary_app: 'App',
        processed: false,
        batch_id: null,
        created_at: 1729267100000 + i * 1000,
        is_idle: false,
        idle_duration_secs: 0,
      }));

      const start = performance.now();
      const latest = findLatestSnapshot(snapshots);
      const duration = performance.now() - start;

      expect(latest?.id).toBe('snap-9999');
      expect(duration).toBeLessThan(50); // Should complete in < 50ms
    });
  });
});
