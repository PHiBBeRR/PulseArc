import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { InWidgetNotification } from '@/shared/components/feedback';
import { useTheme } from '@/shared/components/layout';
import { useInWidgetNotification } from '@/shared/hooks';
import { audioService, idleSyncMetrics } from '@/shared/services';
import { celebrateMilestone, celebrateWithConfetti, haptic } from '@/shared/utils';
import { formatTime } from '@/shared/utils/timeFormat';
import { motion } from 'framer-motion';
import { BarChart3, Hammer, List, Pause, Play, Plus, Settings, Square, Zap } from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { IdleDetectionModal } from '../../idle-detection';
import { QuickProjectSwitcher } from '../../project';
import { SaveEntryModal } from '../../time-entry';
import { timerService } from '../services';
// import { calendarService } from '../../settings/services/calendarService';
import { TIMER_STATE_EVT, safeEmit, type TimerStateEventV1 } from '@/shared/events/timer-events';
import type { ActivityContext } from '@/shared/types/generated';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { TimerProps, timerState } from '../types';
import { SuggestedEntries } from './SuggestedEntries';
import { WbsAutocomplete } from './WbsAutocomplete';

export function MainTimer({
  onEntriesClick,
  onSettingsClick,
  onAnalyticsClick,
  onQuickEntry,
  onTimelineClick,
  onBuildMyDayClick,
  onNotificationTriggerReady,
  onTimerStateChange,
  initialStatus = 'inactive',
  initialElapsed = 0,
}: TimerProps) {
  const [timerState, setTimerState] = useState<timerState>(initialStatus);
  const [elapsed, setElapsed] = useState(initialElapsed);
  const [currentTime, setCurrentTime] = useState(new Date());
  const [showQuickSwitcher, setShowQuickSwitcher] = useState(false);
  const [showIdleModal, setShowIdleModal] = useState(false);
  const [showSaveModal, setShowSaveModal] = useState(false);
  const [idleMinutes, setIdleMinutes] = useState(0);
  const [savedElapsed, setSavedElapsed] = useState(0);
  const [nextEvent, setNextEvent] = useState<{
    title: string;
    minutesUntil: number | null;
    eventTime: Date | null;
  } | null>(null);
  const [phraseIndex, setPhraseIndex] = useState(0);
  const [suggestionCount, setSuggestionCount] = useState(0); // Track number of visible suggestion cards
  const [isCollapsed, setIsCollapsed] = useState(false); // Track collapsed state of Recent Activity
  const [isHovering, setIsHovering] = useState(false);
  const [activityContext, setActivityContext] = useState<ActivityContext | null>(null);
  // const [isSyncing, setIsSyncing] = useState(false);
  const [showSwitchActivity, setShowSwitchActivity] = useState(false);
  const [switchProject, setSwitchProject] = useState('');
  const [switchActivity, setSwitchActivity] = useState('');
  const [eventTextOverflows, setEventTextOverflows] = useState(false);
  useTheme();
  const { notification, showNotification, dismiss } = useInWidgetNotification();

  const lastActivityRef = useRef<number>(Date.now());
  const eventTextRef = useRef<HTMLDivElement>(null);
  const eventContainerRef = useRef<HTMLDivElement>(null);

  // Format upcoming event display based on time until event
  const formatEventDisplay = useCallback(
    (minutesUntil: number, eventTime: Date): { prefix: string; suffix: string } => {
      // Negative value means event is ongoing (minutesUntil is actually minutesRemaining * -1)
      if (minutesUntil < 0) {
        const minutesRemaining = Math.abs(minutesUntil);
        return {
          prefix: 'IN PROGRESS:',
          suffix: ` (${minutesRemaining} min left)`,
        };
      }

      // More than 1 hour away: show as "UPCOMING: [Title] at [Time]"
      if (minutesUntil > 60) {
        const timeStr = formatTime(eventTime);
        return {
          prefix: 'UPCOMING:',
          suffix: ` at ${timeStr}`,
        };
      }

      // Less than 1 hour: show countdown "in X min"
      return {
        prefix: 'UPCOMING:',
        suffix: ` in ${minutesUntil} min`,
      };
    },
    []
  );

  // Refs to avoid stale closures in idle detection
  const elapsedRef = useRef(elapsed);
  useEffect(() => {
    elapsedRef.current = elapsed;
  }, [elapsed]);

  const timerStateRef = useRef(timerState);
  useEffect(() => {
    timerStateRef.current = timerState;
  }, [timerState]);

  const onTimerStateChangeRef = useRef(onTimerStateChange);
  useEffect(() => {
    onTimerStateChangeRef.current = onTimerStateChange;
  }, [onTimerStateChange]);

  // Expose notification trigger to parent
  useEffect(() => {
    onNotificationTriggerReady?.(showNotification);
  }, [showNotification, onNotificationTriggerReady]);

  // FEATURE-009: Real-time activity tracking (pure event-driven, no polling)
  useEffect(() => {
    // Use a ref to store the unlisten function so cleanup can access it even if async
    // This prevents a race condition where cleanup runs before async listen() completes
    const unlistenRef = { current: undefined as (() => void) | undefined };

    const setupRealtimeListener = async () => {
      try {
        // Listen to backend events (pure event-driven)
        const unlistenFn = await listen<ActivityContext>('activity-context-updated', (event) => {
          setActivityContext(event.payload);
        });

        // Store in ref so cleanup can access it
        unlistenRef.current = unlistenFn;
      } catch (error) {
        console.error('Failed to setup event listener:', error);
      }
    };

    void setupRealtimeListener();

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
      }
    };
  }, []);

  // FEATURE-019: Dynamically resize window based on suggestion count, collapse state, and timer state
  // Track previous height to determine if we're collapsing or expanding
  const prevHeightRef = useRef<number>(300);

  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        const currentWindow = getCurrentWindow();

        const targetWidth = 460; // Fixed width
        const maxHeight = 650; // Match tauri.conf.json maxHeight
        let targetHeight = 325; // Base height when timer is active or paused (collapsed)

        if (timerState === 'inactive') {
          // Base height: timer controls + upcoming event = 300px
          const baseHeight = 300;

          if (suggestionCount === 0) {
            // No Recent Activity section at all
            targetHeight = baseHeight;
          } else if (isCollapsed) {
            // Recent Activity header only (collapsed) - stay at base height
            targetHeight = baseHeight;
          } else {
            // Recent Activity expanded with cards
            const headerHeight = 60; // Collapsible header
            const tabsHeight = 40; // Two-tab system
            const cardHeight = 95; // Each suggestion card (compact design)
            const spacing = suggestionCount > 1 ? (suggestionCount - 1) * 10 : 0; // 10px gap between cards (space-y-2.5)
            const bottomPadding = 10; // Adjusted for 715px total with 3 suggestions

            const contentHeight =
              headerHeight + tabsHeight + suggestionCount * cardHeight + spacing + bottomPadding;
            targetHeight = baseHeight + contentHeight;
          }
        } else if (timerState === 'active' || timerState === 'paused') {
          // Timer is active/paused - account for Switch Activity section
          const baseHeight = 325; // Base height with buttons collapsed

          if (showSwitchActivity) {
            // Switch Activity expanded to 450px total
            targetHeight = 450;
          } else {
            targetHeight = baseHeight;
          }
        }

        // Cap at max height to prevent exceeding Tauri config limit
        targetHeight = Math.min(targetHeight, maxHeight);

        console.log('[MainTimer] Resizing window:', {
          suggestionCount,
          isCollapsed,
          timerState,
          showSwitchActivity,
          targetHeight,
          prevHeight: prevHeightRef.current,
        });

        // Track previous height for future comparisons
        prevHeightRef.current = targetHeight;

        await currentWindow.setSize(new LogicalSize(targetWidth, targetHeight));
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    };

    // Calculate if we're collapsing first
    let targetHeight = 325;
    if (timerState === 'inactive') {
      const baseHeight = 300;
      if (suggestionCount === 0) {
        targetHeight = baseHeight;
      } else if (isCollapsed) {
        targetHeight = baseHeight;
      } else {
        const headerHeight = 60;
        const tabsHeight = 40;
        const cardHeight = 95;
        const spacing = suggestionCount > 1 ? (suggestionCount - 1) * 10 : 0;
        const bottomPadding = 10;
        const contentHeight =
          headerHeight + tabsHeight + suggestionCount * cardHeight + spacing + bottomPadding;
        targetHeight = baseHeight + contentHeight;
      }
    } else if (timerState === 'active' || timerState === 'paused') {
      const baseHeight = 325;
      if (showSwitchActivity) {
        targetHeight = 450;
      } else {
        targetHeight = baseHeight;
      }
    }
    targetHeight = Math.min(targetHeight, 650);

    const isCollapsing = targetHeight < prevHeightRef.current;

    // Only delay on collapse, expand immediately
    if (isCollapsing) {
      const timeoutId = setTimeout(() => {
        void resizeWindow();
      }, 245); // 245ms delay for collapse
      return () => clearTimeout(timeoutId);
    } else {
      void resizeWindow();
      return undefined;
    }
  }, [suggestionCount, isCollapsed, timerState, showSwitchActivity]);

  // Play sound feedback
  const playSound = () => audioService.playClick();

  // Update current time every second
  useEffect(() => {
    const interval = setInterval(() => {
      setCurrentTime(new Date());
    }, 1000);

    return () => clearInterval(interval);
  }, []);

  // Rotating phrases when there are no events
  const noEventPhrases = [
    "That's it! You're all done for the day!",
    'All clear for deep work',
    'Your calendar is wide open',
    'No meetings—time to focus',
    'Perfect time for that project you love',
    "Calendar cleared—you've got this!",
    'Free to create something amazing',
    'Time to tackle your to-do list',
    'Your schedule is all yours',
    'Nothing but opportunity ahead',
    'Coast is clear—dive into deep work',
    'No interruptions coming up',
    'Smooth sailing from here',
    "Calendar's quiet—make it count",
    'Time to do your best work',
    "Calendar's done—great job today!",
    "That's a wrap on meetings!",
    "You're free for the rest of the day",
    'Time to wind down or power through',
    'The rest of the day is yours',
  ];

  // Rotate phrase every 15 minutes
  useEffect(() => {
    const rotatePhraseInterval = setInterval(
      () => {
        setPhraseIndex((prev) => (prev + 1) % noEventPhrases.length);
      },
      15 * 60 * 1000
    ); // 15 minutes

    return () => clearInterval(rotatePhraseInterval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Check if event text overflows and needs marquee effect
  useEffect(() => {
    const checkOverflow = () => {
      if (!eventTextRef.current || !eventContainerRef.current) return;

      // Measure the hidden element's full width against the container
      const textWidth = eventTextRef.current.scrollWidth;
      const containerWidth = eventContainerRef.current.clientWidth;

      const overflows = textWidth > containerWidth;
      console.log('[MainTimer] Overflow check:', {
        textWidth,
        containerWidth,
        overflows,
        title: nextEvent?.title,
      });
      setEventTextOverflows(overflows);
    };

    // Check on mount and when event changes
    checkOverflow();

    // Small delay to ensure layout is complete
    const timeout = setTimeout(checkOverflow, 100);

    // Recheck on window resize
    window.addEventListener('resize', checkOverflow);
    return () => {
      clearTimeout(timeout);
      window.removeEventListener('resize', checkOverflow);
    };
  }, [nextEvent]);

  // Get next calendar event from timeline
  useEffect(() => {
    const getNextEvent = async () => {
      try {
        // Import timelineService dynamically to avoid circular dependencies
        const { timelineService } = await import('../../timeline/services/timelineService');

        // Get today's calendar events
        const today = new Date();
        const calendarEvents = await timelineService.getCalendarEvents(today);

        // Convert to { title, time, endTime } format and check for ongoing or upcoming events
        const now = Date.now();
        const events = calendarEvents
          .map((event) => ({
            title: event.originalSummary || event.task,
            startTime: new Date(event.startEpoch * 1000),
            endTime: new Date(event.startEpoch * 1000 + event.duration * 60 * 1000), // duration is in minutes
          }))
          .filter((event) => event.endTime.getTime() > now) // Include ongoing events (end time > now)
          .sort((a, b) => a.startTime.getTime() - b.startTime.getTime());

        if (events.length > 0) {
          const nextEvent = events[0];
          if (nextEvent) {
            // Check if event is ongoing (started but not ended)
            const isOngoing =
              nextEvent.startTime.getTime() <= now && nextEvent.endTime.getTime() > now;

            if (isOngoing) {
              // Show "IN PROGRESS" for ongoing events
              const minutesRemaining = Math.floor((nextEvent.endTime.getTime() - now) / 60000);
              setNextEvent({
                title: nextEvent.title,
                minutesUntil: -minutesRemaining,
                eventTime: nextEvent.startTime,
              });
            } else {
              // Show "UPCOMING" for future events
              const minutesUntil = Math.floor((nextEvent.startTime.getTime() - now) / 60000);
              setNextEvent({
                title: nextEvent.title,
                minutesUntil,
                eventTime: nextEvent.startTime,
              });
            }
          }
        } else {
          // Show a rotating message when there are no more events
          const phrase = noEventPhrases[phraseIndex] ?? 'All clear for the day!';
          setNextEvent({ title: phrase, minutesUntil: null, eventTime: null });
        }
      } catch (error) {
        console.error('Failed to fetch next event:', error);
        // Fallback to no-event phrase on error
        const phrase = noEventPhrases[phraseIndex] ?? 'All clear for the day!';
        setNextEvent({ title: phrase, minutesUntil: null, eventTime: null });
      }
    };

    void getNextEvent();
    const interval = setInterval(() => void getNextEvent(), 60000); // Update every minute

    return () => clearInterval(interval);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [phraseIndex]);

  // Timer logic - only increment when active
  useEffect(() => {
    if (timerState !== 'active') return;

    const interval = setInterval(() => {
      setElapsed((prev) => prev + 1);
    }, 1000);

    return () => clearInterval(interval);
  }, [timerState]);

  // Idle detection - only when active (FEATURE-008: Uses backend system idle detection)
  useEffect(() => {
    if (timerState !== 'active') return;

    const IDLE_THRESHOLD_SECONDS = 5 * 60; // 5 minutes
    const CHECK_INTERVAL_MS = 30_000; // Check every 30 seconds

    const markIdle = (idleSeconds: number) => {
      const now = Date.now();
      const idleMinutesCalc = Math.floor(idleSeconds / 60);

      // FEATURE-012: Record idle detection with latency
      const expectedIdleTime = now - idleSeconds * 1000;
      const detectionLatencyMs = Math.max(
        0,
        now - expectedIdleTime - IDLE_THRESHOLD_SECONDS * 1000
      );
      void idleSyncMetrics.recordIdleDetection(detectionLatencyMs);

      setIdleMinutes(idleMinutesCalc);
      setShowIdleModal(true);
      setTimerState('idle');
      onTimerStateChangeRef.current?.('idle', elapsedRef.current);

      // Emit to tracker window (versioned payload with error handling)
      const emitStart = performance.now();
      const payload: TimerStateEventV1 = {
        state: 'idle',
        elapsed: elapsedRef.current,
        ts: now,
        source: 'timer',
        v: 1,
      };
      void safeEmit(TIMER_STATE_EVT, payload)
        .then(() => {
          // FEATURE-012: Record event emission latency (microseconds)
          const emitLatencyUs = Math.round((performance.now() - emitStart) * 1000);
          void idleSyncMetrics.recordTimerEventEmission(emitLatencyUs, true);
        })
        .catch(() => {
          void idleSyncMetrics.recordTimerEventEmission(0, false);
        });
    };

    const checkIdle = async () => {
      try {
        // FEATURE-008 Phase 0: Use backend for accurate system-level idle time
        const idleSeconds = await invoke<number>('get_system_idle_seconds');

        if (idleSeconds >= IDLE_THRESHOLD_SECONDS) {
          markIdle(idleSeconds);
        }
      } catch (error) {
        // Graceful fallback to frontend detection if backend unavailable
        console.warn('Backend idle detection unavailable, using frontend fallback:', error);

        const now = Date.now();
        const idleMs = now - lastActivityRef.current;
        const idleSeconds = Math.floor(idleMs / 1000);

        if (idleSeconds >= IDLE_THRESHOLD_SECONDS) {
          markIdle(idleSeconds);
        }
      }
    };

    // Initial check
    void checkIdle();

    // Periodic checks
    const intervalId = setInterval(() => void checkIdle(), CHECK_INTERVAL_MS);

    const onActivity = (event?: Event) => {
      lastActivityRef.current = Date.now();

      // Use functional state update to get current state (avoid stale closure)
      setTimerState((currentState) => {
        if (currentState === 'idle') {
          // FEATURE-012: Record activity wake event type
          void idleSyncMetrics.recordActivityWake(event?.type ?? 'unknown');

          setShowIdleModal(false);
          onTimerStateChangeRef.current?.('active', elapsedRef.current);

          const emitStart = performance.now();
          const payload: TimerStateEventV1 = {
            state: 'active',
            elapsed: elapsedRef.current,
            ts: Date.now(),
            source: 'timer',
            v: 1,
          };
          void safeEmit(TIMER_STATE_EVT, payload)
            .then(() => {
              // FEATURE-012: Record event emission latency
              const emitLatencyUs = Math.round((performance.now() - emitStart) * 1000);
              void idleSyncMetrics.recordTimerEventEmission(emitLatencyUs, true);
            })
            .catch(() => {
              void idleSyncMetrics.recordTimerEventEmission(0, false);
            });

          return 'active';
        }
        return currentState;
      });
    };

    // Broader activity coverage with passive listeners (for fallback and UI responsiveness)
    const opts: AddEventListenerOptions = { passive: true };
    window.addEventListener('pointermove', onActivity, opts);
    window.addEventListener('pointerdown', onActivity, opts);
    window.addEventListener('wheel', onActivity, opts);
    window.addEventListener('keydown', onActivity); // not passive

    const onVisibilityChange = () => {
      if (!document.hidden) {
        // Re-check idle state when window becomes visible
        void checkIdle();
      }
    };
    document.addEventListener('visibilitychange', onVisibilityChange);

    // Initialize
    lastActivityRef.current = Date.now();

    return () => {
      clearInterval(intervalId);
      window.removeEventListener('pointermove', onActivity);
      window.removeEventListener('pointerdown', onActivity);
      window.removeEventListener('wheel', onActivity);
      window.removeEventListener('keydown', onActivity);
      document.removeEventListener('visibilitychange', onVisibilityChange);
    };
  }, [timerState]); // ✅ Only timerState, no elapsed

  // FEATURE-012 Phase 5: Listen to user-activity events from tracker window
  useEffect(() => {
    let unlistenActivity: (() => void) | undefined;

    const setupActivityListener = async () => {
      try {
        const { listen: listenFn } = await import('@tauri-apps/api/event');
        unlistenActivity = await listenFn<{ timestamp: number; source: string }>(
          'pulsarc:user-activity:v1',
          () => {
            // Reset activity timestamp when tracker window has user activity
            lastActivityRef.current = Date.now();
          }
        );
      } catch (error) {
        console.error('❌ Failed to setup user-activity listener:', error);
      }
    };

    void setupActivityListener();

    return () => {
      if (unlistenActivity) {
        unlistenActivity();
      }
    };
  }, []);

  // Listen to activity tracker state changes
  useEffect(() => {
    let unlistenTrackerState: (() => void) | undefined;

    const setupTrackerStateListener = async () => {
      try {
        const { listen: listenFn } = await import('@tauri-apps/api/event');
        unlistenTrackerState = await listenFn<TimerStateEventV1>(TIMER_STATE_EVT, (event) => {
          const p = event.payload;

          // Only process events from the activity tracker, ignore our own emissions
          if (p.source !== 'tracker') {
            return;
          }

          console.log(`🔄 Tracker state sync: tracker ${p.state} → timer`);

          // Update timer state based on tracker state
          // Note: Modal handling is done in the source window (where user clicked stop)
          if (p.state === 'inactive') {
            // Capture final elapsed before resetting
            const finalElapsed = elapsedRef.current;

            console.log(`[Timer] Tracker stopped - resetting timer from ${finalElapsed}s to 0`);
            setTimerState('inactive');
            setElapsed(0); // Reset elapsed time when tracker stops
            onTimerStateChangeRef.current?.('inactive', 0);

            // Send final elapsed back to tracker so it can show the modal
            const response: TimerStateEventV1 = {
              state: 'inactive',
              elapsed: finalElapsed,
              ts: Date.now(),
              source: 'timer',
              v: 1,
            };
            void safeEmit(TIMER_STATE_EVT, response);
            console.log(`[Timer] ✅ Sent final elapsed ${finalElapsed}s back to tracker`);
          } else if (p.state === 'active') {
            setTimerState('active');
            onTimerStateChangeRef.current?.('active', elapsedRef.current);
          } else if (p.state === 'paused') {
            setTimerState('paused');
            onTimerStateChangeRef.current?.('paused', elapsedRef.current);
          }
        });
        console.log('✅ Activity tracker state listener registered');
      } catch (error) {
        console.error('❌ Failed to setup tracker state listener:', error);
      }
    };

    void setupTrackerStateListener();

    return () => {
      if (unlistenTrackerState) {
        unlistenTrackerState();
        console.log('🔌 Activity tracker state listener disconnected');
      }
    };
  }, []);

  const handleToggleTimer = useCallback(async () => {
    playSound();
    lastActivityRef.current = Date.now();

    if (timerState === 'inactive' || timerState === 'paused') {
      // Check if there's an in-progress calendar event and set it as default
      if (
        timerState === 'inactive' &&
        nextEvent &&
        nextEvent.minutesUntil !== null &&
        nextEvent.minutesUntil < 0
      ) {
        // There's an in-progress event (minutesUntil is negative for ongoing events)
        try {
          const { timelineService } = await import('../../timeline/services/timelineService');
          const today = new Date();
          const calendarEvents = await timelineService.getCalendarEvents(today);

          const now = Date.now();
          const inProgressEvent = calendarEvents.find((event) => {
            const startTime = event.startEpoch * 1000;
            const endTime = startTime + event.duration * 60 * 1000;
            return startTime <= now && endTime > now;
          });

          if (inProgressEvent) {
            // Set the activity context to the in-progress event
            setActivityContext({
              suggested_client: inProgressEvent.project,
              suggested_matter: null,
              detected_activity: inProgressEvent.task,
              active_app: {
                app_name: inProgressEvent.project,
                window_title: inProgressEvent.task,
                bundle_id: null,
                url: null,
                url_host: null,
                document_name: null,
                file_path: null,
              },
              recent_apps: [],
              work_type: null,
              activity_category: 'meeting',
              billable_confidence: 0.95,
              suggested_task_code: null,
              extracted_metadata: {
                document_name: null,
                file_path: null,
                project_code: null,
                client_identifier: inProgressEvent.project,
                matter_number: null,
                email_subject: null,
              },
              evidence: {
                reasons: ['Calendar event in progress'],
              },
            });
          }
        } catch (error) {
          console.error('Failed to set in-progress event:', error);
        }
      }

      // Starting or resuming
      setTimerState('active');
      onTimerStateChangeRef.current?.('active', elapsedRef.current);

      const emitStart = performance.now();
      const payload: TimerStateEventV1 = {
        state: 'active',
        elapsed: elapsedRef.current,
        ts: Date.now(),
        source: 'timer',
        v: 1,
      };
      void safeEmit(TIMER_STATE_EVT, payload)
        .then(() => {
          const emitLatencyUs = Math.round((performance.now() - emitStart) * 1000);
          void idleSyncMetrics.recordTimerEventEmission(emitLatencyUs, true);
        })
        .catch(() => {
          void idleSyncMetrics.recordTimerEventEmission(0, false);
        });

      showNotification('success', timerState === 'paused' ? 'Timer resumed' : 'Timer started');
    } else {
      // Pausing
      setTimerState('paused');
      onTimerStateChangeRef.current?.('paused', elapsedRef.current);

      const emitStart = performance.now();
      const payload: TimerStateEventV1 = {
        state: 'paused',
        elapsed: elapsedRef.current,
        ts: Date.now(),
        source: 'timer',
        v: 1,
      };
      void safeEmit(TIMER_STATE_EVT, payload)
        .then(() => {
          const emitLatencyUs = Math.round((performance.now() - emitStart) * 1000);
          void idleSyncMetrics.recordTimerEventEmission(emitLatencyUs, true);
        })
        .catch(() => {
          void idleSyncMetrics.recordTimerEventEmission(0, false);
        });

      showNotification('info', 'Timer paused');
    }
  }, [timerState, showNotification]);

  // Keyboard shortcuts
  useEffect(() => {
    const handleKeyPress = (e: KeyboardEvent) => {
      // Space to start/pause
      if (e.code === 'Space' && !e.shiftKey && !e.ctrlKey && !e.metaKey) {
        const target = e.target as HTMLElement;
        if (target.tagName !== 'INPUT' && target.tagName !== 'TEXTAREA') {
          e.preventDefault();
          void handleToggleTimer();
        }
      }

      // Cmd/Ctrl + N for quick entry
      if ((e.metaKey || e.ctrlKey) && e.key === 'n') {
        e.preventDefault();
        playSound();
        onQuickEntry?.();
      }

      // Cmd/Ctrl + E for entries
      if ((e.metaKey || e.ctrlKey) && e.key === 'e') {
        e.preventDefault();
        playSound();
        onEntriesClick?.();
      }

      // Cmd/Ctrl + K for quick switcher
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        playSound();
        setShowQuickSwitcher(!showQuickSwitcher);
      }

      // Esc to close quick switcher
      if (e.key === 'Escape') {
        setShowQuickSwitcher(false);
      }
    };

    window.addEventListener('keydown', handleKeyPress);
    return () => window.removeEventListener('keydown', handleKeyPress);
  }, [showQuickSwitcher, onQuickEntry, onEntriesClick, handleToggleTimer]);

  const handleStop = () => {
    playSound();
    setTimerState('inactive');
    onTimerStateChangeRef.current?.('inactive', 0);

    const payload: TimerStateEventV1 = {
      state: 'inactive',
      elapsed: 0,
      ts: Date.now(),
      source: 'timer',
      v: 1,
    };
    void safeEmit(TIMER_STATE_EVT, payload).catch(() => {
      // Emission failure already logged inside safeEmit
    });

    setSavedElapsed(elapsedRef.current);
    setShowSaveModal(true);
    haptic.light();
  };

  const handleAcceptSuggestion = (data: { project: string; task: string; duration: string }) => {
    const savedTime = savedElapsed;
    setElapsed(0);

    showNotification('success', `Saved: ${data.task} (${data.duration})`);

    // Haptic success feedback
    haptic.success();

    // Confetti for milestones (30min, 1hr, 2hr, etc.)
    if (savedTime >= 7200) {
      // 2+ hours
      celebrateMilestone();
    } else if (savedTime >= 3600) {
      // 1+ hour
      celebrateWithConfetti({ particleCount: 80, spread: 60 });
    } else if (savedTime >= 1800) {
      // 30+ minutes
      celebrateWithConfetti({ particleCount: 50, spread: 50 });
    }
  };

  const handleRejectSuggestion = () => {
    // Discard the time - reset to 0
    setElapsed(0);
    setSavedElapsed(0);
    showNotification('info', 'Time entry discarded');
  };

  const handleQuickProjectSelect = (project: { project: string }) => {
    playSound();
    // Start timer with selected project
    setTimerState('active');
    onTimerStateChangeRef.current?.('active', elapsedRef.current);
    lastActivityRef.current = Date.now();

    const emitStart = performance.now();
    const payload: TimerStateEventV1 = {
      state: 'active',
      elapsed: elapsedRef.current,
      ts: Date.now(),
      source: 'timer',
      v: 1,
    };
    void safeEmit(TIMER_STATE_EVT, payload)
      .then(() => {
        const emitLatencyUs = Math.round((performance.now() - emitStart) * 1000);
        void idleSyncMetrics.recordTimerEventEmission(emitLatencyUs, true);
      })
      .catch(() => {
        void idleSyncMetrics.recordTimerEventEmission(0, false);
      });

    showNotification('success', `Switched to ${project.project}`);
  };

  const handleKeepIdleTime = () => {
    playSound();
    setShowIdleModal(false);
    setTimerState('active');
    onTimerStateChangeRef.current?.('active', elapsedRef.current);
    lastActivityRef.current = Date.now();

    const emitStart = performance.now();
    const payload: TimerStateEventV1 = {
      state: 'active',
      elapsed: elapsedRef.current,
      ts: Date.now(),
      source: 'timer',
      v: 1,
    };
    void safeEmit(TIMER_STATE_EVT, payload)
      .then(() => {
        const emitLatencyUs = Math.round((performance.now() - emitStart) * 1000);
        void idleSyncMetrics.recordTimerEventEmission(emitLatencyUs, true);
      })
      .catch(() => {
        void idleSyncMetrics.recordTimerEventEmission(0, false);
      });

    showNotification('success', `Kept ${idleMinutes}m idle time`);
  };

  const handleDiscardIdleTime = () => {
    playSound();
    const discardedTime = idleMinutes;
    setShowIdleModal(false);
    setElapsed(Math.max(0, elapsed - idleMinutes * 60));

    showNotification('success', `Discarded ${discardedTime}m idle time`);
  };

  const formatDuration = (seconds: number) => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);

    if (hours > 0 && minutes > 0) {
      return `${hours}h ${minutes}m`;
    } else if (hours > 0) {
      return `${hours}h`;
    } else {
      return `${minutes}m`;
    }
  };

  // const handleSyncNow = async () => {
  //   if (isSyncing) return;

  //   setIsSyncing(true);
  //   playSound();

  //   try {
  //     await calendarService.syncNow(); // Sync all providers
  //     showNotification('success', 'Calendar synced successfully');
  //     haptic.success();
  //   } catch (error) {
  //     console.error('Failed to sync calendar:', error);
  //     showNotification('error', 'Failed to sync calendar');
  //     haptic.error();
  //   } finally {
  //     setIsSyncing(false);
  //   }
  // };

  const handleActivitySwitch = () => {
    if (!switchProject || !switchActivity.trim()) {
      return;
    }

    playSound();

    // Update the activity context with the new activity
    setActivityContext({
      suggested_client: switchProject,
      suggested_matter: null,
      detected_activity: switchActivity.trim(),
      active_app: {
        app_name: switchProject,
        window_title: switchActivity.trim(),
        bundle_id: null,
        url: null,
        url_host: null,
        document_name: null,
        file_path: null,
      },
      recent_apps: [],
      work_type: null,
      activity_category: 'client_work',
      billable_confidence: 0.9,
      suggested_task_code: null,
      extracted_metadata: {
        document_name: null,
        file_path: null,
        project_code: null,
        client_identifier: switchProject,
        matter_number: null,
        email_subject: null,
      },
      evidence: {
        reasons: ['Manual activity switch'],
      },
    });

    showNotification('success', `Switched to ${switchProject}`);
    haptic.success();

    // Reset and collapse
    setSwitchProject('');
    setSwitchActivity('');
    setShowSwitchActivity(false);
  };

  const handleBuildMyDayClick = () => {
    playSound();
    haptic.light();
    onBuildMyDayClick?.();
  };

  return (
    <div
      className="relative w-full h-full"
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
    >
      {timerState === 'active' && (
        <motion.div
          className="absolute inset-0 rounded-[2.5rem] bg-black/5 dark:bg-white/5 pointer-events-none"
          animate={{
            scale: [1, 1.02, 1],
            opacity: [0.3, 0.6, 0.3],
          }}
          transition={{
            duration: 2,
            repeat: Infinity,
            ease: 'easeInOut',
          }}
        />
      )}

      <div className="relative w-full h-full backdrop-blur-[24px] flex flex-col">
        {/* In-widget notification - positioned inside timer */}
        <div className="absolute top-4 left-1/2 -translate-x-1/2 z-50 w-full px-4">
          <InWidgetNotification notification={notification} onDismiss={dismiss} />
        </div>

        {/* Invisible drag region - always enabled on hover */}
        {isHovering && (
          <div
            data-tauri-drag-region
            className="absolute inset-0 z-0 cursor-move pointer-events-none"
          />
        )}

        {/* Top Controls */}
        <div className="px-4 pt-4 pb-0 relative z-10">
          <div className="flex items-center justify-between">
            {/* Left: Quick Start, Manual Entry, Build My Day - only visible on hover */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: isHovering ? 1 : 0 }}
              transition={{ duration: 0.2 }}
              className="flex items-center gap-1.5 w-[102px] shrink-0"
            >
              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  playSound();
                  setShowQuickSwitcher(!showQuickSwitcher);
                }}
                onMouseEnter={() => haptic.light()}
                className="h-7 w-7 min-w-7 min-h-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 transform-none"
              >
                <Zap className="w-3.5 h-3.5" />
              </Button>

              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  playSound();
                  onQuickEntry?.();
                }}
                onMouseEnter={() => haptic.light()}
                className="h-7 w-7 min-w-7 min-h-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 transform-none"
              >
                <Plus className="w-3.5 h-3.5" />
              </Button>

              <Button
                variant="ghost"
                size="icon"
                onClick={handleBuildMyDayClick}
                onMouseEnter={() => haptic.light()}
                className="h-7 w-7 min-w-7 min-h-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 transform-none"
                title="Build My Day"
              >
                <Hammer className="w-3.5 h-3.5" />
              </Button>
            </motion.div>

            {/* Center: "Upcoming:" / "IN PROGRESS:" label with drag handler - always visible */}
            {nextEvent && nextEvent.minutesUntil !== null && nextEvent.eventTime !== null ? (
              <div data-tauri-drag-region className="flex-1 text-center px-4 cursor-move">
                <span className="text-sm font-medium text-gray-500 dark:text-gray-400">
                  {formatEventDisplay(nextEvent.minutesUntil, nextEvent.eventTime).prefix.trim()}
                </span>
              </div>
            ) : (
              <div data-tauri-drag-region className="flex-1 text-center px-4 cursor-move" />
            )}

            {/* Right: Entries, Analytics, Settings - only visible on hover */}
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: isHovering ? 1 : 0 }}
              transition={{ duration: 0.2 }}
              className="flex items-center gap-1.5 w-[102px] justify-end shrink-0"
            >
              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  playSound();
                  onEntriesClick?.();
                }}
                onMouseEnter={() => haptic.light()}
                className="h-7 w-7 min-w-7 min-h-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 transform-none"
              >
                <List className="w-3.5 h-3.5" />
              </Button>

              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  playSound();
                  onAnalyticsClick?.();
                }}
                onMouseEnter={() => haptic.light()}
                className="h-7 w-7 min-w-7 min-h-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 transform-none"
              >
                <BarChart3 className="w-3.5 h-3.5" />
              </Button>

              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  playSound();
                  onSettingsClick?.();
                }}
                onMouseEnter={() => haptic.light()}
                className="h-7 w-7 min-w-7 min-h-7 text-gray-700 dark:text-gray-300 hover:bg-white/20 dark:hover:bg-white/10 transform-none"
              >
                <Settings className="w-3.5 h-3.5" />
              </Button>
            </motion.div>
          </div>

          {/* Event text - row below - always visible */}
          {nextEvent && (
            <button
              onClick={() => {
                playSound();
                // Always open timeline
                onTimelineClick?.();
              }}
              className="text-center px-4 mt-1 w-full hover:opacity-80 transition-opacity cursor-pointer"
            >
              <div
                ref={eventContainerRef}
                className="text-sm text-gray-700 dark:text-gray-300 overflow-hidden relative"
              >
                {nextEvent.minutesUntil !== null && nextEvent.eventTime !== null ? (
                  <>
                    {/* Hidden measurement element to detect truncation */}
                    {(() => {
                      const { suffix } = formatEventDisplay(
                        nextEvent.minutesUntil,
                        nextEvent.eventTime
                      );
                      return (
                        <div
                          ref={eventTextRef}
                          className="absolute whitespace-nowrap opacity-0 pointer-events-none"
                          aria-hidden="true"
                        >
                          <span>{nextEvent.title}</span>
                          <span className="text-blue-600 dark:text-blue-500 animate-pulse">
                            {suffix}
                          </span>
                        </div>
                      );
                    })()}

                    <motion.div
                      key={nextEvent.title}
                      initial={{ y: 20, opacity: 0 }}
                      animate={{ y: 0, opacity: 1 }}
                      exit={{ y: -20, opacity: 0 }}
                      transition={{ duration: 0.4, ease: 'easeInOut' }}
                      className={eventTextOverflows ? 'activity-marquee-wrapper' : ''}
                    >
                      {(() => {
                        const { suffix } = formatEventDisplay(
                          nextEvent.minutesUntil,
                          nextEvent.eventTime
                        );
                        return eventTextOverflows ? (
                          <span className="activity-marquee whitespace-nowrap">
                            <span>{nextEvent.title}</span>
                            <span className="text-blue-600 dark:text-blue-500 animate-pulse">
                              {suffix}
                            </span>
                            <span className="mx-4"></span>
                            <span>{nextEvent.title}</span>
                            <span className="text-blue-600 dark:text-blue-500 animate-pulse">
                              {suffix}
                            </span>
                          </span>
                        ) : (
                          <span className="whitespace-nowrap">
                            <span>{nextEvent.title}</span>
                            <span className="text-blue-600 dark:text-blue-500 animate-pulse">
                              {suffix}
                            </span>
                          </span>
                        );
                      })()}
                    </motion.div>
                  </>
                ) : (
                  <motion.div
                    key={`no-event-${phraseIndex}`}
                    initial={{ y: 20, opacity: 0 }}
                    animate={{ y: 0, opacity: 1 }}
                    exit={{ y: -20, opacity: 0 }}
                    transition={{ duration: 0.4, ease: 'easeInOut' }}
                  >
                    <span className="text-gray-500 dark:text-gray-400">{nextEvent.title}</span>
                  </motion.div>
                )}
              </div>
            </button>
          )}
        </div>

        {/* Divider */}
        <div className="px-8 pt-4">
          <div className="h-px bg-gray-300/80 dark:bg-gray-600/60" />
        </div>

        {/* Timer Display - with drag region overlay */}
        <div className="text-center px-8 pt-8 pb-1 relative">
          {/* Invisible drag region - covers timer display for easy dragging */}
          <div
            data-tauri-drag-region
            className="absolute inset-0 cursor-move z-10"
            aria-label="Drag to move window"
          />

          <style>{`
              @keyframes constant-marquee {
                0% { transform: translateX(0); }
                100% { transform: translateX(-50%); }
              }
              .activity-marquee-wrapper {
                overflow: hidden;
                width: 100%;
                position: relative;
              }
              .activity-marquee {
                display: inline-block;
                white-space: nowrap;
                animation: constant-marquee 15s linear infinite;
                will-change: transform;
              }
            `}</style>
          <div className="text-5xl mb-3 tracking-tight text-gray-900 dark:text-gray-50 tabular-nums relative z-0">
            {timerState === 'active' || timerState === 'paused'
              ? timerService.formatTime(elapsed)
              : timerService.formatCurrentTime(currentTime)}
          </div>
          {timerState === 'inactive' && (
            <div className="text-base text-gray-700 dark:text-gray-300">
              {timerService.getGreeting(currentTime)}, Lewis
            </div>
          )}
        </div>

        {/* Control Buttons */}
        <div className={`px-4 pb-4 ${timerState !== 'inactive' ? 'pt-8' : ''}`}>
          {timerState === 'inactive' ? null : timerState === 'paused' ? ( // No start button when inactive - timer starts from Quick Start or Activity Tracker
            <>
              <div className="grid grid-cols-2 gap-2">
                <Button
                  onClick={() => void handleToggleTimer()}
                  variant="outline"
                  className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100"
                >
                  <Play className="w-4 h-4 mr-2" />
                  Resume
                </Button>
                <Button
                  onClick={handleStop}
                  variant="outline"
                  className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100"
                >
                  <Square className="w-4 h-4 mr-2" />
                  Stop
                </Button>
              </div>

              {/* Switch Activity Collapsible */}
              <div className="mt-2">
                <Button
                  onClick={() => setShowSwitchActivity(!showSwitchActivity)}
                  variant="outline"
                  className="w-full backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100"
                >
                  {showSwitchActivity ? 'Hide' : 'Switch Activity'}
                </Button>

                {showSwitchActivity && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: 'auto' }}
                    exit={{ opacity: 0, height: 0 }}
                    transition={{ duration: 0.2 }}
                    className="mt-3 space-y-2"
                  >
                    {/* Project/WBS Search */}
                    <div>
                      <WbsAutocomplete
                        value={switchProject}
                        onChange={setSwitchProject}
                        placeholder="Search projects..."
                      />
                    </div>

                    {/* Activity Description */}
                    <div>
                      <Input
                        value={switchActivity}
                        onChange={(e) => setSwitchActivity(e.target.value)}
                        placeholder="Activity description..."
                        className="text-sm"
                      />
                    </div>

                    {/* Switch Button */}
                    <Button
                      onClick={handleActivitySwitch}
                      disabled={!switchProject || !switchActivity.trim()}
                      variant="outline"
                      className="w-full backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                      size="sm"
                    >
                      Switch
                    </Button>
                  </motion.div>
                )}
              </div>
            </>
          ) : (
            <>
              <div className="grid grid-cols-2 gap-2">
                <Button
                  onClick={() => void handleToggleTimer()}
                  variant="outline"
                  className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100"
                >
                  <Pause className="w-4 h-4 mr-2" />
                  Pause
                </Button>
                <Button
                  onClick={handleStop}
                  variant="outline"
                  className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100"
                >
                  <Square className="w-4 h-4 mr-2" />
                  Stop
                </Button>
              </div>

              {/* Switch Activity Collapsible */}
              <div className="mt-2">
                <Button
                  onClick={() => setShowSwitchActivity(!showSwitchActivity)}
                  variant="outline"
                  className="w-full backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100"
                >
                  {showSwitchActivity ? 'Hide' : 'Switch Activity'}
                </Button>

                {showSwitchActivity && (
                  <motion.div
                    initial={{ opacity: 0, height: 0 }}
                    animate={{ opacity: 1, height: 'auto' }}
                    exit={{ opacity: 0, height: 0 }}
                    transition={{ duration: 0.2 }}
                    className="mt-3 space-y-2"
                  >
                    {/* Project/WBS Search */}
                    <div>
                      <WbsAutocomplete
                        value={switchProject}
                        onChange={setSwitchProject}
                        placeholder="Search projects..."
                      />
                    </div>

                    {/* Activity Description */}
                    <div>
                      <Input
                        value={switchActivity}
                        onChange={(e) => setSwitchActivity(e.target.value)}
                        placeholder="Activity description..."
                        className="text-sm"
                      />
                    </div>

                    {/* Switch Button */}
                    <Button
                      onClick={handleActivitySwitch}
                      disabled={!switchProject || !switchActivity.trim()}
                      variant="outline"
                      className="w-full backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 hover:bg-white/30 dark:hover:bg-white/15 shadow-[0_4px_16px_0_rgba(0,0,0,0.1),0_0_0_1px_rgba(255,255,255,0.5)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset] text-gray-900 dark:text-gray-100 disabled:opacity-50 disabled:cursor-not-allowed"
                      size="sm"
                    >
                      Switch
                    </Button>
                  </motion.div>
                )}
              </div>
            </>
          )}
        </div>

        {/* FEATURE-019: Suggested Entries - only show when timer is inactive */}
        {timerState === 'inactive' && (
          <SuggestedEntries
            onBuildMyDay={handleBuildMyDayClick}
            onCountChange={(count) => {
              setSuggestionCount(count);
            }}
            onCollapseChange={(collapsed) => {
              setIsCollapsed(collapsed);
            }}
            onAcceptEntry={(entry, remainingCount) => {
              playSound();
              showNotification('success', `Added: ${entry.task} (${entry.duration})`);
              haptic.success();
              setSuggestionCount(Math.min(remainingCount, 3));
            }}
            onDismissEntry={(_entryId, remainingCount) => {
              playSound();
              haptic.light();
              setSuggestionCount(Math.min(remainingCount, 3));
            }}
          />
        )}
      </div>

      {/* Quick Project Switcher */}
      <QuickProjectSwitcher
        isOpen={showQuickSwitcher}
        onClose={() => setShowQuickSwitcher(false)}
        onSelect={handleQuickProjectSelect}
      />

      {/* Idle Detection Modal */}
      <IdleDetectionModal
        isOpen={showIdleModal}
        onKeepTime={handleKeepIdleTime}
        onDiscardTime={handleDiscardIdleTime}
        idleMinutes={idleMinutes}
      />

      {/* Save Entry Modal with AI Suggestion */}
      <SaveEntryModal
        isOpen={showSaveModal}
        onClose={() => setShowSaveModal(false)}
        onAccept={handleAcceptSuggestion}
        onReject={handleRejectSuggestion}
        duration={formatDuration(savedElapsed)}
        elapsedSeconds={savedElapsed}
        activityContext={activityContext}
      />
    </div>
  );
}
