/**
 * Web API Service - FEATURE-016 Phase 3
 *
 * Service layer for Main Pulsarc API authentication and sync operations.
 * Provides frontend interface to Rust backend OAuth and sync commands.
 */

import { invoke } from '@tauri-apps/api/core';
import type { OutboxStatusSummary } from '@/shared/types/generated';

export type WebApiAuthStatus = {
  authenticated: boolean;
  userEmail?: string;
  lastSync?: number;
  pendingCount?: number;
};

export type WebApiUserInfo = {
  email: string;
  name?: string;
};

export type OutboxStatus = {
  pending: number;
  sent: number;
  failed: number;
};

/**
 * Web API Service Layer
 *
 * Provides a clean TypeScript interface to Main Pulsarc API integration:
 * - Auth: Login/logout with Auth0 OAuth
 * - Outbox: Status monitoring for main_api target
 * - Scheduler: Control background sync
 */
export class WebApiService {
  /**
   * Start Web API OAuth login flow
   *
   * Opens browser for Auth0 authentication.
   * Uses port 8888 (different from SAP's 8889).
   * Backend handles OAuth callback automatically in background.
   *
   * @returns Authorization URL
   */
  static async startLogin(): Promise<string> {
    // Get auth URL from backend (callback server starts in background)
    const authUrl = await invoke<string>('webapi_start_login');

    // Open browser with auth URL
    const { openUrl } = await import('@tauri-apps/plugin-opener');
    await openUrl(authUrl);

    return authUrl;
  }

  /**
   * Complete Web API OAuth login
   *
   * NOTE: This is handled automatically by the backend's loopback HTTP server.
   * You typically don't need to call this from the frontend.
   * The backend receives the OAuth callback at http://localhost:8888/callback
   * and automatically exchanges the code for tokens.
   *
   * @param code - Authorization code from callback
   * @param state - State parameter from callback
   * @internal
   */
  static async completeLogin(code: string, state: string): Promise<void> {
    return invoke<void>('webapi_complete_login', { code, state });
  }

  /**
   * Check if user is authenticated with Main API
   *
   * @returns Authentication status
   */
  static async isAuthenticated(): Promise<boolean> {
    return invoke<boolean>('webapi_is_authenticated');
  }

  /**
   * Logout from Main API
   *
   * Clears tokens from keychain
   */
  static async logout(): Promise<void> {
    return invoke<void>('webapi_logout');
  }

  /**
   * Get current user info from ID token
   *
   * @returns User info if authenticated, null otherwise
   */
  static async getUserInfo(): Promise<WebApiUserInfo | null> {
    return invoke<WebApiUserInfo | null>('webapi_get_user_info');
  }

  /**
   * Get authentication status with user info
   *
   * Useful for UI status indicators
   */
  static async getAuthStatus(): Promise<WebApiAuthStatus> {
    const authenticated = await this.isAuthenticated();

    if (!authenticated) {
      return { authenticated: false };
    }

    const userInfo = await this.getUserInfo();

    return {
      authenticated: true,
      userEmail: userInfo?.email,
      lastSync: Date.now(),
    };
  }

  /**
   * Get outbox status for main_api target
   *
   * @returns Outbox counts (pending, sent, failed)
   */
  static async getOutboxStatus(): Promise<OutboxStatus> {
    const summary = await invoke<OutboxStatusSummary>('webapi_get_outbox_status');

    return {
      pending: summary.pending_count,
      sent: summary.sent_count,
      failed: summary.failed_count,
    };
  }

  /**
   * Start the Web API scheduler
   *
   * Starts background processing every 10 seconds
   */
  static async startScheduler(): Promise<void> {
    return invoke<void>('webapi_start_scheduler');
  }

  /**
   * Stop the Web API scheduler
   */
  static async stopScheduler(): Promise<void> {
    return invoke<void>('webapi_stop_scheduler');
  }

  /**
   * Check if scheduler is running
   */
  static async isSchedulerRunning(): Promise<boolean> {
    return invoke<boolean>('webapi_scheduler_is_running');
  }
}
