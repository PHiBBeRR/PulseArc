import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import { useSuggestionManager } from './useSuggestionManager';
import type { ActivityContext } from '../types';
import { createMockActivityContext } from '@/shared/test/fixtures/backend-types';

describe('useSuggestionManager (FIX-010 Issue #8)', () => {
  const originalLog = console.log;

  beforeEach(() => {
    vi.useFakeTimers();
    console.log = vi.fn();
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    console.log = originalLog;
  });

  const createMockContext = (detectedActivity: string): ActivityContext =>
    createMockActivityContext({
      detected_activity: detectedActivity,
    });

  describe('Basic Behavior', () => {
    it('should create suggestion when activity is detected and tracking is active', () => {
      const { result } = renderHook(() =>
        useSuggestionManager({
          activityContext: createMockContext('Writing tests'),
          inputValue: '',
          userHasTyped: false,
          isTracking: true,
        })
      );

      expect(result.current.currentSuggestion).toEqual({
        text: 'Writing tests',
        confidence: 0.85,
        timestamp: expect.any(Number),
        source: 'activity',
        metadata: {
          appName: 'Test App',
        },
      });
    });

    it('should not create suggestion when tracking is inactive', () => {
      const { result } = renderHook(() =>
        useSuggestionManager({
          activityContext: createMockContext('Writing tests'),
          inputValue: '',
          userHasTyped: false,
          isTracking: false,
        })
      );

      expect(result.current.currentSuggestion).toBeNull();
    });

    it('should not create suggestion when activity is empty', () => {
      const { result } = renderHook(() =>
        useSuggestionManager({
          activityContext: createMockContext('   '),
          inputValue: '',
          userHasTyped: false,
          isTracking: true,
        })
      );

      expect(result.current.currentSuggestion).toBeNull();
    });
  });

  describe('Edge Case: User Typed Then Stopped (Issue #8)', () => {
    it('should NOT update suggestion if user is actively typing', () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('Initial activity'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      // Initial suggestion created
      expect(result.current.currentSuggestion?.text).toBe('Initial activity');

      // User starts typing
      rerender({
        activityContext: createMockContext('New activity'),
        inputValue: 'User is typing',
        userHasTyped: true,
      });

      // Suggestion should NOT update while user is typing (within debounce period)
      expect(result.current.currentSuggestion?.text).toBe('Initial activity');
    });

    it('should update suggestion if user typed but stopped AND activity changed significantly', async () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('Writing documentation'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      // Initial suggestion
      expect(result.current.currentSuggestion?.text).toBe('Writing documentation');

      // User typed something
      rerender({
        activityContext: createMockContext('Writing documentation'),
        inputValue: 'Documenting API',
        userHasTyped: true,
      });

      // Wait for debounce timeout (500ms)
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });

      // Activity changed significantly (user switched to a different task)
      rerender({
        activityContext: createMockContext('Reviewing pull requests'),
        inputValue: 'Documenting API',
        userHasTyped: false,
      });

      // Suggestion SHOULD update because:
      // 1. User stopped typing (debounce elapsed)
      // 2. Activity changed significantly
      expect(result.current.currentSuggestion?.text).toBe('Reviewing pull requests');
      expect(console.log).toHaveBeenCalledWith(
        '🔍 Suggestion: Activity changed significantly, updating suggestion'
      );
    });

    it('should NOT override user input if activity matches what they typed', async () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('Writing tests'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      // Initial suggestion
      expect(result.current.currentSuggestion?.text).toBe('Writing tests');

      // User typed something similar
      rerender({
        activityContext: createMockContext('Writing unit tests'),
        inputValue: 'tests',
        userHasTyped: true,
      });

      // Wait for debounce
      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });

      // Activity matches user input (fuzzy match)
      rerender({
        activityContext: createMockContext('Writing unit tests'),
        inputValue: 'tests',
        userHasTyped: false,
      });

      // Suggestion should NOT update (activity matches user input)
      expect(result.current.currentSuggestion?.text).toBe('Writing tests');
      expect(console.log).toHaveBeenCalledWith(
        '🔍 Suggestion: Activity matches user input, keeping user input'
      );
    });

    it('should allow updates if input is empty after user stopped typing', async () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('Initial activity'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      expect(result.current.currentSuggestion?.text).toBe('Initial activity');

      // User typed then cleared input
      rerender({
        activityContext: createMockContext('New activity'),
        inputValue: '',
        userHasTyped: false,
      });

      // Suggestion should update (no user input)
      expect(result.current.currentSuggestion?.text).toBe('New activity');
    });
  });

  describe('Fuzzy Matching', () => {
    it('should recognize partial matches (activity contains input)', async () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('Writing documentation'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      rerender({
        activityContext: createMockContext('Writing API documentation'),
        inputValue: 'documentation',
        userHasTyped: true,
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });

      rerender({
        activityContext: createMockContext('Writing API documentation'),
        inputValue: 'documentation',
        userHasTyped: false,
      });

      // Should not override (activity contains user input)
      expect(result.current.currentSuggestion?.text).toBe('Writing documentation');
    });

    it('should recognize partial matches (input contains activity)', async () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('Testing'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      rerender({
        activityContext: createMockContext('Testing'),
        inputValue: 'Writing unit tests',
        userHasTyped: true,
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });

      rerender({
        activityContext: createMockContext('Testing'),
        inputValue: 'Writing unit tests',
        userHasTyped: false,
      });

      // Should not override (user input contains activity)
      expect(result.current.currentSuggestion?.text).toBe('Testing');
    });

    it('should be case-insensitive', async () => {
      const { result, rerender } = renderHook(
        ({ activityContext, inputValue, userHasTyped }) =>
          useSuggestionManager({
            activityContext,
            inputValue,
            userHasTyped,
            isTracking: true,
          }),
        {
          initialProps: {
            activityContext: createMockContext('TESTING'),
            inputValue: '',
            userHasTyped: false,
          },
        }
      );

      rerender({
        activityContext: createMockContext('Testing Code'),
        inputValue: 'testing',
        userHasTyped: true,
      });

      await act(async () => {
        await vi.advanceTimersByTimeAsync(500);
      });

      rerender({
        activityContext: createMockContext('Testing Code'),
        inputValue: 'testing',
        userHasTyped: false,
      });

      // Should not override (case-insensitive match)
      expect(result.current.currentSuggestion?.text).toBe('TESTING');
    });
  });

  describe('Clear Suggestion', () => {
    it('should clear suggestion and prevent recreation when tracking stops', async () => {
      type TestProps = { isTracking: boolean; activityContext: ActivityContext | null };
      const { result, rerender } = renderHook(
        ({ isTracking, activityContext }: TestProps) =>
          useSuggestionManager({
            activityContext,
            inputValue: '',
            userHasTyped: false,
            isTracking,
          }),
        {
          initialProps: {
            isTracking: true,
            activityContext: createMockContext('Writing tests'),
          } as TestProps,
        }
      );

      // FIX-011 Issue #4: Wait for async suggestion creation (advance timers by 2 seconds for debounce)
      await act(async () => {
        await vi.advanceTimersByTimeAsync(2000);
      });

      // Now suggestion should be created
      expect(result.current.currentSuggestion).not.toBeNull();
      const suggestionBefore = result.current.currentSuggestion;

      // Clear the suggestion and stop tracking (simulates submitting an entry)
      act(() => {
        result.current.clearSuggestion();
      });

      // Rerender with tracking stopped and no activity context (real-world scenario after submission)
      rerender({
        isTracking: false,
        activityContext: null,
      });

      // Clear any pending timers
      vi.clearAllTimers();

      // Verify suggestion is cleared and stays cleared
      expect(result.current.currentSuggestion).toBeNull();
      expect(result.current.currentSuggestion).not.toBe(suggestionBefore);
    });
  });
});
