/**
 * Unit tests for timer event types and validators
 *
 * Tests the type guards and validators for timer state events that flow between
 * the backend and frontend via Tauri's event system.
 *
 * Test Coverage:
 * - Type Guard Validation: isTimerStateEventV1 correctly validates event payloads
 * - Valid Payloads: All valid timer states (inactive, active, paused, idle)
 * - Valid Sources: Both timer and tracker event sources
 * - Invalid Payloads: Null, undefined, malformed objects
 * - Missing Fields: Handling incomplete event data
 * - Type Safety: Runtime validation matches TypeScript types
 * - Version Handling: Event versioning (v1) validation
 */

import { describe, expect, it } from 'vitest';
import { isTimerStateEventV1, type TimerStateEventV1 } from './timer-events';

describe('isTimerStateEventV1', () => {
  it('should return true for valid TimerStateEventV1 payload', () => {
    const validPayload: TimerStateEventV1 = {
      state: 'active',
      elapsed: 120,
      ts: Date.now(),
      source: 'timer',
      v: 1,
    };
    expect(isTimerStateEventV1(validPayload)).toBe(true);
  });

  it('should return true for all valid states', () => {
    const states: Array<'inactive' | 'active' | 'paused' | 'idle'> = [
      'inactive',
      'active',
      'paused',
      'idle',
    ];

    states.forEach((state) => {
      const payload: TimerStateEventV1 = {
        state,
        elapsed: 0,
        ts: Date.now(),
        source: 'timer',
        v: 1,
      };
      expect(isTimerStateEventV1(payload)).toBe(true);
    });
  });

  it('should return true for both valid sources (timer and tracker)', () => {
    const sources: Array<'timer' | 'tracker'> = ['timer', 'tracker'];
    sources.forEach((source) => {
      const payload: TimerStateEventV1 = {
        state: 'active',
        elapsed: 0,
        ts: Date.now(),
        source,
        v: 1,
      };
      expect(isTimerStateEventV1(payload)).toBe(true);
    });
  });

  it('should return false for null or undefined', () => {
    expect(isTimerStateEventV1(null)).toBe(false);
    expect(isTimerStateEventV1(undefined)).toBe(false);
  });

  it('should return false for wrong version', () => {
    const wrongVersion = {
      state: 'active',
      elapsed: 120,
      ts: Date.now(),
      source: 'timer',
      v: 2, // Wrong version
    };
    expect(isTimerStateEventV1(wrongVersion)).toBe(false);
  });

  it('should return false for missing required fields', () => {
    expect(isTimerStateEventV1({ v: 1 })).toBe(false);
    expect(isTimerStateEventV1({ state: 'active', v: 1 })).toBe(false);
    expect(isTimerStateEventV1({ state: 'active', elapsed: 120, v: 1 })).toBe(false);
    expect(isTimerStateEventV1({ state: 'active', elapsed: 120, ts: Date.now(), v: 1 })).toBe(
      false
    );
  });

  it('should return false for wrong source', () => {
    const wrongSource = {
      state: 'active',
      elapsed: 120,
      ts: Date.now(),
      source: 'other', // Wrong source (valid sources are 'timer' or 'tracker')
      v: 1,
    };
    expect(isTimerStateEventV1(wrongSource)).toBe(false);
  });

  it('should return false for invalid state value', () => {
    const invalidState = {
      state: 'invalid',
      elapsed: 120,
      ts: Date.now(),
      source: 'timer',
      v: 1,
    };
    expect(isTimerStateEventV1(invalidState)).toBe(false);
  });

  it('should return false for wrong field types', () => {
    const wrongTypes = {
      state: 'active',
      elapsed: '120', // Should be number
      ts: Date.now(),
      source: 'timer',
      v: 1,
    };
    expect(isTimerStateEventV1(wrongTypes)).toBe(false);

    const wrongTypes2 = {
      state: 'active',
      elapsed: 120,
      ts: 'now', // Should be number
      source: 'timer',
      v: 1,
    };
    expect(isTimerStateEventV1(wrongTypes2)).toBe(false);
  });

  it('should return false for non-object values', () => {
    expect(isTimerStateEventV1('string')).toBe(false);
    expect(isTimerStateEventV1(123)).toBe(false);
    expect(isTimerStateEventV1(true)).toBe(false);
    expect(isTimerStateEventV1([])).toBe(false);
  });
});
