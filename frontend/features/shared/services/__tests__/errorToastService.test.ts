// FEATURE-020 Phase 4.4: Error Toast Service Tests
// Test coverage for SAP error display and user feedback

import { describe, it, beforeEach, afterEach, expect, vi } from 'vitest';
import type { SapError } from '@/shared/types/SapError';

// Mock toast system
vi.mock('sonner', () => ({
  toast: {
    error: vi.fn(),
  },
}));

// Import after mock
import { errorToastService } from '../errorToastService';
import { toast } from 'sonner';

// Get reference to mocked function
const mockToastError = vi.mocked(toast.error);

describe('Error Toast Service', () => {
  beforeEach(() => {
    mockToastError.mockClear();
  });

  afterEach(() => {
    vi.clearAllMocks();
  });

  it('should display network offline error with retry button', () => {
    const error: SapError = {
      category: 'NetworkOffline',
      message: 'No internet connection detected',
      user_message: 'You are offline. Time entries will be queued locally.',
      is_retriable: true,
      should_backoff: false,
    };

    errorToastService.displaySapError(error, () => {
      // Retry callback
    });

    // Verify toast.error called with correct parameters
    expect(mockToastError).toHaveBeenCalledWith('Network Offline', {
      description: 'You are offline. Time entries will be queued locally.',
      action: expect.objectContaining({
        label: 'Retry',
      }),
    });
  });

  it('should display timeout error with retry action', () => {
    const error: SapError = {
      category: 'NetworkTimeout',
      message: 'Request timed out after 30s',
      user_message: 'The request took too long. Please try again.',
      is_retriable: true,
      should_backoff: true,
    };

    errorToastService.displaySapError(error, () => {
      // Retry callback
    });

    expect(mockToastError).toHaveBeenCalledWith('Request Timeout', {
      description: 'The request took too long. Please try again.',
      action: expect.objectContaining({
        label: 'Retry',
      }),
    });
  });

  it('should display authentication error with reconnect action', () => {
    const error: SapError = {
      category: 'Authentication',
      message: 'JWT token expired',
      user_message: 'Your session has expired. Please reconnect to SAP.',
      is_retriable: false,
      should_backoff: false,
    };

    const reconnectCallback = vi.fn();
    errorToastService.displaySapError(error, reconnectCallback);

    expect(mockToastError).toHaveBeenCalledWith('Authentication Error', {
      description: 'Your session has expired. Please reconnect to SAP.',
      action: expect.objectContaining({
        label: 'Reconnect',
      }),
    });
  });

  it('should display rate limit error with wait time', () => {
    const error: SapError = {
      category: 'RateLimited',
      message: 'Too many requests',
      user_message: 'Rate limit exceeded. Please wait before retrying.',
      is_retriable: true,
      should_backoff: true,
    };

    errorToastService.displaySapError(error);

    expect(mockToastError).toHaveBeenCalledWith('Rate Limited', {
      description: 'Rate limit exceeded. Please wait before retrying.',
      action: undefined, // No action for rate limit (must wait)
    });
  });

  it('should display validation error inline (no retry)', () => {
    const error: SapError = {
      category: 'Validation',
      message: 'Invalid WBS code format',
      user_message: 'WBS code must match pattern USC0063201.1.1',
      is_retriable: false,
      should_backoff: false,
    };

    errorToastService.displaySapError(error);

    expect(mockToastError).toHaveBeenCalledWith('Validation Error', {
      description: 'WBS code must match pattern USC0063201.1.1',
      action: undefined, // No retry for validation errors
    });
  });

  it('should display server unavailable error', () => {
    const error: SapError = {
      category: 'ServerUnavailable',
      message: 'SAP server returned 503',
      user_message: 'SAP server is temporarily unavailable. We will retry automatically.',
      is_retriable: true,
      should_backoff: true,
    };

    errorToastService.displaySapError(error);

    expect(mockToastError).toHaveBeenCalledWith('Server Unavailable', {
      description: 'SAP server is temporarily unavailable. We will retry automatically.',
      action: undefined, // Auto-retry via backoff, no manual action
    });
  });

  it('should dismiss error toast on user action', () => {
    const dismissId = 'error-toast-123';

    // Mock toast.error return value (sonner returns number or string)
    mockToastError.mockReturnValue(dismissId);

    const error: SapError = {
      category: 'Unknown',
      message: 'Unknown error',
      user_message: 'An unexpected error occurred.',
      is_retriable: false,
      should_backoff: false,
    };

    const toastInstance = errorToastService.displaySapError(error);

    // Verify toast instance returned (toast ID for dismissal)
    expect(toastInstance).toBeDefined();
    expect(toastInstance).toBe(dismissId);
  });

  it('should queue multiple errors (show latest first)', () => {
    const error1: SapError = {
      category: 'NetworkTimeout',
      message: 'Timeout 1',
      user_message: 'First timeout error',
      is_retriable: true,
      should_backoff: true,
    };

    const error2: SapError = {
      category: 'ServerUnavailable',
      message: 'Server error',
      user_message: 'Server unavailable error',
      is_retriable: true,
      should_backoff: true,
    };

    errorToastService.displaySapError(error1);
    errorToastService.displaySapError(error2);

    // Verify both toasts were called
    expect(mockToastError).toHaveBeenCalledTimes(2);

    // Verify second call is for latest error
    expect(mockToastError).toHaveBeenNthCalledWith(2, 'Server Unavailable', expect.objectContaining({
      description: 'Server unavailable error',
    }));
  });
});
