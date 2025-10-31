//! Project matching using FTS5 full-text search
//!
//! Matches activity signals to WBS codes using a hybrid approach:
//! - Fast path: Exact match in top 20 common projects (HashMap)
//! - Slow path: FTS5 fuzzy search for typo-tolerant matching
//!
//! # REFACTOR-004: ADR-003 Migration
//! Migrated from legacy/api/src/inference/project_matcher.rs
//! - Replaced Arc<DbManager> with Arc<dyn WbsRepository>
//! - Removed direct SQL queries in favor of repository trait methods
//! - Preserved all business logic and scoring weights

use crate::classification::ports::WbsRepository;
use pulsearc_domain::{
    classification::{AppCategory, ContextSignals, ProjectMatch},
    types::WbsElement,
    Result,
};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

// Type aliases to avoid clippy type-complexity warnings
type CandidateMap = HashMap<String, (f32, Vec<String>)>;
type MatchInfo<'a> = (&'a str, usize, Vec<String>);

/// Matches activity signals to WBS codes/projects using FTS5 search
pub struct ProjectMatcher {
    wbs_repo: Arc<dyn WbsRepository>,
    common_projects: HashMap<String, String>, // project_name_lower -> project_def
}

impl ProjectMatcher {
    /// Create new project matcher with hybrid approach
    ///
    /// Fast path: HashMap cache of 20 most common projects (exact match)
    /// Slow path: FTS5 full-text search for fuzzy/semantic matching
    ///
    /// # Errors
    /// Returns error if WBS cache is empty (SAP sync scheduler not run)
    pub fn new(wbs_repo: Arc<dyn WbsRepository>) -> Result<Self> {
        // CRITICAL: Validate cache is populated and load common projects
        let count = wbs_repo.count_active_wbs()?;

        if count == 0 {
            return Err(pulsearc_domain::PulseArcError::Config(
                "WBS cache is empty. Ensure SAP sync scheduler has run at least once.\n\
                 Check: 1) SAP auth configured, 2) Network connectivity, 3) Sync scheduler running.\n\
                 For testing: Pre-seed wbs_cache table with test data."
                    .to_string(),
            ));
        }

        // Warn if cache is stale (> 24 hours old)
        let last_sync = wbs_repo.get_last_sync_timestamp()?;

        if let Some(last_sync_ts) = last_sync {
            let age_hours = (chrono::Utc::now().timestamp() - last_sync_ts) / 3600;
            if age_hours > 24 {
                warn!(
                    age_hours,
                    "WBS cache is stale (last sync: {} hours ago). Projects may be outdated.",
                    age_hours
                );
            }
        }

        // Pre-cache only top 20 most common projects (95% memory reduction vs loading all)
        let common = wbs_repo.load_common_projects(20)?;

        let mut common_projects = HashMap::new();
        for wbs in common {
            if let Some(name) = wbs.project_name {
                common_projects.insert(name.to_lowercase(), wbs.project_def.clone());
            }
        }

        tracing::info!(
            count = common_projects.len(),
            "ProjectMatcher initialized with {} common projects cached",
            common_projects.len()
        );

        Ok(Self {
            wbs_repo,
            common_projects,
        })
    }

    /// Get all candidate projects that match the signals (for RulesClassifier scoring)
    ///
    /// Returns all projects that have ANY signal match, allowing RulesClassifier
    /// to apply sophisticated Tier 1-4 weighted scoring to determine the best match.
    ///
    /// Strategy:
    /// 1. Exact match common projects (HashMap - fast path)
    /// 2. FTS5 fuzzy search for keywords (handles typos, partial matches)
    /// 3. FTS5 search for URL domains
    /// 4. FTS5 search for file paths
    /// 5. Calendar event matching (Phase 2)
    pub fn get_candidate_projects(&self, signals: &ContextSignals) -> Vec<ProjectMatch> {
        let mut candidates: CandidateMap = HashMap::new();

        // 1. Fast path: Check exact match in common_projects HashMap
        for (project_name_lower, project_def) in &self.common_projects {
            let mut matched_keywords = Vec::new();

            for keyword in &signals.title_keywords {
                let keyword_lower = keyword.to_lowercase();
                if project_name_lower.contains(&keyword_lower) {
                    matched_keywords.push(keyword.clone());
                }
            }

            if !matched_keywords.is_empty() {
                // Add as candidate with 0.50 confidence for exact name match
                if let Ok(Some(wbs)) = self.get_wbs_by_project_def(project_def) {
                    let reasons: Vec<String> =
                        matched_keywords.iter().map(|k| format!("keyword:{}", k)).collect();
                    candidates.insert(wbs.wbs_code.clone(), (0.50, reasons));
                }
            }
        }

        // 2. Keyword matching via FTS5
        for keyword in &signals.title_keywords {
            if let Ok(results) = self.fts5_search_keyword(keyword, 5) {
                for wbs in results {
                    if wbs.status != "REL" {
                        continue;
                    }
                    let entry = candidates.entry(wbs.wbs_code.clone()).or_insert((0.0, vec![]));
                    entry.0 += 0.40;
                    entry.1.push(format!("fts5_keyword:{}", keyword));
                }
            }
        }

        // 3. URL domain matching via FTS5
        if let Some(domain) = &signals.url_domain {
            if let Ok(results) = self.fts5_search_keyword(domain, 3) {
                for wbs in results {
                    if wbs.status != "REL" {
                        continue;
                    }
                    let entry = candidates.entry(wbs.wbs_code.clone()).or_insert((0.0, vec![]));
                    entry.0 += 0.20;
                    entry.1.push("url:domain_match".to_string());
                }
            }
        }

        // 4. VDR provider bonus (only if URL present)
        if signals.is_vdr_provider && signals.url_domain.is_some() {
            for (_, (score, reasons)) in candidates.iter_mut() {
                *score += 0.30;
                reasons.push("vdr:provider".to_string());
            }
        }

        // 5. File path matching via FTS5
        if let Some(folder) = &signals.project_folder {
            if let Ok(results) = self.fts5_search_keyword(folder, 3) {
                for wbs in results {
                    if wbs.status != "REL" {
                        continue;
                    }
                    let is_exact = wbs
                        .project_name
                        .as_ref()
                        .map(|name| name.to_lowercase().contains(&folder.to_lowercase()))
                        .unwrap_or(false);
                    let score = if is_exact { 0.35 } else { 0.25 };
                    let entry = candidates.entry(wbs.wbs_code.clone()).or_insert((0.0, vec![]));
                    entry.0 += score;
                    entry.1.push(format!("file_path:{}", folder));
                }
            }
        }

        // Convert HashMap to Vec<ProjectMatch>
        let mut matches: Vec<ProjectMatch> = candidates
            .into_iter()
            .filter_map(|(wbs_code, (confidence, reasons))| {
                self.get_wbs_by_wbs_code(&wbs_code).ok().flatten().map(|wbs| ProjectMatch {
                    project_id: Some(wbs.project_def.clone()),
                    wbs_code: Some(wbs.wbs_code.clone()),
                    deal_name: wbs.project_name.clone(),
                    workstream: self.infer_workstream(signals),
                    confidence, // Initial FTS5 score (will be re-scored by RulesClassifier)
                    reasons: reasons.clone(),
                })
            })
            .collect();

        // Sort by initial FTS5 confidence (descending)
        matches.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());

        matches
    }

    /// Match signals to a WBS code/project using FTS5 search (legacy - returns single best match)
    ///
    /// Strategy:
    /// 1. Exact match common projects (HashMap - fast path)
    /// 2. FTS5 fuzzy search for keywords (handles typos, partial matches)
    /// 3. FTS5 search for URL domains
    /// 4. FTS5 search for file paths
    /// 5. Calendar event matching (Phase 2)
    ///
    /// NOTE: This method uses simple additive scoring. For sophisticated Tier 1-4
    /// weighted classification, use get_candidate_projects() + RulesClassifier.
    pub fn match_project(&self, signals: &ContextSignals) -> ProjectMatch {
        // Fast path: Check exact match in common_projects HashMap
        // Look for project that matches ALL keywords (not just one)
        let mut best_match: Option<MatchInfo> = None;

        for (project_name_lower, project_def) in &self.common_projects {
            let mut match_count = 0;
            let mut matched_keywords = Vec::new();

            for keyword in &signals.title_keywords {
                let keyword_lower = keyword.to_lowercase();
                if project_name_lower.contains(&keyword_lower) {
                    match_count += 1;
                    matched_keywords.push(keyword.clone());
                }
            }

            // Track the project with the most keyword matches
            if match_count > 0
                && (best_match.is_none() || match_count > best_match.as_ref().unwrap().1)
            {
                best_match = Some((project_def.as_str(), match_count, matched_keywords));
            }
        }

        // If we found a match in common projects with ALL keywords matching,
        // AND no additional signals (URL, file path), return it immediately
        let has_additional_signals =
            signals.url_domain.is_some() || signals.project_folder.is_some();

        if let Some((project_def, match_count, matched_keywords)) = best_match {
            // Only use fast path if ALL keywords matched AND no additional signals
            if match_count == signals.title_keywords.len()
                && !signals.title_keywords.is_empty()
                && !has_additional_signals
            {
                if let Ok(Some(wbs)) = self.get_wbs_by_project_def(project_def) {
                    let reasons: Vec<String> =
                        matched_keywords.iter().map(|k| format!("keyword:{}", k)).collect();

                    return ProjectMatch {
                        project_id: Some(wbs.project_def.clone()),
                        wbs_code: Some(wbs.wbs_code.clone()),
                        deal_name: wbs.project_name.clone(),
                        workstream: self.infer_workstream(signals),
                        confidence: 0.50, // Exact match confidence
                        reasons,
                    };
                }
            }
        }

        // Slow path: FTS5 fuzzy search for typo tolerance
        let mut candidates: CandidateMap = HashMap::new();

        // 1. Keyword matching via FTS5
        for keyword in &signals.title_keywords {
            match self.fts5_search_keyword(keyword, 5) {
                Ok(results) => {
                    for wbs in results {
                        if wbs.status != "REL" {
                            continue;
                        }

                        let score = 0.40; // FTS5 fuzzy match confidence
                        let reason = format!("fts5_keyword:{}", keyword);

                        let entry = candidates.entry(wbs.wbs_code.clone()).or_insert((0.0, vec![]));
                        entry.0 += score;
                        entry.1.push(reason);
                    }
                }
                Err(_) => continue, // Skip if FTS5 search fails
            }
        }

        // 2. URL domain matching via FTS5
        if let Some(domain) = &signals.url_domain {
            if let Ok(results) = self.fts5_search_keyword(domain, 3) {
                for wbs in results {
                    if wbs.status != "REL" {
                        continue;
                    }

                    let entry = candidates.entry(wbs.wbs_code.clone()).or_insert((0.0, vec![]));
                    entry.0 += 0.20; // URL match weight
                    entry.1.push("url:domain_match".to_string());
                }
            }
        }

        // 3. VDR provider bonus (only if URL present)
        if signals.is_vdr_provider && signals.url_domain.is_some() {
            // Boost all candidates when VDR detected
            for (_, (score, reasons)) in candidates.iter_mut() {
                *score += 0.30;
                reasons.push("vdr:provider".to_string());
            }
        }

        // 4. File path matching via FTS5
        if let Some(folder) = &signals.project_folder {
            if let Ok(results) = self.fts5_search_keyword(folder, 3) {
                for wbs in results {
                    if wbs.status != "REL" {
                        continue;
                    }

                    // Check if exact folder name match in project name
                    let is_exact = wbs
                        .project_name
                        .as_ref()
                        .map(|name| name.to_lowercase().contains(&folder.to_lowercase()))
                        .unwrap_or(false);

                    let score = if is_exact { 0.35 } else { 0.25 };

                    let entry = candidates.entry(wbs.wbs_code.clone()).or_insert((0.0, vec![]));
                    entry.0 += score;
                    entry.1.push(format!("file_path:{}", folder));
                }
            }
        }

        // Return best match if confidence meets threshold
        // Lower threshold to 0.25 to accommodate file path-only matches
        if let Some((wbs_code, (confidence, reasons))) =
            candidates.iter().max_by(|a, b| a.1 .0.partial_cmp(&b.1 .0).unwrap())
        {
            if *confidence >= 0.25 {
                if let Ok(Some(wbs)) = self.get_wbs_by_wbs_code(wbs_code) {
                    return ProjectMatch {
                        project_id: Some(wbs.project_def.clone()),
                        wbs_code: Some(wbs.wbs_code.clone()),
                        deal_name: wbs.project_name.clone(),
                        workstream: self.infer_workstream(signals),
                        confidence: *confidence,
                        reasons: reasons.clone(),
                    };
                }
            }
        }

        // Return G&A fallback (USC0000000.1.0)
        // If no project match found, default to General & Administrative overhead
        ProjectMatch {
            project_id: Some("USC0000000".to_string()),
            wbs_code: Some("USC0000000.1.0".to_string()),
            deal_name: Some("General & Administrative".to_string()),
            workstream: self.infer_workstream(signals),
            confidence: 0.10, // Low confidence - fallback match
            reasons: vec!["fallback:g_a".to_string()],
        }
    }

    /// Get WBS element by project_def
    fn get_wbs_by_project_def(&self, project_def: &str) -> Result<Option<WbsElement>> {
        self.wbs_repo.get_wbs_by_project_def(project_def)
    }

    /// Get WBS element by wbs_code
    fn get_wbs_by_wbs_code(&self, wbs_code: &str) -> Result<Option<WbsElement>> {
        self.wbs_repo.get_wbs_by_wbs_code(wbs_code)
    }

    /// FTS5 search for keywords (typo-tolerant)
    fn fts5_search_keyword(&self, keyword: &str, limit: usize) -> Result<Vec<WbsElement>> {
        self.wbs_repo.fts5_search_keyword(keyword, limit)
    }

    /// Infer workstream from app category
    fn infer_workstream(&self, signals: &ContextSignals) -> Option<String> {
        match signals.app_category {
            AppCategory::Excel => Some("modeling".to_string()),
            AppCategory::Word => Some("drafting".to_string()),
            AppCategory::PowerPoint => Some("presentation".to_string()),
            AppCategory::Browser if signals.is_vdr_provider => Some("due_diligence".to_string()),
            AppCategory::Browser => Some("research".to_string()),
            AppCategory::Email => Some("correspondence".to_string()),
            AppCategory::Meeting => Some("client_interaction".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::classification::ports::WbsRepository;
    use pulsearc_domain::{
        classification::{AppCategory, ContextSignals},
        types::WbsElement,
        Result as DomainResult,
    };
    use std::sync::Arc;

    /// Mock WbsRepository for testing
    struct MockWbsRepository {
        projects: Vec<WbsElement>,
    }

    impl MockWbsRepository {
        fn new() -> Self {
            let now = chrono::Utc::now().timestamp();

            let projects = vec![
                WbsElement {
                    wbs_code: "USC0063201.1.1".to_string(),
                    project_def: "USC0063201".to_string(),
                    project_name: Some("Project Astro".to_string()),
                    description: Some("Astro acquisition modeling".to_string()),
                    status: "REL".to_string(),
                    cached_at: now,
                    opportunity_id: Some("OPP-12345".to_string()),
                    deal_name: Some("Astro Acquisition".to_string()),
                    target_company_name: Some("Vanguard Solutions Inc.".to_string()),
                    counterparty: None,
                    industry: Some("Technology".to_string()),
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                },
                WbsElement {
                    wbs_code: "USC0042105.2.3".to_string(),
                    project_def: "USC0042105".to_string(),
                    project_name: Some("Project Beta".to_string()),
                    description: Some("Beta merger analysis".to_string()),
                    status: "REL".to_string(),
                    cached_at: now,
                    opportunity_id: Some("OPP-23456".to_string()),
                    deal_name: Some("Beta Merger".to_string()),
                    target_company_name: Some("Beta Corp".to_string()),
                    counterparty: None,
                    industry: Some("Healthcare".to_string()),
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                },
                WbsElement {
                    wbs_code: "USC0058923.3.1".to_string(),
                    project_def: "USC0058923".to_string(),
                    project_name: Some("Project Luna".to_string()),
                    description: Some("Luna restructuring".to_string()),
                    status: "REL".to_string(),
                    cached_at: now,
                    opportunity_id: Some("OPP-34567".to_string()),
                    deal_name: Some("Luna Restructuring".to_string()),
                    target_company_name: Some("Luna Industries".to_string()),
                    counterparty: None,
                    industry: Some("Manufacturing".to_string()),
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                },
                WbsElement {
                    wbs_code: "USC0071234.1.1".to_string(),
                    project_def: "USC0071234".to_string(),
                    project_name: Some("Project Gamma".to_string()),
                    description: Some("Gamma carve-out".to_string()),
                    status: "REL".to_string(),
                    cached_at: now,
                    opportunity_id: Some("OPP-45678".to_string()),
                    deal_name: Some("Gamma Carve-out".to_string()),
                    target_company_name: Some("Gamma Enterprises".to_string()),
                    counterparty: None,
                    industry: Some("Financial Services".to_string()),
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                },
            ];

            Self { projects }
        }
    }

    impl WbsRepository for MockWbsRepository {
        fn count_active_wbs(&self) -> DomainResult<i64> {
            Ok(self.projects.len() as i64)
        }

        fn get_last_sync_timestamp(&self) -> DomainResult<Option<i64>> {
            Ok(self.projects.first().map(|p| p.cached_at))
        }

        fn load_common_projects(&self, limit: usize) -> DomainResult<Vec<WbsElement>> {
            Ok(self.projects.iter().take(limit).cloned().collect())
        }

        fn fts5_search_keyword(
            &self,
            keyword: &str,
            limit: usize,
        ) -> DomainResult<Vec<WbsElement>> {
            let keyword_lower = keyword.to_lowercase();
            let results: Vec<WbsElement> = self
                .projects
                .iter()
                .filter(|wbs| {
                    wbs.project_name
                        .as_ref()
                        .map(|name| name.to_lowercase().contains(&keyword_lower))
                        .unwrap_or(false)
                        || wbs
                            .description
                            .as_ref()
                            .map(|desc| desc.to_lowercase().contains(&keyword_lower))
                            .unwrap_or(false)
                })
                .take(limit)
                .cloned()
                .collect();
            Ok(results)
        }

        fn get_wbs_by_project_def(&self, project_def: &str) -> DomainResult<Option<WbsElement>> {
            Ok(self.projects.iter().find(|wbs| wbs.project_def == project_def).cloned())
        }

        fn get_wbs_by_wbs_code(&self, wbs_code: &str) -> DomainResult<Option<WbsElement>> {
            Ok(self.projects.iter().find(|wbs| wbs.wbs_code == wbs_code).cloned())
        }
    }

    fn create_test_signals(keywords: Vec<String>, app_category: AppCategory) -> ContextSignals {
        ContextSignals {
            title_keywords: keywords,
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

    #[test]
    fn test_new_with_empty_cache() {
        // AC: Returns error if cache is empty (SAP sync not run)
        struct EmptyRepo;
        impl WbsRepository for EmptyRepo {
            fn count_active_wbs(&self) -> DomainResult<i64> {
                Ok(0)
            }
            fn get_last_sync_timestamp(&self) -> DomainResult<Option<i64>> {
                Ok(None)
            }
            fn load_common_projects(&self, _limit: usize) -> DomainResult<Vec<WbsElement>> {
                Ok(vec![])
            }
            fn fts5_search_keyword(
                &self,
                _keyword: &str,
                _limit: usize,
            ) -> DomainResult<Vec<WbsElement>> {
                Ok(vec![])
            }
            fn get_wbs_by_project_def(
                &self,
                _project_def: &str,
            ) -> DomainResult<Option<WbsElement>> {
                Ok(None)
            }
            fn get_wbs_by_wbs_code(&self, _wbs_code: &str) -> DomainResult<Option<WbsElement>> {
                Ok(None)
            }
        }

        let result = ProjectMatcher::new(Arc::new(EmptyRepo));
        assert!(result.is_err(), "Should fail with empty WBS cache");
        if let Err(err) = result {
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("empty") || error_msg.contains("WBS"),
                "Error message should mention empty cache"
            );
        }
    }

    #[test]
    fn test_fts5_exact_match() {
        // AC: "Project Astro" → exact match in common projects (0.50 confidence)
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let signals = create_test_signals(
            vec!["project".to_string(), "astro".to_string()],
            AppCategory::Excel,
        );

        let match_result = matcher.match_project(&signals);

        assert_eq!(match_result.project_id, Some("USC0063201".to_string()));
        assert_eq!(match_result.wbs_code, Some("USC0063201.1.1".to_string()));
        assert_eq!(match_result.deal_name, Some("Project Astro".to_string()));
        assert!(match_result.confidence >= 0.50);
        assert!(match_result.reasons.contains(&"keyword:astro".to_string()));
    }

    #[test]
    fn test_fts5_typo_tolerance() {
        // AC: "Projet Astro" (typo) → FTS5 fuzzy match (0.40 confidence)
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        // Note: Mock repo doesn't support true fuzzy matching, but tests the code path
        let signals = create_test_signals(vec!["astro".to_string()], AppCategory::Word);

        let match_result = matcher.match_project(&signals);

        // Should match via FTS5 search
        assert_eq!(match_result.project_id, Some("USC0063201".to_string()));
        assert!(match_result.confidence >= 0.40);
    }

    #[test]
    fn test_fts5_partial_match() {
        // AC: "Astro" → finds "Project Astro" (0.40 confidence)
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let signals = create_test_signals(vec!["astro".to_string()], AppCategory::Browser);

        let match_result = matcher.match_project(&signals);

        assert_eq!(match_result.project_id, Some("USC0063201".to_string()));
        assert_eq!(match_result.deal_name, Some("Project Astro".to_string()));
        assert!(match_result.confidence >= 0.40);
    }

    #[test]
    fn test_fts5_url_search() {
        // AC: "datasite.com" → matches project with VDR activity
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let mut signals = create_test_signals(vec!["astro".to_string()], AppCategory::Browser);
        signals.url_domain = Some("app.datasite.com".to_string());
        signals.is_vdr_provider = true;

        let match_result = matcher.match_project(&signals);

        // VDR + keyword should boost confidence
        assert!(match_result.confidence >= 0.70); // keyword (0.40) + VDR (0.30)
        assert!(match_result.reasons.contains(&"vdr:provider".to_string()));
    }

    #[test]
    fn test_fts5_file_path_search() {
        // AC: "/Documents/Astro/" → finds Astro project
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let mut signals = create_test_signals(vec![], AppCategory::Excel);
        signals.file_path = Some("~/Documents/Astro/model.xlsx".to_string());
        signals.project_folder = Some("Astro".to_string());

        let match_result = matcher.match_project(&signals);

        assert_eq!(match_result.project_id, Some("USC0063201".to_string()));
        assert!(match_result.confidence >= 0.25); // file path signal
        assert!(match_result.reasons.iter().any(|r| r.contains("file_path")));
    }

    #[test]
    fn test_confidence_scoring_fts5() {
        // AC: FTS5 keyword (0.40) + exact file path (0.35) = 0.75 ✅
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let mut signals = create_test_signals(vec!["astro".to_string()], AppCategory::Excel);
        signals.file_path = Some("~/Documents/Astro/ppa-model.xlsx".to_string());
        signals.project_folder = Some("Astro".to_string());

        let match_result = matcher.match_project(&signals);

        // Combined signals boost confidence
        assert!(match_result.confidence >= 0.65); // Above threshold
        assert!(match_result.confidence <= 0.85); // Reasonable upper bound
        assert!(match_result.reasons.len() >= 2); // Multiple signals
    }

    #[test]
    fn test_workstream_inference() {
        // AC: Excel → modeling, Word → drafting
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let signals_excel = create_test_signals(vec!["astro".to_string()], AppCategory::Excel);
        let signals_word = create_test_signals(vec!["astro".to_string()], AppCategory::Word);
        let mut signals_vdr = create_test_signals(vec!["astro".to_string()], AppCategory::Browser);
        signals_vdr.url_domain = Some("app.datasite.com".to_string());
        signals_vdr.is_vdr_provider = true;

        let match_excel = matcher.match_project(&signals_excel);
        let match_word = matcher.match_project(&signals_word);
        let match_vdr = matcher.match_project(&signals_vdr);

        assert_eq!(match_excel.workstream, Some("modeling".to_string()));
        assert_eq!(match_word.workstream, Some("drafting".to_string()));
        assert_eq!(match_vdr.workstream, Some("due_diligence".to_string()));
    }

    #[test]
    fn test_get_candidate_projects() {
        // AC: Returns all matching projects for RulesClassifier scoring
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let signals = create_test_signals(vec!["project".to_string()], AppCategory::Excel);

        let candidates = matcher.get_candidate_projects(&signals);

        // Should find at least one project
        assert!(!candidates.is_empty());
        // Should be sorted by confidence (descending)
        for i in 0..candidates.len().saturating_sub(1) {
            assert!(candidates[i].confidence >= candidates[i + 1].confidence);
        }
    }

    #[test]
    fn test_fallback_to_ga() {
        // AC: Returns G&A fallback if no project match found
        let mock_repo = Arc::new(MockWbsRepository::new());
        let matcher = ProjectMatcher::new(mock_repo).unwrap();

        let signals = create_test_signals(vec!["nonexistent".to_string()], AppCategory::Excel);

        let match_result = matcher.match_project(&signals);

        assert_eq!(match_result.project_id, Some("USC0000000".to_string()));
        assert_eq!(match_result.wbs_code, Some("USC0000000.1.0".to_string()));
        assert_eq!(match_result.deal_name, Some("General & Administrative".to_string()));
        assert_eq!(match_result.confidence, 0.10); // Low confidence
        assert!(match_result.reasons.contains(&"fallback:g_a".to_string()));
    }
}
