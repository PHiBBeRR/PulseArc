//! SAP integration port interfaces (feature: sap)
//!
//! This module is only compiled when the `sap` feature is enabled.

use async_trait::async_trait;
use pulsearc_domain::Result;

/// SAP time entry identifier
pub type SapEntryId = String;

/// Time entry for SAP forwarding
pub struct TimeEntry {
    pub wbs_code: String,
    pub description: String,
    pub duration_hours: f32,
    pub date: String,
}

/// Trait for SAP client operations
#[async_trait]
pub trait SapClient: Send + Sync {
    /// Forward a time entry to SAP
    async fn forward_entry(&self, entry: &TimeEntry) -> Result<SapEntryId>;

    /// Validate a WBS code
    async fn validate_wbs(&self, wbs_code: &str) -> Result<bool>;
}
