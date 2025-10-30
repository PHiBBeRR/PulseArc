// Centralized time formatting utility
// Formats times according to user's time format preference (12h or 24h)

import { settingsService } from '@/features/settings/services/settingsService';

// Cache the time format preference to avoid repeated localStorage reads
let cachedTimeFormat: '12h' | '24h' | null = null;

/**
 * Get the cached time format preference, loading from settings if not yet cached.
 * Call invalidateTimeFormatCache() when settings change.
 */
function getCachedTimeFormat(): '12h' | '24h' {
  if (cachedTimeFormat === null) {
    cachedTimeFormat = settingsService.loadSettings().timeFormat;
  }
  return cachedTimeFormat;
}

/**
 * Invalidate the cached time format preference.
 * Call this when settings are updated to force a reload on next access.
 */
export function invalidateTimeFormatCache(): void {
  cachedTimeFormat = null;
}

/**
 * Format a Date object to a time string (e.g., "4:30 PM" or "16:30")
 * Uses the user's time format preference from settings
 */
export function formatTime(date: Date, format?: '12h' | '24h'): string {
  // Get format from settings cache if not provided
  const timeFormat = format ?? getCachedTimeFormat();

  const hours = date.getHours();
  const minutes = date.getMinutes();

  if (timeFormat === '24h') {
    // 24-hour format (e.g., "16:30")
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`;
  } else {
    // 12-hour format with AM/PM (e.g., "4:30 PM")
    const period = hours >= 12 ? 'PM' : 'AM';
    const displayHours = hours > 12 ? hours - 12 : hours === 0 ? 12 : hours;
    return `${displayHours}:${minutes.toString().padStart(2, '0')} ${period}`;
  }
}

/**
 * Format a time string (HH:MM) to user's preferred format
 * Input is expected to be in 24-hour format (e.g., "16:30")
 */
export function formatTimeString(timeStr: string, format?: '12h' | '24h'): string {
  const parts = timeStr.split(':').map(Number);
  const hours = parts[0] ?? 0;
  const minutes = parts[1] ?? 0;

  // Get format from settings cache if not provided
  const timeFormat = format ?? getCachedTimeFormat();

  if (timeFormat === '24h') {
    // 24-hour format (e.g., "16:30")
    return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}`;
  } else {
    // 12-hour format with AM/PM (e.g., "4:30 PM")
    const period = hours >= 12 ? 'PM' : 'AM';
    const displayHours = hours > 12 ? hours - 12 : hours === 0 ? 12 : hours;
    return `${displayHours}:${minutes.toString().padStart(2, '0')} ${period}`;
  }
}

/**
 * Get time format options for toLocaleTimeString based on user's preference
 * Returns options object compatible with Intl.DateTimeFormatOptions
 */
export function getTimeFormatOptions(format?: '12h' | '24h'): Intl.DateTimeFormatOptions {
  const timeFormat = format ?? getCachedTimeFormat();

  if (timeFormat === '24h') {
    return {
      hour: '2-digit',
      minute: '2-digit',
      hour12: false,
    };
  } else {
    return {
      hour: 'numeric',
      minute: '2-digit',
      hour12: true,
    };
  }
}
