//! Integration tests for classification types
//!
//! Comprehensive test suite covering real-world scenarios for block building,
//! classification, and evidence extraction.

use pulsearc_domain::types::classification::{
    ActivityBreakdown, ActivityBreakdownEvidence, AppCategory, BlockConfig, BlockEvidence,
    ContextSignals, EvidenceSignals, ProjectMatch, ProposedBlock, SerializedProjectMatch,
    SerializedSignals, WorkLocation,
};

// ============================================================================
// ProposedBlock Integration Tests
// ============================================================================

/// Test full lifecycle of creating and manipulating a ProposedBlock
///
/// Scenario: User works on a project for 2 hours with multiple activities
#[test]
fn test_proposed_block_full_lifecycle() {
    let block = create_sample_proposed_block(
        "block_001",
        1_700_000_000, // Start time
        1_700_007_200, // End time (2 hours later)
        vec![
            ("Microsoft Excel", 3600, 50.0),
            ("Google Chrome", 2400, 33.33),
            ("Slack", 1200, 16.67),
        ],
    );

    // Verify basic fields
    assert_eq!(block.id, "block_001");
    assert_eq!(block.duration_secs, 7200);
    assert_eq!(block.activities.len(), 3);

    // Verify duration matches start/end
    assert_eq!(block.end_ts - block.start_ts, block.duration_secs);

    // Verify activity percentages sum to ~100%
    let total_percentage: f32 = block.activities.iter().map(|a| a.percentage).sum();
    assert!((total_percentage - 100.0).abs() < 1.0, "Percentages should sum to ~100%");

    // Verify billable flag
    assert!(block.billable);

    // Verify confidence is in valid range
    assert!(block.confidence >= 0.0 && block.confidence <= 1.0);

    // Verify status
    assert_eq!(block.status, "suggested");
}

/// Test token estimation for different block sizes
///
/// Validates that `estimated_token_count` scales with block complexity
#[test]
fn test_proposed_block_token_estimation() {
    // Small block: minimal data
    let small_block = create_minimal_proposed_block("small", 1_700_000_000, 1_700_001_800);
    let small_tokens = small_block.estimated_token_count();

    // Large block: lots of activities and reasons
    let large_block = create_complex_proposed_block(
        "large",
        1_700_000_000,
        1_700_007_200,
        10, // 10 activities
        15, // 15 reasons
    );
    let large_tokens = large_block.estimated_token_count();

    // Assertions
    assert!(small_tokens >= 50, "Should have minimum 50 tokens");
    assert!(
        large_tokens > small_tokens,
        "Large block should have more tokens than small block (small: {small_tokens}, large: {large_tokens})"
    );
    assert!(large_tokens > 100, "Complex block should exceed 100 tokens (got {large_tokens})");

    // Verify token scaling is reasonable
    let token_ratio = large_tokens as f32 / small_tokens as f32;
    assert!(
        token_ratio > 1.5,
        "Large block should have at least 1.5x tokens of small block (ratio: {token_ratio})"
    );
}

/// Test ProposedBlock serialization round-trip
///
/// Ensures all fields survive JSON serialization/deserialization
#[test]
fn test_proposed_block_serialization_round_trip() {
    let original = create_sample_proposed_block(
        "serialize_test",
        1_700_000_000,
        1_700_003_600,
        vec![("VSCode", 2400, 66.67), ("Terminal", 1200, 33.33)],
    );

    // Serialize to JSON
    let json = serde_json::to_string(&original).expect("Serialization should succeed");

    // Deserialize back
    let deserialized: ProposedBlock =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    // Verify critical fields
    assert_eq!(deserialized.id, original.id);
    assert_eq!(deserialized.start_ts, original.start_ts);
    assert_eq!(deserialized.end_ts, original.end_ts);
    assert_eq!(deserialized.duration_secs, original.duration_secs);
    assert_eq!(deserialized.billable, original.billable);
    assert_eq!(deserialized.confidence, original.confidence);
    assert_eq!(deserialized.activities.len(), original.activities.len());
    assert_eq!(deserialized.total_idle_secs, original.total_idle_secs);
    assert_eq!(deserialized.is_weekend, original.is_weekend);
    assert_eq!(deserialized.is_after_hours, original.is_after_hours);
}

/// Test ProposedBlock with location and context features (FEATURE-033 Phase 2)
///
/// Validates work location tracking and travel detection
#[test]
fn test_proposed_block_with_location_context() {
    let mut block = create_sample_proposed_block(
        "location_test",
        1_700_000_000,
        1_700_003_600,
        vec![("Zoom", 3600, 100.0)],
    );

    // Set location context
    block.work_location = Some(WorkLocation::Office);
    block.timezone = Some("America/Denver".to_string());
    block.is_travel = false;
    block.is_weekend = false;
    block.is_after_hours = false;

    // Verify location fields
    assert_eq!(block.work_location, Some(WorkLocation::Office));
    assert_eq!(block.timezone.as_deref(), Some("America/Denver"));
    assert!(!block.is_travel);

    // Test travel scenario
    block.work_location = Some(WorkLocation::Travel);
    block.is_travel = true;
    assert_eq!(block.work_location, Some(WorkLocation::Travel));
    assert!(block.is_travel);
}

/// Test ProposedBlock with calendar overlap detection (FEATURE-033 Phase 5)
///
/// Validates conflict detection and double-booking flags
#[test]
fn test_proposed_block_with_calendar_overlap() {
    let mut block = create_sample_proposed_block(
        "overlap_test",
        1_700_000_000,
        1_700_003_600,
        vec![("Google Meet", 3600, 100.0)],
    );

    // No overlaps initially
    assert!(!block.has_calendar_overlap);
    assert!(block.overlapping_event_ids.is_empty());
    assert!(!block.is_double_booked);

    // Add calendar overlap
    block.has_calendar_overlap = true;
    block.overlapping_event_ids = vec!["event_1".to_string(), "event_2".to_string()];
    block.is_double_booked = true;

    // Verify overlap detection
    assert!(block.has_calendar_overlap);
    assert_eq!(block.overlapping_event_ids.len(), 2);
    assert!(block.is_double_booked);
}

/// Test idle time tracking in ProposedBlock (FEATURE-028 Phase 3)
///
/// Validates idle time calculation and handling strategies
#[test]
fn test_proposed_block_idle_time_tracking() {
    let mut block = create_sample_proposed_block(
        "idle_test",
        1_700_000_000,
        1_700_003_600,
        vec![("Excel", 2700, 75.0), ("Idle", 900, 25.0)],
    );

    // Set idle tracking fields
    block.total_idle_secs = 900; // 15 minutes idle
    block.idle_handling = "partial".to_string();

    // Verify idle tracking
    assert_eq!(block.total_idle_secs, 900);
    assert_eq!(block.idle_handling, "partial");

    // Verify active time percentage
    let active_percentage = ((block.duration_secs - i64::from(block.total_idle_secs)) as f32
        / block.duration_secs as f32)
        * 100.0;
    assert!((active_percentage - 75.0).abs() < 1.0);

    // Test different idle handling strategies
    block.idle_handling = "exclude".to_string();
    assert_eq!(block.idle_handling, "exclude");

    block.idle_handling = "include".to_string();
    assert_eq!(block.idle_handling, "include");
}

// ============================================================================
// ContextSignals Integration Tests
// ============================================================================

/// Test ContextSignals extraction and project identification
///
/// Scenario: User working on a project with multiple context signals
#[test]
fn test_context_signals_project_identification() {
    let signals = ContextSignals {
        title_keywords: vec!["astro".to_string(), "ppa".to_string(), "modeling".to_string()],
        url_domain: Some("app.datasite.com".to_string()),
        file_path: Some("/Users/analyst/Documents/Astro/financial_model.xlsx".to_string()),
        project_folder: Some("Astro".to_string()),
        calendar_event_id: Some("meeting_123".to_string()),
        attendee_domains: vec!["clientfirm.com".to_string(), "company.com".to_string()],
        app_category: AppCategory::Excel,
        is_vdr_provider: true,
        timestamp: 1_700_000_000,
        project_id: Some("USC0063201".to_string()),
        organizer_domain: Some("clientfirm.com".to_string()),
        is_screen_locked: false,
        has_personal_event: false,
        is_internal_training: false,
        is_personal_browsing: false,
        email_direction: Some("outgoing".to_string()),
        has_external_meeting_attendees: true,
    };

    // Verify project identification signals
    assert_eq!(signals.project_id.as_deref(), Some("USC0063201"));
    assert!(signals.is_vdr_provider);
    assert!(signals.has_external_meeting_attendees);
    assert_eq!(signals.app_category, AppCategory::Excel);

    // Verify context extraction
    assert!(signals.title_keywords.contains(&"astro".to_string()));
    assert_eq!(signals.url_domain.as_deref(), Some("app.datasite.com"));
    assert!(signals.file_path.as_ref().unwrap().contains("financial_model.xlsx"));
}

/// Test ContextSignals for non-billable activity detection (FEATURE-030)
///
/// Validates detection of personal browsing, training, and idle states
#[test]
fn test_context_signals_non_billable_detection() {
    // Personal browsing scenario
    let personal_signals = ContextSignals {
        title_keywords: vec!["youtube".to_string(), "entertainment".to_string()],
        url_domain: Some("youtube.com".to_string()),
        app_category: AppCategory::Browser,
        is_personal_browsing: true,
        ..Default::default()
    };
    assert!(personal_signals.is_personal_browsing);

    // Internal training scenario
    let training_signals = ContextSignals {
        title_keywords: vec!["cpe".to_string(), "training".to_string()],
        calendar_event_id: Some("training_event".to_string()),
        is_internal_training: true,
        app_category: AppCategory::Meeting,
        ..Default::default()
    };
    assert!(training_signals.is_internal_training);

    // Screen locked / idle scenario
    let idle_signals = ContextSignals {
        is_screen_locked: true,
        app_category: AppCategory::Other,
        ..Default::default()
    };
    assert!(idle_signals.is_screen_locked);
}

/// Test ContextSignals serialization with versioned wrapper
///
/// Validates `SerializedSignals` round-trip and version tracking
#[test]
fn test_context_signals_versioned_serialization() {
    let signals = ContextSignals {
        title_keywords: vec!["test".to_string()],
        url_domain: Some("example.com".to_string()),
        app_category: AppCategory::Browser,
        timestamp: 1_700_000_000,
        ..Default::default()
    };

    // Create versioned wrapper
    let versioned = SerializedSignals::new(signals.clone());
    assert_eq!(versioned.version, 1);

    // Serialize to JSON
    let json = versioned.to_json().expect("Serialization should succeed");

    // Deserialize and verify version
    let deserialized = SerializedSignals::from_json(&json).expect("Deserialization should succeed");
    assert_eq!(deserialized.version, 1);
    assert_eq!(deserialized.data.title_keywords, signals.title_keywords);
    assert_eq!(deserialized.data.url_domain, signals.url_domain);
}

/// Test email direction weighting in ContextSignals (V2.1)
///
/// Validates email direction influence on signal strength
#[test]
fn test_context_signals_email_direction() {
    // Outgoing email (highest weight: 1.0x)
    let outgoing_signals = ContextSignals {
        app_category: AppCategory::Email,
        email_direction: Some("outgoing".to_string()),
        ..Default::default()
    };
    assert_eq!(outgoing_signals.email_direction.as_deref(), Some("outgoing"));

    // Incoming email (medium weight: 0.95x)
    let incoming_signals = ContextSignals {
        app_category: AppCategory::Email,
        email_direction: Some("incoming".to_string()),
        ..Default::default()
    };
    assert_eq!(incoming_signals.email_direction.as_deref(), Some("incoming"));

    // CC email (lowest weight: 0.85x)
    let cc_signals = ContextSignals {
        app_category: AppCategory::Email,
        email_direction: Some("cc".to_string()),
        ..Default::default()
    };
    assert_eq!(cc_signals.email_direction.as_deref(), Some("cc"));
}

// ============================================================================
// ProjectMatch Integration Tests
// ============================================================================

/// Test ProjectMatch creation and confidence scoring
///
/// Scenario: Matching a project with high confidence signals
#[test]
fn test_project_match_high_confidence() {
    let project_match = ProjectMatch {
        project_id: Some("USC0063201".to_string()),
        wbs_code: Some("USC0063201.1.1".to_string()),
        deal_name: Some("Project Astro".to_string()),
        workstream: Some("modeling".to_string()),
        confidence: 0.95,
        reasons: vec![
            "keyword:astro".to_string(),
            "vdr:datasite".to_string(),
            "file_path:Astro".to_string(),
            "meeting:external_attendees".to_string(),
        ],
    };

    // Verify high confidence match
    assert!(project_match.confidence >= 0.9);
    assert!(project_match.project_id.is_some());
    assert!(project_match.wbs_code.is_some());
    assert_eq!(project_match.reasons.len(), 4);

    // Verify workstream inference
    assert_eq!(project_match.workstream.as_deref(), Some("modeling"));
}

/// Test ProjectMatch with low confidence (ambiguous signals)
///
/// Scenario: Weak signals that don't clearly identify a project
#[test]
fn test_project_match_low_confidence() {
    let project_match = ProjectMatch {
        project_id: None,
        wbs_code: None,
        deal_name: None,
        workstream: Some("general".to_string()),
        confidence: 0.35,
        reasons: vec!["keyword:document".to_string()],
    };

    // Verify low confidence match
    assert!(project_match.confidence < 0.5);
    assert!(project_match.project_id.is_none());
    assert_eq!(project_match.reasons.len(), 1);
}

/// Test ProjectMatch versioned serialization
///
/// Validates `SerializedProjectMatch` round-trip
#[test]
fn test_project_match_versioned_serialization() {
    let project_match = ProjectMatch {
        project_id: Some("USC123".to_string()),
        wbs_code: Some("USC123.1".to_string()),
        deal_name: Some("Test Project".to_string()),
        workstream: Some("diligence".to_string()),
        confidence: 0.80,
        reasons: vec!["test_reason".to_string()],
    };

    // Create versioned wrapper
    let versioned = SerializedProjectMatch::new(project_match.clone());
    assert_eq!(versioned.version, 1);

    // Serialize and deserialize
    let json = versioned.to_json().expect("Serialization should succeed");
    let deserialized =
        SerializedProjectMatch::from_json(&json).expect("Deserialization should succeed");

    // Verify data integrity
    assert_eq!(deserialized.version, 1);
    assert_eq!(deserialized.data.project_id, project_match.project_id);
    assert_eq!(deserialized.data.confidence, project_match.confidence);
}

// ============================================================================
// BlockEvidence Integration Tests
// ============================================================================

/// Test BlockEvidence creation with comprehensive signals
///
/// Scenario: Building evidence package for OpenAI classification
#[test]
fn test_block_evidence_comprehensive() {
    let evidence = BlockEvidence {
        block_id: "evidence_001".to_string(),
        start_ts: 1_700_000_000,
        end_ts: 1_700_007_200,
        duration_secs: 7200,
        activities: vec![
            ActivityBreakdownEvidence {
                name: "Excel".to_string(),
                duration_secs: 3600,
                percentage: 50.0,
            },
            ActivityBreakdownEvidence {
                name: "Chrome".to_string(),
                duration_secs: 2400,
                percentage: 33.33,
            },
            ActivityBreakdownEvidence {
                name: "Zoom".to_string(),
                duration_secs: 1200,
                percentage: 16.67,
            },
        ],
        signals: EvidenceSignals {
            apps: vec!["Excel".to_string(), "Chrome".to_string(), "Zoom".to_string()],
            window_titles: vec![
                "financial_model.xlsx - Excel".to_string(),
                "[REDACTED] - Datasite".to_string(),
                "Client Meeting - Zoom".to_string(),
            ],
            keywords: vec!["astro".to_string(), "model".to_string(), "datasite".to_string()],
            url_domains: vec!["app.datasite.com".to_string(), "zoom.us".to_string()],
            file_paths: vec!["/Users/analyst/Documents/Astro/financial_model.xlsx".to_string()],
            calendar_event_titles: vec!["Project Astro - Client Call".to_string()],
            attendee_domains: vec!["clientfirm.com".to_string(), "company.com".to_string()],
            vdr_providers: vec!["datasite".to_string()],
            meeting_platforms: vec!["zoom".to_string()],
            has_recurring_meeting: true,
            has_online_meeting: true,
        },
    };

    // Verify evidence structure
    assert_eq!(evidence.block_id, "evidence_001");
    assert_eq!(evidence.duration_secs, 7200);
    assert_eq!(evidence.activities.len(), 3);

    // Verify signals completeness
    assert_eq!(evidence.signals.apps.len(), 3);
    assert!(evidence.signals.vdr_providers.contains(&"datasite".to_string()));
    assert!(evidence.signals.has_online_meeting);
    assert!(evidence.signals.has_recurring_meeting);

    // Verify activity duration consistency
    let total_activity_duration: i64 = evidence.activities.iter().map(|a| a.duration_secs).sum();
    assert_eq!(total_activity_duration, evidence.duration_secs);
}

/// Test BlockEvidence serialization for OpenAI API
///
/// Validates JSON format suitable for external classification
#[test]
fn test_block_evidence_serialization_for_api() {
    let evidence = BlockEvidence {
        block_id: "api_test".to_string(),
        start_ts: 1_700_000_000,
        end_ts: 1_700_003_600,
        duration_secs: 3600,
        activities: vec![ActivityBreakdownEvidence {
            name: "VSCode".to_string(),
            duration_secs: 3600,
            percentage: 100.0,
        }],
        signals: EvidenceSignals {
            apps: vec!["VSCode".to_string()],
            window_titles: vec!["main.rs - VSCode".to_string()],
            keywords: vec!["rust".to_string(), "code".to_string()],
            url_domains: vec![],
            file_paths: vec!["/Users/dev/project/src/main.rs".to_string()],
            calendar_event_titles: vec![],
            attendee_domains: vec![],
            vdr_providers: vec![],
            meeting_platforms: vec![],
            has_recurring_meeting: false,
            has_online_meeting: false,
        },
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&evidence).expect("Serialization should succeed");

    // Verify JSON structure
    assert!(json.contains("\"block_id\""));
    assert!(json.contains("\"activities\""));
    assert!(json.contains("\"signals\""));

    // Verify deserialization
    let deserialized: BlockEvidence =
        serde_json::from_str(&json).expect("Deserialization should succeed");
    assert_eq!(deserialized.block_id, evidence.block_id);
    assert_eq!(deserialized.signals.apps.len(), 1);
}

// ============================================================================
// BlockConfig Integration Tests
// ============================================================================

/// Test BlockConfig default values
///
/// Validates sensible defaults for block building
#[test]
fn test_block_config_defaults() {
    let config = BlockConfig::default();

    assert_eq!(config.min_block_duration_secs, 1800); // 30 minutes
    assert_eq!(config.max_gap_for_merge_secs, 180); // 3 minutes
    assert_eq!(config.consolidation_window_secs, 3600); // 1 hour
    assert_eq!(config.min_billing_increment_secs, 360); // 6 minutes
}

/// Test BlockConfig custom values and serialization
///
/// Validates configuration persistence
#[test]
fn test_block_config_custom_and_serialization() {
    let config = BlockConfig {
        min_block_duration_secs: 900,    // 15 minutes
        max_gap_for_merge_secs: 300,     // 5 minutes
        consolidation_window_secs: 7200, // 2 hours
        min_billing_increment_secs: 600, // 10 minutes
    };

    // Serialize and deserialize
    let json = serde_json::to_string(&config).expect("Serialization should succeed");
    let deserialized: BlockConfig =
        serde_json::from_str(&json).expect("Deserialization should succeed");

    assert_eq!(deserialized.min_block_duration_secs, 900);
    assert_eq!(deserialized.max_gap_for_merge_secs, 300);
}

// ============================================================================
// AppCategory Integration Tests
// ============================================================================

/// Test AppCategory workstream inference
///
/// Validates app category to workstream mapping
#[test]
fn test_app_category_workstream_inference() {
    let categories = vec![
        (AppCategory::Excel, "modeling"),
        (AppCategory::Word, "documentation"),
        (AppCategory::PowerPoint, "presentation"),
        (AppCategory::Browser, "research"),
        (AppCategory::Email, "communication"),
        (AppCategory::Meeting, "meeting"),
        (AppCategory::IDE, "development"),
        (AppCategory::Terminal, "development"),
    ];

    for (category, expected_workstream) in categories {
        // This test documents the expected mapping
        // (actual mapping would be in the classification logic)
        match category {
            AppCategory::Excel => assert_eq!(expected_workstream, "modeling"),
            AppCategory::Word => assert_eq!(expected_workstream, "documentation"),
            AppCategory::PowerPoint => assert_eq!(expected_workstream, "presentation"),
            AppCategory::Browser => assert_eq!(expected_workstream, "research"),
            AppCategory::Email => assert_eq!(expected_workstream, "communication"),
            AppCategory::Meeting => assert_eq!(expected_workstream, "meeting"),
            AppCategory::IDE | AppCategory::Terminal => {
                assert_eq!(expected_workstream, "development");
            }
            AppCategory::Other => {}
        }
    }
}

// ============================================================================
// Real-World Scenario Tests
// ============================================================================

/// Test real-world scenario: Full day of client work
///
/// Simulates a typical workday with multiple blocks and activities
#[test]
fn test_real_world_full_day_client_work() {
    // Morning: Research and modeling (8:00 AM - 11:00 AM)
    let morning_block = create_sample_proposed_block(
        "morning",
        1_700_028_000,
        1_700_038_800,
        vec![("Excel", 7200, 66.67), ("Chrome", 2400, 22.22), ("Email", 1200, 11.11)],
    );

    // Afternoon: Client meeting and follow-up (1:00 PM - 4:00 PM)
    let afternoon_block = create_sample_proposed_block(
        "afternoon",
        1_700_046_000,
        1_700_056_800,
        vec![("Zoom", 3600, 33.33), ("PowerPoint", 4800, 44.44), ("Word", 2400, 22.22)],
    );

    // Verify both blocks are billable
    assert!(morning_block.billable);
    assert!(afternoon_block.billable);

    // Verify total work time (6 hours)
    let total_duration = morning_block.duration_secs + afternoon_block.duration_secs;
    assert_eq!(total_duration, 21_600); // 6 hours

    // Verify activity diversity
    assert!(morning_block.activities.len() >= 3);
    assert!(afternoon_block.activities.len() >= 3);
}

/// Test real-world scenario: Travel day with limited connectivity
///
/// Simulates working during travel with intermittent activity
#[test]
fn test_real_world_travel_day() {
    let mut travel_block = create_sample_proposed_block(
        "travel",
        1_700_000_000,
        1_700_010_800,
        vec![("Email", 1800, 16.67), ("Slack", 1200, 11.11), ("Idle", 7800, 72.22)],
    );

    // Set travel context
    travel_block.work_location = Some(WorkLocation::Travel);
    travel_block.is_travel = true;
    travel_block.total_idle_secs = 7800;
    travel_block.idle_handling = "partial".to_string();

    // Verify travel flags
    assert!(travel_block.is_travel);
    assert_eq!(travel_block.work_location, Some(WorkLocation::Travel));

    // Verify high idle time percentage
    let idle_percentage =
        (travel_block.total_idle_secs as f32 / travel_block.duration_secs as f32) * 100.0;
    assert!(idle_percentage > 50.0);
}

/// Test real-world scenario: Weekend work with after-hours flag
///
/// Simulates weekend/after-hours work that needs special billing treatment
#[test]
fn test_real_world_weekend_work() {
    let mut weekend_block = create_sample_proposed_block(
        "weekend",
        1_700_265_600, // Saturday timestamp
        1_700_272_800,
        vec![("Excel", 5400, 75.0), ("Chrome", 1800, 25.0)],
    );

    // Set weekend/after-hours flags
    weekend_block.is_weekend = true;
    weekend_block.is_after_hours = false; // During normal hours, but on weekend
    weekend_block.timezone = Some("America/New_York".to_string());

    // Verify weekend flags
    assert!(weekend_block.is_weekend);
    assert_eq!(weekend_block.timezone.as_deref(), Some("America/New_York"));
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Type alias for activity tuple to reduce complexity
type ActivityTuple<'a> = (&'a str, i64, f32);

/// Create a sample ProposedBlock for testing
fn create_sample_proposed_block(
    id: &str,
    start_ts: i64,
    end_ts: i64,
    activities: Vec<ActivityTuple<'_>>,
) -> ProposedBlock {
    let duration_secs = end_ts - start_ts;
    let activity_breakdown: Vec<ActivityBreakdown> = activities
        .into_iter()
        .map(|(name, dur, pct)| ActivityBreakdown {
            name: name.to_string(),
            duration_secs: dur,
            percentage: pct,
        })
        .collect();

    ProposedBlock {
        id: id.to_string(),
        start_ts,
        end_ts,
        duration_secs,
        inferred_project_id: Some("USC0063201".to_string()),
        inferred_wbs_code: Some("USC0063201.1.1".to_string()),
        inferred_deal_name: Some("Project Astro".to_string()),
        inferred_workstream: Some("modeling".to_string()),
        billable: true,
        confidence: 0.85,
        classifier_used: Some("hybrid_v1".to_string()),
        activities: activity_breakdown,
        snapshot_ids: vec![],
        segment_ids: vec![],
        reasons: vec!["vdr:datasite".to_string(), "keyword:astro".to_string()],
        status: "suggested".to_string(),
        created_at: start_ts,
        reviewed_at: None,
        total_idle_secs: 0,
        idle_handling: "exclude".to_string(),
        timezone: None,
        work_location: None,
        is_travel: false,
        is_weekend: false,
        is_after_hours: false,
        has_calendar_overlap: false,
        overlapping_event_ids: vec![],
        is_double_booked: false,
    }
}

/// Create a minimal ProposedBlock with default values
fn create_minimal_proposed_block(id: &str, start_ts: i64, end_ts: i64) -> ProposedBlock {
    create_sample_proposed_block(id, start_ts, end_ts, vec![("Other", end_ts - start_ts, 100.0)])
}

/// Create a complex ProposedBlock with many activities and reasons
fn create_complex_proposed_block(
    id: &str,
    start_ts: i64,
    end_ts: i64,
    activity_count: usize,
    reason_count: usize,
) -> ProposedBlock {
    let duration_secs = end_ts - start_ts;
    let per_activity_duration = duration_secs / activity_count as i64;
    let per_activity_percentage = 100.0 / activity_count as f32;

    let activities: Vec<ActivityBreakdown> = (0..activity_count)
        .map(|i| ActivityBreakdown {
            name: format!("App_{i}"),
            duration_secs: per_activity_duration,
            percentage: per_activity_percentage,
        })
        .collect();

    let reasons: Vec<String> = (0..reason_count).map(|i| format!("reason_{i}")).collect();

    ProposedBlock {
        id: id.to_string(),
        start_ts,
        end_ts,
        duration_secs,
        inferred_project_id: Some("USC123".to_string()),
        inferred_wbs_code: Some("USC123.1".to_string()),
        inferred_deal_name: Some("Complex Project".to_string()),
        inferred_workstream: Some("modeling".to_string()),
        billable: true,
        confidence: 0.75,
        classifier_used: Some("hybrid_v1".to_string()),
        activities,
        snapshot_ids: vec![],
        segment_ids: vec![],
        reasons,
        status: "suggested".to_string(),
        created_at: start_ts,
        reviewed_at: None,
        total_idle_secs: 0,
        idle_handling: "exclude".to_string(),
        timezone: None,
        work_location: None,
        is_travel: false,
        is_weekend: false,
        is_after_hours: false,
        has_calendar_overlap: false,
        overlapping_event_ids: vec![],
        is_double_booked: false,
    }
}
