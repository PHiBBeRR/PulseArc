/**
 * Unit tests for IdleDetectionModal component
 *
 * Tests the modal UI that prompts users when idle time is detected.
 * Covers rendering, user interactions, and different idle duration displays.
 *
 * Related: FEATURE-028 (Idle Time Tracking - Phase 4 Frontend Integration)
 */

import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { IdleDetectionModal } from './IdleDetectionModal';

describe('IdleDetectionModal', () => {
  const defaultProps = {
    isOpen: true,
    onKeepTime: vi.fn(),
    onDiscardTime: vi.fn(),
    idleMinutes: 10,
  };

  it('should render when open', () => {
    render(<IdleDetectionModal {...defaultProps} />);
    expect(screen.getByText('Idle Time Detected')).toBeInTheDocument();
  });

  it('should not render when closed', () => {
    render(<IdleDetectionModal {...defaultProps} isOpen={false} />);
    expect(screen.queryByText('Idle Time Detected')).not.toBeInTheDocument();
  });

  it('should display idle duration in minutes', () => {
    render(<IdleDetectionModal {...defaultProps} />);
    expect(screen.getByText(/10 minutes/i)).toBeInTheDocument();
  });

  it('should display formatted idle duration', () => {
    render(<IdleDetectionModal {...defaultProps} />);
    expect(screen.getByText('10m')).toBeInTheDocument();
  });

  it('should call onKeepTime when Keep Time clicked', async () => {
    const user = userEvent.setup();
    const onKeepTime = vi.fn();
    render(<IdleDetectionModal {...defaultProps} onKeepTime={onKeepTime} />);

    await user.click(screen.getByText('Keep Time'));
    expect(onKeepTime).toHaveBeenCalledTimes(1);
  });

  it('should call onDiscardTime when Discard clicked', async () => {
    const user = userEvent.setup();
    const onDiscardTime = vi.fn();
    render(<IdleDetectionModal {...defaultProps} onDiscardTime={onDiscardTime} />);

    await user.click(screen.getByText('Discard'));
    expect(onDiscardTime).toHaveBeenCalledTimes(1);
  });

  it('should render both buttons', () => {
    render(<IdleDetectionModal {...defaultProps} />);
    expect(screen.getByText('Keep Time')).toBeInTheDocument();
    expect(screen.getByText('Discard')).toBeInTheDocument();
  });

  it('should display idle duration label', () => {
    render(<IdleDetectionModal {...defaultProps} />);
    expect(screen.getByText(/Idle duration:/i)).toBeInTheDocument();
  });

  it('should handle different idle durations', () => {
    const { rerender } = render(<IdleDetectionModal {...defaultProps} idleMinutes={5} />);
    expect(screen.getByText(/5 minutes/i)).toBeInTheDocument();

    rerender(<IdleDetectionModal {...defaultProps} idleMinutes={30} />);
    expect(screen.getByText(/30 minutes/i)).toBeInTheDocument();
  });

  it('should not call handlers when modal is closed', () => {
    const onKeepTime = vi.fn();
    const onDiscardTime = vi.fn();
    render(
      <IdleDetectionModal
        {...defaultProps}
        isOpen={false}
        onKeepTime={onKeepTime}
        onDiscardTime={onDiscardTime}
      />
    );

    // Modal should not be in the document
    expect(screen.queryByText('Keep Time')).not.toBeInTheDocument();
    expect(screen.queryByText('Discard')).not.toBeInTheDocument();
    expect(onKeepTime).not.toHaveBeenCalled();
    expect(onDiscardTime).not.toHaveBeenCalled();
  });

  it('should display icon for idle detection', () => {
    const { container } = render(<IdleDetectionModal {...defaultProps} />);
    // Check for Clock icon by looking for SVG elements
    const svgs = container.querySelectorAll('svg');
    expect(svgs.length).toBeGreaterThan(0);
  });

  it('should have accessible button labels', () => {
    render(<IdleDetectionModal {...defaultProps} />);
    const keepButton = screen.getByText('Keep Time');
    const discardButton = screen.getByText('Discard');

    expect(keepButton).toBeInTheDocument();
    expect(discardButton).toBeInTheDocument();
  });

  // ============================================================================
  // FEATURE-028: Idle Time Tracking Tests (Phase 4 - Frontend Integration)
  // ============================================================================

  describe('FEATURE-028 Integration', () => {
    it.skip('should call update_idle_period_action when Keep Time clicked', async () => {
      // Test: Mock update_idle_period_action Tauri command
      // Test: Render modal with idle period ID
      // Test: User clicks "Keep Time"
      // Test: Verify update_idle_period_action called with:
      //   - period_id from props
      //   - action = "kept"
      //   - notes = None
    });

    it.skip('should call update_idle_period_action when Discard clicked', async () => {
      // Test: Mock update_idle_period_action Tauri command
      // Test: Render modal with idle period ID
      // Test: User clicks "Discard"
      // Test: Verify update_idle_period_action called with:
      //   - period_id from props
      //   - action = "discarded"
      //   - notes = None
    });

    it.skip('should display idle period breakdown by trigger type', () => {
      // Test: Render modal with idle period details
      // Test: Idle period triggered by multiple events:
      //   - 5 min from threshold detection
      //   - 10 min from lock screen
      // Test: Verify breakdown displayed: "Threshold: 5m, Lock Screen: 10m"
    });

    it.skip('should show system sleep duration separately', () => {
      // Test: Render modal with idle period triggered by sleep
      // Test: Duration: 60 minutes (system sleep)
      // Test: Verify "System Sleep: 1h" displayed
      // Test: Verify different styling/icon for sleep trigger
    });

    it.skip('should show lock screen duration separately', () => {
      // Test: Render modal with idle period triggered by lock screen
      // Test: Duration: 15 minutes (lock screen)
      // Test: Verify "Lock Screen: 15m" displayed
      // Test: Verify lock icon displayed
    });

    it.skip('should handle API errors gracefully', async () => {
      // Test: Mock update_idle_period_action to return error
      // Test: User clicks "Keep Time"
      // Test: Verify error message displayed
      // Test: Verify modal remains open for retry
      // Test: Verify buttons re-enabled after error
    });

    it.skip('should disable buttons during submission', async () => {
      // Test: Mock update_idle_period_action with delay
      // Test: User clicks "Keep Time"
      // Test: Immediately verify both buttons disabled
      // Test: Verify loading state shown
      // Test: After completion, buttons re-enabled
    });

    it.skip('should show loading state during API call', async () => {
      // Test: Mock update_idle_period_action with delay
      // Test: User clicks "Discard"
      // Test: Verify loading spinner or text displayed
      // Test: Verify "Processing..." or similar message
      // Test: After completion, loading state removed
    });

    it.skip('should close modal after successful submission', async () => {
      // Test: Mock update_idle_period_action to succeed
      // Test: User clicks "Keep Time"
      // Test: Wait for API call to complete
      // Test: Verify modal closes (onClose callback called)
    });

    it.skip('should display idle period ID for debugging', () => {
      // Test: Render modal in development mode
      // Test: Verify idle period ID displayed (or in data attribute)
      // Test: Useful for debugging and support
    });
  });
});
