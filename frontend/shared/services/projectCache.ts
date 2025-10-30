/**
 * FEATURE-009: ProjectCache Service
 * 
 * Singleton cache for project ID → name mappings
 * Prevents N+1 query problem when displaying suggested entries
 * Uses 5-minute TTL to balance freshness vs performance
 */

import { invoke } from '@tauri-apps/api/core';
import type { Project } from '@/shared/types/generated';

/**
 * Singleton cache for project ID → name mappings
 */
class ProjectCache {
  private cache: Map<string, string> = new Map();
  private fetchPromise: Promise<void> | null = null;
  private lastFetch: number = 0;
  private readonly TTL = 5 * 60 * 1000; // 5 minutes

  /**
   * Fetch all projects from backend and populate cache
   * Uses singleton promise to prevent duplicate fetches
   */
  async fetchProjects(): Promise<void> {
    const now = Date.now();

    // Return cached data if fresh (within TTL)
    if (now - this.lastFetch < this.TTL && this.cache.size > 0) {
      return;
    }

    // Reuse in-flight request if exists (prevents race conditions)
    if (this.fetchPromise) {
      return this.fetchPromise;
    }

    this.fetchPromise = (async () => {
      try {
        // Call Tauri command to fetch projects from Neon
        const projects = await invoke<Project[]>('get_user_projects');

        // Replace cache contents
        this.cache.clear();
        projects.forEach((p) => this.cache.set(p.id, p.name));
        this.lastFetch = Date.now();

        console.log(`✓ Cached ${projects.length} projects`);
      } catch (error) {
        console.error('Failed to fetch projects:', error);
        // Keep stale cache on error (graceful degradation)
      } finally {
        this.fetchPromise = null;
      }
    })();

    return this.fetchPromise;
  }

  /**
   * Get project name by ID
   * @param projectId - CUID from time entry
   * @returns Project name or projectId if not found (graceful degradation)
   */
  getProjectName(projectId: string): string {
    const name = this.cache.get(projectId);
    if (!name) {
      console.warn(`Project not found in cache: ${projectId}`);
      return projectId; // Fallback to CUID
    }
    return name;
  }

  /**
   * Check if cache needs refresh
   */
  isStale(): boolean {
    const now = Date.now();
    return now - this.lastFetch >= this.TTL;
  }

  /**
   * Preload projects (call on app mount)
   */
  async preload(): Promise<void> {
    await this.fetchProjects();
  }

  /**
   * Force refresh (for testing or manual refresh)
   */
  async refresh(): Promise<void> {
    this.lastFetch = 0; // Invalidate cache
    await this.fetchProjects();
  }

  /**
   * Clear cache completely (for testing)
   * @internal
   */
  clear(): void {
    this.cache.clear();
    this.lastFetch = 0;
    this.fetchPromise = null;
  }
}

// Export singleton instance
export const projectCache = new ProjectCache();

