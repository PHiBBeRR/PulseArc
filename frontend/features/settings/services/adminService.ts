// Admin service for dangerous operations
// Provides data clearing and reset functionality

import { invoke } from '@tauri-apps/api/core';

/**
 * Admin service for dangerous operations
 * Use with caution - these operations are irreversible
 */
export const adminService = {
  /**
   * Clear all activity snapshots
   * WARNING: This is irreversible
   */
  clearSnapshots: async (): Promise<void> => {
    await invoke('clear_local_activities');
  },

  /**
   * Clear all outbox entries
   * WARNING: This is irreversible
   */
  clearOutbox: async (): Promise<void> => {
    await invoke('clear_outbox');
  },

  /**
   * Clear all local data (snapshots + outbox)
   * WARNING: This is irreversible
   */
  clearAllData: async (): Promise<void> => {
    await invoke('clear_local_activities');
    await invoke('clear_outbox');
  },
};
