/**
 * FEATURE-009: Wire Frontend Timer UI to Tauri Backend Data
 * Integration tests for projectStore with Tauri backend
 *
 * Tests Issue #3: Project Store Fetches Real Recent Projects
 */

import { createMockActivitySnapshot } from '@/shared/test/fixtures/backend-types';
import type { ActivitySnapshot } from '@/shared/types/tauri-backend.types';
import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri invoke (must be hoisted before imports)
const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

import { useProjectStore } from './projectStore';

describe('ProjectStore - Issue #3: Fetch Real Recent Projects', () => {
  let mockActivitySnapshots: ActivitySnapshot[];

  beforeEach(() => {
    vi.clearAllMocks();

    // Reset Zustand store state
    const { setState } = useProjectStore;
    setState({
      projects: [],
      recentProjects: [],
      loading: false,
      error: null,
    });

    // Mock activity snapshots with various apps
    mockActivitySnapshots = [
      createMockActivitySnapshot({
        id: 'snap-1',
        timestamp: Date.now() - 1000,
        detected_activity: 'Writing code',
        work_type: 'modeling',
        activity_category: 'work',
        primary_app: 'Visual Studio Code',
        created_at: Date.now() - 1000,
      }),
      createMockActivitySnapshot({
        id: 'snap-2',
        timestamp: Date.now() - 2000,
        detected_activity: 'Reading documentation',
        work_type: 'research',
        activity_category: 'work',
        primary_app: 'Google Chrome',
        created_at: Date.now() - 2000,
      }),
      createMockActivitySnapshot({
        id: 'snap-3',
        timestamp: Date.now() - 3000,
        detected_activity: 'Different activity',
        work_type: 'modeling',
        activity_category: 'work',
        primary_app: 'Visual Studio Code', // Duplicate - should dedupe
        created_at: Date.now() - 3000,
      }),
      createMockActivitySnapshot({
        id: 'snap-4',
        timestamp: Date.now() - 4000,
        detected_activity: 'Design work',
        work_type: 'modeling',
        activity_category: 'work',
        primary_app: 'Figma',
        created_at: Date.now() - 4000,
      }),
    ];

    mockInvoke.mockResolvedValue(mockActivitySnapshots);
  });

  describe('Fetching Recent Projects from Backend', () => {
    it('should call get_recent_activities with limit parameter', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      expect(mockInvoke).toHaveBeenCalledWith('get_recent_activities', {
        limit: 20,
      });
    });

    it('should extract unique projects from activity snapshots', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toHaveLength(3); // VS Code, Chrome, Figma (dedupe VS Code)
      });

      const projectNames = result.current.recentProjects.map((p) => p.project);
      expect(projectNames).toContain('Visual Studio Code');
      expect(projectNames).toContain('Google Chrome');
      expect(projectNames).toContain('Figma');
    });

    it('should use primary_app as project name', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        const vscodeProject = result.current.recentProjects.find(
          (p) => p.project === 'Visual Studio Code'
        );
        expect(vscodeProject).toBeDefined();
        expect(vscodeProject?.id).toBe('Visual Studio Code'); // primary_app used as ID
      });
    });

    it('should use detected_activity as task name', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        const vscodeProject = result.current.recentProjects.find(
          (p) => p.project === 'Visual Studio Code'
        );
        expect(vscodeProject?.task).toBe('Writing code');
      });
    });

    it('should handle missing primary_app gracefully', async () => {
      const snapshotsWithMissingApp: ActivitySnapshot[] = [
        createMockActivitySnapshot({
          primary_app: '',
        }),
      ];

      mockInvoke.mockResolvedValue(snapshotsWithMissingApp);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toHaveLength(1);
        expect(result.current.recentProjects[0]?.project).toBe('Unassigned');
        expect(result.current.recentProjects[0]?.id).toBe('unassigned');
      });
    });

    it('should handle missing detected_activity gracefully', async () => {
      const snapshotsWithMissingLabel: ActivitySnapshot[] = [
        createMockActivitySnapshot({
          detected_activity: '',
        }),
      ];

      mockInvoke.mockResolvedValue(snapshotsWithMissingLabel);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects[0]?.task).toBe('Recent activity');
      });
    });
  });

  describe('Loading State Management', () => {
    it('should set loading=true while fetching', async () => {
      // Function to manually resolve the Promise (assigned in Promise callback)

      let resolveInvoke: ((_: ActivitySnapshot[]) => void) | undefined;

      mockInvoke.mockReturnValue(
        new Promise<ActivitySnapshot[]>((resolve) => {
          resolveInvoke = resolve;
        })
      );

      const { result } = renderHook(() => useProjectStore());

      const fetchPromise = result.current.fetchRecentProjects();

      // Should be loading
      await waitFor(() => {
        expect(result.current.loading).toBe(true);
      });

      // Resolve the fetch
      resolveInvoke?.(mockActivitySnapshots);

      await fetchPromise;

      // Should no longer be loading
      expect(result.current.loading).toBe(false);
    });

    it('should set loading=false after successful fetch', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.loading).toBe(false);
      });
    });

    it('should clear error on successful fetch', async () => {
      const { result } = renderHook(() => useProjectStore());

      // Set an error first
      act(() => {
        result.current.setError('Previous error');
      });
      expect(result.current.error).toBe('Previous error');

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.error).toBeNull();
      });
    });
  });

  describe('Error Handling', () => {
    it('should handle backend fetch errors gracefully', async () => {
      mockInvoke.mockRejectedValue(new Error('Backend unavailable'));

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.error).toBe('Backend unavailable');
        expect(result.current.loading).toBe(false);
      });
    });

    it('should preserve previous state on error', async () => {
      const { result } = renderHook(() => useProjectStore());

      // First successful fetch
      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toHaveLength(3);
      });

      const previousProjects = result.current.recentProjects;

      // Second fetch fails
      mockInvoke.mockRejectedValue(new Error('Network error'));
      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.error).toBe('Network error');
      });

      // Previous data should still be there
      expect(result.current.recentProjects).toEqual(previousProjects);
    });

    it('should extract error message from Error objects', async () => {
      mockInvoke.mockRejectedValue(new Error('Custom error message'));

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.error).toBe('Custom error message');
      });
    });
  });

  describe('Deduplication Logic', () => {
    it('should deduplicate projects with same primary_app', async () => {
      const duplicateSnapshots: ActivitySnapshot[] = [
        createMockActivitySnapshot({ primary_app: 'Duplicate App' }),
        createMockActivitySnapshot({ primary_app: 'Duplicate App' }),
        createMockActivitySnapshot({ primary_app: 'Duplicate App' }),
      ];

      mockInvoke.mockResolvedValue(duplicateSnapshots);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toHaveLength(1);
        expect(result.current.recentProjects[0]?.project).toBe('Duplicate App');
      });
    });

    it('should keep first occurrence when deduplicating', async () => {
      const snapshotsWithDifferentLabels: ActivitySnapshot[] = [
        createMockActivitySnapshot({
          primary_app: 'Same App',
          detected_activity: 'First activity',
        }),
        createMockActivitySnapshot({
          primary_app: 'Same App',
          detected_activity: 'Second activity', // Should be ignored
        }),
      ];

      mockInvoke.mockResolvedValue(snapshotsWithDifferentLabels);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toHaveLength(1);
        expect(result.current.recentProjects[0]?.task).toBe('First activity'); // First one kept
      });
    });
  });

  describe('Empty State Handling', () => {
    it('should handle empty activity list', async () => {
      mockInvoke.mockResolvedValue([]);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toEqual([]);
        expect(result.current.loading).toBe(false);
        expect(result.current.error).toBeNull();
      });
    });
  });

  describe('Removal of Mock Data', () => {
    it('should NOT return hardcoded mock projects', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        // Should NOT contain the old mock data
        const projectNames = result.current.recentProjects.map((p) => p.project);
        expect(projectNames).not.toContain('Project Alpha');
        expect(projectNames).not.toContain('Project Beta');
        expect(projectNames).not.toContain('Deep Work');
        expect(projectNames).not.toContain('Meetings');
      });
    });

    it('should return data from backend only', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        // Should contain ONLY backend data
        const projectNames = result.current.recentProjects.map((p) => p.project);
        expect(projectNames).toContain('Visual Studio Code');
        expect(projectNames).toContain('Google Chrome');
        expect(projectNames).toContain('Figma');
        expect(projectNames).toHaveLength(3); // Exactly 3, no mock data
      });
    });
  });

  describe('Utility Methods', () => {
    it('should find recent project by ID', async () => {
      const { result } = renderHook(() => useProjectStore());

      await result.current.fetchRecentProjects();

      await waitFor(() => {
        const project = result.current.getRecentProjectById('Visual Studio Code');
        expect(project).toBeDefined();
        expect(project?.project).toBe('Visual Studio Code');
      });
    });

    it('should return undefined for non-existent project', async () => {
      const { result } = renderHook(() => useProjectStore());

      await result.current.fetchRecentProjects();

      await waitFor(() => {
        const project = result.current.getRecentProjectById('Non-existent App');
        expect(project).toBeUndefined();
      });
    });
  });

  describe('Project vs Application Semantics', () => {
    it('should use application names (not business projects)', async () => {
      const { result } = renderHook(() => useProjectStore());

      await result.current.fetchRecentProjects();

      await waitFor(() => {
        // Should return OS application names
        const projectNames = result.current.recentProjects.map((p) => p.project);
        expect(projectNames).toContain('Visual Studio Code'); // Application name
        expect(projectNames).toContain('Google Chrome'); // Application name
      });

      // Note: UI should say "Recent Apps" not "Recent Projects"
      // (This is a naming clarification, not a functional bug)
    });

    it('should clarify that primary_app represents applications, not business projects', async () => {
      const mockActivities = [
        createMockActivitySnapshot({ primary_app: 'Visual Studio Code' }),
        createMockActivitySnapshot({ primary_app: 'Google Chrome' }),
      ];

      mockInvoke.mockResolvedValue(mockActivities);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      // Verify data is from applications
      expect(result.current.recentProjects[0]?.project).toBe('Visual Studio Code');
      expect(result.current.recentProjects[1]?.project).toBe('Google Chrome');

      // This test documents that these are app names, not business project names
      // UI should say "Recent Apps" or fetch actual projects via project_id lookup
    });
  });

  describe('Backend Integration', () => {
    it('should enable backend integration by default', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        // Should call backend by default
        expect(mockInvoke).toHaveBeenCalledWith('get_recent_activities', { limit: 20 });
      });
    });

    it('should call backend when enabled', async () => {
      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith('get_recent_activities', { limit: 20 });
      });
    });
  });

  describe('Empty Activities Handling', () => {
    it('should handle empty activities list gracefully', async () => {
      mockInvoke.mockResolvedValue([]);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        expect(result.current.recentProjects).toEqual([]);
        expect(result.current.error).toBeNull();
        expect(result.current.loading).toBe(false);
      });
    });
  });

  describe('Deduplication with Multiple Activities from Same App', () => {
    it('should keep only first occurrence when same app appears multiple times', async () => {
      const mockActivities = [
        createMockActivitySnapshot({
          primary_app: 'Visual Studio Code',
          detected_activity: 'Editing MainTimer.tsx',
          timestamp: 3,
        }),
        createMockActivitySnapshot({
          primary_app: 'Google Chrome',
          detected_activity: 'Reading docs',
          timestamp: 2,
        }),
        createMockActivitySnapshot({
          primary_app: 'Visual Studio Code',
          detected_activity: 'Debugging tests',
          timestamp: 1, // Earlier
        }),
      ];

      mockInvoke.mockResolvedValue(mockActivities);

      const { result } = renderHook(() => useProjectStore());

      await act(async () => {
        await result.current.fetchRecentProjects();
      });

      await waitFor(() => {
        // Should have only 2 unique apps
        expect(result.current.recentProjects).toHaveLength(2);

        // Should keep first occurrence (most recent activity)
        const vscodeProject = result.current.recentProjects.find(
          (p) => p.project === 'Visual Studio Code'
        );
        expect(vscodeProject?.task).toBe('Editing MainTimer.tsx');
      });
    });
  });
});
