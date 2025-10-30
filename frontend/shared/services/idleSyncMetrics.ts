/**
 * Idle Sync Metrics Service (FEATURE-012)
 * 
 * Provides functions to record idle state synchronization metrics
 * for activity monitor tracking and validation.
 */

import { invoke } from '@tauri-apps/api/core';

/**
 * Record an idle detection event with latency measurement
 * @param latencyMs - Time taken to detect idle after 5-minute threshold
 */
export async function recordIdleDetection(latencyMs: number): Promise<void> {
  try {
    await invoke('record_idle_detection', { latencyMs });
  } catch (error) {
    console.error('Failed to record idle detection:', error);
  }
}

/**
 * Record an activity wake event
 * @param eventType - Type of event that woke from idle (e.g., 'pointermove', 'keydown')
 */
export async function recordActivityWake(eventType: string): Promise<void> {
  try {
    await invoke('record_activity_wake', { eventType });
  } catch (error) {
    console.error('Failed to record activity wake:', error);
  }
}

/**
 * Record a timer-state event emission with latency
 * @param latencyUs - Emission latency in microseconds
 * @param success - Whether the emission was successful
 */
export async function recordTimerEventEmission(
  latencyUs: number,
  success: boolean
): Promise<void> {
  try {
    await invoke('record_timer_event_emission', { latencyUs, success });
  } catch (error) {
    console.error('Failed to record timer event emission:', error);
  }
}

/**
 * Record a timer-state event reception with sync latency
 * @param syncLatencyMs - Time between event emission and reception
 */
export async function recordTimerEventReception(syncLatencyMs: number): Promise<void> {
  try {
    await invoke('record_timer_event_reception', { syncLatencyMs });
  } catch (error) {
    console.error('Failed to record timer event reception:', error);
  }
}

/**
 * Record an invalid payload rejection
 */
export async function recordInvalidPayload(): Promise<void> {
  try {
    await invoke('record_invalid_payload');
  } catch (error) {
    console.error('Failed to record invalid payload:', error);
  }
}

/**
 * Record a state transition with timing
 * @param from - Previous state
 * @param to - New state
 * @param durationMs - Transition duration in milliseconds
 */
export async function recordStateTransition(
  from: string,
  to: string,
  durationMs: number
): Promise<void> {
  try {
    await invoke('record_state_transition', { from, to, durationMs });
  } catch (error) {
    console.error('Failed to record state transition:', error);
  }
}

/**
 * Record an autoStartTracker rule application
 * @param ruleNum - Rule number (1=inactive always, 2=auto on override, 3=mirror exact)
 * @param timerState - Current timer state
 * @param autoStart - autoStartTracker setting value
 * @param isCorrect - Whether the derivation was correct
 */
export async function recordAutoStartTrackerRule(
  ruleNum: number,
  timerState: string,
  autoStart: boolean,
  isCorrect: boolean
): Promise<void> {
  try {
    await invoke('record_auto_start_tracker_rule', {
      ruleNum,
      timerState,
      autoStart,
      isCorrect,
    });
  } catch (error) {
    console.error('Failed to record autoStartTracker rule:', error);
  }
}

/**
 * Idle Sync Metrics namespace for grouped exports
 */
export const idleSyncMetrics = {
  recordIdleDetection,
  recordActivityWake,
  recordTimerEventEmission,
  recordTimerEventReception,
  recordInvalidPayload,
  recordStateTransition,
  recordAutoStartTrackerRule,
};

