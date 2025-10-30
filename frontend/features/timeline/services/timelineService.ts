// Timeline business logic service

import { invoke } from '@tauri-apps/api/core';
import type { TimelineEntry, DayData, MonthSummary } from '../types';
import type { TimelineCalendarEvent } from '@/shared/types/generated/TimelineCalendarEvent';
import { formatTimeString } from '@/shared/utils/timeFormat';

// Mock data removed - using real calendar data only

export const timelineService = {
  /**
   * Get timeline entries for a date
   * Returns empty array - use getTimelineWithCalendar() for calendar events
   */
  getTimelineEntries: (): TimelineEntry[] => {
    return [];
  },

  /**
   * Get status color classes
   */
  getStatusColor: (status: string): string => {
    switch (status) {
      case 'suggested':
        return 'bg-blue-500/20 dark:bg-blue-400/20 border-blue-500/30 dark:border-blue-400/30';
      case 'approved':
        return 'bg-blue-500/30 dark:bg-blue-400/30 border-blue-500/40 dark:border-blue-400/40';
      default:
        return 'bg-slate-400/20 dark:bg-slate-500/20 border-slate-400/30 dark:border-slate-500/30';
    }
  },

  /**
   * Calculate position on timeline (percentage)
   */
  calculatePosition: (time: string): number => {
    const parts = time.split(':').map(Number);
    const hours = parts[0] ?? 0;
    const minutes = parts[1] ?? 0;
    const totalMinutes = (hours - 8) * 60 + minutes; // Start from 8 AM
    return (totalMinutes / (13 * 60)) * 100; // 13 hour range (8 AM to 9 PM)
  },

  /**
   * Calculate width on timeline (percentage)
   */
  calculateWidth: (duration: number): number => {
    return (duration / (13 * 60)) * 100;
  },

  /**
   * Format time display based on user's time format preference (12h or 24h)
   */
  formatTime: (time: string, format?: '12h' | '24h'): string => {
    return formatTimeString(time, format);
  },

  /**
   * Format duration
   */
  formatDuration: (minutes: number): string => {
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
   * Get week data for the current week (Sunday to Saturday)
   * Fetches calendar events for each day and calculates totals
   */
  getWeekData: async (referenceDate: Date = new Date()): Promise<DayData[]> => {
    // Get the start of the week (Sunday)
    const startOfWeek = new Date(referenceDate);
    startOfWeek.setDate(referenceDate.getDate() - referenceDate.getDay());
    startOfWeek.setHours(0, 0, 0, 0);

    const weekData: DayData[] = [];

    // Fetch data for each day of the week (Sun-Sat)
    for (let i = 0; i < 7; i++) {
      const currentDay = new Date(startOfWeek);
      currentDay.setDate(startOfWeek.getDate() + i);

      try {
        // Get calendar events for this day
        const events = await timelineService.getTimelineWithCalendar(currentDay);

        // Calculate total hours (convert minutes to hours)
        const totalMinutes = events.reduce((sum, event) => sum + event.duration, 0);
        const totalHours = Math.round((totalMinutes / 60) * 10) / 10;

        // Format day name (e.g., "Mon", "Tue")
        const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
        const dayName = dayNames[currentDay.getDay()] ?? 'Day';

        weekData.push({
          day: dayName,
          hours: totalHours,
          entries: events.length,
        });
      } catch (error) {
        console.error(`Failed to fetch data for ${currentDay.toDateString()}:`, error);
        // Add empty data for failed day
        const dayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];
        weekData.push({
          day: dayNames[currentDay.getDay()] ?? 'Day',
          hours: 0,
          entries: 0,
        });
      }
    }

    return weekData;
  },

  /**
   * Get all events for the week in calendar format
   * Returns a map of day index (0-6) to events for that day
   */
  getWeekCalendarEvents: async (referenceDate: Date = new Date()): Promise<Map<number, TimelineEntry[]>> => {
    // Get the start of the week (Sunday)
    const startOfWeek = new Date(referenceDate);
    startOfWeek.setDate(referenceDate.getDate() - referenceDate.getDay());
    startOfWeek.setHours(0, 0, 0, 0);

    const weekEvents = new Map<number, TimelineEntry[]>();

    // Fetch events for each day of the week (Sun-Sat)
    for (let i = 0; i < 7; i++) {
      const currentDay = new Date(startOfWeek);
      currentDay.setDate(startOfWeek.getDate() + i);

      try {
        const events = await timelineService.getTimelineWithCalendar(currentDay);
        weekEvents.set(i, events);
      } catch (error) {
        console.error(`Failed to fetch events for ${currentDay.toDateString()}:`, error);
        weekEvents.set(i, []);
      }
    }

    return weekEvents;
  },

  /**
   * Get month summary
   */
  getMonthSummary: (): MonthSummary => {
    return {
      totalHours: 172.5,
      totalEntries: 248,
      billableHours: 145.2,
      avgHoursPerDay: 6.9,
    };
  },

  /**
   * Get hour markers for display
   */
  getHourMarkers: (): number[] => {
    return Array.from({ length: 13 }, (_, i) => i + 8); // 8 AM to 8 PM
  },

  /**
   * Format hour marker
   */
  formatHourMarker: (hour: number): string => {
    if (hour === 12) return '12p';
    return hour > 12 ? `${hour - 12}p` : `${hour}a`;
  },

  /**
   * Calculate total hours for day
   */
  calculateTotalHours: (entries: TimelineEntry[]): number => {
    const totalMinutes = entries.reduce((sum, entry) => sum + entry.duration, 0);
    return Math.round((totalMinutes / 60) * 10) / 10; // Round to 1 decimal
  },

  /**
   * Group entries by status
   */
  groupByStatus: (entries: TimelineEntry[]): Record<string, TimelineEntry[]> => {
    return entries.reduce(
      (acc, entry) => {
        acc[entry.status] ??= [];
        acc[entry.status]?.push(entry);
        return acc;
      },
      {} as Record<string, TimelineEntry[]>
    );
  },

  /**
   * Get status badge label
   */
  getStatusLabel: (status: string): string => {
    switch (status) {
      case 'suggested':
        return 'Suggested';
      case 'approved':
        return 'Approved';
      case 'pending':
        return 'Pending';
      default:
        return status;
    }
  },

  /**
   * Get calendar events for timeline display
   * Fetches events from Google Calendar for the specified date range
   * Includes 2 weeks of prior events to populate historical data
   */
  getCalendarEvents: async (date: Date): Promise<TimelineCalendarEvent[]> => {
    // IMPORTANT: Create new Date objects from the date components to avoid mutation
    // Start from 2 weeks (14 days) before the selected date
    const twoWeeksAgo = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0, 0);
    twoWeeksAgo.setDate(twoWeeksAgo.getDate() - 14);

    // End at 11:59:59 PM of the selected date (LOCAL TIME)
    const endOfDay = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 59, 999);

    const startEpoch = Math.floor(twoWeeksAgo.getTime() / 1000);
    const endEpoch = Math.floor(endOfDay.getTime() / 1000);

    // IMPORTANT: Tauri 2.x expects camelCase for command parameters
    const calendarEvents = await invoke<TimelineCalendarEvent[]>(
      'get_calendar_events_for_timeline',
      {
        startDate: startEpoch,
        endDate: endEpoch,
      },
    );

    return calendarEvents;
  },

  /**
   * Get timeline entries with calendar events
   * Returns ONLY calendar events (not merged with activity snapshots)
   * IMPORTANT: Filters events to only include those that overlap with the selected date (local time)
   * Handles multi-day events by splitting them across day boundaries
   */
  getTimelineWithCalendar: async (date: Date): Promise<TimelineEntry[]> => {
    // Fetch calendar events from ALL connected providers (Google + Microsoft)
    const calendarEvents = await timelineService.getCalendarEvents(date);

    // Filter and potentially split calendar events for the selected day
    const filteredCalendarEvents: TimelineEntry[] = [];

    for (const event of calendarEvents) {
      const eventStart = new Date(event.startEpoch * 1000);
      const eventEnd = new Date((event.startEpoch + event.duration * 60) * 1000);

      // Get day boundaries for selected date (local timezone)
      const dayStart = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 0, 0, 0, 0);
      const dayEnd = new Date(date.getFullYear(), date.getMonth(), date.getDate(), 23, 59, 59, 999);

      // Check if event overlaps with this day
      if (eventEnd <= dayStart || eventStart > dayEnd) {
        continue; // Event doesn't overlap with this day
      }

      // Calculate overlap start and end times (clamp to day boundaries)
      const overlapStart = eventStart < dayStart ? dayStart : eventStart;
      const overlapEnd = eventEnd > dayEnd ? dayEnd : eventEnd;

      // Calculate duration of the overlap in minutes
      const overlapDurationMs = overlapEnd.getTime() - overlapStart.getTime();
      const overlapDurationMinutes = Math.round(overlapDurationMs / (1000 * 60));

      // Convert overlap start to local time in 24h format (internal representation)
      const localHours = overlapStart.getHours();
      const localMinutes = overlapStart.getMinutes();
      const localStartTime = `${localHours.toString().padStart(2, '0')}:${localMinutes.toString().padStart(2, '0')}`;

      // Create timeline entry for this day's portion
      filteredCalendarEvents.push({
        ...event,
        startEpoch: Math.floor(overlapStart.getTime() / 1000),
        duration: overlapDurationMinutes,
        startTime: localStartTime,
        status: event.status as 'pending' | 'approved' | 'suggested',
        isCalendarEvent: true,
      });
    }

    // Sort by startEpoch for chronological order
    // Note: All calendar events have startEpoch set, so non-null assertion is safe
    return filteredCalendarEvents.sort((a, b) => (a.startEpoch ?? 0) - (b.startEpoch ?? 0));
  },

  /**
   * Parse time string (HH:MM) to epoch timestamp for given date
   */
  parseTimeToEpoch: (time: string, date: Date): number => {
    const parts = time.split(':').map(Number);
    const hours = parts[0] ?? 0;
    const minutes = parts[1] ?? 0;
    const dateWithTime = new Date(date);
    dateWithTime.setHours(hours, minutes, 0, 0);
    return Math.floor(dateWithTime.getTime() / 1000);
  },
};
