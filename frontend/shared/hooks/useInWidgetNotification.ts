// Hook for managing in-widget notifications

import { useState, useCallback } from 'react';

type NotificationType = 'success' | 'error' | 'info' | 'warning';

interface Notification {
  id: string;
  type: NotificationType;
  message: string;
  action?: {
    label: string;
    onClick: () => void;
  };
  duration?: number;
}

export function useInWidgetNotification() {
  const [notification, setNotification] = useState<Notification | null>(null);

  const showNotification = useCallback(
    (
      type: 'success' | 'error' | 'info' | 'warning',
      message: string,
      action?: { label: string; onClick: () => void },
      duration?: number
    ) => {
      const id = Date.now().toString();
      setNotification({
        id,
        type,
        message,
        action,
        duration,
      });
    },
    []
  );

  const dismiss = useCallback((id: string) => {
    setNotification((current: Notification | null) => (current?.id === id ? null : current));
  }, []);

  return {
    notification,
    showNotification,
    dismiss,
  };
}
