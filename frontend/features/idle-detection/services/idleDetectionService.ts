/**
 * Idle Detection Service
 *
 * FEATURE-008 Phase 0: System-level idle detection with backend integration
 *
 * CURRENT IMPLEMENTATION (Phase 0):
 * - Uses backend CGEventSource API for system-level idle time (macOS only)
 * - Frontend polling every 30 seconds to check idle status
 * - Graceful fallback to frontend-only detection if backend unavailable
 * - Activity tracking for UI responsiveness and fallback
 *
 * KNOWN LIMITATIONS (Phase 0):
 * - No sleep/wake detection: 2-hour laptop sleep = "2 hours idle"
 * - No lock screen awareness: locked screen not treated differently
 * - Backend only works on macOS (graceful fallback on other platforms)
 *
 * PRODUCTION-READY SOLUTION (Phase 1-2):
 * See FEATURE-008 implementation plan for:
 * - IOKit sleep/wake detection (kIOMessageSystemWillSleep/HasPoweredOn)
 * - Lock screen detection (CFNotificationCenter)
 * - System-wide activity tracking (CGEvent global monitoring)
 * - Circuit breaker for API failures
 * - 95%+ idle detection accuracy with <5% false positives
 *
 * @see tickets/active/FEATURE-008-idle-detection-enhancement.md
 */

import type { IdleDetectionConfig } from '../types';

export const idleDetectionService = {
  /**
   * Calculate idle duration in minutes
   */
  calculateIdleMinutes: (lastActivityTime: number): number => {
    const now = Date.now();
    const idleTime = now - lastActivityTime;
    return Math.floor(idleTime / 60000); // Convert to minutes
  },

  /**
   * Check if idle threshold is exceeded
   */
  isIdleThresholdExceeded: (lastActivityTime: number, thresholdMinutes: number = 5): boolean => {
    const idleMinutes = idleDetectionService.calculateIdleMinutes(lastActivityTime);
    return idleMinutes >= thresholdMinutes;
  },

  /**
   * Format idle duration for display
   */
  formatIdleDuration: (minutes: number): string => {
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;

    if (hours > 0 && mins > 0) {
      return `${hours}h ${mins}m`;
    } else if (hours > 0) {
      return `${hours}h`;
    } else {
      return `${mins}m`;
    }
  },

  /**
   * Get default idle detection configuration
   */
  getDefaultConfig: (): IdleDetectionConfig => ({
    idleThresholdMinutes: 5,
    checkIntervalSeconds: 30,
    enableIdleDetection: true,
  }),

  /**
   * Calculate time to discard (in seconds)
   */
  calculateTimeToDiscard: (idleMinutes: number): number => {
    return idleMinutes * 60; // Convert to seconds
  },

  /**
   * Get idle detection message
   */
  getIdleMessage: (idleMinutes: number): string => {
    const duration = idleDetectionService.formatIdleDuration(idleMinutes);
    return `You've been idle for ${duration}. Would you like to keep or discard this time?`;
  },

  /**
   * Determine idle severity (for UI coloring)
   */
  getIdleSeverity: (idleMinutes: number): 'low' | 'medium' | 'high' => {
    if (idleMinutes >= 30) return 'high';
    if (idleMinutes >= 15) return 'medium';
    return 'low';
  },
};
