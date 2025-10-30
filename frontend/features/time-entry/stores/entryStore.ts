// Time Entry feature Zustand store
import { create } from 'zustand';
import type { TimeEntry } from '../types';
import type { TimeEntryOutbox, PrismaTimeEntryDto } from '@/shared/types/generated';
import { formatTime } from '@/shared/utils/timeFormat';

// Helper to format duration in seconds to "Xh Ym" format
const formatDuration = (seconds: number): string => {
  const hours = Math.floor(seconds / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
};

 
interface EntryState {
  // State
  entries: TimeEntry[];
  loading: boolean;
  error: string | null;
  syncing: boolean;

  // Actions
  setEntries: (entries: TimeEntry[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  setSyncing: (syncing: boolean) => void;

  // Data fetching from backend
  fetchEntries: (timeFilter?: string) => Promise<void>;

  // Local CRUD operations (no backend - local state only)
  updateEntry: (id: string, updates: Partial<TimeEntry>) => Promise<TimeEntry | null>;
  deleteEntry: (id: string) => Promise<boolean>;
  optimisticAddEntry: (entry: TimeEntry) => void;

  // Utility actions
  getEntryById: (id: string) => TimeEntry | undefined;
  filterEntriesByStatus: (status: 'pending' | 'approved' | 'suggested') => TimeEntry[];
}
 

/**
 * Time Entry store - manages time entry state
 *
 * Note: CRUD operations are handled directly via Tauri commands where needed.
 * This store provides a centralized state for time entries displayed in the UI.
 */
export const useEntryStore = create<EntryState>((set, get) => ({
  // Initial state
  entries: [],
  loading: false,
  error: null,
  syncing: false,

  // Setters
  setEntries: (entries) => set({ entries }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),
  setSyncing: (syncing) => set({ syncing }),

  // Fetch entries from backend (approved/sent time entries)
  fetchEntries: async (timeFilter?: string) => {
    set({ loading: true, error: null });
    try {
      const { invoke } = await import('@tauri-apps/api/core');
      const { projectCache } = await import('@/shared/services');

      // Fetch project cache for name lookups
      try {
        await projectCache.fetchProjects();
      } catch (cacheError) {
        console.error('Project cache fetch failed, will use fallback:', cacheError);
      }

      // Get sent (approved) entries from backend
      const outboxEntries = await invoke<TimeEntryOutbox[]>('get_time_entries', {
        timeFilter,
      });

      // Map outbox entries to TimeEntry format
      const entries: TimeEntry[] = outboxEntries.map((entry) => {
        // Parse payload_json to get the DTO data
        let dto: PrismaTimeEntryDto | null = null;
        try {
          dto =
            typeof entry.payload_json === 'string'
              ? (JSON.parse(entry.payload_json) as PrismaTimeEntryDto)
              : (entry.payload_json as PrismaTimeEntryDto);
        } catch (error) {
          console.error('Failed to parse payload_json:', error);
        }

        // Backend timestamps are in SECONDS, multiply by 1000 for JS Date
        const createdAtMs = entry.created_at * 1000;

        // Determine if this is a calendar event or AI-classified activity
        const isCalendarEvent = dto?.source === 'calendar';

        // Calendar events: Use parsed project/task (backend handles "General" bucket)
        // AI entries: Use project cache lookup
        const projectDisplay = isCalendarEvent
          ? dto?._displayProject ?? 'General'
          : projectCache.getProjectName(dto?.projectId ?? 'unassigned');

        const taskDisplay = isCalendarEvent ? dto?._displayTask ?? '' : dto?.notes ?? 'Activity detected';

        // Calendar events include parsed confidence, AI defaults to 85%
        const confidence = dto?._confidence ? Math.round(dto._confidence * 100) : 85;

        const entryDate = new Date(createdAtMs);

        return {
          id: entry.id,
          time: formatTime(entryDate),
          shortDate: entryDate.toLocaleDateString([], {
            month: '2-digit',
            day: '2-digit',
            year: 'numeric',
          }),
          project: projectDisplay,
          task: taskDisplay,
          duration: formatDuration(dto?.durationSec ?? 0),
          status: 'approved', // All sent entries are approved
          confidence,
          durationSeconds: dto?.durationSec ?? 0,
          startTime: dto?.startTime ? new Date(dto.startTime) : undefined,
          endTime: dto?.endTime ? new Date(dto.endTime) : undefined,
          wbsCode: dto?._wbsCode ?? undefined,
        };
      });

      set({ entries, loading: false });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to fetch entries';
      set({ error: errorMessage, loading: false });
    }
  },

  // Local-only CRUD operations (no backend persistence)
  updateEntry: async (id, updates) => {
    set({ syncing: true });
    set((state) => ({
      entries: state.entries.map((entry) => (entry.id === id ? { ...entry, ...updates } : entry)),
      syncing: false,
    }));
    return get().entries.find((e) => e.id === id) ?? null;
  },

  deleteEntry: async (id) => {
    set({ syncing: true });
    set((state) => ({
      entries: state.entries.filter((entry) => entry.id !== id),
      syncing: false,
    }));
    return true;
  },

  optimisticAddEntry: (entry) => {
    set((state) => ({
      entries: [entry, ...state.entries],
    }));
  },

  // Utility actions
  getEntryById: (id) => {
    const state = get();
    return state.entries.find((e) => e.id === id);
  },

  filterEntriesByStatus: (status) => {
    const state = get();
    return state.entries.filter((e) => e.status === status);
  },
}));
