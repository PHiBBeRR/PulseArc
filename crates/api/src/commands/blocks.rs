//! Block building and management commands - Phase 4B.1
//!
//! Commands for building time blocks from activity segments, and managing
//! proposed blocks (accept/dismiss). Uses hexagonal architecture with
//! AppContext dependency injection.
//!
//! # Commands
//!
//! - `build_my_day` - Build blocks for a specific day from segments
//! - `accept_proposed_block` - Accept a block and enqueue for SAP sync
//! - `dismiss_proposed_block` - Reject a proposed block
//!
//! # Note
//!
//! - `get_proposed_blocks` is implemented in suggestions.rs to avoid
//!   duplication
//! - Classification is handled separately by ClassificationScheduler

use std::sync::Arc;

use chrono::{DateTime, Local, Utc};
use pulsearc_core::classification::BlockBuilder;
use pulsearc_domain::types::classification::ProposedBlock;
use pulsearc_domain::types::{OutboxStatus, TimeEntryOutbox};
use pulsearc_domain::{PulseArcError, Result};
use tauri::{Emitter, State};
use tracing::{info, warn};

use crate::adapters::blocks::{block_to_time_entry_dto, generate_idempotency_key};
use crate::context::AppContext;

// ============================================================================
// Command: build_my_day
// ============================================================================

/// Build time blocks for a specific day from activity segments
///
/// # Arguments
///
/// * `ctx` - Application context with repositories
/// * `day_epoch` - Unix timestamp for start of day (optional, defaults to
///   today)
///
/// # Returns
///
/// Vector of ProposedBlock with status "pending_classification"
///
/// # Phase 4B.1 Migration Notes
///
/// - Uses AppContext repositories instead of legacy DbManager
/// - Returns blocks with status="pending_classification" (classification is
///   separate)
/// - Idempotent: Returns existing blocks if already built for the day
/// - Uses BlockBuilder from core for business logic
#[tauri::command]
pub async fn build_my_day(
    ctx: State<'_, Arc<AppContext>>,
    day_epoch: Option<i64>,
) -> Result<Vec<ProposedBlock>> {
    let app_ctx = Arc::clone(&ctx);

    // Determine target day (default to today)
    let target_day = match day_epoch {
        Some(epoch) => epoch,
        None => {
            // Calculate today at midnight
            let today = Local::now().date_naive();
            let midnight = today.and_hms_opt(0, 0, 0).ok_or_else(|| {
                PulseArcError::Internal("Failed to construct midnight time".to_string())
            })?;
            midnight
                .and_local_timezone(Local)
                .single()
                .ok_or_else(|| {
                    PulseArcError::Internal("Ambiguous local timezone conversion".to_string())
                })?
                .timestamp()
        }
    };

    let date = DateTime::from_timestamp(target_day, 0)
        .ok_or_else(|| PulseArcError::InvalidInput(format!("Invalid day_epoch: {}", target_day)))?
        .date_naive();

    info!(day = %date, "Building blocks for day");

    // Idempotency check: Return existing blocks if already built
    let existing_blocks = app_ctx.block_repository.get_proposed_blocks(date).await?;
    let existing_suggested: Vec<_> = existing_blocks
        .into_iter()
        .filter(|b| b.status == "suggested" || b.status == "pending_classification")
        .collect();

    if !existing_suggested.is_empty() {
        info!(count = existing_suggested.len(), "Returning existing blocks (idempotent)");
        return Ok(existing_suggested);
    }

    // Fetch segments for the day (synchronous repository call via spawn_blocking)
    let segment_repo = Arc::clone(&app_ctx.segment_repository);
    let segments = tokio::task::spawn_blocking(move || segment_repo.find_segments_by_date(date))
        .await
        .map_err(|e| PulseArcError::Internal(format!("Task join error: {}", e)))?
        .map_err(|e| PulseArcError::Internal(format!("Failed to fetch segments: {}", e)))?;

    if segments.is_empty() {
        warn!(day = %date, "No segments found for day");
        return Ok(vec![]);
    }

    // Build blocks using BlockBuilder
    let config = app_ctx.block_repository.get_block_config().await?;
    let builder = BlockBuilder::new(config)?;
    let mut blocks = builder.build_daily_blocks_from_segments(&segments, target_day)?;

    info!(count = blocks.len(), "Built {} blocks from segments", blocks.len());

    // Save all blocks to repository
    for block in &blocks {
        app_ctx.block_repository.save_proposed_block(block).await?;
    }

    // Update status to "pending_classification" (classification happens separately)
    for block in &mut blocks {
        block.status = "pending_classification".to_string();
    }

    info!(count = blocks.len(), "Saved blocks with pending_classification status");

    Ok(blocks)
}

// ============================================================================
// Command: accept_proposed_block
// ============================================================================

/// Accept a proposed block and enqueue it for SAP sync
///
/// # Arguments
///
/// * `ctx` - Application context
/// * `block_id` - ID of the block to accept
///
/// # Returns
///
/// Success message or error
///
/// # Phase 4B.1 Migration Notes
///
/// - Uses block_repository.approve_block() instead of raw SQL
/// - Uses outbox_queue.enqueue() for SAP sync
/// - Emits "outbox-updated" event for frontend reactivity
/// - Gets user_id from user_profile repository (single-user system)
#[tauri::command]
pub async fn accept_proposed_block(
    ctx: State<'_, Arc<AppContext>>,
    app: tauri::AppHandle,
    block_id: String,
) -> Result<String> {
    let app_ctx = Arc::clone(&ctx);

    info!(block_id = %block_id, "Accepting proposed block");

    // Fetch the block
    let block = app_ctx
        .block_repository
        .get_proposed_block(&block_id)
        .await?
        .ok_or_else(|| PulseArcError::InvalidInput(format!("Block {} not found", block_id)))?;

    // Get current user profile (single-user system assumption)
    let user_profile = app_ctx.user_profile.get_current_profile().await?.ok_or_else(|| {
        PulseArcError::InvalidInput("No user profile found. Please log in.".into())
    })?;

    let user_id = user_profile.auth0_id;
    let org_id = user_profile.org_id;

    // Convert block to time entry DTO for SAP
    let dto = block_to_time_entry_dto(&block, &user_id, &org_id)
        .map_err(|e| PulseArcError::Internal(format!("Failed to convert block to DTO: {}", e)))?;

    // Generate idempotency key
    let idempotency_key = generate_idempotency_key(&block.id, &user_id, block.start_ts);

    let now = Utc::now().timestamp();

    // Create outbox entry
    let outbox_entry = TimeEntryOutbox {
        id: uuid::Uuid::now_v7().to_string(),
        idempotency_key: idempotency_key.clone(),
        user_id: user_id.clone(),
        payload_json: serde_json::to_string(&dto)
            .map_err(|e| PulseArcError::Internal(format!("Failed to serialize payload: {}", e)))?,
        backend_cuid: None,
        status: OutboxStatus::Pending,
        attempts: 0,
        last_error: None,
        retry_after: None,
        created_at: now,
        sent_at: None,
        correlation_id: None,
        local_status: None,
        remote_status: None,
        sap_entry_id: None,
        next_attempt_at: None,
        error_code: None,
        last_forwarded_at: None,
        wbs_code: block.inferred_wbs_code.clone(),
        target: "sap".to_string(),
        description: None,
        auto_applied: false,
        version: 1,
        last_modified_by: user_id.clone(),
        last_modified_at: Some(now),
    };

    // Enqueue for sync
    app_ctx.outbox_queue.enqueue(&outbox_entry).await?;

    // Mark block as approved
    app_ctx.block_repository.approve_block(&block_id, Utc::now()).await?;

    info!(
        block_id = %block_id,
        idempotency_key = %idempotency_key,
        "Block accepted and queued for sync"
    );

    if let Err(err) = app.emit("outbox-updated", ()) {
        warn!(
            block_id = %block_id,
            error = %err,
            "failed to emit outbox-updated event after block acceptance"
        );
    }

    Ok(format!("Block {} accepted successfully", block_id))
}

// ============================================================================
// Command: dismiss_proposed_block
// ============================================================================

/// Dismiss/reject a proposed block
///
/// # Arguments
///
/// * `ctx` - Application context
/// * `block_id` - ID of the block to dismiss
///
/// # Returns
///
/// Success message or error
///
/// # Phase 4B.1 Migration Notes
///
/// - Uses block_repository.reject_block() instead of raw SQL
/// - Simple rejection, no outbox entry created
#[tauri::command]
pub async fn dismiss_proposed_block(
    ctx: State<'_, Arc<AppContext>>,
    block_id: String,
) -> Result<String> {
    let app_ctx = Arc::clone(&ctx);

    info!(block_id = %block_id, "Dismissing proposed block");

    // Verify block exists
    let _block = app_ctx
        .block_repository
        .get_proposed_block(&block_id)
        .await?
        .ok_or_else(|| PulseArcError::InvalidInput(format!("Block {} not found", block_id)))?;

    // Reject the block
    app_ctx.block_repository.reject_block(&block_id, Utc::now()).await?;

    info!(block_id = %block_id, "Block dismissed successfully");

    Ok(format!("Block {} dismissed", block_id))
}
