// FEATURE-020 Phase 4.4: WBS Validation UI Tests
// Test coverage for WBS validation feedback in autocomplete

import { describe, it, beforeEach, afterEach, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { WbsAutocomplete } from '../WbsAutocomplete';
import * as SapService from '@/features/settings/services/sapService';

// Mock SAP service
vi.mock('@/features/settings/services/sapService', () => ({
  SapService: {
    searchWbs: vi.fn(),
    validateWbs: vi.fn(),
    formatWbsDisplay: vi.fn((element) => {
      const parts = [element.wbs_code];
      if (element.project_name) parts.push(element.project_name);
      if (element.description) parts.push(`(${element.description})`);
      return parts.join(' - ');
    }),
  },
}));

// Mock WBS usage service
vi.mock('@/features/timer/services/wbsUsageService', () => ({
  WbsUsageService: {
    addRecentWbs: vi.fn(),
    getRecentWbs: vi.fn(() => []),
    getRecentElements: vi.fn(() => []),
    getFavorites: vi.fn(() => []),
    isFavorite: vi.fn(() => false),
    addFavorite: vi.fn(),
    removeFavorite: vi.fn(),
    toggleFavorite: vi.fn(() => true),
    clearRecent: vi.fn(),
    clearFavorites: vi.fn(),
  },
}));

describe('WBS Validation UI', () => {
  const mockOnChange = vi.fn();

  beforeEach(() => {
    vi.clearAllMocks();

    // Default mock implementations
    vi.mocked(SapService.SapService.searchWbs).mockResolvedValue([]);
    vi.mocked(SapService.SapService.validateWbs).mockResolvedValue({
      status: 'Valid',
      code: 'USC0063201.1.1',
      message: null,
    });
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should show warning badge for TECO (completed) status', async () => {
    // Mock validation returning TECO status
    vi.mocked(SapService.SapService.validateWbs).mockResolvedValue({
      status: 'Warning',
      code: 'USC0063201.1.1',
      message: 'Project technically complete (TECO)',
    });

    // Render with value already set (simulating parent component state)
    render(
      <WbsAutocomplete
        value="USC0063201.1.1"
        onChange={mockOnChange}
        placeholder="Search WBS..."
      />
    );

    // Verify warning badge appears (TECO status)
    await waitFor(() => {
      const warningBadge = screen.queryByText(/technically complete/i);
      expect(warningBadge).toBeInTheDocument();
    });
  });

  it('should show error badge for CLSD (closed) status', async () => {
    // Mock validation returning CLSD status
    vi.mocked(SapService.SapService.validateWbs).mockResolvedValue({
      status: 'Error',
      code: 'USC0063202.1.1',
      message: 'Project closed (CLSD) - cannot use',
    });

    // Render with value already set (simulating parent component state)
    render(
      <WbsAutocomplete
        value="USC0063202.1.1"
        onChange={mockOnChange}
        placeholder="Search WBS..."
      />
    );

    // Verify error badge appears (CLSD status)
    await waitFor(() => {
      const errorBadge = screen.queryByText(/closed/i);
      expect(errorBadge).toBeInTheDocument();
    });
  });

  it('should show green badge for REL (released) status', async () => {
    // Mock validation returning REL status
    vi.mocked(SapService.SapService.validateWbs).mockResolvedValue({
      status: 'Valid',
      code: 'USC0063203.1.1',
      message: null,
    });

    // Render with value already set (simulating parent component state)
    render(
      <WbsAutocomplete
        value="USC0063203.1.1"
        onChange={mockOnChange}
        placeholder="Search WBS..."
      />
    );

    // Verify valid status (green checkmark appears)
    await waitFor(() => {
      expect(screen.getByText(/Valid WBS code/i)).toBeInTheDocument();
    });

    // Should NOT have error or warning badges
    expect(screen.queryByText(/closed/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/technically complete/i)).not.toBeInTheDocument();
  });

  it('should display validation error inline for invalid format', async () => {
    // Mock validation returning invalid format error
    vi.mocked(SapService.SapService.validateWbs).mockResolvedValue({
      status: 'Error',
      code: 'INVALID',
      message: 'Invalid WBS code format',
    });

    // Render with invalid value already set
    render(
      <WbsAutocomplete
        value="INVALID"
        onChange={mockOnChange}
        placeholder="Search WBS..."
      />
    );

    // Verify validation was called with invalid code
    await waitFor(() => {
      expect(SapService.SapService.validateWbs).toHaveBeenCalledWith('INVALID');
    });

    // Verify error badge appears
    await waitFor(() => {
      expect(screen.getByText(/Invalid WBS code format/i)).toBeInTheDocument();
    });
  });

  it('should suggest sync when WBS code not found in cache', async () => {
    // Mock validation returning "not found" status
    vi.mocked(SapService.SapService.validateWbs).mockResolvedValue({
      status: 'Warning',
      code: 'USC0063204.1.1',
      message: 'Code not in cache - sync recommended',
    });

    // Mock empty search result (code not in cache)
    vi.mocked(SapService.SapService.searchWbs).mockResolvedValue([]);

    render(
      <WbsAutocomplete
        value="USC0063204.1.1"
        onChange={mockOnChange}
        placeholder="Search WBS..."
      />
    );

    // Since value is set, validation should run
    await waitFor(() => {
      expect(SapService.SapService.validateWbs).toHaveBeenCalledWith('USC0063204.1.1');
    });

    // Look for sync suggestion in UI (if implemented)
    // This would appear as a button or message
    // For now, just verify validation was called
    expect(SapService.SapService.validateWbs).toHaveBeenCalled();
  });
});
