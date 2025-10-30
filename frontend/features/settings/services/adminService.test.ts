import { describe, it, expect, vi, beforeEach } from 'vitest';
import { adminService } from './adminService';
import { invoke } from '@tauri-apps/api/core';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

describe('adminService', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  describe('clearSnapshots', () => {
    it('should call clear_local_activities command', async () => {
      await adminService.clearSnapshots();
      expect(invoke).toHaveBeenCalledWith('clear_local_activities');
    });

    it('should propagate errors', async () => {
      const error = new Error('Database error');
      vi.mocked(invoke).mockRejectedValueOnce(error);

      await expect(adminService.clearSnapshots()).rejects.toThrow('Database error');
    });
  });

  describe('clearOutbox', () => {
    it('should call clear_outbox command', async () => {
      await adminService.clearOutbox();
      expect(invoke).toHaveBeenCalledWith('clear_outbox');
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
      expect(invoke).toHaveBeenNthCalledWith(1, 'clear_local_activities');
      expect(invoke).toHaveBeenNthCalledWith(2, 'clear_outbox');
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
