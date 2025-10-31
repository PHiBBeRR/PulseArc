import { Button } from '@/components/ui/button';
import { AlertCircle, RefreshCw, WifiOff } from 'lucide-react';

interface CompactErrorAlertProps {
  type: 'network' | 'sync';
  onRetry?: () => void;
}

export function CompactErrorAlert({ type, onRetry }: CompactErrorAlertProps) {
  const isNetworkError = type === 'network';

  return (
    <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 mb-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
      <div className="flex items-start gap-2">
        {isNetworkError ? (
          <WifiOff className="w-4 h-4 text-gray-700 dark:text-gray-300 flex-shrink-0 mt-0.5" />
        ) : (
          <AlertCircle className="w-4 h-4 text-gray-700 dark:text-gray-300 flex-shrink-0 mt-0.5" />
        )}
        <div className="flex-1 min-w-0">
          <div className="text-xs text-gray-900 dark:text-gray-100 mb-0.5">
            {isNetworkError ? 'Network Error' : 'Sync Failed'}
          </div>
          <p className="text-xs text-gray-600 dark:text-gray-400">
            {isNetworkError ? 'Check your connection' : 'Changes saved locally'}
          </p>
        </div>
        {onRetry && (
          <Button onClick={onRetry} variant="ghost" size="sm" className="h-7 px-2 text-xs">
            <RefreshCw className="w-3 h-3 mr-1" />
            Retry
          </Button>
        )}
      </div>
    </div>
  );
}
