//! Signal extraction from activity snapshots
//!
//! Extracts context signals (keywords, URLs, file paths, calendar info)
//! from activity snapshots for project matching.

use std::sync::Arc;

use pulsearc_domain::types::classification::{AppCategory, ContextSignals};
use pulsearc_domain::{ActivitySnapshot, CalendarEventRow};
use serde::Deserialize;

use crate::classification::ports::CalendarEventRepository;

/// Activity context structure (matches JSON in activity_context_json field)
#[derive(Debug, Clone, Deserialize, Default)]
struct ActivityContext {
    pub active_app: ActiveApp,
}

#[derive(Debug, Clone, Deserialize, Default)]
struct ActiveApp {
    #[serde(default)]
    pub app_name: String,
    #[serde(default)]
    pub window_title: String,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub document_name: Option<String>,
}

/// File path information (path, project_folder)
type FilePathInfo = (Option<String>, Option<String>);

#[derive(Debug, Default)]
struct CalendarEventMatch {
    event_id: Option<String>,
    attendee_domains: Vec<String>,
    organizer_domain: Option<String>,
    has_external_attendees: bool,
}

/// Extracts context signals from activity snapshots
pub struct SignalExtractor {
    keyword_patterns: Vec<String>,
    calendar_repo: Option<Arc<dyn CalendarEventRepository>>,
}

impl SignalExtractor {
    /// Create a new signal extractor with default keyword patterns
    pub fn new() -> Self {
        Self {
            keyword_patterns: vec![
                // Project names
                "astro".to_string(),
                "beta".to_string(),
                "delta".to_string(),
                "gamma".to_string(),
                "luna".to_string(),
                "thunderbolt".to_string(),
                "phoenix".to_string(),
                "eclipse".to_string(),
                "summit".to_string(),
                "horizon".to_string(),
                "odyssey".to_string(),
                // M&A tax keywords
                "section 368".to_string(),
                "ppa".to_string(),
                "382".to_string(),
                "qofe".to_string(),
                "spa".to_string(),
                "loi".to_string(),
                "ioi".to_string(),
                "diligence".to_string(),
                "closing".to_string(),
                "merger".to_string(),
                "acquisition".to_string(),
                // Workstream indicators
                "modeling".to_string(),
                "analysis".to_string(),
                "research".to_string(),
                "memo".to_string(),
                "review".to_string(),
            ],
            calendar_repo: None,
        }
    }

    /// Create a signal extractor with calendar repository for event integration
    pub fn with_calendar(calendar_repo: Arc<dyn CalendarEventRepository>) -> Self {
        Self {
            keyword_patterns: vec![
                // Project names
                "astro".to_string(),
                "beta".to_string(),
                "delta".to_string(),
                "gamma".to_string(),
                "luna".to_string(),
                "thunderbolt".to_string(),
                "phoenix".to_string(),
                "eclipse".to_string(),
                "summit".to_string(),
                "horizon".to_string(),
                "odyssey".to_string(),
                // M&A tax keywords
                "section 368".to_string(),
                "ppa".to_string(),
                "382".to_string(),
                "qofe".to_string(),
                "spa".to_string(),
                "loi".to_string(),
                "ioi".to_string(),
                "diligence".to_string(),
                "closing".to_string(),
                "merger".to_string(),
                "acquisition".to_string(),
                // Workstream indicators
                "modeling".to_string(),
                "analysis".to_string(),
                "research".to_string(),
                "memo".to_string(),
                "review".to_string(),
            ],
            calendar_repo: Some(calendar_repo),
        }
    }

    /// Extract and merge signals from multiple snapshots (for ProposedBlocks
    /// with multiple activities)
    pub async fn extract_and_merge(&self, snapshots: &[ActivitySnapshot]) -> ContextSignals {
        if snapshots.is_empty() {
            return ContextSignals::default();
        }

        // Extract signals from all snapshots
        let mut all_signals = Vec::new();
        for snap in snapshots {
            all_signals.push(self.extract(snap).await);
        }

        // Merge signals (deduplication, strongest signals win)
        self.merge_signals(&all_signals)
    }

    /// Merge multiple ContextSignals into one (for multi-activity blocks)
    pub fn merge_signals(&self, signals: &[ContextSignals]) -> ContextSignals {
        if signals.is_empty() {
            return ContextSignals::default();
        }

        if signals.len() == 1 {
            return signals[0].clone();
        }

        // Merge title keywords (deduplicate)
        let mut all_keywords: Vec<String> =
            signals.iter().flat_map(|s| s.title_keywords.clone()).collect();
        all_keywords.sort();
        all_keywords.dedup();

        // Merge attendee domains (deduplicate)
        let mut all_domains: Vec<String> =
            signals.iter().flat_map(|s| s.attendee_domains.clone()).collect();
        all_domains.sort();
        all_domains.dedup();

        // Take first non-None value for optional fields
        let url_domain = signals.iter().find_map(|s| s.url_domain.clone());
        let file_path = signals.iter().find_map(|s| s.file_path.clone());
        let project_folder = signals.iter().find_map(|s| s.project_folder.clone());
        let calendar_event_id = signals.iter().find_map(|s| s.calendar_event_id.clone());
        let project_id = signals.iter().find_map(|s| s.project_id.clone());
        let organizer_domain = signals.iter().find_map(|s| s.organizer_domain.clone());

        // Check if ANY snapshot had VDR, Tier 4 signals
        let is_vdr_provider = signals.iter().any(|s| s.is_vdr_provider);
        let is_screen_locked = signals.iter().any(|s| s.is_screen_locked);
        let has_personal_event = signals.iter().any(|s| s.has_personal_event);
        let is_internal_training = signals.iter().any(|s| s.is_internal_training);
        let is_personal_browsing = signals.iter().any(|s| s.is_personal_browsing);
        let has_external_meeting_attendees =
            signals.iter().any(|s| s.has_external_meeting_attendees);

        // Use the most specific app category
        let app_category = signals
            .iter()
            .map(|s| &s.app_category)
            .max_by_key(|cat| Self::app_category_priority(cat))
            .cloned()
            .unwrap_or(AppCategory::Other);

        // Use the latest timestamp
        let timestamp = signals.iter().map(|s| s.timestamp).max().unwrap_or(0);

        ContextSignals {
            title_keywords: all_keywords,
            url_domain,
            file_path,
            project_folder,
            calendar_event_id,
            attendee_domains: all_domains,
            app_category,
            is_vdr_provider,
            timestamp,
            project_id,
            organizer_domain,
            is_screen_locked,
            has_personal_event,
            is_internal_training,
            is_personal_browsing,
            email_direction: None,
            has_external_meeting_attendees,
        }
    }

    /// Extract all context signals from a snapshot
    pub async fn extract(&self, snapshot: &ActivitySnapshot) -> ContextSignals {
        // Parse activity context JSON
        let context: ActivityContext =
            serde_json::from_str(&snapshot.activity_context_json).unwrap_or_default();

        // Extract title keywords
        let title_keywords = self.extract_keywords(&context.active_app.window_title);

        // Analyze URL if browser
        let url_domain = context.active_app.url.as_ref().and_then(|url| self.extract_domain(url));

        // Extract file path if available
        let (file_path, project_folder) = self.extract_file_info(&context.active_app.document_name);

        // Check if VDR provider
        let is_vdr = url_domain.as_ref().is_some_and(|d| {
            d.contains("datasite")
                || d.contains("intralinks")
                || d.contains("firmex")
                || (d.contains("box") && d.contains("enterprise"))
        });

        // Categorize app
        let app_category = self.categorize_app(&context.active_app.app_name);

        // Query calendar events by timestamp (if repository available)
        let CalendarEventMatch {
            event_id: calendar_event_id,
            attendee_domains,
            organizer_domain,
            has_external_attendees,
        } = self.query_calendar_event(snapshot.timestamp).await.unwrap_or_default();

        // Detect Tier 4 override signals
        let is_screen_locked = snapshot.is_idle && snapshot.idle_duration_secs.unwrap_or(0) > 300;
        let has_personal_event = Self::is_personal_title(&context.active_app.window_title);
        let is_internal_training = context
            .active_app
            .url
            .as_ref()
            .map(|u| u.contains("lms.deloitte.com") || u.contains("training"))
            .unwrap_or(false);
        let is_personal_browsing = Self::is_personal_browsing_url(&context.active_app.url);

        ContextSignals {
            title_keywords,
            url_domain,
            file_path,
            project_folder,
            calendar_event_id,
            attendee_domains,
            app_category,
            is_vdr_provider: is_vdr,
            timestamp: snapshot.timestamp,
            project_id: None,
            organizer_domain,
            is_screen_locked,
            has_personal_event,
            is_internal_training,
            is_personal_browsing,
            email_direction: None,
            has_external_meeting_attendees: has_external_attendees,
        }
    }

    /// Extract meaningful keywords from title
    fn extract_keywords(&self, title: &str) -> Vec<String> {
        let title_lower = title.to_lowercase();

        self.keyword_patterns
            .iter()
            .filter(|pattern| title_lower.contains(pattern.as_str()))
            .cloned()
            .collect()
    }

    /// Extract domain from URL
    fn extract_domain(&self, url: &str) -> Option<String> {
        url::Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h.to_string()))
    }

    /// Extract file path and project folder
    fn extract_file_info(&self, document_name: &Option<String>) -> FilePathInfo {
        if let Some(path) = document_name {
            // Extract project folder from path
            // ~/Documents/Astro/model.xlsx → "Astro"
            if let Some(folder_name) = path.split('/').rev().nth(1) {
                return (Some(path.clone()), Some(folder_name.to_string()));
            }
            return (Some(path.clone()), None);
        }
        (None, None)
    }

    /// Categorize app into type
    fn categorize_app(&self, app_name: &str) -> AppCategory {
        let app_lower = app_name.to_lowercase();

        if app_lower.contains("excel") {
            AppCategory::Excel
        } else if app_lower.contains("word") {
            AppCategory::Word
        } else if app_lower.contains("powerpoint") {
            AppCategory::PowerPoint
        } else if app_lower.contains("chrome")
            || app_lower.contains("safari")
            || app_lower.contains("firefox")
        {
            AppCategory::Browser
        } else if app_lower.contains("outlook") || app_lower.contains("mail") {
            AppCategory::Email
        } else if app_lower.contains("zoom")
            || app_lower.contains("teams")
            || app_lower.contains("meet")
        {
            AppCategory::Meeting
        } else if app_lower.contains("terminal") || app_lower.contains("iterm") {
            AppCategory::Terminal
        } else if app_lower.contains("cursor")
            || app_lower.contains("code")
            || app_lower.contains("xcode")
        {
            AppCategory::IDE
        } else {
            AppCategory::Other
        }
    }

    /// Query calendar event by timestamp
    ///
    /// Returns calendar metadata (id, attendee domains, organizer domain,
    /// external attendee flag) if found.
    async fn query_calendar_event(&self, timestamp: i64) -> Option<CalendarEventMatch> {
        let calendar_repo = self.calendar_repo.as_ref()?;

        // Query events within ±15 minutes of snapshot timestamp
        let time_window = 900; // 15 minutes in seconds

        match calendar_repo.find_event_by_timestamp(timestamp, time_window).await {
            Ok(Some(event)) => {
                let attendee_domains = Self::extract_attendee_domains(&event);
                let organizer_domain = event.organizer_domain.clone();
                let has_external_attendees = event.has_external_attendees.unwrap_or(false);

                Some(CalendarEventMatch {
                    event_id: Some(event.id),
                    attendee_domains,
                    organizer_domain,
                    has_external_attendees,
                })
            }
            Ok(None) | Err(_) => None,
        }
    }

    /// Check if title indicates personal event
    fn is_personal_title(title: &str) -> bool {
        let title_lower = title.to_lowercase();
        title_lower.contains("personal")
            || title_lower.contains("lunch")
            || title_lower.contains("break")
    }

    fn extract_attendee_domains(event: &CalendarEventRow) -> Vec<String> {
        Self::extract_attendee_domain(event).into_iter().collect::<Vec<_>>()
    }

    fn extract_attendee_domain(event: &CalendarEventRow) -> Option<String> {
        event
            .organizer_domain
            .as_deref()
            .and_then(Self::normalize_domain)
            .or_else(|| event.organizer_email.as_deref().and_then(Self::extract_domain_from_email))
    }

    fn normalize_domain(raw: &str) -> Option<String> {
        let trimmed = raw.trim().trim_matches(|c| c == '<' || c == '>');
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_ascii_lowercase())
    }

    fn extract_domain_from_email(email: &str) -> Option<String> {
        let at_pos = email.rfind('@')?;
        let domain = email.get(at_pos + 1..)?.trim();
        Self::normalize_domain(domain)
    }

    /// Check if URL is personal browsing
    fn is_personal_browsing_url(url: &Option<String>) -> bool {
        url.as_ref()
            .map(|u| {
                u.contains("reddit.com")
                    || u.contains("youtube.com")
                    || u.contains("facebook.com")
                    || u.contains("twitter.com")
                    || u.contains("instagram.com")
                    || u.contains("spotify.com")
            })
            .unwrap_or(false)
    }

    /// Get priority for app category (for merge logic)
    fn app_category_priority(cat: &AppCategory) -> i32 {
        match cat {
            AppCategory::Excel => 9,
            AppCategory::Word => 8,
            AppCategory::PowerPoint => 7,
            AppCategory::Meeting => 6,
            AppCategory::Email => 5,
            AppCategory::IDE => 4,
            AppCategory::Browser => 3,
            AppCategory::Terminal => 2,
            AppCategory::Other => 1,
        }
    }
}

impl Default for SignalExtractor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_extraction() {
        let extractor = SignalExtractor::new();
        let title = "Working on Project Astro PPA modeling";

        let keywords = extractor.extract_keywords(title);

        assert!(keywords.contains(&"astro".to_string()));
        assert!(keywords.contains(&"ppa".to_string()));
        assert!(keywords.contains(&"modeling".to_string()));
        assert_eq!(keywords.len(), 3);
    }

    #[tokio::test]
    async fn test_signal_extraction_performance() {
        let extractor = SignalExtractor::new();
        let snapshot = create_test_snapshot_with_context(
            r#"{"active_app": {"app_name": "Microsoft Excel", "window_title": "Project Astro PPA Model - Section 368 Analysis", "document_name": "~/Documents/Astro/ppa-model.xlsx", "url": "https://app.datasite.com/project/123"}}"#,
        );

        let iterations = 100;
        let start = std::time::Instant::now();
        for _ in 0..iterations {
            let _ = extractor.extract(&snapshot).await;
        }
        let duration = start.elapsed();
        let avg_per_snapshot = duration.as_micros() / iterations;

        assert!(
            avg_per_snapshot < 10_000,
            "Signal extraction should be <10ms per snapshot (got {} µs)",
            avg_per_snapshot
        );
    }

    #[test]
    fn test_vdr_detection() {
        let extractor = SignalExtractor::new();

        let datasite_domain = extractor.extract_domain("https://app.datasite.com/project/123");
        assert_eq!(datasite_domain, Some("app.datasite.com".to_string()));

        let intralinks_domain =
            extractor.extract_domain("https://services.intralinks.com/vault/456");
        assert_eq!(intralinks_domain, Some("services.intralinks.com".to_string()));

        assert!(datasite_domain.unwrap().contains("datasite"));
    }

    #[test]
    fn test_app_categorization() {
        let extractor = SignalExtractor::new();

        assert_eq!(extractor.categorize_app("Microsoft Excel"), AppCategory::Excel);
        assert_eq!(extractor.categorize_app("Google Chrome"), AppCategory::Browser);
        assert_eq!(extractor.categorize_app("Zoom"), AppCategory::Meeting);
        assert_eq!(extractor.categorize_app("Microsoft Word"), AppCategory::Word);
        assert_eq!(extractor.categorize_app("Cursor"), AppCategory::IDE);
    }

    #[test]
    fn test_url_domain_extraction() {
        let extractor = SignalExtractor::new();

        assert_eq!(
            extractor.extract_domain("https://github.com/user/repo"),
            Some("github.com".to_string())
        );
        assert_eq!(
            extractor.extract_domain("https://app.datasite.com/project/123?param=value"),
            Some("app.datasite.com".to_string())
        );
        assert_eq!(extractor.extract_domain("invalid-url"), None);
    }

    #[test]
    fn test_file_path_extraction() {
        let extractor = SignalExtractor::new();

        let (path1, folder1) =
            extractor.extract_file_info(&Some("~/Documents/Astro/model.xlsx".to_string()));
        let (path2, folder2) =
            extractor.extract_file_info(&Some("/Users/user/Projects/Beta/memo.docx".to_string()));
        let (path3, folder3) = extractor.extract_file_info(&None);

        assert_eq!(path1, Some("~/Documents/Astro/model.xlsx".to_string()));
        assert_eq!(folder1, Some("Astro".to_string()));

        assert_eq!(path2, Some("/Users/user/Projects/Beta/memo.docx".to_string()));
        assert_eq!(folder2, Some("Beta".to_string()));

        assert_eq!(path3, None);
        assert_eq!(folder3, None);
    }

    #[tokio::test]
    #[ignore = "TDD: Implement calendar context extraction"]
    async fn test_calendar_context_extraction() {
        let extractor = SignalExtractor::new();
        let snapshot = create_test_snapshot_with_timestamp(1729468800);

        let signals = extractor.extract(&snapshot).await;

        assert_eq!(signals.calendar_event_id, None);
        assert!(signals.attendee_domains.is_empty());
    }

    #[tokio::test]
    async fn test_empty_context_handling() {
        let extractor = SignalExtractor::new();
        let snapshot_empty = create_test_snapshot_with_context("{}");
        let snapshot_invalid = create_test_snapshot_with_context("invalid json");

        let signals_empty = extractor.extract(&snapshot_empty).await;
        let signals_invalid = extractor.extract(&snapshot_invalid).await;

        assert!(signals_empty.title_keywords.is_empty());
        assert_eq!(signals_empty.url_domain, None);
        assert_eq!(signals_empty.file_path, None);

        assert!(signals_invalid.title_keywords.is_empty());
        assert_eq!(signals_invalid.app_category, AppCategory::Other);
    }

    #[test]
    fn test_extract_attendee_domain_uses_explicit_domain() {
        let event = calendar_event_fixture(Some("ClientCorp.com"), Some("host@fallback.com"));
        let domain = SignalExtractor::extract_attendee_domain(&event);

        assert_eq!(domain.as_deref(), Some("clientcorp.com"));
    }

    #[test]
    fn test_extract_attendee_domain_falls_back_to_email() {
        let event = calendar_event_fixture(None, Some("Presenter.Name@ClientCorp.com"));
        let domain = SignalExtractor::extract_attendee_domain(&event);

        assert_eq!(domain.as_deref(), Some("clientcorp.com"));
    }

    #[test]
    fn test_extract_attendee_domain_rejects_invalid_email() {
        let event = calendar_event_fixture(None, Some("invalid-email"));
        let domain = SignalExtractor::extract_attendee_domain(&event);

        assert!(domain.is_none());
    }

    fn create_test_snapshot_with_timestamp(timestamp: i64) -> ActivitySnapshot {
        ActivitySnapshot {
            id: "test-snap-1".to_string(),
            timestamp,
            detected_activity: "Working".to_string(),
            work_type: Some("development".to_string()),
            primary_app: "Microsoft Excel".to_string(),
            activity_category: Some("work".to_string()),
            activity_context_json: r#"{"active_app": {"app_name": "Microsoft Excel", "window_title": "Project Astro PPA Model"}}"#.to_string(),
            processed: false,
            batch_id: None,
            created_at: timestamp,
            processed_at: None,
            is_idle: false,
            idle_duration_secs: None,
        }
    }

    fn create_test_snapshot_with_context(context_json: &str) -> ActivitySnapshot {
        ActivitySnapshot {
            id: "test-snap-2".to_string(),
            timestamp: 1729468800,
            detected_activity: "Working".to_string(),
            work_type: Some("development".to_string()),
            primary_app: "Unknown".to_string(),
            activity_category: Some("other".to_string()),
            activity_context_json: context_json.to_string(),
            processed: false,
            batch_id: None,
            created_at: 1729468800,
            processed_at: None,
            is_idle: false,
            idle_duration_secs: None,
        }
    }

    fn calendar_event_fixture(
        organizer_domain: Option<&str>,
        organizer_email: Option<&str>,
    ) -> CalendarEventRow {
        CalendarEventRow {
            id: "event-1".to_string(),
            google_event_id: "google-event-1".to_string(),
            user_email: "user@pulsearc.com".to_string(),
            summary: "Weekly Sync".to_string(),
            description: None,
            start_ts: 1_700_000_000,
            end_ts: 1_700_003_600,
            is_all_day: false,
            recurring_event_id: None,
            parsed_project: None,
            parsed_workstream: None,
            parsed_task: None,
            confidence_score: None,
            meeting_platform: None,
            is_recurring_series: false,
            is_online_meeting: false,
            has_external_attendees: None,
            organizer_email: organizer_email.map(|s| s.to_string()),
            organizer_domain: organizer_domain.map(|s| s.to_string()),
            meeting_id: None,
            attendee_count: None,
            external_attendee_count: None,
            created_at: 1_700_000_000,
        }
    }
}
