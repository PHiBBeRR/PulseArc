// Settings feature types

export interface SettingsViewProps {
  onBack?: () => void;
  onRestartTutorial?: () => void;
}

export interface SettingsPanelProps {
  isOpen: boolean;
  onClose: () => void;
}

export interface Integration {
  id: string;
  name: string;
  icon: string;
  connected: boolean;
}

export interface SettingsState {
  autoApply: boolean;
  notifications: boolean;
  confidence: number;
  integrations: Record<string, boolean>;
  timeFormat: '12h' | '24h';
}

export interface NotificationSettings {
  enabled: boolean;
  showBadges: boolean;
  soundEnabled: boolean;
}

export interface MLSettings {
  autoApply: boolean;
  confidenceThreshold: number;
  enableSuggestions: boolean;
}
