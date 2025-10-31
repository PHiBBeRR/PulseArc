// Time Entry feature types
import type { ActivityBreakdown, ActivityContext } from '@/shared/types/generated';

export interface TimeEntry {
  id: string;
  time: string;
  project: string;
  task: string;
  duration: string;
  status: 'pending' | 'approved' | 'suggested';
  confidence?: number;
  description?: string;
  startTime?: Date;
  endTime?: Date;
  durationSeconds?: number;
  source?: 'calendar' | 'ai'; // Source of the suggestion
  shortDate?: string; // Short date format (MM/DD/YYYY)
  category?: 'personal' | 'general' | 'project' | 'ai'; // Category for icon display
  wbsCode?: string; // WBS code from project
  activities?: ActivityBreakdown[]; // : Activity breakdown for blocks
  idleSeconds?: number; // Idle time within entry
}

export interface SaveEntryModalProps {
  isOpen: boolean;
  onClose: () => void;

  onAccept: (data: { project: string; task: string; duration: string }) => void;
  onReject: () => void;
  duration: string; // formatted duration like "1h 15m"
  elapsedSeconds: number; // for AI suggestions
  activityContext?: ActivityContext | null;
}

export interface CompactQuickEntryProps {
  isOpen: boolean;
  onClose: () => void;

  onSave?: (data: EntryData) => void;
  isLoading?: boolean;
  showValidationErrors?: boolean;
}

export interface EntryData {
  project: string;
  task: string;
  duration: string;
  description: string;
}

export interface EntriesViewProps {
  onBack?: () => void;
  onQuickEntry?: () => void;
  showEmpty?: boolean;
  onViewModeChange?: (viewMode: 'day' | 'week') => void;

  onNotificationTriggerReady?: (
    _trigger: (
      _type: 'success' | 'error' | 'info' | 'warning',

      _message: string,

      _action?: { label: string; onClick: () => void }
    ) => void
  ) => void;
}

export interface EntriesPanelProps {
  isOpen: boolean;
  onClose: () => void;
  showEmpty?: boolean;
}

export interface CompactEntriesProps {
  onQuickEntry?: () => void;
}

export interface AISuggestion {
  project: string;
  task: string;
  confidence: number;
  reason: string;
}

export interface EntryFormErrors {
  project: boolean;
  task: boolean;
  duration: boolean;
}
