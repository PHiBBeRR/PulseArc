// Shared types
export type { TimeEntry, Project, AppSettings, NotificationConfig, ViewMode } from './common.types';

// Auto-generated backend types (from Rust via ts-rs)
export type {
  ActivitySnapshot,
  ActivitySegment,
  ActivityContext,
  WindowContext,
  WorkType,
  ActivityCategory,
  ConfidenceEvidence,
  ActivityMetadata,
  BatchQueue,
  BatchStatus,
  BatchStats,
  TimeEntryOutbox,
  OutboxStatus,
  PrismaTimeEntryDto,
  IdMapping,
  DatabaseStats,
  SyncStats,
  OutboxStats,
  DlqBatch,
} from './generated';

// Tauri API wrappers with timestamp normalization
export {
  getRecentSnapshots,
  getRecentActivities,
  getRecentSegments,
  getBatchStatus,
  getOutboxStatus,
  getSyncStats,
  getDatabaseStats,
  getCostSummary,
} from './tauri-backend.types';
