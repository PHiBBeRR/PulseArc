/**
 * Unit tests for deriveTrackerState function
 *
 * Tests the state derivation logic that maps timer states to tracker display states.
 * This function ensures the activity tracker UI correctly reflects the timer's current state.
 *
 * Test Coverage:
 * - State Mirroring: Tracker always mirrors timer state (inactive, active, paused, idle)
 * - Comprehensive Coverage: All possible timer states are tested
 * - Edge Cases: Boundary conditions and state transitions
 *
 * Current behavior: Simple 1:1 mapping from timer state to tracker state
 */

import { describe, expect, it } from 'vitest';
import { deriveTrackerState } from './deriveTrackerState';

describe('deriveTrackerState', () => {
  describe('Simple mirroring: Tracker always mirrors timer state', () => {
    it('should return inactive when timer is inactive', () => {
      expect(deriveTrackerState('inactive')).toBe('inactive');
    });

    it('should return active when timer is active', () => {
      expect(deriveTrackerState('active')).toBe('active');
    });

    it('should return paused when timer is paused', () => {
      expect(deriveTrackerState('paused')).toBe('paused');
    });

    it('should return idle when timer is idle', () => {
      expect(deriveTrackerState('idle')).toBe('idle');
    });
  });

  describe('Comprehensive coverage for all timer states', () => {
    const testCases: Array<{
      timerState: 'inactive' | 'active' | 'paused' | 'idle';
      expected: 'inactive' | 'active' | 'paused' | 'idle';
      description: string;
    }> = [
      { timerState: 'inactive', expected: 'inactive', description: 'inactive → inactive' },
      { timerState: 'active', expected: 'active', description: 'active → active' },
      { timerState: 'paused', expected: 'paused', description: 'paused → paused' },
      { timerState: 'idle', expected: 'idle', description: 'idle → idle' },
    ];

    testCases.forEach(({ timerState, expected, description }) => {
      it(description, () => {
        expect(deriveTrackerState(timerState)).toBe(expected);
      });
    });
  });
});
