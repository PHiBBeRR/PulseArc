//! Debug-only command to seed activity snapshots for testing
//!
//! This command generates test activity snapshots with realistic data patterns
//! for classifier testing and UI development. Only available in debug builds.
//!
//! # Test Data Structure
//!
//! - Creates snapshots from embedded test blocks (first 10 blocks used)
//! - Generates snapshots at 60-second intervals within each block
//! - Adds 120-second gaps between blocks
//! - Uses realistic app names, window titles, and calendar events
//!
//! # Example Usage
//!
//! ```javascript
//! // From frontend (debug builds only)
//! const result = await invoke('seed_activity_snapshots', { count: 5 });
//! console.log(result); // "✅ Seeded 150 activity snapshots from 5 blocks"
//! ```

use std::sync::{Arc, OnceLock};

use chrono::Utc;
use pulsearc_domain::types::database::ActivitySnapshot;
use serde::{Deserialize, Serialize};
use tauri::State;
use tracing::{debug, info};
use uuid::Uuid;

use crate::context::AppContext;

/// Response from seed command
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedResponse {
    /// Number of snapshots created
    pub snapshots_created: usize,
    /// Number of blocks processed
    pub blocks_processed: usize,
    /// Human-readable success message
    pub message: String,
}

/// Embedded test block data (simplified from activity_example.json)
#[derive(Debug, Clone, Deserialize, Serialize)]
struct TestBlock {
    app_name: String,
    window_title: String,
    duration_secs: i32,
    url: Option<String>,
    bundle_id: Option<String>,
    calendar_title: Option<String>,
    has_calendar_event: bool,
}

/// Seed activity snapshots for testing (debug builds only)
///
/// # Arguments
///
/// * `ctx` - Application context with database access
/// * `count` - Optional number of blocks to seed (default: 10, max: 10)
///
/// # Returns
///
/// Success message with snapshot and block counts, or error string
#[tauri::command]
pub async fn seed_activity_snapshots(
    ctx: State<'_, Arc<AppContext>>,
    count: Option<usize>,
) -> Result<SeedResponse, String> {
    let base_start = seed_base_timestamp();
    let blocks_to_use = count.unwrap_or(10).min(10);
    debug!(blocks = blocks_to_use, "Seeding activity snapshots");

    // Generate test blocks
    let test_blocks = generate_test_blocks();
    let blocks = &test_blocks[..blocks_to_use.min(test_blocks.len())];

    // Build snapshots
    let snapshots = build_snapshots_from_blocks(blocks, base_start);
    let snapshot_count = snapshots.len();

    // Save using repository (wrap in spawn_blocking since repository is sync)
    let snapshots_clone = Arc::new(snapshots);
    let snapshot_repo = ctx.snapshots.clone();

    tokio::task::spawn_blocking(move || {
        snapshot_repo
            .store_snapshots_batch(&snapshots_clone)
            .map_err(|e| format!("Failed to save snapshots: {}", e))
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))??;

    let message =
        format!("✅ Seeded {} activity snapshots from {} blocks", snapshot_count, blocks.len());

    info!(
        snapshots = snapshot_count,
        blocks = blocks.len(),
        "Successfully seeded activity snapshots"
    );

    Ok(SeedResponse { snapshots_created: snapshot_count, blocks_processed: blocks.len(), message })
}

/// Generate embedded test blocks (simplified real-world patterns)
fn generate_test_blocks() -> Vec<TestBlock> {
    vec![
        TestBlock {
            app_name: "Visual Studio Code".into(),
            window_title: "main.rs - pulsearc-app".into(),
            duration_secs: 1200, // 20 minutes
            url: None,
            bundle_id: Some("com.microsoft.VSCode".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Google Chrome".into(),
            window_title: "GitHub - Pull Request #123".into(),
            duration_secs: 900, // 15 minutes
            url: Some("https://github.com/example/repo/pull/123".into()),
            bundle_id: Some("com.google.Chrome".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Zoom".into(),
            window_title: "Team Standup - Zoom Meeting".into(),
            duration_secs: 1800, // 30 minutes
            url: None,
            bundle_id: Some("us.zoom.xos".into()),
            calendar_title: Some("Team Standup".into()),
            has_calendar_event: true,
        },
        TestBlock {
            app_name: "Slack".into(),
            window_title: "#engineering - Slack".into(),
            duration_secs: 600, // 10 minutes
            url: None,
            bundle_id: Some("com.tinyspeck.slackmacgap".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Terminal".into(),
            window_title: "bash - ~/Documents/PulseArc".into(),
            duration_secs: 1500, // 25 minutes
            url: None,
            bundle_id: Some("com.apple.Terminal".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Figma".into(),
            window_title: "Dashboard Design - Figma".into(),
            duration_secs: 2400, // 40 minutes
            url: Some("https://figma.com/file/abc123".into()),
            bundle_id: Some("com.figma.Desktop".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Google Chrome".into(),
            window_title: "Stack Overflow - rust async traits".into(),
            duration_secs: 480, // 8 minutes
            url: Some("https://stackoverflow.com/questions/12345".into()),
            bundle_id: Some("com.google.Chrome".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Notion".into(),
            window_title: "Project Planning - Notion".into(),
            duration_secs: 1200, // 20 minutes
            url: Some("https://notion.so/workspace/page".into()),
            bundle_id: Some("notion.id".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
        TestBlock {
            app_name: "Zoom".into(),
            window_title: "1:1 with Manager - Zoom Meeting".into(),
            duration_secs: 1800, // 30 minutes
            url: None,
            bundle_id: Some("us.zoom.xos".into()),
            calendar_title: Some("1:1 with Manager".into()),
            has_calendar_event: true,
        },
        TestBlock {
            app_name: "Visual Studio Code".into(),
            window_title: "tests.rs - pulsearc-app".into(),
            duration_secs: 1800, // 30 minutes
            url: None,
            bundle_id: Some("com.microsoft.VSCode".into()),
            calendar_title: None,
            has_calendar_event: false,
        },
    ]
}

/// Build activity snapshots from test blocks
///
/// Creates snapshots at 60-second intervals with 120-second gaps between blocks
fn build_snapshots_from_blocks(blocks: &[TestBlock], base_start: i64) -> Vec<ActivitySnapshot> {
    let mut snapshots = Vec::new();
    let mut cursor = base_start;

    for block in blocks.iter() {
        let start_time = cursor;
        let end_time = start_time + block.duration_secs as i64;

        // Create snapshots every 60 seconds
        for timestamp in (start_time..end_time).step_by(60) {
            let activity_context = build_activity_context(block);
            let detected_activity = if let Some(title) = &block.calendar_title {
                title.clone()
            } else {
                block.window_title.clone()
            };

            snapshots.push(ActivitySnapshot {
                id: Uuid::now_v7().to_string(),
                timestamp,
                activity_context_json: serde_json::to_string(&activity_context)
                    .unwrap_or_else(|_| "{}".into()),
                detected_activity,
                work_type: None,
                activity_category: None,
                primary_app: block.app_name.clone(),
                processed: false,
                batch_id: Some("debug-seed".to_string()),
                created_at: timestamp,
                processed_at: None,
                is_idle: false,
                idle_duration_secs: None,
            });
        }

        // Advance cursor by block duration plus 120-second gap
        cursor = end_time + 120;
    }

    snapshots
}

/// Ensure the seed dataset starts at a stable, minute-aligned timestamp
fn seed_base_timestamp() -> i64 {
    static SEED_START: OnceLock<i64> = OnceLock::new();
    *SEED_START.get_or_init(|| align_to_minute(Utc::now().timestamp()))
}

fn align_to_minute(ts: i64) -> i64 {
    ts - (ts % 60)
}

/// Build activity context JSON from test block
fn build_activity_context(block: &TestBlock) -> serde_json::Value {
    let mut context = serde_json::json!({
        "active_app": {
            "app_name": block.app_name,
            "window_title": block.window_title,
            "bundle_id": block.bundle_id,
        },
        "recent_apps": [],
        "detected_activity": block.window_title,
    });

    // Add URL if present
    if let Some(url) = &block.url {
        context["active_app"]["url"] = serde_json::json!(url);
        if let Some(host) = extract_url_host(url) {
            context["active_app"]["url_host"] = serde_json::json!(host);
        }
    }

    // Add calendar event if present
    if block.has_calendar_event {
        if let Some(title) = &block.calendar_title {
            context["calendar_event"] = serde_json::json!({
                "title": title,
                "has_external_attendees": false,
                "organizer_domain": "example.com",
            });
        }
    }

    context
}

/// Extract host from URL for url_host field
fn extract_url_host(url: &str) -> Option<String> {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(String::from))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_test_blocks_returns_10_blocks() {
        let blocks = generate_test_blocks();
        assert_eq!(blocks.len(), 10);
    }

    #[test]
    fn build_snapshots_creates_expected_count() {
        let blocks = generate_test_blocks();
        let snapshots = build_snapshots_from_blocks(&blocks[..3], seed_base_timestamp());

        // First 3 blocks: 1200s + 900s + 1800s = 3900s
        // Snapshots every 60s = 3900 / 60 = 65 snapshots
        assert_eq!(snapshots.len(), 65);
    }

    #[test]
    fn build_activity_context_includes_required_fields() {
        let block = TestBlock {
            app_name: "Test App".into(),
            window_title: "Test Window".into(),
            duration_secs: 300,
            url: Some("https://example.com/page".into()),
            bundle_id: Some("com.test.app".into()),
            calendar_title: Some("Test Meeting".into()),
            has_calendar_event: true,
        };

        let context = build_activity_context(&block);

        assert_eq!(context["active_app"]["app_name"], "Test App");
        assert_eq!(context["active_app"]["window_title"], "Test Window");
        assert_eq!(context["active_app"]["bundle_id"], "com.test.app");
        assert_eq!(context["active_app"]["url"], "https://example.com/page");
        assert_eq!(context["active_app"]["url_host"], "example.com");
        assert_eq!(context["calendar_event"]["title"], "Test Meeting");
    }

    #[test]
    fn extract_url_host_handles_various_formats() {
        assert_eq!(extract_url_host("https://github.com/owner/repo"), Some("github.com".into()));
        assert_eq!(extract_url_host("http://localhost:3000/page"), Some("localhost".into()));
        assert_eq!(extract_url_host("invalid-url"), None);
    }

    #[test]
    fn snapshots_have_correct_gaps() {
        let blocks = vec![
            TestBlock {
                app_name: "App1".into(),
                window_title: "Window1".into(),
                duration_secs: 120, // 2 minutes = 2 snapshots
                url: None,
                bundle_id: None,
                calendar_title: None,
                has_calendar_event: false,
            },
            TestBlock {
                app_name: "App2".into(),
                window_title: "Window2".into(),
                duration_secs: 180, // 3 minutes = 3 snapshots
                url: None,
                bundle_id: None,
                calendar_title: None,
                has_calendar_event: false,
            },
        ];

        let snapshots = build_snapshots_from_blocks(&blocks, seed_base_timestamp());

        // First block: 2 snapshots (0s, 60s)
        // Gap: 120s
        // Second block: 3 snapshots (240s, 300s, 360s)
        assert_eq!(snapshots.len(), 5);

        // Verify gap between blocks
        let gap = snapshots[2].timestamp - snapshots[1].timestamp;
        assert_eq!(gap, 180); // 60s (remaining in block 1) + 120s (gap)
    }
}
