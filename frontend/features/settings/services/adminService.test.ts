/**
 * Unit tests for adminService
 *
 * Tests the service that provides administrative functions for managing
 * local data, including clearing snapshots, outbox, and all data.
 *
 * Test Coverage:
 * - Clear Snapshots: Removing all activity snapshots from local database
 * - Clear Outbox: Removing all pending time entries from outbox
 * - Clear All Data: Comprehensive cleanup of all local data
 * - Error Handling: Propagating database errors
 * - Command Invocation: Correct Tauri command calls
 * - Sequential Operations: Proper ordering when clearing multiple datasets
 */

import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { adminService } from './adminService';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('adminService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('clearSnapshots', () => {
    it('should call clear_snapshots command', async () => {
      await adminService.clearSnapshots();
      expect(invoke).toHaveBeenCalledWith('clear_snapshots');
    });

    it('should propagate errors', async () => {
      const error = new Error('Database error');
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(adminService.clearSnapshots()).rejects.toThrow('Database error');
    });
  });

  describe('clearOutbox', () => {
    it('should call clear_suggestions command', async () => {
      await adminService.clearOutbox();
      expect(invoke).toHaveBeenCalledWith('clear_suggestions');
    });

    it('should propagate errors', async () => {
      const error = new Error('Database error');
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(adminService.clearOutbox()).rejects.toThrow('Database error');
    });
  });

  describe('clearAllData', () => {
    it('should call both clear commands', async () => {
      await adminService.clearAllData();

      expect(invoke).toHaveBeenCalledTimes(2);
      expect(invoke).toHaveBeenNthCalledWith(1, 'clear_snapshots');
      expect(invoke).toHaveBeenNthCalledWith(2, 'clear_suggestions');
    });

    it('should propagate errors from snapshots', async () => {
      const error = new Error('Snapshots error');
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(adminService.clearAllData()).rejects.toThrow('Snapshots error');
    });

    it('should propagate errors from outbox', async () => {
      const error = new Error('Outbox error');
      vi.mocked(invoke).mockResolvedValueOnce(undefined);
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(adminService.clearAllData()).rejects.toThrow('Outbox error');
    });
  });
});
