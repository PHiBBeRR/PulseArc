//! Evidence Extractor - Extracts structured signals from snapshots for OpenAI classification
//!
//! REFACTOR-004: This module replaces internal ML-like heuristics with pure evidence collection.
//! The app collects all relevant signals (apps, keywords, domains, VDR providers, calendar
//! events), and OpenAI performs all classification (billable/G&A, project matching, workstream
//! inference).

use chrono::{TimeZone, Utc};
use pulsearc_domain::types::classification::{
    ActivityBreakdownEvidence, BlockEvidence, EvidenceSignals, ProposedBlock,
};
use pulsearc_domain::{ActivityContext, ActivitySnapshot, Result};
use std::collections::HashSet;
use std::sync::Arc;

use crate::tracking::ports::{CalendarEventRepository, SnapshotRepository};

/// EvidenceExtractor - Collects structured signals from snapshots
///
/// REFACTOR-004: No inference, just fact extraction
pub struct EvidenceExtractor {
    snapshot_repo: Arc<dyn SnapshotRepository>,
    calendar_repo: Option<Arc<dyn CalendarEventRepository>>,
}

impl EvidenceExtractor {
    /// Create new evidence extractor with required snapshot repository
    pub fn new(snapshot_repo: Arc<dyn SnapshotRepository>) -> Self {
        Self { snapshot_repo, calendar_repo: None }
    }

    /// Create evidence extractor with calendar integration
    pub fn with_calendar(
        snapshot_repo: Arc<dyn SnapshotRepository>,
        calendar_repo: Arc<dyn CalendarEventRepository>,
    ) -> Self {
        Self { snapshot_repo, calendar_repo: Some(calendar_repo) }
    }

    /// Extract evidence for a block
    ///
    /// This is the main entry point. Given a ProposedBlock, it:
    /// 1. Fetches all snapshots referenced by the block (via snapshot_ids)
    /// 2. Extracts all signals from those snapshots
    /// 3. Packages as structured BlockEvidence for OpenAI
    ///
    /// # Arguments
    /// * `block` - ProposedBlock to extract evidence for
    ///
    /// # Returns
    /// BlockEvidence with all extracted signals
    pub async fn extract_evidence(&self, block: &ProposedBlock) -> Result<BlockEvidence> {
        // Fetch snapshots for this block
        let snapshots = self.fetch_snapshots_for_block(block).await?;

        // Extract signals from snapshots
        let signals = self.extract_signals_from_snapshots(&snapshots, block).await?;

        // Convert block activities to evidence format
        let activities: Vec<ActivityBreakdownEvidence> = block
            .activities
            .iter()
            .map(|a| ActivityBreakdownEvidence {
                name: a.name.clone(),
                duration_secs: a.duration_secs,
                percentage: a.percentage,
            })
            .collect();

        Ok(BlockEvidence {
            block_id: block.id.clone(),
            start_ts: block.start_ts,
            end_ts: block.end_ts,
            duration_secs: block.duration_secs,
            activities,
            signals,
        })
    }

    /// Fetch snapshots for a block
    ///
    /// Queries the repository for all snapshots in the block's time range.
    /// Filters to only snapshots whose IDs are in the block's snapshot_ids list.
    ///
    /// # Arguments
    /// * `block` - ProposedBlock containing snapshot_ids and time range
    ///
    /// # Returns
    /// Vec of ActivitySnapshots
    async fn fetch_snapshots_for_block(
        &self,
        block: &ProposedBlock,
    ) -> Result<Vec<ActivitySnapshot>> {
        // Convert i64 timestamps to DateTime<Utc>
        let start_time = Utc.timestamp_opt(block.start_ts, 0).single().ok_or_else(|| {
            pulsearc_domain::PulseArcError::Config(format!(
                "Invalid start timestamp: {}",
                block.start_ts
            ))
        })?;
        let end_time = Utc.timestamp_opt(block.end_ts, 0).single().ok_or_else(|| {
            pulsearc_domain::PulseArcError::Config(format!(
                "Invalid end timestamp: {}",
                block.end_ts
            ))
        })?;

        // Query snapshots by time range (synchronous call)
        let all_snapshots = self
            .snapshot_repo
            .find_snapshots_by_time_range(start_time, end_time)
            .map_err(|e| pulsearc_domain::PulseArcError::Database(e.to_string()))?;

        // Filter to only snapshots in the block's snapshot_ids
        let snapshot_id_set: HashSet<&String> = block.snapshot_ids.iter().collect();
        let filtered: Vec<ActivitySnapshot> =
            all_snapshots.into_iter().filter(|snap| snapshot_id_set.contains(&snap.id)).collect();

        if filtered.is_empty() {
            return Err(pulsearc_domain::PulseArcError::NotFound(format!(
                "No snapshots found for block {}",
                block.id
            )));
        }

        Ok(filtered)
    }

    /// Extract signals from snapshots
    ///
    /// Collects all relevant signals (apps, titles, keywords, domains, VDR providers, etc.)
    /// from the given snapshots. Uses HashSet for deduplication.
    ///
    /// FEATURE-029 Phase 4: Now also queries calendar events for meeting metadata.
    ///
    /// # Arguments
    /// * `snapshots` - Snapshots to extract signals from
    /// * `block` - ProposedBlock to get time range for calendar query
    ///
    /// # Returns
    /// EvidenceSignals with deduplicated signals
    async fn extract_signals_from_snapshots(
        &self,
        snapshots: &[ActivitySnapshot],
        block: &ProposedBlock,
    ) -> Result<EvidenceSignals> {
        let mut apps: HashSet<String> = HashSet::new();
        let mut window_titles: HashSet<String> = HashSet::new();
        let mut keywords: HashSet<String> = HashSet::new();
        let mut url_domains: HashSet<String> = HashSet::new();
        let mut file_paths: HashSet<String> = HashSet::new();
        let mut calendar_event_titles: HashSet<String> = HashSet::new();
        let attendee_domains: HashSet<String> = HashSet::new();
        let mut vdr_providers: HashSet<String> = HashSet::new();

        // FEATURE-029 Phase 4: Meeting metadata
        let mut meeting_platforms: HashSet<String> = HashSet::new();
        let mut has_recurring_meeting = false;
        let mut has_online_meeting = false;

        for snapshot in snapshots {
            // Add primary app
            apps.insert(snapshot.primary_app.clone());

            // Parse activity context
            let context: ActivityContext = serde_json::from_str(&snapshot.activity_context_json)
                .map_err(|e| {
                    pulsearc_domain::PulseArcError::Database(format!(
                        "Failed to parse activity_context_json for snapshot {}: {}",
                        snapshot.id, e
                    ))
                })?;

            // Add window title
            let window_title = &context.active_app.window_title;
            if !window_title.is_empty() {
                window_titles.insert(window_title.clone());

                // Extract keywords from title
                let title_keywords = Self::extract_keywords(window_title);
                keywords.extend(title_keywords);
            }

            // Add URL domain (if available)
            if let Some(url_host) = &context.active_app.url_host {
                if !url_host.is_empty() {
                    url_domains.insert(url_host.clone());

                    // Check if VDR provider
                    if Self::is_vdr_domain(url_host) {
                        if let Some(provider) = Self::get_vdr_provider_name(url_host) {
                            vdr_providers.insert(provider);
                        }
                    }
                }
            }

            // Add file path (if available)
            if let Some(file_path) = &context.active_app.file_path {
                if !file_path.is_empty() {
                    file_paths.insert(file_path.clone());
                }
            }
        }

        // FEATURE-029 Phase 4: Extract calendar event metadata
        if let Some(calendar_repo) = &self.calendar_repo {
            // Query all calendar events that overlap with this block
            // We use a simple approach: query events where start < block.end and end > block.start
            // This is a simplification - in production we'd want a more sophisticated query

            // For now, we'll iterate through the time range and query events
            // In a real implementation, we'd have a single query method on the repository
            let time_window = 900; // 15 minutes
            for ts in (block.start_ts..=block.end_ts).step_by(time_window as usize) {
                if let Ok(Some(event)) =
                    calendar_repo.find_event_by_timestamp(ts, time_window).await
                {
                    // Add event title
                    calendar_event_titles.insert(event.summary);

                    // Extract meeting platform (if available)
                    if let Some(platform) = event.meeting_platform {
                        meeting_platforms.insert(platform);
                    }

                    // Check for recurring meetings
                    if event.is_recurring_series {
                        has_recurring_meeting = true;
                    }

                    // Check for online meetings
                    if event.is_online_meeting {
                        has_online_meeting = true;
                    }
                }
            }
        }

        Ok(EvidenceSignals {
            apps: apps.into_iter().collect(),
            window_titles: window_titles.into_iter().collect(),
            keywords: keywords.into_iter().collect(),
            url_domains: url_domains.into_iter().collect(),
            file_paths: file_paths.into_iter().collect(),
            calendar_event_titles: calendar_event_titles.into_iter().collect(),
            attendee_domains: attendee_domains.into_iter().collect(),
            vdr_providers: vdr_providers.into_iter().collect(),
            meeting_platforms: meeting_platforms.into_iter().collect(),
            has_recurring_meeting,
            has_online_meeting,
        })
    }

    /// Extract keywords from text (simple word splitting, >3 chars)
    ///
    /// # Arguments
    /// * `text` - Text to extract keywords from
    ///
    /// # Returns
    /// Vec of lowercase keywords (>3 chars)
    fn extract_keywords(text: &str) -> Vec<String> {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|word| word.len() > 3)
            .map(|word| word.to_lowercase())
            .collect()
    }

    /// Check if domain is a VDR provider
    ///
    /// # Arguments
    /// * `domain` - Domain to check (e.g., "datasite.com")
    ///
    /// # Returns
    /// true if domain is a known VDR provider
    fn is_vdr_domain(domain: &str) -> bool {
        let vdr_domains = [
            "datasite.com",
            "intralinks.com",
            "firmex.com",
            "box.com",
            "merrill.com", // Merrill DataSite
            "dfinsolutions.com",
            "ansarada.com",
        ];

        vdr_domains.iter().any(|vdr| domain.contains(vdr) || vdr.contains(domain))
    }

    /// Get VDR provider name from domain
    ///
    /// # Arguments
    /// * `domain` - Domain to check (e.g., "datasite.com")
    ///
    /// # Returns
    /// VDR provider name (e.g., "datasite")
    fn get_vdr_provider_name(domain: &str) -> Option<String> {
        if domain.contains("datasite") || domain.contains("merrill") {
            Some("datasite".to_string())
        } else if domain.contains("intralinks") {
            Some("intralinks".to_string())
        } else if domain.contains("firmex") {
            Some("firmex".to_string())
        } else if domain.contains("box.com") {
            Some("box".to_string())
        } else if domain.contains("dfinsolutions") {
            Some("dfin".to_string())
        } else if domain.contains("ansarada") {
            Some("ansarada".to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_keywords() {
        let text = "Project Astro Model.xlsx - Excel 2024";
        let keywords = EvidenceExtractor::extract_keywords(text);

        assert!(keywords.contains(&"project".to_string()));
        assert!(keywords.contains(&"astro".to_string()));
        assert!(keywords.contains(&"model".to_string()));
        assert!(keywords.contains(&"xlsx".to_string()));
        assert!(keywords.contains(&"excel".to_string()));
        assert!(keywords.contains(&"2024".to_string()));
    }

    #[test]
    fn test_is_vdr_domain() {
        assert!(EvidenceExtractor::is_vdr_domain("datasite.com"));
        assert!(EvidenceExtractor::is_vdr_domain("app.datasite.com"));
        assert!(EvidenceExtractor::is_vdr_domain("intralinks.com"));
        assert!(EvidenceExtractor::is_vdr_domain("firmex.com"));
        assert!(EvidenceExtractor::is_vdr_domain("box.com"));
        assert!(!EvidenceExtractor::is_vdr_domain("google.com"));
        assert!(!EvidenceExtractor::is_vdr_domain("github.com"));
    }

    #[test]
    fn test_get_vdr_provider_name() {
        assert_eq!(
            EvidenceExtractor::get_vdr_provider_name("datasite.com"),
            Some("datasite".to_string())
        );
        assert_eq!(
            EvidenceExtractor::get_vdr_provider_name("intralinks.com"),
            Some("intralinks".to_string())
        );
        assert_eq!(
            EvidenceExtractor::get_vdr_provider_name("firmex.com"),
            Some("firmex".to_string())
        );
        assert_eq!(EvidenceExtractor::get_vdr_provider_name("box.com"), Some("box".to_string()));
        assert_eq!(EvidenceExtractor::get_vdr_provider_name("google.com"), None);
    }

    #[test]
    fn test_extract_keywords_filters_short_words() {
        let text = "a an the and or but";
        let keywords = EvidenceExtractor::extract_keywords(text);

        // All words are ≤3 chars, should be empty
        assert!(keywords.is_empty());
    }

    #[test]
    fn test_extract_keywords_lowercase() {
        let text = "PROJECT Astro MODEL";
        let keywords = EvidenceExtractor::extract_keywords(text);

        assert!(keywords.contains(&"project".to_string()));
        assert!(keywords.contains(&"astro".to_string()));
        assert!(keywords.contains(&"model".to_string()));
    }
}
