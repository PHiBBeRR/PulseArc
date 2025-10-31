import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { Tooltip, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/components/ui/utils';
import { projectCache } from '@/shared/services';
import type {
  AcceptPatch,
  PrismaTimeEntryDto,
  ProposedBlock,
  TimeEntryOutbox,
} from '@/shared/types/generated';
import { haptic } from '@/shared/utils';
import { formatTime } from '@/shared/utils/timeFormat';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { useVirtualizer } from '@tanstack/react-virtual';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AnimatePresence, motion } from 'framer-motion';
import {
  Activity,
  Brain,
  Calendar,
  Check,
  ChevronUp,
  Filter,
  Loader2,
  Pencil,
  RefreshCw,
  Trash2,
  Undo2,
  User,
  Users,
  X,
} from 'lucide-react';
import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { ClassifyEntryModal } from '../../time-entry/components/ClassifyEntryModal';
import { DismissFeedbackModal } from '../../time-entry/components/DismissFeedbackModal';
import { EditEntryModal } from '../../time-entry/components/EditEntryModal';
import { entryService } from '../../time-entry/services';
import type { TimeEntry } from '../../time-entry/types';
import { ActivityBreakdownTooltip } from './ActivityBreakdownTooltip';
import { FilterSortPopover, type FilterSortState } from './FilterSortPopover';

// Custom TooltipContent without arrow
function TooltipContentNoArrow({
  className,
  sideOffset = 0,
  children,
  ...props
}: React.ComponentProps<typeof TooltipPrimitive.Content>) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        sideOffset={sideOffset}
        className={cn(
          'animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95 data-[side=bottom]:slide-in-from-top-2 data-[side=left]:slide-in-from-right-2 data-[side=right]:slide-in-from-left-2 data-[side=top]:slide-in-from-bottom-2 z-50 w-fit origin-(--radix-tooltip-content-transform-origin) rounded-md px-3 py-1.5 text-xs',
          className
        )}
        {...props}
      >
        {children}
      </TooltipPrimitive.Content>
    </TooltipPrimitive.Portal>
  );
}

interface SuggestedEntriesProps {
  onAcceptEntry?: (entry: TimeEntry, remainingCount: number) => void;
  onDismissEntry?: (entryId: string, remainingCount: number) => void;
  onCountChange?: (count: number) => void;
  onCollapseChange?: (isCollapsed: boolean) => void;
  isBuilding?: boolean;
  onBuildMyDay?: () => void;
}

type TabType = 'suggestions' | 'dismissed';

export function SuggestedEntries({
  onAcceptEntry,
  onDismissEntry,
  onCountChange,
  onCollapseChange,
  isBuilding = false,
  onBuildMyDay,
}: SuggestedEntriesProps) {
  const [suggestedEntries, setSuggestedEntries] = useState<TimeEntry[]>([]);
  const [dismissedEntries, setDismissedEntries] = useState<TimeEntry[]>([]);
  const [activeTab, setActiveTab] = useState<TabType>('suggestions');
  const [unclassifiedBlocksCount, setUnclassifiedBlocksCount] = useState<number>(0);

  // Load collapsed state from localStorage on mount
  const [isCollapsed, setIsCollapsed] = useState(() => {
    const stored = localStorage.getItem('recentActivityCollapsed');
    return stored === 'true';
  });

  const [isLoading, setIsLoading] = useState(true);

  // Track optimistic updates to prevent loading spinner flash
  const isOptimisticUpdateRef = useRef(false);

  // FEATURE-019 Phase 3: Modal state
  const [editModalOpen, setEditModalOpen] = useState(false);
  const [entryToEdit, setEntryToEdit] = useState<TimeEntry | null>(null);
  const [classifyModalOpen, setClassifyModalOpen] = useState(false);
  const [entryToClassify, setEntryToClassify] = useState<TimeEntry | null>(null);
  const [dismissModalOpen, setDismissModalOpen] = useState(false);
  const [entryToDismiss, setEntryToDismiss] = useState<TimeEntry | null>(null);
  const [authErrorModalOpen, setAuthErrorModalOpen] = useState(false);

  // Filter/Sort state
  const [filterSortState, setFilterSortState] = useState<FilterSortState>(() => {
    const stored = localStorage.getItem('suggestionsFilterSort');
    if (stored) {
      try {
        const parsed = JSON.parse(stored);
        return {
          sortBy: parsed.sortBy || 'most-recent',
          sourceFilters: new Set(parsed.sourceFilters || []),
          categoryFilters: new Set(parsed.categoryFilters || []),
        };
      } catch {
        return {
          sortBy: 'most-recent',
          sourceFilters: new Set(),
          categoryFilters: new Set(),
        };
      }
    }
    return {
      sortBy: 'most-recent',
      sourceFilters: new Set(),
      categoryFilters: new Set(),
    };
  });

  // Persist filter/sort state to localStorage
  useEffect(() => {
    localStorage.setItem(
      'suggestionsFilterSort',
      JSON.stringify({
        sortBy: filterSortState.sortBy,
        sourceFilters: Array.from(filterSortState.sourceFilters),
        categoryFilters: Array.from(filterSortState.categoryFilters),
      })
    );
  }, [filterSortState]);

  // Categorize calendar events based on keywords
  const categorizeCalendarEvent = useCallback(
    (projectName: string, taskName: string): 'personal' | 'general' | 'project' => {
      // If backend defaulted to "General", only check the task name
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
    },
    []
  );

  // FEATURE-021: Map ProposedBlock to TimeEntry format
  const mapBlockToTimeEntry = useCallback(
    (block: ProposedBlock): TimeEntry => {
      // Backend timestamps are in SECONDS, multiply by 1000 for JS Date
      const startMs = block.start_ts * 1000;
      const endMs = block.end_ts * 1000;

      // Categorize AI blocks based on project/workstream (similar to calendar events)
      const projectName = block.inferred_deal_name ?? 'Unknown Project';
      const taskName = block.inferred_workstream ?? 'General work';
      const category = categorizeCalendarEvent(projectName, taskName);

      return {
        id: block.id,
        time: formatTime(new Date(startMs)),
        project: projectName,
        task: taskName,
        duration: formatDuration(block.duration_secs),
        status: 'suggested' as const,
        confidence: Math.round(block.confidence * 100),
        durationSeconds: block.duration_secs,
        source: 'ai' as const,
        shortDate: new Date(startMs).toLocaleDateString('en-US', {
          month: '2-digit',
          day: '2-digit',
          year: 'numeric',
        }),
        category, // Use categorized value (general/personal/project)
        wbsCode: block.inferred_wbs_code ?? undefined,
        startTime: new Date(startMs),
        endTime: new Date(endMs), // Time range end
        activities: block.activities, // FEATURE-021: Real activity breakdown from backend
        idleSeconds: block.total_idle_secs, // FEATURE-028: Idle time within block
      };
    },
    [categorizeCalendarEvent]
  );

  // Helper to map outbox entry to TimeEntry format (legacy)
  const mapOutboxToTimeEntry = useCallback(
    (entry: TimeEntryOutbox & { dto: PrismaTimeEntryDto | null }) => {
      // Backend timestamps: check if in seconds or milliseconds
      // If < 10 billion, it's in seconds (before year 2286), otherwise already in milliseconds
      const createdAtMs =
        entry.created_at < 10_000_000_000 ? entry.created_at * 1000 : entry.created_at;
      const entryDate = new Date(createdAtMs);
      const dto = entry.dto; // May be null if JSON parsing failed

      // FEATURE-015: Both calendar and AI entries use projectId for consistent project lookup
      const isCalendarEvent = dto?.source === 'calendar';

      // Calendar events: Use _displayProject (parsed from event title, no DB lookup needed)
      // AI entries: Use project cache lookup with projectId (UUIDv7 maps to WBS code)
      let projectDisplay = isCalendarEvent
        ? (dto?._displayProject ?? 'General')
        : projectCache.getProjectName(dto?.projectId ?? 'unassigned');

      // Clean up project name: remove "PROJECT_" prefix and replace underscores with spaces
      projectDisplay = projectDisplay
        .replace(/^PROJECT_/i, '') // Remove PROJECT_ prefix (case-insensitive)
        .replace(/_/g, ' '); // Replace all underscores with spaces

      // Task display: Both calendar and AI entries use _displayTask field
      const taskDisplay = dto?._displayTask ?? 'Activity detected';

      // Categorize calendar events (personal/general/project) using display fields for pattern matching
      // Display fields are hints for categorization, but projectId is the source of truth for display
      const category: 'personal' | 'general' | 'project' | 'ai' = isCalendarEvent
        ? categorizeCalendarEvent(dto?._displayProject ?? projectDisplay, taskDisplay)
        : 'ai';

      // FEATURE-019: Remove hardcoded fallback - trust backend to provide confidence
      const confidence = dto?._confidence ? Math.round(dto._confidence * 100) : 0;

      // Description: Don't show for either calendar or AI entries (redundant with task field)
      const description = undefined;

      // Format short date (MM/DD/YYYY)
      const shortDate = entryDate.toLocaleDateString('en-US', {
        month: '2-digit',
        day: '2-digit',
        year: 'numeric',
      });

      // Override project display for personal events
      const finalProjectDisplay = category === 'personal' ? 'Personal' : projectDisplay;

      return {
        id: entry.id,
        time: formatTime(new Date(createdAtMs)),
        project: finalProjectDisplay,
        task: taskDisplay,
        duration: formatDuration(dto?.durationSec ?? 0),
        status: 'suggested' as const,
        confidence,
        description,
        durationSeconds: dto?.durationSec ?? 0,
        source: isCalendarEvent ? ('calendar' as const) : ('ai' as const),
        shortDate,
        category,
        wbsCode: dto?._wbsCode ?? undefined,
        startTime: new Date(createdAtMs), // Add for sorting
        // Note: Legacy outbox entries don't have activities - only ProposedBlocks do
      };
    },
    [categorizeCalendarEvent]
  );

  // FEATURE-021: Fetch pending suggestions from ProposedBlocks
  const fetchSuggestions = useCallback(async () => {
    try {
      // Try to fetch projects (continue even if this fails)
      try {
        await projectCache.fetchProjects();
      } catch (cacheError) {
        console.error('Project cache fetch failed, will use fallback:', cacheError);
      }

      // Get today's epoch for filtering blocks
      const todayEpoch = Math.floor(Date.now() / 1000);

      // FEATURE-021: Fetch ProposedBlocks (consolidated 30+ min blocks with activities)
      const blocks = await invoke<ProposedBlock[]>('get_proposed_blocks', {
        dayEpoch: todayEpoch,
        status: 'suggested', // Only fetch blocks pending user review
      });

      console.log(`📦 Fetched ${blocks.length} ProposedBlocks for today`);

      // Map blocks to TimeEntry format
      const suggestions = blocks
        .sort((a, b) => b.start_ts - a.start_ts) // Most recent first
        .map(mapBlockToTimeEntry);

      setSuggestedEntries(suggestions);
    } catch (error) {
      console.error('Failed to fetch ProposedBlocks:', error);
      // Fallback: try legacy outbox entries if ProposedBlocks fail
      console.log('Falling back to legacy TimeEntryOutbox...');
      try {
        const outbox = await invoke<TimeEntryOutbox[]>('get_outbox_status');
        const parsedOutbox = outbox.map((entry) => {
          let dto: PrismaTimeEntryDto | null = null;
          try {
            dto =
              typeof entry.payload_json === 'string'
                ? (JSON.parse(entry.payload_json) as PrismaTimeEntryDto)
                : (entry.payload_json as PrismaTimeEntryDto);
          } catch (error) {
            console.error('Failed to parse payload_json:', error);
          }
          return { ...entry, dto };
        });

        const suggestions = parsedOutbox
          .filter((entry) => entry.status === 'pending')
          .sort((a, b) => b.created_at - a.created_at)
          .map(mapOutboxToTimeEntry);

        setSuggestedEntries(suggestions);
      } catch (fallbackError) {
        console.error('Fallback to outbox also failed:', fallbackError);
      }
    }
  }, [mapBlockToTimeEntry, mapOutboxToTimeEntry]);

  // FEATURE-019: Fetch dismissed entries from backend
  const fetchDismissed = useCallback(async () => {
    try {
      // Get dismissed outbox entries
      const dismissed = await invoke<TimeEntryOutbox[]>('get_dismissed_suggestions');

      // Parse payload_json
      const parsedDismissed = dismissed.map((entry) => {
        let dto: PrismaTimeEntryDto | null = null;
        try {
          dto =
            typeof entry.payload_json === 'string'
              ? (JSON.parse(entry.payload_json) as PrismaTimeEntryDto)
              : (entry.payload_json as PrismaTimeEntryDto);
        } catch (error) {
          console.error('Failed to parse payload_json:', error);
        }
        return { ...entry, dto };
      });

      // Sort by created_at descending (most recent first)
      const dismissedEntries = parsedDismissed
        .sort((a, b) => b.created_at - a.created_at) // Most recent first
        .map(mapOutboxToTimeEntry);
      setDismissedEntries(dismissedEntries);
    } catch (error) {
      console.error('Failed to fetch dismissed entries:', error);
    }
  }, [mapOutboxToTimeEntry]);

  // Fetch count of pending blocks (blocks that need to be classified via Build My Day)
  const fetchUnclassifiedBlocksCount = useCallback(async () => {
    try {
      const todayEpoch = Math.floor(Date.now() / 1000);

      // Get pending blocks for today (blocks that haven't been classified yet)
      const pendingBlocks = await invoke<ProposedBlock[]>('get_proposed_blocks', {
        dayEpoch: todayEpoch,
        status: 'pending', // Only get blocks that need Build My Day
      });

      setUnclassifiedBlocksCount(pendingBlocks.length);
      console.log(`📊 Fetched ${pendingBlocks.length} pending blocks for today`);
    } catch (error) {
      console.error('Failed to fetch pending blocks count:', error);
      setUnclassifiedBlocksCount(0);
    }
  }, []);

  // FEATURE-019/021: Fetch all entry types (suggestions, pending blocks, dismissed)
  const fetchAllEntries = useCallback(async () => {
    // Skip loading indicator if this is triggered by an optimistic update
    const showLoading = !isOptimisticUpdateRef.current;

    if (showLoading) {
      setIsLoading(true);
    }

    try {
      await Promise.all([fetchSuggestions(), fetchDismissed(), fetchUnclassifiedBlocksCount()]);
    } finally {
      if (showLoading) {
        setIsLoading(false);
      }
      // Reset the flag after fetch completes
      isOptimisticUpdateRef.current = false;
    }
  }, [fetchSuggestions, fetchDismissed, fetchUnclassifiedBlocksCount]);

  // FEATURE-019: Event-driven updates - listen for outbox changes from backend
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    const setupListener = async () => {
      try {
        // Initial fetch
        await fetchAllEntries();

        // Listen for real-time outbox updates (event-driven, no polling)
        unlisten = await listen('outbox-updated', () => {
          void fetchAllEntries();
        });
      } catch (error) {
        console.error('Failed to setup suggestion listener:', error);
      }
    };

    void setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [fetchAllEntries]);

  // Helper to format duration
  const formatDuration = (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const mins = Math.floor((seconds % 3600) / 60);
    return hours > 0 ? `${hours}h ${mins}m` : `${mins}m`;
  };

  // Calculate active filter count
  const activeFilterCount =
    filterSortState.sourceFilters.size + filterSortState.categoryFilters.size;

  // Refs for virtual scrolling containers
  const suggestionsParentRef = useRef<HTMLDivElement>(null);
  const dismissedParentRef = useRef<HTMLDivElement>(null);

  // Apply filters and sorting to suggested entries
  const filteredAndSortedSuggestions = useMemo(() => {
    let filtered = [...suggestedEntries];

    // Apply source filters
    if (filterSortState.sourceFilters.size > 0) {
      filtered = filtered.filter(
        (entry) => entry.source && filterSortState.sourceFilters.has(entry.source)
      );
    }

    // Apply category filters
    if (filterSortState.categoryFilters.size > 0) {
      filtered = filtered.filter(
        (entry) => entry.category && filterSortState.categoryFilters.has(entry.category)
      );
    }

    // Apply sorting
    switch (filterSortState.sortBy) {
      case 'most-recent':
        filtered.sort((a, b) => {
          const aTime = a.startTime?.getTime() || 0;
          const bTime = b.startTime?.getTime() || 0;
          return bTime - aTime; // Descending (most recent first)
        });
        break;
      case 'oldest-first':
        filtered.sort((a, b) => {
          const aTime = a.startTime?.getTime() || 0;
          const bTime = b.startTime?.getTime() || 0;
          return aTime - bTime; // Ascending (oldest first)
        });
        break;
      case 'longest-duration':
        filtered.sort((a, b) => (b.durationSeconds || 0) - (a.durationSeconds || 0));
        break;
      case 'shortest-duration':
        filtered.sort((a, b) => (a.durationSeconds || 0) - (b.durationSeconds || 0));
        break;
    }

    return filtered;
  }, [suggestedEntries, filterSortState]);

  // Apply filters and sorting to dismissed entries
  const filteredAndSortedDismissed = useMemo(() => {
    let filtered = [...dismissedEntries];

    // Apply source filters
    if (filterSortState.sourceFilters.size > 0) {
      filtered = filtered.filter(
        (entry) => entry.source && filterSortState.sourceFilters.has(entry.source)
      );
    }

    // Apply category filters
    if (filterSortState.categoryFilters.size > 0) {
      filtered = filtered.filter(
        (entry) => entry.category && filterSortState.categoryFilters.has(entry.category)
      );
    }

    // Apply sorting
    switch (filterSortState.sortBy) {
      case 'most-recent':
        filtered.sort((a, b) => {
          const aTime = a.startTime?.getTime() || 0;
          const bTime = b.startTime?.getTime() || 0;
          return bTime - aTime; // Descending (most recent first)
        });
        break;
      case 'oldest-first':
        filtered.sort((a, b) => {
          const aTime = a.startTime?.getTime() || 0;
          const bTime = b.startTime?.getTime() || 0;
          return aTime - bTime; // Ascending (oldest first)
        });
        break;
      case 'longest-duration':
        filtered.sort((a, b) => (b.durationSeconds || 0) - (a.durationSeconds || 0));
        break;
      case 'shortest-duration':
        filtered.sort((a, b) => (a.durationSeconds || 0) - (b.durationSeconds || 0));
        break;
    }

    return filtered;
  }, [dismissedEntries, filterSortState]);

  // Virtual scrolling setup for suggestions tab
  // Dynamic card height based on content with measurement
  const suggestionsVirtualizer = useVirtualizer({
    count: filteredAndSortedSuggestions.length,
    getScrollElement: () => suggestionsParentRef.current,
    estimateSize: () => 95, // Estimated average height including 8px gap
    overscan: 2, // Render 2 extra items above/below viewport
    measureElement:
      typeof window !== 'undefined' && navigator.userAgent.indexOf('Firefox') === -1
        ? (element) => element.getBoundingClientRect().height
        : undefined,
  });

  // Virtual scrolling setup for dismissed tab
  const dismissedVirtualizer = useVirtualizer({
    count: filteredAndSortedDismissed.length,
    getScrollElement: () => dismissedParentRef.current,
    estimateSize: () => 95, // Estimated average height including 8px gap
    overscan: 2,
    measureElement:
      typeof window !== 'undefined' && navigator.userAgent.indexOf('Firefox') === -1
        ? (element) => element.getBoundingClientRect().height
        : undefined,
  });

  // FEATURE-019: Report count changes to parent for window resizing
  // Always show max 3 cards in viewport (virtual scroll handles the rest)
  // Report at least 1 to account for empty state when section is always visible
  useEffect(() => {
    const suggestionsCount = Math.min(suggestedEntries.length, 3);
    const dismissedCount = Math.min(dismissedEntries.length, 3);
    const visibleCount = Math.max(suggestionsCount, dismissedCount);

    // Always report at least 1 to show empty state (not 0)
    const finalCount = Math.max(visibleCount, 1);

    onCountChange?.(finalCount);
  }, [suggestedEntries.length, dismissedEntries.length, onCountChange, activeTab]);

  // FEATURE-019: Report collapse state changes to parent and persist to localStorage
  useEffect(() => {
    onCollapseChange?.(isCollapsed);
    localStorage.setItem('recentActivityCollapsed', String(isCollapsed));
  }, [isCollapsed, onCollapseChange]);

  // FEATURE-019: Accept handler (with classification check)
  const handleAccept = async (entry: TimeEntry) => {
    haptic.light();

    // Check authentication before accepting entry
    try {
      const isAuthenticated = await invoke<boolean>('webapi_is_authenticated');
      if (!isAuthenticated) {
        setAuthErrorModalOpen(true);
        return;
      }
    } catch (error) {
      console.error('Failed to check authentication:', error);
      setAuthErrorModalOpen(true);
      return;
    }

    // FEATURE-019 Phase 3: Check if personal (no WBS code) - require classification first
    // Personal entries need user to classify as billable project or G&A
    if (entry.category === 'personal') {
      setEntryToClassify(entry);
      setClassifyModalOpen(true);
      return;
    }

    try {
      // Set flag to prevent loading spinner when backend emits outbox-updated event
      isOptimisticUpdateRef.current = true;

      // Mark as accepted in backend (optimistic - sync worker will process it)
      await invoke('accept_suggestion', { id: entry.id });

      // Update local state
      const newEntries = suggestedEntries.filter((e) => e.id !== entry.id);
      setSuggestedEntries(newEntries);
      onAcceptEntry?.(entry, newEntries.length);
    } catch (error) {
      console.error('Failed to accept suggestion:', error);
      // Keep entry visible on error - user can retry
      // Reset flag on error
      isOptimisticUpdateRef.current = false;
    }
  };

  // FEATURE-019 Phase 3: Edit handler
  const handleEdit = (entry: TimeEntry) => {
    haptic.light();
    setEntryToEdit(entry);
    setEditModalOpen(true);
  };

  // FEATURE-019 Phase 3: Dismiss handler (instant dismiss to dismissed tab - optimistic UI)
  const handleDismiss = async (entryId: string) => {
    haptic.light();

    // Find entry before optimistic update
    const entry = suggestedEntries.find((e) => e.id === entryId);
    if (!entry) return;

    // Optimistic UI update - instant visual feedback
    setSuggestedEntries((prev) => prev.filter((e) => e.id !== entryId));
    setDismissedEntries((prev) => [...prev, entry]);
    onDismissEntry?.(entry.id, suggestedEntries.length - 1);

    // Set flag to prevent loading spinner when backend emits outbox-updated event
    isOptimisticUpdateRef.current = true;

    // Backend sync in background (non-blocking)
    try {
      // ProposedBlocks have activities array, legacy outbox entries don't
      // Use different dismiss commands based on entry type
      if (entry.activities && entry.activities.length > 0) {
        await invoke('dismiss_proposed_block', { blockId: entryId });
      } else {
        await invoke('dismiss_suggestion', { id: entryId, reason: '' });
      }
    } catch (error) {
      console.error('Failed to dismiss suggestion:', error);
      // On error, revert the optimistic update
      setSuggestedEntries((prev) => [entry, ...prev]);
      setDismissedEntries((prev) => prev.filter((e) => e.id !== entryId));
      // Reset flag on error
      isOptimisticUpdateRef.current = false;
    }
  };

  // FEATURE-019: Restore handler (from Dismissed tab)
  const handleRestore = async (entryId: string) => {
    haptic.light();

    try {
      await invoke('restore_suggestion', { id: entryId });

      // Move entry from dismissed back to suggestions (maintain sort order)
      const entry = dismissedEntries.find((e) => e.id === entryId);
      if (entry) {
        setDismissedEntries((prev) => prev.filter((e) => e.id !== entryId));
        setSuggestedEntries((prev) => {
          // Re-fetch will handle sorting, but for immediate feedback insert at top
          return [entry, ...prev];
        });
        setActiveTab('suggestions'); // Auto-switch to Suggestions tab
      }
    } catch (error) {
      console.error('Failed to restore suggestion:', error);
    }
  };

  // FEATURE-019: Delete handler (opens feedback modal before permanent deletion)
  const handleDelete = (entryId: string) => {
    haptic.light();

    const entry = dismissedEntries.find((e) => e.id === entryId);
    if (entry) {
      setEntryToDismiss(entry);
      setDismissModalOpen(true);
    }
  };

  // FEATURE-019 Phase 3: Modal save handlers
  const handleSaveEdit = async (entry: TimeEntry, patch: AcceptPatch) => {
    try {
      await invoke('update_suggestion', {
        id: entry.id,
        title: patch.title,
        project: patch.project,
        wbs_code: patch.wbs_code,
        duration_sec: patch.duration_sec,
        entry_date: patch.entry_date,
      });

      // Remove from suggestions list (now marked as sent)
      setSuggestedEntries((prev) => prev.filter((e) => e.id !== entry.id));
    } catch (error) {
      console.error('Failed to save edit:', error);
      throw error; // Re-throw to let modal handle error
    }
  };

  const handleClassify = async (entry: TimeEntry, patch: AcceptPatch) => {
    try {
      await invoke('update_suggestion', {
        id: entry.id,
        title: patch.title,
        project: patch.project,
        wbs_code: patch.wbs_code,
        duration_sec: patch.duration_sec,
        entry_date: patch.entry_date,
      });

      // Remove from suggestions list (now marked as sent)
      setSuggestedEntries((prev) => prev.filter((e) => e.id !== entry.id));
    } catch (error) {
      console.error('Failed to classify entry:', error);
      throw error; // Re-throw to let modal handle error
    }
  };

  const handleConfirmDismiss = async (entry: TimeEntry, _reason: string) => {
    try {
      // Set flag to prevent loading spinner when backend emits outbox-updated event
      isOptimisticUpdateRef.current = true;

      // Permanently delete from outbox (feedback modal now used for deletion only)
      await invoke('delete_outbox_entry', { id: entry.id });

      // Remove from dismissed list
      setDismissedEntries((prev) => prev.filter((e) => e.id !== entry.id));
    } catch (error) {
      console.error('Failed to delete entry:', error);
      // Reset flag on error
      isOptimisticUpdateRef.current = false;
      throw error; // Re-throw to let modal handle error
    }
  };

  // Always show the section, even when empty
  return (
    <div className="pt-1 pb-3">
      {/* FEATURE-019: Collapsible Header */}
      <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl mx-4 mb-3">
        <div className="w-full flex items-center justify-between px-4 py-2 gap-2">
          {/* Activity/Build My Day icon - shows Activity by default, RefreshCw on hover */}
          {onBuildMyDay ? (
            <Tooltip>
              <TooltipTrigger asChild>
                <button
                  onClick={onBuildMyDay}
                  disabled={isBuilding}
                  aria-label="Build my day"
                  title="Build my day"
                  className="group h-7 w-7 -ml-1 rounded-md flex items-center justify-center bg-white/30 dark:bg-white/20 hover:bg-white/10 transition-all disabled:opacity-50 disabled:cursor-not-allowed flex-shrink-0 relative"
                >
                  {/* Activity icon - default state */}
                  <Activity
                    className={`w-4 h-4 text-white absolute transition-opacity ${isBuilding ? 'opacity-0' : 'group-hover:opacity-0'}`}
                  />
                  {/* RefreshCw icon - hover/building state */}
                  <RefreshCw
                    className={`w-4 h-4 text-white absolute transition-opacity ${isBuilding ? 'opacity-100 animate-spin' : 'opacity-0 group-hover:opacity-100'}`}
                  />
                </button>
              </TooltipTrigger>
              <TooltipContentNoArrow
                side="top"
                sideOffset={6}
                align="start"
                alignOffset={2}
                className="backdrop-blur-xl bg-white/80 dark:bg-gray-900/80 text-gray-900 dark:text-gray-100"
              >
                Build my day
              </TooltipContentNoArrow>
            </Tooltip>
          ) : (
            <div className="flex items-center justify-center h-7 w-7 -ml-1 rounded-md bg-white/30 dark:bg-white/20 flex-shrink-0">
              <Activity className="w-4 h-4 text-white" />
            </div>
          )}

          {/* Collapse button - separate */}
          <button
            onClick={() => setIsCollapsed(!isCollapsed)}
            aria-expanded={!isCollapsed}
            aria-controls="recent-activity-panel"
            className="flex-1 flex items-center gap-2 hover:opacity-80 transition-opacity text-left"
          >
            <div className="flex flex-col items-start">
              <h3 className="text-sm font-medium text-gray-900 dark:text-gray-50">
                Recent Activity
              </h3>
              <span className="text-xs text-blue-600 dark:text-blue-400 font-medium">
                {suggestedEntries.length > 0
                  ? `${suggestedEntries.length} ${suggestedEntries.length === 1 ? 'suggestion' : 'suggestions'} pending approval`
                  : unclassifiedBlocksCount > 0
                    ? `${unclassifiedBlocksCount} ${unclassifiedBlocksCount === 1 ? 'block' : 'blocks'} to classify`
                    : 'No activity'}
              </span>
            </div>
          </button>

          {/* Filter/Sort or Chevron icon with smooth animation */}
          <div className="relative w-7 h-7 flex items-center justify-center">
            {/* Chevron button - always mounted */}
            <motion.button
              onClick={() => setIsCollapsed(!isCollapsed)}
              aria-label="Expand recent activity"
              animate={{
                opacity: isCollapsed ? 1 : 0,
                scale: isCollapsed ? 1 : 0.8,
                rotate: isCollapsed ? 0 : 90,
              }}
              transition={{
                opacity: { duration: 0.15, ease: 'easeInOut' },
                scale: { duration: 0.2, ease: 'easeInOut' },
                rotate: { duration: 0.2, ease: 'easeInOut' },
              }}
              className="absolute inset-0 flex items-center justify-center h-7 w-7 rounded-md hover:bg-white/10 transition-all flex-shrink-0"
              style={{
                pointerEvents: isCollapsed ? 'auto' : 'none',
                willChange: 'opacity, transform',
                visibility: isCollapsed ? 'visible' : 'hidden',
              }}
            >
              <ChevronUp className="w-5 h-5 text-gray-500 dark:text-gray-400 rotate-180" />
            </motion.button>

            {/* Filter button - always mounted */}
            <motion.div
              animate={{
                opacity: !isCollapsed ? 1 : 0,
                scale: !isCollapsed ? 1 : 0.8,
                rotate: !isCollapsed ? 0 : -90,
              }}
              transition={{
                opacity: { duration: 0.15, ease: 'easeInOut' },
                scale: { duration: 0.2, ease: 'easeInOut' },
                rotate: { duration: 0.2, ease: 'easeInOut' },
              }}
              className="absolute inset-0 flex items-center justify-center"
              style={{
                pointerEvents: !isCollapsed ? 'auto' : 'none',
                willChange: 'opacity, transform',
                visibility: !isCollapsed ? 'visible' : 'hidden',
              }}
            >
              <FilterSortPopover
                trigger={
                  <button
                    className="h-7 w-7 rounded-md flex items-center justify-center hover:bg-white/10 transition-all flex-shrink-0 relative"
                    aria-label="Filter and sort suggestions"
                  >
                    <Filter className="w-4 h-4 text-white" />
                    {activeFilterCount > 0 && (
                      <motion.span
                        initial={{ scale: 0 }}
                        animate={{ scale: 1 }}
                        transition={{ delay: 0.1, type: 'spring', stiffness: 500, damping: 15 }}
                        className="absolute -top-0.5 -right-0.5 h-3 w-3 bg-blue-500 rounded-full text-[8px] text-white font-bold flex items-center justify-center"
                      >
                        {activeFilterCount}
                      </motion.span>
                    )}
                  </button>
                }
                filterSortState={filterSortState}
                onFilterSortChange={setFilterSortState}
                activeFilterCount={activeFilterCount}
              />
            </motion.div>
          </div>
        </div>
      </div>

      {/* Content Area (tabs + entries) */}
      <AnimatePresence>
        {!isCollapsed && (
          <motion.section
            id="recent-activity-panel"
            role="region"
            aria-label="Recent Activity"
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: 'auto' }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.2 }}
            className="px-4"
          >
            <Tabs value={activeTab} onValueChange={(value) => setActiveTab(value as TabType)}>
              {/* Two-Tab System: Suggestions | Dismissed */}
              <TabsList className="w-full bg-white/20 dark:bg-white/10 backdrop-blur-xl mb-1 p-0.5">
                <TabsTrigger
                  value="suggestions"
                  className="flex-1 text-sm font-medium gap-1.5 data-[state=active]:bg-white/30 data-[state=active]:dark:bg-white/15 h-[calc(100%-0px)] text-gray-900 dark:text-gray-50"
                >
                  Suggested
                  <span className="text-xs text-blue-600 dark:text-blue-400 font-semibold">
                    {filteredAndSortedSuggestions.length}
                  </span>
                </TabsTrigger>
                <TabsTrigger
                  value="dismissed"
                  className="flex-1 text-sm font-medium gap-1.5 data-[state=active]:bg-white/30 data-[state=active]:dark:bg-white/15 h-[calc(100%-0px)] text-gray-900 dark:text-gray-50"
                >
                  Dismissed
                  <span className="text-xs">{filteredAndSortedDismissed.length}</span>
                </TabsTrigger>
              </TabsList>

              {/* Suggestions Tab Content */}
              <TabsContent value="suggestions" className="mt-0">
                {isLoading ? (
                  <div className="flex items-center justify-center py-4">
                    <Loader2 className="w-4 h-4 animate-spin text-gray-400" />
                  </div>
                ) : filteredAndSortedSuggestions.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-8 px-4">
                    <Activity className="w-8 h-8 text-gray-400 dark:text-gray-500 mb-2" />
                    <p className="text-sm text-gray-600 dark:text-gray-400 text-center">
                      No suggestions yet
                    </p>
                    <p className="text-xs text-gray-500 dark:text-gray-500 text-center mt-1">
                      Start the timer or use Build My Day to generate entries
                    </p>
                  </div>
                ) : (
                  <div
                    ref={suggestionsParentRef}
                    className="overflow-auto scrollbar-hide rounded-2xl"
                    style={{
                      height: `${Math.min(suggestionsVirtualizer.getTotalSize(), 297)}px`,
                      maxHeight: '297px', // 3 cards max (approx)
                    }}
                  >
                    <div
                      style={{
                        height: `${suggestionsVirtualizer.getTotalSize()}px`,
                        width: '100%',
                        position: 'relative',
                      }}
                    >
                      {suggestionsVirtualizer.getVirtualItems().map((virtualRow) => {
                        const entry = filteredAndSortedSuggestions[virtualRow.index];
                        if (!entry) return null;
                        return (
                          <div
                            key={entry.id}
                            data-index={virtualRow.index}
                            ref={suggestionsVirtualizer.measureElement}
                            style={{
                              position: 'absolute',
                              top: 0,
                              left: 0,
                              width: '100%',
                              transform: `translateY(${virtualRow.start}px)`,
                              paddingBottom: '8px', // Equal gap between cards
                            }}
                          >
                            <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 hover:bg-white/30 dark:hover:bg-white/15 transition-colors">
                              <div className="flex items-start gap-3">
                                {/* Category icons */}
                                {entry.category === 'personal' ? (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md bg-yellow-500/20 text-yellow-600 dark:text-yellow-400 flex-shrink-0">
                                    <User className="w-4 h-4" />
                                  </span>
                                ) : entry.category === 'general' ? (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md bg-blue-500/20 text-blue-600 dark:text-blue-400 flex-shrink-0">
                                    <Users className="w-4 h-4" />
                                  </span>
                                ) : entry.category === 'project' ? (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md text-orange-600 dark:text-orange-400 flex-shrink-0">
                                    <Calendar className="w-4 h-4" />
                                  </span>
                                ) : (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md bg-purple-500/20 text-purple-600 dark:text-purple-400 flex-shrink-0">
                                    <Brain className="w-4 h-4" />
                                  </span>
                                )}

                                {/* Entry Info */}
                                <div className="flex-1 min-w-0">
                                  <div className="flex items-center gap-2 mb-1">
                                    <span className="text-sm font-medium text-gray-900 dark:text-gray-50 truncate">
                                      {entry.project}
                                    </span>
                                    {/* Confidence badge - AI entries only - FEATURE-021: Added activity breakdown tooltip */}
                                    {entry.source === 'ai' &&
                                      entry.confidence &&
                                      entry.confidence > 0 && (
                                        <>
                                          {entry.activities && entry.activities.length > 0 ? (
                                            <Tooltip>
                                              <TooltipTrigger asChild>
                                                <span
                                                  className={`text-[10px] px-1.5 py-0.5 rounded-md font-medium cursor-help hover:opacity-80 transition-opacity ${entryService.getConfidenceColor(
                                                    entry.confidence ?? 0
                                                  )}`}
                                                >
                                                  {entry.confidence}%
                                                </span>
                                              </TooltipTrigger>
                                              <TooltipContentNoArrow
                                                side="bottom"
                                                align="center"
                                                sideOffset={4}
                                                className="bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 shadow-xl p-2"
                                              >
                                                <ActivityBreakdownTooltip
                                                  activities={entry.activities}
                                                  idleSeconds={entry.idleSeconds}
                                                  totalSeconds={entry.durationSeconds}
                                                  category={entry.category}
                                                />
                                              </TooltipContentNoArrow>
                                            </Tooltip>
                                          ) : (
                                            <span
                                              className={`text-[10px] px-1.5 py-0.5 rounded-md font-medium ${entryService.getConfidenceColor(
                                                entry.confidence ?? 0
                                              )}`}
                                            >
                                              {entry.confidence}%
                                            </span>
                                          )}
                                        </>
                                      )}
                                  </div>
                                  <div className="text-xs text-gray-700 dark:text-gray-300 truncate mb-1">
                                    {entry.task}
                                  </div>
                                  <div className="flex items-center gap-1.5 text-xs text-gray-600 dark:text-gray-400">
                                    <span>{entry.shortDate}</span>
                                    <span>•</span>
                                    <span>{entry.duration}</span>
                                    {entry.wbsCode && (
                                      <>
                                        <span>•</span>
                                        <span>{entry.wbsCode}</span>
                                      </>
                                    )}
                                  </div>
                                  {entry.description && (
                                    <div className="text-[10px] text-gray-600 dark:text-gray-400 mt-1 italic">
                                      {entry.description}
                                    </div>
                                  )}
                                </div>

                                {/* Action Buttons - Suggestions */}
                                <div className="flex items-center gap-1 flex-shrink-0">
                                  {/* Accept */}
                                  <button
                                    onClick={() => void handleAccept(entry)}
                                    aria-label="Accept suggestion"
                                    className="h-7 w-7 rounded-full flex items-center justify-center bg-gray-500/10 text-gray-600 dark:text-gray-400 border border-white/30 dark:border-white/20 hover:bg-green-500/30 hover:text-green-700 dark:hover:text-green-400 hover:border-green-500/50 transition-all hover:scale-110 active:scale-90"
                                  >
                                    <Check className="w-3.5 h-3.5" />
                                  </button>
                                  {/* Edit */}
                                  <button
                                    onClick={() => handleEdit(entry)}
                                    aria-label="Edit suggestion"
                                    className="h-7 w-7 rounded-full flex items-center justify-center bg-gray-500/10 text-gray-600 dark:text-gray-400 border border-white/30 dark:border-white/20 hover:bg-blue-500/30 hover:text-blue-700 dark:hover:text-blue-400 hover:border-blue-500/50 transition-all hover:scale-110 active:scale-90"
                                  >
                                    <Pencil className="w-3.5 h-3.5" />
                                  </button>
                                  {/* Dismiss */}
                                  <button
                                    onClick={() => void handleDismiss(entry.id)}
                                    aria-label="Dismiss suggestion"
                                    className="h-7 w-7 rounded-full flex items-center justify-center bg-gray-500/10 text-gray-600 dark:text-gray-400 border border-white/30 dark:border-white/20 hover:bg-red-500/30 hover:text-red-700 dark:hover:text-red-400 hover:border-red-500/50 transition-all hover:scale-110 active:scale-90"
                                  >
                                    <X className="w-3.5 h-3.5" />
                                  </button>
                                </div>
                              </div>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </TabsContent>

              {/* Dismissed Tab Content */}
              <TabsContent value="dismissed" className="mt-0">
                {isLoading ? (
                  <div className="flex items-center justify-center py-4">
                    <Loader2 className="w-4 h-4 animate-spin text-gray-400" />
                  </div>
                ) : filteredAndSortedDismissed.length === 0 ? (
                  <div className="flex flex-col items-center justify-center py-8 px-4">
                    <X className="w-8 h-8 text-gray-400 dark:text-gray-500 mb-2" />
                    <p className="text-sm text-gray-600 dark:text-gray-400 text-center">
                      No dismissed entries
                    </p>
                    <p className="text-xs text-gray-500 dark:text-gray-500 text-center mt-1">
                      Dismissed suggestions will appear here
                    </p>
                  </div>
                ) : (
                  <div
                    ref={dismissedParentRef}
                    className="overflow-auto scrollbar-hide rounded-2xl"
                    style={{
                      height: `${Math.min(dismissedVirtualizer.getTotalSize(), 297)}px`,
                      maxHeight: '297px', // 3 cards max (approx)
                    }}
                  >
                    <div
                      style={{
                        height: `${dismissedVirtualizer.getTotalSize()}px`,
                        width: '100%',
                        position: 'relative',
                      }}
                    >
                      {dismissedVirtualizer.getVirtualItems().map((virtualRow) => {
                        const entry = filteredAndSortedDismissed[virtualRow.index];
                        if (!entry) return null;
                        return (
                          <div
                            key={entry.id}
                            data-index={virtualRow.index}
                            ref={dismissedVirtualizer.measureElement}
                            style={{
                              position: 'absolute',
                              top: 0,
                              left: 0,
                              width: '100%',
                              transform: `translateY(${virtualRow.start}px)`,
                              paddingBottom: '8px', // Equal gap between cards
                            }}
                          >
                            <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 hover:bg-white/30 dark:hover:bg-white/15 transition-colors">
                              <div className="flex items-start gap-3">
                                {/* Category icons */}
                                {entry.category === 'personal' ? (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md bg-yellow-500/20 text-yellow-600 dark:text-yellow-400 flex-shrink-0">
                                    <User className="w-4 h-4" />
                                  </span>
                                ) : entry.category === 'general' ? (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md bg-blue-500/20 text-blue-600 dark:text-blue-400 flex-shrink-0">
                                    <Users className="w-4 h-4" />
                                  </span>
                                ) : entry.category === 'project' ? (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md text-orange-600 dark:text-orange-400 flex-shrink-0">
                                    <Calendar className="w-4 h-4" />
                                  </span>
                                ) : (
                                  <span className="inline-flex items-center justify-center h-7 w-7 rounded-md bg-purple-500/20 text-purple-600 dark:text-purple-400 flex-shrink-0">
                                    <Brain className="w-4 h-4" />
                                  </span>
                                )}

                                {/* Entry Info */}
                                <div className="flex-1 min-w-0">
                                  <div className="flex items-center gap-2 mb-1">
                                    <span className="text-sm font-medium text-gray-900 dark:text-gray-50 truncate">
                                      {entry.project}
                                    </span>
                                    {/* Confidence badge - AI entries only - FEATURE-021: Added activity breakdown tooltip */}
                                    {entry.source === 'ai' &&
                                      entry.confidence &&
                                      entry.confidence > 0 && (
                                        <>
                                          {entry.activities && entry.activities.length > 0 ? (
                                            <Tooltip>
                                              <TooltipTrigger asChild>
                                                <span
                                                  className={`text-[10px] px-1.5 py-0.5 rounded-md font-medium cursor-help hover:opacity-80 transition-opacity ${entryService.getConfidenceColor(
                                                    entry.confidence ?? 0
                                                  )}`}
                                                >
                                                  {entry.confidence}%
                                                </span>
                                              </TooltipTrigger>
                                              <TooltipContentNoArrow
                                                side="bottom"
                                                align="center"
                                                sideOffset={4}
                                                className="bg-neutral-100 dark:bg-neutral-900 border-2 border-neutral-200 dark:border-neutral-700 shadow-xl p-2"
                                              >
                                                <ActivityBreakdownTooltip
                                                  activities={entry.activities}
                                                  idleSeconds={entry.idleSeconds}
                                                  totalSeconds={entry.durationSeconds}
                                                  category={entry.category}
                                                />
                                              </TooltipContentNoArrow>
                                            </Tooltip>
                                          ) : (
                                            <span
                                              className={`text-[10px] px-1.5 py-0.5 rounded-md font-medium ${entryService.getConfidenceColor(
                                                entry.confidence ?? 0
                                              )}`}
                                            >
                                              {entry.confidence}%
                                            </span>
                                          )}
                                        </>
                                      )}
                                  </div>
                                  <div className="text-xs text-gray-700 dark:text-gray-300 truncate mb-1">
                                    {entry.task}
                                  </div>
                                  <div className="flex items-center gap-1.5 text-xs text-gray-600 dark:text-gray-400">
                                    <span>{entry.shortDate}</span>
                                    <span>•</span>
                                    <span>{entry.duration}</span>
                                    {entry.wbsCode && (
                                      <>
                                        <span>•</span>
                                        <span>{entry.wbsCode}</span>
                                      </>
                                    )}
                                  </div>
                                  {entry.description && (
                                    <div className="text-[10px] text-gray-600 dark:text-gray-400 mt-1 italic">
                                      {entry.description}
                                    </div>
                                  )}
                                </div>

                                {/* Action Buttons - Dismissed */}
                                <div className="flex items-center gap-1 flex-shrink-0">
                                  {/* Restore */}
                                  <button
                                    onClick={() => void handleRestore(entry.id)}
                                    aria-label="Restore"
                                    className="h-7 w-7 rounded-full flex items-center justify-center bg-gray-500/10 text-gray-600 dark:text-gray-400 border border-white/30 dark:border-white/20 hover:bg-blue-500/30 hover:text-blue-700 dark:hover:text-blue-400 hover:border-blue-500/50 transition-all hover:scale-110 active:scale-90"
                                  >
                                    <Undo2 className="w-3.5 h-3.5" />
                                  </button>
                                  {/* Delete */}
                                  <button
                                    onClick={() => void handleDelete(entry.id)}
                                    aria-label="Delete permanently"
                                    className="h-7 w-7 rounded-full flex items-center justify-center bg-gray-500/10 text-gray-600 dark:text-gray-400 border border-white/30 dark:border-white/20 hover:bg-red-500/30 hover:text-red-700 dark:hover:text-red-400 hover:border-red-500/50 transition-all hover:scale-110 active:scale-90"
                                  >
                                    <Trash2 className="w-3.5 h-3.5" />
                                  </button>
                                </div>
                              </div>
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  </div>
                )}
              </TabsContent>
            </Tabs>
          </motion.section>
        )}
      </AnimatePresence>

      {/* FEATURE-019 Phase 3: Modals */}
      <EditEntryModal
        entry={entryToEdit}
        isOpen={editModalOpen}
        onClose={() => setEditModalOpen(false)}
        onSave={handleSaveEdit}
      />

      <ClassifyEntryModal
        entry={entryToClassify}
        isOpen={classifyModalOpen}
        onClose={() => setClassifyModalOpen(false)}
        onClassify={handleClassify}
      />

      <DismissFeedbackModal
        entry={entryToDismiss}
        isOpen={dismissModalOpen}
        onClose={() => setDismissModalOpen(false)}
        onDismiss={handleConfirmDismiss}
      />

      {/* Authentication Error Modal */}
      <Dialog open={authErrorModalOpen} onOpenChange={setAuthErrorModalOpen}>
        <DialogContent className="sm:max-w-md bg-black/[0.925] dark:bg-black/[0.925] border-2 border-white/20 dark:border-white/10 shadow-[0_8px_32px_0_rgba(0,0,0,0.2),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_8px_32px_0_rgba(0,0,0,0.4),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
          <DialogHeader>
            <DialogTitle className="text-lg font-semibold text-gray-100 dark:text-gray-100">
              Authentication Required
            </DialogTitle>
            <DialogDescription className="text-sm text-gray-300 dark:text-gray-300 space-y-3 pt-2">
              <p>You need to sign in to your account before approving time entries.</p>
              <p>
                Time entries are synced to the Pulsarc API using GraphQL, which requires
                authentication.
              </p>
              <div className="pt-2">
                <p className="text-xs text-gray-400 dark:text-gray-400">
                  Go to{' '}
                  <span className="font-semibold text-gray-300 dark:text-gray-300">
                    Settings → Account
                  </span>{' '}
                  to sign in.
                </p>
              </div>
            </DialogDescription>
          </DialogHeader>
          <div className="flex justify-end gap-2 pt-4">
            <Button
              onClick={() => setAuthErrorModalOpen(false)}
              className="text-xs bg-white/10 dark:bg-white/10 hover:bg-white/20 dark:hover:bg-white/20 border border-white/20 dark:border-white/20 text-gray-100 dark:text-gray-100"
            >
              Got it
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
