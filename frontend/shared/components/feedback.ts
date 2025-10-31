// Feedback components barrel export
// Re-export feedback-related components from their actual locations

// From UI components
export { CompactErrorAlert } from './ui/compact-error-alert';
export { ErrorMessage } from './ui/error-message';
export { InWidgetNotification } from './ui/in-widget-notification';
export { LoadingSpinner } from './ui/loading-spinner';
export type { Notification, NotificationType } from './ui/in-widget-notification';

// From Layout components (skeletons and empty states)
export { CompactEmptyState } from './layout/CompactEmptyState';
export {
  AnalyticsChartSkeleton,
  EntriesListSkeleton,
  EntryCardSkeleton,
  StatCardSkeleton,
  TimelineEntrySkeleton,
} from './layout/LoadingSkeletons';
export { SkeletonTimeline } from './layout/SkeletonTimeline';