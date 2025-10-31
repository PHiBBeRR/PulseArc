/**
 * FEATURE-009: Wire Frontend Timer UI to Tauri Backend Data
 * Unit tests for ProjectCache service
 *
 * Tests Issue #5: Project Name Lookup for Suggested Entries
 *
 * NOTE: This test file is ready for TDD implementation of the projectCache service.
 * The service module does not exist yet. Uncomment the import below when implementing.
 */

import type { Project } from '@/shared/types/generated';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

// Mock Tauri invoke (must be hoisted before imports)
const { mockInvoke } = vi.hoisted(() => ({
  mockInvoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: mockInvoke,
}));

import { projectCache } from './projectCache';

describe('ProjectCache - Issue #5: CUID to Name Lookup', () => {
  let mockProjects: Project[];

  beforeEach(() => {
    vi.clearAllMocks();

    // Reset cache state between tests
    projectCache.clear();

    mockProjects = [
      { id: 'cmglxfa357c03ceb9e3cf9d98', name: 'Project Alpha' },
      { id: 'proj-beta-cuid-abc123', name: 'Client Website' },
      { id: 'proj-gamma-xyz456', name: 'Backend API' },
      { id: 'proj-delta-789def', name: 'Mobile App Redesign' },
    ];

    mockInvoke.mockResolvedValue(mockProjects);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  describe('Fetching Projects from Backend', () => {
    it('should call get_user_projects Tauri command', async () => {
      await projectCache.fetchProjects();

      expect(mockInvoke).toHaveBeenCalledWith('get_user_projects');
    });

    it('should populate cache with project ID → name mappings', async () => {
      await projectCache.fetchProjects();

      expect(projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98')).toBe('Project Alpha');
      expect(projectCache.getProjectName('proj-beta-cuid-abc123')).toBe('Client Website');
      expect(projectCache.getProjectName('proj-gamma-xyz456')).toBe('Backend API');
    });

    it('should log cache size after successful fetch', async () => {
      const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

      await projectCache.fetchProjects();

      expect(consoleLogSpy).toHaveBeenCalledWith('✓ Cached 4 projects');

      consoleLogSpy.mockRestore();
    });
  });

  describe('TTL-Based Caching', () => {
    it('should return cached data within TTL (5 minutes)', async () => {
      await projectCache.fetchProjects();

      expect(mockInvoke).toHaveBeenCalledTimes(1);

      // Fetch again immediately (within TTL)
      await projectCache.fetchProjects();

      // Should NOT call backend again
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    it('should refetch after TTL expires (5 minutes)', async () => {
      vi.useFakeTimers();

      await projectCache.fetchProjects();

      expect(mockInvoke).toHaveBeenCalledTimes(1);

      // Fast-forward past TTL (5 minutes)
      vi.advanceTimersByTime(5 * 60 * 1000 + 1000);

      // Fetch again after TTL expiry
      await projectCache.fetchProjects();

      // Should call backend again
      expect(mockInvoke).toHaveBeenCalledTimes(2);

      vi.useRealTimers();
    });

    it('should check staleness with isStale()', async () => {
      vi.useFakeTimers();

      expect(projectCache.isStale()).toBe(true); // Never fetched

      await projectCache.fetchProjects();

      expect(projectCache.isStale()).toBe(false); // Fresh

      // Fast-forward past TTL
      vi.advanceTimersByTime(5 * 60 * 1000 + 1000);

      expect(projectCache.isStale()).toBe(true); // Stale

      vi.useRealTimers();
    });
  });

  describe('Preventing Duplicate Fetches (Race Condition)', () => {
    it('should reuse in-flight request if fetch is already pending', async () => {
      // Function to manually resolve the Promise (assigned in Promise callback)

      let resolveFirst: ((_: Project[]) => void) | undefined;

      mockInvoke.mockReturnValue(
        new Promise<Project[]>((resolve) => {
          resolveFirst = resolve;
        })
      );

      // Start two concurrent fetches
      const promise1 = projectCache.fetchProjects();
      const promise2 = projectCache.fetchProjects();

      // Resolve the backend call
      resolveFirst?.(mockProjects);

      await Promise.all([promise1, promise2]);

      // Should only call backend ONCE
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });

    // Note: fetchPromise is a private implementation detail, skipping this test
    // it('should clear fetchPromise after completion', async () => {
    //   await projectCache.fetchProjects();
    //   expect(projectCache.fetchPromise).toBeNull();
    // });
  });

  describe('Getting Project Names', () => {
    it('should return project name for valid CUID', async () => {
      await projectCache.fetchProjects();

      const name = projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98');
      expect(name).toBe('Project Alpha');
    });

    it('should return CUID as fallback if project not found', async () => {
      await projectCache.fetchProjects();

      const unknownCuid = 'unknown-cuid-xyz';
      const name = projectCache.getProjectName(unknownCuid);

      expect(name).toBe(unknownCuid); // Graceful degradation
    });

    it('should log warning if project not found in cache', async () => {
      const consoleWarnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      await projectCache.fetchProjects();

      const unknownCuid = 'missing-project-id';
      projectCache.getProjectName(unknownCuid);

      expect(consoleWarnSpy).toHaveBeenCalledWith(`Project not found in cache: ${unknownCuid}`);

      consoleWarnSpy.mockRestore();
    });

    it('should work immediately after fetching without additional calls', async () => {
      await projectCache.fetchProjects();

      // Multiple lookups should NOT trigger additional fetches
      projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98');
      projectCache.getProjectName('proj-beta-cuid-abc123');
      projectCache.getProjectName('proj-gamma-xyz456');

      expect(mockInvoke).toHaveBeenCalledTimes(1); // Only initial fetch
    });
  });

  describe('Error Handling', () => {
    it('should handle backend fetch errors gracefully', async () => {
      const consoleErrorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
      mockInvoke.mockRejectedValue(new Error('Backend unavailable'));

      await projectCache.fetchProjects();

      expect(consoleErrorSpy).toHaveBeenCalledWith('Failed to fetch projects:', expect.any(Error));

      consoleErrorSpy.mockRestore();
    });

    it('should keep stale cache on error (graceful degradation)', async () => {
      vi.useFakeTimers();

      // First successful fetch
      await projectCache.fetchProjects();

      expect(projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98')).toBe('Project Alpha');

      // Fast-forward past TTL
      vi.advanceTimersByTime(5 * 60 * 1000 + 1000);

      // Second fetch fails
      mockInvoke.mockRejectedValue(new Error('Network error'));
      await projectCache.fetchProjects();

      // Should still have stale cache data
      expect(projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98')).toBe('Project Alpha');

      vi.useRealTimers();
    });

    // Note: fetchPromise is a private implementation detail, skipping this test
    // it('should clear fetchPromise on error', async () => {
    //   mockInvoke.mockRejectedValue(new Error('Fetch failed'));
    //   await projectCache.fetchProjects();
    //   expect(projectCache.fetchPromise).toBeNull();
    // });
  });

  describe('Preload Functionality', () => {
    it('should preload projects on demand', async () => {
      await projectCache.preload();

      expect(mockInvoke).toHaveBeenCalledWith('get_user_projects');
      expect(projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98')).toBe('Project Alpha');
    });

    it('should be idempotent with fetchProjects', async () => {
      await projectCache.preload();
      await projectCache.fetchProjects();

      // Should only fetch once (within TTL)
      expect(mockInvoke).toHaveBeenCalledTimes(1);
    });
  });

  describe('Manual Refresh', () => {
    it('should force refresh even if cache is fresh', async () => {
      await projectCache.fetchProjects();

      expect(mockInvoke).toHaveBeenCalledTimes(1);

      // Force refresh (bypasses TTL)
      await projectCache.refresh();

      expect(mockInvoke).toHaveBeenCalledTimes(2);
    });

    it('should invalidate cache before refreshing', async () => {
      vi.useFakeTimers();

      await projectCache.fetchProjects();

      expect(projectCache.isStale()).toBe(false);

      // Refresh invalidates cache
      const refreshPromise = projectCache.refresh();

      expect(projectCache.isStale()).toBe(true); // Invalidated

      await refreshPromise;

      expect(projectCache.isStale()).toBe(false); // Fresh again

      vi.useRealTimers();
    });
  });

  describe('Cache Replacement Strategy', () => {
    it('should replace cache contents on each fetch', async () => {
      await projectCache.fetchProjects();

      expect(projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98')).toBe('Project Alpha');

      // Mock different projects on next fetch
      const newProjects: Project[] = [
        { id: 'new-proj-1', name: 'New Project 1' },
        { id: 'new-proj-2', name: 'New Project 2' },
      ];

      mockInvoke.mockResolvedValue(newProjects);

      // Force refresh to get new data
      await projectCache.refresh();

      // Old project should no longer be in cache
      expect(projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98')).toBe(
        'cmglxfa357c03ceb9e3cf9d98'
      ); // Falls back to CUID

      // New projects should be cached
      expect(projectCache.getProjectName('new-proj-1')).toBe('New Project 1');
      expect(projectCache.getProjectName('new-proj-2')).toBe('New Project 2');
    });
  });

  describe('N+1 Query Prevention', () => {
    it('should cache all projects in a single fetch', async () => {
      await projectCache.fetchProjects();

      // Multiple lookups should NOT trigger additional backend calls
      for (let i = 0; i < 100; i++) {
        projectCache.getProjectName('cmglxfa357c03ceb9e3cf9d98');
        projectCache.getProjectName('proj-beta-cuid-abc123');
        projectCache.getProjectName('proj-gamma-xyz456');
      }

      expect(mockInvoke).toHaveBeenCalledTimes(1); // Only the initial fetch
    });
  });

  describe('Singleton Pattern', () => {
    it('should export single instance of ProjectCache', () => {
      // All imports should reference the same instance
      expect(projectCache).toBeDefined();
      expect(projectCache).toBe(projectCache); // Same reference
    });
  });

  describe('Empty Projects List', () => {
    it('should handle empty projects list gracefully', async () => {
      mockInvoke.mockResolvedValue([]);

      await projectCache.fetchProjects();

      // Should not throw, just have empty cache
      expect(projectCache.getProjectName('any-id')).toBe('any-id');
    });
  });
});
