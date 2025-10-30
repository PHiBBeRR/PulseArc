// Shared timer event types and utilities for cross-window communication

export const TIMER_STATE_EVT = 'Pulsarc-state-v1' as const;

export interface TimerStateEventV1 {
  state: 'inactive' | 'active' | 'paused' | 'idle';
  elapsed: number;
  ts: number;
  source: 'timer' | 'tracker';
  v: 1;
}

/**
 * Type guard for runtime validation of timer state events
 * @param payload - Unknown payload from event listener
 * @returns True if payload is a valid TimerStateEventV1
 */
export function isTimerStateEventV1(payload: unknown): payload is TimerStateEventV1 {
  return (
    typeof payload === 'object' &&
    payload !== null &&
    'v' in payload &&
    payload.v === 1 &&
    'state' in payload &&
    typeof (payload as Record<string, unknown>).state === 'string' &&
    ['inactive', 'active', 'paused', 'idle'].includes((payload as Record<string, unknown>).state as string) &&
    'elapsed' in payload &&
    typeof (payload as Record<string, unknown>).elapsed === 'number' &&
    'ts' in payload &&
    typeof (payload as Record<string, unknown>).ts === 'number' &&
    'source' in payload &&
    ['timer', 'tracker'].includes((payload as Record<string, unknown>).source as string)
  );
}

/**
 * Safe wrapper for emitting Tauri events with error handling
 * @param channel - Event channel name
 * @param payload - Event payload
 */
export async function safeEmit(channel: string, payload: unknown): Promise<void> {
  try {
    const { emit } = await import('@tauri-apps/api/event');
    await emit(channel, payload);
  } catch (error) {
    console.error(`[emit-error] Failed to emit event on channel "${channel}":`, error);
    // Event emission failed - state sync may be inconsistent
    // Future enhancement: could fall back to localStorage-based sync
    throw error;
  }
}
