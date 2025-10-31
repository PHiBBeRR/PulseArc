// FEATURE-015/017: Calendar Integration - Frontend Service
// Service for calendar OAuth and sync commands (Google + Microsoft)

import type { CalendarConnectionStatus } from '@/shared/types/generated/CalendarConnectionStatus';
import type { CalendarSyncSettings } from '@/shared/types/generated/CalendarSyncSettings';
import { invoke } from '@tauri-apps/api/core';
import { openUrl } from '@tauri-apps/plugin-opener';

/**
 * Calendar provider type
 * FEATURE-017: Supports Google and Microsoft calendar providers
 */
export type CalendarProvider = 'google' | 'microsoft';

/**
 * Calendar service for multi-provider calendar integration
 * Provides OAuth, sync, and settings management for Google and Microsoft calendars
 */
export const calendarService = {
  /**
   * Initiate Calendar OAuth flow for specified provider
   * Opens system browser with provider-specific consent screen
   * FEATURE-017: Multi-provider support (google, microsoft)
   */
  connect: async (provider: CalendarProvider): Promise<void> => {
    try {
      console.warn(`[calendarService] Starting ${provider} OAuth flow...`);
      const authUrl = await invoke<string>('initiate_calendar_auth', { provider });
      console.warn('[calendarService] Got auth URL:', authUrl);
      await openUrl(authUrl);
      console.warn('[calendarService] Browser opened successfully');
    } catch (error) {
      console.error(`[calendarService] Failed to connect ${provider}:`, error);
      throw error;
    }
  },

  /**
   * Disconnect and revoke calendar access for specified provider
   * Clears tokens from keychain and database
   * FEATURE-017: Provider-specific disconnect
   */
  disconnect: async (provider: CalendarProvider): Promise<void> => {
    await invoke('disconnect_calendar', { provider });
  },

  /**
   * Get current connection status
   * Returns connection state, email, and last sync time
   * FEATURE-017: Now returns array to support multiple providers
   */
  getStatus: async (): Promise<CalendarConnectionStatus[]> => {
    return await invoke('get_calendar_connection_status');
  },

  /**
   * Manually trigger calendar sync for ALL providers
   * Fetches events and generates suggestions from all connected calendars
   * Automatically emits 'outbox-updated' event when complete
   * @param force - Force sync even if recently synced
   */
  syncNow: async (force = true): Promise<number> => {
    return await invoke('sync_calendar_events', { force });
  },

  /**
   * Manually trigger calendar sync for a specific provider
   * Fetches events and generates suggestions for the specified calendar
   * Automatically emits 'outbox-updated' event when complete
   * FEATURE-017: Provider-specific sync
   * @param provider - Provider to sync (google, microsoft)
   */
  syncProvider: async (provider: CalendarProvider): Promise<number> => {
    return await invoke('sync_calendar_provider', { provider });
  },

  /**
   * Get calendar sync settings for a user
   */
  getSettings: async (userEmail: string): Promise<CalendarSyncSettings> => {
    return await invoke('get_calendar_sync_settings', { user_email: userEmail });
  },

  /**
   * Update calendar sync settings
   */
  updateSettings: async (userEmail: string, settings: CalendarSyncSettings): Promise<void> => {
    await invoke('update_calendar_sync_settings', {
      user_email: userEmail,
      settings,
    });
  },
};
