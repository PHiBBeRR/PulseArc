import { Button } from '@/shared/components/ui/button';
import { Calendar } from '@/shared/components/ui/calendar';
import { Popover, PopoverContent, PopoverTrigger } from '@/shared/components/ui/popover';
import { ScrollArea, ScrollBar } from '@/shared/components/ui/scroll-area';
import { Tabs, TabsList, TabsTrigger } from '@/shared/components/ui/tabs';
import { TooltipProvider } from '@/shared/components/ui/tooltip';
import { TimelineEntrySkeleton } from '@/shared/components/feedback';
import { formatTimeString } from '@/shared/utils/timeFormat';
import { AnimatePresence, motion } from 'framer-motion';
import { ArrowLeft, CalendarDays, ChevronLeft, ChevronRight, GripHorizontal } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { settingsService } from '../../settings/services/settingsService';
import { timelineService } from '../services/timelineService';
import type { DayData, TimelineDayViewProps, TimelineEntry } from '../types';
import { MarqueeText } from './MarqueeText';

export function TimelineDayView({ onBack, onViewModeChange }: TimelineDayViewProps) {
  const [isLoading, setIsLoading] = useState(false);
  const [selectedDate, setSelectedDate] = useState<Date>(new Date());
  const [viewMode, setViewMode] = useState<'day' | 'week'>('day');
  const [timelineEntries, setTimelineEntries] = useState<TimelineEntry[]>([]);
  const [weekData, setWeekData] = useState<DayData[]>([]);
  const [weekCalendarEvents, setWeekCalendarEvents] = useState<Map<number, TimelineEntry[]>>(
    new Map()
  );
  const [isCalendarOpen, setIsCalendarOpen] = useState(false);

  // Format hour label based on user's time format preference
  const formatHourLabel = (hour: number): string => {
    const timeFormat = settingsService.loadSettings().timeFormat;

    if (timeFormat === '24h') {
      // 24-hour format: "00:00", "13:00", "23:00"
      return `${hour.toString().padStart(2, '0')}:00`;
    } else {
      // 12-hour format: "12 AM", "1 PM", etc.
      if (hour === 0) return '12 AM';
      if (hour === 12) return '12 PM';
      if (hour > 12) return `${hour - 12} PM`;
      return `${hour} AM`;
    }
  };

  // Categorize calendar events based on keywords (same logic as SuggestedEntries)
  const categorizeCalendarEvent = (
    projectName: string,
    taskName: string
  ): 'personal' | 'general' | 'project' => {
    const textToCheck =
      projectName === 'General'
        ? taskName.toLowerCase()
        : `${projectName} ${taskName}`.toLowerCase();

    // Project: contains "project" keyword
    if (textToCheck.match(/\bproject\b/i)) {
      return 'project';
    }

    // General/Admin keywords
    if (
      textToCheck.match(
        /\b(team|admin|meeting|standup|sync|review|deployment|all-hands|townhall|status)\b/i
      )
    ) {
      return 'general';
    }

    // Default to Personal
    return 'personal';
  };

  // Get category colors (same as SuggestedEntries)
  const getCategoryColors = (category: 'personal' | 'general' | 'project') => {
    switch (category) {
      case 'personal':
        return {
          bg: 'bg-yellow-500/30 dark:bg-yellow-400/30',
          border: 'border-yellow-500/40 dark:border-yellow-400/40',
          dot: 'bg-yellow-500 dark:bg-yellow-400',
          text: 'text-yellow-600 dark:text-yellow-400',
        };
      case 'general':
        return {
          bg: 'bg-blue-500/30 dark:bg-blue-400/30',
          border: 'border-blue-500/40 dark:border-blue-400/40',
          dot: 'bg-blue-500 dark:bg-blue-400',
          text: 'text-blue-600 dark:text-blue-400',
        };
      case 'project':
        return {
          bg: 'bg-orange-500/30 dark:bg-orange-400/30',
          border: 'border-orange-500/40 dark:border-orange-400/40',
          dot: 'bg-orange-500 dark:bg-orange-400',
          text: 'text-orange-600 dark:text-orange-400',
        };
    }
  };

  // Fetch timeline entries (including calendar events) when date changes
  useEffect(() => {
    const fetchEntries = async () => {
      setIsLoading(true);
      try {
        const entries = await timelineService.getTimelineWithCalendar(selectedDate);
        setTimelineEntries(entries);
      } catch (error) {
        console.error('Failed to fetch timeline entries:', error);
        // Show empty state on error
        setTimelineEntries([]);
      } finally {
        setIsLoading(false);
      }
    };
    void fetchEntries();
  }, [selectedDate]);

  // Fetch week data when in week view or when date changes
  useEffect(() => {
    const fetchWeekData = async () => {
      if (viewMode === 'week') {
        setIsLoading(true);
        try {
          const [data, events] = await Promise.all([
            timelineService.getWeekData(selectedDate),
            timelineService.getWeekCalendarEvents(selectedDate),
          ]);
          setWeekData(data);
          setWeekCalendarEvents(events);
        } catch (error) {
          console.error('Failed to fetch week data:', error);
          setWeekData([]);
          setWeekCalendarEvents(new Map());
        } finally {
          setIsLoading(false);
        }
      }
    };
    void fetchWeekData();
  }, [viewMode, selectedDate]);

  // Notify parent when view mode changes (for window resizing)
  useEffect(() => {
    onViewModeChange?.(viewMode);
  }, [viewMode, onViewModeChange]);

  // Separate all-day events from regular events for day view
  const { allDayEvents, regularEvents } = useMemo(() => {
    const allDay = timelineEntries.filter((entry) => entry.isAllDay === true);
    const regular = timelineEntries.filter((entry) => entry.isAllDay !== true);
    return { allDayEvents: allDay, regularEvents: regular };
  }, [timelineEntries]);

  // Separate all-day events from regular events for week view
  const weekRegularEvents = useMemo(() => {
    const regularMap = new Map<number, TimelineEntry[]>();

    weekCalendarEvents.forEach((events, dayIndex) => {
      const regular = events.filter((event) => event.isAllDay !== true);
      regularMap.set(dayIndex, regular);
    });

    return regularMap;
  }, [weekCalendarEvents]);

  // Extract all-day events for the week
  const weekAllDayEvents = useMemo(() => {
    const allDayMap = new Map<number, TimelineEntry[]>();

    weekCalendarEvents.forEach((events, dayIndex) => {
      const allDay = events.filter((event) => event.isAllDay === true);
      allDayMap.set(dayIndex, allDay);
    });

    return allDayMap;
  }, [weekCalendarEvents]);

  // Dynamically resize window based on view mode (like EntriesView)
  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        const currentWindow = getCurrentWindow();

        let targetWidth = 680;
        let targetHeight = 720;

        if (viewMode === 'week') {
          targetWidth = 1450;
          targetHeight = 720;
        } else {
          // day view
          targetWidth = 680;
          targetHeight = 720;
        }

        // Resize window for view mode
        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));

        // Set resizable based on view mode
        await currentWindow.setResizable(true);
        // Allow resizing for both day and week views
        await currentWindow.setMinSize(new LogicalSize(targetWidth - 100, targetHeight - 100));
        await currentWindow.setMaxSize(new LogicalSize(targetWidth + 400, targetHeight + 400));
      } catch (error) {
        console.error('[TimelineDayView] Failed to resize window:', error);
      }
    };

    void resizeWindow();
  }, [viewMode]);

  // Show 24-hour timeline from midnight (12 AM) to 11 PM
  const hours = Array.from({ length: 24 }, (_, i) => i);

  const timeToMinutes = (time: string) => {
    const parts = time.split(':').map(Number);
    const hours = parts[0] ?? 0;
    const minutes = parts[1] ?? 0;
    return hours * 60 + minutes;
  };

  // Convert minutes to pixel offset (80px per hour = 1.333px per minute)
  const minutesToPixels = (minutes: number) => {
    return (minutes / 60) * 80;
  };

  const formatDate = (date: Date) => {
    return date.toLocaleDateString('en-US', {
      weekday: 'short',
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  };

  const navigateDate = (direction: 'prev' | 'next') => {
    const newDate = new Date(selectedDate);
    if (viewMode === 'day') {
      newDate.setDate(newDate.getDate() + (direction === 'next' ? 1 : -1));
    } else if (viewMode === 'week') {
      newDate.setDate(newDate.getDate() + (direction === 'next' ? 7 : -7));
    }
    setSelectedDate(newDate);
  };

  return (
    <TooltipProvider>
      <div className="backdrop-blur-[24px] overflow-hidden h-full flex flex-col">
        {/* Drag handle bar */}
        <div
          data-tauri-drag-region
          className="flex items-center justify-center py-2 cursor-move rounded-t-[40px] select-none"
        >
          <GripHorizontal className="w-8 h-3 text-gray-400/50 dark:text-gray-500/50 pointer-events-none" />
        </div>

        {/* Header - Fixed, doesn't scroll */}
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
              <h2 className="text-sm text-gray-900 dark:text-gray-100">Timeline</h2>
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

              {/* Today Button */}
              <Button
                variant="ghost"
                size="sm"
                onClick={() => setSelectedDate(new Date())}
                className="h-7 text-xs px-2 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10"
              >
                Today
              </Button>
            </div>
          </div>

          {/* Date Navigation and Total Summary */}
          <div className="flex items-center justify-between gap-2">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => navigateDate('prev')}
              className="h-6 w-6 flex-shrink-0"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
            </Button>

            <Popover open={isCalendarOpen} onOpenChange={setIsCalendarOpen}>
              <PopoverTrigger asChild>
                <button className="flex flex-col items-center gap-1 min-w-0 hover:bg-white/10 dark:hover:bg-white/5 rounded-lg px-3 py-1 transition-colors cursor-pointer">
                  <div className="flex items-center gap-1.5">
                    <CalendarDays className="w-3.5 h-3.5 text-gray-500 dark:text-gray-400" />
                    <div className="text-sm font-medium text-gray-700 dark:text-gray-300 whitespace-nowrap">
                      {formatDate(selectedDate)}
                    </div>
                  </div>
                  <div className="text-xs text-gray-500 dark:text-gray-400 whitespace-nowrap">
                    {viewMode === 'day' && (
                      <>
                        {regularEvents.length > 0
                          ? timelineService.formatDuration(
                              regularEvents.reduce((sum, entry) => sum + entry.duration, 0)
                            )
                          : '0m'}{' '}
                        · {regularEvents.length} {regularEvents.length === 1 ? 'event' : 'events'}
                      </>
                    )}
                    {viewMode === 'week' && (
                      <>
                        {weekData.reduce((sum, day) => sum + day.hours, 0).toFixed(1)}h ·{' '}
                        {weekData.reduce((sum, day) => sum + day.entries, 0)}{' '}
                        {weekData.reduce((sum, day) => sum + day.entries, 0) === 1
                          ? 'event'
                          : 'events'}
                      </>
                    )}
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
              className="h-6 w-6 flex-shrink-0"
            >
              <ChevronRight className="w-3.5 h-3.5" />
            </Button>
          </div>
        </div>

        {/* All-Day Events Section for Day View (outside scroll area) */}
        {viewMode === 'day' && allDayEvents.length > 0 && !isLoading && (
          <div className="px-4 py-2 border-b border-white/10 dark:border-white/5">
            <div className="text-xs font-medium text-gray-500 dark:text-gray-400 mb-2">
              All-Day Events
            </div>
            <div className="space-y-2 mb-2">
              {allDayEvents.map((event) => {
                const isCalendarEvent = event.isCalendarEvent === true;
                const category = isCalendarEvent
                  ? categorizeCalendarEvent(event.project, event.task)
                  : 'general';
                const colors = isCalendarEvent
                  ? getCategoryColors(category)
                  : {
                      bg: 'bg-white/30 dark:bg-white/20',
                      border: 'border-white/30 dark:border-white/20',
                      dot: 'bg-blue-500 dark:bg-blue-400',
                      text: 'text-gray-900 dark:text-gray-50',
                    };

                return (
                  <div
                    key={event.id}
                    className={`backdrop-blur-3xl ${colors.bg} border-2 ${colors.border} rounded-xl p-2.5 cursor-pointer transition-all hover:backdrop-blur-xl hover:brightness-125 flex items-center`}
                  >
                    <div className="flex items-center gap-1 min-w-0 flex-1">
                      <div className={`w-2 h-2 rounded-full flex-shrink-0 ${colors.dot}`} />
                      <div className={`text-sm font-medium ${colors.text} flex-shrink-0`}>
                        {event.project}
                      </div>
                      <MarqueeText
                        text={`· ${event.task}`}
                        className="text-sm text-gray-900 dark:text-gray-100 flex-1 min-w-0"
                      />
                    </div>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        {/* Week View Headers and All-Day Events (outside scroll area) */}
        {viewMode === 'week' && (
          <div className="overflow-hidden px-4">
            <div className="min-w-[1400px]">
              {/* Day Headers */}
              <div
                className="grid border-b border-white/10 dark:border-white/5"
                style={{ gridTemplateColumns: '70px repeat(7, minmax(180px, 1fr))' }}
              >
                {/* Empty cell for time column */}
                <div className="p-2 text-center" />
                {/* Day names */}
                {['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'].map((dayName, index) => {
                  const startOfWeek = new Date(selectedDate);
                  startOfWeek.setDate(selectedDate.getDate() - selectedDate.getDay());
                  const currentDayDate = new Date(startOfWeek);
                  currentDayDate.setDate(startOfWeek.getDate() + index);
                  const isToday = currentDayDate.toDateString() === new Date().toDateString();

                  return (
                    <div
                      key={dayName}
                      className="relative p-2 text-center border-l border-white/10 dark:border-white/5"
                    >
                      {/* Subtle white shading from the bottom */}
                      <div className="absolute bottom-0 left-0 right-0 h-16 bg-gradient-to-t from-white/20 to-transparent dark:from-white/10 pointer-events-none" />
                      <div
                        className={`relative text-xs font-medium ${isToday ? 'text-blue-600 dark:text-blue-400' : 'text-gray-700 dark:text-gray-300'}`}
                      >
                        {dayName}
                      </div>
                      <div
                        className={`relative text-[10px] mt-0.5 ${isToday ? 'text-blue-500 dark:text-blue-500' : 'text-gray-500 dark:text-gray-400'}`}
                      >
                        {currentDayDate.getDate()}
                      </div>
                    </div>
                  );
                })}
              </div>

              {/* All-Day Events Row */}
              {Array.from(weekAllDayEvents.values()).some((events) => events.length > 0) && (
                <div
                  className="grid border-b border-white/10 dark:border-white/5"
                  style={{ gridTemplateColumns: '70px repeat(7, minmax(180px, 1fr))' }}
                >
                  <div className="p-2 text-xs text-gray-500 dark:text-gray-400 text-right pr-2">
                    All-Day
                  </div>
                  {[0, 1, 2, 3, 4, 5, 6].map((dayIndex) => {
                    const dayEvents = weekAllDayEvents.get(dayIndex) ?? [];

                    return (
                      <div
                        key={dayIndex}
                        className="border-l border-white/10 dark:border-white/5 p-1 space-y-1 min-h-[40px]"
                      >
                        {dayEvents.map((event) => {
                          const isCalendarEvent = event.isCalendarEvent === true;
                          const category = isCalendarEvent
                            ? categorizeCalendarEvent(event.project, event.task)
                            : 'general';
                          const colors = isCalendarEvent
                            ? getCategoryColors(category)
                            : {
                                bg: 'bg-white/20 dark:bg-white/10',
                                border: 'border-white/30 dark:border-white/20',
                                dot: 'bg-blue-500 dark:bg-blue-400',
                                text: 'text-gray-900 dark:text-gray-50',
                              };

                          return (
                            <div
                              key={event.id}
                              className={`${colors.bg} border-l-4 ${colors.border.replace('border-', 'border-l-')} rounded-r px-1.5 py-1 cursor-pointer hover:brightness-110 transition-all flex items-center`}
                            >
                              <div className="flex items-center gap-1 min-w-0 flex-1">
                                <div
                                  className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${colors.dot}`}
                                />
                                <div
                                  className={`text-[9px] font-medium ${colors.text} flex-shrink-0`}
                                >
                                  {event.project}
                                </div>
                                <MarqueeText
                                  text={`· ${event.task}`}
                                  className="text-[9px] text-gray-900 dark:text-gray-100 flex-1 min-w-0"
                                />
                              </div>
                            </div>
                          );
                        })}
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          </div>
        )}

        {/* Content Area */}
        <ScrollArea className="h-[550px] rounded-b-[2.5rem] overflow-hidden">
          {viewMode === 'week' && <ScrollBar orientation="horizontal" />}
          <div className={viewMode === 'day' ? 'p-4' : 'px-4'}>
            <AnimatePresence mode="wait">
              {/* Day View */}
              {viewMode === 'day' && (
                <motion.div
                  key="day"
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: 20 }}
                  transition={{ duration: 0.2 }}
                >
                  {isLoading ? (
                    <div className="space-y-8">
                      {[1, 2, 3, 4].map((i) => (
                        <div key={i} className="space-y-3">
                          <div className="flex items-center gap-3 mb-3">
                            <div className="w-12 h-3 bg-white/20 dark:bg-white/10 rounded" />
                            <div className="flex-1 h-px bg-white/20 dark:bg-white/10" />
                          </div>
                          <div className="ml-16">
                            <TimelineEntrySkeleton />
                          </div>
                        </div>
                      ))}
                    </div>
                  ) : (
                    <div className="relative" style={{ height: '1920px' }}>
                      {/* Hour markers and grid lines */}
                      {hours.map((hour) => {
                        const topPosition = hour * 80 - 5; // Move lines up slightly
                        return (
                          <div
                            key={hour}
                            className="absolute left-0 right-0 flex items-center gap-3"
                            style={{ top: `${topPosition}px` }}
                          >
                            {/* Hour label */}
                            <div
                              className="text-xs text-gray-500 dark:text-gray-400 w-12 text-right flex-shrink-0"
                              style={{ marginTop: '-5px' }}
                            >
                              {formatHourLabel(hour)}
                            </div>
                            {/* Horizontal line - aligned exactly at hour boundary, 2px to match event borders */}
                            <div className="flex-1 h-0.5 bg-white/20 dark:bg-white/10" />
                          </div>
                        );
                      })}

                      {/* Absolutely positioned events */}
                      {regularEvents.map((entry) => {
                        const startMinutes = timeToMinutes(entry.startTime);
                        const topPosition = minutesToPixels(startMinutes);
                        const height = Math.max(minutesToPixels(entry.duration), 20); // Min 20px height
                        const isCalendarEvent = entry.isCalendarEvent === true;

                        // Get category and colors for calendar events
                        const category = isCalendarEvent
                          ? categorizeCalendarEvent(entry.project, entry.task)
                          : 'general';
                        const colors = isCalendarEvent
                          ? getCategoryColors(category)
                          : {
                              bg: 'bg-white/20 dark:bg-white/10',
                              border: 'border-white/30 dark:border-white/20',
                              dot: '',
                              text: '',
                            };
                        const borderClass = isCalendarEvent
                          ? `border-l-4 ${colors.border.replace('border-', 'border-l-')}`
                          : 'border-l-4 border-gray-400';

                        return (
                          <div
                            key={entry.id}
                            className={`absolute left-16 right-0 ${colors.bg} ${borderClass} rounded-r cursor-pointer hover:brightness-110 hover:z-10 transition-all overflow-hidden px-1.5 py-1 flex ${entry.duration <= 45 ? 'items-center' : 'flex-col justify-start'}`}
                            style={{
                              top: `${topPosition}px`,
                              height: `${height}px`,
                            }}
                          >
                            {/* Short events (<= 45min): Single line layout */}
                            {entry.duration <= 45 ? (
                              (() => {
                                // Calculate end time
                                const [hours, minutes] = entry.startTime.split(':').map(Number);
                                const startMinutes = (hours ?? 0) * 60 + (minutes ?? 0);
                                const endMinutes = startMinutes + entry.duration;
                                const endHours = Math.floor(endMinutes / 60) % 24;
                                const endMins = endMinutes % 60;
                                const endTime = `${endHours.toString().padStart(2, '0')}:${endMins.toString().padStart(2, '0')}`;
                                const timeRange = `${formatTimeString(entry.startTime)} - ${formatTimeString(endTime)}`;

                                return (
                                  <div className="flex-1 min-w-0 flex items-center gap-1">
                                    <div
                                      className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${colors.dot}`}
                                    />
                                    <MarqueeText className="text-[11px] flex-1 min-w-0">
                                      <span className={`font-medium ${colors.text}`}>
                                        {entry.project}
                                      </span>
                                      <span className="text-gray-900 dark:text-gray-100">
                                        {' '}
                                        · {entry.task}
                                      </span>
                                      <span className="text-gray-600 dark:text-gray-400">
                                        {' '}
                                        · {timeRange}
                                      </span>
                                    </MarqueeText>
                                  </div>
                                );
                              })()
                            ) : (
                              <div className="flex-1 min-w-0 flex flex-col">
                                <div className="flex items-center gap-1 mb-0.5">
                                  <div
                                    className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${colors.dot}`}
                                  />
                                  <MarqueeText
                                    text={entry.project}
                                    className={`text-[11px] font-medium ${colors.text} flex-1 min-w-0`}
                                  />
                                </div>
                                <MarqueeText
                                  text={entry.task}
                                  className="text-[12px] text-gray-900 dark:text-gray-100 leading-tight"
                                />
                                {(() => {
                                  // Calculate end time
                                  const [hours, minutes] = entry.startTime.split(':').map(Number);
                                  const startMinutes = (hours ?? 0) * 60 + (minutes ?? 0);
                                  const endMinutes = startMinutes + entry.duration;
                                  const endHours = Math.floor(endMinutes / 60) % 24;
                                  const endMins = endMinutes % 60;
                                  const endTime = `${endHours.toString().padStart(2, '0')}:${endMins.toString().padStart(2, '0')}`;

                                  return (
                                    <div className="text-[11px] text-gray-600 dark:text-gray-400 leading-tight mt-0.5">
                                      {formatTimeString(entry.startTime)} -{' '}
                                      {formatTimeString(endTime)}
                                    </div>
                                  );
                                })()}
                              </div>
                            )}
                          </div>
                        );
                      })}
                    </div>
                  )}
                </motion.div>
              )}

              {/* Week Calendar View */}
              {viewMode === 'week' && (
                <motion.div
                  key="week"
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  exit={{ opacity: 0, x: 20 }}
                  transition={{ duration: 0.2 }}
                >
                  {/* Calendar Grid */}
                  <div className="min-w-[1400px]">
                    {/* Time Rows */}
                    <div className="relative" style={{ height: '1440px' }}>
                      {/* Hour grid lines */}
                      {hours.map((hour) => (
                        <div
                          key={hour}
                          className="absolute left-0 right-0 grid border-b border-white/10 dark:border-white/5"
                          style={{
                            gridTemplateColumns: '70px repeat(7, minmax(180px, 1fr))',
                            top: `${hour * 60}px`,
                            height: '60px',
                          }}
                        >
                          {/* Time label */}
                          <div className="p-2 text-xs text-gray-500 dark:text-gray-400 text-right pr-2">
                            {formatHourLabel(hour)}
                          </div>

                          {/* Day columns - click handlers */}
                          {[0, 1, 2, 3, 4, 5, 6].map((dayIndex) => (
                            <div
                              key={dayIndex}
                              className="border-l border-white/10 dark:border-white/5 cursor-pointer hover:bg-white/10 dark:hover:bg-white/5 transition-colors"
                              onClick={() => {
                                const startOfWeek = new Date(selectedDate);
                                startOfWeek.setDate(selectedDate.getDate() - selectedDate.getDay());
                                const newDate = new Date(startOfWeek);
                                newDate.setDate(startOfWeek.getDate() + dayIndex);
                                setSelectedDate(newDate);
                                setViewMode('day');
                              }}
                            />
                          ))}
                        </div>
                      ))}

                      {/* Absolutely positioned events for each day */}
                      {[0, 1, 2, 3, 4, 5, 6].map((dayIndex) => {
                        const dayEvents = weekRegularEvents.get(dayIndex) ?? [];

                        return dayEvents.map((event) => {
                          const startMinutes = timeToMinutes(event.startTime);
                          const topPosition = startMinutes; // 1px per minute (60px per hour)
                          const height = Math.max(event.duration, 15); // Min 15px height, 1px per minute

                          // Calculate left position based on day index
                          // Grid: 70px (time) + 7 equal columns
                          const columnWidth = `calc((100% - 70px) / 7)`;
                          const leftOffset = 70; // Time column width

                          // Get category and colors
                          const category = event.isCalendarEvent
                            ? categorizeCalendarEvent(event.project, event.task)
                            : 'general';
                          const colors = event.isCalendarEvent
                            ? getCategoryColors(category)
                            : {
                                bg: 'bg-white/20 dark:bg-white/10',
                                border: 'border-white/30 dark:border-white/20',
                                dot: '',
                                text: '',
                              };
                          const borderClass = event.isCalendarEvent
                            ? `border-l-4 ${colors.border.replace('border-', 'border-l-')}`
                            : 'border-l-4 border-gray-400';

                          return (
                            <div
                              key={`${dayIndex}-${event.id}`}
                              className={`absolute ${colors.bg} ${borderClass} rounded-r px-1.5 py-1 overflow-hidden cursor-pointer hover:brightness-110 hover:z-10 transition-all flex ${event.duration <= 30 ? 'items-center' : 'flex-col justify-start'}`}
                              style={{
                                top: `${topPosition}px`,
                                height: `${height}px`,
                                left: `calc(${leftOffset}px + ${columnWidth} * ${dayIndex} + 2px)`,
                                width: `calc(${columnWidth} - 4px)`,
                              }}
                              onClick={() => {
                                const startOfWeek = new Date(selectedDate);
                                startOfWeek.setDate(selectedDate.getDate() - selectedDate.getDay());
                                const newDate = new Date(startOfWeek);
                                newDate.setDate(startOfWeek.getDate() + dayIndex);
                                setSelectedDate(newDate);
                                setViewMode('day');
                              }}
                            >
                              {/* Short events (<= 30min): Single line layout */}
                              {event.duration <= 30 ? (
                                (() => {
                                  // Calculate end time
                                  const [hours, minutes] = event.startTime.split(':').map(Number);
                                  const startMinutes = (hours ?? 0) * 60 + (minutes ?? 0);
                                  const endMinutes = startMinutes + event.duration;
                                  const endHours = Math.floor(endMinutes / 60) % 24;
                                  const endMins = endMinutes % 60;
                                  const endTime = `${endHours.toString().padStart(2, '0')}:${endMins.toString().padStart(2, '0')}`;
                                  const timeRange = `${formatTimeString(event.startTime)} - ${formatTimeString(endTime)}`;

                                  return (
                                    <div className="flex items-center gap-1 min-w-0 w-full">
                                      <div
                                        className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${colors.dot}`}
                                      />
                                      <MarqueeText className="text-[9px] flex-1 min-w-0">
                                        <span className={`font-medium ${colors.text}`}>
                                          {event.project}
                                        </span>
                                        <span className="text-gray-900 dark:text-gray-100">
                                          {' '}
                                          · {event.task}
                                        </span>
                                        <span className="text-gray-600 dark:text-gray-400">
                                          {' '}
                                          · {timeRange}
                                        </span>
                                      </MarqueeText>
                                    </div>
                                  );
                                })()
                              ) : (
                                <>
                                  {/* Longer events: Multi-line layout */}
                                  {/* Project name with colored dot indicator */}
                                  <div className="flex items-center gap-1 mb-0.5 min-w-0">
                                    <div
                                      className={`w-1.5 h-1.5 rounded-full flex-shrink-0 ${colors.dot}`}
                                    />
                                    <MarqueeText
                                      text={event.project}
                                      className={`text-[9px] font-medium ${colors.text} flex-1 min-w-0`}
                                    />
                                  </div>

                                  {/* Task/Summary */}
                                  <MarqueeText
                                    text={event.task}
                                    className="text-[10px] text-gray-900 dark:text-gray-100 leading-tight"
                                  />

                                  {/* Time (if enough height) */}
                                  {height > 35 &&
                                    (() => {
                                      // Calculate end time
                                      const [hours, minutes] = event.startTime
                                        .split(':')
                                        .map(Number);
                                      const startMinutes = (hours ?? 0) * 60 + (minutes ?? 0);
                                      const endMinutes = startMinutes + event.duration;
                                      const endHours = Math.floor(endMinutes / 60) % 24;
                                      const endMins = endMinutes % 60;
                                      const endTime = `${endHours.toString().padStart(2, '0')}:${endMins.toString().padStart(2, '0')}`;

                                      return (
                                        <div className="text-[9px] text-gray-600 dark:text-gray-400 leading-tight mt-0.5">
                                          {formatTimeString(event.startTime)} -{' '}
                                          {formatTimeString(endTime)}
                                        </div>
                                      );
                                    })()}
                                </>
                              )}
                            </div>
                          );
                        });
                      })}
                    </div>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </ScrollArea>
      </div>
    </TooltipProvider>
  );
}
