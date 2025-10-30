import { Clock, Plus } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface CompactEmptyStateProps {
  onCreateEntry?: () => void;
}

export function CompactEmptyState({ onCreateEntry }: CompactEmptyStateProps) {
  return (
    <div className="backdrop-blur-2xl bg-white/80 dark:bg-gray-900/80 border border-gray-200/30 dark:border-gray-700/30 rounded-3xl p-10 text-center">
      <div className="inline-flex items-center justify-center w-12 h-12 rounded-full bg-gray-100 dark:bg-gray-800 mb-3">
        <Clock className="w-6 h-6 text-gray-400 dark:text-gray-500" />
      </div>

      <h3 className="mb-1.5 text-gray-900 dark:text-gray-100 text-sm">No entries yet</h3>
      <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">Start tracking your time</p>

      <Button onClick={onCreateEntry} size="sm" className="bg-blue-500 hover:bg-blue-600 text-white h-8 text-xs">
        <Plus className="w-3 h-3 mr-2" />
        Create Entry
      </Button>
    </div>
  );
}
