import { Skeleton } from '@/shared/components/ui/skeleton';

export function EntryCardSkeleton() {
  return (
    <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
      <div className="flex items-start justify-between mb-3">
        <div className="flex-1 space-y-2">
          <Skeleton className="h-4 w-24 bg-white/40 dark:bg-white/20" />
          <Skeleton className="h-5 w-40 bg-white/40 dark:bg-white/20" />
          <Skeleton className="h-4 w-32 bg-white/40 dark:bg-white/20" />
        </div>
        <Skeleton className="h-6 w-12 bg-white/40 dark:bg-white/20" />
      </div>
      <Skeleton className="h-4 w-20 bg-white/40 dark:bg-white/20" />
    </div>
  );
}

export function EntriesListSkeleton() {
  return (
    <div className="space-y-3">
      {[1, 2, 3, 4].map((i) => (
        <EntryCardSkeleton key={i} />
      ))}
    </div>
  );
}

export function TimelineEntrySkeleton() {
  return (
    <div className="backdrop-blur-xl border rounded-xl p-3 bg-white/20 dark:bg-white/10 border-white/30 dark:border-white/20">
      <div className="flex items-start justify-between mb-2">
        <div className="flex-1 space-y-2">
          <Skeleton className="h-4 w-32 bg-white/40 dark:bg-white/20" />
          <Skeleton className="h-3 w-24 bg-white/40 dark:bg-white/20" />
        </div>
      </div>
      <div className="flex items-center gap-2">
        <Skeleton className="h-3 w-16 bg-white/40 dark:bg-white/20" />
        <Skeleton className="h-3 w-12 bg-white/40 dark:bg-white/20" />
      </div>
    </div>
  );
}

export function AnalyticsChartSkeleton() {
  return (
    <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-6 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
      <Skeleton className="h-5 w-32 mb-4 bg-white/40 dark:bg-white/20" />
      <div className="h-64 flex items-end gap-2">
        {[1, 2, 3, 4, 5, 6, 7].map((i) => (
          <div key={i} className="flex-1 flex flex-col gap-2 justify-end">
            <Skeleton
              className="w-full bg-white/40 dark:bg-white/20"
              style={{ height: `${Math.random() * 80 + 20}%` }}
            />
          </div>
        ))}
      </div>
    </div>
  );
}

export function StatCardSkeleton() {
  return (
    <div className="backdrop-blur-xl bg-white/20 dark:bg-white/10 border border-white/30 dark:border-white/20 rounded-2xl p-4 shadow-[0_4px_16px_0_rgba(0,0,0,0.08),0_0_0_1px_rgba(255,255,255,0.6)_inset] dark:shadow-[0_4px_16px_0_rgba(0,0,0,0.3),0_0_0_1px_rgba(255,255,255,0.1)_inset]">
      <Skeleton className="h-4 w-20 mb-2 bg-white/40 dark:bg-white/20" />
      <Skeleton className="h-8 w-24 bg-white/40 dark:bg-white/20" />
    </div>
  );
}
