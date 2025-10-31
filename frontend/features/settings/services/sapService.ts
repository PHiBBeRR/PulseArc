// FEATURE-020 Phase 2: SAP Service Layer
// Service wrapper for SAP Tauri commands

import type { OutboxStatusSummary, SapSyncSettings, WbsElement } from '@/shared/types/generated';
import { invoke } from '@tauri-apps/api/core';

export type SapAuthStatus = {
  isAuthenticated: boolean;
  lastChecked: number;
};

export type OutboxStatus = {
  pending: number;
  sent: number;
  failed: number;
};

// FEATURE-020 Phase 4.4: Error handling types
export type ConnectionHealthStatus = {
  healthy: boolean;
  latency_ms: number | null;
  last_error: string | null;
};

export type RetrySyncResult = {
  success: boolean;
  elements_synced: number;
  error: string | null;
};

export type ValidationResponse = {
  status: 'Valid' | 'Warning' | 'Error';
  code: string;
  message: string | null;
};

/**
 * SAP Service Layer
 *
 * Provides a clean TypeScript interface to SAP integration features:
 * - Auth: Login/logout with Auth0 OAuth
 * - WBS Search: FTS5 full-text search in local cache
 * - Outbox: Status monitoring and retry failed entries
 */
export class SapService {
  /**
   * Start SAP OAuth login flow
   *
   * Opens browser for Auth0 authentication
   * @returns Authorization URL
   */
  static async startLogin(): Promise<string> {
    return invoke<string>('sap_start_login');
  }

  /**
   * Complete SAP OAuth login
   *
   * Validates and stores tokens after callback
   * @param code - Authorization code from callback
   * @param state - State parameter from callback
   */
  static async completeLogin(code: string, state: string): Promise<void> {
    return invoke<void>('sap_complete_login', { code, state });
  }

  /**
   * Check if user is authenticated with SAP
   *
   * @returns Authentication status
   */
  static async isAuthenticated(): Promise<boolean> {
    return invoke<boolean>('sap_is_authenticated');
  }

  /**
   * Logout from SAP
   *
   * Clears tokens from keychain
   */
  static async logout(): Promise<void> {
    return invoke<void>('sap_logout');
  }

  /**
   * Get authentication status with timestamp
   *
   * Useful for UI status indicators
   */
  static async getAuthStatus(): Promise<SapAuthStatus> {
    const isAuthenticated = await this.isAuthenticated();
    return {
      isAuthenticated,
      lastChecked: Date.now(),
    };
  }

  /**
   * Search WBS codes in local cache
   *
   * Uses FTS5 full-text search with BM25 ranking
   * @param query - Search term (matches code, project name, description)
   * @returns Matching WBS elements (max 20)
   */
  static async searchWbs(query: string): Promise<WbsElement[]> {
    if (!query || query.trim().length === 0) {
      return [];
    }
    return invoke<WbsElement[]>('sap_search_wbs', { query: query.trim() });
  }

  /**
   * Get outbox status summary
   *
   * Returns counts of pending/sent/failed time entries
   * @returns Outbox status with counts
   */
  static async getOutboxStatus(): Promise<OutboxStatus> {
    const summary = await invoke<OutboxStatusSummary>('sap_get_outbox_status');
    return {
      pending: summary.pending_count,
      sent: summary.sent_count,
      failed: summary.failed_count,
    };
  }

  /**
   * Retry all failed outbox entries
   *
   * Resets status from 'failed' to 'pending' for forwarder to retry
   * @returns Number of entries reset
   */
  static async retryFailedEntries(): Promise<number> {
    return invoke<number>('sap_retry_failed_entries');
  }

  /**
   * Start the outbox forwarder
   *
   * Begins background sync to server
   */
  static async startForwarder(): Promise<void> {
    return invoke<void>('sap_start_forwarder');
  }

  /**
   * Stop the outbox forwarder
   *
   * Stops background sync (useful for debugging)
   */
  static async stopForwarder(): Promise<void> {
    return invoke<void>('sap_stop_forwarder');
  }

  /**
   * Format WBS element for display
   *
   * @param element - WBS element from search
   * @returns Formatted display string
   */
  static formatWbsDisplay(element: WbsElement): string {
    const parts = [element.wbs_code];
    if (element.project_name) {
      parts.push(element.project_name);
    }
    if (element.description) {
      parts.push(`(${element.description})`);
    }
    return parts.join(' - ');
  }

  /**
   * Validate WBS code format
   *
   * Format: [ProjectCode].[Platform].[Team]
   * Example: USC0063201.1.1
   *
   * Project Codes: USC0063200-USC0063211 (12 projects)
   * Platform: 1=Deals, 2=Compliance, 3=Advisory
   * Team: 1=M&A Tax, 2=SALT, 3=International, 4=Partnerships
   *
   * @param code - WBS code to validate
   * @returns True if format is valid
   */
  static validateWbsCode(code: string): boolean {
    // Format: [ProjectCode].[Platform].[Team]
    // Example: USC0063201.1.1, USC0063202.2.3
    return /^[A-Z]{3}\d{7}\.[1-3]\.[1-4]$/.test(code);
  }

  // =========================================================================
  // FEATURE-020 Phase 4.4: Error Handling & Health Check Methods
  // =========================================================================

  /**
   * Check SAP connector connection health
   *
   * Performs lightweight HEAD request to /health endpoint with 5s timeout
   * @returns Health status with optional latency and error message
   */
  static async checkConnectionHealth(): Promise<ConnectionHealthStatus> {
    return invoke<ConnectionHealthStatus>('sap_check_connection_health');
  }

  /**
   * Retry WBS sync now (bypass backoff)
   *
   * Triggers immediate sync regardless of scheduler's backoff state
   * Useful for manual retry after failed syncs
   * @returns Sync result with success status, element count, and optional error
   */
  static async retrySyncNow(): Promise<RetrySyncResult> {
    return invoke<RetrySyncResult>('sap_retry_sync_now');
  }

  /**
   * Validate a WBS code
   *
   * Checks format, existence in cache, and status (REL/TECO/CLSD)
   * @param code - WBS code to validate
   * @returns Validation response with status and optional message
   */
  static async validateWbs(code: string): Promise<ValidationResponse> {
    const response = await invoke<{
      Valid?: null;
      Warning?: { message: string };
      Error?: { message: string };
    }>('sap_validate_wbs', { code });

    // Convert Rust enum to TypeScript-friendly format
    if ('Valid' in response) {
      return { status: 'Valid', code, message: null };
    } else if ('Warning' in response && response.Warning) {
      return { status: 'Warning', code, message: response.Warning.message };
    } else if ('Error' in response && response.Error) {
      return { status: 'Error', code, message: response.Error.message };
    }

    // Fallback (should never happen)
    return { status: 'Error', code, message: 'Unknown validation error' };
  }

  // =========================================================================
  // FEATURE-020 Phase 3: Sync Scheduler Methods
  // =========================================================================

  /**
   * Get SAP sync settings
   *
   * @returns Current sync configuration (enabled, interval, last sync)
   */
  static async getSyncSettings(): Promise<SapSyncSettings> {
    return invoke<SapSyncSettings>('sap_get_sync_settings');
  }

  /**
   * Update SAP sync settings
   *
   * @param enabled - Enable/disable background sync
   * @param syncIntervalHours - Hours between syncs (1-24)
   */
  static async updateSyncSettings(enabled: boolean, syncIntervalHours: number): Promise<void> {
    return invoke<void>('sap_update_sync_settings', {
      enabled,
      syncIntervalHours,
    });
  }

  /**
   * Start SAP sync scheduler
   *
   * @param intervalHours - Hours between syncs
   */
  static async startScheduler(intervalHours: number): Promise<void> {
    return invoke<void>('sap_start_scheduler', { intervalHours });
  }

  /**
   * Stop SAP sync scheduler
   */
  static async stopScheduler(): Promise<void> {
    return invoke<void>('sap_stop_scheduler');
  }

  /**
   * Trigger manual WBS sync now
   *
   * Performs immediate sync bypassing scheduler interval
   */
  static async triggerSyncNow(): Promise<void> {
    return invoke<void>('sap_trigger_sync_now');
  }

  /**
   * Sync WBS codes from Neon database
   *
   * Fetches enriched WBS data from Neon and populates local cache
   * @returns Number of WBS codes synced
   */
  static async syncFromNeon(): Promise<number> {
    return invoke<number>('neon_sync_wbs_cache');
  }

  /**
   * Clear WBS cache
   *
   * Removes all cached WBS elements
   * @returns Number of entries deleted
   */
  static async clearCache(): Promise<number> {
    return invoke<number>('sap_clear_cache');
  }
}

export const sapService = new SapService();
