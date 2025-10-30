import type { LucideIcon } from 'lucide-react';

export interface AIQuickAction {
  icon: LucideIcon;
  label: string;
  onClick: () => void;
}

export interface SuggestionState {
  text: string;
  confidence?: number;
  timestamp: number;
  source: 'activity' | 'project' | 'meeting' | 'history';
  metadata?: {
    projectName?: string;
    meetingTime?: string;
    appName?: string;
    isStale?: boolean;
    isUpdated?: boolean;
  };
}

/**
 * Re-export generated backend types
 * These types are auto-generated from Rust structs
 */
export type { WindowContext, ActivityContext } from '@/shared/types/generated';
