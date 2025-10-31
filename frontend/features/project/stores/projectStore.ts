// Project feature Zustand store
import type { ActivitySnapshot } from '@/shared/types/generated';
import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import type { Project, RecentProject } from '../types';

interface ProjectState {
  // State
  projects: Project[];
  recentProjects: RecentProject[];
  loading: boolean;
  error: string | null;

  // Actions
  setProjects: (projects: Project[]) => void;
  setRecentProjects: (recentProjects: RecentProject[]) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;

  // Tauri backend integration
  fetchProjects: () => Promise<void>;
  fetchRecentProjects: () => Promise<void>;

  // Utility actions
  getProjectById: (id: string) => Project | undefined;
  getRecentProjectById: (id: string) => RecentProject | undefined;
}

/**
 * Project store - manages project state with Tauri backend integration
 */
export const useProjectStore = create<ProjectState>((set, get) => ({
  // Initial state
  projects: [],
  recentProjects: [],
  loading: false,
  error: null,

  // Setters
  setProjects: (projects) => set({ projects }),
  setRecentProjects: (recentProjects) => set({ recentProjects }),
  setLoading: (loading) => set({ loading }),
  setError: (error) => set({ error }),

  // Fetch projects from Tauri backend
  fetchProjects: async () => {
    set({ loading: true, error: null });
    try {
      const projects = await invoke<Project[]>('get_user_projects');
      set({ projects, loading: false });
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Failed to fetch projects';
      set({ error: errorMessage, loading: false });
    }
  },

  // Fetch recent projects from activity snapshots
  fetchRecentProjects: async () => {
    set({ loading: true, error: null });
    try {
      const activities = await invoke<ActivitySnapshot[]>('get_recent_activities', {
        limit: 20,
      });

      // Extract unique projects (applications) from activities
      const projectMap = new Map<string, RecentProject>();

      activities.forEach((activity) => {
        const projectId = activity.primary_app || 'unassigned';
        const projectName = activity.primary_app || 'Unassigned';
        const task = activity.detected_activity || 'Recent activity';

        // Only add if not already in map (keeps first occurrence)
        if (!projectMap.has(projectId)) {
          projectMap.set(projectId, {
            id: projectId,
            project: projectName,
            task: task,
          });
        }
      });

      const recentProjects = Array.from(projectMap.values());
      set({ recentProjects, loading: false });
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : 'Failed to fetch recent projects';
      set({ error: errorMessage, loading: false });
    }
  },

  // Utility actions
  getProjectById: (id) => {
    const state = get();
    return state.projects.find((p) => p.id === id);
  },

  getRecentProjectById: (id) => {
    const state = get();
    return state.recentProjects.find((p) => p.id === id);
  },
}));
