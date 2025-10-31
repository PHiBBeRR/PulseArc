import { timelineService } from '../services/timelineService';
import type { TimelineViewProps } from '../types';

export function TimelineView({ entries }: TimelineViewProps) {
  const hours = timelineService.getHourMarkers();

  return (
    <div className="mt-4">
      <div className="text-sm font-medium text-gray-500 dark:text-gray-400 mb-2 px-3">Timeline</div>
      <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-3 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
        {/* Hour markers */}
        <div className="relative mb-2">
          <div className="flex justify-between text-sm text-gray-400 dark:text-gray-500 mb-1">
            {hours.map((hour) => (
              <span key={hour} className="w-8 text-center">
                {timelineService.formatHourMarker(hour)}
              </span>
            ))}
          </div>
          <div className="h-px bg-white/30 dark:bg-white/20" />
        </div>

        {/* Timeline entries */}
        <div className="relative h-20 mt-3">
          {entries.map((entry, index) => {
            const left = timelineService.calculatePosition(entry.startTime);
            const width = timelineService.calculateWidth(entry.duration);

            return (
              <div
                key={entry.id}
                className={`absolute h-12 rounded-lg border ${timelineService.getStatusColor(entry.status)} backdrop-blur-sm shadow-sm overflow-hidden group cursor-pointer hover:scale-105 transition-transform`}
                style={{
                  left: `${left}%`,
                  width: `${width}%`,
                  top: `${index * 16}px`,
                }}
              >
                <div className="px-2 py-1.5 h-full flex flex-col justify-center">
                  <div className="text-xs text-gray-900 dark:text-gray-100 truncate">
                    {entry.task}
                  </div>
                  <div className="text-xs text-gray-600 dark:text-gray-400 truncate">
                    {entry.duration}m
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
