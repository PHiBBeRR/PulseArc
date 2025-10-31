// Timer feature types

// Unified state type matching Activity Tracker
export type timerState = 'inactive' | 'active' | 'paused' | 'idle';

export interface TimerState {
  status: timerState;
  elapsed: number; // in seconds
  currentProject: string;
  currentTask: string;
  startTime?: Date;
}

export interface TimerProps {
  onEntriesClick?: () => void;
  onSettingsClick?: () => void;
  onAnalyticsClick?: () => void;
  onQuickEntry?: () => void;
  onTimelineClick?: () => void;
  onBuildMyDayClick?: () => void;
  syncStatus?: 'idle' | 'syncing' | 'synced' | 'error';
  onNotificationTriggerReady?: (_trigger: NotificationTrigger) => void;
  onTimerStateChange?: (_status: timerState, _elapsed: number) => void;
  initialStatus?: timerState;
  initialElapsed?: number;
}

export type NotificationTrigger = (
  _type: 'success' | 'error' | 'info' | 'warning',
  _message: string,
  _action?: { label: string; onClick: () => void }
) => void;

export interface TimerIdleState {
  showModal: boolean;
  idleMinutes: number;
  savedElapsed: number;
}
