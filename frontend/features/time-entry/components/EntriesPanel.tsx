import { ErrorMessage, LoadingSpinner } from '@/shared/components';
import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/shared/components/ui/dropdown-menu';
import { ScrollArea } from '@/shared/components/ui/scroll-area';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/shared/components/ui/select';
import { Sheet, SheetContent, SheetHeader, SheetTitle } from '@/shared/components/ui/sheet';
import { Calendar, Filter, MoreHorizontal, Sparkles } from 'lucide-react';
import { useEffect } from 'react';
import { useEntryStore } from '../stores';
import type { EntriesPanelProps, TimeEntry } from '../types';

export function EntriesPanel({ isOpen, onClose, showEmpty = false }: EntriesPanelProps) {
  const { entries, loading, error, fetchEntries } = useEntryStore();

  useEffect(() => {
    if (isOpen && entries.length === 0 && !loading) {
      void fetchEntries();
    }
  }, [isOpen, entries.length, loading, fetchEntries]);

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
    <Sheet open={isOpen} onOpenChange={onClose}>
      <SheetContent
        side="left"
        className="w-full sm:max-w-md backdrop-blur-3xl bg-white/90 dark:bg-gray-900/90 border-gray-200/30 dark:border-gray-700/30 p-0"
      >
        <SheetHeader className="p-5 pb-4 border-b border-gray-200/30 dark:border-gray-700/30">
          <SheetTitle className="text-gray-900 dark:text-gray-100 flex items-center justify-between">
            <span>Time Entries</span>
            <div className="flex items-center gap-2">
              <Select defaultValue="today">
                <SelectTrigger className="w-28 h-7 text-xs border-gray-200/50 dark:border-gray-700/50">
                  <Calendar className="w-3 h-3 mr-1" />
                  <SelectValue placeholder="Today" />
                </SelectTrigger>
                <SelectContent className="backdrop-blur-xl bg-white/95 dark:bg-gray-900/95">
                  <SelectItem value="today">Today</SelectItem>
                  <SelectItem value="yesterday">Yesterday</SelectItem>
                  <SelectItem value="week">This Week</SelectItem>
                  <SelectItem value="month">This Month</SelectItem>
                </SelectContent>
              </Select>
              <Button
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-gray-700 dark:text-gray-300"
              >
                <Filter className="w-3.5 h-3.5" />
              </Button>
            </div>
          </SheetTitle>
        </SheetHeader>

        {error ? (
          <ErrorMessage message={error} onRetry={() => void fetchEntries()} className="h-96" />
        ) : showEmpty || (entries.length === 0 && !loading) ? (
          <div className="flex items-center justify-center h-96 p-8">
            <div className="text-center">
              <div className="inline-flex items-center justify-center w-12 h-12 rounded-full mb-3 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20">
                <Calendar className="w-6 h-6 text-gray-400 dark:text-gray-500" />
              </div>
              <h3 className="mb-1.5 text-gray-900 dark:text-gray-100 text-sm">No entries yet</h3>
              <p className="text-xs text-gray-500 dark:text-gray-400">Start tracking your time</p>
            </div>
          </div>
        ) : loading ? (
          <div className="flex items-center justify-center h-96">
            <LoadingSpinner size="md" text="Loading entries..." />
          </div>
        ) : (
          <>
            <div className="px-5 py-3 border-b border-gray-200/30 dark:border-gray-700/30 bg-gray-50/50 dark:bg-gray-800/30">
              <div className="text-xs text-gray-500 dark:text-gray-400">
                Total today: <span className="text-gray-700 dark:text-gray-300">4h 30m</span>
              </div>
            </div>

            <ScrollArea className="h-[calc(100vh-180px)]">
              <div className="p-3">
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
                          {entry.status === 'suggested' && (
                            <DropdownMenuItem>
                              <Sparkles className="w-3.5 h-3.5 mr-2" />
                              Accept Suggestion
                            </DropdownMenuItem>
                          )}
                          <DropdownMenuItem>Edit</DropdownMenuItem>
                          <DropdownMenuItem className="text-red-600 dark:text-red-400">
                            Delete
                          </DropdownMenuItem>
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
          </>
        )}
      </SheetContent>
    </Sheet>
  );
}
