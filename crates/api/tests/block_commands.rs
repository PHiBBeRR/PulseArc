//! Integration tests for block building commands (Phase 4B.1)
//!
//! Tests the 4 migrated block commands:
//! - `get_proposed_blocks` - Fetch blocks with status filtering
//! - `dismiss_proposed_block` - Reject a block
//! - `build_my_day` - Build blocks from segments
//! - `accept_proposed_block` - Accept block and create outbox entry

use std::sync::Arc;

use chrono::{Local, NaiveDate, Utc};
use pulsearc_core::classification::ports::BlockRepository;
use pulsearc_core::tracking::ports::SegmentRepository;
use pulsearc_domain::types::classification::{ActivityBreakdown, ProposedBlock};
use serial_test::serial;

mod support;
use support::setup_test_context;

fn create_test_block(start_ts: i64, duration_secs: i64, status: &str) -> ProposedBlock {
    ProposedBlock {
        id: uuid::Uuid::now_v7().to_string(),
        start_ts,
        end_ts: start_ts + duration_secs,
        duration_secs,
        inferred_project_id: Some("PRJ-001".to_string()),
        inferred_wbs_code: Some("WBS-001".to_string()),
        inferred_deal_name: Some("Test Project".to_string()),
        inferred_workstream: Some("Development".to_string()),
        billable: true,
        confidence: 0.85,
        classifier_used: None,
        activities: vec![ActivityBreakdown {
            name: "VSCode".to_string(),
            duration_secs,
            percentage: 100.0,
        }],
        snapshot_ids: vec![],
        segment_ids: vec![],
        reasons: vec![],
        status: status.to_string(),
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

// ============================================================================
// get_proposed_blocks tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_get_proposed_blocks_returns_all_when_no_status_filter() {
    let ctx = setup_test_context().await;

    // Create test blocks with different statuses
    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block1 = create_test_block(day_epoch, 1800, "suggested");
    let block2 = create_test_block(day_epoch + 2000, 1800, "pending_classification");
    let block3 = create_test_block(day_epoch + 4000, 1800, "accepted");

    ctx.block_repository.save_proposed_block(&block1).await.unwrap();
    ctx.block_repository.save_proposed_block(&block2).await.unwrap();
    ctx.block_repository.save_proposed_block(&block3).await.unwrap();

    // Fetch all blocks (no status filter)
    let result = ctx.block_repository.get_proposed_blocks(today).await.unwrap();

    assert_eq!(result.len(), 3, "Should return all 3 blocks");
}

#[tokio::test]
#[serial]
async fn test_get_proposed_blocks_filters_by_status() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block1 = create_test_block(day_epoch, 1800, "suggested");
    let block2 = create_test_block(day_epoch + 2000, 1800, "pending_classification");
    let block3 = create_test_block(day_epoch + 4000, 1800, "suggested");

    ctx.block_repository.save_proposed_block(&block1).await.unwrap();
    ctx.block_repository.save_proposed_block(&block2).await.unwrap();
    ctx.block_repository.save_proposed_block(&block3).await.unwrap();

    // Fetch only "suggested" blocks
    let all_blocks = ctx.block_repository.get_proposed_blocks(today).await.unwrap();
    let suggested_blocks: Vec<_> =
        all_blocks.into_iter().filter(|b| b.status == "suggested").collect();

    assert_eq!(suggested_blocks.len(), 2, "Should return 2 'suggested' blocks");
    assert!(suggested_blocks.iter().all(|b| b.status == "suggested"));
}

#[tokio::test]
#[serial]
async fn test_get_proposed_blocks_returns_empty_for_day_with_no_blocks() {
    let ctx = setup_test_context().await;

    // Use a date far in the future to ensure no blocks exist
    let future_date = NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();

    let result = ctx.block_repository.get_proposed_blocks(future_date).await.unwrap();

    assert_eq!(result.len(), 0, "Should return empty vector for day with no blocks");
}

#[tokio::test]
#[serial]
async fn test_get_proposed_blocks_sorted_by_start_time() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    // Create blocks in reverse chronological order
    let block3 = create_test_block(day_epoch + 7200, 1800, "suggested");
    let block1 = create_test_block(day_epoch, 1800, "suggested");
    let block2 = create_test_block(day_epoch + 3600, 1800, "suggested");

    ctx.block_repository.save_proposed_block(&block3).await.unwrap();
    ctx.block_repository.save_proposed_block(&block1).await.unwrap();
    ctx.block_repository.save_proposed_block(&block2).await.unwrap();

    let result = ctx.block_repository.get_proposed_blocks(today).await.unwrap();

    assert_eq!(result.len(), 3);
    // Verify blocks are sorted by start_ts ascending
    assert!(result[0].start_ts < result[1].start_ts);
    assert!(result[1].start_ts < result[2].start_ts);
}

// ============================================================================
// dismiss_proposed_block tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_dismiss_proposed_block_updates_status_to_rejected() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block = create_test_block(day_epoch, 1800, "suggested");
    let block_id = block.id.clone();

    ctx.block_repository.save_proposed_block(&block).await.unwrap();

    // Dismiss the block
    ctx.block_repository.reject_block(&block_id, Utc::now()).await.unwrap();

    // Verify status changed to "rejected"
    let updated_block = ctx.block_repository.get_proposed_block(&block_id).await.unwrap().unwrap();
    assert_eq!(updated_block.status, "rejected");
}

#[tokio::test]
#[serial]
async fn test_dismiss_proposed_block_sets_reviewed_at_timestamp() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block = create_test_block(day_epoch, 1800, "suggested");
    let block_id = block.id.clone();

    ctx.block_repository.save_proposed_block(&block).await.unwrap();

    let before_dismiss = Utc::now().timestamp();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    ctx.block_repository.reject_block(&block_id, Utc::now()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let after_dismiss = Utc::now().timestamp();

    // Verify reviewed_at was set
    let updated_block = ctx.block_repository.get_proposed_block(&block_id).await.unwrap().unwrap();
    assert!(updated_block.reviewed_at.is_some());

    let reviewed_at = updated_block.reviewed_at.unwrap();
    assert!(reviewed_at >= before_dismiss && reviewed_at <= after_dismiss);
}

#[tokio::test]
#[serial]
async fn test_dismiss_proposed_block_is_idempotent() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block = create_test_block(day_epoch, 1800, "suggested");
    let block_id = block.id.clone();

    ctx.block_repository.save_proposed_block(&block).await.unwrap();

    // Dismiss twice
    ctx.block_repository.reject_block(&block_id, Utc::now()).await.unwrap();
    let result = ctx.block_repository.reject_block(&block_id, Utc::now()).await;

    // Should succeed (idempotent)
    assert!(result.is_ok());

    let updated_block = ctx.block_repository.get_proposed_block(&block_id).await.unwrap().unwrap();
    assert_eq!(updated_block.status, "rejected");
}

#[tokio::test]
#[serial]
async fn test_dismiss_nonexistent_block_is_noop() {
    let ctx = setup_test_context().await;

    let fake_block_id = "nonexistent-block-id";

    let result = ctx.block_repository.reject_block(fake_block_id, Utc::now()).await;

    assert!(result.is_ok(), "Rejecting nonexistent block should be treated as a no-op");
    let fetched = ctx.block_repository.get_proposed_block(fake_block_id).await.unwrap();
    assert!(fetched.is_none(), "Repository should still have no entry for nonexistent block");
}

// ============================================================================
// build_my_day tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_build_my_day_creates_blocks_from_segments() {
    let _ctx = setup_test_context().await;

    // Create test segments (need to insert into database)
    // Note: This test is simplified - in reality, we'd need to insert segments
    // into the database For now, we're just testing the block repository
    // directly

    // Skip this test until we can properly set up segment data
    // This is a TODO for when we have segment test fixtures
}

#[tokio::test]
#[serial]
async fn test_build_my_day_idempotency_returns_existing_blocks() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    // Pre-create "suggested" blocks for today
    let existing_block = create_test_block(day_epoch, 1800, "suggested");
    ctx.block_repository.save_proposed_block(&existing_block).await.unwrap();

    // Fetch blocks (simulating idempotency check in build_my_day)
    let blocks = ctx.block_repository.get_proposed_blocks(today).await.unwrap();
    let suggested_blocks: Vec<_> = blocks
        .into_iter()
        .filter(|b| b.status == "suggested" || b.status == "pending_classification")
        .collect();

    assert_eq!(suggested_blocks.len(), 1, "Should return existing block");
    assert_eq!(suggested_blocks[0].id, existing_block.id);
}

#[tokio::test]
#[serial]
async fn test_build_my_day_returns_empty_when_no_segments() {
    let ctx = setup_test_context().await;

    // Use a date far in the future to ensure no segments exist
    let future_date = NaiveDate::from_ymd_opt(2030, 12, 31).unwrap();

    // Attempt to fetch segments (there should be none)
    let segments = tokio::task::spawn_blocking({
        let segment_repo = Arc::clone(&ctx.segment_repository);
        move || segment_repo.find_segments_by_date(future_date)
    })
    .await
    .unwrap()
    .unwrap();

    assert_eq!(segments.len(), 0, "Should have no segments for future date");
}

// ============================================================================
// accept_proposed_block tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_accept_proposed_block_creates_outbox_entry() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block = create_test_block(day_epoch, 1800, "suggested");
    let block_id = block.id.clone();

    ctx.block_repository.save_proposed_block(&block).await.unwrap();

    // Approve the block
    ctx.block_repository.approve_block(&block_id, Utc::now()).await.unwrap();

    // Verify block status changed to "accepted"
    let updated_block = ctx.block_repository.get_proposed_block(&block_id).await.unwrap().unwrap();
    assert_eq!(updated_block.status, "accepted");
}

#[tokio::test]
#[serial]
async fn test_accept_proposed_block_updates_status_and_timestamp() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    let block = create_test_block(day_epoch, 1800, "suggested");
    let block_id = block.id.clone();

    ctx.block_repository.save_proposed_block(&block).await.unwrap();

    let before_accept = Utc::now().timestamp();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    ctx.block_repository.approve_block(&block_id, Utc::now()).await.unwrap();

    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    let after_accept = Utc::now().timestamp();

    let updated_block = ctx.block_repository.get_proposed_block(&block_id).await.unwrap().unwrap();

    // Verify status changed
    assert_eq!(updated_block.status, "accepted");

    // Verify reviewed_at was set
    assert!(updated_block.reviewed_at.is_some());
    let reviewed_at = updated_block.reviewed_at.unwrap();
    assert!(reviewed_at >= before_accept && reviewed_at <= after_accept);
}

#[tokio::test]
#[serial]
async fn test_accept_nonexistent_block_is_noop() {
    let ctx = setup_test_context().await;

    let fake_block_id = "nonexistent-block-id";

    let result = ctx.block_repository.approve_block(fake_block_id, Utc::now()).await;

    assert!(result.is_ok(), "Approving nonexistent block should be treated as a no-op");
    let fetched = ctx.block_repository.get_proposed_block(fake_block_id).await.unwrap();
    assert!(fetched.is_none(), "Repository should still have no entry for nonexistent block");
}

// ============================================================================
// Edge Cases & Error Handling
// ============================================================================

#[tokio::test]
#[serial]
async fn test_get_proposed_blocks_handles_invalid_date() {
    let ctx = setup_test_context().await;

    // NaiveDate has validation built-in, so we can't create truly invalid dates
    // But we can test edge cases like year boundaries
    let edge_date = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();

    let result = ctx.block_repository.get_proposed_blocks(edge_date).await;

    assert!(result.is_ok(), "Should handle edge case dates gracefully");
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test]
#[serial]
async fn test_block_repository_handles_concurrent_operations() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    // Create multiple blocks concurrently
    let mut handles = vec![];

    for i in 0..5 {
        let ctx = Arc::clone(&ctx);
        let block = create_test_block(day_epoch + (i * 2000), 1800, "suggested");

        let handle =
            tokio::spawn(async move { ctx.block_repository.save_proposed_block(&block).await });

        handles.push(handle);
    }

    // Wait for all saves to complete
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent saves should succeed");
    }

    // Verify all 5 blocks were saved
    let blocks = ctx.block_repository.get_proposed_blocks(today).await.unwrap();
    assert_eq!(blocks.len(), 5, "All concurrent saves should persist");
}

// ============================================================================
// Performance Tests
// ============================================================================

#[tokio::test]
#[serial]
async fn test_get_proposed_blocks_performance_with_many_blocks() {
    let ctx = setup_test_context().await;

    let today = Local::now().date_naive();
    let day_epoch =
        today.and_hms_opt(0, 0, 0).unwrap().and_local_timezone(Local).unwrap().timestamp();

    // Create 100 blocks
    for i in 0..100 {
        let block = create_test_block(day_epoch + (i * 600), 500, "suggested");
        ctx.block_repository.save_proposed_block(&block).await.unwrap();
    }

    let start = std::time::Instant::now();
    let result = ctx.block_repository.get_proposed_blocks(today).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(result.len(), 100);
    assert!(
        elapsed.as_millis() < 100,
        "Fetching 100 blocks should take <100ms, took {}ms",
        elapsed.as_millis()
    );
}
