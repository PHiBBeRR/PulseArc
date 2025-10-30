import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { DismissFeedbackModal } from './DismissFeedbackModal';
import type { TimeEntry } from '../types';

// Mock haptic utility
vi.mock('@/shared/utils', () => ({
  haptic: {
    light: vi.fn(),
  },
}));

describe('DismissFeedbackModal', () => {
  const mockEntry: TimeEntry = {
    id: 'entry-789',
    time: '3:30 PM',
    project: 'Beta Project',
    task: 'Code review session',
    duration: '45m',
    status: 'suggested',
    confidence: 0.75,
    source: 'ai',
  };

  const mockOnClose = vi.fn();
  const mockOnDismiss = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();
    mockOnDismiss.mockResolvedValue(undefined);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  describe('Rendering', () => {
    it('should not render when isOpen is false', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={false}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    });

    it('should render modal with correct title when isOpen is true', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByRole('dialog')).toBeInTheDocument();
      expect(screen.getByText('Help improve suggestions')).toBeInTheDocument();
    });

    it('should display entry summary', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByText('Code review session')).toBeInTheDocument();
      expect(screen.getByText(/beta project/i)).toBeInTheDocument();
      expect(screen.getByText(/45m/i)).toBeInTheDocument();
    });

    it('should render combobox trigger with placeholder text', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByRole('combobox')).toBeInTheDocument();
      expect(screen.getByText('Choose a reason...')).toBeInTheDocument();
    });
  });

  describe('Reason Selection via Combobox', () => {
    it('should open dropdown when combobox is clicked', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const combobox = screen.getByRole('combobox');
      await user.click(combobox);

      await waitFor(() => {
        expect(screen.getByText('Wrong project')).toBeInTheDocument();
        expect(screen.getByText('Duplicate entry')).toBeInTheDocument();
        expect(screen.getByText('Not relevant to my work')).toBeInTheDocument();
        expect(screen.getByText('Other (please specify)')).toBeInTheDocument();
      });
    });

    it('should select reason and close dropdown when option is clicked', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const combobox = screen.getByRole('combobox');
      await user.click(combobox);

      const option = screen.getByText('Wrong project');
      await user.click(option);

      await waitFor(() => {
        expect(screen.getByRole('combobox')).toHaveTextContent('Wrong project');
      });
    });

    it('should auto-fill textarea when non-other reason is selected', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const combobox = screen.getByRole('combobox');
      await user.click(combobox);

      const option = screen.getByText('Not relevant to my work');
      await user.click(option);

      await waitFor(() => {
        const textarea = screen.getByPlaceholderText(/Optional: Add more details/i);
        expect(textarea).toHaveValue('Not relevant to my work');
      });
    });

    it('should clear textarea when "Other" is selected', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const combobox = screen.getByRole('combobox');
      await user.click(combobox);

      const option = screen.getByText('Other (please specify)');
      await user.click(option);

      await waitFor(() => {
        const textarea = screen.getByPlaceholderText(/Optional: Add more details/i);
        expect(textarea).toHaveValue('');
      });
    });
  });

  describe('Submit and Skip Buttons', () => {
    it('should show Submit and Skip buttons', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      expect(screen.getByRole('button', { name: /skip/i })).toBeInTheDocument();
      expect(screen.getByRole('button', { name: /submit/i })).toBeInTheDocument();
    });

    it('should call onDismiss with empty string when Skip is clicked', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const skipButton = screen.getByRole('button', { name: /skip/i });
      await user.click(skipButton);

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith(mockEntry, '');
      });
    });

    it('should call onDismiss with feedback text when Submit is clicked', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      // Select a reason
      const combobox = screen.getByRole('combobox');
      await user.click(combobox);
      const option = screen.getByText('Wrong project');
      await user.click(option);

      // Click submit
      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith(mockEntry, 'Wrong project');
      });
    });

    it('should call onDismiss with custom text when user types in textarea', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const textarea = screen.getByPlaceholderText(/Optional: Add more details/i);
      await user.clear(textarea);
      await user.type(textarea, 'Custom dismissal reason');

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith(mockEntry, 'Custom dismissal reason');
      });
    });

    it('should call onClose after successful submit', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(mockOnClose).toHaveBeenCalled();
      });
    });

    it('should call onClose after successful skip', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const skipButton = screen.getByRole('button', { name: /skip/i });
      await user.click(skipButton);

      await waitFor(() => {
        expect(mockOnClose).toHaveBeenCalled();
      });
    });
  });

  describe('Textarea Behavior', () => {
    it('should allow user to edit auto-filled feedback', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      // Select a reason that auto-fills
      const combobox = screen.getByRole('combobox');
      await user.click(combobox);
      const option = screen.getByText('Duplicate entry');
      await user.click(option);

      // Edit the textarea
      const textarea = screen.getByPlaceholderText(/Optional: Add more details/i);
      await user.clear(textarea);
      await user.type(textarea, 'Modified feedback text');

      await waitFor(() => {
        expect(textarea).toHaveValue('Modified feedback text');
      });
    });

    it('should accept empty feedback when submitting', async () => {
      const user = userEvent.setup();
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalledWith(mockEntry, '');
      });
    });
  });

  describe('Form Reset', () => {
    it('should reset form state after successful submission', async () => {
      const user = userEvent.setup();
      const { rerender } = render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      // Select a reason and submit
      const combobox = screen.getByRole('combobox');
      await user.click(combobox);
      const option = screen.getByText('Wrong project');
      await user.click(option);

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(mockOnClose).toHaveBeenCalled();
      });

      // Reopen modal
      rerender(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={false}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      rerender(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      // Form should be reset - combobox should show placeholder
      expect(screen.getByText('Choose a reason...')).toBeInTheDocument();
    });
  });

  describe('Error Handling', () => {
    it('should handle dismiss errors gracefully', async () => {
      const user = userEvent.setup();
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockOnDismiss.mockRejectedValue(new Error('Network error'));

      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith(
          'Failed to dismiss entry:',
          expect.any(Error)
        );
      });

      consoleErrorSpy.mockRestore();
    });

    it('should keep modal open on error', async () => {
      const user = userEvent.setup();
      vi.spyOn(console, 'error').mockImplementation(() => {});
      mockOnDismiss.mockRejectedValue(new Error('Network error'));

      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(mockOnDismiss).toHaveBeenCalled();
      });

      // Modal should still be open (onClose not called on error)
      expect(mockOnClose).not.toHaveBeenCalled();
    });

    it('should handle skip errors gracefully', async () => {
      const user = userEvent.setup();
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockOnDismiss.mockRejectedValue(new Error('Network error'));

      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const skipButton = screen.getByRole('button', { name: /skip/i });
      await user.click(skipButton);

      await waitFor(() => {
        expect(consoleErrorSpy).toHaveBeenCalledWith(
          'Failed to dismiss entry:',
          expect.any(Error)
        );
      });

      consoleErrorSpy.mockRestore();
    });
  });

  describe('Accessibility', () => {
    it('should have proper combobox ARIA attributes', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const combobox = screen.getByRole('combobox');
      expect(combobox).toHaveAttribute('aria-expanded');
    });

    it('should have accessible button labels', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      const skipButton = screen.getByRole('button', { name: /skip/i });

      expect(submitButton).toHaveAccessibleName();
      expect(skipButton).toHaveAccessibleName();
    });

    it('should have accessible textarea', () => {
      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const textarea = screen.getByPlaceholderText(/Optional: Add more details/i);
      expect(textarea).toBeInTheDocument();
      expect(textarea).toHaveAttribute('placeholder');
    });
  });

  describe('Disabled States', () => {
    it('should disable buttons while submitting', async () => {
      const user = userEvent.setup();
      // Make onDismiss hang to simulate loading state
      mockOnDismiss.mockImplementation(() => new Promise(() => {}));

      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(submitButton).toBeDisabled();
        expect(screen.getByRole('button', { name: /skip/i })).toBeDisabled();
      });
    });

    it('should disable combobox while submitting', async () => {
      const user = userEvent.setup();
      // Make onDismiss hang to simulate loading state
      mockOnDismiss.mockImplementation(() => new Promise(() => {}));

      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        expect(screen.getByRole('combobox')).toBeDisabled();
      });
    });

    it('should disable textarea while submitting', async () => {
      const user = userEvent.setup();
      // Make onDismiss hang to simulate loading state
      mockOnDismiss.mockImplementation(() => new Promise(() => {}));

      render(
        <DismissFeedbackModal
          entry={mockEntry}
          isOpen={true}
          onClose={mockOnClose}
          onDismiss={mockOnDismiss}
        />
      );

      const submitButton = screen.getByRole('button', { name: /submit/i });
      await user.click(submitButton);

      await waitFor(() => {
        const textarea = screen.getByPlaceholderText(/Optional: Add more details/i);
        expect(textarea).toBeDisabled();
      });
    });
  });
});
