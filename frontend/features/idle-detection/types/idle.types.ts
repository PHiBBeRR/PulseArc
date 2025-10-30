// Idle Detection feature types

export interface IdleDetectionModalProps {
  isOpen: boolean;
  onKeepTime: () => void;
  onDiscardTime: () => void;
  idleMinutes: number;
}

export interface IdleDetectionState {
  isIdle: boolean;
  idleStartTime: number | null;
  lastActivityTime: number;
  idleDuration: number;
}

export interface IdleDetectionConfig {
  idleThresholdMinutes: number;
  checkIntervalSeconds: number;
  enableIdleDetection: boolean;
}
