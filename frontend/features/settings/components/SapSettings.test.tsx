// FEATURE-020 Phase 2: SAP Settings Component Tests
// Test coverage for SAP settings UI

import { describe, it, beforeEach, afterEach, vi, expect } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { SapSettings } from './SapSettings';
import { SapService } from '../services/sapService';

// Mock SapService
vi.mock('../services/sapService', () => ({
  SapService: {
    isAuthenticated: vi.fn(),
    startLogin: vi.fn(),
    logout: vi.fn(),
    completeLogin: vi.fn(),
  },
}));

// Mock sonner toast
vi.mock('sonner', () => ({
  toast: {
    info: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
  },
}));

// Mock OutboxStatusComponent
vi.mock('@/features/timer/components/OutboxStatus', () => ({
  OutboxStatusComponent: () => <div data-testid="outbox-status">Outbox Status</div>,
}));

describe('SapSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // Default to disconnected state
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should render connection status indicator', async () => {
    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Connection Status')).toBeInTheDocument();
    });
  });

  it('should display "Disconnected" badge when not authenticated', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Disconnected')).toBeInTheDocument();
    });
  });

  it('should display "Connected" badge when authenticated', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(true);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Connected')).toBeInTheDocument();
    });
  });

  it('should show "Connect to SAP" button when disconnected', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Connect to SAP')).toBeInTheDocument();
    });
  });

  it('should show "Disconnect" button when connected', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(true);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Disconnect')).toBeInTheDocument();
    });
  });

  it('should call SapService.startLogin when Connect button clicked', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);
    vi.mocked(SapService.startLogin).mockResolvedValue('https://auth0.example.com/authorize');

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Connect to SAP')).toBeInTheDocument();
    });

    const connectButton = screen.getByText('Connect to SAP');
    await user.click(connectButton);

    await waitFor(() => {
      expect(SapService.startLogin).toHaveBeenCalledTimes(1);
    });
  });

  it('should display loading state during login', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);
    vi.mocked(SapService.startLogin).mockImplementation(
      () => new Promise((resolve) => setTimeout(() => resolve('https://auth0.example.com'), 1000))
    );

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Connect to SAP')).toBeInTheDocument();
    });

    const connectButton = screen.getByText('Connect to SAP');
    await user.click(connectButton);

    // Should show connecting state
    await waitFor(() => {
      expect(screen.getByText('Connecting...')).toBeInTheDocument();
    });
  });

  it('should call SapService.logout when Disconnect button clicked', async () => {
    const user = userEvent.setup();
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(true);
    vi.mocked(SapService.logout).mockResolvedValue(undefined);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('Disconnect')).toBeInTheDocument();
    });

    const disconnectButton = screen.getByText('Disconnect');
    await user.click(disconnectButton);

    await waitFor(() => {
      expect(SapService.logout).toHaveBeenCalledTimes(1);
    });
  });

  it('should display OutboxStatus component when authenticated', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(true);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByTestId('outbox-status')).toBeInTheDocument();
    });
  });

  it('should hide OutboxStatus component when not authenticated', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.queryByTestId('outbox-status')).not.toBeInTheDocument();
    });
  });

  it('should display "How it works" info section when disconnected', async () => {
    vi.mocked(SapService.isAuthenticated).mockResolvedValue(false);

    render(<SapSettings />);

    await waitFor(() => {
      expect(screen.getByText('How it works')).toBeInTheDocument();
      expect(screen.getByText(/Securely authenticate with Auth0 OAuth/)).toBeInTheDocument();
    });
  });
});
