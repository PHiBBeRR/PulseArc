//! Domain types and models
//!
//! PHASE-0: Full legacy type contracts ported for compatibility

pub mod classification;
pub mod database;
pub mod idle;
pub mod sap;
pub mod stats;
pub mod user;

use chrono::{DateTime, Utc};
// Re-export classification types
pub use classification::ProposedBlock;
// Re-export database types for convenience
pub use database::{
    AcceptPatch, ActivitySegment, ActivitySnapshot, BatchQueue, BatchStatus, CalendarEventParams,
    CalendarEventRow, CalendarSyncSettingsParams, CalendarSyncSettingsRow, CalendarTokenRow,
    ContextPart, DatabaseSize, HealthStatus, IdMapping, OutboxStatus, ParsedFields,
    PrismaTimeEntryDto, Project, ProjectWithWbs, SuggestionFeedbackParams, TableStats,
    TimeEntryOutbox, TimeRange,
};
pub use idle::{IdlePeriod, IdleSummary};
pub use sap::{OutboxStatusSummary, SapSyncSettings, WbsElement};
use serde::{Deserialize, Serialize};
pub use stats::{
    BatchStats, ClassificationMode, DatabaseStats, DlqBatch, OutboxStats, SyncStats, TokenUsage,
    TokenVariance, UserCostSummary,
};
pub use user::UserProfile;

// Type alias for API compatibility
/// Block is an alias for ProposedBlock (used in API contexts)
pub type Block = ProposedBlock;
#[cfg(feature = "ts-gen")]
use ts_rs::TS;
use uuid::Uuid;

/// Time entry representing classified work ready for persistence
///
/// This structure merges the legacy Prisma DTO fields (strings, optional
/// metadata) with strongly-typed chrono/UUID fields used by the new domain
/// services.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct TimeEntry {
    #[cfg_attr(feature = "ts-gen", ts(type = "string"))]
    pub id: Uuid,
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub org_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub user_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub project_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub task_id: Option<String>,
    #[cfg_attr(feature = "ts-gen", ts(type = "string"))]
    pub start_time: DateTime<Utc>,
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub end_time: Option<DateTime<Utc>>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub duration_seconds: Option<i64>,
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub duration_minutes: Option<i32>,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-gen", ts(type = "string", optional))]
    pub entry_date: Option<String>,
    pub wbs_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_workstream: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_task: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts-gen", ts(type = "number", optional))]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_breakdown: Option<Vec<database::ContextPart>>,
}

impl TimeEntry {
    /// Create a new time entry with the required core fields.
    ///
    /// Additional metadata (billable, task_id, etc.) defaults to `None`.
    #[must_use]
    pub fn new(params: TimeEntryParams) -> Self {
        let TimeEntryParams {
            id,
            start_time,
            end_time,
            duration_seconds,
            description,
            project_id,
            wbs_code,
        } = params;

        Self {
            id,
            org_id: None,
            user_id: None,
            project_id,
            task_id: None,
            start_time,
            end_time,
            duration_seconds,
            duration_minutes: duration_seconds.map(|secs| (secs / 60) as i32),
            description,
            notes: None,
            billable: None,
            source: None,
            status: None,
            entry_date: Some(start_time.date_naive().to_string()),
            wbs_code,
            display_project: None,
            display_workstream: None,
            display_task: None,
            confidence: None,
            context_breakdown: None,
        }
    }
}

/// Core parameters required to construct a `TimeEntry`.
#[derive(Debug)]
pub struct TimeEntryParams {
    pub id: Uuid,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub description: String,
    pub project_id: Option<String>,
    pub wbs_code: Option<String>,
}

// ============================================================================
// Core Activity Types
// ============================================================================

/// WorkType: WHAT you're doing (separate from billable classification)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkType {
    Modeling,        // Spreadsheet modeling, financial analysis
    DocReview,       // PDF/document review, contracts
    Research,        // Bloomberg, FactSet, PitchBook, web research
    Email,           // Email correspondence
    Meeting,         // Video calls, calendar meetings
    DMS,             // Document management (iManage, NetDocuments, SharePoint)
    DataRoom,        // Virtual data rooms (Datasite, Intralinks)
    AccountingSuite, // QuickBooks, tax software, bookkeeping
    Documentation,   // Writing reports, proposals, memos
    Unknown,
}

/// ActivityCategory: SHOULD it bill (drives billing classification)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityCategory {
    ClientWork,     // Direct billable work (0.95 base)
    Research,       // Potentially billable (0.60 base)
    Communication,  // Context-dependent (0.70 base)
    Administrative, // Non-billable (0.10 base)
    Meeting,        // Context-dependent (0.75 base)
    Documentation,  // Potentially billable (0.65 base)
    Internal,       // Non-billable (0.15 base)
}

impl ActivityCategory {
    /// Returns the base confidence score for this activity category
    pub fn base_confidence(&self) -> f32 {
        match self {
            Self::ClientWork => 0.95,
            Self::Research => 0.60,
            Self::Communication => 0.70,
            Self::Meeting => 0.75,
            Self::Documentation => 0.65,
            Self::Administrative => 0.10,
            Self::Internal => 0.15,
        }
    }
}

impl Default for ActivityCategory {
    fn default() -> Self {
        Self::Internal // Default to non-billable when unknown
    }
}

/// Confidence evidence for auditability
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfidenceEvidence {
    pub reasons: Vec<String>,
}

/// Extracted metadata from activity context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ActivityMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_identifier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matter_number: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_subject: Option<String>,
}

/// Window context (app and enrichment metadata)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowContext {
    pub app_name: String,
    pub window_title: String,
    pub bundle_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url_host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

/// Calendar event context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CalendarEventContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_title: Option<String>,
    #[serde(default)]
    pub has_external_attendees: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizer_email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meeting_platform: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parsed_workstream: Option<String>,
}

/// Location context (FEATURE-033)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LocationContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_location: Option<String>,
    #[serde(default)]
    pub is_travel: bool,
}

/// Temporal context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemporalContext {
    #[serde(default)]
    pub is_weekend: bool,
    #[serde(default)]
    pub is_after_hours: bool,
}

/// Classification context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ClassificationContext {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred_project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred_wbs_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred_deal_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inferred_workstream: Option<String>,
    #[serde(default)]
    pub billable: bool,
    #[serde(default)]
    pub confidence: f32,
}

/// Activity context captured from operating system with full enrichment
///
/// PHASE-0: Full legacy structure with 13+ nested fields
/// Source: legacy/api/src/shared/types/mod.rs:173-204
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityContext {
    pub active_app: WindowContext,
    pub recent_apps: Vec<WindowContext>,
    pub detected_activity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub work_type: Option<WorkType>,
    #[serde(default)]
    pub activity_category: ActivityCategory,
    #[serde(default)]
    pub billable_confidence: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_client: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_matter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_task_code: Option<String>,
    #[serde(default)]
    pub extracted_metadata: ActivityMetadata,
    #[serde(default)]
    pub evidence: ConfidenceEvidence,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calendar_event: Option<CalendarEventContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temporal_context: Option<TemporalContext>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub classification: Option<ClassificationContext>,
}
