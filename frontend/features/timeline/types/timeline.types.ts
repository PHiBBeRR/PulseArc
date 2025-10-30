// Timeline feature types

export interface TimelineEntry {
  id: string;
  project: string;
  task: string;
  startTime: string;
  duration: number; // in minutes
  status: 'pending' | 'approved' | 'suggested';
  startEpoch?: number; // Unix timestamp (seconds) for proper sorting
  isCalendarEvent?: boolean; // Flag to distinguish calendar events from regular entries
  isAllDay?: boolean; // All-day event flag (calendar events only)
  originalSummary?: string; // Original calendar event title (calendar events only)
}

export interface TimelineViewProps {
  entries: TimelineEntry[];
}

export interface TimelineDayViewProps {
  onBack?: () => void;
  onViewModeChange?: (viewMode: 'day' | 'week') => void;
}

export interface DayData {
  day: string;
  hours: number;
  entries: number;
}

export interface MonthSummary {
  totalHours: number;
  totalEntries: number;
  billableHours: number;
  avgHoursPerDay: number;
}

export interface TimelineFilter {
  startDate: Date;
  endDate: Date;
  projects?: string[];
  status?: ('pending' | 'approved' | 'suggested')[];
}
