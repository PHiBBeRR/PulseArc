/**
 * Unit tests for SuggestionChip component
 *
 * Tests the UI component that displays activity-based suggestions to users
 * for logging time entries. Suggestions are derived from detected activities
 * and can be accepted or dismissed with haptic feedback.
 *
 * Test Coverage:
 * - Rendering: Display of suggestion text, labels, and metadata
 * - User Interactions: Accept/dismiss buttons with callbacks
 * - Haptic Feedback: Touch feedback on user actions
 * - Visual States: Different states based on suggestion source
 * - Accessibility: Button labels and keyboard navigation
 */

import { haptic } from '@/shared/utils';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { type SuggestionState } from '../types';
import { SuggestionChip } from './SuggestionChip';

describe('SuggestionChip', () => {
  let mockSuggestion: SuggestionState;
  let mockOnAccept: ReturnType<typeof vi.fn>;
  let mockOnDismiss: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    mockSuggestion = {
      text: 'Working on AI entry improvements',
      timestamp: Date.now(),
      source: 'activity',
      metadata: {
        appName: 'Cursor',
      },
    };

    mockOnAccept = vi.fn();
    mockOnDismiss = vi.fn();
    vi.clearAllMocks();
  });

  describe('Rendering', () => {
    it('renders suggestion text correctly', () => {
      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Working on AI entry improvements')).toBeInTheDocument();
    });

    it('renders the "SUGGESTION" label', () => {
      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Suggestion')).toBeInTheDocument();
    });

    it('renders "Use" button', () => {
      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByRole('button', { name: /use/i })).toBeInTheDocument();
    });

    it('renders dismiss button with X icon', () => {
      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Check for dismiss button with aria-label
      const dismissButton = screen.getByRole('button', { name: /dismiss/i });
      expect(dismissButton).toBeInTheDocument();
    });

    it('truncates long suggestion text', () => {
      const longSuggestion: SuggestionState = {
        ...mockSuggestion,
        text: 'This is a very long suggestion text that should be truncated when it exceeds the available space in the suggestion chip container',
      };

      render(
        <SuggestionChip
          suggestion={longSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const textElement = screen.getByText(longSuggestion.text);
      expect(textElement).toHaveClass('truncate');
    });
  });

  describe('User Interactions', () => {
    it('calls onAccept when "Use" button is clicked', async () => {
      const user = userEvent.setup();

      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const useButton = screen.getByRole('button', { name: /use/i });
      await user.click(useButton);

      expect(mockOnAccept).toHaveBeenCalledTimes(1);
    });

    it('calls onDismiss when dismiss button is clicked', async () => {
      const user = userEvent.setup();

      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Use aria-label to find dismiss button
      const dismissButton = screen.getByRole('button', { name: /dismiss/i });
      await user.click(dismissButton);

      expect(mockOnDismiss).toHaveBeenCalledTimes(1);
    });

    it('triggers haptic feedback on accept', async () => {
      const user = userEvent.setup();

      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const useButton = screen.getByRole('button', { name: /use/i });
      await user.click(useButton);

      expect(haptic.light).toHaveBeenCalled();
    });

    it('triggers haptic feedback on dismiss', async () => {
      const user = userEvent.setup();

      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const dismissButton = screen.getByRole('button', { name: /dismiss/i });
      await user.click(dismissButton);

      expect(haptic.light).toHaveBeenCalled();
    });

    it('does not call onDismiss when accept is clicked', async () => {
      const user = userEvent.setup();

      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const useButton = screen.getByRole('button', { name: /use/i });
      await user.click(useButton);

      expect(mockOnDismiss).not.toHaveBeenCalled();
    });

    it('does not call onAccept when dismiss is clicked', async () => {
      const user = userEvent.setup();

      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const dismissButton = screen.getByRole('button', { name: /dismiss/i });
      await user.click(dismissButton);

      expect(mockOnAccept).not.toHaveBeenCalled();
    });
  });

  describe('Styling and Visual Feedback', () => {
    it('has proper gradient background', () => {
      const { container } = render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const chipElement = container.querySelector('.bg-gradient-to-br');
      expect(chipElement).toBeInTheDocument();
    });

    it('displays sparkles icon', () => {
      const { container } = render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Check for svg element (lucide-react icons render as svg)
      const icons = container.querySelectorAll('svg');
      expect(icons.length).toBeGreaterThan(0);
    });

    it('has animation classes', () => {
      const { container } = render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const chipElement = container.querySelector('.animate-in');
      expect(chipElement).toBeInTheDocument();
    });
  });

  describe('Different Suggestion Sources', () => {
    it('renders suggestion from activity source', () => {
      render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Working on AI entry improvements')).toBeInTheDocument();
    });

    it('renders suggestion from project source', () => {
      const projectSuggestion: SuggestionState = {
        text: 'Pulsarc: Bug fixes',
        timestamp: Date.now(),
        source: 'project',
        metadata: {
          projectName: 'Pulsarc',
        },
      };

      render(
        <SuggestionChip
          suggestion={projectSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Pulsarc: Bug fixes')).toBeInTheDocument();
    });

    it('renders suggestion from meeting source', () => {
      const meetingSuggestion: SuggestionState = {
        text: 'Meeting: Engineering Standup',
        timestamp: Date.now(),
        source: 'meeting',
        metadata: {
          meetingTime: '14:00-14:30',
        },
      };

      render(
        <SuggestionChip
          suggestion={meetingSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Meeting: Engineering Standup')).toBeInTheDocument();
    });
  });

  describe('Phase 2: Stale Suggestions', () => {
    it('renders stale suggestion with dashed border', () => {
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      const { container } = render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const chipElement = container.querySelector('.border-dashed');
      expect(chipElement).toBeInTheDocument();
    });

    it('displays timestamp for stale suggestion', () => {
      const twoMinutesAgo = Date.now() - 2 * 60 * 1000;
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: twoMinutesAgo,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('2m ago')).toBeInTheDocument();
    });

    it('shows clock icon for stale suggestion', () => {
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      const { container } = render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Clock icon is an SVG with specific size
      const icons = container.querySelectorAll('svg');
      expect(icons.length).toBeGreaterThan(0);
    });

    it('has reduced opacity for stale suggestion', () => {
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      const { container } = render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const chipElement = container.querySelector('.opacity-70');
      expect(chipElement).toBeInTheDocument();
    });
  });

  describe('Phase 2: Updated Suggestions', () => {
    it('displays "UPDATED" badge for updated suggestion', () => {
      const updatedSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isUpdated: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={updatedSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Updated')).toBeInTheDocument();
    });

    it('shows amber border for updated suggestion', () => {
      const updatedSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isUpdated: true,
        },
      };

      const { container } = render(
        <SuggestionChip
          suggestion={updatedSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const chipElement = container.querySelector('.border-amber-400');
      expect(chipElement).toBeInTheDocument();
    });

    it('shows RefreshCw icon for updated suggestion', () => {
      const updatedSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isUpdated: true,
        },
      };

      const { container } = render(
        <SuggestionChip
          suggestion={updatedSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // RefreshCw icon is an SVG with animate-spin class
      const spinningIcon = container.querySelector('.animate-spin');
      expect(spinningIcon).toBeInTheDocument();
    });

    it('does not show timestamp for updated suggestion', () => {
      const updatedSuggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: Date.now() - 60000, // 1 minute ago
        metadata: {
          ...mockSuggestion.metadata,
          isUpdated: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={updatedSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.queryByText('1m ago')).not.toBeInTheDocument();
    });
  });

  describe('Phase 2: Timestamp Formatting', () => {
    it('formats timestamp as "just now" for recent suggestions', () => {
      const recentSuggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: Date.now() - 30000, // 30 seconds ago
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true, // Need stale to show timestamp
        },
      };

      render(
        <SuggestionChip
          suggestion={recentSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('just now')).toBeInTheDocument();
    });

    it('formats timestamp as "1m ago" for 1 minute old suggestion', () => {
      const oneMinuteAgo = Date.now() - 60000;
      const suggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: oneMinuteAgo,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      render(
        <SuggestionChip suggestion={suggestion} onAccept={mockOnAccept} onDismiss={mockOnDismiss} />
      );

      expect(screen.getByText('1m ago')).toBeInTheDocument();
    });

    it('formats timestamp correctly for multiple minutes', () => {
      const fiveMinutesAgo = Date.now() - 5 * 60 * 1000;
      const suggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: fiveMinutesAgo,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      render(
        <SuggestionChip suggestion={suggestion} onAccept={mockOnAccept} onDismiss={mockOnDismiss} />
      );

      expect(screen.getByText('5m ago')).toBeInTheDocument();
    });
  });

  describe('Phase 2: Context-Aware Stale Detection', () => {
    it('can still accept stale suggestions', async () => {
      const user = userEvent.setup();
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: Date.now() - 3 * 60 * 1000, // 3 minutes ago
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const useButton = screen.getByRole('button', { name: /use/i });
      await user.click(useButton);

      expect(mockOnAccept).toHaveBeenCalledTimes(1);
    });

    it('shows gray "Use" button for stale suggestions', () => {
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const useButton = screen.getByRole('button', { name: /use/i });
      expect(useButton).toHaveClass('bg-gray-500');
    });

    it('transitions from fresh to stale appearance', () => {
      const { container, rerender } = render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Initially fresh - should have sparkles icon
      const icons = container.querySelectorAll('svg');
      expect(icons.length).toBeGreaterThan(0);

      // Update to stale with old timestamp
      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: Date.now() - 3 * 60 * 1000, // 3 minutes ago
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      rerender(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Should now have dashed border
      const chipElement = container.querySelector('.border-dashed');
      expect(chipElement).toBeInTheDocument();

      // Should show timestamp
      expect(screen.getByText('3m ago')).toBeInTheDocument();
    });

    it('shows different visual states for fresh vs stale', () => {
      const { container: freshContainer } = render(
        <SuggestionChip
          suggestion={mockSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      const staleSuggestion: SuggestionState = {
        ...mockSuggestion,
        timestamp: Date.now() - 3 * 60 * 1000,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      const { container: staleContainer } = render(
        <SuggestionChip
          suggestion={staleSuggestion}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Fresh should have red gradient
      const freshChip = freshContainer.querySelector('.from-red-50');
      expect(freshChip).toBeInTheDocument();

      // Stale should have gray gradient
      const staleChip = staleContainer.querySelector('.from-gray-50');
      expect(staleChip).toBeInTheDocument();

      // Stale should have reduced opacity
      const staleOpacity = staleContainer.querySelector('.opacity-70');
      expect(staleOpacity).toBeInTheDocument();
    });
  });

  describe('Confidence Score Display', () => {
    it('displays confidence percentage when confidence is provided', () => {
      const suggestionWithConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.95,
      };

      render(
        <SuggestionChip
          suggestion={suggestionWithConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('95%')).toBeInTheDocument();
    });

    it('displays different confidence levels correctly', () => {
      const lowConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.6,
      };

      const { rerender } = render(
        <SuggestionChip
          suggestion={lowConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('60%')).toBeInTheDocument();

      const highConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.99,
      };

      rerender(
        <SuggestionChip
          suggestion={highConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('99%')).toBeInTheDocument();
    });

    it('rounds confidence to nearest whole number', () => {
      const suggestionWithDecimal: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.847, // Should round to 85%
      };

      render(
        <SuggestionChip
          suggestion={suggestionWithDecimal}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('85%')).toBeInTheDocument();
    });

    it('does not display confidence for stale suggestions', () => {
      const staleSuggestionWithConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.85,
        metadata: {
          ...mockSuggestion.metadata,
          isStale: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={staleSuggestionWithConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.queryByText('85%')).not.toBeInTheDocument();
    });

    it('does not display confidence when not provided', () => {
      const suggestionWithoutConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: undefined,
      };

      const { container } = render(
        <SuggestionChip
          suggestion={suggestionWithoutConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Should not have green confidence badge
      const confidenceBadge = container.querySelector('.bg-green-500\\/20');
      expect(confidenceBadge).not.toBeInTheDocument();
    });

    it('does not display confidence when value is 0', () => {
      const suggestionWithZeroConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0,
      };

      render(
        <SuggestionChip
          suggestion={suggestionWithZeroConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.queryByText('0%')).not.toBeInTheDocument();
    });

    it('displays confidence badge with correct styling', () => {
      const suggestionWithConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.92,
      };

      const { container } = render(
        <SuggestionChip
          suggestion={suggestionWithConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Check for green badge styling
      const confidenceBadge = container.querySelector('.bg-green-500\\/20');
      expect(confidenceBadge).toBeInTheDocument();
      expect(confidenceBadge).toHaveTextContent('92%');
    });

    it('displays both updated badge and confidence score when both present', () => {
      const updatedSuggestionWithConfidence: SuggestionState = {
        ...mockSuggestion,
        confidence: 0.88,
        metadata: {
          ...mockSuggestion.metadata,
          isUpdated: true,
        },
      };

      render(
        <SuggestionChip
          suggestion={updatedSuggestionWithConfidence}
          onAccept={mockOnAccept}
          onDismiss={mockOnDismiss}
        />
      );

      // Both badges should be present
      expect(screen.getByText('Updated')).toBeInTheDocument();
      expect(screen.getByText('88%')).toBeInTheDocument();
    });
  });
});
