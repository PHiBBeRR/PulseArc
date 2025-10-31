import type { ActivityBreakdown } from '@/shared/types/generated';
import { Moon } from 'lucide-react';

interface ActivityBreakdownTooltipProps {
  activities: ActivityBreakdown[];
  idleSeconds?: number; // Idle time within entry
  totalSeconds?: number; // Total duration for calculating idle percentage
  category?: 'personal' | 'general' | 'project' | 'ai'; // Entry category to determine if idle should be shown
}

/**
 *  Phase 4: Activity Breakdown Tooltip
 * Enhanced with idle period display
 *
 * Displays a color-coded breakdown of activities within a time block.
 * Shows activity name, duration, and percentage with visual progress bars.
 * Includes idle periods as a separate item when present (only for General entries).
 */
export function ActivityBreakdownTooltip({
  activities,
  idleSeconds,
  totalSeconds,
  category,
}: ActivityBreakdownTooltipProps) {
  // Calculate idle percentage if we have both idle and total
  const idlePercentage =
    idleSeconds && totalSeconds && totalSeconds > 0
      ? Math.round((idleSeconds / totalSeconds) * 100)
      : 0;

  const hasActivities = activities && activities.length > 0;
  // Only show idle time for General entries and if >= 1 minute (60 seconds)
  const hasIdle = category === 'general' && idleSeconds && idleSeconds >= 60;

  if (!hasActivities && !hasIdle) {
    return (
      <div className="text-xs text-gray-500 dark:text-gray-400">
        No activity breakdown available
      </div>
    );
  }

  // Map app names to color classes
  const getAppColor = (appName: string): string => {
    const name = appName.toLowerCase();

    if (name.includes('excel')) return 'bg-green-500';
    if (name.includes('chrome') || name.includes('browser')) return 'bg-yellow-500';
    if (name.includes('outlook') || name.includes('mail')) return 'bg-blue-500';
    if (name.includes('word')) return 'bg-blue-400';
    if (name.includes('powerpoint')) return 'bg-orange-500';
    if (name.includes('teams')) return 'bg-purple-500';
    if (name.includes('zoom')) return 'bg-indigo-500';
    if (name.includes('slack')) return 'bg-pink-500';
    if (name.includes('vscode') || name.includes('code')) return 'bg-cyan-500';
    if (name.includes('terminal') || name.includes('iterm')) return 'bg-gray-700';

    return 'bg-gray-500';
  };

  return (
    <div className="space-y-1.5 min-w-[160px]">
      {/* Header */}
      <div className="text-[10px] font-semibold text-gray-900 dark:text-gray-100 mb-1">
        Activity Breakdown
      </div>

      {/* Activity list */}
      {hasActivities &&
        activities.map((activity, idx) => (
          <div key={idx} className="space-y-0.5">
            {/* App name + duration */}
            <div className="flex justify-between items-center gap-2">
              <span className="text-[10px] font-medium text-gray-700 dark:text-gray-300 truncate">
                {activity.name}
              </span>
              <span className="text-[9px] text-gray-500 dark:text-gray-400 tabular-nums whitespace-nowrap">
                {Math.floor(activity.duration_secs / 60)}m
              </span>
            </div>

            {/* Color-coded progress bar */}
            <div className="w-full h-1 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
              <div
                className={`h-full ${getAppColor(activity.name)} transition-all duration-300`}
                style={{ width: `${activity.percentage}%` }}
              />
            </div>
          </div>
        ))}

      {/* Idle period (if present) */}
      {hasIdle && (
        <div className="space-y-0.5">
          {/* Idle label + duration */}
          <div className="flex justify-between items-center gap-2">
            <div className="flex items-center gap-1">
              <Moon className="w-2.5 h-2.5 text-amber-600 dark:text-amber-400" />
              <span className="text-[10px] font-medium text-amber-700 dark:text-amber-300 truncate">
                Idle
              </span>
            </div>
            <span className="text-[9px] text-amber-600 dark:text-amber-400 tabular-nums whitespace-nowrap">
              {Math.floor((idleSeconds ?? 0) / 60)}m
            </span>
          </div>

          {/* Amber progress bar for idle */}
          <div className="w-full h-1 bg-gray-200 dark:bg-gray-700 rounded-full overflow-hidden">
            <div
              className="h-full bg-amber-500 transition-all duration-300"
              style={{ width: `${idlePercentage}%` }}
            />
          </div>
        </div>
      )}
    </div>
  );
}
