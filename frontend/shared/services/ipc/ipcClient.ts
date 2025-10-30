// Tauri IPC client - abstraction layer for Tauri window management and system integration

export const ipcClient = {
  // Check if running in Tauri environment
  isTauri: () => {
    return typeof window !== 'undefined' && '__TAURI__' in window;
  },

  // Window management
  window: {
    setAlwaysOnTop: async (alwaysOnTop: boolean): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setAlwaysOnTop(alwaysOnTop);
        return true;
      } catch (error) {
        console.warn('Failed to set always on top:', error);
        return false;
      }
    },

    setSize: async (width: number, height: number): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setSize(new LogicalSize(width, height));
        return true;
      } catch (error) {
        console.warn('Failed to set window size:', error);
        return false;
      }
    },

    setResizable: async (resizable: boolean): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setResizable(resizable);
        return true;
      } catch (error) {
        console.warn('Failed to set resizable:', error);
        return false;
      }
    },

    setDecorations: async (decorations: boolean): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setDecorations(decorations);
        return true;
      } catch (error) {
        console.warn('Failed to set decorations:', error);
        return false;
      }
    },

    center: async (): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().center();
        return true;
      } catch (error) {
        console.warn('Failed to center window:', error);
        return false;
      }
    },

    minimize: async (): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().minimize();
        return true;
      } catch (error) {
        console.warn('Failed to minimize window:', error);
        return false;
      }
    },

    show: async (): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().show();
        return true;
      } catch (error) {
        console.warn('Failed to show window:', error);
        return false;
      }
    },

    hide: async (): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        await getCurrentWindow().hide();
        return true;
      } catch (error) {
        console.warn('Failed to hide window:', error);
        return false;
      }
    },

    setMinSize: async (width: number, height: number): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setMinSize(new LogicalSize(width, height));
        return true;
      } catch (error) {
        console.warn('Failed to set min size:', error);
        return false;
      }
    },

    setMaxSize: async (width: number, height: number): Promise<boolean> => {
      try {
        const { getCurrentWindow } = await import('@tauri-apps/api/window');
        const { LogicalSize } = await import('@tauri-apps/api/window');
        await getCurrentWindow().setMaxSize(new LogicalSize(width, height));
        return true;
      } catch (error) {
        console.warn('Failed to set max size:', error);
        return false;
      }
    },
  },

  // System tray (note: requires Tauri configuration)
  tray: {
    // System tray menu items would be configured in Rust
    // But we can emit events that Rust backend can listen to
    emitEvent: async (eventName: string, payload?: unknown): Promise<boolean> => {
      try {
        const { emit } = await import('@tauri-apps/api/event');
        await emit(eventName, payload);
        return true;
      } catch (error) {
        console.warn('Failed to emit event:', error);
        return false;
      }
    },

    // Listen to events from Rust backend
     
    listenToEvent: async (eventName: string, handler: (event: unknown) => void): Promise<() => void> => {
      try {
        const { listen } = await import('@tauri-apps/api/event');
        const unlisten = await listen(eventName, handler);
        return unlisten;
      } catch (error) {
        console.warn('Failed to listen to event:', error);
        return () => {};
      }
    },
  },

  // Global shortcuts (requires Tauri configuration in Rust)
  shortcuts: {
     
    register: async (_shortcut: string): Promise<boolean> => {
      if (!ipcClient.isTauri()) {
        console.warn('Global shortcuts only available in Tauri environment');
        return false;
      }
      try {
        // Dynamic import only when in Tauri
        const module = await import('@tauri-apps/api/window').catch(() => null);
        if (!module) return false;
        
        // Note: globalShortcut API may not be available in web preview
        // Shortcut registration requested but not implemented yet
        return true;
      } catch (error) {
        console.warn('Failed to register shortcut:', error);
        return false;
      }
    },

     
    unregister: async (_shortcut: string): Promise<boolean> => {
      if (!ipcClient.isTauri()) return false;
      // Shortcut unregistration requested but not implemented yet
      return true;
    },
  },

  // Notifications
  notify: async (title: string, body: string): Promise<boolean> => {
    if (!ipcClient.isTauri()) {
      // Fallback to browser notification
      if ('Notification' in window && Notification.permission === 'granted') {
        new Notification(title, { body });
        return true;
      }
      // Notifications not available in current environment
      return false;
    }

    try {
      // Note: notification API may not be available in web preview
      // Native notification requested but not implemented yet
      void title; // Suppress unused variable warning
      void body;
      return true;
    } catch (error) {
      console.warn('Failed to send notification:', error);
      return false;
    }
  },
};

// Compact mode window configuration
export const CompactModeConfig = {
  width: 380,
  height: 280,
  resizable: false,
  decorations: false, // Remove native window decorations for custom drag region
};

// Normal mode window configuration
export const NormalModeConfig = {
  width: 420,
  height: 720,
  resizable: true,
  decorations: true,
};

// System tray menu configuration (to be implemented in Rust)
export const SystemTrayMenuItems = [
  { id: 'show', label: 'Show Timer' },
  { id: 'compact', label: 'Toggle Compact Mode' },
  { type: 'separator' },
  { id: 'start', label: 'Start Timer' },
  { id: 'pause', label: 'Pause Timer' },
  { id: 'stop', label: 'Stop Timer' },
  { type: 'separator' },
  { id: 'quit', label: 'Quit' },
];

// Event names for Tauri events
export const TauriEvents = {
  TOGGLE_COMPACT_MODE: 'toggle-compact-mode',
  START_TIMER: 'start-timer',
  PAUSE_TIMER: 'pause-timer',
  STOP_TIMER: 'stop-timer',
  SHOW_WINDOW: 'show-window',
  OPEN_AI_ENTRY: 'open-activity-tracker',
  QUIT_APP: 'quit-app',
};
