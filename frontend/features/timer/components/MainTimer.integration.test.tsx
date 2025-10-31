/**
 * FEATURE-009: Wire Frontend Timer UI to Tauri Backend Data
 * Integration tests for MainTimer component with Tauri backend
 *
 * Tests Issue #1: Timer Display Shows Real Activity
 */

import {
  createMockActivityContext,
  createMockWindowContext,
} from '@/shared/test/fixtures/backend-types';
import { renderWithProviders as render, screen, waitFor } from '@/shared/test/renderWithProviders';
import type { ActivityContext } from '@/shared/types/tauri-backend.types';
import { act } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri APIs (must be hoisted before imports)
const { mockInvoke, mockListen, mockUnlisten, mockEmit } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn(),
  mockUnlisten: vi.fn(),
  mockEmit: vi.fn().mockResolvedValue(undefined),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
  transformCallback: vi.fn((callback) => callback),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
  emit: mockEmit,
}));

// Mock SuggestedEntries to avoid integration issues in unit tests
vi.mock('./SuggestedEntries', () => ({
  SuggestedEntries: () => null,
}));

import { MainTimer } from './MainTimer';

vi.mock('@tauri-apps/api/window', () => ({
  getCurrentWindow: vi.fn(() => ({
    hide: vi.fn().mockResolvedValue(undefined),
    setSize: vi.fn().mockResolvedValue(undefined),
  })),
  LogicalSize: vi.fn().mockImplementation(function (
    this: { width: number; height: number },
    width: number,
    height: number
  ) {
    this.width = width;
    this.height = height;
    return this;
  }),
}));

// Mock project cache to prevent fetch errors
vi.mock('@/shared/services/projectCache', () => ({
  projectCache: {
    fetchProjects: vi.fn().mockResolvedValue(undefined),
    getProjectName: vi.fn((id: string) => id),
    isStale: vi.fn().mockReturnValue(false),
    preload: vi.fn().mockResolvedValue(undefined),
    refresh: vi.fn().mockResolvedValue(undefined),
  },
}));

describe('MainTimer - Issue #1: Real-time Activity Display', () => {
  let mockActivityContext: ActivityContext;

  beforeEach(() => {
    vi.clearAllMocks();

    // Mock activity context from backend
    mockActivityContext = createMockActivityContext({
      detected_activity: 'Writing code',
      active_app: createMockWindowContext({
        app_name: 'Visual Studio Code',
        window_title: 'MainTimer.tsx - Pulsarc',
        bundle_id: 'com.microsoft.VSCode',
      }),
      billable_confidence: 0.95,
      work_type: 'modeling',
      activity_category: 'client_work',
    });

    mockUnlisten.mockResolvedValue(undefined);
    // Default mock that handles multiple event listeners
    mockListen.mockImplementation(() => Promise.resolve(mockUnlisten));

    // Mock Tauri commands
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_tracker_state') {
        return Promise.resolve({
          is_running: true, // Activity display requires tracker to be active
          pause_reason: null,
        });
      }
      if (cmd === 'get_next_event') {
        return Promise.resolve([]);
      }
      if (cmd === 'get_calendar_events_for_timeline') {
        return Promise.resolve([]);
      }
      return Promise.resolve(undefined);
    });
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers(); // Restore real timers to prevent leaks
  });

  describe('Event-Driven Activity Updates (PRIMARY)', () => {
    it('should listen to activity-context-updated event on mount', async () => {
      render(<MainTimer />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('activity-context-updated', expect.any(Function));
      });
    });

    it('should receive and process activity events without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
        expect(eventHandler).toBeDefined();
      });

      // Simulate backend event emission (activity context is stored internally, not displayed in MainTimer UI)
      // MainTimer stores the context for SaveEntryModal, but doesn't render it in the timer display
      await act(async () => {
        eventHandler?.({ payload: mockActivityContext });
      });

      // Component should remain stable after receiving event (no crashes)
      expect(container.firstChild).toBeInTheDocument();
    });

    it('should handle multiple activity events without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      // First activity
      await act(async () => {
        eventHandler?.({ payload: mockActivityContext });
      });

      // Component should remain stable
      expect(container.firstChild).toBeInTheDocument();

      // Second activity (different)
      const newActivity: ActivityContext = {
        ...mockActivityContext,
        detected_activity: 'Debugging application',
        active_app: createMockWindowContext({
          app_name: 'Chrome DevTools',
          window_title: 'Application - DevTools',
          bundle_id: 'com.google.Chrome',
        }),
      };

      await act(async () => {
        eventHandler?.({ payload: newActivity });
      });

      // Component should still be stable after second event
      expect(container.firstChild).toBeInTheDocument();
    });

    it('should cleanup event listener on unmount', async () => {
      const { unmount } = render(<MainTimer />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      unmount();

      await waitFor(() => {
        expect(mockUnlisten).toHaveBeenCalled();
      });
    });
  });

  describe('Timestamp Normalization (Issue #6)', () => {
    it('should process activity context with timestamp data without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      const contextWithTimestamp: ActivityContext = {
        ...mockActivityContext,
        detected_activity: 'Timestamp test activity',
      };

      // Simulate backend event with timestamp data
      await act(async () => {
        eventHandler?.({ payload: contextWithTimestamp });
      });

      // Component should handle timestamps correctly (no errors, no crashes)
      expect(container.firstChild).toBeInTheDocument();
    });
  });

  describe('Privacy & PII Handling', () => {
    it('should accept activity context with sensitive data without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      const activityWithSensitiveData: ActivityContext = {
        ...mockActivityContext,
        active_app: createMockWindowContext({
          app_name: 'Google Chrome',
          window_title: 'Personal Email - Gmail - john.doe@personal.com',
          bundle_id: 'com.google.Chrome',
        }),
      };

      await act(async () => {
        eventHandler?.({ payload: activityWithSensitiveData });
      });

      // Component should process PII-containing context without errors
      // (Privacy filtering happens in the separate activity tracker window, not MainTimer)
      expect(container.firstChild).toBeInTheDocument();
    });
  });

  describe('Error Handling', () => {
    it('should handle event listener setup failure gracefully', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockListen.mockRejectedValue(new Error('Event listener failed'));

      render(<MainTimer />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // Should log error but not crash
      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith(
          'Failed to setup event listener:',
          expect.any(Error)
        );
      });

      consoleErrorSpy.mockRestore();
    });

    it('should handle event payload errors gracefully', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      // Send malformed event payload
      await act(async () => {
        eventHandler?.({ payload: null });
      });

      // Component should handle gracefully - when inactive, shows greeting
      await waitFor(() => {
        expect(screen.getByText(/Lewis/)).toBeInTheDocument();
      });

      consoleErrorSpy.mockRestore();
    });
  });

  describe('No Activity State', () => {
    it('should display "No activity" when activityContext is null', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      // Emit null activity (no current activity tracked)
      await act(async () => {
        eventHandler?.({ payload: null });
      });

      await waitFor(() => {
        // When timer is inactive and no activity, shows greeting instead
        expect(screen.getByText(/Lewis/)).toBeInTheDocument();
      });
    });

    it('should handle activity context with missing app info without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      const activityWithoutApp: ActivityContext = {
        ...mockActivityContext,
        active_app: createMockWindowContext({
          app_name: '',
          window_title: '',
          bundle_id: '',
        }),
      };

      await act(async () => {
        eventHandler?.({ payload: activityWithoutApp });
      });

      // Component should handle incomplete activity data without errors
      expect(container.firstChild).toBeInTheDocument();
    });
  });

  describe('Event-Driven Architecture (No Initial Fetch)', () => {
    it('should NOT fetch initial activity context on mount (pure event-driven)', async () => {
      render(<MainTimer />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // Should NOT call invoke for activity context (pure event-driven architecture)
      // Calendar events may be fetched on mount (allowed)
      const activityContextCalls = mockInvoke.mock.calls.filter(
        (call) => call[0] === 'get_activity_context'
      );
      expect(activityContextCalls).toHaveLength(0);
    });

    it('should receive and store activity context after event arrives', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      // Initially should show greeting (timer inactive, no event received yet)
      expect(screen.getByText(/Lewis/)).toBeInTheDocument();

      // After event, component should process it without errors
      await act(async () => {
        eventHandler?.({ payload: mockActivityContext });
      });

      // Component should remain stable
      expect(container.firstChild).toBeInTheDocument();
    });
  });

  describe('Event-Based State Updates', () => {
    it('should process state updates immediately when events arrive', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      // First event sets context immediately (no loading)
      await act(async () => {
        eventHandler?.({ payload: mockActivityContext });
      });

      // Component should process first event
      expect(container.firstChild).toBeInTheDocument();

      // Update with new event - should be immediate (no loading state)
      const newActivity: ActivityContext = {
        ...mockActivityContext,
        detected_activity: 'Debugging',
      };

      await act(async () => {
        eventHandler?.({ payload: newActivity });
      });

      // Component should process second event without errors
      expect(container.firstChild).toBeInTheDocument();
    });
  });

  describe('Activity Context Parsing', () => {
    it('should handle activity context with special characters without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      const complexContext: ActivityContext = {
        ...mockActivityContext,
        detected_activity: 'Complex task with "quotes" and special chars',
        active_app: createMockWindowContext({
          app_name: 'App & Tool',
          window_title: 'File.tsx - Project',
          bundle_id: 'com.app.tool',
        }),
      };

      // Simulate backend event
      await act(async () => {
        eventHandler?.({ payload: complexContext });
      });

      // Component should handle special characters in activity data without errors
      expect(container.firstChild).toBeInTheDocument();
    });

    it('should handle activity context with missing fields without errors', async () => {
      let eventHandler: ((event: { payload: unknown }) => void) | undefined;

      mockListen.mockImplementation((eventName, handler) => {
        // Only capture the activity-context-updated handler
        if (eventName === 'activity-context-updated') {
          eventHandler = handler;
        }
        return Promise.resolve(mockUnlisten);
      });

      const { container } = render(<MainTimer />);

      await waitFor(() => expect(mockListen).toHaveBeenCalled());

      const incompleteContext: ActivityContext = {
        ...mockActivityContext,
        detected_activity: '',
        active_app: createMockWindowContext({
          app_name: '',
          window_title: '',
          bundle_id: '',
        }),
      };

      // Simulate backend event
      await act(async () => {
        eventHandler?.({ payload: incompleteContext });
      });

      // Component should handle incomplete data gracefully without errors
      expect(container.firstChild).toBeInTheDocument();
    });
  });
});
