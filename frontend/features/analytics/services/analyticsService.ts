// Analytics business logic service

import type { IdleSummary, TimeEntryAnalytics } from '@/shared/types/generated';
import { invoke } from '@tauri-apps/api/core';
import type {
  AnalyticsStats,
  DailyIdleSummary,
  PieChartData,
  TimeData,
  TimePeriod,
} from '../types';

export const analyticsService = {
  /**
   * Fetch idle summary for a specific date
   * @param date - Date string in YYYY-MM-DD format
   */
  fetchIdleSummary: async (date: string): Promise<IdleSummary> => {
    try {
      return await invoke<IdleSummary>('get_idle_summary', { date });
    } catch (error) {
      console.error('Failed to fetch idle summary:', error);
      // Return zero values on error (graceful degradation)
      return {
        total_active_secs: 0,
        total_idle_secs: 0,
        idle_periods_count: 0,
        idle_kept_secs: 0,
        idle_discarded_secs: 0,
        idle_pending_secs: 0,
      };
    }
  },

  /**
   * Fetch idle summaries for a date range
   * @param startDate - Start date string in YYYY-MM-DD format
   * @param endDate - End date string in YYYY-MM-DD format
   */
  fetchIdleSummariesForRange: async (
    startDate: string,
    endDate: string
  ): Promise<DailyIdleSummary[]> => {
    const summaries: DailyIdleSummary[] = [];
    const start = new Date(startDate);
    const end = new Date(endDate);

    // Iterate through each day in the range
    for (let d = new Date(start); d <= end; d.setDate(d.getDate() + 1)) {
      const dateStr = d.toISOString().split('T')[0]; // YYYY-MM-DD
      const summary = await analyticsService.fetchIdleSummary(dateStr);
      summaries.push({
        date: dateStr,
        totalActiveSecs: summary.total_active_secs,
        totalIdleSecs: summary.total_idle_secs,
        idlePeriodsCount: summary.idle_periods_count,
        idleKeptSecs: summary.idle_kept_secs,
        idleDiscardedSecs: summary.idle_discarded_secs,
        idlePendingSecs: summary.idle_pending_secs,
      });
    }

    return summaries;
  },

  /**
   * Fetch time entry analytics with idle period adjustments
   * @param startDate - Start date string in YYYY-MM-DD format
   * @param endDate - End date string in YYYY-MM-DD format
   * @returns Time entry analytics with adjusted billable hours
   */
  fetchTimeEntryAnalytics: async (
    startDate: string,
    endDate: string
  ): Promise<TimeEntryAnalytics[]> => {
    try {
      return await invoke<TimeEntryAnalytics[]>('get_time_entry_analytics', {
        startDate,
        endDate,
      });
    } catch (error) {
      console.error('Failed to fetch time entry analytics:', error);
      return [];
    }
  },

  /**
   * Convert TimeEntryAnalytics to TimeData format for charts
   * @param analytics - Array of TimeEntryAnalytics from backend
   * @param period - Time period for grouping (week/month/etc)
   * @returns TimeData array formatted for recharts
   */
  convertAnalyticsToTimeData: (analytics: TimeEntryAnalytics[], period: TimePeriod): TimeData[] => {
    if (analytics.length === 0) return [];

    // For week period, return daily data
    if (period === 'week') {
      return analytics.map((a) => ({
        day: new Date(a.date).toLocaleDateString('en-US', { weekday: 'short' }),
        billable: a.adjusted_billable_minutes / 60, // Convert to hours
        nonBillable: a.adjusted_non_billable_minutes / 60,
        active: (a.total_minutes - a.idle_discarded_minutes) / 60, // Effective work time
        idle: a.idle_discarded_minutes / 60,
      }));
    }

    type GroupedTotals = {
      billableMinutes: number;
      nonBillableMinutes: number;
      activeMinutes: number;
      idleMinutes: number;
      label: string;
      labelKey: 'week' | 'month';
    };

    const getUtcMidnight = (input: Date): Date => {
      return new Date(Date.UTC(input.getUTCFullYear(), input.getUTCMonth(), input.getUTCDate()));
    };

    const getWeekGrouping = (date: Date) => {
      const weekStart = getUtcMidnight(date);
      const weekday = weekStart.getUTCDay();
      const mondayOffset = (weekday + 6) % 7; // Monday as start of week
      weekStart.setUTCDate(weekStart.getUTCDate() - mondayOffset);

      return {
        key: weekStart.getTime(),
        label: `Week of ${weekStart.toLocaleDateString('en-US', {
          month: 'short',
          day: 'numeric',
        })}`,
        labelKey: 'week' as const,
      };
    };

    const getMonthGrouping = (date: Date) => {
      const monthStart = new Date(Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), 1));
      return {
        key: monthStart.getTime(),
        label: monthStart.toLocaleDateString('en-US', { month: 'short', year: 'numeric' }),
        labelKey: 'month' as const,
      };
    };

    const grouped = new Map<number, GroupedTotals>();

    for (const entry of analytics) {
      const parsedDate = new Date(entry.date);
      if (Number.isNaN(parsedDate.getTime())) {
        continue;
      }

      const { key, label, labelKey } =
        period === 'month' ? getWeekGrouping(parsedDate) : getMonthGrouping(parsedDate);

      const existing = grouped.get(key) ?? {
        billableMinutes: 0,
        nonBillableMinutes: 0,
        activeMinutes: 0,
        idleMinutes: 0,
        label,
        labelKey,
      };

      existing.billableMinutes += entry.adjusted_billable_minutes;
      existing.nonBillableMinutes += entry.adjusted_non_billable_minutes;
      existing.activeMinutes += entry.total_minutes - entry.idle_discarded_minutes;
      existing.idleMinutes += entry.idle_discarded_minutes;
      existing.label = label;
      existing.labelKey = labelKey;

      grouped.set(key, existing);
    }

    return Array.from(grouped.entries())
      .sort(([keyA], [keyB]) => keyA - keyB)
      .map(([, totals]) => ({
        [totals.labelKey]: totals.label,
        billable: totals.billableMinutes / 60,
        nonBillable: totals.nonBillableMinutes / 60,
        active: totals.activeMinutes / 60,
        idle: totals.idleMinutes / 60,
      }));
  },

  /**
   * Calculate analytics statistics from TimeEntryAnalytics data
   *
   * This implementation now uses real time entry data with idle period adjustments.
   *
   * Idle time integration:
   * - Effective Work Time = Total Time - Discarded Idle
   * - Adjusted Billable = Billable entries - (Discarded idle within billable periods)
   * - Pending idle periods create uncertainty in billable calculations
   */
  calculateStats: (
    data: TimeData[],
    idleSummaries?: DailyIdleSummary[],
    timeEntryAnalytics?: TimeEntryAnalytics[]
  ): AnalyticsStats => {
    // If we have time entry analytics, use that for accurate calculations
    if (timeEntryAnalytics && timeEntryAnalytics.length > 0) {
      const totalBillable = timeEntryAnalytics.reduce((sum, a) => sum + a.billable_minutes / 60, 0);
      const totalNonBillable = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.non_billable_minutes / 60,
        0
      );
      const adjustedBillable = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.adjusted_billable_minutes / 60,
        0
      );
      const adjustedNonBillable = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.adjusted_non_billable_minutes / 60,
        0
      );
      const effectiveWorkTime = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.effective_work_minutes / 60,
        0
      );
      const totalIdleDiscarded = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.idle_discarded_minutes / 60,
        0
      );
      const totalIdleKept = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.idle_kept_minutes / 60,
        0
      );
      const totalIdlePending = timeEntryAnalytics.reduce(
        (sum, a) => sum + a.idle_pending_minutes / 60,
        0
      );

      const total = totalBillable + totalNonBillable;
      const billablePercentage = total > 0 ? Math.round((totalBillable / total) * 100) : 0;
      const adjustedTotal = adjustedBillable + adjustedNonBillable;
      const adjustedBillablePercentage =
        adjustedTotal > 0 ? Math.round((adjustedBillable / adjustedTotal) * 100) : 0;

      return {
        total,
        totalBillable,
        totalNonBillable,
        billablePercentage,
        totalActive: effectiveWorkTime,
        totalIdle: totalIdleDiscarded + totalIdleKept + totalIdlePending,
        totalIdleKept,
        totalIdleDiscarded,
        totalIdlePending,
        effectiveWorkTime,
        adjustedBillable,
        adjustedNonBillable,
        adjustedBillablePercentage,
      };
    }

    // Fallback to mock data calculations (legacy)
    const totalBillable = data.reduce((sum, item) => sum + item.billable, 0);
    const totalNonBillable = data.reduce((sum, item) => sum + item.nonBillable, 0);
    const total = totalBillable + totalNonBillable;
    const billablePercentage = total > 0 ? Math.round((totalBillable / total) * 100) : 0;

    // Calculate idle time totals if summaries provided
    let totalActive = 0;
    let totalIdle = 0;
    let totalIdleKept = 0;
    let totalIdleDiscarded = 0;
    let totalIdlePending = 0;
    let effectiveWorkTime: number | undefined;

    if (idleSummaries) {
      totalActive = idleSummaries.reduce((sum, s) => sum + s.totalActiveSecs / 3600, 0); // Convert to hours
      totalIdle = idleSummaries.reduce((sum, s) => sum + s.totalIdleSecs / 3600, 0);
      totalIdleKept = idleSummaries.reduce((sum, s) => sum + s.idleKeptSecs / 3600, 0);
      totalIdleDiscarded = idleSummaries.reduce((sum, s) => sum + s.idleDiscardedSecs / 3600, 0);
      totalIdlePending = idleSummaries.reduce((sum, s) => sum + s.idlePendingSecs / 3600, 0);

      // Effective work time = active time + kept idle (time user chose to count)
      effectiveWorkTime = totalActive + totalIdleKept;
    }

    return {
      total,
      totalBillable,
      totalNonBillable,
      billablePercentage,
      totalActive,
      totalIdle,
      totalIdleKept,
      totalIdleDiscarded,
      totalIdlePending,
      effectiveWorkTime,
    };
  },

  /**
   * Get pie chart data
   */
  getPieChartData: (stats: AnalyticsStats): PieChartData[] => {
    return [
      { name: 'Billable', value: stats.totalBillable, color: '#3b82f6' },
      { name: 'Non-Billable', value: stats.totalNonBillable, color: '#94a3b8' },
    ];
  },

  /**
   * Get X-axis key for charts based on period
   */
  getXAxisKey: (period: TimePeriod): string => {
    if (period === 'week') return 'day';
    if (period === 'month') return 'week';
    return 'month';
  },

  /**
   * Get period label for display
   */
  getPeriodLabel: (period: TimePeriod): string => {
    switch (period) {
      case 'week':
        return 'Past Week';
      case 'month':
        return 'Past Month';
      case '3months':
        return 'Past 3 Months';
      case '6months':
        return 'Past 6 Months';
    }
  },

  /**
   * Format hours for display
   */
  formatHours: (hours: number): string => {
    return `${hours.toFixed(1)}h`;
  },

  /**
   * Calculate average hours per day
   */
  calculateAveragePerDay: (total: number, period: TimePeriod): number => {
    const days = period === 'week' ? 7 : period === 'month' ? 30 : 90;
    return total / days;
  },

  /**
   * Get trend direction (up, down, neutral)
   */
  getTrend: (current: number, previous: number): 'up' | 'down' | 'neutral' => {
    if (current > previous) return 'up';
    if (current < previous) return 'down';
    return 'neutral';
  },

  /**
   * Get date range for a period
   * @param period - Time period (week, month, 3months, 6months)
   * @returns Object with startDate and endDate in YYYY-MM-DD format
   */
  getDateRangeForPeriod: (period: TimePeriod): { startDate: string; endDate: string } => {
    const today = new Date();
    today.setHours(0, 0, 0, 0); // Start of today
    const endDate = today.toISOString().split('T')[0];

    const start = new Date(today);

    switch (period) {
      case 'week':
        start.setDate(today.getDate() - 6); // Last 7 days including today
        break;
      case 'month':
        start.setDate(today.getDate() - 29); // Last 30 days including today
        break;
      case '3months':
        start.setMonth(today.getMonth() - 3);
        break;
      case '6months':
        start.setMonth(today.getMonth() - 6);
        break;
    }

    const startDate = start.toISOString().split('T')[0];
    return { startDate, endDate };
  },

  /**
   * Format seconds to hours with 1 decimal place
   */
  formatSecsToHours: (secs: number): string => {
    const hours = secs / 3600;
    return hours.toFixed(1);
  },
};
