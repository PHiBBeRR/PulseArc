// Idle Detection Settings Component (Phase 4)
//
// This component provides UI for configuring idle detection settings.

import { invoke } from '@tauri-apps/api/core';
import { useCallback, useEffect, useState } from 'react';

export interface IdleDetectionSettingsProps {
  className?: string;
}

export interface IdleSettings {
  pause_on_idle: boolean;
  idle_threshold_secs: number;
}

export function IdleDetectionSettings({ className = '' }: IdleDetectionSettingsProps) {
  const [pauseOnIdle, setPauseOnIdle] = useState(true);
  const [idleThreshold, setIdleThreshold] = useState(600); // 10 minutes default
  const [isLoading, setIsLoading] = useState(true);

  // Load settings on mount
  useEffect(() => {
    const loadSettings = async () => {
      try {
        const settings = await invoke<IdleSettings>('get_idle_settings');
        setPauseOnIdle(settings.pause_on_idle);
        setIdleThreshold(settings.idle_threshold_secs);
      } catch (error) {
        console.error('Failed to load idle settings:', error);
      } finally {
        setIsLoading(false);
      }
    };

    void loadSettings();
  }, []);

  const handleThresholdChange = useCallback(
    (event: { target: { value: string } }) => {
      const newThreshold = Number(event.target.value);
      const previousThreshold = idleThreshold;
      const previousPauseOnIdle = pauseOnIdle;

      // If selecting 0 (No Idle), disable pause on idle
      const shouldPause = newThreshold > 0;

      setIdleThreshold(newThreshold);
      setPauseOnIdle(shouldPause);

      void (async () => {
        try {
          // Update both settings
          await invoke('set_idle_enabled', { enabled: shouldPause });
          if (shouldPause) {
            await invoke('set_idle_threshold', { threshold_secs: newThreshold });
          }
        } catch (error) {
          console.error('Failed to update idle settings:', error);
          // Revert on error
          setIdleThreshold(previousThreshold);
          setPauseOnIdle(previousPauseOnIdle);
        }
      })();
    },
    [idleThreshold, pauseOnIdle]
  );

  if (isLoading) {
    return (
      <div className={className}>
        <div className="mb-3">
          <h3 className="text-sm text-gray-900 dark:text-gray-100 mb-1">Idle Detection</h3>
          <p className="text-xs text-gray-500 dark:text-gray-400">
            Automatically pause tracking after a period of inactivity
          </p>
        </div>
        <div className="text-sm text-gray-500 dark:text-gray-400">Loading settings...</div>
      </div>
    );
  }

  return (
    <div className={className}>
      <div className="mb-3">
        <h3 className="text-sm text-gray-900 dark:text-gray-100 mb-1">Idle Detection</h3>
        <p className="text-xs text-gray-500 dark:text-gray-400">
          Automatically pause tracking after a period of inactivity
        </p>
      </div>

      <select
        id="idle-threshold"
        value={idleThreshold}
        onChange={handleThresholdChange}
        className="w-full px-3 py-2 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-lg text-sm text-gray-900 dark:text-gray-100 focus:outline-none hover:bg-white/30 dark:hover:bg-white/15 transition-colors appearance-none"
      >
        <option value={0}>No Idle</option>
        <option value={300}>5 minutes</option>
        <option value={600}>10 minutes (recommended)</option>
        <option value={900}>15 minutes</option>
        <option value={1800}>30 minutes</option>
      </select>
    </div>
  );
}
