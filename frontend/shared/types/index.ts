// Shared types
export type { AppSettings, NotificationConfig, Project, TimeEntry, ViewMode } from './common.types';

// Auto-generated backend types (from Rust via ts-rs)
export type {
  ActivityCategory,
  ActivityContext,
  ActivityMetadata,
  ActivitySegment,
  ActivitySnapshot,
  BatchQueue,
  BatchStats,
  BatchStatus,
  ConfidenceEvidence,
  DatabaseStats,
  DlqBatch,
  IdMapping,
  OutboxStats,
  OutboxStatus,
  PrismaTimeEntryDto,
  SyncStats,
  TimeEntryOutbox,
  WindowContext,
  WorkType,
} from './generated';

// Tauri API wrappers with timestamp normalization
export {
  getBatchStatus,
  getCostSummary,
  getDatabaseStats,
  getOutboxStatus,
  getRecentActivities,
  getRecentSegments,
  getRecentSnapshots,
  getSyncStats,
} from './tauri-backend.types';
