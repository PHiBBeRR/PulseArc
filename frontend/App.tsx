import { invoke } from '@tauri-apps/api/core';
import { lazy, Suspense, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ActivityTrackerView } from './features/activity-tracker';
import { CompactQuickEntry } from './features/time-entry';
import { MainTimer } from './features/timer';
import './globals.css';
import { ThemeProvider } from './shared/components/layout';
import { TooltipProvider } from './shared/components/ui/tooltip';
import { projectCache } from './shared/services';
import { TauriAPI, TauriEvents } from './shared/services/ipc';

// Lazy load heavy components for better initial load and view switching
const EntriesView = lazy(() =>
  import('./features/time-entry').then((m) => ({ default: m.EntriesView }))
);
const SettingsView = lazy(() =>
  import('./features/settings').then((m) => ({ default: m.SettingsView }))
);
const AnalyticsView = lazy(() =>
  import('./features/analytics').then((m) => ({ default: m.AnalyticsView }))
);
const TimelineDayView = lazy(() =>
  import('./features/timeline').then((m) => ({ default: m.TimelineDayView }))
);
const BuildMyDayView = lazy(() =>
  import('./features/build-my-day').then((m) => ({ default: m.BuildMyDayView }))
);
const CompactErrorAlert = lazy(() =>
  import('./shared/components/feedback').then((m) => ({ default: m.CompactErrorAlert }))
);

// Loading component for Suspense fallback
const ViewLoader = () => (
  <div className="min-h-screen flex items-center justify-center bg-transparent">
    <div className="backdrop-blur-xl bg-white/10 dark:bg-white/5 border border-white/20 dark:border-white/10 rounded-2xl p-6">
      <div className="flex items-center gap-3">
        <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
        <span className="text-sm text-gray-700 dark:text-gray-300">Loading...</span>
      </div>
    </div>
  </div>
);

type ViewType = 'timer' | 'entries' | 'settings' | 'analytics' | 'timeline' | 'build';
type SyncStatus = 'idle' | 'syncing' | 'synced' | 'error';

// View-specific window configurations
const ViewSizeConfig = {
  timer: { width: 420, height: 300, resizable: false, maxHeight: 560 }, // Dynamic height managed by MainTimer
  entries: {
    day: { width: 680, height: 620, resizable: true }, // Day view - compact size
    week: { width: 790, height: 410, resizable: false }, // Week view - 7-day grid (fixed size)
  },
  settings: { width: 580, height: 450, resizable: false }, // Compact for settings
  analytics: { width: 580, height: 1025, resizable: false }, // Tall for charts - locked size
  timeline: {
    day: { width: 680, height: 720, resizable: true }, // Day view - standard size
    week: { width: 1450, height: 720, resizable: true }, // Week view - full calendar grid (min-w-[1400px])
  },
  build: { width: 680, height: 620, resizable: true }, // Build My Day view - same as entries day view
} as const;

function AppContent() {
  // IMMEDIATE DETECTION CHECK - runs before any other code
  const urlParams = new URLSearchParams(window.location.search);
  const urlHash = window.location.hash;
  const viewParam = urlParams.get('view');

  console.warn('🚀 App Loading:', {
    href: window.location.href,
    search: window.location.search,
    hash: urlHash,
    viewParam: viewParam,
    isActivityTracker: viewParam === 'activity-tracker' || urlHash === '#/activity-tracker',
  });

  const [currentView, setCurrentView] = useState<ViewType>('timer');
  const [showQuickEntry, setShowQuickEntry] = useState(false);
  const [showEmptyState] = useState(false);
  const [syncStatus, setSyncStatus] = useState<SyncStatus>('syncing');
  const [quickEntryLoading, setQuickEntryLoading] = useState(false);
  const [showValidationErrors, setShowValidationErrors] = useState(false);
  const [showNetworkError, setShowNetworkError] = useState(false);
  const [showSyncError, setShowSyncError] = useState(false);

  // Notification trigger ref
  type NotificationFn = (
    type: 'success' | 'error' | 'info' | 'warning',

    message: string,

    action?: { label: string; onClick: () => void }
  ) => void;
  const notificationTriggerRef = useRef<NotificationFn | null>(null);

  // Timer state - persisted across view navigation
  const [timerState, setTimerState] = useState<'inactive' | 'active' | 'paused' | 'idle'>(
    'inactive'
  );
  const [timerElapsed, setTimerElapsed] = useState(0);

  // Store window sizes for each view (persisted across navigation)
  const [entriesViewMode, setEntriesViewMode] = useState<'day' | 'week'>('day');
  const [timelineViewMode, setTimelineViewMode] = useState<'day' | 'week'>('day');

  const viewSizesRef = useRef<Record<ViewType, { width: number; height: number }>>({
    timer: { width: 420, height: 300 },
    entries: { width: 680, height: 620 },
    settings: { width: 580, height: 450 },
    analytics: { width: 580, height: 1025 },
    timeline: { width: 680, height: 720 },
    build: { width: 680, height: 620 },
  });

  // Check if we're in the AI Entry window - try both hash and query parameter
  const params = useMemo(() => new URLSearchParams(window.location.search), []);
  const isActivityTrackerWindow = useMemo(() => {
    const hash = window.location.hash === '#/activity-tracker';
    const query = params.get('view') === 'activity-tracker';
    const result = hash || query;

    // Always log on mount and when detection changes
    console.warn('🔍 Activity Tracker Detection:', {
      hash: window.location.hash,
      search: window.location.search,
      viewParam: params.get('view'),
      hashMatch: hash,
      queryMatch: query,
      finalResult: result,
      fullUrl: window.location.href,
    });

    return result;
  }, [params]);

  // Listen for manual window resizes and store the size for the current view
  useEffect(() => {
    if (!TauriAPI.isTauri()) return;

    let unlisten: (() => void) | undefined;

    const setupResizeListener = async () => {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      const currentWindow = getCurrentWindow();

      // Listen to resize events
      unlisten = await currentWindow.onResized((event) => {
        const { width, height } = event.payload;

        // Store the new size for the current view
        viewSizesRef.current[currentView] = {
          width: Math.round(width),
          height: Math.round(height),
        };
      });
    };

    void setupResizeListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [currentView]);

  // Dynamically resize window based on current view AND timer state (and entries view mode)
  useEffect(() => {
    const resizeWindowForView = async () => {
      if (!TauriAPI.isTauri()) {
        return;
      }

      // Get config - handle entries and timeline view modes
      let config;
      if (currentView === 'entries') {
        config = ViewSizeConfig.entries[entriesViewMode];
      } else if (currentView === 'timeline') {
        config = ViewSizeConfig.timeline[timelineViewMode];
      } else {
        config = ViewSizeConfig[currentView];
      }

      // For entries/timeline/build views, use the config size based on view mode (day/week)
      // For other views, use stored size or default
      const targetWidth =
        currentView === 'entries' || currentView === 'timeline' || currentView === 'build'
          ? config.width
          : viewSizesRef.current[currentView].width;
      const targetHeight =
        currentView === 'entries' || currentView === 'timeline' || currentView === 'build'
          ? config.height
          : viewSizesRef.current[currentView].height;

      try {
        console.log(`🎬 Animating window resize to ${targetWidth}x${targetHeight}`);

        // Use native macOS animated resize (smooth growing from center)
        await invoke('animate_window_resize', { width: targetWidth, height: targetHeight });

        console.log('✅ Animation command sent successfully');

        // Wait for animation to complete
        await new Promise((resolve) => setTimeout(resolve, 350));

        // Set window properties after animation
        await TauriAPI.window.setResizable(config.resizable);

        // DON'T center - this causes the window to snap
        // await TauriAPI.window.center();

        // Force window to recalculate bounds for edge detection
        if (config.resizable) {
          await TauriAPI.window.setMinSize(targetWidth - 100, targetHeight - 100);
          await TauriAPI.window.setMaxSize(targetWidth + 400, targetHeight + 400);
        } else if (currentView === 'timer') {
          // For timer view, allow dynamic sizing between 260-560px
          await TauriAPI.window.setMinSize(420, 260);
          await TauriAPI.window.setMaxSize(420, 560);
        } else {
          // For other non-resizable windows, use static sizing
          await TauriAPI.window.setMinSize(targetWidth, targetHeight);
          await TauriAPI.window.setMaxSize(targetWidth, targetHeight);
        }
      } catch (error) {
        console.error('❌ Failed to animate window resize:', error);
        // Fallback to instant resize
        await TauriAPI.window.setSize(targetWidth, targetHeight);
      }
    };

    void resizeWindowForView();
  }, [currentView, entriesViewMode, timelineViewMode]);

  // Force repaint after view change to prevent ghost shadows and debug dimensions
  useEffect(() => {
    // Trigger a repaint to clean up any lingering compositing layers
    const cleanup = setTimeout(() => {
      document.body.style.transform = 'translateZ(0)';
      requestAnimationFrame(() => {
        document.body.style.transform = '';
      });

      // Debug: Check for elements exceeding window bounds (development only)
      if (import.meta.env.DEV && TauriAPI.isTauri()) {
        // Get expected dimensions based on current view/mode
        let expectedWidth: number, expectedHeight: number;
        if (currentView === 'entries') {
          const entriesConfig = ViewSizeConfig.entries[entriesViewMode];
          expectedWidth = entriesConfig.width;
          expectedHeight = entriesConfig.height;
        } else if (currentView === 'timeline') {
          const timelineConfig = ViewSizeConfig.timeline[timelineViewMode];
          expectedWidth = timelineConfig.width;
          expectedHeight = timelineConfig.height;
        } else {
          const config = ViewSizeConfig[currentView];
          expectedWidth = config.width;
          expectedHeight = config.height;
        }

        const allElements = document.querySelectorAll('*');
        allElements.forEach((el) => {
          const rect = el.getBoundingClientRect();
          if (rect.width > expectedWidth || rect.height > expectedHeight) {
            console.warn('Element exceeds window bounds:', {
              element: el.tagName,
              class: el.className,
              width: rect.width,
              height: rect.height,
              expected: `${expectedWidth}x${expectedHeight}`,
            });
          }
        });
      }
    }, 300); // After animation completes

    return () => clearTimeout(cleanup);
  }, [currentView, entriesViewMode, timelineViewMode]);

  // Timer controls - memoized for stable references
  const handleToggleTimer = useCallback(() => {
    if (timerState === 'inactive' || timerState === 'paused') {
      setTimerState('active');
    } else {
      setTimerState('paused');
    }
  }, [timerState]);

  const handleStopTimer = useCallback(() => {
    setTimerState('inactive');
  }, []);

  // Global keyboard shortcuts - optimized with stable handler
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + H: Hide/minimize window
      if ((e.metaKey || e.ctrlKey) && e.key === 'h') {
        e.preventDefault();
        if (TauriAPI.isTauri()) {
          import('@tauri-apps/api/window')
            .then(({ getCurrentWindow }) => {
              void getCurrentWindow().hide();
            })
            .catch((err) => {
              console.error('❌ Failed to hide window:', err);
            });
        }
        return;
      }

      // Cmd/Ctrl + I: Open AI Entry Window
      if ((e.metaKey || e.ctrlKey) && e.key === 'i') {
        e.preventDefault();
        if (TauriAPI.isTauri()) {
          void import('@tauri-apps/api/core').then(({ invoke }) => {
            void invoke('open_ai_entry').catch((err) => {
              console.error('❌ Failed to invoke open_ai_entry:', err);
            });
          });
        }
        return;
      }

      // Escape key handler
      if (e.key === 'Escape') {
        if (showQuickEntry) {
          setShowQuickEntry(false);
        } else if (currentView !== 'timer') {
          setCurrentView('timer');
        } else {
          // If we're at the base timer view, hide the window
          if (TauriAPI.isTauri()) {
            import('@tauri-apps/api/window')
              .then(({ getCurrentWindow }) => {
                void getCurrentWindow().hide();
              })
              .catch((err) => {
                console.error('❌ Failed to hide window:', err);
              });
          }
        }
        return;
      }

      // Space: Toggle timer (only in timer view)
      if (e.code === 'Space' && currentView === 'timer') {
        // Don't trigger if typing in an input
        if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
          return;
        }
        e.preventDefault();
        handleToggleTimer();
        return;
      }
    };

    window.addEventListener('keydown', handleKeyDown);

    return () => {
      window.removeEventListener('keydown', handleKeyDown);
    };
  }, [showQuickEntry, currentView, handleToggleTimer]);

  // Listen to Tauri system tray events
  useEffect(() => {
    if (!TauriAPI.isTauri()) return;

    let unlisteners: Array<() => void> = [];

    const setupListeners = async () => {
      // Start timer from system tray
      const unlisten1 = await TauriAPI.tray.listenToEvent(TauriEvents.START_TIMER, () => {
        setTimerState('active');
      });

      // Pause timer from system tray
      const unlisten2 = await TauriAPI.tray.listenToEvent(TauriEvents.PAUSE_TIMER, () => {
        setTimerState('paused');
      });

      // Stop timer from system tray
      const unlisten3 = await TauriAPI.tray.listenToEvent(TauriEvents.STOP_TIMER, () => {
        handleStopTimer();
      });

      // Show window from system tray
      const unlisten4 = await TauriAPI.tray.listenToEvent(TauriEvents.SHOW_WINDOW, () => {
        void TauriAPI.window.show();
      });

      // Open AI Entry from system tray
      const unlisten5 = await TauriAPI.tray.listenToEvent(TauriEvents.OPEN_AI_ENTRY, () => {
        void (async () => {
          try {
            const { invoke } = await import('@tauri-apps/api/core');
            await invoke('open_ai_entry');
          } catch (err) {
            console.error('❌ Failed to open AI Entry:', err);
          }
        })();
      });

      unlisteners = [unlisten1, unlisten2, unlisten3, unlisten4, unlisten5];
    };

    void setupListeners();

    return () => {
      unlisteners.forEach((unlisten) => unlisten());
    };
  }, [handleStopTimer]);

  // FEATURE-009: Preload project cache on app mount
  useEffect(() => {
    if (!TauriAPI.isTauri()) return;

    void projectCache.preload();
  }, []);

  // REFACTOR-007 Phase 5: Send DbManager + Activity metrics to Datadog every 60 seconds
  useEffect(() => {
    if (!TauriAPI.isTauri()) return;

    const sendMetrics = async () => {
      try {
        const dbResult = await invoke<string>('send_db_metrics_to_datadog');
        console.log('📊 DB metrics sent:', dbResult);
      } catch (error) {
        console.warn('⚠️ Failed to send DB metrics:', error);
      }

      try {
        const activityResult = await invoke<string>('send_activity_metrics_to_datadog');
        console.log('📊 Activity metrics sent:', activityResult);
      } catch (error) {
        console.warn('⚠️ Failed to send activity metrics:', error);
      }
    };

    // Send initial metrics after 5 seconds (give app time to initialize)
    const initialTimeout = setTimeout(() => {
      void sendMetrics();
    }, 5000);

    // Then send metrics every 60 seconds
    const interval = setInterval(() => {
      void sendMetrics();
    }, 60000);

    return () => {
      clearTimeout(initialTimeout);
      clearInterval(interval);
    };
  }, []);

  // PERF-002: Listen for cached data and initialization status
  useEffect(() => {
    if (!TauriAPI.isTauri()) return;

    let statusUnlisten: (() => void) | undefined;
    let cacheUnlisten: (() => void) | undefined;

    const setupListener = async () => {
      const { listen } = await import('@tauri-apps/api/event');

      // TIER 0: Listen for cached startup data (instant display)
      cacheUnlisten = await listen('cached-data-loaded', (event) => {
        console.warn('⚡ PERF-002: Cached data loaded (instant)', event.payload);
        // Cached data available immediately - child components can use this
        // Status stays 'syncing' since we'll refresh with live data
      });

      // Backend initialization status
      statusUnlisten = await listen<string>('initialization-status', (event) => {
        const status = event.payload;
        console.warn('📡 Initialization status:', status);

        if (status === 'ready') {
          setSyncStatus('synced');
          // TIER 1 & 2: Fresh data will be loaded by child components
          // Timeline/timer components will call get_recent_snapshots_all, etc.
        } else if (status === 'error') {
          setSyncStatus('error');
        } else if (status === 'initializing') {
          setSyncStatus('syncing');
        }
      });
    };

    void setupListener();

    return () => {
      if (statusUnlisten) {
        statusUnlisten();
      }
      if (cacheUnlisten) {
        cacheUnlisten();
      }
    };
  }, []);

  // If this is the AI Entry window, render only that
  if (isActivityTrackerWindow) {
    return (
      <div className="fixed inset-0 w-full h-full bg-transparent overflow-hidden">
        <ActivityTrackerView />
      </div>
    );
  }

  // Render normal mode
  return (
    <div className="fixed inset-0 w-screen h-screen bg-transparent overflow-hidden max-w-screen max-h-screen">
      <Suspense fallback={<ViewLoader />}>
        {(showNetworkError || showSyncError) && (
          <CompactErrorAlert
            type={showNetworkError ? 'network' : 'sync'}
            onRetry={() => {
              setShowNetworkError(false);
              setShowSyncError(false);
              setSyncStatus('synced');
            }}
          />
        )}
      </Suspense>

      {/* View Switching with smooth transitions - only render active view */}
      <div className="view-container fixed inset-0 w-full h-full max-w-full max-h-full">
        <Suspense fallback={<ViewLoader />}>
          {/* Use a single conditional to ensure only one view exists in DOM */}
          {currentView === 'timer' ? (
            <div className="view-transition absolute inset-0 overflow-hidden" key="timer">
              <MainTimer
                onEntriesClick={() => setCurrentView('entries')}
                onSettingsClick={() => setCurrentView('settings')}
                onAnalyticsClick={() => setCurrentView('analytics')}
                onQuickEntry={() => setShowQuickEntry(true)}
                onTimelineClick={() => setCurrentView('timeline')}
                onBuildMyDayClick={() => setCurrentView('build')}
                syncStatus={syncStatus}
                onNotificationTriggerReady={(trigger: NotificationFn) => {
                  notificationTriggerRef.current = trigger;
                }}
                onTimerStateChange={(
                  status: 'inactive' | 'active' | 'paused' | 'idle',
                  elapsed: number
                ) => {
                  setTimerState(status);
                  setTimerElapsed(elapsed);
                }}
                initialStatus={timerState}
                initialElapsed={timerElapsed}
              />
            </div>
          ) : currentView === 'entries' ? (
            <div className="view-transition absolute inset-0" key="entries">
              <EntriesView
                onBack={() => setCurrentView('timer')}
                onQuickEntry={() => setShowQuickEntry(true)}
                showEmpty={showEmptyState}
                onViewModeChange={(viewMode) => setEntriesViewMode(viewMode)}
                onNotificationTriggerReady={(trigger) => {
                  notificationTriggerRef.current = trigger;
                }}
              />
            </div>
          ) : currentView === 'settings' ? (
            <div className="view-transition absolute inset-0" key="settings">
              <SettingsView
                onBack={() => setCurrentView('timer')}
                onRestartTutorial={() => {
                  setCurrentView('timer');
                }}
              />
            </div>
          ) : currentView === 'analytics' ? (
            <div className="view-transition absolute inset-0" key="analytics">
              <AnalyticsView onBack={() => setCurrentView('timer')} />
            </div>
          ) : currentView === 'timeline' ? (
            <div className="view-transition absolute inset-0" key="timeline">
              <TimelineDayView
                onBack={() => setCurrentView('timer')}
                onViewModeChange={(viewMode) => setTimelineViewMode(viewMode)}
              />
            </div>
          ) : currentView === 'build' ? (
            <div className="view-transition absolute inset-0" key="build">
              <BuildMyDayView onBack={() => setCurrentView('timer')} />
            </div>
          ) : null}
        </Suspense>
      </div>

      {/* Quick Entry Modal */}
      <CompactQuickEntry
        isOpen={showQuickEntry}
        onClose={() => {
          setShowQuickEntry(false);
          setQuickEntryLoading(false);
          setShowValidationErrors(false);
        }}
        isLoading={quickEntryLoading}
        showValidationErrors={showValidationErrors}
      />
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider>
      <TooltipProvider>
        <AppContent />
      </TooltipProvider>
    </ThemeProvider>
  );
}
