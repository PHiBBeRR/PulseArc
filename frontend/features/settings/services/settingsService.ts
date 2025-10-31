// Settings business logic service

import type { Integration, MLSettings, NotificationSettings, SettingsState } from '../types';

// Available integrations (legacy - most moved to CalendarProviderCard)
// Note: Microsoft Teams is now part of the Microsoft 365 calendar integration
const AVAILABLE_INTEGRATIONS: Integration[] = [
  {
    id: 'sap-s4hana',
    name: 'SAP S/4HANA',
    icon: 'sap-s4hana',
    connected: false,
  },
];

export const settingsService = {
  /**
   * Get available integrations
   */
  getAvailableIntegrations: (): Integration[] => {
    return AVAILABLE_INTEGRATIONS;
  },

  /**
   * Get default settings
   */
  getDefaultSettings: (): SettingsState => {
    return {
      autoApply: true,
      notifications: true,
      confidence: 75,
      integrations: AVAILABLE_INTEGRATIONS.reduce(
        (acc, integration) => {
          acc[integration.id] = integration.connected;
          return acc;
        },
        {} as Record<string, boolean>
      ),
      timeFormat: '12h',
    };
  },

  /**
   * Load settings from localStorage
   */
  loadSettings: (): SettingsState => {
    try {
      const saved = localStorage.getItem('timer-settings');
      console.warn('🔍 [settingsService] loadSettings - raw localStorage:', saved);
      if (saved) {
        const parsed = JSON.parse(saved);
        const defaults = settingsService.getDefaultSettings();
        const merged = { ...defaults, ...parsed };
        console.warn('🔍 [settingsService] loadSettings - parsed:', parsed);
        console.warn('🔍 [settingsService] loadSettings - defaults:', defaults);
        console.warn('🔍 [settingsService] loadSettings - merged result:', merged);
        return merged;
      }
    } catch (error) {
      console.error('Failed to load settings:', error);
    }
    console.warn('🔍 [settingsService] loadSettings - no saved settings, returning defaults');
    return settingsService.getDefaultSettings();
  },

  /**
   * Save settings to localStorage
   */
  saveSettings: (settings: SettingsState): void => {
    try {
      const json = JSON.stringify(settings);
      console.warn('💾 [settingsService] saveSettings - saving to localStorage:', {
        settings,
        json,
      });
      localStorage.setItem('timer-settings', json);
      // Verify it was saved
      const verified = localStorage.getItem('timer-settings');
      console.warn('✅ [settingsService] saveSettings - verified saved value:', verified);
    } catch (error) {
      console.error('Failed to save settings:', error);
    }
  },

  /**
   * Validate confidence threshold
   */
  validateConfidence: (value: number): boolean => {
    return value >= 50 && value <= 100;
  },

  /**
   * Get confidence label
   */
  getConfidenceLabel: (confidence: number): string => {
    if (confidence >= 90) return 'Very High';
    if (confidence >= 75) return 'High';
    if (confidence >= 60) return 'Medium';
    return 'Low';
  },

  /**
   * Get default ML settings
   */
  getDefaultMLSettings: (): MLSettings => {
    return {
      autoApply: true,
      confidenceThreshold: 75,
      enableSuggestions: true,
    };
  },

  /**
   * Get default notification settings
   */
  getDefaultNotificationSettings: (): NotificationSettings => {
    return {
      enabled: true,
      showBadges: true,
      soundEnabled: false,
    };
  },

  /**
   * Check if integration is connected
   */
  isIntegrationConnected: (id: string, integrations: Record<string, boolean>): boolean => {
    return integrations[id] === true;
  },

  /**
   * Get connected integrations count
   */
  getConnectedCount: (integrations: Record<string, boolean>): number => {
    return Object.values(integrations).filter(Boolean).length;
  },

  /**
   * Export settings as JSON
   */
  exportSettings: (settings: SettingsState): string => {
    return JSON.stringify(settings, null, 2);
  },

  /**
   * Import settings from JSON
   */
  importSettings: (json: string): SettingsState | null => {
    try {
      const parsed = JSON.parse(json);
      return { ...settingsService.getDefaultSettings(), ...parsed };
    } catch (error) {
      console.error('Failed to import settings:', error);
      return null;
    }
  },
};
