// Loading spinner component
import { Loader2 } from 'lucide-react';
import { cn } from '@/components/ui/utils';

export interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  className?: string;
  text?: string;
}

export function LoadingSpinner({ size = 'md', className, text }: LoadingSpinnerProps) {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-6 h-6',
    lg: 'w-8 h-8',
  };

  return (
    <div className={cn('flex flex-col items-center justify-center gap-2', className)}>
      <Loader2 className={cn('animate-spin text-gray-500 dark:text-gray-400', sizeClasses[size])} />
      {text && <p className="text-xs text-gray-500 dark:text-gray-400">{text}</p>}
    </div>
  );
}
