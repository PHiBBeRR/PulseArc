// Analytics feature types

export type TimePeriod = 'week' | 'month' | '3months' | '6months';

export interface AnalyticsViewProps {
  onBack?: () => void;
}

export interface TimeData {
  day?: string;
  week?: string;
  month?: string;
  billable: number;
  nonBillable: number;
  active?: number;
  idle?: number;
}

export interface PieChartData {
  name: string;
  value: number;
  color: string;
  [key: string]: string | number;
}

export interface AnalyticsStats {
  total: number;
  totalBillable: number;
  totalNonBillable: number;
  billablePercentage: number;
  totalActive?: number;
  totalIdle?: number;
  totalIdleKept?: number;
  totalIdleDiscarded?: number;
  totalIdlePending?: number;
  // Future: Idle-adjusted billable calculations
  effectiveWorkTime?: number; // active + kept idle
  adjustedBillable?: number; // billable hours excluding discarded idle
  adjustedNonBillable?: number; // non-billable hours excluding discarded idle
  adjustedBillablePercentage?: number; // adjusted billable / (adjusted billable + adjusted non-billable)
}

export interface AnalyticsPeriodData {
  period: TimePeriod;
  data: TimeData[];
  stats: AnalyticsStats;
}

export interface DailyIdleSummary {
  date: string;
  totalActiveSecs: number;
  totalIdleSecs: number;
  idlePeriodsCount: number;
  idleKeptSecs: number;
  idleDiscardedSecs: number;
  idlePendingSecs: number;
}
