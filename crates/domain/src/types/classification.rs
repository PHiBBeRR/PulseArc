//! Type definitions for block builder inference
//!
//! This module defines core types for block building, signal extraction,
//! and project matching. Types exported to TypeScript use ts-rs derives.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

/// Configuration for block building behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct BlockConfig {
    /// Minimum block duration in seconds (default: 1800 = 30 min)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub min_block_duration_secs: i64,

    /// Maximum gap for merging within same project (default: 180 = 3 min)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub max_gap_for_merge_secs: i64,

    /// Consolidation window for related activities (default: 3600 = 1 hour)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub consolidation_window_secs: i64,

    /// Minimum billing increment (default: 360 = 6 min)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub min_billing_increment_secs: i64,
}

impl Default for BlockConfig {
    fn default() -> Self {
        Self {
            min_block_duration_secs: 1800,   // 30 minutes
            max_gap_for_merge_secs: 180,     // 3 minutes
            consolidation_window_secs: 3600, // 1 hour
            min_billing_increment_secs: 360, // 6 minutes
        }
    }
}

/// Work location category for location context tracking (FEATURE-033 Phase 2)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub enum WorkLocation {
    /// Working from home
    Home,
    /// Working from office
    Office,
    /// Traveling (flights, trains, mobile hotspot)
    Travel,
}

/// A proposed time block (30+ minutes of consolidated work)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ProposedBlock {
    /// Unique identifier for the block
    pub id: String,

    /// Start timestamp (Unix epoch seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_ts: i64,

    /// End timestamp (Unix epoch seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end_ts: i64,

    /// Total duration in seconds
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub duration_secs: i64,

    // Inferred context
    /// Inferred project ID (e.g., "USC0063201")
    pub inferred_project_id: Option<String>,

    /// Inferred WBS code (e.g., "USC0063201.1.1")
    pub inferred_wbs_code: Option<String>,

    /// Inferred deal/project name (e.g., "Project Astro")
    pub inferred_deal_name: Option<String>,

    /// Inferred workstream (e.g., "modeling", "due_diligence")
    pub inferred_workstream: Option<String>,

    // Classification
    /// Whether this block is billable
    pub billable: bool,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Which classifier was used (FEATURE-030)
    pub classifier_used: Option<String>,

    // Supporting data
    /// Breakdown of activities within the block
    pub activities: Vec<ActivityBreakdown>,

    /// IDs of snapshots that compose this block (internal traceability)
    pub snapshot_ids: Vec<String>,

    /// IDs of segments that compose this block (internal traceability)
    /// REFACTOR-004: Added for layered traceability (segments → blocks)
    pub segment_ids: Vec<String>,

    /// Reasoning for project assignment and classification
    pub reasons: Vec<String>,

    // User interaction
    /// Block status: "suggested", "accepted", "rejected", "edited"
    pub status: String,

    /// When the block was created (Unix epoch seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,

    /// When the user reviewed the block (Unix epoch seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub reviewed_at: Option<i64>,

    // FEATURE-028 Phase 3: Idle time tracking
    /// Total idle time within block (for analytics)
    pub total_idle_secs: i32,
    /// Idle handling strategy: 'exclude' | 'include' | 'partial'
    pub idle_handling: String,

    // FEATURE-033 Phase 2: Location & context tracking
    /// User timezone (e.g., "America/Denver")
    pub timezone: Option<String>,
    /// Work location (home/office/travel)
    pub work_location: Option<WorkLocation>,
    /// Flag for travel time blocks (flights, trains, etc.)
    #[serde(default)]
    pub is_travel: bool,
    /// Flag for weekend work
    #[serde(default)]
    pub is_weekend: bool,
    /// Flag for after-hours work (outside 8am-6pm)
    #[serde(default)]
    pub is_after_hours: bool,

    // FEATURE-033 Phase 5: Overlap detection & conflict flagging
    /// Whether this block overlaps with any calendar events
    #[serde(default)]
    pub has_calendar_overlap: bool,
    /// IDs of calendar events that overlap with this block
    #[serde(default)]
    pub overlapping_event_ids: Vec<String>,
    /// Whether multiple calendar events overlap at the same time
    /// (double-booked)
    #[serde(default)]
    pub is_double_booked: bool,
}

impl ProposedBlock {
    /// Estimate token count for this block (for OpenAI API batching)
    /// Rough estimate: ~1 token per 4 characters
    pub fn estimated_token_count(&self) -> usize {
        let mut char_count = 0;

        // Duration and timestamps
        char_count += 30;

        // Project and workstream info
        if let Some(ref project) = self.inferred_project_id {
            char_count += project.len();
        }
        if let Some(ref workstream) = self.inferred_workstream {
            char_count += workstream.len();
        }

        // Activities breakdown
        for activity in &self.activities {
            char_count += activity.name.len() + 20; // name + metadata
        }

        // Reasons
        for reason in &self.reasons {
            char_count += reason.len();
        }

        // Convert to tokens (rough estimate: 1 token ≈ 4 chars)
        (char_count / 4).max(50) // Minimum 50 tokens
    }
}

/// Individual activity within a block
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ActivityBreakdown {
    /// Activity name (e.g., "Microsoft Excel", "Google Chrome")
    pub name: String,

    /// Duration of this activity in seconds
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub duration_secs: i64,

    /// Percentage of total block duration (0-100)
    pub percentage: f32,
}

/// Context signals extracted from an activity snapshot
///
/// This type is internal and not exported to TypeScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSignals {
    /// Keywords extracted from window title (e.g., ["astro", "ppa",
    /// "modeling"])
    pub title_keywords: Vec<String>,

    /// Extracted URL domain (e.g., "app.datasite.com")
    pub url_domain: Option<String>,

    /// Full file path if available (e.g., "~/Documents/Astro/model.xlsx")
    pub file_path: Option<String>,

    /// Project folder name extracted from file path (e.g., "Astro")
    pub project_folder: Option<String>,

    /// Associated calendar event ID (Phase 2)
    pub calendar_event_id: Option<String>,

    /// Email domains of meeting attendees (Phase 2)
    pub attendee_domains: Vec<String>,

    /// Categorized application type
    pub app_category: AppCategory,

    /// Whether the activity involves a VDR provider (Datasite, Intralinks,
    /// etc.)
    pub is_vdr_provider: bool,

    // FEATURE-030: Additional fields for comprehensive classification
    /// Timestamp of this signal (Unix epoch)
    #[serde(default)]
    pub timestamp: i64,

    /// Project ID this signal points to (if identified)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Organizer domain for calendar events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organizer_domain: Option<String>,

    /// Screen is locked / idle state
    #[serde(default)]
    pub is_screen_locked: bool,

    /// Personal calendar event detected
    #[serde(default)]
    pub has_personal_event: bool,

    /// Internal training/CPE detected
    #[serde(default)]
    pub is_internal_training: bool,

    /// Personal browsing detected (social media, entertainment)
    #[serde(default)]
    pub is_personal_browsing: bool,

    /// Email direction for email signals (V2.1)
    /// "outgoing" = 1.0x, "incoming" = 0.95x, "cc" = 0.85x
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email_direction: Option<String>,

    /// FEATURE-033 Phase 4: Meeting has external (non-company) attendees
    /// Used to infer client work vs internal meetings
    #[serde(default)]
    pub has_external_meeting_attendees: bool,
}

impl ContextSignals {
    /// Create a minimal ContextSignals for testing (all optional fields
    /// defaulted)
    #[cfg(test)]
    pub fn test_minimal(title_keywords: Vec<String>, app_category: AppCategory) -> Self {
        Self {
            title_keywords,
            url_domain: None,
            file_path: None,
            project_folder: None,
            calendar_event_id: None,
            attendee_domains: vec![],
            app_category,
            is_vdr_provider: false,
            timestamp: 0,
            project_id: None,
            organizer_domain: None,
            is_screen_locked: false,
            has_personal_event: false,
            is_internal_training: false,
            is_personal_browsing: false,
            email_direction: None,
            has_external_meeting_attendees: false,
        }
    }
}

impl Default for ContextSignals {
    fn default() -> Self {
        Self {
            title_keywords: vec![],
            url_domain: None,
            file_path: None,
            project_folder: None,
            calendar_event_id: None,
            attendee_domains: vec![],
            app_category: AppCategory::Other,
            is_vdr_provider: false,
            timestamp: 0,
            project_id: None,
            organizer_domain: None,
            is_screen_locked: false,
            has_personal_event: false,
            is_internal_training: false,
            is_personal_browsing: false,
            email_direction: None,
            has_external_meeting_attendees: false,
        }
    }
}

/// Application category for workstream inference
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppCategory {
    /// Microsoft Excel or spreadsheet applications
    Excel,
    /// Microsoft Word or document editors
    Word,
    /// Microsoft PowerPoint or presentation software
    PowerPoint,
    /// Web browsers (Chrome, Safari, Firefox)
    Browser,
    /// Email clients (Outlook, Mail)
    Email,
    /// Video conferencing (Zoom, Teams, Meet)
    Meeting,
    /// Terminal/command line
    Terminal,
    /// Code editors (Cursor, VS Code, Xcode)
    IDE,
    /// Other/uncategorized applications
    Other,
}

/// Project match result from signal scoring
///
/// This type is internal and not exported to TypeScript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMatch {
    /// Matched project ID (e.g., "USC0063201")
    pub project_id: Option<String>,

    /// Matched WBS code (e.g., "USC0063201.1.1")
    pub wbs_code: Option<String>,

    /// Deal/project name (e.g., "Project Astro")
    pub deal_name: Option<String>,

    /// Inferred workstream based on app category
    pub workstream: Option<String>,

    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,

    /// Reasons for the match (e.g., ["keyword:astro", "vdr:datasite"])
    pub reasons: Vec<String>,
}

// ============================================================================
// REFACTOR-003: Versioned Serialization Wrappers
// ============================================================================

/// Versioned wrapper for ContextSignals serialization
///
/// This wrapper provides forward compatibility by tracking the schema version.
/// If the ContextSignals struct changes in the future, we can deserialize old
/// versions and migrate them to the new format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedSignals {
    /// Schema version (starts at 1)
    pub version: u32,
    /// The actual signals data
    pub data: ContextSignals,
}

impl SerializedSignals {
    /// Create a new versioned signals wrapper (current version: 1)
    pub fn new(signals: ContextSignals) -> Self {
        Self { version: 1, data: signals }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Versioned wrapper for ProjectMatch serialization
///
/// This wrapper provides forward compatibility by tracking the schema version.
/// If the ProjectMatch struct changes in the future, we can deserialize old
/// versions and migrate them to the new format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedProjectMatch {
    /// Schema version (starts at 1)
    pub version: u32,
    /// The actual project match data
    pub data: ProjectMatch,
}

impl SerializedProjectMatch {
    /// Create a new versioned project match wrapper (current version: 1)
    pub fn new(project_match: ProjectMatch) -> Self {
        Self { version: 1, data: project_match }
    }

    /// Serialize to JSON string
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Deserialize from JSON string
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// ============================================================================
// REFACTOR-003: Unit Tests for Serialization
// ============================================================================

#[cfg(test)]
mod refactor_003_tests {
    use super::*;

    /// Test 1: Verify ContextSignals can be serialized and deserialized
    /// REQUIREMENT: Phase 2 - Add Serialize/Deserialize derives to
    /// ContextSignals
    #[test]
    fn test_context_signals_serialization_round_trip() {
        // Arrange: Create ContextSignals instance
        let signals = ContextSignals {
            title_keywords: vec!["astro".to_string(), "model".to_string()],
            url_domain: Some("app.datasite.com".to_string()),
            file_path: Some("/Users/test/Astro/model.xlsx".to_string()),
            project_folder: Some("Astro".to_string()),
            calendar_event_id: None,
            attendee_domains: vec![],
            app_category: AppCategory::Excel,
            is_vdr_provider: true,
            timestamp: 0,
            project_id: None,
            organizer_domain: None,
            is_screen_locked: false,
            has_personal_event: false,
            is_internal_training: false,
            is_personal_browsing: false,
            email_direction: None,
            has_external_meeting_attendees: false,
        };

        // Act: Serialize to JSON
        let json = serde_json::to_string(&signals).unwrap();
        let deserialized: ContextSignals = serde_json::from_str(&json).unwrap();

        // Assert: Round-trip preserves all fields
        assert_eq!(deserialized.title_keywords, signals.title_keywords);
        assert_eq!(deserialized.url_domain, signals.url_domain);
        assert_eq!(deserialized.is_vdr_provider, signals.is_vdr_provider);
    }

    /// Test 2: Verify ProjectMatch can be serialized and deserialized
    /// REQUIREMENT: Phase 2 - Add Serialize/Deserialize derives to ProjectMatch
    #[test]
    fn test_project_match_serialization_round_trip() {
        // Arrange: Create ProjectMatch instance
        let project_match = ProjectMatch {
            project_id: Some("USC123".to_string()),
            wbs_code: Some("USC123.1.1".to_string()),
            deal_name: Some("Project Astro".to_string()),
            workstream: Some("modeling".to_string()),
            confidence: 0.85,
            reasons: vec!["keyword:astro".to_string(), "vdr:datasite".to_string()],
        };

        // Act: Serialize to JSON
        let json = serde_json::to_string(&project_match).unwrap();
        let deserialized: ProjectMatch = serde_json::from_str(&json).unwrap();

        // Assert: Round-trip preserves all fields
        assert_eq!(deserialized.project_id, project_match.project_id);
        assert_eq!(deserialized.confidence, project_match.confidence);
        assert_eq!(deserialized.reasons.len(), 2);
    }

    /// Test 3: Verify ContextSignals produces valid JSON structure
    /// REQUIREMENT: Phase 2 - JSON structure for database storage
    #[test]
    fn test_context_signals_to_json_valid() {
        // Arrange: Create ContextSignals
        let signals = ContextSignals {
            title_keywords: vec!["test".to_string()],
            url_domain: None,
            file_path: None,
            project_folder: None,
            calendar_event_id: None,
            attendee_domains: vec![],
            app_category: AppCategory::Excel,
            is_vdr_provider: false,
            timestamp: 0,
            project_id: None,
            organizer_domain: None,
            is_screen_locked: false,
            has_personal_event: false,
            is_internal_training: false,
            is_personal_browsing: false,
            email_direction: None,
            has_external_meeting_attendees: false,
        };

        // Act: Serialize to JSON
        let json = serde_json::to_string(&signals).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Assert: JSON has expected structure
        assert!(parsed.is_object());
        assert!(parsed.get("title_keywords").is_some());
        assert!(parsed.get("app_category").is_some());
    }

    /// Test 4: Verify ProjectMatch produces valid JSON structure
    /// REQUIREMENT: Phase 2 - JSON structure for database storage
    #[test]
    fn test_project_match_to_json_valid() {
        // Arrange: Create ProjectMatch
        let project_match = ProjectMatch {
            project_id: Some("USC123".to_string()),
            wbs_code: None,
            deal_name: Some("Test Project".to_string()),
            workstream: None,
            confidence: 0.75,
            reasons: vec!["test".to_string()],
        };

        // Act: Serialize to JSON
        let json = serde_json::to_string(&project_match).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        // Assert: JSON has expected structure
        assert!(parsed.is_object());
        assert!(parsed.get("project_id").is_some());
        assert!(parsed.get("confidence").is_some());
        assert_eq!(parsed["confidence"].as_f64().unwrap(), 0.75);
    }
}

// ============================================================================
// REFACTOR-004: Evidence Extraction Types
// ============================================================================

/// Evidence package for a single block
///
/// Contains all contextual signals needed for OpenAI classification.
/// This replaces internal ML-like heuristics with pure evidence collection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct BlockEvidence {
    /// Block ID (external identifier sent to OpenAI)
    pub block_id: String,

    /// Start timestamp (Unix epoch seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub start_ts: i64,

    /// End timestamp (Unix epoch seconds)
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub end_ts: i64,

    /// Duration in seconds
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub duration_secs: i64,

    /// Activity breakdown (app names and durations)
    pub activities: Vec<ActivityBreakdownEvidence>,

    /// Extracted signals (keywords, domains, VDR providers, etc.)
    pub signals: EvidenceSignals,
}

/// Activity breakdown for evidence (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct ActivityBreakdownEvidence {
    pub name: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub duration_secs: i64,
    pub percentage: f32,
}

/// Structured signals extracted from snapshots
///
/// These are facts/evidence, not inferences. Used for OpenAI classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct EvidenceSignals {
    /// Unique apps used (e.g., ["Excel", "Chrome", "VSCode"])
    pub apps: Vec<String>,

    /// Window titles (PII-redacted, e.g., "[EMAIL] - Gmail", "Model.xlsx -
    /// Excel")
    pub window_titles: Vec<String>,

    /// Keywords extracted from titles (>3 chars, lowercase)
    pub keywords: Vec<String>,

    /// URL domains (e.g., ["datasite.com", "github.com"])
    pub url_domains: Vec<String>,

    /// File paths (if available)
    pub file_paths: Vec<String>,

    /// Calendar event titles (if available)
    pub calendar_event_titles: Vec<String>,

    /// Attendee email domains (e.g., ["clientfirm.com", "company.com"])
    pub attendee_domains: Vec<String>,

    /// VDR providers detected (e.g., ["datasite", "intralinks"])
    pub vdr_providers: Vec<String>,

    /// Meeting platforms detected (e.g., ["zoom", "google_meet", "teams"])
    pub meeting_platforms: Vec<String>,

    /// True if any calendar events in this block are part of recurring series
    pub has_recurring_meeting: bool,

    /// True if any calendar events in this block have online meeting links
    pub has_online_meeting: bool,
}
