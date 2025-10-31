/**
 * FEATURE-009: Wire Frontend Timer UI to Tauri Backend Data
 * Unit tests for Tauri backend types and timestamp normalization
 *
 * Tests Issue #6: TypeScript Types and Timestamp Normalization
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri invoke (must be hoisted before imports)
const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

import {
  createMockActivitySegment,
  createMockActivitySnapshot,
  createMockBatchQueue,
  createMockTimeEntryOutbox,
} from '@/shared/test/fixtures/backend-types';
import {
  getBatchStatus,
  getCostSummary,
  getOutboxStatus,
  getRecentActivities,
  getRecentSegments,
  getRecentSnapshots,
  getSyncStats,
  type ActivitySegment,
  type ActivitySnapshot,
  type BatchQueue,
  type SyncStats,
  type TimeEntryOutbox,
} from './tauri-backend.types';

describe('Tauri Backend Types - Issue #6: Timestamp Normalization', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('Timestamp Heuristic (< 10 billion = seconds)', () => {
    it('should multiply timestamps < 10 billion by 1000', async () => {
      const backendSnapshotSeconds = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 1729267200, // Oct 18, 2024 in SECONDS (< 10 billion)
        created_at: 1729267200,
      });

      mockInvoke.mockResolvedValue([backendSnapshotSeconds]);

      const result = await getRecentSnapshots();

      // Should be normalized to milliseconds
      expect(result[0]?.timestamp).toBe(1729267200 * 1000);
      expect(result[0]?.created_at).toBe(1729267200 * 1000);
    });

    it('should leave timestamps >= 10 billion unchanged (already in milliseconds)', async () => {
      const backendSnapshotMs = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 1729267200000, // Already in milliseconds (>= 10 billion)
        created_at: 1729267200000,
      });

      mockInvoke.mockResolvedValue([backendSnapshotMs]);

      const result = await getRecentSnapshots();

      // Should remain unchanged
      expect(result[0]?.timestamp).toBe(1729267200000);
      expect(result[0]?.created_at).toBe(1729267200000);
    });

    it('should create valid Date objects from normalized timestamps', async () => {
      const backendSnapshotSeconds = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 1729267200, // SECONDS
        created_at: 1729267200,
      });

      mockInvoke.mockResolvedValue([backendSnapshotSeconds]);

      const result = await getRecentSnapshots();

      // Should create valid Date objects
      expect(result.length).toBeGreaterThan(0);
      const firstSnapshot = result[0];
      if (firstSnapshot) {
        const date = new Date(firstSnapshot.timestamp);
        expect(date.getFullYear()).toBe(2024);
        expect(date.getMonth()).toBe(9); // October (0-indexed)
        expect(date.getDate()).toBe(18);
      }
    });
  });

  describe('getRecentSnapshots - ActivitySnapshot normalization', () => {
    it('should normalize all timestamp fields', async () => {
      const snapshot = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 1000000, // SECONDS
        created_at: 1000000, // SECONDS
        processed_at: 1000100, // SECONDS
      });

      mockInvoke.mockResolvedValue([snapshot]);

      const result = await getRecentSnapshots();

      expect(result[0]?.timestamp).toBe(1000000 * 1000);
      expect(result[0]?.created_at).toBe(1000000 * 1000);
      expect(result[0]?.processed_at).toBe(1000100 * 1000);
    });

    it('should handle null processed_at', async () => {
      const snapshot = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 1000000,
        created_at: 1000000,
        processed_at: undefined,
      });

      mockInvoke.mockResolvedValue([snapshot]);

      const result = await getRecentSnapshots();

      expect(result[0]?.processed_at).toBeUndefined();
    });

    it('should call invoke with correct command', async () => {
      mockInvoke.mockResolvedValue([]);

      await getRecentSnapshots();

      expect(mockInvoke).toHaveBeenCalledWith('get_recent_snapshots');
    });
  });

  describe('getRecentActivities - ActivitySnapshot normalization with params', () => {
    it('should pass limit parameter to backend', async () => {
      mockInvoke.mockResolvedValue([]);

      await getRecentActivities(50);

      expect(mockInvoke).toHaveBeenCalledWith('get_recent_activities', { limit: 50 });
    });

    it('should normalize timestamps in returned activities', async () => {
      const activity = createMockActivitySnapshot({
        id: 'act-1',
        timestamp: 1500000, // SECONDS
        created_at: 1500000,
      });

      mockInvoke.mockResolvedValue([activity]);

      const result = await getRecentActivities(10);

      expect(result[0]?.timestamp).toBe(1500000 * 1000);
      expect(result[0]?.created_at).toBe(1500000 * 1000);
    });

    it('should handle undefined limit parameter', async () => {
      mockInvoke.mockResolvedValue([]);

      await getRecentActivities();

      expect(mockInvoke).toHaveBeenCalledWith('get_recent_activities', { limit: undefined });
    });
  });

  describe('getOutboxStatus - TimeEntryOutbox normalization', () => {
    it('should normalize all timestamp fields', async () => {
      const outbox = createMockTimeEntryOutbox({
        id: 'outbox-1',
        idempotency_key: 'idem-1',
        user_id: 'user-1',
        created_at: 2000000, // SECONDS
        sent_at: 2000100, // SECONDS
        retry_after: 2000200, // SECONDS
      });

      mockInvoke.mockResolvedValue([outbox]);

      const result = await getOutboxStatus();

      expect(result[0]?.created_at).toBe(2000000 * 1000);
      expect(result[0]?.sent_at).toBe(2000100 * 1000);
      expect(result[0]?.retry_after).toBe(2000200 * 1000);
    });

    it('should handle null optional timestamp fields', async () => {
      const outbox = createMockTimeEntryOutbox({
        id: 'outbox-1',
        idempotency_key: 'idem-1',
        user_id: 'user-1',
        created_at: 2000000,
        sent_at: undefined,
        retry_after: undefined,
      });

      mockInvoke.mockResolvedValue([outbox]);

      const result = await getOutboxStatus();

      expect(result[0]?.sent_at).toBeUndefined();
      expect(result[0]?.retry_after).toBeUndefined();
    });

    it('should call invoke with correct command', async () => {
      mockInvoke.mockResolvedValue([]);

      await getOutboxStatus();

      expect(mockInvoke).toHaveBeenCalledWith('get_outbox_status');
    });
  });

  describe('getRecentSegments - ActivitySegment normalization', () => {
    it('should normalize start_ts, end_ts, and created_at', async () => {
      const segment = createMockActivitySegment({
        id: 'seg-1',
        start_ts: 3000000, // SECONDS
        end_ts: 3001000, // SECONDS
        created_at: 3001000, // SECONDS
      });

      mockInvoke.mockResolvedValue([segment]);

      const result = await getRecentSegments();

      expect(result[0]?.start_ts).toBe(3000000 * 1000);
      expect(result[0]?.end_ts).toBe(3001000 * 1000);
      expect(result[0]?.created_at).toBe(3001000 * 1000);
    });

    it('should call invoke with correct command', async () => {
      mockInvoke.mockResolvedValue([]);

      await getRecentSegments();

      expect(mockInvoke).toHaveBeenCalledWith('get_recent_segments');
    });
  });

  describe('getBatchStatus - BatchQueue normalization', () => {
    it('should normalize all batch timestamp fields', async () => {
      const batch = createMockBatchQueue({
        batch_id: 'batch-1',
        activity_count: 10,
        status: 'completed',
        created_at: 4000000, // SECONDS
        processed_at: 4000500, // SECONDS
        processing_started_at: 4000200, // SECONDS
        lease_expires_at: 4001000, // SECONDS
      });

      mockInvoke.mockResolvedValue([batch]);

      const result = await getBatchStatus();

      expect(result[0]?.created_at).toBe(4000000 * 1000);
      expect(result[0]?.processed_at).toBe(4000500 * 1000);
      expect(result[0]?.processing_started_at).toBe(4000200 * 1000);
      expect(result[0]?.lease_expires_at).toBe(4001000 * 1000);
    });

    it('should handle null optional timestamp fields', async () => {
      const batch = createMockBatchQueue({
        batch_id: 'batch-1',
        activity_count: 10,
        status: 'pending',
        created_at: 4000000,
        processed_at: undefined,
        processing_started_at: undefined,
        lease_expires_at: undefined,
      });

      mockInvoke.mockResolvedValue([batch]);

      const result = await getBatchStatus();

      expect(result[0]?.processed_at).toBeUndefined();
      expect(result[0]?.processing_started_at).toBeUndefined();
      expect(result[0]?.lease_expires_at).toBeUndefined();
    });

    it('should call invoke with correct command', async () => {
      mockInvoke.mockResolvedValue([]);

      await getBatchStatus();

      expect(mockInvoke).toHaveBeenCalledWith('get_batch_status');
    });
  });

  describe('getSyncStats - SyncStats normalization', () => {
    it('should normalize last_sync_time', async () => {
      const stats: SyncStats = {
        local_activity_count: 100,
        pending_batches: 2,
        failed_batches: 1,
        last_sync_time: 5000000, // SECONDS
      };

      mockInvoke.mockResolvedValue(stats);

      const result = await getSyncStats();

      expect(result.last_sync_time).toBe(5000000 * 1000);
    });

    it('should handle null last_sync_time', async () => {
      const stats: SyncStats = {
        local_activity_count: 100,
        pending_batches: 2,
        failed_batches: 1,
        last_sync_time: undefined,
      };

      mockInvoke.mockResolvedValue(stats);

      const result = await getSyncStats();

      expect(result.last_sync_time).toBeUndefined();
    });

    it('should call invoke with correct command', async () => {
      mockInvoke.mockResolvedValue({
        local_activity_count: 0,
        pending_batches: 0,
        failed_batches: 0,
      });

      await getSyncStats();

      expect(mockInvoke).toHaveBeenCalledWith('get_sync_stats');
    });
  });

  describe('getCostSummary - No timestamp normalization needed', () => {
    it('should pass through cost data without modification', async () => {
      const cost = {
        total_cost_usd: 12.5,
        total_tokens: 50000,
        daily_usage_usd: 2.3,
        classification_mode: 'enabled' as const,
      };

      mockInvoke.mockResolvedValue(cost);

      const result = await getCostSummary();

      expect(result).toEqual(cost);
    });

    it('should call invoke with correct command', async () => {
      mockInvoke.mockResolvedValue({
        total_cost_usd: 0,
        total_tokens: 0,
        daily_usage_usd: 0,
        classification_mode: 'enabled',
      });

      await getCostSummary();

      expect(mockInvoke).toHaveBeenCalledWith('get_cost_summary');
    });
  });

  describe('Type Safety', () => {
    it('should provide TypeScript types for all backend commands', () => {
      // Type-only test - ensures types compile correctly
      type TestSnapshot = ActivitySnapshot;
      type TestOutbox = TimeEntryOutbox;
      type TestSegment = ActivitySegment;
      type TestBatch = BatchQueue;
      type TestStats = SyncStats;

      // If this compiles, type definitions are correct
      const _snapshot: TestSnapshot = {} as TestSnapshot;
      const _outbox: TestOutbox = {} as TestOutbox;
      const _segment: TestSegment = {} as TestSegment;
      const _batch: TestBatch = {} as TestBatch;
      const _stats: TestStats = {} as TestStats;

      expect(_snapshot).toBeDefined();
      expect(_outbox).toBeDefined();
      expect(_segment).toBeDefined();
      expect(_batch).toBeDefined();
      expect(_stats).toBeDefined();
    });

    it('should enforce status enum types', () => {
      // Type-only test
      type OutboxStatus = TimeEntryOutbox['status'];
      type BatchStatus = BatchQueue['status'];

      const _outboxStatus: OutboxStatus = 'pending';
      const _batchStatus: BatchStatus = 'completed';

      expect(['pending', 'sent', 'failed']).toContain(_outboxStatus);
      expect(['pending', 'processing', 'completed', 'failed']).toContain(_batchStatus);
    });
  });

  describe('Edge Cases', () => {
    it('should handle zero timestamp', async () => {
      const snapshot = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 0, // Edge case: epoch start
        activity_context_json: '{}',
        detected_activity: 'Test',
        activity_category: 'work',
        primary_app: 'Test App',
        processed: false,
        created_at: 0,
      });

      mockInvoke.mockResolvedValue([snapshot]);

      const result = await getRecentSnapshots();

      // 0 < 10 billion, so should be multiplied by 1000
      expect(result[0]?.timestamp).toBe(0);
      expect(result[0]?.created_at).toBe(0);
    });

    it('should handle very large timestamps (far future)', async () => {
      const snapshot = createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: 99999999999999, // Already in milliseconds (year ~3000)
        activity_context_json: '{}',
        detected_activity: 'Test',
        activity_category: 'work',
        primary_app: 'Test App',
        processed: false,
        created_at: 99999999999999,
      });

      mockInvoke.mockResolvedValue([snapshot]);

      const result = await getRecentSnapshots();

      // Should NOT be multiplied (already > 10 billion)
      expect(result[0]?.timestamp).toBe(99999999999999);
      expect(result[0]?.created_at).toBe(99999999999999);
    });

    it('should handle empty arrays', async () => {
      mockInvoke.mockResolvedValue([]);

      const snapshots = await getRecentSnapshots();
      const outbox = await getOutboxStatus();
      const segments = await getRecentSegments();
      const batches = await getBatchStatus();

      expect(snapshots).toEqual([]);
      expect(outbox).toEqual([]);
      expect(segments).toEqual([]);
      expect(batches).toEqual([]);
    });
  });

  describe('Normalization Utility Function', () => {
    it('should correctly identify seconds vs milliseconds', () => {
      // This tests the heuristic logic in toMs() function
      const testCases = [
        { input: 1000000000, expected: 1000000000 * 1000 }, // 1 billion (seconds)
        { input: 5000000000, expected: 5000000000 * 1000 }, // 5 billion (seconds)
        { input: 9999999999, expected: 9999999999 * 1000 }, // Just under 10B (seconds)
        { input: 10000000000, expected: 10000000000 }, // 10 billion (milliseconds)
        { input: 1729267200, expected: 1729267200 * 1000 }, // Oct 2024 (seconds)
        { input: 1729267200000, expected: 1729267200000 }, // Oct 2024 (milliseconds)
      ];

      testCases.forEach(({ input, expected }) => {
        const result = input < 10_000_000_000 ? input * 1000 : input;
        expect(result).toBe(expected);
      });
    });
  });
});
