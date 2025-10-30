// Shared types used across features

export interface TimeEntry {
  id: string;
  project: string;
  task: string;
  startTime: Date;
  endTime?: Date;
  duration: number; // in seconds
  isRunning: boolean;
}

export interface Project {
  id: string;
  name: string;
  color?: string;
  description?: string;
}

export interface AppSettings {
  theme: 'light' | 'dark';
  compactMode: boolean;
  alwaysOnTop: boolean;
  soundEnabled: boolean;
  notificationsEnabled: boolean;
  idleDetectionEnabled: boolean;
  idleThresholdMinutes: number;
}

export interface NotificationConfig {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  duration?: number;
}

export type ViewMode = 'timer' | 'entries' | 'analytics' | 'timeline' | 'settings';
