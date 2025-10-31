// Error message component
import { Button } from '@/shared/components/ui/button';
import { cn } from '@/shared/components/ui/utils';
import { AlertCircle } from 'lucide-react';

export interface ErrorMessageProps {
  title?: string;
  message: string;
  onRetry?: () => void;
  className?: string;
}

export function ErrorMessage({ title = 'Error', message, onRetry, className }: ErrorMessageProps) {
  return (
    <div className={cn('flex flex-col items-center justify-center gap-3 p-6', className)}>
      <div className="inline-flex items-center justify-center w-12 h-12 rounded-full bg-red-100 dark:bg-red-900/20 mb-1">
        <AlertCircle className="w-6 h-6 text-red-600 dark:text-red-400" />
      </div>
      <div className="text-center">
        <h3 className="mb-1 text-gray-900 dark:text-gray-100 text-sm font-medium">{title}</h3>
        <p className="text-xs text-gray-500 dark:text-gray-400 max-w-xs">{message}</p>
      </div>
      {onRetry && (
        <Button
          onClick={onRetry}
          size="sm"
          className="mt-2 backdrop-blur-xl bg-white/20 hover:bg-white/30 dark:bg-white/10 dark:hover:bg-white/15 text-gray-900 dark:text-white border border-white/30 dark:border-white/20 h-8 text-xs"
        >
          Try Again
        </Button>
      )}
    </div>
  );
}
