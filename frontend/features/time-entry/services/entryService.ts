// Time Entry business logic service

import type { AISuggestion } from '../types';

export const entryService = {
  /**
   * Get AI suggestion based on time of day, duration, and patterns
   */
  getAISuggestion: (elapsedSeconds: number): AISuggestion => {
    const hour = new Date().getHours();
    const durationMinutes = Math.floor(elapsedSeconds / 60);

    // Morning suggestions (6am - 12pm)
    if (hour >= 6 && hour < 12) {
      if (durationMinutes < 30) {
        return {
          project: 'Daily Standup',
          task: 'Team sync meeting',
          confidence: 92,
          reason: 'Short morning sessions usually match your standup pattern',
        };
      }
      return {
        project: 'Project Alpha',
        task: 'Feature development',
        confidence: 88,
        reason: 'You typically code in the morning',
      };
    }

    // Afternoon suggestions (12pm - 5pm)
    if (hour >= 12 && hour < 17) {
      if (durationMinutes < 20) {
        return {
          project: 'Meetings',
          task: 'Quick sync call',
          confidence: 85,
          reason: 'Short afternoon blocks match your meeting schedule',
        };
      }
      if (durationMinutes > 90) {
        return {
          project: 'Deep Work',
          task: 'Focus session',
          confidence: 90,
          reason: 'Long afternoon sessions typically indicate focused work',
        };
      }
      return {
        project: 'Project Beta',
        task: 'Code review',
        confidence: 87,
        reason: 'Based on your typical afternoon work patterns',
      };
    }

    // Evening suggestions (5pm - 10pm)
    if (hour >= 17 && hour < 22) {
      if (durationMinutes < 30) {
        return {
          project: 'Admin',
          task: 'Email & planning',
          confidence: 83,
          reason: 'Evening wrap-up sessions match this pattern',
        };
      }
      return {
        project: 'Learning',
        task: 'Tutorial & documentation',
        confidence: 86,
        reason: 'You often learn new skills in the evening',
      };
    }

    // Default/Late night
    return {
      project: 'Personal Project',
      task: 'Side project work',
      confidence: 80,
      reason: 'Late sessions typically match personal project time',
    };
  },

  /**
   * Format duration from seconds to human-readable format
   */
  formatDuration: (seconds: number): string => {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);

    if (hours > 0 && minutes > 0) {
      return `${hours}h ${minutes}m`;
    } else if (hours > 0) {
      return `${hours}h`;
    } else {
      return `${minutes}m`;
    }
  },

  /**
   * Parse duration string to seconds
   */
  parseDuration: (durationStr: string): number => {
    const hourMatch = durationStr.match(/(\d+)h/);
    const minuteMatch = durationStr.match(/(\d+)m/);

    const hours = hourMatch?.[1] ? parseInt(hourMatch[1]) : 0;
    const minutes = minuteMatch?.[1] ? parseInt(minuteMatch[1]) : 0;

    return hours * 3600 + minutes * 60;
  },

  /**
   * Validate entry data
   */
  validateEntry: (data: {
    project: string;
    task: string;
    duration: string;
  }): {
    isValid: boolean;
    errors: { project: boolean; task: boolean; duration: boolean };
  } => {
    const errors = {
      project: !data.project || data.project.trim() === '',
      task: !data.task || data.task.trim() === '',
      duration: !data.duration || data.duration.trim() === '',
    };

    return {
      isValid: !errors.project && !errors.task && !errors.duration,
      errors,
    };
  },

  /**
   * Get confidence color for badge (red/yellow/green scale)
   */
  getConfidenceColor: (confidence: number): string => {
    if (confidence >= 90)
      return 'bg-green-500/20 text-green-700 dark:text-green-400 border border-green-500/30';
    if (confidence >= 70)
      return 'bg-yellow-500/20 text-yellow-700 dark:text-yellow-400 border border-yellow-500/30';
    return 'bg-red-500/20 text-red-700 dark:text-red-400 border border-red-500/30';
  },

  /**
   * Get status badge color
   */
  getStatusColor: (status: 'pending' | 'approved' | 'suggested'): string => {
    switch (status) {
      case 'suggested':
        return 'bg-blue-500/20 text-blue-700 dark:text-blue-300 border-blue-500/30';
      case 'approved':
        return 'bg-green-500/20 text-green-700 dark:text-green-300 border-green-500/30';
      case 'pending':
        return 'bg-yellow-500/20 text-yellow-700 dark:text-yellow-300 border-yellow-500/30';
      default:
        return 'bg-gray-500/20 text-gray-700 dark:text-gray-300 border-gray-500/30';
    }
  },

  /**
   * Sort entries by time (most recent first)
   */
  sortEntriesByTime: <T extends { time: string }>(entries: T[]): T[] => {
    return [...entries].sort((a, b) => {
      // Simple time comparison - would need proper date parsing in production
      return b.time.localeCompare(a.time);
    });
  },

  /**
   * Filter entries by status
   */
  filterEntriesByStatus: <T extends { status: string }>(entries: T[], status?: string): T[] => {
    if (!status || status === 'all') return entries;
    return entries.filter((entry) => entry.status === status);
  },

  /**
   * Get today's total duration in seconds
   */
  getTodayTotal: (entries: { durationSeconds?: number }[]): number => {
    return entries.reduce((total, entry) => {
      return total + (entry.durationSeconds ?? 0);
    }, 0);
  },

  /**
   * Get ML explanation for entry
   */
  getMLExplanation: (entry: { status: string; project: string }): string => {
    if (entry.status === 'suggested') {
      return `AI detected similar work patterns at this time based on your ${entry.project} activity history`;
    }
    return '';
  },
};
