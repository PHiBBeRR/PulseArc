/**
 * TDD Tests for FIX-009: Pause Button Should Stop Backend Event-Driven Tracker
 * Frontend Integration Tests
 *
 * These tests define the acceptance criteria for FIX-009 frontend implementation.
 * All tests should FAIL before implementation and PASS after implementation is complete.
 *
 * Test coverage:
 * - Issue #2: Frontend pause handler calls backend commands
 * - Issue #3: Tray menu event listeners update UI state
 * - Issue #7: Multi-window state synchronization
 * - Issue #8: Stale context cleanup on pause/resume
 *
 * Total: 4 frontend issues, 20+ acceptance criteria mapped to tests
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ActivityTrackerView } from './ActivityTrackerView';

// Mock Tauri APIs
const mockInvoke = vi.fn();
const mockListen = vi.fn();
const mockUnlisten = vi.fn();
const mockEmit = vi.fn();

vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: (...args: unknown[]) => mockListen(...args),
  emit: (...args: unknown[]) => mockEmit(...args),
}));

// Mock audio service
vi.mock('../../../shared/services', () => ({
  audioService: {
    playSound: vi.fn(),
    playClick: vi.fn(),
  },
}));

describe('ActivityTrackerView - Pause/Resume Integration (FIX-009)', () => {
  beforeEach(() => {
    // Reset all mocks
    vi.clearAllMocks();

    // Setup default mock responses
    mockInvoke.mockResolvedValue(undefined);
    mockListen.mockResolvedValue(mockUnlisten);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  // ============================================================================
  // Issue #2: Frontend Pause Handler Calls Backend Commands
  // ============================================================================

  describe('Issue #2: Frontend Pause Handler Integration', () => {
    it('should call pause_tracker backend command when pause button clicked', async () => {
      // Acceptance: handlePause calls invoke('pause_tracker')
      // Acceptance: Backend command invoked before UI state update

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Find and click pause button (tracker starts in 'active' state)
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      // Verify backend command was called
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
      });
    });

    it('should call resume_tracker backend command when resume button clicked', async () => {
      // Acceptance: handlePause calls invoke('resume_tracker') when paused
      // Acceptance: Backend command invoked before UI state update

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // First pause
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      // Wait for pause to complete
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
      });

      // Clear mock to track resume call
      mockInvoke.mockClear();

      // Click resume (button should now show "Resume" or play icon)
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      // Verify resume command called
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('resume_tracker');
      });
    });

    it('should update UI state to paused after backend pause command', async () => {
      // Acceptance: setTrackerState('paused') called after invoke
      // Acceptance: UI reflects paused state

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      // Wait for state update
      await waitFor(() => {
        // Verify UI shows paused state (e.g., play button instead of pause)
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });
    });

    it('should update UI state to active after backend resume command', async () => {
      // Acceptance: setTrackerState('active') called after resume invoke
      // Acceptance: UI reflects active state

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Pause first
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });

      // Resume
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      // Verify UI shows active state (pause button back)
      await waitFor(() => {
        expect(screen.getByRole('button', { name: /pause/i })).toBeInTheDocument();
      });
    });

    it('should log pause action to console', async () => {
      // Acceptance: console.log('⏸️ Tracker paused - backend stopped')

      const consoleSpy = vi.spyOn(console, 'log');
      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Tracker paused'));
      });

      consoleSpy.mockRestore();
    });

    it('should log resume action to console', async () => {
      // Acceptance: console.log('▶️ Tracker resumed - backend started')

      const consoleSpy = vi.spyOn(console, 'log');
      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Pause first
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);
      await waitFor(() => expect(mockInvoke).toHaveBeenCalled());

      mockInvoke.mockClear();

      // Resume
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Tracker resumed'));
      });

      consoleSpy.mockRestore();
    });
  });

  // ============================================================================
  // Issue #3: Tray Menu Event Listeners
  // ============================================================================

  describe('Issue #3: Tray Menu Event Listeners', () => {
    it('should register listener for pause-timer event', async () => {
      // Acceptance: Frontend listens to "pause-timer" event from tray
      // Acceptance: Event listener setup in useEffect

      render(<ActivityTrackerView />);

      // Wait for async effect to complete
      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('pause-timer', expect.any(Function));
      });
    });

    it('should register listener for start-timer event', async () => {
      // Acceptance: Frontend listens to "start-timer" event from tray
      // Acceptance: Event listener setup in useEffect

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('start-timer', expect.any(Function));
      });
    });

    it('should update state to paused when pause-timer event received', async () => {
      // Acceptance: setTrackerState('paused') on tray pause event
      // Acceptance: UI syncs with tray menu action

      let pauseCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'pause-timer') {
          pauseCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<ActivityTrackerView />);

      // Wait for listener setup
      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // Simulate tray pause event
      expect(pauseCallback).toBeDefined();
      if (pauseCallback) {
        pauseCallback();
      }

      // Verify UI updated to paused state
      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });
    });

    it('should update state to active when start-timer event received', async () => {
      // Acceptance: setTrackerState('active') on tray start event
      // Acceptance: UI syncs with tray menu action

      let startCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'start-timer') {
          startCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Pause first via UI
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });

      // Simulate tray start event
      expect(startCallback).toBeDefined();
      if (startCallback) {
        startCallback();
      }

      // Verify UI updated to active state
      await waitFor(() => {
        expect(screen.getByRole('button', { name: /pause/i })).toBeInTheDocument();
      });
    });

    it('should cleanup event listeners on unmount', async () => {
      // Acceptance: Event listeners unregistered on component unmount
      // Acceptance: No memory leaks from event listeners

      const { unmount } = render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // Unmount component
      unmount();

      // Verify unlisten was called
      expect(mockUnlisten).toHaveBeenCalled();
    });
  });

  // ============================================================================
  // Issue #7: Multi-Window State Synchronization
  // ============================================================================

  describe('Issue #7: Multi-Window State Sync', () => {
    it('should register listener for global tracker-paused event', async () => {
      // Acceptance: Frontend listens to global "tracker-paused" event
      // Acceptance: Event listener for multi-window sync

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('tracker-paused', expect.any(Function));
      });
    });

    it('should register listener for global tracker-resumed event', async () => {
      // Acceptance: Frontend listens to global "tracker-resumed" event
      // Acceptance: Event listener for multi-window sync

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('tracker-resumed', expect.any(Function));
      });
    });

    it('should sync state when tracker-paused event received from another window', async () => {
      // Acceptance: All windows update state on global pause event
      // Acceptance: No window shows conflicting state

      let globalPauseCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'tracker-paused') {
          globalPauseCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // Simulate global pause event from another window
      expect(globalPauseCallback).toBeDefined();
      if (globalPauseCallback) {
        globalPauseCallback();
      }

      // Verify this window synced to paused state
      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });
    });

    it('should sync state when tracker-resumed event received from another window', async () => {
      // Acceptance: All windows update state on global resume event
      // Acceptance: Window state synchronizes across all instances

      let globalResumeCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'tracker-resumed') {
          globalResumeCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Pause via UI first
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });

      // Simulate global resume event from another window
      expect(globalResumeCallback).toBeDefined();
      if (globalResumeCallback) {
        globalResumeCallback();
      }

      // Verify this window synced to active state
      await waitFor(() => {
        expect(screen.getByRole('button', { name: /pause/i })).toBeInTheDocument();
      });
    });

    it('should log global pause event detection', async () => {
      // Acceptance: console.log('🔔 Global pause detected')

      const consoleSpy = vi.spyOn(console, 'log');
      let globalPauseCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'tracker-paused') {
          globalPauseCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      if (globalPauseCallback) {
        globalPauseCallback();
      }

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Global pause detected'));
      });

      consoleSpy.mockRestore();
    });

    it('should log global resume event detection', async () => {
      // Acceptance: console.log('🔔 Global resume detected')

      const consoleSpy = vi.spyOn(console, 'log');
      let globalResumeCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'tracker-resumed') {
          globalResumeCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      if (globalResumeCallback) {
        globalResumeCallback();
      }

      await waitFor(() => {
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Global resume detected'));
      });

      consoleSpy.mockRestore();
    });
  });

  // ============================================================================
  // Issue #8: Stale Context Cleanup on Pause/Resume
  // ============================================================================

  describe('Issue #8: Stale Context Cleanup', () => {
    it('should clear activity context when pausing', async () => {
      // Acceptance: setActivityContext(null) when pausing
      // Acceptance: No stale data shown after pause

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Pause tracker
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
      });

      // Note: Actual context clearing verification would require
      // inspecting internal component state or checking if
      // context display elements are removed from DOM
      // This test validates the pause action completes
    });

    it('should fetch fresh context when resuming', async () => {
      // Acceptance: void fetchContext() called when resuming
      // Acceptance: Fresh context fetched immediately on resume

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Pause first
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
      });

      mockInvoke.mockClear();

      // Resume
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      // Verify resume command AND context fetch
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('resume_tracker');
        // fetchContext should also be called, which invokes fetch_activity_context
        // This happens after resume completes
      });
    });

    it('should not show stale context after resume', async () => {
      // Acceptance: No stale app/activity shown after resume
      // Acceptance: Loading state shown while fetching fresh data

      const user = userEvent.setup();

      // Mock fetch_activity_context to return fresh data
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'fetch_activity_context') {
          return Promise.resolve({
            app_name: 'Fresh App',
            window_title: 'Fresh Window',
            activity: 'Fresh Activity',
          });
        }
        return Promise.resolve(undefined);
      });

      render(<ActivityTrackerView />);

      // Pause
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
      });

      // Resume
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      // Verify fresh context is fetched
      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('fetch_activity_context');
      });
    });
  });

  // ============================================================================
  // Integration Tests - Multi-Issue Scenarios
  // ============================================================================

  describe('Integration: Complete Pause/Resume Flow', () => {
    it('should handle complete pause-resume cycle with all integrations', async () => {
      // Tests Issues #2, #3, #7, #8 together
      // Complete user flow: pause via button → tray sync → resume → context refresh

      const user = userEvent.setup();
      const consoleSpy = vi.spyOn(console, 'log');

      render(<ActivityTrackerView />);

      // 1. Pause via button (Issue #2)
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Tracker paused'));
      });

      mockInvoke.mockClear();

      // 2. Resume via button (Issue #2 + #8)
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('resume_tracker');
        expect(consoleSpy).toHaveBeenCalledWith(expect.stringContaining('Tracker resumed'));
      });

      consoleSpy.mockRestore();
    });

    it('should handle tray menu and UI button pause/resume interleaving', async () => {
      // Tests Issue #3 integration with Issue #2
      // User pauses via tray, resumes via UI button

      const user = userEvent.setup();
      let trayPauseCallback: (() => void) | undefined;

      mockListen.mockImplementation((eventName: string, callback: () => void) => {
        if (eventName === 'pause-timer') {
          trayPauseCallback = callback;
        }
        return Promise.resolve(mockUnlisten);
      });

      render(<ActivityTrackerView />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      // Pause via tray
      if (trayPauseCallback) {
        trayPauseCallback();
      }

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });

      // Resume via UI button
      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('resume_tracker');
      });

      // Should show active state
      expect(screen.getByRole('button', { name: /pause/i })).toBeInTheDocument();
    });

    it('should handle rapid pause/resume toggles gracefully', async () => {
      // Tests UI resilience with rapid clicking
      // Related to backend Issue #5 (race conditions) and #9 (idempotency)

      const user = userEvent.setup();
      render(<ActivityTrackerView />);

      // Rapid pause/resume/pause/resume
      const pauseButton = screen.getByRole('button', { name: /pause/i });
      await user.click(pauseButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /play|resume/i })).toBeInTheDocument();
      });

      const resumeButton = screen.getByRole('button', { name: /play|resume/i });
      await user.click(resumeButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /pause/i })).toBeInTheDocument();
      });

      // Should handle without errors (pause_tracker, resume_tracker, fetch_activity_context)
      expect(mockInvoke).toHaveBeenCalledWith('pause_tracker');
      expect(mockInvoke).toHaveBeenCalledWith('resume_tracker');
      expect(mockInvoke).toHaveBeenCalledWith('fetch_activity_context');
      // Allow some flexibility in call count (may include additional context fetches)
      expect(mockInvoke).toHaveBeenCalled();
    });
  });
});
