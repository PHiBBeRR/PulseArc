// Project business logic service

import type { RecentProject, Project } from '../types';

export const projectService = {
  /**
   * Search projects by name or task
   */
  searchProjects: (query: string, projects: RecentProject[]): RecentProject[] => {
    const lowerQuery = query.toLowerCase();
    return projects.filter(
      (p) => p.project.toLowerCase().includes(lowerQuery) || p.task.toLowerCase().includes(lowerQuery)
    );
  },

  /**
   * Get project by ID
   */
  getProjectById: (id: string, projects: RecentProject[]): RecentProject | undefined => {
    return projects.find((p) => p.id === id);
  },

  /**
   * Sort projects by most recently used
   */
  sortByRecent: (projects: RecentProject[]): RecentProject[] => {
    // In a real app, this would sort by last used timestamp
    return [...projects];
  },

  /**
   * Group projects by category (if needed)
   */
  groupByCategory: (): Record<string, Project[]> => {
    // For future implementation - projects parameter will be used when implemented
    return {};
  },

  /**
   * Validate project name
   */
  validateProjectName: (name: string): { valid: boolean; error?: string } => {
    if (!name || name.trim() === '') {
      return { valid: false, error: 'Project name is required' };
    }
    if (name.length > 50) {
      return { valid: false, error: 'Project name is too long (max 50 characters)' };
    }
    return { valid: true };
  },
};
