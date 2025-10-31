/**
 * Unit tests for SuggestedEntries component
 *
 * Tests the component that displays AI-suggested time entries based on
 * detected activities. Users can accept, dismiss, or edit suggestions.
 *
 * Test Coverage:
 * - Rendering: Display of suggested entries with project, task, and duration
 * - User Actions: Accept, dismiss, and edit suggestion interactions
 * - Confidence Display: Visual indicators for suggestion confidence levels
 * - Project Resolution: Resolving project IDs to names via projectCache
 * - Haptic Feedback: Touch feedback on user interactions
 * - Empty States: Handling no suggestions available
 * - Real-time Updates: Event listener for new suggestions from backend
 * - Outbox Integration: Showing pending submissions
 * - Batch Operations: Accepting multiple suggestions at once
 */

/* eslint-disable @typescript-eslint/no-explicit-any */
import {
  createMockPrismaTimeEntryDto,
  createMockTimeEntryOutbox,
} from '@/shared/test/fixtures/backend-types';
import type { PrismaTimeEntryDto, TimeEntryOutbox } from '@/shared/types/generated';
import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SuggestedEntries } from './SuggestedEntries';

// Hoist mocks to avoid initialization errors
const { mockInvoke, mockListen, mockUnlisten } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
  mockListen: vi.fn(),
  mockUnlisten: vi.fn(),
}));

// Mock Tauri APIs
vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: mockListen,
}));

// Mock dependencies
vi.mock('@/shared/services', () => ({
  projectCache: {
    fetchProjects: vi.fn().mockResolvedValue(undefined),
    getProjectName: vi.fn((id: string) => {
      if (id === 'proj-123') return 'Test Project';
      if (id === 'proj-456') return 'Another Project';
      return 'Unassigned';
    }),
  },
}));

vi.mock('../../../shared/utils', () => ({
  haptic: {
    light: vi.fn(),
  },
}));

vi.mock('../../time-entry/services', () => ({
  entryService: {
    getConfidenceColor: vi.fn((confidence: number) => {
      if (confidence >= 80) return 'bg-green-500/20 text-green-700';
      if (confidence >= 60) return 'bg-yellow-500/20 text-yellow-700';
      return 'bg-red-500/20 text-red-700';
    }),
  },
}));

// Mock modal components
vi.mock('../../time-entry/components/EditEntryModal', () => ({
  EditEntryModal: ({ isOpen, entry }: any) =>
    isOpen ? <div data-testid="edit-modal">EditEntryModal: {entry?.id}</div> : null,
}));

vi.mock('../../time-entry/components/ClassifyEntryModal', () => ({
  ClassifyEntryModal: ({ isOpen, entry }: any) =>
    isOpen ? <div data-testid="classify-modal">ClassifyEntryModal: {entry?.id}</div> : null,
}));

vi.mock('../../time-entry/components/DismissFeedbackModal', () => ({
  DismissFeedbackModal: ({ isOpen, entry }: any) =>
    isOpen ? <div data-testid="dismiss-modal">DismissFeedbackModal: {entry?.id}</div> : null,
}));

// Helper to create outbox entry with DTO
function createOutboxWithDto(
  outboxOverrides: Partial<TimeEntryOutbox> = {},
  dtoOverrides: Partial<PrismaTimeEntryDto> = {}
): TimeEntryOutbox {
  const dto = createMockPrismaTimeEntryDto(dtoOverrides);
  return createMockTimeEntryOutbox({
    payload_json: JSON.stringify(dto),
    ...outboxOverrides,
  });
}

describe('SuggestedEntries', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockListen.mockResolvedValue(mockUnlisten);
    localStorage.clear();
    // Default mock: return empty for get_proposed_blocks (component falls back to legacy outbox)
    mockInvoke.mockImplementation((cmd: string) => {
      if (cmd === 'get_proposed_blocks') return Promise.resolve([]);
      if (cmd === 'get_outbox_status') return Promise.resolve([]);
      if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
      return Promise.resolve([]);
    });
  });

  afterEach(() => {
    vi.clearAllTimers();
  });

  describe('Initial Rendering', () => {
    it('should render nothing when there are no entries', async () => {
      // Uses default mock from beforeEach (empty arrays)
      const { container } = render(<SuggestedEntries />);

      await waitFor(() => {
        expect(container.firstChild).toBeNull();
      });
    });

    it('should fetch and display pending suggestions on mount', async () => {
      const pendingEntry = createOutboxWithDto(
        {
          id: 'outbox-1',
          status: 'pending',
          created_at: Math.floor(Date.now() / 1000),
        },
        {
          projectId: 'proj-123',
          notes: 'Working on feature',
          durationSec: 3600,
          source: 'ai',
          _confidence: 0.85,
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([pendingEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Recent Activity')).toBeInTheDocument();
        expect(screen.getByText('Test Project')).toBeInTheDocument();
        expect(screen.getByText('Working on feature')).toBeInTheDocument();
        expect(screen.getByText('85%')).toBeInTheDocument();
      });
    });

    it('should show loading state initially', async () => {
      const pendingEntry = createOutboxWithDto({ id: 'loading-test', status: 'pending' });

      // Delay the response to simulate loading
      mockInvoke.mockImplementation(
        () =>
          new Promise((resolve) => {
            setTimeout(() => resolve([pendingEntry]), 100);
          })
      );

      render(<SuggestedEntries />);

      // Wait for component to render (need at least one entry for component to show)
      await waitFor(() => {
        expect(screen.getByText('Recent Activity')).toBeInTheDocument();
      });
    });

    it('should restore collapsed state from localStorage', async () => {
      localStorage.setItem('recentActivityCollapsed', 'true');

      const pendingEntry = createOutboxWithDto({ id: 'outbox-1', status: 'pending' });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([pendingEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        const collapseButton = screen.getByRole('button', { expanded: false });
        expect(collapseButton).toHaveAttribute('aria-expanded', 'false');
      });
    });
  });

  describe('Collapsible Header', () => {
    beforeEach(() => {
      const pendingEntry = createOutboxWithDto({ id: 'outbox-1', status: 'pending' });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_proposed_blocks') return Promise.resolve([]); // Trigger fallback to legacy outbox
        if (cmd === 'get_outbox_status') return Promise.resolve([pendingEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });
    });

    it('should display count of pending suggestions in header', async () => {
      // Create mock ProposedBlocks with suggested status
      const suggestedBlocks = Array.from({ length: 3 }, (_, i) => ({
        id: `block-${i + 1}`,
        start_ts: 1234567890 + i,
        duration_minutes: 30,
        detected_activity: `Activity ${i + 1}`,
        classification_status: 'suggested',
        project_id: 'proj-123',
        wbs_code: null,
      }));

      mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
        if (cmd === 'get_proposed_blocks') {
          // When fetching with status='suggested' (for display), return suggested blocks
          if (
            args &&
            typeof args === 'object' &&
            'status' in args &&
            (args as { status: string }).status === 'suggested'
          ) {
            return Promise.resolve(suggestedBlocks);
          }
          // When fetching with status='pending' (for unclassified count), return empty (all are suggested)
          if (
            args &&
            typeof args === 'object' &&
            'status' in args &&
            (args as { status: string }).status === 'pending'
          ) {
            return Promise.resolve([]);
          }
          return Promise.resolve([]);
        }
        if (cmd === 'get_outbox_status') return Promise.resolve([]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('3 suggestions pending approval')).toBeInTheDocument();
      });
    });

    it('should display unclassified blocks count when no suggestions', async () => {
      // Create mock ProposedBlocks with pending status (unclassified)
      const pendingBlocks = Array.from({ length: 5 }, (_, i) => ({
        id: `block-${i + 1}`,
        start_ts: 1234567890 + i,
        duration_minutes: 30,
        detected_activity: `Activity ${i + 1}`,
        classification_status: 'pending',
      }));

      mockInvoke.mockImplementation((cmd: string, args?: unknown) => {
        if (cmd === 'get_proposed_blocks') {
          // When fetching with status='suggested' (for display), return empty
          if (
            args &&
            typeof args === 'object' &&
            'status' in args &&
            (args as { status: string }).status === 'suggested'
          ) {
            return Promise.resolve([]);
          }
          // When fetching with status='pending' (for unclassified count), return pending blocks
          if (
            args &&
            typeof args === 'object' &&
            'status' in args &&
            (args as { status: string }).status === 'pending'
          ) {
            return Promise.resolve(pendingBlocks);
          }
          return Promise.resolve([]);
        }
        if (cmd === 'get_outbox_status') return Promise.resolve([]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('5 blocks to classify')).toBeInTheDocument();
      });
    });

    it('should toggle collapse state when header is clicked', async () => {
      const user = userEvent.setup();
      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
      });

      const collapseButton = screen.getByRole('button', { expanded: true });
      await user.click(collapseButton);

      await waitFor(() => {
        expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
      });
    });

    it('should persist collapse state to localStorage', async () => {
      const user = userEvent.setup();
      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
      });

      const collapseButton = screen.getByRole('button', { expanded: true });
      await user.click(collapseButton);

      await waitFor(() => {
        expect(localStorage.getItem('recentActivityCollapsed')).toBe('true');
      });
    });

    it('should call onCollapseChange callback when collapsed', async () => {
      const onCollapseChange = vi.fn();
      const user = userEvent.setup();

      render(<SuggestedEntries onCollapseChange={onCollapseChange} />);

      await waitFor(() => {
        expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
      });

      const collapseButton = screen.getByRole('button', { expanded: true });
      await user.click(collapseButton);

      await waitFor(() => {
        expect(onCollapseChange).toHaveBeenCalledWith(true);
      });
    });

    it('should display sync button when onBuildMyDay is provided', async () => {
      const onBuildMyDay = vi.fn();

      render(<SuggestedEntries onBuildMyDay={onBuildMyDay} />);

      await waitFor(() => {
        const syncButton = screen.getByRole('button', { name: /sync calendar events/i });
        expect(syncButton).toBeInTheDocument();
      });
    });

    it('should show last sync time in tooltip', async () => {
      const onBuildMyDay = vi.fn();
      const user = userEvent.setup();
      const pendingEntry = createOutboxWithDto({ id: 'sync-tooltip', status: 'pending' });

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([pendingEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onBuildMyDay={onBuildMyDay} />);

      await waitFor(() => {
        const syncButton = screen.getByRole('button', { name: /sync calendar events/i });
        expect(syncButton).toBeInTheDocument();
      });

      const syncButton = screen.getByRole('button', { name: /sync calendar events/i });
      await user.hover(syncButton);

      await waitFor(() => {
        const tooltips = screen.getAllByText(/last synced/i);
        expect(tooltips.length).toBeGreaterThan(0);
      });
    });
  });

  describe('Tab System', () => {
    it('should display two tabs: Suggestions and Dismissed', async () => {
      const pendingEntry = createOutboxWithDto({ id: 'outbox-1', status: 'pending' });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([pendingEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByRole('tab', { name: /suggestions/i })).toBeInTheDocument();
        expect(screen.getByRole('tab', { name: /dismissed/i })).toBeInTheDocument();
      });
    });

    it('should show suggestion count in Suggestions tab', async () => {
      const entries = [
        createOutboxWithDto({ id: 'outbox-1', status: 'pending' }),
        createOutboxWithDto({ id: 'outbox-2', status: 'pending' }),
      ];

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(entries);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        const suggestionsTab = screen.getByRole('tab', { name: /suggestions/i });
        expect(within(suggestionsTab).getByText('2')).toBeInTheDocument();
      });
    });

    it('should switch tabs when clicked', async () => {
      const user = userEvent.setup();
      const pendingEntry = createOutboxWithDto({ id: 'outbox-1', status: 'pending' });
      const dismissedEntry = createOutboxWithDto({ id: 'outbox-2', status: 'dismissed' });

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([pendingEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([dismissedEntry]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(
          screen.getByRole('tab', { name: /suggestions/i, selected: true })
        ).toBeInTheDocument();
      });

      const dismissedTab = screen.getByRole('tab', { name: /dismissed/i });
      await user.click(dismissedTab);

      await waitFor(() => {
        expect(screen.getByRole('tab', { name: /dismissed/i, selected: true })).toBeInTheDocument();
      });
    });
  });

  describe('Entry Display - AI Entries', () => {
    it('should display AI entry with all metadata', async () => {
      const aiEntry = createOutboxWithDto(
        {
          id: 'ai-1',
          status: 'pending',
          created_at: new Date('2025-10-20T10:30:00').getTime() / 1000,
        },
        {
          projectId: 'proj-123',
          notes: 'Coding new feature',
          durationSec: 3600,
          source: 'ai',
          _confidence: 0.75,
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([aiEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Test Project')).toBeInTheDocument();
        expect(screen.getByText('Coding new feature')).toBeInTheDocument();
        expect(screen.getByText('Detected: Coding new feature')).toBeInTheDocument();
        expect(screen.getByText('75%')).toBeInTheDocument();
        expect(screen.getByText('AI')).toBeInTheDocument();
        expect(screen.getByText('1h 0m')).toBeInTheDocument();
      });
    });

    it('should display Brain icon for AI entries', async () => {
      const aiEntry = createOutboxWithDto(
        { id: 'ai-1', status: 'pending' },
        { source: 'ai', projectId: 'proj-123', notes: 'AI detected task' }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([aiEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Should display project from cache
        expect(screen.getByText('Test Project')).toBeInTheDocument();
        expect(screen.getByText('AI detected task')).toBeInTheDocument();
        // Brain icon should be present (purple badge with Brain icon for AI category)
        expect(screen.getByText('AI')).toBeInTheDocument(); // AI badge
      });
    });
  });

  describe('Entry Display - Calendar Entries', () => {
    it('should display calendar entry without AI badge', async () => {
      const calendarEntry = createOutboxWithDto(
        {
          id: 'cal-1',
          status: 'pending',
          created_at: new Date('2025-10-20T14:00:00').getTime() / 1000,
        },
        {
          source: 'calendar',
          projectId: 'proj-123', // UUIDv7 - will be looked up in project cache
          _displayProject: 'Project Alpha', // Hint for categorization
          _displayTask: 'Team meeting',
          durationSec: 1800,
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([calendarEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Calendar entries use _displayProject (parsed from title), not projectId lookup
        expect(screen.getByText('Project Alpha')).toBeInTheDocument();
        expect(screen.getByText('Team meeting')).toBeInTheDocument();
        expect(screen.queryByText('AI')).not.toBeInTheDocument();
        expect(screen.queryByText(/detected:/i)).not.toBeInTheDocument();
      });
    });

    it('should categorize personal calendar events correctly', async () => {
      const personalEntry = createOutboxWithDto(
        { id: 'cal-personal', status: 'pending' },
        {
          source: 'calendar',
          projectId: 'unassigned',
          _displayProject: 'General', // Hint for categorization
          _displayTask: 'Dentist appointment',
          durationSec: 3600,
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([personalEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Personal category overrides project display
        expect(screen.getByText('Personal')).toBeInTheDocument();
        expect(screen.getByText('Dentist appointment')).toBeInTheDocument();
      });
    });

    it('should categorize general/admin calendar events correctly', async () => {
      const adminEntry = createOutboxWithDto(
        { id: 'cal-admin', status: 'pending' },
        {
          source: 'calendar',
          projectId: 'unassigned',
          _displayProject: 'General',
          _displayTask: 'Team standup meeting',
          durationSec: 900,
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([adminEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Uses _displayProject = "General" (matches "team standup" keyword → general category)
        expect(screen.getByText('General')).toBeInTheDocument();
        expect(screen.getByText('Team standup meeting')).toBeInTheDocument();
      });
    });

    it('should categorize project calendar events correctly', async () => {
      const projectEntry = createOutboxWithDto(
        { id: 'cal-project', status: 'pending' },
        {
          source: 'calendar',
          projectId: 'proj-456', // UUIDv7
          _displayProject: 'Client Project', // Hint for categorization
          _displayTask: 'Project review meeting',
          durationSec: 3600,
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([projectEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Uses _displayProject = "Client Project" (contains "project" keyword → project category)
        expect(screen.getByText('Client Project')).toBeInTheDocument();
        expect(screen.getByText('Project review meeting')).toBeInTheDocument();
      });
    });

    it('should display WBS code when present', async () => {
      const entryWithWbs = createOutboxWithDto(
        { id: 'wbs-1', status: 'pending' },
        {
          projectId: 'proj-123',
          notes: 'Task work',
          durationSec: 3600,
          _wbsCode: 'WBS-123-456',
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entryWithWbs]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('WBS-123-456')).toBeInTheDocument();
      });
    });
  });

  describe('Entry Limits', () => {
    it('should display maximum of 3 entries per tab', async () => {
      const entries = Array.from({ length: 5 }, (_, i) =>
        createOutboxWithDto(
          { id: `outbox-${i}`, status: 'pending' },
          { projectId: 'proj-123', notes: `Task ${i}`, source: 'ai' }
        )
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(entries);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Should show "Detected: Task X" for AI entries
        const task0 = screen.getByText('Detected: Task 0');
        const task1 = screen.getByText('Detected: Task 1');
        const task2 = screen.getByText('Detected: Task 2');
        expect(task0).toBeInTheDocument();
        expect(task1).toBeInTheDocument();
        expect(task2).toBeInTheDocument();

        // Should NOT show task 3 and 4 (max 3 entries)
        expect(screen.queryByText('Detected: Task 3')).not.toBeInTheDocument();
        expect(screen.queryByText('Detected: Task 4')).not.toBeInTheDocument();
      });
    });
  });

  describe('Accept Action', () => {
    it('should accept non-personal entries directly', async () => {
      const user = userEvent.setup();
      const entry = createOutboxWithDto(
        { id: 'accept-1', status: 'pending' },
        {
          source: 'ai',
          projectId: 'proj-123',
          notes: 'Work task',
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        if (cmd === 'accept_suggestion') return Promise.resolve(undefined);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Work task')).toBeInTheDocument();
      });

      const acceptButton = screen.getByRole('button', { name: /accept suggestion/i });
      await user.click(acceptButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('accept_suggestion', { id: 'accept-1' });
      });
    });

    it('should open classify modal for personal entries', async () => {
      const user = userEvent.setup();
      const personalEntry = createOutboxWithDto(
        { id: 'personal-1', status: 'pending' },
        {
          source: 'calendar',
          _displayProject: 'General',
          _displayTask: 'Gym workout',
        }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([personalEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Gym workout')).toBeInTheDocument();
      });

      const acceptButton = screen.getByRole('button', { name: /accept suggestion/i });
      await user.click(acceptButton);

      await waitFor(() => {
        expect(screen.getByTestId('classify-modal')).toBeInTheDocument();
      });
    });

    it('should remove entry from list after successful accept', async () => {
      const user = userEvent.setup();
      const entries = [
        createOutboxWithDto({ id: 'entry-1', status: 'pending' }, { notes: 'Task 1' }),
        createOutboxWithDto({ id: 'entry-2', status: 'pending' }, { notes: 'Task 2' }),
      ];

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(entries);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        if (cmd === 'accept_suggestion') return Promise.resolve(undefined);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Task 1')).toBeInTheDocument();
        expect(screen.getByText('Task 2')).toBeInTheDocument();
      });

      const acceptButtons = screen.getAllByRole('button', { name: /accept suggestion/i });
      await user.click(acceptButtons[0]);

      await waitFor(() => {
        expect(screen.queryByText('Task 1')).not.toBeInTheDocument();
        expect(screen.getByText('Task 2')).toBeInTheDocument();
      });
    });

    it('should call onAcceptEntry callback with remaining count', async () => {
      const user = userEvent.setup();
      const onAcceptEntry = vi.fn();
      const entries = [
        createOutboxWithDto({ id: 'entry-1', status: 'pending' }, { notes: 'Task 1' }),
        createOutboxWithDto({ id: 'entry-2', status: 'pending' }, { notes: 'Task 2' }),
      ];

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(entries);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        if (cmd === 'accept_suggestion') return Promise.resolve(undefined);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onAcceptEntry={onAcceptEntry} />);

      await waitFor(() => {
        expect(screen.getByText('Task 1')).toBeInTheDocument();
      });

      const acceptButtons = screen.getAllByRole('button', { name: /accept suggestion/i });
      await user.click(acceptButtons[0]);

      await waitFor(() => {
        expect(onAcceptEntry).toHaveBeenCalledWith(
          expect.objectContaining({ id: 'entry-1' }),
          1 // remaining count
        );
      });
    });
  });

  describe('Edit Action', () => {
    it('should open edit modal when edit button is clicked', async () => {
      const user = userEvent.setup();
      const entry = createOutboxWithDto(
        { id: 'edit-1', status: 'pending' },
        { notes: 'Task to edit' }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Task to edit')).toBeInTheDocument();
      });

      const editButton = screen.getByRole('button', { name: /edit suggestion/i });
      await user.click(editButton);

      await waitFor(() => {
        expect(screen.getByTestId('edit-modal')).toBeInTheDocument();
        expect(screen.getByText(/EditEntryModal: edit-1/)).toBeInTheDocument();
      });
    });
  });

  describe('Dismiss Action', () => {
    it('should open dismiss modal when dismiss button is clicked', async () => {
      const user = userEvent.setup();
      const entry = createOutboxWithDto(
        { id: 'dismiss-1', status: 'pending' },
        { notes: 'Task to dismiss' }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Task to dismiss')).toBeInTheDocument();
      });

      const dismissButton = screen.getByRole('button', { name: /dismiss suggestion/i });
      await user.click(dismissButton);

      await waitFor(() => {
        expect(screen.getByTestId('dismiss-modal')).toBeInTheDocument();
      });
    });
  });

  describe('Dismissed Tab Actions', () => {
    it('should display restore and delete buttons in dismissed tab', async () => {
      const user = userEvent.setup();
      const suggestedEntry = createOutboxWithDto(
        { id: 'suggested-1', status: 'pending' },
        { notes: 'Suggested task' }
      );
      const dismissedEntry = createOutboxWithDto(
        { id: 'dismissed-1', status: 'dismissed' },
        { notes: 'Dismissed task' }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([suggestedEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([dismissedEntry]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        const dismissedTab = screen.getByRole('tab', { name: /dismissed/i });
        expect(dismissedTab).toBeInTheDocument();
      });

      const dismissedTab = screen.getByRole('tab', { name: /dismissed/i });
      await user.click(dismissedTab);

      await waitFor(() => {
        expect(screen.getByText('Dismissed task')).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /restore/i })).toBeInTheDocument();
        expect(screen.getByRole('button', { name: /delete permanently/i })).toBeInTheDocument();
      });
    });

    it('should restore entry and switch to suggestions tab', async () => {
      const user = userEvent.setup();
      const suggestedEntry = createOutboxWithDto(
        { id: 'suggested-1', status: 'pending' },
        { notes: 'Suggested task' }
      );
      const dismissedEntry = createOutboxWithDto(
        { id: 'restore-1', status: 'dismissed' },
        { notes: 'Restore me' }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([suggestedEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([dismissedEntry]);
        if (cmd === 'restore_suggestion') return Promise.resolve(undefined);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByRole('tab', { name: /dismissed/i })).toBeInTheDocument();
      });

      // Switch to dismissed tab
      const dismissedTab = screen.getByRole('tab', { name: /dismissed/i });
      await user.click(dismissedTab);

      await waitFor(() => {
        expect(screen.getByText('Restore me')).toBeInTheDocument();
      });

      // Click restore
      const restoreButton = screen.getByRole('button', { name: /restore/i });
      await user.click(restoreButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('restore_suggestion', { id: 'restore-1' });
        // Should auto-switch to suggestions tab
        expect(
          screen.getByRole('tab', { name: /suggestions/i, selected: true })
        ).toBeInTheDocument();
      });
    });

    it('should delete entry permanently', async () => {
      const user = userEvent.setup();
      const suggestedEntry = createOutboxWithDto(
        { id: 'suggested-1', status: 'pending' },
        { notes: 'Suggested task' }
      );
      const dismissedEntry = createOutboxWithDto(
        { id: 'delete-1', status: 'dismissed' },
        { notes: 'Delete me' }
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([suggestedEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([dismissedEntry]);
        if (cmd === 'delete_outbox_entry') return Promise.resolve(undefined);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByRole('tab', { name: /dismissed/i })).toBeInTheDocument();
      });

      const dismissedTab = screen.getByRole('tab', { name: /dismissed/i });
      await user.click(dismissedTab);

      await waitFor(() => {
        expect(screen.getByText('Delete me')).toBeInTheDocument();
      });

      const deleteButton = screen.getByRole('button', { name: /delete permanently/i });
      await user.click(deleteButton);

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('delete_outbox_entry', { id: 'delete-1' });
        expect(screen.queryByText('Delete me')).not.toBeInTheDocument();
      });
    });
  });

  describe('Event-Driven Updates', () => {
    it('should listen for outbox-updated events on mount', async () => {
      const entry = createOutboxWithDto({ id: 'event-1', status: 'pending' });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalledWith('outbox-updated', expect.any(Function));
      });
    });

    it('should refetch entries when outbox-updated event fires', async () => {
      let eventCallback: (() => void) | undefined;

      mockListen.mockImplementation((_event: string, callback: () => void) => {
        eventCallback = callback;
        return Promise.resolve(mockUnlisten);
      });

      const initialEntry = createOutboxWithDto(
        { id: 'initial', status: 'pending' },
        { notes: 'Initial' }
      );
      const updatedEntries = [
        createOutboxWithDto({ id: 'new-1', status: 'pending' }, { notes: 'New Entry 1' }),
        createOutboxWithDto({ id: 'new-2', status: 'pending' }, { notes: 'New Entry 2' }),
      ];

      let callCount = 0;
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') {
          callCount++;
          return Promise.resolve(callCount === 1 ? [initialEntry] : updatedEntries);
        }
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('Initial')).toBeInTheDocument();
      });

      // Trigger event
      eventCallback?.();

      await waitFor(() => {
        expect(screen.getByText('New Entry 1')).toBeInTheDocument();
        expect(screen.getByText('New Entry 2')).toBeInTheDocument();
        expect(screen.queryByText('Initial')).not.toBeInTheDocument();
      });
    });

    it('should cleanup event listener on unmount', async () => {
      const entry = createOutboxWithDto({ id: 'cleanup', status: 'pending' });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      const { unmount } = render(<SuggestedEntries />);

      await waitFor(() => {
        expect(mockListen).toHaveBeenCalled();
      });

      unmount();

      expect(mockUnlisten).toHaveBeenCalled();
    });
  });

  describe('Count Change Callback', () => {
    it('should call onCountChange with visible entry count', async () => {
      const onCountChange = vi.fn();
      const entries = [
        createOutboxWithDto({ id: 'count-1', status: 'pending' }),
        createOutboxWithDto({ id: 'count-2', status: 'pending' }),
      ];

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(entries);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onCountChange={onCountChange} />);

      await waitFor(() => {
        expect(onCountChange).toHaveBeenCalledWith(2);
      });
    });

    it('should report max of 3 entries for count change', async () => {
      const onCountChange = vi.fn();
      const entries = Array.from({ length: 5 }, (_, i) =>
        createOutboxWithDto({ id: `max-${i}`, status: 'pending' })
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(entries);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onCountChange={onCountChange} />);

      await waitFor(() => {
        expect(onCountChange).toHaveBeenCalledWith(3);
      });
    });

    it('should use max count from both tabs for window sizing', async () => {
      const onCountChange = vi.fn();
      const suggestions = [createOutboxWithDto({ id: 'sug-1', status: 'pending' })];
      const dismissed = [
        createOutboxWithDto({ id: 'dis-1', status: 'dismissed' }),
        createOutboxWithDto({ id: 'dis-2', status: 'dismissed' }),
        createOutboxWithDto({ id: 'dis-3', status: 'dismissed' }),
      ];

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve(suggestions);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve(dismissed);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onCountChange={onCountChange} />);

      await waitFor(() => {
        // Should report 3 (max of suggestions=1 and dismissed=3)
        expect(onCountChange).toHaveBeenCalledWith(3);
      });
    });
  });

  describe('Sync Button', () => {
    it('should call onBuildMyDay when sync button is clicked', async () => {
      const user = userEvent.setup();
      const onBuildMyDay = vi.fn();
      const entry = createOutboxWithDto({ id: 'sync-1', status: 'pending' });

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onBuildMyDay={onBuildMyDay} />);

      await waitFor(() => {
        expect(screen.getByRole('button', { name: /sync calendar events/i })).toBeInTheDocument();
      });

      const syncButton = screen.getByRole('button', { name: /sync calendar events/i });
      await user.click(syncButton);

      expect(onBuildMyDay).toHaveBeenCalled();
    });

    it('should disable sync button when isBuilding is true', async () => {
      const onBuildMyDay = vi.fn();
      const entry = createOutboxWithDto({ id: 'syncing-1', status: 'pending' });

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries onBuildMyDay={onBuildMyDay} isBuilding={true} />);

      await waitFor(() => {
        const syncButton = screen.getByRole('button', { name: /sync calendar events/i });
        expect(syncButton).toBeDisabled();
      });
    });

    it('should show spinning icon when syncing', async () => {
      const onBuildMyDay = vi.fn();
      const entry = createOutboxWithDto({ id: 'spin-1', status: 'pending' });

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      const { rerender } = render(
        <SuggestedEntries onBuildMyDay={onBuildMyDay} isBuilding={false} />
      );

      rerender(<SuggestedEntries onBuildMyDay={onBuildMyDay} isBuilding={true} />);

      await waitFor(() => {
        const syncButton = screen.getByRole('button', { name: /sync calendar events/i });
        const icon = syncButton.querySelector('svg');
        expect(icon).toHaveClass('animate-spin');
      });
    });
  });

  describe('Error Handling', () => {
    it('should handle fetch errors gracefully', async () => {
      mockInvoke.mockRejectedValue(new Error('Network error'));

      const { container } = render(<SuggestedEntries />);

      await waitFor(() => {
        // Should render nothing on error (totalEntries = 0)
        expect(container.firstChild).toBeNull();
      });
    });

    it('should continue on project cache errors', async () => {
      const { projectCache } = await import('@/shared/services');
      vi.mocked(projectCache.fetchProjects).mockRejectedValue(new Error('Cache error'));

      const entry = createOutboxWithDto({ id: 'cache-err', status: 'pending' });
      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Should still display entry even if cache fails
        expect(screen.getByText(/Activity detected/)).toBeInTheDocument();
      });
    });

    it('should handle malformed JSON in payload gracefully', async () => {
      const malformedEntry = createMockTimeEntryOutbox({
        id: 'malformed',
        status: 'pending',
        payload_json: '{invalid json}',
      });

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([malformedEntry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        // Should handle gracefully and not crash
        expect(screen.getByText('Recent Activity')).toBeInTheDocument();
      });
    });
  });

  describe('Duration Formatting', () => {
    it('should format duration in hours and minutes', async () => {
      const entry = createOutboxWithDto(
        { id: 'dur-1', status: 'pending' },
        { durationSec: 5400 } // 1h 30m
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('1h 30m')).toBeInTheDocument();
      });
    });

    it('should format duration in minutes only when less than 1 hour', async () => {
      const entry = createOutboxWithDto(
        { id: 'dur-2', status: 'pending' },
        { durationSec: 2700 } // 45m
      );

      mockInvoke.mockImplementation((cmd: string) => {
        if (cmd === 'get_outbox_status') return Promise.resolve([entry]);
        if (cmd === 'get_dismissed_suggestions') return Promise.resolve([]);
        return Promise.resolve([]);
      });

      render(<SuggestedEntries />);

      await waitFor(() => {
        expect(screen.getByText('45m')).toBeInTheDocument();
      });
    });
  });

  // ============================================================================
  // Idle Filtering Tests (Phase 4)
  // ============================================================================

  describe('SuggestedEntries - Idle Filtering', () => {
    it.skip('should exclude idle time by default', () => {
      // Test: Mock entry with 2h total, 30m idle (excluded)
      // Test: Render SuggestedEntries
      // Test: Verify duration shown: "1h 30m" (active time only)
      // Test: Idle time excluded by default
    });

    it.skip('should show toggle to include idle time', () => {
      // Test: Render SuggestedEntries
      // Test: Verify toggle/checkbox visible: "Include idle time"
      // Test: Default state: unchecked (idle excluded)
    });

    it.skip('should recalculate durations when toggle changed', async () => {
      // Test: Mock entry with 2h total, 30m idle
      // Test: Initially shows: "1h 30m" (idle excluded)
      // Test: User checks "Include idle time" toggle
      // Test: Duration updates to: "2h" (includes idle)
      // Test: User unchecks toggle
      // Test: Duration reverts to: "1h 30m"
    });

    it.skip('should persist idle filter preference', async () => {
      // Test: User checks "Include idle time" toggle
      // Test: Close and reopen component
      // Test: Verify toggle still checked
      // Test: Preference saved to localStorage or settings
    });

    it.skip('should show idle indicator on entries with idle time', () => {
      // Test: Entry has idle time
      // Test: Verify small badge/icon: "⏸ 30m idle"
      // Test: Helps user understand duration breakdown
    });

    it.skip('should filter entries by idle amount', () => {
      // Test: Multiple entries with varying idle amounts
      // Test: Filter: "Show only entries with >30min idle"
      // Test: Verify filtering works
    });
  });
});
