import { useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { Check, X, AlertCircle, Info } from 'lucide-react';
import { Button } from '@/components/ui/button';

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

interface InWidgetNotificationProps {
  notification: Notification | null;
   
  onDismiss: (_id: string) => void;
}

export function InWidgetNotification({ notification, onDismiss }: InWidgetNotificationProps) {
  useEffect(() => {
    if (!notification) return;

    const duration = notification.duration ?? 3000;
    const timer = setTimeout(() => {
      onDismiss(notification.id);
    }, duration);

    return () => clearTimeout(timer);
  }, [notification, onDismiss]);

  const getIcon = () => {
    switch (notification?.type) {
      case 'success':
        return <Check className="w-4 h-4 text-white" />;
      case 'error':
        return <X className="w-4 h-4 text-white" />;
      case 'info':
        return <Info className="w-4 h-4 text-white" />;
      case 'warning':
        return <AlertCircle className="w-4 h-4 text-white" />;
      default:
        return null;
    }
  };

  const getBgColor = () => {
    switch (notification?.type) {
      case 'success':
        return 'bg-green-500/95 dark:bg-green-500/95';
      case 'error':
        return 'bg-red-500/95 dark:bg-red-500/95';
      case 'info':
        return 'bg-blue-500/95 dark:bg-blue-500/95';
      case 'warning':
        return 'bg-yellow-500/95 dark:bg-yellow-500/95';
      default:
        return 'bg-gray-500/95 dark:bg-gray-500/95';
    }
  };

  return (
    <AnimatePresence>
      {notification && (
        <motion.div
          initial={{ opacity: 0, y: -20, scale: 0.95 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          exit={{ opacity: 0, y: -20, scale: 0.95 }}
          transition={{ duration: 0.2 }}
          className={`w-full backdrop-blur-xl ${getBgColor()} border border-white/30 dark:border-white/20 rounded-2xl px-4 py-3 shadow-[0_8px_32px_0_rgba(0,0,0,0.2),0_0_0_1px_rgba(255,255,255,0.8)_inset] dark:shadow-[0_8px_32px_0_rgba(0,0,0,0.4),0_0_0_1px_rgba(255,255,255,0.15)_inset]`}
        >
          <div className="flex items-center gap-3">
            <div className="flex-shrink-0">{getIcon()}</div>
            <p className="text-sm text-white flex-1">{notification.message}</p>
            {notification.action && (
              <Button
                onClick={() => {
                  notification.action?.onClick();
                  onDismiss(notification.id);
                }}
                variant="ghost"
                size="sm"
                className="text-white hover:bg-white/20 h-7 px-2 text-xs"
              >
                {notification.action.label}
              </Button>
            )}
            <button
              onClick={() => onDismiss(notification.id)}
              className="flex-shrink-0 text-white/80 hover:text-white hover:bg-white/20 rounded-full p-1 transition-all"
            >
              <X className="w-4 h-4" />
            </button>
          </div>
        </motion.div>
      )}
    </AnimatePresence>
  );
}

export type { Notification, NotificationType };
