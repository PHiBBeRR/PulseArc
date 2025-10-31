import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/shared/components/ui/dropdown-menu';
import { Input } from '@/shared/components/ui/input';
import { ScrollArea } from '@/shared/components/ui/scroll-area';
import { Tabs, TabsList, TabsTrigger } from '@/shared/components/ui/tabs';
import { Textarea } from '@/shared/components/ui/textarea';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/shared/components/ui/tooltip';
import { EntriesListSkeleton, ErrorMessage, InWidgetNotification } from '@/shared/components';
import { useInWidgetNotification } from '@/shared/hooks';
import { haptic, quickConfetti } from '@/shared/utils';
import { listen } from '@tauri-apps/api/event';
import { AnimatePresence, motion } from 'framer-motion';
import {
  ArrowLeft,
  Calendar,
  Check,
  ChevronLeft,
  ChevronRight,
  Edit2,
  GripHorizontal,
  Minus,
  MoreHorizontal,
  Plus,
  Sparkles,
  Trash2,
} from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { entryService } from '../services/entryService';
import { useEntryStore } from '../stores';
import type { EntriesViewProps, TimeEntry } from '../types';

// Helper functions for duration parsing and formatting
const parseDuration = (duration: string): number => {
  if (!duration) return 0;
  const hourMatch = duration.match(/(\d+)\s*h/);
  const minuteMatch = duration.match(/(\d+)\s*m/);
  const hours = hourMatch?.[1] ? parseInt(hourMatch[1]) : 0;
  const minutes = minuteMatch?.[1] ? parseInt(minuteMatch[1]) : 0;
  return hours * 60 + minutes;
};

const formatDuration = (minutes: number): string => {
  if (minutes === 0) return '0m';
  const hours = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (hours === 0) return `${mins}m`;
  if (mins === 0) return `${hours}h`;
  return `${hours}h ${mins}m`;
};

export function EntriesView({
  onBack,
  onQuickEntry,
  showEmpty = false,
  onNotificationTriggerReady,
  onViewModeChange,
}: EntriesViewProps) {
  // Zustand store
  const { entries, loading, error, fetchEntries, updateEntry, deleteEntry, optimisticAddEntry } =
    useEntryStore();

  // Local UI state
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [selectedEntry, setSelectedEntry] = useState<TimeEntry | null>(null);
  const [editedProject, setEditedProject] = useState('');
  const [editedTask, setEditedTask] = useState('');
  const [editedDuration, setEditedDuration] = useState('');
  const [editedDescription, setEditedDescription] = useState('');
  const [deletedEntry, setDeletedEntry] = useState<TimeEntry | null>(null);
  const [selectedDate, setSelectedDate] = useState<Date>(new Date());
  const [viewMode, setViewMode] = useState<'day' | 'week'>('day');
  const { notification, showNotification, dismiss } = useInWidgetNotification();

  // Convert selected date to time filter string
  const getTimeFilterForDate = useCallback((date: Date): string => {
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const compareDate = new Date(date);
    compareDate.setHours(0, 0, 0, 0);

    const diffTime = today.getTime() - compareDate.getTime();
    const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24));

    if (diffDays === 0) return 'today';
    if (diffDays === 1) return 'yesterday';
    if (diffDays >= 2 && diffDays <= 7) return 'week';
    return 'month';
  }, []);

  // Fetch entries on mount and when date or view mode changes
  useEffect(() => {
    // In week view, always fetch 'week' to get full week data
    const timeFilter = viewMode === 'week' ? 'week' : getTimeFilterForDate(selectedDate);
    void fetchEntries(timeFilter);
  }, [fetchEntries, selectedDate, viewMode, getTimeFilterForDate]);

  // Expose notification trigger to parent
  useEffect(() => {
    onNotificationTriggerReady?.(showNotification);
  }, [showNotification, onNotificationTriggerReady]);

  // Notify parent when view mode changes (for window resizing)
  useEffect(() => {
    onViewModeChange?.(viewMode);
  }, [viewMode, onViewModeChange]);

  // Dynamically resize window based on view mode (like MainTimer does)
  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        const currentWindow = getCurrentWindow();

        let targetWidth = 680;
        let targetHeight = 720;

        if (viewMode === 'week') {
          targetWidth = 790;
          targetHeight = 435;
        } else {
          // day view
          targetWidth = 680;
          targetHeight = 620;
        }

        console.log('[EntriesView] Resizing window:', { viewMode, targetWidth, targetHeight });
        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));

        // Set resizable based on view mode
        if (viewMode === 'week') {
          await currentWindow.setResizable(false);
          // Set min/max to lock the size
          await currentWindow.setMinSize(new LogicalSize(targetWidth, targetHeight));
          await currentWindow.setMaxSize(new LogicalSize(targetWidth, targetHeight));
        } else {
          await currentWindow.setResizable(true);
          // Allow resizing for day view
          await currentWindow.setMinSize(new LogicalSize(targetWidth - 100, targetHeight - 100));
          await currentWindow.setMaxSize(new LogicalSize(targetWidth + 400, targetHeight + 400));
        }
      } catch (error) {
        console.error('[EntriesView] Failed to resize window:', error);
      }
    };

    void resizeWindow();
  }, [viewMode]);

  // Real-time updates: Listen for outbox changes (when suggestions are accepted)
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        // Listen for outbox updates (suggestions accepted/dismissed)
        unlisten = await listen('outbox-updated', () => {
          // Refresh entries when outbox changes
          const timeFilter = viewMode === 'week' ? 'week' : getTimeFilterForDate(selectedDate);
          void fetchEntries(timeFilter);
        });
      } catch (error) {
        console.error('Failed to setup outbox listener:', error);
      }
    };

    void setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [fetchEntries, selectedDate, getTimeFilterForDate, viewMode]);

  // Calculate total duration (backend already filtered by timeFilter)
  const totalDuration = entries.reduce((total, entry) => {
    return total + parseDuration(entry.duration);
  }, 0);

  const handleEditStart = (entry: TimeEntry) => {
    setSelectedEntry(entry);
    setEditedProject(entry.project);
    setEditedTask(entry.task);
    setEditedDuration(entry.duration);
    setEditedDescription('');
    setEditModalOpen(true);
  };

  const handleEditSave = async () => {
    if (!selectedEntry) return;

    // Update entry via store
    const result = await updateEntry(selectedEntry.id, {
      project: editedProject,
      task: editedTask,
      duration: editedDuration,
    });

    setEditModalOpen(false);
    setSelectedEntry(null);

    if (result) {
      showNotification('success', `Updated ${editedTask}`);
    } else {
      showNotification('error', 'Failed to update entry');
    }
  };

  const handleDelete = async () => {
    if (!selectedEntry) return;

    // Store the deleted entry for undo
    setDeletedEntry(selectedEntry);

    // Delete entry via store
    const success = await deleteEntry(selectedEntry.id);

    setEditModalOpen(false);
    setSelectedEntry(null);

    if (success) {
      // Show notification with undo action
      showNotification(
        'success',
        `Deleted ${selectedEntry.task}`,
        {
          label: 'Undo',
          onClick: () => handleUndoDelete(),
        },
        5000
      );
      haptic.medium();
    } else {
      showNotification('error', 'Failed to delete entry');
    }
  };

  const handleUndoDelete = () => {
    if (!deletedEntry) return;

    showNotification('success', 'Entry restored');
    haptic.success();

    // Restore the deleted entry via optimistic add
    optimisticAddEntry(deletedEntry);
    setDeletedEntry(null);
  };

  const handleAcceptSuggestion = async (entry: TimeEntry) => {
    // Update entry status to approved via store
    await updateEntry(entry.id, { status: 'approved' } as Partial<TimeEntry>);

    showNotification('success', 'Suggestion accepted');
    haptic.success();
    quickConfetti();
  };

  // Date navigation
  const navigateDate = (direction: 'prev' | 'next') => {
    const newDate = new Date(selectedDate);
    const delta = viewMode === 'week' ? 7 : 1;
    newDate.setDate(newDate.getDate() + (direction === 'next' ? delta : -delta));
    setSelectedDate(newDate);
  };

  const formatDate = (date: Date) => {
    if (viewMode === 'week') {
      // Get Monday of the week containing the selected date
      const monday = new Date(date);
      const day = monday.getDay();
      const diff = monday.getDate() - day + (day === 0 ? -6 : 1); // Adjust when day is Sunday
      monday.setDate(diff);

      // Get Sunday (end of week)
      const sunday = new Date(monday);
      sunday.setDate(monday.getDate() + 6);

      const formatShort = (d: Date) =>
        d.toLocaleDateString('en-US', {
          month: 'short',
          day: 'numeric',
        });

      return `${formatShort(monday)} - ${formatShort(sunday)}`;
    }

    // Day view formatting
    const today = new Date();
    today.setHours(0, 0, 0, 0);
    const compareDate = new Date(date);
    compareDate.setHours(0, 0, 0, 0);

    if (today.getTime() === compareDate.getTime()) {
      return 'Today';
    }

    const yesterday = new Date(today);
    yesterday.setDate(yesterday.getDate() - 1);
    if (yesterday.getTime() === compareDate.getTime()) {
      return 'Yesterday';
    }

    return date.toLocaleDateString('en-US', {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
      year: date.getFullYear() !== today.getFullYear() ? 'numeric' : undefined,
    });
  };

  // Get week data aggregated by project
  const getWeekData = useCallback(() => {
    // Get Monday of the week containing the selected date
    const monday = new Date(selectedDate);
    const day = monday.getDay();
    const diff = monday.getDate() - day + (day === 0 ? -6 : 1);
    monday.setDate(diff);
    monday.setHours(0, 0, 0, 0);

    // Create array of dates for the week (Mon-Sun)
    const weekDates = Array.from({ length: 7 }, (_, i) => {
      const date = new Date(monday);
      date.setDate(monday.getDate() + i);
      return date;
    });

    // Group entries by project/WBS
    const projectMap = new Map<
      string,
      {
        project: string;
        wbsCode?: string;
        days: number[]; // Minutes for each day (Mon-Sun)
      }
    >();

    entries.forEach((entry) => {
      // Parse the shortDate (MM/DD/YYYY format) to get the entry date
      if (!entry.shortDate) return;

      const [month, day, year] = entry.shortDate.split('/').map(Number);
      const entryDate = new Date(year, month - 1, day);
      entryDate.setHours(0, 0, 0, 0);

      // Find which day of the week this entry belongs to
      const dayIndex = weekDates.findIndex(
        (weekDate) => weekDate.getTime() === entryDate.getTime()
      );

      if (dayIndex === -1) return; // Entry not in this week

      const key = `${entry.project}|${entry.wbsCode ?? ''}`;
      if (!projectMap.has(key)) {
        projectMap.set(key, {
          project: entry.project,
          wbsCode: entry.wbsCode,
          days: [0, 0, 0, 0, 0, 0, 0],
        });
      }

      const projectData = projectMap.get(key);
      if (projectData) {
        projectData.days[dayIndex] += parseDuration(entry.duration);
      }
    });

    return { weekDates, projectData: Array.from(projectMap.values()) };
  }, [selectedDate, entries]);

  const getStatusBadge = (entry: TimeEntry) => {
    if (entry.status === 'suggested') {
      return (
        <Tooltip>
          <TooltipTrigger>
            <Badge
              variant="outline"
              className="border-blue-500/30 dark:border-blue-400/30 text-blue-600 dark:text-blue-400 backdrop-blur-xl bg-blue-500/10 dark:bg-blue-400/10 text-xs shadow-[0_0_0_1px_rgba(59,130,246,0.2)_inset] cursor-help"
            >
              <Sparkles className="w-2.5 h-2.5 mr-1" />
              {entry.confidence}%
            </Badge>
          </TooltipTrigger>
          <TooltipContent
            side="top"
            className="backdrop-blur-xl bg-black/80 border-white/20 text-white text-xs max-w-[200px]"
          >
            {entryService.getMLExplanation(entry)}
          </TooltipContent>
        </Tooltip>
      );
    }
    if (entry.status === 'approved') {
      return (
        <span className="inline-flex items-center justify-center h-4 w-4 rounded-full bg-green-500/20 text-green-600 dark:text-green-400 flex-shrink-0">
          <Check className="w-3 h-3" />
        </span>
      );
    }
    return (
      <Badge
        variant="outline"
        className="border-slate-400/30 dark:border-slate-500/30 text-slate-600 dark:text-slate-400 backdrop-blur-xl bg-slate-400/10 dark:bg-slate-500/10 text-xs shadow-[0_0_0_1px_rgba(148,163,184,0.2)_inset]"
      >
        !
      </Badge>
    );
  };

  return (
    <TooltipProvider delayDuration={300}>
      <div className="relative backdrop-blur-[24px] overflow-hidden h-full flex flex-col">
        {/* In-widget notification */}
        <InWidgetNotification notification={notification} onDismiss={dismiss} />

        {/* Drag handle bar */}
        <div
          data-tauri-drag-region
          className="flex items-center justify-center py-2 cursor-move rounded-t-[40px] select-none"
        >
          <GripHorizontal className="w-8 h-3 text-gray-400/50 dark:text-gray-500/50 pointer-events-none" />
        </div>

        {/* Header */}
        <div className="p-4 pt-2 border-b border-white/10 dark:border-white/5">
          <div className="flex items-center justify-between mb-3">
            <div className="flex items-center gap-2">
              <Button
                variant="ghost"
                size="icon"
                onClick={onBack}
                className="h-7 w-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
              >
                <ArrowLeft className="w-3.5 h-3.5" />
              </Button>
              <h2 className="text-sm text-gray-900 dark:text-gray-100">Time Entries</h2>
            </div>

            <div className="flex items-center gap-2">
              {/* View Mode Tabs */}
              <Tabs
                value={viewMode}
                onValueChange={(v) => setViewMode(v as 'day' | 'week')}
                className="h-7"
              >
                <TabsList className="h-7 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20">
                  <TabsTrigger value="day" className="h-6 text-xs px-2">
                    Day
                  </TabsTrigger>
                  <TabsTrigger value="week" className="h-6 text-xs px-2">
                    Week
                  </TabsTrigger>
                </TabsList>
              </Tabs>

              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={onQuickEntry}
                    className="h-7 w-7 hover:bg-white/20 dark:hover:bg-white/10"
                  >
                    <Plus className="w-3.5 h-3.5" />
                  </Button>
                </TooltipTrigger>
                <TooltipContent
                  side="bottom"
                  className="backdrop-blur-xl bg-black/80 border-white/20 text-white text-xs"
                >
                  New Entry · ⌘N
                </TooltipContent>
              </Tooltip>
            </div>
          </div>

          {/* Date Navigation */}
          <div className="flex items-center justify-between gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => navigateDate('prev')}
              className="h-6 w-6 flex-shrink-0 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
            </Button>

            <div className="flex flex-col items-center gap-0.5 min-w-0">
              <div className="text-sm font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap">
                {formatDate(selectedDate)}
              </div>
              <div className="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                {formatDuration(totalDuration)} · {entries.length}{' '}
                {entries.length === 1 ? 'entry' : 'entries'}
              </div>
            </div>

            <Button
              variant="ghost"
              size="icon"
              onClick={() => navigateDate('next')}
              className="h-6 w-6 flex-shrink-0 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
            >
              <ChevronRight className="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        {error ? (
          <ErrorMessage
            message={error}
            onRetry={() => void fetchEntries(getTimeFilterForDate(selectedDate))}
            className="h-96"
          />
        ) : showEmpty || (entries.length === 0 && !loading) ? (
          <div className="flex items-center justify-center h-96 p-8">
            <div className="text-center">
              <div className="inline-flex items-center justify-center w-12 h-12 rounded-full mb-3 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20">
                <Calendar className="w-6 h-6 text-gray-400 dark:text-gray-500" />
              </div>
              <h3 className="mb-1.5 text-gray-900 dark:text-gray-100 text-sm">No entries yet</h3>
              <p className="text-xs text-gray-500 dark:text-gray-400 mb-4">
                Start tracking your time
              </p>
              <Button
                onClick={onQuickEntry}
                size="sm"
                className="backdrop-blur-xl bg-white/20 hover:bg-white/30 dark:bg-white/10 dark:hover:bg-white/15 text-gray-900 dark:text-white border border-white/30 dark:border-white/20 h-8 text-xs shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]"
              >
                <Plus className="w-3 h-3 mr-2" />
                Create Entry
              </Button>
            </div>
          </div>
        ) : loading ? (
          <div className="p-3">
            <EntriesListSkeleton />
          </div>
        ) : viewMode === 'week' ? (
          <div className="rounded-b-[2.5rem] overflow-hidden">
            <div className="p-3">
              {/* Week View Table */}
              {(() => {
                const { weekDates, projectData } = getWeekData();
                const dayNames = ['Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat', 'Sun'];
                const dayTotals = [0, 0, 0, 0, 0, 0, 0];

                // Calculate column totals
                projectData.forEach((project) => {
                  project.days.forEach((minutes, i) => {
                    dayTotals[i] += minutes;
                  });
                });

                return (
                  <div className="backdrop-blur-xl bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-2xl overflow-hidden">
                    <div className="overflow-x-auto">
                      <table className="w-full text-xs">
                        <thead>
                          <tr className="border-b border-white/20 dark:border-white/10">
                            <th className="text-left p-2 text-gray-900 dark:text-gray-100 font-medium sticky left-0 bg-white/10 dark:bg-white/5 backdrop-blur-xl min-w-[140px]">
                              Project / WBS
                            </th>
                            {weekDates.map((date, i) => (
                              <th
                                key={i}
                                className="text-center p-2 text-gray-900 dark:text-gray-100 font-medium min-w-[70px]"
                              >
                                <div>{dayNames[i]}</div>
                                <div className="text-[10px] text-gray-500 dark:text-gray-400 font-normal">
                                  {date.toLocaleDateString('en-US', {
                                    month: 'numeric',
                                    day: 'numeric',
                                  })}
                                </div>
                              </th>
                            ))}
                            <th className="text-right p-2 text-gray-900 dark:text-gray-100 font-medium min-w-[70px]">
                              Total
                            </th>
                          </tr>
                        </thead>
                        <tbody>
                          {projectData.map((project, idx) => {
                            const rowTotal = project.days.reduce((sum, mins) => sum + mins, 0);
                            return (
                              <tr
                                key={idx}
                                className="border-b border-white/10 dark:border-white/5 hover:bg-white/10 dark:hover:bg-white/5"
                              >
                                <td className="p-2 text-gray-900 dark:text-gray-50 sticky left-0 bg-white/10 dark:bg-white/5 backdrop-blur-xl">
                                  <div className="truncate">
                                    {project.project}
                                    {project.wbsCode && (
                                      <span className="text-gray-500 dark:text-gray-400 ml-1">
                                        • {project.wbsCode}
                                      </span>
                                    )}
                                  </div>
                                </td>
                                {project.days.map((minutes, i) => (
                                  <td
                                    key={i}
                                    className="text-center p-2 text-gray-600 dark:text-gray-400"
                                  >
                                    {minutes > 0 ? formatDuration(minutes) : '-'}
                                  </td>
                                ))}
                                <td className="text-right p-2 text-gray-900 dark:text-gray-50 font-medium">
                                  {formatDuration(rowTotal)}
                                </td>
                              </tr>
                            );
                          })}
                        </tbody>
                        <tfoot>
                          <tr className="border-t border-white/30 dark:border-white/20 bg-white/15 dark:bg-white/10">
                            <td className="p-2 text-gray-900 dark:text-gray-100 font-medium sticky left-0 bg-white/15 dark:bg-white/10 backdrop-blur-xl">
                              Total Hours
                            </td>
                            {dayTotals.map((minutes, i) => (
                              <td
                                key={i}
                                className="text-center p-2 text-gray-900 dark:text-gray-50 font-medium"
                              >
                                {minutes > 0 ? formatDuration(minutes) : '-'}
                              </td>
                            ))}
                            <td className="text-right p-2 text-gray-900 dark:text-gray-50 font-bold">
                              {formatDuration(dayTotals.reduce((sum, mins) => sum + mins, 0))}
                            </td>
                          </tr>
                        </tfoot>
                      </table>
                    </div>
                  </div>
                );
              })()}
            </div>
          </div>
        ) : (
          <ScrollArea className="h-[550px] rounded-b-[2.5rem] overflow-hidden">
            <div className="p-3">
              {/* Day View - Entries List */}
              <div>
                {entries.map((entry) => (
                  <div
                    key={entry.id}
                    className="backdrop-blur-xl bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-2xl p-3 hover:bg-white/15 dark:hover:bg-white/8 transition-colors mb-2.5"
                  >
                    <div className="flex items-start justify-between gap-3">
                      <div className="flex-1 min-w-0">
                        {/* Project/Category Name with Status Badge */}
                        <div className="flex items-center gap-2 mb-1">
                          <div className="text-sm text-gray-900 dark:text-gray-50 truncate">
                            {entry.project}
                            {entry.wbsCode && (
                              <span className="text-gray-500 dark:text-gray-400 ml-1.5">
                                • {entry.wbsCode}
                              </span>
                            )}
                          </div>
                          {getStatusBadge(entry)}
                        </div>
                        {/* Task Description */}
                        <div className="text-xs text-gray-600 dark:text-gray-400 mb-1 truncate">
                          {entry.task}
                        </div>
                        {/* Date and Duration */}
                        <div className="text-xs text-gray-600 dark:text-gray-400 flex items-center gap-1">
                          <span>{entry.shortDate ?? entry.time}</span>
                          <span>•</span>
                          <span>{entry.duration}</span>
                        </div>
                      </div>
                      <div className="flex items-center gap-1 flex-shrink-0">
                        <DropdownMenu modal={false}>
                          <DropdownMenuTrigger asChild>
                            <Button
                              variant="ghost"
                              size="icon"
                              className="h-7 w-7 text-gray-700 dark:text-gray-300"
                            >
                              <MoreHorizontal className="w-4 h-4" />
                            </Button>
                          </DropdownMenuTrigger>
                          <DropdownMenuContent
                            align="end"
                            className="backdrop-blur-[60px] bg-white/80 dark:bg-black/80 border-2 border-white/20 dark:border-white/10 shadow-[0_8px_32px_0_rgba(0,0,0,0.2)]"
                            sideOffset={5}
                          >
                            {entry.status === 'suggested' && (
                              <DropdownMenuItem
                                className="cursor-pointer"
                                onSelect={() => void handleAcceptSuggestion(entry)}
                              >
                                <Sparkles className="w-3.5 h-3.5 mr-2" />
                                Accept Suggestion
                              </DropdownMenuItem>
                            )}
                            <DropdownMenuItem
                              className="cursor-pointer"
                              onSelect={() => handleEditStart(entry)}
                            >
                              <Edit2 className="w-3.5 h-3.5 mr-2" />
                              Edit
                            </DropdownMenuItem>
                          </DropdownMenuContent>
                        </DropdownMenu>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </ScrollArea>
        )}

        {/* Edit Entry Modal */}
        <AnimatePresence>
          {editModalOpen && (
            <>
              {/* Backdrop overlay */}
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="absolute inset-0 z-40"
                onClick={() => setEditModalOpen(false)}
              />

              {/* Modal content */}
              <motion.div
                initial={{ opacity: 0, scale: 0.95 }}
                animate={{ opacity: 1, scale: 1 }}
                exit={{ opacity: 0, scale: 0.95 }}
                transition={{ duration: 0.15 }}
                className="absolute inset-0 z-50 flex items-center justify-center p-6 pointer-events-none"
              >
                <div className="w-full max-w-xs bg-black/[0.925] dark:bg-black/[0.925] border-2 border-white/20 dark:border-white/10 rounded-[40px] p-5 shadow-[0_8px_32px_0_rgba(0,0,0,0.2),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_8px_32px_0_rgba(0,0,0,0.4),0_0_0_1px_rgba(255,255,255,0.1)_inset] pointer-events-auto">
                  <div className="flex items-center gap-2 mb-3 pb-2 border-b border-white/10 dark:border-white/10">
                    <Edit2 className="w-3.5 h-3.5 text-gray-400 dark:text-gray-400" />
                    <span className="text-sm text-gray-100 dark:text-gray-100 font-semibold">
                      Edit Entry
                    </span>
                  </div>

                  <div className="space-y-2.5">
                    <div>
                      <Input
                        list="project-suggestions-entries"
                        id="project"
                        value={editedProject}
                        onChange={(e) => setEditedProject(e.target.value)}
                        className="h-8 bg-white/10 dark:bg-white/10 border-white/20 dark:border-white/20 text-gray-100 dark:text-gray-100 text-sm"
                        placeholder="Project name"
                      />
                      <datalist id="project-suggestions-entries">
                        <option value="Project Alpha" />
                        <option value="Project Beta" />
                        <option value="Internal" />
                        <option value="Meetings" />
                        <option value="Admin" />
                      </datalist>
                    </div>
                    <div>
                      <Input
                        list="task-suggestions-entries"
                        id="task"
                        value={editedTask}
                        onChange={(e) => setEditedTask(e.target.value)}
                        className="h-8 bg-white/10 dark:bg-white/10 border-white/20 dark:border-white/20 text-gray-100 dark:text-gray-100 text-sm"
                        placeholder="What did you work on?"
                      />
                      <datalist id="task-suggestions-entries">
                        <option value="Feature development" />
                        <option value="Bug fixing" />
                        <option value="Code review" />
                        <option value="Meeting" />
                        <option value="Documentation" />
                        <option value="Planning" />
                        <option value="Testing" />
                      </datalist>
                    </div>
                    <div>
                      <div className="flex items-center gap-1.5">
                        <Button
                          type="button"
                          variant="ghost"
                          onClick={() => {
                            const minutes = parseDuration(editedDuration);
                            if (minutes > 15) {
                              setEditedDuration(formatDuration(minutes - 15));
                            }
                          }}
                          className="h-8 w-8 p-0 bg-white/10 hover:bg-white/20 dark:bg-white/10 dark:hover:bg-white/20 text-gray-300 dark:text-gray-300"
                        >
                          <Minus className="w-3.5 h-3.5" />
                        </Button>
                        <Input
                          id="duration"
                          value={editedDuration}
                          onChange={(e) => setEditedDuration(e.target.value)}
                          className="h-8 flex-1 bg-white/10 dark:bg-white/10 border-white/20 dark:border-white/20 text-gray-100 dark:text-gray-100 text-sm text-center"
                          placeholder="e.g., 1h 30m"
                        />
                        <Button
                          type="button"
                          variant="ghost"
                          onClick={() => {
                            const minutes = parseDuration(editedDuration);
                            setEditedDuration(formatDuration(minutes + 15));
                          }}
                          className="h-8 w-8 p-0 bg-white/10 hover:bg-white/20 dark:bg-white/10 dark:hover:bg-white/20 text-gray-300 dark:text-gray-300"
                        >
                          <Plus className="w-3.5 h-3.5" />
                        </Button>
                      </div>
                    </div>
                    <div>
                      <Textarea
                        id="description"
                        value={editedDescription}
                        onChange={(e) => setEditedDescription(e.target.value)}
                        className="bg-white/10 dark:bg-white/10 border-white/20 dark:border-white/20 text-gray-100 dark:text-gray-100 text-sm min-h-[60px]"
                        placeholder="Add notes..."
                      />
                    </div>
                  </div>

                  <div className="flex flex-col gap-1.5 pt-3">
                    <div className="flex gap-1.5">
                      <Button
                        variant="ghost"
                        onClick={() => setEditModalOpen(false)}
                        className="flex-1 h-8 bg-white/10 hover:bg-white/20 dark:bg-white/10 dark:hover:bg-white/20 text-gray-300 dark:text-gray-300 text-xs"
                      >
                        Cancel
                      </Button>
                      <Button
                        onClick={() => void handleEditSave()}
                        className="flex-1 h-8 bg-green-500/20 hover:bg-green-500/30 dark:bg-green-400/20 dark:hover:bg-green-400/30 text-green-400 dark:text-green-400 text-xs"
                      >
                        Save
                      </Button>
                    </div>
                    <Button
                      variant="ghost"
                      onClick={() => void handleDelete()}
                      className="w-full h-8 bg-red-500/10 hover:bg-red-500/20 dark:bg-red-400/10 dark:hover:bg-red-400/20 text-red-400 dark:text-red-400 text-xs"
                    >
                      <Trash2 className="w-3.5 h-3.5 mr-1" />
                      Delete Entry
                    </Button>
                  </div>
                </div>
              </motion.div>
            </>
          )}
        </AnimatePresence>
      </div>
    </TooltipProvider>
  );
}
