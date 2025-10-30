// FEATURE-020 Phase 2: WBS Autocomplete Component Tests
// Test coverage for WBS autocomplete UI with FTS5 search

import { describe, it, beforeEach, afterEach, vi, expect } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { WbsAutocomplete } from './WbsAutocomplete';
import { SapService } from '@/features/settings/services/sapService';
import type { WbsElement } from '@/shared/types/generated';

// Mock SapService
vi.mock('@/features/settings/services/sapService', () => ({
  SapService: {
    searchWbs: vi.fn(),
    formatWbsDisplay: vi.fn((element: WbsElement) => {
      const parts = [element.wbs_code];
      if (element.project_name) parts.push(element.project_name);
      if (element.description) parts.push(`(${element.description})`);
      return parts.join(' - ');
    }),
  },
}));

describe('WbsAutocomplete', () => {
  const mockOnChange = vi.fn();
  const mockWbsElements: WbsElement[] = [
    {
      wbs_code: 'USC0063201.1.1',
      project_def: 'USC0063201',
      project_name: 'Project Astro - Tech Acquisition',
      description: 'Project Astro - Deals - M&A Tax',
      status: 'REL',
      cached_at: Date.now(),
      // FEATURE-029: Enriched fields
      opportunity_id: null,
      deal_name: null,
      target_company_name: null,
      counterparty: null,
      industry: null,
      region: null,
      amount: null,
      stage_name: null,
      project_code: null,
    },
    {
      wbs_code: 'USC0063202.1.1',
      project_def: 'USC0063202',
      project_name: 'Project Beta - Pharma Merger',
      description: 'Project Beta - Deals - M&A Tax',
      status: 'REL',
      cached_at: Date.now(),
      // FEATURE-029: Enriched fields
      opportunity_id: null,
      deal_name: null,
      target_company_name: null,
      counterparty: null,
      industry: null,
      region: null,
      amount: null,
      stage_name: null,
      project_code: null,
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(SapService.searchWbs).mockResolvedValue([]);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should render combobox button with placeholder', () => {
    render(<WbsAutocomplete value="" onChange={mockOnChange} placeholder="Select WBS code..." />);

    expect(screen.getByRole('combobox')).toBeInTheDocument();
    expect(screen.getByText('Select WBS code...')).toBeInTheDocument();
  });

  it('should display selected WBS code when value provided', () => {
    render(<WbsAutocomplete value="USC0063201.1.1" onChange={mockOnChange} />);

    expect(screen.getByText('USC0063201.1.1')).toBeInTheDocument();
  });

  it('should open dropdown when combobox clicked', async () => {
    const user = userEvent.setup();
    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    await waitFor(() => {
      expect(screen.getByPlaceholderText(/Search WBS code/)).toBeInTheDocument();
    });
  });

  it('should debounce search input (200ms)', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.searchWbs).mockResolvedValue(mockWbsElements);

    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    const searchInput = await screen.findByPlaceholderText(/Search WBS code/);
    await user.type(searchInput, 'Astro');

    // Should not call immediately
    expect(SapService.searchWbs).not.toHaveBeenCalled();

    // Wait for debounce (200ms)
    await waitFor(
      () => {
        expect(SapService.searchWbs).toHaveBeenCalledWith('Astro');
      },
      { timeout: 300 }
    );

    // Should only call once despite multiple keystrokes
    expect(SapService.searchWbs).toHaveBeenCalledTimes(1);
  });

  it('should display search results in dropdown', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.searchWbs).mockResolvedValue(mockWbsElements);

    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    const searchInput = await screen.findByPlaceholderText(/Search WBS code/);
    await user.type(searchInput, 'Project');

    await waitFor(() => {
      expect(screen.getByText('USC0063201.1.1')).toBeInTheDocument();
      expect(screen.getByText('USC0063202.1.1')).toBeInTheDocument();
    });
  });

  it('should display WBS code + project name + description in results', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.searchWbs).mockResolvedValue([mockWbsElements[0]]);

    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    const searchInput = await screen.findByPlaceholderText(/Search WBS code/);
    await user.type(searchInput, 'Astro');

    await waitFor(() => {
      expect(screen.getByText('USC0063201.1.1')).toBeInTheDocument();
      expect(screen.getByText('Project Astro - Tech Acquisition')).toBeInTheDocument();
      expect(screen.getByText('Project Astro - Deals - M&A Tax')).toBeInTheDocument();
    });
  });

  it('should show loading spinner during search', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.searchWbs).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve(mockWbsElements), 500))
    );

    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    const searchInput = await screen.findByPlaceholderText(/Search WBS code/);
    await user.type(searchInput, 'Project');

    await waitFor(() => {
      expect(screen.getByText('Searching...')).toBeInTheDocument();
    });
  });

  it('should display "No WBS codes found" when search returns empty', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.searchWbs).mockResolvedValue([]);

    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    const searchInput = await screen.findByPlaceholderText(/Search WBS code/);
    await user.type(searchInput, 'NonexistentCode');

    await waitFor(() => {
      expect(screen.getByText('No WBS codes found.')).toBeInTheDocument();
    });
  });

  it('should call onChange with selected WBS code when result clicked', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.searchWbs).mockResolvedValue([mockWbsElements[0]]);

    render(<WbsAutocomplete value="" onChange={mockOnChange} />);

    const combobox = screen.getByRole('combobox');
    await user.click(combobox);

    const searchInput = await screen.findByPlaceholderText(/Search WBS code/);
    await user.type(searchInput, 'Astro');

    await waitFor(() => {
      expect(screen.getByText('USC0063201.1.1')).toBeInTheDocument();
    });

    const resultItem = screen.getByText('USC0063201.1.1');
    await user.click(resultItem);

    await waitFor(() => {
      expect(mockOnChange).toHaveBeenCalledWith('USC0063201.1.1', mockWbsElements[0]);
    });
  });

  it('should show clear button when value is set', () => {
    render(<WbsAutocomplete value="USC0063201.1.1" onChange={mockOnChange} />);

    const clearButton = screen.getByRole('button', { name: /clear selection/i });
    expect(clearButton).toBeInTheDocument();
  });

  it('should clear selection when clear button clicked', async () => {
    const user = userEvent.setup();
    render(<WbsAutocomplete value="USC0063201.1.1" onChange={mockOnChange} />);

    const clearButton = screen.getByRole('button', { name: /clear selection/i });
    await user.click(clearButton);

    expect(mockOnChange).toHaveBeenCalledWith('');
  });

  it('should be disabled when disabled prop is true', () => {
    render(<WbsAutocomplete value="" onChange={mockOnChange} disabled={true} />);

    const combobox = screen.getByRole('combobox');
    expect(combobox).toBeDisabled();
  });
});
