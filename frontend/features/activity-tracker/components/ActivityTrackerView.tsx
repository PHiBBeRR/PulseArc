import {
  TIMER_STATE_EVT,
  isTimerStateEventV1,
  safeEmit,
  type TimerStateEventV1,
} from '@/shared/events/timer-events';
import { audioService, idleSyncMetrics } from '@/shared/services';
import { deriveTrackerState } from '@/shared/state/deriveTrackerState';
import { invoke } from '@tauri-apps/api/core';
import { emit, listen } from '@tauri-apps/api/event';
import { AnimatePresence, motion } from 'framer-motion';
import {
  Activity as ActivityIcon,
  ArrowRight,
  Check,
  Lightbulb,
  Pause,
  Play,
  Settings,
  X,
} from 'lucide-react';
import React, { useCallback, useEffect, useRef, useState, type ChangeEvent } from 'react';
import { SaveEntryModal } from '../../time-entry';
import { useSuggestionManager } from '../hooks/useSuggestionManager';
import { type ActivityContext } from '../types';

// Event system constants
const EVENT_ACTIVITY_UPDATED = 'activity-context-updated';
const USER_ACTIVITY_EVT = 'pulsarc:user-activity:v1' as const;

// Types
type TrackerState = 'inactive' | 'active' | 'paused' | 'idle';

type ActivityTrackerSettings = {
  autoResize: boolean;
  sidebarPosition: 'left' | 'right';
};

export function ActivityTrackerView() {
  // Load settings first to determine initial tracker state
  const loadSettings = (): ActivityTrackerSettings => {
    const saved = localStorage.getItem('activityTrackerSettings');
    if (saved) {
      try {
        const settings = JSON.parse(saved) as Partial<ActivityTrackerSettings>;
        return {
          autoResize: settings.autoResize ?? true,
          sidebarPosition: settings.sidebarPosition ?? 'left',
        };
      } catch (e) {
        console.error('Failed to parse settings:', e);
      }
    }
    return {
      autoResize: true,
      sidebarPosition: 'left',
    };
  };

  const initialSettings = loadSettings();

  // State - Always start inactive; user manually starts when ready
  const [trackerState, setTrackerState] = useState<TrackerState>('inactive');
  const [activity, setActivity] = useState<string>('');
  const [activityContext, setActivityContext] = useState<ActivityContext | null>(null);
  const [isLoadingContext, setIsLoadingContext] = useState(false);
  const [hasReceivedInitialContext, setHasReceivedInitialContext] = useState(false);
  const [userHasTyped, setUserHasTyped] = useState(false);
  const [showRecentApps, setShowRecentApps] = useState(false);
  const [showAppDropdown, setShowAppDropdown] = useState(false);
  const [isHovering, setIsHovering] = useState(false);
  const [isWindowFocused, setIsWindowFocused] = useState(true);
  const [autoResize, setAutoResize] = useState(initialSettings.autoResize);
  const [sidebarPosition, setSidebarPosition] = useState<'left' | 'right'>(
    initialSettings.sidebarPosition
  );
  const [showSettings, setShowSettings] = useState(false);
  const [showSaveModal, setShowSaveModal] = useState(false);
  const [savedElapsed, setSavedElapsed] = useState(0);

  // Load settings from localStorage and sync changes
  useEffect(() => {
    // Listen for settings changes
    const handleStorageChange = (e: StorageEvent) => {
      // Handle activity tracker settings (window size, position)
      if (e.key === 'activityTrackerSettings' && e.newValue) {
        try {
          const settings = JSON.parse(e.newValue) as Partial<ActivityTrackerSettings>;
          setAutoResize(settings.autoResize ?? true);
          setSidebarPosition(settings.sidebarPosition ?? 'left');
        } catch (err) {
          console.error('Failed to parse activityTrackerSettings from storage event:', err);
        }
      }
    };

    window.addEventListener('storage', handleStorageChange);

    return () => {
      window.removeEventListener('storage', handleStorageChange);
    };
  }, []);
  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const prevTrackerStateRef = useRef<TrackerState>(trackerState);

  // Keep ref in sync with state
  useEffect(() => {
    prevTrackerStateRef.current = trackerState;
  }, [trackerState]);

  // Play sound feedback
  const playSound = useCallback(() => {
    audioService.playClick();
  }, []);

  // Use the suggestion manager hook
  const { currentSuggestion, clearSuggestion } = useSuggestionManager({
    activityContext,
    inputValue: activity,
    userHasTyped,
    isTracking: trackerState === 'active',
  });

  // Computed values
  const isActive = trackerState === 'active' || trackerState === 'paused';
  const appName = activityContext?.active_app?.app_name ?? 'Pulsarc';

  // Determine if window should be expanded (focused or hovering, and auto-resize is enabled)
  const isExpanded = autoResize ? isWindowFocused || isHovering : true;

  // Fetch activity context
  const fetchContext = useCallback(async () => {
    try {
      setIsLoadingContext(true);
      const context = await invoke<ActivityContext>('fetch_activity_context');
      setActivityContext(context);

      if (!hasReceivedInitialContext) {
        setHasReceivedInitialContext(true);
      }
    } catch (error) {
      console.error('❌ Failed to fetch activity context:', error);
    } finally {
      setIsLoadingContext(false);
    }
  }, [hasReceivedInitialContext]);

  // Listen for pre-fetched activity context
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        unlisten = await listen<ActivityContext>('initial-activity-context', (event) => {
          setActivityContext(event.payload);
          setHasReceivedInitialContext(true);
          setIsLoadingContext(false);
        });
      } catch (error) {
        console.error('Failed to setup initial context listener:', error);
      }
    };

    void setupListener();
    return () => {
      if (unlisten) unlisten();
    };
  }, []);

  // Track window focus/blur
  useEffect(() => {
    const handleFocus = () => {
      setIsWindowFocused(true);
    };

    const handleBlur = () => {
      setIsWindowFocused(false);
    };

    window.addEventListener('focus', handleFocus);
    window.addEventListener('blur', handleBlur);

    return () => {
      window.removeEventListener('focus', handleFocus);
      window.removeEventListener('blur', handleBlur);
    };
  }, []);

  // Click outside handler for app dropdown
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(event.target as Node)) {
        setShowAppDropdown(false);
      }
    };

    if (showAppDropdown) {
      document.addEventListener('mousedown', handleClickOutside);
    }

    return () => {
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [showAppDropdown]);

  // Initial setup
  useEffect(() => {
    setTimeout(() => inputRef.current?.focus(), 100);

    // Set initial window size
    const setInitialSize = async () => {
      try {
        const windowModule = await import('@tauri-apps/api/window');
        const currentWindow = windowModule.getCurrentWindow();
        // Set initial size - 600px width, 180px height (active with 2 buttons: Pause, Settings)
        await currentWindow.setSize(new windowModule.LogicalSize(600, 180));
      } catch (error) {
        console.error('Failed to set initial window size:', error);
      }
    };

    void setInitialSize();

    // Fallback context fetch (only if auto-start is enabled, use session-frozen value)
    const fallbackFetch = setTimeout(() => {
      if (!hasReceivedInitialContext && trackerState === 'active') {
        void fetchContext();
      }
    }, 150);

    return () => clearTimeout(fallbackFetch);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Real-time event listener (pure event-driven, no polling fallback)
  useEffect(() => {
    // Only run when tracker is active
    if (trackerState !== 'active') {
      return;
    }

    // Use a ref to store the unlisten function so cleanup can access it even if async
    // This prevents a race condition where cleanup runs before async listen() completes
    const unlistenRef = { current: undefined as (() => void) | undefined };

    const setupRealtimeListener = async () => {
      try {
        // Setup event listener
        const unlistenFn = await listen<ActivityContext>(EVENT_ACTIVITY_UPDATED, (event) => {
          console.log('📡 Real-time activity update received');
          setActivityContext(event.payload);
          setIsLoadingContext(false);

          if (!hasReceivedInitialContext) {
            setHasReceivedInitialContext(true);
          }
        });

        // Store in ref so cleanup can access it
        unlistenRef.current = unlistenFn;
        console.log('✅ Real-time listener active (event-driven only)');
      } catch (error) {
        console.error('❌ Failed to setup event listener:', error);
      }
    };

    void setupRealtimeListener();

    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        console.log('🔌 Event listener disconnected');
      }
    };
  }, [trackerState, hasReceivedInitialContext]);

  // FIX-011 Issue #1: Tray Menu Event Listeners
  useEffect(() => {
    let unlistenPause: (() => void) | undefined;
    let unlistenStart: (() => void) | undefined;

    const setupTrayListeners = async () => {
      try {
        // Listen for pause event from tray menu
        unlistenPause = await listen('pause-timer', () => {
          setTrackerState('paused');
          setShowRecentApps(false);
          // Keep context for recent apps list
          console.log('⏸️ Paused from tray menu (context retained)');
        });

        // Listen for start/resume event from tray menu
        unlistenStart = await listen('start-timer', () => {
          setTrackerState('active');
          setShowRecentApps(false);
          void fetchContext(); // Refresh activity context
          console.log('▶️ Started from tray menu');
        });

        console.log('✅ Tray menu listeners registered');
      } catch (error) {
        console.error('❌ Failed to setup tray menu listeners:', error);
      }
    };

    void setupTrayListeners();

    return () => {
      if (unlistenPause) {
        unlistenPause();
        console.log('🔌 Tray pause listener disconnected');
      }
      if (unlistenStart) {
        unlistenStart();
        console.log('🔌 Tray start listener disconnected');
      }
    };
  }, []);

  // Timer state synchronization (versioned, namespaced event)
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupTimerStateListener = async () => {
      try {
        // Record listener registration
        unlisten = await listen<TimerStateEventV1>(TIMER_STATE_EVT, (event) => {
          const p = event.payload;
          const receiveTime = Date.now();

          // Validate payload with type guard
          if (!isTimerStateEventV1(p)) {
            console.warn('[timer-state] Invalid or unknown payload version:', p);
            void idleSyncMetrics.recordInvalidPayload();
            return;
          }

          // Record event reception with sync latency
          const syncLatencyMs = Math.max(0, receiveTime - p.ts);
          void idleSyncMetrics.recordTimerEventReception(syncLatencyMs);

          const transitionStart = performance.now();
          const prevState = prevTrackerStateRef.current;
          const newTrackerState = deriveTrackerState(p.state);

          // Record state mirroring (tracker always mirrors timer)
          void idleSyncMetrics.recordAutoStartTrackerRule(
            1, // Rule 1: Always mirror timer state
            p.state,
            false, // autoStartTracker removed
            true // Always correct since we just mirror
          );

          console.log(
            `[Tracker] Event received: source=${p.source}, state=${p.state}, elapsed=${p.elapsed}, currentTrackerState=${trackerState}`
          );

          setTrackerState(newTrackerState);

          // Timer is the source of truth for elapsed time

          // Record state transition with duration
          if (prevState !== newTrackerState) {
            const transitionDurationMs = Math.round(performance.now() - transitionStart);
            void idleSyncMetrics.recordStateTransition(
              prevState,
              newTrackerState,
              transitionDurationMs
            );
            prevTrackerStateRef.current = newTrackerState;
          }

          // Check if timer is responding with final elapsed after tracker stopped
          console.log(
            `[Tracker] Modal check: source=${p.source}, state=${p.state}, elapsed=${p.elapsed}, newTrackerState=${newTrackerState}`
          );
          if (
            p.source === 'timer' &&
            p.state === 'inactive' &&
            p.elapsed > 0 &&
            newTrackerState === 'inactive'
          ) {
            console.log(`📊 [Tracker] SHOWING MODAL with elapsed ${p.elapsed}s`);
            setSavedElapsed(p.elapsed);
            setShowSaveModal(true);
          } else {
            console.log(
              `[Tracker] Modal NOT shown - checks: source=${p.source === 'timer'}, inactive=${p.state === 'inactive'}, hasElapsed=${p.elapsed > 0}, trackerInactive=${newTrackerState === 'inactive'}`
            );
          }

          // Call backend commands to sync tracker state (only if event came from timer)
          if (p.source === 'timer' && prevState !== newTrackerState) {
            if (newTrackerState === 'active') {
              // Start/resume the tracker backend
              invoke('resume_tracker').catch((err) => {
                console.error('❌ Failed to resume tracker from timer event:', err);
              });
              void fetchContext();
            } else if (newTrackerState === 'inactive' || newTrackerState === 'paused') {
              // Pause the tracker backend
              invoke('pause_tracker').catch((err) => {
                console.error('❌ Failed to pause tracker from timer event:', err);
              });
            }
          } else if (newTrackerState === 'active' && p.state === 'active') {
            // Fetch context even if state didn't change but we're active
            void fetchContext();
          }

          console.log(
            `🔄 Timer state sync: ${p.state} → tracker: ${newTrackerState} (source: ${p.source}, elapsed: ${p.elapsed})`
          );
        });
        console.log('✅ Timer state listener registered');
      } catch (error) {
        console.error('❌ Failed to setup timer state listener:', error);
      }
    };

    void setupTimerStateListener();

    return () => {
      if (unlisten) {
        unlisten();
        console.log('🔌 Timer state listener disconnected');
      }
    };
  }, []); // Only set up once on mount, never recreate

  // Cross-window activity wake (emit throttled user-activity events)
  useEffect(() => {
    let lastEmitTime = 0;
    const THROTTLE_MS = 1000; // Max 1 emit/second to prevent event bus flooding

    const onActivity = () => {
      const now = performance.now();
      if (now - lastEmitTime < THROTTLE_MS) return; // Throttle

      lastEmitTime = now;
      void safeEmit(USER_ACTIVITY_EVT, {
        timestamp: Date.now(),
        source: 'tracker',
      }).catch(() => {
        // safeEmit already logs; swallow to prevent unhandled rejection noise
      });
    };

    const opts: AddEventListenerOptions = { passive: true };
    window.addEventListener('pointermove', onActivity, opts);
    window.addEventListener('pointerdown', onActivity, opts);
    window.addEventListener('wheel', onActivity, opts);
    window.addEventListener('keydown', onActivity);

    return () => {
      window.removeEventListener('pointermove', onActivity);
      window.removeEventListener('pointerdown', onActivity);
      window.removeEventListener('wheel', onActivity);
      window.removeEventListener('keydown', onActivity);
    };
  }, []);

  // FIX-011 Issue #2: Multi-Window State Synchronization
  useEffect(() => {
    let unlistenPaused: (() => void) | undefined;
    let unlistenResumed: (() => void) | undefined;

    const setupGlobalListeners = async () => {
      try {
        // Listen for pause events from other windows
        unlistenPaused = await listen('tracker-paused', () => {
          console.log('🌐 Global pause detected - syncing state');
          setTrackerState('paused');
          setShowRecentApps(false);
          // Keep context for recent apps list
        });

        // Listen for resume events from other windows
        unlistenResumed = await listen('tracker-resumed', () => {
          console.log('🌐 Global resume detected - syncing state');
          setTrackerState('active');
          setShowRecentApps(false);
          void fetchContext(); // Fetch fresh context
        });

        console.log('✅ Multi-window sync listeners registered');
      } catch (error) {
        console.error('❌ Failed to setup multi-window sync listeners:', error);
      }
    };

    void setupGlobalListeners();

    return () => {
      if (unlistenPaused) {
        unlistenPaused();
        console.log('🔌 Global pause listener disconnected');
      }
      if (unlistenResumed) {
        unlistenResumed();
        console.log('🔌 Global resume listener disconnected');
      }
    };
  }, []);

  // Dynamic window sizing based on tracker state and expansion
  useEffect(() => {
    const resizeWindow = async () => {
      try {
        const windowModule = await import('@tauri-apps/api/window');
        const currentWindow = windowModule.getCurrentWindow();

        // Determine width based on expansion state
        const width = isExpanded ? 600 : 80; // 80px for sidebar only

        // Determine height based on tracker state
        let height = 180; // Default for active/paused/idle
        if (trackerState === 'inactive') {
          height = 120;
        } else if (trackerState === 'idle') {
          height = 180; // Same as active/paused
        }

        await currentWindow.setSize(new windowModule.LogicalSize(width, height));
      } catch (error) {
        console.error('Failed to resize window:', error);
      }
    };

    void resizeWindow();
  }, [trackerState, isExpanded]);

  // Event handlers
  const handleToggleLive = async () => {
    playSound();

    if (trackerState === 'inactive') {
      try {
        await invoke('resume_tracker');
        setTrackerState('active');
        setShowRecentApps(false);

        // Emit TIMER_STATE_EVT for cross-window sync
        const payload: TimerStateEventV1 = {
          state: 'active',
          elapsed: 0,
          ts: Date.now(),
          source: 'tracker',
          v: 1,
        };
        void safeEmit(TIMER_STATE_EVT, payload);

        await emit('tracker-resumed');
        void fetchContext();
      } catch (error) {
        console.error('❌ Failed to resume tracker from Live toggle:', error);
      }
      return;
    }

    // If active or paused, stop the tracker
    await handleStop();
  };

  const handlePause = async () => {
    if (trackerState === 'active') {
      // Call backend pause command
      await invoke('pause_tracker');
      setTrackerState('paused');
      setShowRecentApps(false);

      // Emit TIMER_STATE_EVT for cross-window sync
      const payload: TimerStateEventV1 = {
        state: 'paused',
        elapsed: 0,
        ts: Date.now(),
        source: 'tracker',
        v: 1,
      };
      void safeEmit(TIMER_STATE_EVT, payload);

      // Keep activity context for showing recent apps when user clicks "No"
      // Don't clear it - it's needed for the recent apps list

      // FIX-011 Issue #2: Emit global event for multi-window sync
      await emit('tracker-paused');
      console.log('⏸️ Tracker paused (context retained for recent apps)');
    } else if (trackerState === 'paused') {
      // Call backend resume command
      await invoke('resume_tracker');
      setTrackerState('active');
      setShowRecentApps(false);

      // Emit TIMER_STATE_EVT for cross-window sync
      const payload: TimerStateEventV1 = {
        state: 'active',
        elapsed: 0,
        ts: Date.now(),
        source: 'tracker',
        v: 1,
      };
      void safeEmit(TIMER_STATE_EVT, payload);

      // FIX-011 Issue #3: Fetch fresh context when resuming
      void fetchContext();

      // FIX-011 Issue #2: Emit global event for multi-window sync
      await emit('tracker-resumed');
      console.log('▶️ Tracker resumed (fetching fresh context)');
    }
    playSound();
  };

  const handleStop = async () => {
    // Stop the tracker backend
    await invoke('pause_tracker');

    setTrackerState('inactive');
    setActivity('');
    setUserHasTyped(false);
    setShowRecentApps(false);

    // Emit stop request to timer - timer will respond with final elapsed
    const payload: TimerStateEventV1 = {
      state: 'inactive',
      elapsed: 0, // Timer will send back the actual final elapsed
      ts: Date.now(),
      source: 'tracker',
      v: 1,
    };
    void safeEmit(TIMER_STATE_EVT, payload);

    // Emit global event for multi-window sync
    await emit('tracker-stopped');
    console.log('⏹️ Tracker stopped - waiting for timer elapsed response');
    playSound();

    // Modal will be shown when timer responds with final elapsed
  };

  const handleYesStillWorking = () => {
    setTrackerState('active');
    setShowRecentApps(false);

    // Emit TIMER_STATE_EVT for cross-window sync
    const payload: TimerStateEventV1 = {
      state: 'active',
      elapsed: 0,
      ts: Date.now(),
      source: 'tracker',
      v: 1,
    };
    void safeEmit(TIMER_STATE_EVT, payload);

    playSound();
  };

  const handleNoNotWorking = () => {
    setShowRecentApps(true);
    playSound();
  };

  const handleSelectRecentApp = async (appName: string) => {
    // Set the activity to the selected app
    const activityText = `Working on ${appName}`;
    setActivity(activityText);
    setUserHasTyped(true);
    setShowRecentApps(false);

    // Optimistically update the activity context to reflect the selected app
    if (activityContext) {
      setActivityContext({
        ...activityContext,
        active_app: {
          app_name: appName,
          window_title: activityContext.active_app?.window_title ?? '',
          bundle_id: activityContext.active_app?.bundle_id ?? null,
          url: activityContext.active_app?.url ?? null,
          url_host: activityContext.active_app?.url_host ?? null,
          document_name: activityContext.active_app?.document_name ?? null,
          file_path: activityContext.active_app?.file_path ?? null,
        },
        detected_activity: activityText,
      });
    }

    // Save the manual activity entry to the database
    try {
      await invoke('save_time_entry', { description: activityText });
      console.log(`💾 Manual time entry saved: ${activityText}`);
    } catch (err) {
      console.error('❌ Failed to save manual time entry:', err);
    }

    // Resume tracker with the selected activity
    await invoke('resume_tracker');
    setTrackerState('active');

    // Emit TIMER_STATE_EVT for cross-window sync
    const payload: TimerStateEventV1 = {
      state: 'active',
      elapsed: 0,
      ts: Date.now(),
      source: 'tracker',
      v: 1,
    };
    void safeEmit(TIMER_STATE_EVT, payload);

    // Emit global event for multi-window sync
    await emit('tracker-resumed');

    // Fetch fresh context when resuming (will update with real data)
    void fetchContext();

    playSound();
    console.log(`✅ Resumed tracking: ${activityText}`);
  };

  const handleToggleAppDropdown = (e: React.MouseEvent) => {
    e.stopPropagation();
    setShowAppDropdown(!showAppDropdown);
    playSound();
  };

  const handleSelectAppFromDropdown = (appName: string) => {
    setActivity(`Working on ${appName}`);
    setUserHasTyped(true);
    setShowAppDropdown(false);
    playSound();
  };

  const handleClose = useCallback(async () => {
    playSound();
    try {
      const windowModule = await import('@tauri-apps/api/window');
      const currentWindow = windowModule.getCurrentWindow();
      await currentWindow.hide();
    } catch (err) {
      console.error('❌ Error hiding window:', err);
    }
  }, [playSound]);

  const handleSubmit = (e?: React.FormEvent) => {
    e?.preventDefault();
    if (!activity.trim()) return;

    // Activity submitted - could send to backend here
    setActivity('');
    setUserHasTyped(false);
    playSound();
  };

  const handleInputChange = (e: ChangeEvent<HTMLInputElement>) => {
    const newValue = e.target.value;
    setActivity(newValue);

    if (newValue.length > 0 && !userHasTyped) {
      setUserHasTyped(true);
    }
    if (newValue.length === 0 && userHasTyped) {
      setUserHasTyped(false);
    }
  };

  const handleUseSuggestion = () => {
    if (!currentSuggestion) return;
    setActivity(currentSuggestion.text);
    playSound();
    // Auto-submit
    setTimeout(() => handleSubmit(), 100);
  };

  const handleDismissSuggestion = () => {
    clearSuggestion();
    playSound();
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

  const handleAcceptSuggestion = () => {
    setShowSaveModal(false);
    setActivityContext(null);
    playSound();
    // Entry is saved by the SaveEntryModal component
    // Timer has already reset elapsed to 0
  };

  const handleRejectSuggestion = () => {
    setShowSaveModal(false);
    setActivityContext(null);
    playSound();
    // Timer has already reset elapsed to 0
  };

  // Determine header text based on state
  const getHeaderText = () => {
    if (trackerState === 'inactive') {
      return (
        <span className="text-white/50">
          Describe what you're doing or click start and I'll handle it
        </span>
      );
    } else if (trackerState === 'paused') {
      return (
        <span className="text-white/50">
          I'm paused, take a breath, I'll be here when you return
        </span>
      );
    } else {
      return (
        <span className="text-white/50">
          What are you doing in{' '}
          <motion.span
            className="text-red-400 cursor-pointer hover:text-red-300 transition-colors"
            animate={{ opacity: [1, 0.6, 1] }}
            transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
            onClick={handleToggleAppDropdown}
          >
            {appName}
          </motion.span>
          ?
        </span>
      );
    }
  };

  // Determine arrow button styles
  const getArrowButtonClasses = () => {
    const hasContent = activity.trim();

    if (!hasContent) {
      return 'bg-white/10';
    }

    if (trackerState === 'inactive') {
      return 'bg-white/20 shadow-[0_0_12px_rgba(255,255,255,0.3)]';
    }

    if (trackerState === 'paused') {
      return 'bg-yellow-500 shadow-[0_2px_12px_rgba(234,179,8,0.3)]';
    }

    return 'bg-red-500 shadow-[0_2px_12px_rgba(239,68,68,0.3)]';
  };

  return (
    <motion.div
      className="relative w-full h-full rounded-[24px] overflow-hidden"
      onMouseEnter={() => setIsHovering(true)}
      onMouseLeave={() => setIsHovering(false)}
      layout
      transition={{ type: 'spring', stiffness: 200, damping: 25 }}
    >
      {/* Invisible drag region overlay */}
      <div data-tauri-drag-region className="absolute top-0 left-0 right-0 h-12 cursor-move z-50" />

      {/* Close button - top right */}
      {isHovering && (
        <button
          onClick={() => void handleClose()}
          className="absolute top-3 right-3 z-[60] h-6 w-6 rounded-md flex items-center justify-center text-white/60 hover:text-white/90 hover:bg-white/10 transition-all"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      )}

      <motion.div
        className="relative w-full h-full backdrop-blur-[24px] flex flex-col rounded-[24px] overflow-hidden"
        layout
        transition={{ type: 'spring', stiffness: 200, damping: 25 }}
      >
        {/* Subtle glow from sidebar when active/paused/idle/inactive */}
        <AnimatePresence>
          {(isActive || trackerState === 'idle' || trackerState === 'inactive') && (
            <motion.div
              className="absolute pointer-events-none overflow-hidden"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.5 }}
              style={{
                // Position based on sidebar location
                left: sidebarPosition === 'left' ? 0 : undefined,
                right: sidebarPosition === 'right' ? 0 : undefined,
                top: 0,
                bottom: isExpanded ? 0 : undefined,
                width: isExpanded ? '200px' : '80px',
                height: isExpanded ? undefined : '120px',
                background:
                  trackerState === 'idle'
                    ? isExpanded
                      ? sidebarPosition === 'left'
                        ? 'linear-gradient(to right, rgba(14, 165, 233, 0.15), transparent)'
                        : 'linear-gradient(to left, rgba(14, 165, 233, 0.15), transparent)'
                      : 'linear-gradient(to bottom, rgba(14, 165, 233, 0.15), transparent)'
                    : trackerState === 'inactive'
                      ? isExpanded
                        ? sidebarPosition === 'left'
                          ? 'linear-gradient(to right, rgba(255, 255, 255, 0.1), transparent)'
                          : 'linear-gradient(to left, rgba(255, 255, 255, 0.1), transparent)'
                        : 'linear-gradient(to bottom, rgba(255, 255, 255, 0.1), transparent)'
                      : trackerState === 'paused'
                        ? isExpanded
                          ? sidebarPosition === 'left'
                            ? 'linear-gradient(to right, rgba(234, 179, 8, 0.15), transparent)'
                            : 'linear-gradient(to left, rgba(234, 179, 8, 0.15), transparent)'
                          : 'linear-gradient(to bottom, rgba(234, 179, 8, 0.15), transparent)'
                        : isExpanded
                          ? sidebarPosition === 'left'
                            ? 'linear-gradient(to right, rgba(239, 68, 68, 0.15), transparent)'
                            : 'linear-gradient(to left, rgba(239, 68, 68, 0.15), transparent)'
                          : 'linear-gradient(to bottom, rgba(239, 68, 68, 0.15), transparent)',
                borderRadius: '24px',
              }}
            />
          )}
        </AnimatePresence>

        {/* Main horizontal layout */}
        <div
          className={`flex h-full flex-1 ${sidebarPosition === 'right' ? 'flex-row-reverse' : ''}`}
        >
          {/* Sidebar - flexible but min width */}
          <div
            className={`w-20 min-w-[80px] flex-shrink-0 flex flex-col items-center justify-start p-3 transition-all duration-500 group/sidebar relative ${
              isExpanded
                ? sidebarPosition === 'left'
                  ? 'border-r border-white/5'
                  : 'border-l border-white/5'
                : ''
            }`}
          >
            {/* Activity Icon */}
            <button
              onClick={() => {
                void handleToggleLive();
              }}
              className="flex flex-col items-center gap-1.5 cursor-pointer"
              style={{ marginTop: '4px' }}
            >
              <motion.div
                className="p-3 bg-white/10 rounded-2xl border border-white/20 backdrop-blur-sm"
                animate={
                  isActive || trackerState === 'inactive'
                    ? {
                        scale: [1, 1.05, 1],
                        opacity: [1, 0.8, 1],
                      }
                    : {}
                }
                transition={
                  isActive || trackerState === 'inactive'
                    ? {
                        duration: 2,
                        repeat: Infinity,
                        ease: 'easeInOut',
                      }
                    : {}
                }
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
              >
                <ActivityIcon
                  className={`w-5 h-5 transition-colors duration-500 ${
                    trackerState === 'idle'
                      ? 'text-sky-500'
                      : trackerState === 'paused'
                        ? 'text-yellow-500'
                        : trackerState === 'active'
                          ? 'text-red-500'
                          : 'text-white/90'
                  }`}
                  strokeWidth={2.5}
                />
              </motion.div>
              <span
                className={`text-[9px] uppercase tracking-wider transition-colors duration-500 ${
                  trackerState === 'idle'
                    ? 'text-sky-400'
                    : trackerState === 'paused'
                      ? 'text-yellow-400'
                      : trackerState === 'active'
                        ? 'text-red-400'
                        : 'text-white/90'
                }`}
              >
                {trackerState === 'idle'
                  ? 'Idle'
                  : trackerState === 'paused'
                    ? 'Paused'
                    : trackerState === 'active'
                      ? 'Live'
                      : 'Start'}
              </span>
            </button>

            {/* Spacer */}
            {isActive && <div className="flex-1" />}

            {/* Control buttons - only visible when active/paused */}
            {isActive && (
              <div className="flex flex-col items-center gap-2">
                {/* Pause/Resume button */}
                <button
                  onClick={() => void handlePause()}
                  aria-label={trackerState === 'paused' ? 'Resume' : 'Pause'}
                  className={`h-8 w-8 rounded-xl flex items-center justify-center transition-all duration-300 hover:scale-105 active:scale-95 ${
                    isExpanded
                      ? 'opacity-0 group-hover/sidebar:opacity-100 translate-y-4 group-hover/sidebar:translate-y-0'
                      : 'opacity-100'
                  } ${
                    trackerState === 'paused'
                      ? 'bg-yellow-500/80 text-white shadow-[0_2px_12px_rgba(234,179,8,0.3)]'
                      : 'bg-red-500/80 text-white shadow-[0_2px_12px_rgba(239,68,68,0.3)]'
                  }`}
                >
                  {trackerState === 'paused' ? (
                    <Play className="w-3 h-3" />
                  ) : (
                    <Pause className="w-3 h-3" />
                  )}
                </button>

                {/* Settings button */}
                <button
                  onClick={() => {
                    setShowSettings(!showSettings);
                    playSound();
                  }}
                  className={`h-8 w-8 rounded-xl bg-white/10 hover:bg-white/20 text-white flex items-center justify-center transition-all duration-300 hover:scale-105 active:scale-95 ${
                    isExpanded
                      ? 'opacity-0 group-hover/sidebar:opacity-100 translate-y-4 group-hover/sidebar:translate-y-0'
                      : 'opacity-100'
                  } ${showSettings ? 'bg-white/20' : ''}`}
                >
                  <Settings className="w-3 h-3" />
                </button>
              </div>
            )}
          </div>

          {/* Content Area - flexible, takes remaining space - hidden when minimized */}
          {isExpanded && (
            <div className="flex-1 min-w-0 p-4 flex flex-col space-y-3">
              {showSettings ? (
                /* Settings View */
                <div className="space-y-2">
                  <div className="flex items-center gap-1.5">
                    <Settings className="w-3 h-3 text-white/60" />
                    <span className="text-xs tracking-wide text-white/50 uppercase">Settings</span>
                  </div>

                  {/* Settings List */}
                  <div className="space-y-1.5">
                    {/* Sidebar Position Setting */}
                    <div className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 transition-all">
                      <div className="flex items-center justify-between gap-3">
                        <div className="flex-1 min-w-0">
                          <div className="text-sm text-white/90 font-medium truncate">
                            Action Bar Position
                          </div>
                          <div className="text-xs text-white/50 truncate mt-0.5">
                            {sidebarPosition === 'left' ? 'Left side' : 'Right side'}
                          </div>
                        </div>
                        <button
                          onClick={() => {
                            const newValue = sidebarPosition === 'left' ? 'right' : 'left';
                            setSidebarPosition(newValue);
                            const settings: ActivityTrackerSettings = {
                              autoResize,
                              sidebarPosition: newValue,
                            };
                            localStorage.setItem(
                              'activityTrackerSettings',
                              JSON.stringify(settings)
                            );
                            playSound();
                          }}
                          className="px-3 py-1.5 rounded-md bg-white/10 hover:bg-white/20 text-white text-xs font-medium transition-all flex-shrink-0"
                        >
                          {sidebarPosition === 'left' ? 'Left' : 'Right'}
                        </button>
                      </div>
                    </div>

                    {/* Auto-resize Setting */}
                    <div className="w-full px-3 py-2 rounded-lg bg-white/5 border border-white/10 transition-all">
                      <div className="flex items-center justify-between gap-3">
                        <div className="flex-1 min-w-0">
                          <div className="text-sm text-white/90 font-medium truncate">
                            Auto-resize
                          </div>
                          <div className="text-xs text-white/50 truncate mt-0.5">
                            Minimize when unfocused
                          </div>
                        </div>
                        <button
                          onClick={() => {
                            const newValue = !autoResize;
                            setAutoResize(newValue);
                            const settings: ActivityTrackerSettings = {
                              autoResize: newValue,
                              sidebarPosition,
                            };
                            localStorage.setItem(
                              'activityTrackerSettings',
                              JSON.stringify(settings)
                            );
                            playSound();
                          }}
                          className={`relative h-6 w-11 rounded-full transition-colors flex-shrink-0 ${
                            autoResize ? 'bg-green-500' : 'bg-white/20'
                          }`}
                        >
                          <motion.div
                            className="absolute top-0.5 left-0.5 h-5 w-5 rounded-full bg-white shadow-md"
                            animate={{ x: autoResize ? 20 : 0 }}
                            transition={{ type: 'spring', stiffness: 500, damping: 30 }}
                          />
                        </button>
                      </div>
                    </div>
                  </div>
                </div>
              ) : (
                /* Normal Content */
                <>
                  {/* Dynamic header text with dropdown */}
                  <div className="relative" ref={dropdownRef}>
                    <AnimatePresence mode="wait">
                      <motion.div
                        key={trackerState}
                        className="text-sm"
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: 10 }}
                        transition={{ duration: 0.6, ease: [0.4, 0, 0.2, 1] }}
                      >
                        {getHeaderText()}
                      </motion.div>
                    </AnimatePresence>

                    {/* App Dropdown */}
                    <AnimatePresence>
                      {showAppDropdown &&
                        activityContext?.recent_apps &&
                        activityContext.recent_apps.length > 0 && (
                          <motion.div
                            className="absolute top-full left-0 right-0 mt-2 z-50"
                            initial={{ opacity: 0, y: -10, scale: 0.95 }}
                            animate={{ opacity: 1, y: 0, scale: 1 }}
                            exit={{ opacity: 0, y: -10, scale: 0.95 }}
                            transition={{ duration: 0.2 }}
                          >
                            <div className="bg-black/80 backdrop-blur-xl border border-white/20 rounded-xl p-2 shadow-xl">
                              <div className="text-[10px] text-white/50 uppercase tracking-wide px-2 py-1 mb-1">
                                Switch to Recent App
                              </div>
                              <div className="space-y-1">
                                {activityContext.recent_apps.slice(0, 3).map((app, index) => (
                                  <button
                                    key={index}
                                    onClick={() => handleSelectAppFromDropdown(app.app_name)}
                                    className="w-full px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 hover:border-red-500/40 text-left transition-all group"
                                  >
                                    <div className="flex items-center justify-between">
                                      <div className="flex-1 min-w-0">
                                        <div className="text-sm text-white/90 font-medium truncate">
                                          {app.app_name}
                                        </div>
                                        {app.window_title && (
                                          <div className="text-xs text-white/50 truncate mt-0.5">
                                            {app.window_title}
                                          </div>
                                        )}
                                      </div>
                                      <ArrowRight className="w-3.5 h-3.5 text-red-400 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0 ml-2" />
                                    </div>
                                  </button>
                                ))}
                              </div>
                            </div>
                          </motion.div>
                        )}
                    </AnimatePresence>
                  </div>

                  {/* Input field with arrow button */}
                  <form onSubmit={handleSubmit} className="relative">
                    <input
                      ref={inputRef}
                      type="text"
                      value={activity}
                      onChange={handleInputChange}
                      placeholder="e.g., Reviewing time entries, Debugging timer logic..."
                      className={`w-full h-11 px-4 pr-11 rounded-[1.5rem] bg-white/10 border-2 transition-all duration-200 text-white placeholder-white/40 text-sm focus:outline-none ${
                        trackerState === 'paused'
                          ? 'border-white/20 focus:border-yellow-500/40'
                          : trackerState === 'active'
                            ? 'border-white/20 focus:border-red-500/40'
                            : 'border-white/20 focus:border-white/30'
                      }`}
                    />
                    <button
                      type="submit"
                      disabled={!activity.trim()}
                      className={`absolute right-1.5 top-1/2 -translate-y-1/2 h-8 w-8 rounded-xl flex items-center justify-center transition-all duration-500 ${getArrowButtonClasses()} disabled:opacity-50 disabled:cursor-not-allowed`}
                    >
                      <ArrowRight className="w-3.5 h-3.5 text-white" />
                    </button>
                  </form>

                  {/* Paused state: Yes/No buttons or Recent Apps */}
                  <AnimatePresence>
                    {trackerState === 'paused' && !showRecentApps && (
                      <motion.div
                        className="space-y-2"
                        initial={{ opacity: 0, y: -10 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0, y: 10 }}
                        transition={{ duration: 0.3 }}
                      >
                        {/* Question text */}
                        <div className="flex items-center gap-1.5">
                          <ActivityIcon className="w-3 h-3 text-yellow-500/60" />
                          <span className="text-xs tracking-wide text-white/50 uppercase">
                            Are you still working on{' '}
                            <motion.span
                              className="text-yellow-400 cursor-pointer hover:text-yellow-300 transition-colors normal-case"
                              animate={{ opacity: [1, 0.6, 1] }}
                              transition={{ duration: 2, repeat: Infinity, ease: 'easeInOut' }}
                              onClick={handleToggleAppDropdown}
                            >
                              {appName}
                            </motion.span>
                            ?
                          </span>
                        </div>

                        {/* Yes/No buttons */}
                        <div className="flex gap-2">
                          <motion.button
                            onClick={handleYesStillWorking}
                            className="flex-1 h-10 rounded-xl bg-green-500/20 hover:bg-green-500/30 border border-green-500/40 text-green-400 flex items-center justify-center gap-2 transition-all"
                            whileHover={{ scale: 1.02 }}
                            whileTap={{ scale: 0.98 }}
                          >
                            <Check className="w-4 h-4" />
                            <span className="text-sm font-medium">Yes</span>
                          </motion.button>
                          <motion.button
                            onClick={handleNoNotWorking}
                            className="flex-1 h-10 rounded-xl bg-red-500/20 hover:bg-red-500/30 border border-red-500/40 text-red-400 flex items-center justify-center gap-2 transition-all"
                            whileHover={{ scale: 1.02 }}
                            whileTap={{ scale: 0.98 }}
                          >
                            <X className="w-4 h-4" />
                            <span className="text-sm font-medium">No</span>
                          </motion.button>
                        </div>
                      </motion.div>
                    )}

                    {/* Recent Apps Selection */}
                    {trackerState === 'paused' &&
                      showRecentApps &&
                      activityContext?.recent_apps && (
                        <motion.div
                          className="space-y-2"
                          initial={{ opacity: 0, height: 0 }}
                          animate={{ opacity: 1, height: 'auto' }}
                          exit={{ opacity: 0, height: 0 }}
                          transition={{ duration: 0.3 }}
                        >
                          <div className="flex items-center gap-1.5">
                            <ActivityIcon className="w-3 h-3 text-yellow-500/60" />
                            <span className="text-xs tracking-wide text-white/50 uppercase">
                              Recent Apps
                            </span>
                          </div>
                          <div className="space-y-1.5 max-h-48 overflow-y-auto">
                            {activityContext.recent_apps.slice(0, 3).map((app, index) => (
                              <motion.button
                                key={index}
                                onClick={() => void handleSelectRecentApp(app.app_name)}
                                className="w-full px-3 py-2 rounded-lg bg-white/5 hover:bg-white/10 border border-white/10 hover:border-yellow-500/40 text-left transition-all group"
                                initial={{ opacity: 0, x: -10 }}
                                animate={{ opacity: 1, x: 0 }}
                                transition={{ delay: index * 0.05 }}
                              >
                                <div className="flex items-center justify-between">
                                  <div className="flex-1 min-w-0">
                                    <div className="text-sm text-white/90 font-medium truncate">
                                      {app.app_name}
                                    </div>
                                    {app.window_title && (
                                      <div className="text-xs text-white/50 truncate mt-0.5">
                                        {app.window_title}
                                      </div>
                                    )}
                                  </div>
                                  <ArrowRight className="w-3.5 h-3.5 text-yellow-400 opacity-0 group-hover:opacity-100 transition-opacity flex-shrink-0 ml-2" />
                                </div>
                              </motion.button>
                            ))}
                          </div>
                        </motion.div>
                      )}

                    {/* Suggestions section - only show when active */}
                    {trackerState === 'active' && currentSuggestion && (
                      <motion.div
                        className="space-y-1.5"
                        initial={{ opacity: 0, height: 0 }}
                        animate={{ opacity: 1, height: 'auto' }}
                        exit={{ opacity: 0, height: 0 }}
                        transition={{ duration: 0.6, ease: [0.4, 0, 0.2, 1] }}
                      >
                        {/* Header */}
                        <div className="flex items-center gap-1.5">
                          <Lightbulb className="w-3 h-3 text-red-500/60" />
                          <span className="text-xs tracking-wide text-white/50 uppercase">
                            Suggested Activity
                          </span>
                        </div>

                        {/* Suggestion card */}
                        <motion.div
                          className="px-2 py-1.5 rounded-lg bg-white/5 border border-white/10 hover:border-white/20 transition-colors flex items-center justify-between gap-2 group/card"
                          initial={{ opacity: 0, x: -10 }}
                          animate={{ opacity: 1, x: 0 }}
                          exit={{ opacity: 0, x: 10 }}
                          transition={{
                            duration: 0.6,
                            ease: [0.4, 0, 0.2, 1],
                          }}
                        >
                          {/* Activity text */}
                          <span className="text-xs text-white/90 truncate flex-1 min-w-0">
                            {currentSuggestion.text}
                          </span>

                          {/* Use button */}
                          <motion.button
                            onClick={handleUseSuggestion}
                            className="h-6 px-2 text-[10px] text-white rounded-md bg-red-500 hover:bg-red-600 transition-all duration-500 flex-shrink-0"
                            whileHover={{ scale: 1.05 }}
                            whileTap={{ scale: 0.95 }}
                          >
                            Use
                          </motion.button>

                          {/* Dismiss button */}
                          <motion.button
                            onClick={handleDismissSuggestion}
                            className="h-6 w-6 rounded-md bg-white/5 hover:bg-white/10 text-white/60 flex items-center justify-center flex-shrink-0 transition-colors"
                            whileHover={{ scale: 1.05 }}
                            whileTap={{ scale: 0.95 }}
                          >
                            <X className="w-2.5 h-2.5" />
                          </motion.button>
                        </motion.div>
                      </motion.div>
                    )}

                    {/* Empty state when active but no suggestions */}
                    {trackerState === 'active' && !currentSuggestion && !isLoadingContext && (
                      <motion.div
                        className="flex flex-col items-center justify-center py-2 space-y-1"
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0 }}
                      >
                        <span className="text-xs text-white/40">Analyzing app activity...</span>
                        <span className="text-[10px] text-white/30">
                          AI will suggest tasks based on your patterns
                        </span>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </>
              )}
            </div>
          )}
        </div>
      </motion.div>

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
    </motion.div>
  );
}
