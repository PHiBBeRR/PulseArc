import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/shared/components/ui/dropdown-menu';
import { ScrollArea } from '@/shared/components/ui/scroll-area';
import { ErrorMessage, LoadingSpinner } from '@/shared/components';
import { Calendar, MoreHorizontal, Sparkles } from 'lucide-react';
import { useEffect } from 'react';
import { useEntryStore } from '../stores';
import type { TimeEntry } from '../types';

export function CompactEntries() {
  const { entries, loading, error, fetchEntries } = useEntryStore();

  useEffect(() => {
    if (entries.length === 0 && !loading) {
      void fetchEntries();
    }
  }, [entries.length, loading, fetchEntries]);

  const getStatusBadge = (entry: TimeEntry) => {
    if (entry.status === 'suggested') {
      return (
        <Badge
          variant="outline"
          className="border-blue-500/50 text-blue-600 dark:text-blue-400 bg-blue-500/10 text-xs"
        >
          <Sparkles className="w-2.5 h-2.5 mr-1" />
          {entry.confidence}%
        </Badge>
      );
    }
    if (entry.status === 'approved') {
      return (
        <Badge
          variant="outline"
          className="border-green-500/50 text-green-600 dark:text-green-400 bg-green-500/10 text-xs"
        >
          ✓
        </Badge>
      );
    }
    return (
      <Badge
        variant="outline"
        className="border-yellow-500/50 text-yellow-600 dark:text-yellow-400 bg-yellow-500/10 text-xs"
      >
        !
      </Badge>
    );
  };

  return (
    <div className="backdrop-blur-2xl bg-white/80 dark:bg-gray-900/80 border border-gray-200/30 dark:border-gray-700/30 rounded-3xl overflow-hidden">
      <div className="p-4 border-b border-gray-200/30 dark:border-gray-700/30">
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-gray-900 dark:text-gray-100">Today's Entries</h3>
          <Button variant="ghost" size="sm" className="h-7 text-xs">
            <Calendar className="w-3 h-3 mr-1" />
            Filter
          </Button>
        </div>
        <div className="text-xs text-gray-500 dark:text-gray-400">Total: 4h 30m</div>
      </div>

      {error ? (
        <ErrorMessage message={error} onRetry={() => void fetchEntries()} className="h-80" />
      ) : loading ? (
        <div className="h-80 flex items-center justify-center">
          <LoadingSpinner size="md" text="Loading entries..." />
        </div>
      ) : entries.length === 0 ? (
        <div className="h-80 flex items-center justify-center p-6">
          <div className="text-center">
            <p className="text-sm text-gray-500 dark:text-gray-400">No entries yet</p>
          </div>
        </div>
      ) : (
        <ScrollArea className="h-80">
          <div className="p-2">
            {entries.map((entry) => (
              <div
                key={entry.id}
                className="p-3 rounded-2xl hover:bg-gray-100/50 dark:hover:bg-gray-800/50 transition-colors mb-1"
              >
                <div className="flex items-start justify-between mb-2">
                  <div className="flex-1 min-w-0">
                    <div className="text-sm text-gray-900 dark:text-gray-100 truncate mb-0.5">
                      {entry.task}
                    </div>
                    <div className="text-xs text-gray-500 dark:text-gray-400 truncate">
                      {entry.project}
                    </div>
                  </div>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button
                        variant="ghost"
                        size="icon"
                        className="h-6 w-6 ml-2 flex-shrink-0 text-gray-700 dark:text-gray-300"
                      >
                        <MoreHorizontal className="w-3.5 h-3.5" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent
                      align="end"
                      className="backdrop-blur-xl bg-white/95 dark:bg-gray-900/95"
                    >
                      <DropdownMenuItem>Edit</DropdownMenuItem>
                      <DropdownMenuItem>Delete</DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
                <div className="flex items-center justify-between text-xs">
                  <span className="text-gray-500 dark:text-gray-400">{entry.time}</span>
                  <div className="flex items-center gap-2">
                    <span className="text-gray-600 dark:text-gray-300">{entry.duration}</span>
                    {getStatusBadge(entry)}
                  </div>
                </div>
              </div>
            ))}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
