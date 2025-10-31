// Project color coding system
// Provides consistent color assignments across the app

import type { ProjectColor, ProjectColorMapping } from '../types';

// Predefined color palette for projects
export const PROJECT_COLORS: Record<string, ProjectColor> = {
  blue: {
    name: 'Blue',
    bg: 'bg-blue-500/10 dark:bg-blue-400/10',
    border: 'border-blue-500/30 dark:border-blue-400/30',
    text: 'text-blue-600 dark:text-blue-400',
    dot: 'bg-blue-500 dark:bg-blue-400',
  },
  purple: {
    name: 'Purple',
    bg: 'bg-purple-500/10 dark:bg-purple-400/10',
    border: 'border-purple-500/30 dark:border-purple-400/30',
    text: 'text-purple-600 dark:text-purple-400',
    dot: 'bg-purple-500 dark:bg-purple-400',
  },
  green: {
    name: 'Green',
    bg: 'bg-green-500/10 dark:bg-green-400/10',
    border: 'border-green-500/30 dark:border-green-400/30',
    text: 'text-green-600 dark:text-green-400',
    dot: 'bg-green-500 dark:bg-green-400',
  },
  orange: {
    name: 'Orange',
    bg: 'bg-orange-500/10 dark:bg-orange-400/10',
    border: 'border-orange-500/30 dark:border-orange-400/30',
    text: 'text-orange-600 dark:text-orange-400',
    dot: 'bg-orange-500 dark:bg-orange-400',
  },
  pink: {
    name: 'Pink',
    bg: 'bg-pink-500/10 dark:bg-pink-400/10',
    border: 'border-pink-500/30 dark:border-pink-400/30',
    text: 'text-pink-600 dark:text-pink-400',
    dot: 'bg-pink-500 dark:bg-pink-400',
  },
  red: {
    name: 'Red',
    bg: 'bg-red-500/10 dark:bg-red-400/10',
    border: 'border-red-500/30 dark:border-red-400/30',
    text: 'text-red-600 dark:text-red-400',
    dot: 'bg-red-500 dark:bg-red-400',
  },
  yellow: {
    name: 'Yellow',
    bg: 'bg-yellow-500/10 dark:bg-yellow-400/10',
    border: 'border-yellow-500/30 dark:border-yellow-400/30',
    text: 'text-yellow-600 dark:text-yellow-400',
    dot: 'bg-yellow-500 dark:bg-yellow-400',
  },
  teal: {
    name: 'Teal',
    bg: 'bg-teal-500/10 dark:bg-teal-400/10',
    border: 'border-teal-500/30 dark:border-teal-400/30',
    text: 'text-teal-600 dark:text-teal-400',
    dot: 'bg-teal-500 dark:bg-teal-400',
  },
  indigo: {
    name: 'Indigo',
    bg: 'bg-indigo-500/10 dark:bg-indigo-400/10',
    border: 'border-indigo-500/30 dark:border-indigo-400/30',
    text: 'text-indigo-600 dark:text-indigo-400',
    dot: 'bg-indigo-500 dark:bg-indigo-400',
  },
  cyan: {
    name: 'Cyan',
    bg: 'bg-cyan-500/10 dark:bg-cyan-400/10',
    border: 'border-cyan-500/30 dark:border-cyan-400/30',
    text: 'text-cyan-600 dark:text-cyan-400',
    dot: 'bg-cyan-500 dark:bg-cyan-400',
  },
  slate: {
    name: 'Slate',
    bg: 'bg-slate-500/10 dark:bg-slate-400/10',
    border: 'border-slate-500/30 dark:border-slate-400/30',
    text: 'text-slate-600 dark:text-slate-400',
    dot: 'bg-slate-500 dark:bg-slate-400',
  },
  gray: {
    name: 'Gray',
    bg: 'bg-gray-500/10 dark:bg-gray-400/10',
    border: 'border-gray-500/30 dark:border-gray-400/30',
    text: 'text-gray-600 dark:text-gray-400',
    dot: 'bg-gray-500 dark:bg-gray-400',
  },
};

// Project to color mapping (can be stored in localStorage later)
export const PROJECT_COLOR_MAP: ProjectColorMapping = {
  'Project Alpha': 'blue',
  'Project Beta': 'purple',
  'Deep Work': 'green',
  Meetings: 'orange',
  'Daily Standup': 'pink',
  Admin: 'gray',
  Learning: 'teal',
  'Personal Project': 'indigo',
  'Code Review': 'cyan',
  'Design System': 'purple',
  'Client Work': 'red',
  Research: 'yellow',
};

// Get color for a project (with fallback)
export function getProjectColor(projectName: string): ProjectColor {
  const colorKey = PROJECT_COLOR_MAP[projectName] ?? 'slate';
  const color = PROJECT_COLORS[colorKey];
  // TypeScript can't infer that slate will always exist, so we use a type assertion
  // We know this is safe because slate is defined in PROJECT_COLORS
  return color ?? PROJECT_COLORS.slate;
}

// Assign a color to a project (for future settings)
export function assignProjectColor(projectName: string, colorKey: string) {
  if (PROJECT_COLORS[colorKey]) {
    PROJECT_COLOR_MAP[projectName] = colorKey;
    // In the future, save to localStorage
    // localStorage.setItem('projectColors', JSON.stringify(PROJECT_COLOR_MAP));
  }
}

// Get a hash-based color for unknown projects (consistent coloring)
export function getHashedProjectColor(projectName: string): ProjectColor {
  const colors = Object.keys(PROJECT_COLORS);
  let hash = 0;
  for (let i = 0; i < projectName.length; i++) {
    hash = projectName.charCodeAt(i) + ((hash << 5) - hash);
  }
  const index = Math.abs(hash) % colors.length;
  const colorKey = colors[index];
  if (!colorKey) return PROJECT_COLORS.slate;
  const color = PROJECT_COLORS[colorKey];
  // TypeScript can't infer that slate will always exist, so we use a type assertion
  // We know this is safe because slate is defined in PROJECT_COLORS
  return color ?? PROJECT_COLORS.slate;
}

// Get all available colors for settings UI
export function getAllColors(): Array<{ key: string; color: ProjectColor }> {
  return Object.entries(PROJECT_COLORS).map(([key, color]) => ({ key, color }));
}
