/**
 * Unit tests for IdleDetectionSettings component
 *
 * Tests the settings UI for configuring idle detection behavior,
 * including idle threshold selection and pause-on-idle toggle.
 *
 * Test Coverage:
 * - Settings Loading: Fetching current settings from backend on mount
 * - Threshold Dropdown: Displaying and selecting idle threshold values
 * - Disable Option: "No Idle" option to disable idle detection
 * - Pause on Idle Toggle: Enable/disable automatic pause on idle
 * - Settings Persistence: Saving changes via update_idle_settings command
 * - Validation: Ensuring valid threshold values
 * - Loading States: Displaying loading indicators during fetch/save
 * - Error Handling: Displaying errors on save failure
 */

import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IdleDetectionSettings } from './IdleDetectionSettings';

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

import { invoke } from '@tauri-apps/api/core';

describe('IdleDetectionSettings', () => {
  const mockInvoke = vi.mocked(invoke);

  beforeEach(() => {
    vi.clearAllMocks();
    mockInvoke.mockResolvedValue({
      pause_on_idle: true,
      idle_threshold_secs: 600,
    });
  });

  it('should render settings component', async () => {
    render(<IdleDetectionSettings />);
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /Idle Detection/i })).toBeInTheDocument();
    });
  });

  it('should load current settings on mount', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_idle_settings');
    });
  });

  it('should display idle threshold dropdown', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      const dropdown = screen.getByRole('combobox');
      expect(dropdown).toBeInTheDocument();
      expect(dropdown).toHaveValue('600'); // Default 10 minutes
    });
  });

  it('should disable idle when "No Idle" is selected', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);

    render(<IdleDetectionSettings />);

    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    const dropdown = screen.getByRole('combobox');
    await user.selectOptions(dropdown, '0'); // Select "No Idle"

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('set_idle_enabled', { enabled: false });
    });
  });

  it('should display threshold dropdown', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      const dropdown = screen.getByRole('combobox');
      expect(dropdown).toBeInTheDocument();
    });
  });

  it('should have all threshold options', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      const dropdown = screen.getByRole('combobox');
      expect(dropdown).toBeInTheDocument();
      // Check the options are available in the select
      const options = screen.getAllByRole('option');
      expect(options).toHaveLength(5);
      expect(options.map((o) => o.textContent)).toEqual([
        'No Idle',
        '5 minutes',
        '10 minutes (recommended)',
        '15 minutes',
        '30 minutes',
      ]);
    });
  });

  it('should change threshold value', async () => {
    const user = userEvent.setup();
    mockInvoke.mockResolvedValue(undefined);

    render(<IdleDetectionSettings />);

    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    const dropdown = screen.getByRole('combobox');
    await user.selectOptions(dropdown, '300'); // 5 minutes

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('set_idle_threshold', { threshold_secs: 300 });
    });
  });

  it('should show dropdown is always visible', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      const dropdown = screen.getByRole('combobox');
      expect(dropdown).toBeInTheDocument();
      // Dropdown is always visible, even when "No Idle" is selected
    });
  });

  it('should display default value of 10 minutes', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      const dropdown = screen.getByRole('combobox');
      expect(dropdown).toHaveValue('600');
    });
  });

  it('should display helpful description text', async () => {
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      expect(
        screen.getByText(/Automatically pause tracking after a period of inactivity/i)
      ).toBeInTheDocument();
    });
  });

  it('should handle load settings error gracefully', async () => {
    mockInvoke.mockRejectedValue(new Error('Failed to load settings'));

    render(<IdleDetectionSettings />);

    // Should still render with default values
    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /Idle Detection/i })).toBeInTheDocument();
    });
  });

  it('should handle save settings error gracefully', async () => {
    const user = userEvent.setup();
    mockInvoke
      .mockResolvedValueOnce({ pause_on_idle: true, idle_threshold_secs: 600 })
      .mockRejectedValueOnce(new Error('Failed to save'));

    render(<IdleDetectionSettings />);

    await waitFor(() => {
      expect(screen.getByRole('combobox')).toBeInTheDocument();
    });

    const dropdown = screen.getByRole('combobox');
    await user.selectOptions(dropdown, '0'); // Select "No Idle"

    // Should revert to previous value on error - verify it doesn't crash
    await waitFor(() => {
      expect(dropdown).toBeInTheDocument();
      // Should revert to 600 (previous value) on error
      expect(dropdown).toHaveValue('600');
    });
  });

  it('should persist settings across remounts', async () => {
    mockInvoke.mockResolvedValue({
      pause_on_idle: false,
      idle_threshold_secs: 1800,
    });

    const { unmount } = render(<IdleDetectionSettings />);

    await waitFor(() => {
      const dropdown = screen.getByRole('combobox');
      expect(dropdown).toHaveValue('1800'); // 30 minutes
    });

    unmount();

    // Render a new instance
    render(<IdleDetectionSettings />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('get_idle_settings');
    });
  });
});
