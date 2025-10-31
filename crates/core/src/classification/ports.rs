//! Port interfaces for activity classification

use async_trait::async_trait;
use chrono::NaiveDate;
use pulsearc_domain::types::classification::{ContextSignals, ProjectMatch, ProposedBlock};
use pulsearc_domain::types::sap::WbsElement;
use pulsearc_domain::{ActivitySnapshot, Result, TimeEntry};

pub use crate::tracking::ports::CalendarEventRepository;

/// Trait for classifying activities into time entries
#[async_trait]
pub trait Classifier: Send + Sync {
    /// Classify a set of snapshots into a time entry
    async fn classify(&self, snapshots: Vec<ActivitySnapshot>) -> Result<TimeEntry>;
}

/// Trait for persisting classified time entries
#[async_trait]
pub trait TimeEntryRepository: Send + Sync {
    /// Save a time entry
    async fn save_entry(&self, entry: TimeEntry) -> Result<()>;

    /// Get time entries within a time range
    async fn get_entries(
        &self,
        start: chrono::DateTime<chrono::Utc>,
        end: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<TimeEntry>>;

    /// Update an existing time entry
    async fn update_entry(&self, entry: TimeEntry) -> Result<()>;

    /// Delete a time entry
    async fn delete_entry(&self, id: uuid::Uuid) -> Result<()>;
}

/// Trait for persisting proposed time blocks
#[async_trait]
pub trait BlockRepository: Send + Sync {
    /// Save a proposed block
    async fn save_proposed_block(&self, block: &ProposedBlock) -> Result<()>;

    /// Get proposed blocks for a specific date
    async fn get_proposed_blocks(&self, date: NaiveDate) -> Result<Vec<ProposedBlock>>;
}

/// Trait for matching activity signals to projects
///
/// Analyzes context signals extracted from activity snapshots and matches them
/// to known projects based on keywords, URLs, file paths, and other signals.
#[async_trait]
pub trait ProjectMatcher: Send + Sync {
    /// Match activity signals to a project
    ///
    /// # Arguments
    /// * `signals` - Context signals extracted from activity snapshots
    ///
    /// # Returns
    /// A ProjectMatch if signals match a known project, or None if no match
    /// found
    async fn match_project(&self, signals: &ContextSignals) -> Result<Option<ProjectMatch>>;
}

/// Repository for Work Breakdown Structure (WBS) cache operations
///
/// Provides FTS5 full-text search for project matching and WBS metadata lookup.
/// Used by `ProjectMatcher` for hybrid fast-path (HashMap) + slow-path (FTS5)
/// matching.
///
/// # Architecture
///
/// The WBS cache stores enriched project metadata from SAP ERP, including:
/// - Project definitions and WBS codes (e.g., "USC0063201", "USC0063201.1.1")
/// - Project names and descriptions
/// - FEATURE-029 enriched Salesforce opportunity data (deal_name,
///   target_company, industry, etc.)
///
/// # Search Strategy
///
/// - **Fast-path**: Substring matching against top 20 common projects
///   (pre-loaded HashMap)
/// - **Slow-path**: FTS5 full-text search with typo tolerance and BM25 ranking
/// - **Fallback**: General & Administrative project (USC0000000.1.0)
///
/// # FTS5 Search Details
///
/// The `fts5_search_keyword` method searches across multiple fields using
/// Porter stemming and typo tolerance:
/// - `wbs_code`, `project_def`, `project_name`, `description`
/// - `deal_name`, `target_company_name`, `counterparty`, `industry`
///
/// Results are ranked by BM25 relevance and filtered to active projects
/// (`status = 'REL'`).
pub trait WbsRepository: Send + Sync {
    /// Validate that WBS cache is populated with at least one active project
    ///
    /// # Returns
    /// Count of active ('REL' status) WBS elements in the cache
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_core::WbsRepository;
    /// # fn example(wbs_repo: &dyn WbsRepository) -> pulsearc_domain::Result<()> {
    /// let count = wbs_repo.count_active_wbs()?;
    /// if count == 0 {
    ///     return Err(pulsearc_domain::PulseArcError::Config(
    ///         "WBS cache is empty - run SAP sync first".into(),
    ///     ));
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn count_active_wbs(&self) -> Result<i64>;

    /// Get timestamp of most recent cache update
    ///
    /// Used to warn users if cache is stale (>24 hours old).
    ///
    /// # Returns
    /// Unix timestamp of most recent `cached_at` value, or None if cache is
    /// empty
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_core::WbsRepository;
    /// # fn example(wbs_repo: &dyn WbsRepository) -> pulsearc_domain::Result<()> {
    /// if let Some(last_sync) = wbs_repo.get_last_sync_timestamp()? {
    ///     let age_hours = (chrono::Utc::now().timestamp() - last_sync) / 3600;
    ///     if age_hours > 24 {
    ///         eprintln!("Warning: WBS cache is {} hours old", age_hours);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_last_sync_timestamp(&self) -> Result<Option<i64>>;

    /// Load top N common projects for fast-path HashMap caching
    ///
    /// Returns most recently cached active projects (`status = 'REL'`)
    /// ordered by `cached_at` DESC. Used by `ProjectMatcher` to pre-populate
    /// a HashMap for sub-millisecond lookups.
    ///
    /// # Arguments
    /// * `limit` - Maximum number of projects to load (typically 20)
    ///
    /// # Returns
    /// Vector of WBS elements sorted by recency (most recent first)
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_core::WbsRepository;
    /// # fn example(wbs_repo: &dyn WbsRepository) -> pulsearc_domain::Result<()> {
    /// let top_projects = wbs_repo.load_common_projects(20)?;
    /// for wbs in &top_projects {
    ///     println!(
    ///         "Project: {} - {}",
    ///         wbs.project_def, // e.g., "USC0063201"
    ///         wbs.project_name.as_deref().unwrap_or("Unnamed")
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn load_common_projects(&self, limit: usize) -> Result<Vec<WbsElement>>;

    /// FTS5 full-text search for WBS elements
    ///
    /// Searches across `wbs_code`, `project_def`, `project_name`,
    /// `description`, `deal_name`, `target_company_name`, `counterparty`,
    /// and `industry` fields using Porter stemming and typo tolerance.
    ///
    /// # Arguments
    /// * `keyword` - Search term (will be quoted for FTS5: `"keyword"`)
    /// * `limit` - Maximum results to return (typically 3-5)
    ///
    /// # Returns
    /// WBS elements ranked by BM25 relevance, filtered to `status = 'REL'`
    ///
    /// # Performance
    /// Expected query time: <3ms per search (measured in benchmarks)
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_core::WbsRepository;
    /// # fn example(wbs_repo: &dyn WbsRepository) -> pulsearc_domain::Result<()> {
    /// // Search for projects related to "stellar"
    /// let matches = wbs_repo.fts5_search_keyword("stellar", 5)?;
    /// for wbs in &matches {
    ///     println!(
    ///         "Match: {} ({})",
    ///         wbs.project_name.as_deref().unwrap_or("Unnamed"),
    ///         wbs.deal_name.as_deref().unwrap_or("No deal")
    ///     );
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn fts5_search_keyword(&self, keyword: &str, limit: usize) -> Result<Vec<WbsElement>>;

    /// Get WBS element by project definition (project_def)
    ///
    /// Looks up a project by its SAP project definition code.
    ///
    /// # Arguments
    /// * `project_def` - Project ID (e.g., "USC0063201")
    ///
    /// # Returns
    /// WbsElement if found with `status = 'REL'`, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_core::WbsRepository;
    /// # fn example(wbs_repo: &dyn WbsRepository) -> pulsearc_domain::Result<()> {
    /// if let Some(wbs) = wbs_repo.get_wbs_by_project_def("USC0063201")? {
    ///     println!("Found project: {}", wbs.project_name.unwrap_or_default());
    ///     println!("WBS code: {}", wbs.wbs_code);
    ///     if let Some(deal) = wbs.deal_name {
    ///         println!("Deal: {}", deal);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_wbs_by_project_def(&self, project_def: &str) -> Result<Option<WbsElement>>;

    /// Get WBS element by WBS code
    ///
    /// Looks up a specific work package by its full WBS code.
    ///
    /// # Arguments
    /// * `wbs_code` - Full WBS code (e.g., "USC0063201.1.1")
    ///
    /// # Returns
    /// WbsElement if found with `status = 'REL'`, None otherwise
    ///
    /// # Example
    /// ```no_run
    /// # use pulsearc_core::WbsRepository;
    /// # fn example(wbs_repo: &dyn WbsRepository) -> pulsearc_domain::Result<()> {
    /// if let Some(wbs) = wbs_repo.get_wbs_by_wbs_code("USC0063201.1.1")? {
    ///     println!("Work package: {}", wbs.description.unwrap_or_default());
    ///     println!("Parent project: {}", wbs.project_def);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    fn get_wbs_by_wbs_code(&self, wbs_code: &str) -> Result<Option<WbsElement>>;
}
