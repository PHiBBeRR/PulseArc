//! SAP-related domain types
//!
//! Ported from `legacy/api/src/integrations/sap/models.rs` so that the new
//! classification ports can depend on WBS metadata without reaching back into
//! the legacy crate hierarchy.

use serde::{Deserialize, Serialize};

#[cfg(feature = "ts-gen")]
use ts_rs::TS;

/// Work Breakdown Structure element as fetched from the SAP cache.
///
/// These records are enriched with opportunity metadata and cached locally so
/// the desktop client can perform offline project matching.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct WbsElement {
    pub wbs_code: String,
    pub project_def: String,
    pub project_name: Option<String>,
    pub description: Option<String>,
    /// SAP status (typically `REL`, `CLSD`, or `TECO`)
    pub status: String,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub cached_at: i64,

    // Opportunity enrichment fields
    pub opportunity_id: Option<String>,
    pub deal_name: Option<String>,
    pub target_company_name: Option<String>,
    pub counterparty: Option<String>,
    pub industry: Option<String>,
    pub region: Option<String>,
    pub amount: Option<f64>,
    pub stage_name: Option<String>,
    pub project_code: Option<String>,
}

/// Aggregate counters for the SAP time-entry outbox.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct OutboxStatusSummary {
    pub pending_count: u32,
    pub sent_count: u32,
    pub failed_count: u32,
}

/// Local synchronisation settings for the SAP integration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct SapSyncSettings {
    pub enabled: bool,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub sync_interval_hours: u64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number | null"))]
    pub last_sync_epoch: Option<i64>,
    pub last_sync_status: Option<String>,
}
