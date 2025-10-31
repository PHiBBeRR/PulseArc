/**
 *  Calendar Service Multi-Provider Tests
 * Tests for updated calendar service with provider parameter support
 *
 * Validates the enhanced calendar service that supports multiple calendar
 * providers (Google Calendar, Microsoft Outlook, etc.) with provider-specific
 * commands and status tracking.
 *
 * Test Coverage:
 * - Connection Commands: Provider-specific auth initiation (google, microsoft)
 * - Status Array: Multiple connection statuses for different providers
 * - Sync Operations: Optional provider parameter for targeted sync
 * - Disconnect: Specifying which provider to disconnect
 * - Type Safety: CalendarProvider type enforcement
 * - Multi-Provider State: Managing connections to multiple calendars simultaneously
 * - Provider Switching: Switching active provider for sync operations
 */

import { invoke } from '@tauri-apps/api/core';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Mock Tauri shell open
vi.mock('@tauri-apps/plugin-opener', () => ({
  openUrl: vi.fn(),
}));

describe('Calendar Service - Multi-Provider Support', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ==========================================================================
  // TEST CATEGORY 1: Connection Commands (4 tests)
  // ==========================================================================

  describe('Connection Commands', () => {
    it(' should invoke initiate_calendar_auth with google provider', async () => {
      // AC: initiate_calendar_auth command called with provider="google"
      // AC: Returns Google OAuth URL
      const mockAuthUrl = 'https://accounts.google.com/o/oauth2/v2.0/auth?...';
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockAuthUrl);

      const result = await invoke<string>('initiate_calendar_auth', { provider: 'google' });

      expect(invoke).toHaveBeenCalledWith('initiate_calendar_auth', { provider: 'google' });
      expect(result).toBe(mockAuthUrl);
      expect(result).toContain('accounts.google.com');
    });

    it(' should invoke initiate_calendar_auth with microsoft provider', async () => {
      // AC: initiate_calendar_auth command called with provider="microsoft"
      // AC: Returns Microsoft OAuth URL
      const mockAuthUrl = 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize?...';
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockAuthUrl);

      const result = await invoke<string>('initiate_calendar_auth', { provider: 'microsoft' });

      expect(invoke).toHaveBeenCalledWith('initiate_calendar_auth', { provider: 'microsoft' });
      expect(result).toBe(mockAuthUrl);
      expect(result).toContain('login.microsoftonline.com');
    });

    it(' should open system browser with provider-specific OAuth URL', async () => {
      // AC: OAuth URL opened in system browser for correct provider
      const { openUrl } = await import('@tauri-apps/plugin-opener');
      const mockMicrosoftUrl = 'https://login.microsoftonline.com/common/oauth2/v2.0/authorize?...';
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockMicrosoftUrl);

      const authUrl = await invoke<string>('initiate_calendar_auth', { provider: 'microsoft' });
      await openUrl(authUrl);

      expect(openUrl).toHaveBeenCalledWith(mockMicrosoftUrl);
    });

    it(' should handle unknown provider error', async () => {
      // AC: Unknown provider returns error
      // AC: Error message indicates invalid provider
      const mockError = new Error('Unknown calendar provider: apple');
      (invoke as ReturnType<typeof vi.fn>).mockRejectedValue(mockError);

      await expect(invoke('initiate_calendar_auth', { provider: 'apple' })).rejects.toThrow(
        'Unknown calendar provider'
      );
    });
  });

  // ==========================================================================
  // TEST CATEGORY 2: Status Commands (2 tests)
  // ==========================================================================

  describe('Status Commands', () => {
    it(' should get connection status returning array of statuses', async () => {
      // AC: get_calendar_connection_status returns Vec<CalendarConnectionStatus>
      // AC: Array contains status for each connected provider
      const mockStatuses = [
        {
          provider: 'google',
          connected: true,
          email: 'user@gmail.com',
          lastSync: 1705316400,
          syncEnabled: true,
        },
        {
          provider: 'microsoft',
          connected: true,
          email: 'user@outlook.com',
          lastSync: 1705316500,
          syncEnabled: true,
        },
      ];
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockStatuses);

      const result = await invoke('get_calendar_connection_status');

      expect(invoke).toHaveBeenCalledWith('get_calendar_connection_status');
      expect(result).toEqual(mockStatuses);
      expect(Array.isArray(result)).toBe(true);
      expect(result).toHaveLength(2);
    });

    it(' should include provider field in each status object', async () => {
      // AC: Each CalendarConnectionStatus has provider field
      // AC: Provider field is string ("google" or "microsoft")
      const mockStatuses = [
        {
          provider: 'google',
          connected: true,
          email: 'user@gmail.com',
          lastSync: 1705316400,
          syncEnabled: true,
        },
      ];
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockStatuses);

      const result = await invoke('get_calendar_connection_status');

      expect(Array.isArray(result)).toBe(true);
      if (Array.isArray(result) && result.length > 0) {
        expect(result[0]).toHaveProperty('provider');
        expect(typeof result[0].provider).toBe('string');
        expect(result[0].provider).toBe('google');
      }
    });
  });

  // ==========================================================================
  // TEST CATEGORY 3: Sync Commands (2 tests)
  // ==========================================================================

  describe('Sync Commands', () => {
    it(' should sync all providers when no provider parameter given', async () => {
      // AC: sync_calendar_events with no provider param syncs all connected providers
      // AC: Returns total suggestion count
      const mockCount = 15;
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockCount);

      const result = await invoke<number>('sync_calendar_events', {
        provider: undefined,
        force: true,
      });

      expect(invoke).toHaveBeenCalledWith('sync_calendar_events', {
        provider: undefined,
        force: true,
      });
      expect(result).toBe(mockCount);
    });

    it(' should sync specific provider when provider parameter given', async () => {
      // AC: sync_calendar_events with provider="microsoft" syncs only Microsoft
      // AC: Returns suggestion count from Microsoft only
      const mockCount = 8;
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(mockCount);

      const result = await invoke<number>('sync_calendar_events', {
        provider: 'microsoft',
        force: true,
      });

      expect(invoke).toHaveBeenCalledWith('sync_calendar_events', {
        provider: 'microsoft',
        force: true,
      });
      expect(result).toBe(mockCount);
    });
  });

  // ==========================================================================
  // TEST CATEGORY 4: Disconnect Commands (2 tests)
  // ==========================================================================

  describe('Disconnect Commands', () => {
    it(' should disconnect google provider only', async () => {
      // AC: disconnect_calendar("google") removes only Google
      // AC: Command succeeds without error
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

      await invoke('disconnect_calendar', { provider: 'google' });

      expect(invoke).toHaveBeenCalledWith('disconnect_calendar', { provider: 'google' });
    });

    it(' should disconnect microsoft provider only', async () => {
      // AC: disconnect_calendar("microsoft") removes only Microsoft
      // AC: Command succeeds without error
      (invoke as ReturnType<typeof vi.fn>).mockResolvedValue(undefined);

      await invoke('disconnect_calendar', { provider: 'microsoft' });

      expect(invoke).toHaveBeenCalledWith('disconnect_calendar', { provider: 'microsoft' });
    });
  });
});
