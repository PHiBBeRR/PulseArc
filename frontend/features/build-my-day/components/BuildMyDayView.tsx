import { InWidgetNotification } from '@/shared/components/feedback';
import { useInWidgetNotification } from '@/shared/hooks';
import type { ProposedBlock } from '@/shared/types/generated';
import { celebrateWithConfetti, haptic } from '@/shared/utils';
import { invoke } from '@tauri-apps/api/core';
import {
  ArrowLeft,
  CalendarDays,
  Calendar as CalendarIcon,
  ChevronLeft,
  ChevronRight,
  Clock,
  GripHorizontal,
  Moon,
  Plane,
  Zap,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { Badge } from '@/shared/components/ui/badge';
import { Button } from '@/shared/components/ui/button';
import { Calendar } from '@/shared/components/ui/calendar';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/components/ui/popover';
import { ScrollArea } from '@/shared/components/ui/scroll-area';

type BuildMyDayViewProps = {
  onBack: () => void;
};

export function BuildMyDayView({ onBack }: BuildMyDayViewProps) {
  const [selectedDate, setSelectedDate] = useState<Date>(new Date());
  const [unclassifiedBlocks, setUnclassifiedBlocks] = useState<ProposedBlock[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isBuilding, setIsBuilding] = useState(false);
  const [isCalendarOpen, setIsCalendarOpen] = useState(false);
  const { notification, showNotification, dismiss } = useInWidgetNotification();

  // Dynamically resize window (Pattern 1: Direct Component Resize)
  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        const currentWindow = getCurrentWindow();

        const targetWidth = 680;
        const targetHeight = 620;

        console.log('[BuildMyDayView] Resizing window:', { targetWidth, targetHeight });
        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));

        // Allow resizing for Build My Day view (same as entries day view)
        await currentWindow.setResizable(true);
        await currentWindow.setMinSize(new LogicalSize(targetWidth - 100, targetHeight - 100));
        await currentWindow.setMaxSize(new LogicalSize(targetWidth + 400, targetHeight + 400));
      } catch (error) {
        console.error('[BuildMyDayView] Failed to resize window:', error);
      }
    };

    void resizeWindow();
  }, []); // Run once on mount

  // Fetch raw blocks (preview mode - NO classification) for the selected date
  useEffect(() => {
    const fetchUnclassifiedBlocks = async () => {
      setIsLoading(true);
      try {
        const dayEpoch = Math.floor(selectedDate.getTime() / 1000);
        // Get preview of raw blocks WITHOUT classification
        const blocks = await invoke<ProposedBlock[]>('preview_blocks_for_day', {
          dayEpoch,
        });
        setUnclassifiedBlocks(blocks);
      } catch (error) {
        console.error('Failed to fetch raw blocks:', error);
        showNotification('error', 'Failed to load activity blocks');
      } finally {
        setIsLoading(false);
      }
    };

    void fetchUnclassifiedBlocks();
  }, [selectedDate]);

  const formatDate = (date: Date): string => {
    const today = new Date();
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);

    if (date.toDateString() === today.toDateString()) {
      return 'Today';
    } else if (date.toDateString() === yesterday.toDateString()) {
      return 'Yesterday';
    } else {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' });
    }
  };

  const formatDuration = (minutes: number): string => {
    if (minutes < 60) {
      return `${minutes}m`;
    }
    const hours = Math.floor(minutes / 60);
    const mins = minutes % 60;
    return mins > 0 ? `${hours}h ${mins}m` : `${hours}h`;
  };

  const formatTime = (epochSeconds: number): string => {
    const date = new Date(epochSeconds * 1000);
    return date.toLocaleTimeString('en-US', { hour: 'numeric', minute: '2-digit', hour12: true });
  };

  const navigateDate = (direction: 'prev' | 'next' | 'today') => {
    const newDate = new Date(selectedDate);

    if (direction === 'today') {
      setSelectedDate(new Date());
    } else if (direction === 'prev') {
      newDate.setDate(newDate.getDate() - 1);
      setSelectedDate(newDate);
    } else {
      newDate.setDate(newDate.getDate() + 1);
      if (newDate <= new Date()) {
        setSelectedDate(newDate);
      }
    }
    haptic.light();
  };

  const handleBuildDay = async () => {
    if (unclassifiedBlocks.length === 0) {
      showNotification('info', 'No unclassified blocks to process');
      return;
    }

    setIsBuilding(true);
    haptic.light();

    try {
      showNotification('info', 'Building your day...');

      const dayEpoch = Math.floor(selectedDate.getTime() / 1000);

      // Call build_my_day Tauri command
      let blocks: unknown[];
      try {
        blocks = await invoke<unknown[]>('build_my_day', { dayEpoch });

        // If no blocks were built, try processing unprocessed snapshots
        if (blocks.length === 0) {
          showNotification('info', 'Processing activity snapshots...');

          try {
            const result = await invoke<string>('process_unprocessed_snapshots');
            console.log('Segmentation result:', result);

            // Retry build_my_day after processing
            showNotification('info', 'Retrying build...');
            blocks = await invoke<unknown[]>('build_my_day', { dayEpoch });
          } catch (segmentError) {
            console.error('Segmentation failed:', segmentError);
          }
        }
      } catch (buildError) {
        console.error('Build failed:', buildError);
        throw buildError;
      }

      showNotification(
        'success',
        `Built ${blocks.length} block${blocks.length !== 1 ? 's' : ''} for ${formatDate(selectedDate)}`
      );
      haptic.success();

      // Confetti celebration
      if (blocks.length > 0) {
        celebrateWithConfetti({ particleCount: 50, spread: 50 });
      }

      // Refresh the unclassified blocks list
      const updatedBlocks = await invoke<ProposedBlock[]>('get_proposed_blocks', {
        dayEpoch,
        statusFilter: 'pending',
      });
      setUnclassifiedBlocks(updatedBlocks);
    } catch (error) {
      console.error('Failed to build day:', error);
      showNotification('error', 'Failed to build day');
      haptic.error();
    } finally {
      setIsBuilding(false);
    }
  };

  const totalDurationSeconds = unclassifiedBlocks.reduce(
    (sum, block) => sum + block.duration_secs,
    0
  );
  const totalDurationMinutes = Math.round(totalDurationSeconds / 60);

  return (
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
            <h2 className="text-sm text-gray-900 dark:text-gray-100">Build My Day</h2>
          </div>

          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => void handleBuildDay()}
              disabled={isBuilding || unclassifiedBlocks.length === 0}
              className="h-7 w-7 hover:bg-white/20 dark:hover:bg-white/10 disabled:opacity-50"
              title={isBuilding ? 'Building...' : 'Build Day'}
            >
              <Zap className={`w-3.5 h-3.5 ${isBuilding ? 'animate-pulse' : ''}`} />
            </Button>
          </div>
        </div>

        {/* Date Navigation with Calendar Popover */}
        <div className="flex items-center justify-between gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => navigateDate('prev')}
            className="h-6 w-6 flex-shrink-0 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
          >
            <ChevronLeft className="w-3.5 h-3.5" />
          </Button>

          <Popover open={isCalendarOpen} onOpenChange={setIsCalendarOpen}>
            <PopoverTrigger asChild>
              <button className="flex flex-col items-center gap-0.5 min-w-0 hover:bg-white/10 dark:hover:bg-white/5 rounded-lg px-3 py-1 transition-colors cursor-pointer">
                <div className="flex items-center gap-1.5">
                  <CalendarDays className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400" />
                  <div className="text-sm font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap">
                    {formatDate(selectedDate)}
                  </div>
                </div>
                <div className="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                  {formatDuration(totalDurationMinutes)} · {unclassifiedBlocks.length} unclassified
                </div>
              </button>
            </PopoverTrigger>
            <PopoverContent
              className="w-auto p-0 bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 rounded-[40px] shadow-xl"
              align="center"
            >
              <Calendar
                mode="single"
                selected={selectedDate}
                onSelect={(date) => {
                  if (date) {
                    setSelectedDate(date);
                    setIsCalendarOpen(false);
                  }
                }}
                className="rounded-[40px]"
              />
            </PopoverContent>
          </Popover>

          <Button
            variant="ghost"
            size="icon"
            onClick={() => navigateDate('next')}
            disabled={selectedDate.toDateString() === new Date().toDateString()}
            className="h-6 w-6 flex-shrink-0 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 disabled:opacity-50"
          >
            <ChevronRight className="w-3.5 h-3.5" />
          </Button>
        </div>
      </div>

      {/* Content Area */}
      {isLoading ? (
        <div className="flex items-center justify-center h-96 p-8">
          <div className="flex items-center gap-3">
            <div className="w-4 h-4 border-2 border-gray-300 border-t-gray-600 dark:border-neutral-700 dark:border-t-neutral-400 rounded-full animate-spin" />
            <span className="text-sm text-gray-600 dark:text-gray-400">
              Loading activity blocks...
            </span>
          </div>
        </div>
      ) : unclassifiedBlocks.length === 0 ? (
        <div className="flex items-center justify-center h-96 p-8">
          <div className="text-center">
            <div className="inline-flex items-center justify-center w-12 h-12 rounded-full mb-3 backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20">
              <Zap className="w-6 h-6 text-gray-400 dark:text-gray-500" />
            </div>
            <h3 className="mb-1.5 text-gray-900 dark:text-gray-100 text-sm">
              No unclassified blocks
            </h3>
            <p className="text-xs text-gray-500 dark:text-gray-400">
              All activity for {formatDate(selectedDate)} has been classified
            </p>
          </div>
        </div>
      ) : (
        <ScrollArea className="h-[460px] rounded-b-[2.5rem] overflow-hidden">
          <div className="p-3 space-y-3">
            {unclassifiedBlocks.map((block) => {
              const durationMinutes = Math.round(block.duration_secs / 60);
              const activeMinutes = Math.round((block.duration_secs - block.total_idle_secs) / 60);
              const idleMinutes = Math.round(block.total_idle_secs / 60);
              const primaryActivity =
                block.activities.length > 0 ? block.activities[0]?.name : 'Unknown';

              return (
                <div
                  key={block.id}
                  className="backdrop-blur-xl bg-white/10 dark:bg-white/5 border border-white/30 dark:border-white/20 rounded-xl p-3 hover:bg-white/15 dark:hover:bg-white/10 transition-colors space-y-2"
                >
                  {/* Header: Time Range & Duration */}
                  <div className="flex items-start justify-between gap-2">
                    <div className="flex-1 min-w-0">
                      <div className="text-sm font-medium text-gray-900 dark:text-gray-100 mb-0.5">
                        {primaryActivity}
                      </div>
                      <div className="text-xs text-gray-500 dark:text-gray-400">
                        {formatTime(block.start_ts)} - {formatTime(block.end_ts)}
                      </div>
                    </div>
                    <div className="text-xs font-medium text-gray-700 dark:text-gray-300 backdrop-blur-xl bg-white/20 dark:bg-white/10 px-2 py-1 rounded-md shrink-0">
                      {formatDuration(durationMinutes)}
                    </div>
                  </div>

                  {/* Active vs Idle Time */}
                  <div className="flex items-center gap-3 text-xs">
                    <div className="flex items-center gap-1 text-gray-600 dark:text-gray-400">
                      <div className="w-2 h-2 rounded-full bg-green-500" />
                      <span className="font-medium">Active:</span>
                      <span>{formatDuration(activeMinutes)}</span>
                    </div>
                    <div className="flex items-center gap-1 text-gray-600 dark:text-gray-400">
                      <div className="w-2 h-2 rounded-full bg-gray-400" />
                      <span className="font-medium">Idle:</span>
                      <span>{formatDuration(idleMinutes)}</span>
                    </div>
                  </div>

                  {/* Context Badges (raw metadata, NOT classification) */}
                  <div className="flex flex-wrap items-center gap-1.5">
                    {block.is_travel && (
                      <Badge
                        variant="outline"
                        className="text-[10px] h-5 px-1.5 bg-purple-500/10 text-purple-700 dark:text-purple-400 border-purple-500/30"
                      >
                        <Plane className="w-2.5 h-2.5 mr-0.5" />
                        Travel
                      </Badge>
                    )}
                    {block.is_after_hours && (
                      <Badge
                        variant="outline"
                        className="text-[10px] h-5 px-1.5 bg-orange-500/10 text-orange-700 dark:text-orange-400 border-orange-500/30"
                      >
                        <Moon className="w-2.5 h-2.5 mr-0.5" />
                        After Hours
                      </Badge>
                    )}
                    {block.is_weekend && (
                      <Badge
                        variant="outline"
                        className="text-[10px] h-5 px-1.5 bg-pink-500/10 text-pink-700 dark:text-pink-400 border-pink-500/30"
                      >
                        <CalendarIcon className="w-2.5 h-2.5 mr-0.5" />
                        Weekend
                      </Badge>
                    )}
                    {block.has_calendar_overlap && (
                      <Badge
                        variant="outline"
                        className="text-[10px] h-5 px-1.5 bg-cyan-500/10 text-cyan-700 dark:text-cyan-400 border-cyan-500/30"
                      >
                        <Clock className="w-2.5 h-2.5 mr-0.5" />
                        Calendar
                      </Badge>
                    )}
                  </div>

                  {/* Activities Breakdown */}
                  {block.activities.length > 0 && (
                    <div className="space-y-1">
                      <div className="text-[10px] font-medium text-gray-600 dark:text-gray-400 uppercase tracking-wide">
                        Applications Used
                      </div>
                      <div className="space-y-0.5">
                        {block.activities.map((activity, idx) => (
                          <div key={idx} className="flex items-center gap-2 text-xs">
                            <div className="flex-1 min-w-0">
                              <span className="text-gray-700 dark:text-gray-300 truncate block">
                                {activity.name}
                              </span>
                            </div>
                            <div className="flex items-center gap-1.5 shrink-0">
                              <span className="text-gray-500 dark:text-gray-400 text-[10px]">
                                {Math.round(activity.percentage)}%
                              </span>
                              <span className="text-gray-500 dark:text-gray-400 text-[10px]">
                                {formatDuration(Math.round(activity.duration_secs / 60))}
                              </span>
                            </div>
                          </div>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        </ScrollArea>
      )}
    </div>
  );
}
