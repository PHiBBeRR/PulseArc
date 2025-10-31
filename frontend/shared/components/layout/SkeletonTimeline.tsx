// PERF-002: Skeleton screen for timeline loading
// Shows animated placeholders during data fetch

export function SkeletonTimeline() {
  return (
    <div className="space-y-4 p-4">
      {[...Array(5)].map((_, i) => (
        <div key={i} className="animate-pulse">
          <div className="flex items-center space-x-4">
            <div className="h-12 w-12 bg-gray-200 dark:bg-gray-700 rounded-full" />
            <div className="flex-1 space-y-2">
              <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-3/4" />
              <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-1/2" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}

export function SkeletonLoader({ count = 3 }: { count?: number }) {
  return (
    <div className="space-y-3">
      {[...Array(count)].map((_, i) => (
        <div key={i} className="animate-pulse">
          <div className="h-4 bg-gray-200 dark:bg-gray-700 rounded w-full mb-2" />
          <div className="h-3 bg-gray-200 dark:bg-gray-700 rounded w-2/3" />
        </div>
      ))}
    </div>
  );
}
