// Pure function for deriving tracker state from timer state and settings

export type TimerState = 'inactive' | 'active' | 'paused' | 'idle';
export type TrackerState = 'inactive' | 'active' | 'paused' | 'idle';

/**
 * Derives tracker state from timer state
 *
 * Simple Rule: Tracker always mirrors timer state
 * - Timer inactive → Tracker inactive
 * - Timer active → Tracker active
 * - Timer paused → Tracker paused
 * - Timer idle → Tracker idle
 *
 * @param timerState - Current timer state
 * @returns Tracker state (mirrors timer)
 */
export const deriveTrackerState = (timerState: TimerState): TrackerState => {
  return timerState;
};
