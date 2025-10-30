// FEATURE-020 Phase 4.4: Error Toast Service
// Displays user-friendly error messages for SAP operations

import { toast } from 'sonner';
import type { SapError, SapErrorCategory } from '@/shared/types/SapError';

type ToastAction = {
  label: string;
  onClick: () => void;
};

/**
 * Maps SapErrorCategory to user-friendly title
 */
function getCategoryTitle(category: SapErrorCategory): string {
  switch (category) {
    case 'NetworkOffline':
      return 'Network Offline';
    case 'NetworkTimeout':
      return 'Request Timeout';
    case 'ServerUnavailable':
      return 'Server Unavailable';
    case 'Authentication':
      return 'Authentication Error';
    case 'RateLimited':
      return 'Rate Limited';
    case 'Validation':
      return 'Validation Error';
    case 'Unknown':
      return 'Error';
  }
}

/**
 * Determines appropriate action button for error category
 */
function getErrorAction(
  error: SapError,
  retryCallback?: () => void
): ToastAction | undefined {
  // Authentication errors should show "Reconnect" instead of "Retry"
  if (error.category === 'Authentication' && retryCallback) {
    return {
      label: 'Reconnect',
      onClick: retryCallback,
    };
  }

  // Rate limited and server unavailable errors should not have manual retry
  // (auto-retry via backoff handles these)
  if (error.category === 'RateLimited' || error.category === 'ServerUnavailable') {
    return undefined;
  }

  // Validation errors cannot be retried (user must fix input)
  if (error.category === 'Validation') {
    return undefined;
  }

  // Retriable errors with callback show retry button
  if (error.is_retriable && retryCallback) {
    return {
      label: 'Retry',
      onClick: retryCallback,
    };
  }

  return undefined;
}

/**
 * Error Toast Service
 *
 * Displays SAP errors with appropriate styling and actions
 */
export const errorToastService = {
  /**
   * Display SAP error as toast notification
   *
   * @param error - SAP error to display
   * @param retryCallback - Optional callback for retry action
   * @returns Toast instance with dismiss method
   */
  displaySapError(error: SapError, retryCallback?: () => void) {
    const title = getCategoryTitle(error.category);
    const action = getErrorAction(error, retryCallback);

    return toast.error(title, {
      description: error.user_message,
      action: action
        ? {
            label: action.label,
            onClick: action.onClick,
          }
        : undefined,
    });
  },
};
