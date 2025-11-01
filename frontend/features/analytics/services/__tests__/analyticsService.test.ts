import { describe, expect, it } from 'vitest';

import { analyticsService } from '../analyticsService';

import type { TimeEntryAnalytics } from '@/shared/types/generated';

const createAnalyticsEntry = (
  overrides: Partial<TimeEntryAnalytics> & { date: string }
): TimeEntryAnalytics => ({
  date: overrides.date,
  total_minutes: overrides.total_minutes ?? 0,
  billable_minutes: overrides.billable_minutes ?? 0,
  non_billable_minutes: overrides.non_billable_minutes ?? 0,
  idle_discarded_minutes: overrides.idle_discarded_minutes ?? 0,
  idle_kept_minutes: overrides.idle_kept_minutes ?? 0,
  idle_pending_minutes: overrides.idle_pending_minutes ?? 0,
  adjusted_billable_minutes: overrides.adjusted_billable_minutes ?? 0,
  adjusted_non_billable_minutes: overrides.adjusted_non_billable_minutes ?? 0,
  effective_work_minutes: overrides.effective_work_minutes ?? 0,
  time_entries_count: overrides.time_entries_count ?? 0,
});

describe('analyticsService.convertAnalyticsToTimeData', () => {
  it('aggregates analytics into weekly buckets for month period', () => {
    const analytics: TimeEntryAnalytics[] = [
      createAnalyticsEntry({
        date: '2024-04-01',
        total_minutes: 480,
        idle_discarded_minutes: 60,
        adjusted_billable_minutes: 270,
        adjusted_non_billable_minutes: 150,
      }),
      createAnalyticsEntry({
        date: '2024-04-03',
        total_minutes: 240,
        idle_discarded_minutes: 30,
        adjusted_billable_minutes: 110,
        adjusted_non_billable_minutes: 100,
      }),
      createAnalyticsEntry({
        date: '2024-04-10',
        total_minutes: 300,
        idle_discarded_minutes: 45,
        adjusted_billable_minutes: 190,
        adjusted_non_billable_minutes: 90,
      }),
      // Invalid record should be ignored gracefully
      createAnalyticsEntry({
        date: 'not-a-date',
        adjusted_billable_minutes: 999,
      }),
    ];

    const result = analyticsService.convertAnalyticsToTimeData(analytics, 'month');

    expect(result).toHaveLength(2);
    expect(result[0].week).toBe('Week of Apr 1');
    expect(result[0].billable).toBeCloseTo(380 / 60, 6);
    expect(result[0].nonBillable).toBeCloseTo(250 / 60, 6);
    expect(result[0].active).toBeCloseTo(630 / 60, 6);
    expect(result[0].idle).toBeCloseTo(90 / 60, 6);

    expect(result[1].week).toBe('Week of Apr 8');
    expect(result[1].billable).toBeCloseTo(190 / 60, 6);
    expect(result[1].nonBillable).toBeCloseTo(90 / 60, 6);
    expect(result[1].active).toBeCloseTo(255 / 60, 6);
    expect(result[1].idle).toBeCloseTo(45 / 60, 6);
  });

  it('groups analytics by month for multi-month periods', () => {
    const analytics: TimeEntryAnalytics[] = [
      createAnalyticsEntry({
        date: '2024-04-02',
        total_minutes: 420,
        idle_discarded_minutes: 60,
        adjusted_billable_minutes: 260,
        adjusted_non_billable_minutes: 140,
      }),
      createAnalyticsEntry({
        date: '2024-04-18',
        total_minutes: 360,
        idle_discarded_minutes: 20,
        adjusted_billable_minutes: 200,
        adjusted_non_billable_minutes: 140,
      }),
      createAnalyticsEntry({
        date: '2024-05-05',
        total_minutes: 180,
        idle_discarded_minutes: 20,
        adjusted_billable_minutes: 120,
        adjusted_non_billable_minutes: 60,
      }),
    ];

    const result = analyticsService.convertAnalyticsToTimeData(analytics, '3months');

    expect(result).toHaveLength(2);
    expect(result[0].month).toBe('Apr 2024');
    expect(result[0].billable).toBeCloseTo((260 + 200) / 60, 6);
    expect(result[0].nonBillable).toBeCloseTo((140 + 140) / 60, 6);
    expect(result[0].active).toBeCloseTo((420 - 60 + (360 - 20)) / 60, 6);
    expect(result[0].idle).toBeCloseTo((60 + 20) / 60, 6);

    expect(result[1].month).toBe('May 2024');
    expect(result[1].billable).toBeCloseTo(120 / 60, 6);
    expect(result[1].nonBillable).toBeCloseTo(60 / 60, 6);
    expect(result[1].active).toBeCloseTo((180 - 20) / 60, 6);
    expect(result[1].idle).toBeCloseTo(20 / 60, 6);
  });

  it('returns daily data for week period without aggregation', () => {
    const analytics: TimeEntryAnalytics[] = [
      createAnalyticsEntry({
        date: '2024-04-01',
        total_minutes: 480,
        idle_discarded_minutes: 60,
        adjusted_billable_minutes: 270,
        adjusted_non_billable_minutes: 150,
      }),
      createAnalyticsEntry({
        date: '2024-04-02',
        total_minutes: 300,
        idle_discarded_minutes: 45,
        adjusted_billable_minutes: 180,
        adjusted_non_billable_minutes: 90,
      }),
    ];

    const result = analyticsService.convertAnalyticsToTimeData(analytics, 'week');

    expect(result).toHaveLength(2);
    expect(result[0].day).toBe('Mon');
    expect(result[0].billable).toBeCloseTo(270 / 60, 6);
    expect(result[0].active).toBeCloseTo((480 - 60) / 60, 6);
    expect(result[0].idle).toBeCloseTo(60 / 60, 6);

    expect(result[1].day).toBe('Tue');
    expect(result[1].billable).toBeCloseTo(180 / 60, 6);
  });
});
