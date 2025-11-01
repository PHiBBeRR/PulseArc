//! Time entry suggestions and proposed blocks commands

use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use chrono::{DateTime, Utc};
use pulsearc_domain::types::classification::ProposedBlock;
use pulsearc_domain::types::database::TimeEntryOutbox;
use pulsearc_domain::{OutboxStatus, PulseArcError};
use tauri::State;
use tokio::task;
use tracing::{info, warn};

// Internal result type for database operations
type DomainResult<T> = std::result::Result<T, PulseArcError>;

use crate::context::AppContext;
use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};

/// Get dismissed time entry suggestions
#[tauri::command]
pub async fn get_dismissed_suggestions(
    ctx: State<'_, Arc<AppContext>>,
) -> DomainResult<Vec<TimeEntryOutbox>> {
    let command_name = "suggestions::get_dismissed_suggestions";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Fetching dismissed suggestions");
    let result = fetch_outbox_entries(&app_ctx, Some(OutboxStatus::Dismissed)).await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_label = result.as_ref().err().map(|err| err.to_string());

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success,
            error_type: error_label.as_deref(),
        },
    )
    .await;

    result
}

/// Get proposed time blocks for a specific day
#[tauri::command]
pub async fn get_proposed_blocks(
    ctx: State<'_, Arc<AppContext>>,
    day_epoch: i64,
    status: Option<String>,
) -> DomainResult<Vec<ProposedBlock>> {
    let command_name = "suggestions::get_proposed_blocks";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(
        command = command_name,
        day_epoch,
    status = ?status,
    "Fetching proposed blocks"
    );
    let result = fetch_proposed_blocks(&app_ctx, day_epoch, status).await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_label = result.as_ref().err().map(|err| err.to_string());

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success,
            error_type: error_label.as_deref(),
        },
    )
    .await;

    result
}

/// Get outbox status (legacy time entry suggestions)
#[tauri::command]
pub async fn get_outbox_status(
    ctx: State<'_, Arc<AppContext>>,
) -> DomainResult<Vec<TimeEntryOutbox>> {
    let command_name = "suggestions::get_outbox_status";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Fetching outbox status");
    let result = fetch_outbox_entries(&app_ctx, None).await;
    let elapsed = start.elapsed();
    let success = result.is_ok();
    let error_label = result.as_ref().err().map(|err| err.to_string());

    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success,
            error_type: error_label.as_deref(),
        },
    )
    .await;

    result
}

async fn fetch_proposed_blocks(
    ctx: &Arc<AppContext>,
    day_epoch: i64,
    status: Option<String>,
) -> DomainResult<Vec<ProposedBlock>> {
    let target_day = DateTime::<Utc>::from_timestamp(day_epoch, 0)
        .ok_or_else(|| PulseArcError::InvalidInput(format!("Invalid day_epoch: {day_epoch}")))?;

    let mut blocks = ctx.block_repository.get_proposed_blocks(target_day.date_naive()).await?;

    if let Some(filter) = status.and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }) {
        blocks.retain(|block| block.status.eq_ignore_ascii_case(&filter));
    }

    Ok(blocks)
}

async fn fetch_outbox_entries(
    ctx: &Arc<AppContext>,
    status_filter: Option<OutboxStatus>,
) -> DomainResult<Vec<TimeEntryOutbox>> {
    let db = Arc::clone(&ctx.db);
    let status_as_string = status_filter.map(|status| status.to_string());

    task::spawn_blocking(move || -> DomainResult<Vec<TimeEntryOutbox>> {
        let conn = db.get_connection()?;

        // Use as_ref() to avoid consuming status_as_string
        let sql = match status_as_string.as_ref() {
            Some(_) => OUTBOX_SELECT_BY_STATUS,
            None => OUTBOX_SELECT_ALL,
        };

        let mut stmt = conn.prepare(sql).map_err(|e| PulseArcError::Database(e.to_string()))?;

        // SqlCipherStatement::query_map returns Vec<T>, not an iterator
        let entries = if let Some(status) = status_as_string {
            stmt.query_map(rusqlite::params![status], map_outbox_row)
                .map_err(|e| PulseArcError::Database(e.to_string()))?
        } else {
            stmt.query_map(rusqlite::params![], map_outbox_row)
                .map_err(|e| PulseArcError::Database(e.to_string()))?
        };

        Ok(entries)
    })
    .await
    .map_err(|err| PulseArcError::Internal(format!("outbox query task failed: {err}")))?
}

fn map_outbox_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<TimeEntryOutbox> {
    Ok(TimeEntryOutbox {
        id: row.get(0)?,
        idempotency_key: row.get(1)?,
        user_id: row.get(2)?,
        payload_json: row.get(3)?,
        backend_cuid: row.get(4)?,
        status: parse_outbox_status(row.get::<_, String>(5)?),
        attempts: row.get(6)?,
        last_error: row.get(7)?,
        retry_after: row.get(8)?,
        created_at: row.get(9)?,
        sent_at: row.get(10)?,
        correlation_id: row.get(11)?,
        local_status: row.get(12)?,
        remote_status: row.get(13)?,
        sap_entry_id: row.get(14)?,
        next_attempt_at: row.get(15)?,
        error_code: row.get(16)?,
        last_forwarded_at: row.get(17)?,
        wbs_code: row.get(18)?,
        target: row.get(19)?,
        description: row.get(20)?,
        auto_applied: int_to_bool(row.get(21)?),
        version: row.get(22)?,
        last_modified_by: row.get(23)?,
        last_modified_at: row.get(24)?,
    })
}

fn parse_outbox_status(raw: String) -> OutboxStatus {
    match OutboxStatus::from_str(&raw) {
        Ok(status) => status,
        Err(err) => {
            warn!(status = %raw, error = %err, "invalid outbox status detected, defaulting to pending");
            OutboxStatus::Pending
        }
    }
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

fn map_rusqlite_error(context: &str, err: rusqlite::Error) -> PulseArcError {
    PulseArcError::Database(format!("{context} failed: {err}"))
}

const OUTBOX_SELECT_ALL: &str = r#"
    SELECT
        id, idempotency_key, user_id, payload_json, backend_cuid, status, attempts, last_error,
        retry_after, created_at, sent_at, correlation_id, local_status, remote_status, sap_entry_id,
        next_attempt_at, error_code, last_forwarded_at, wbs_code, target, description, auto_applied,
        version, last_modified_by, last_modified_at
    FROM time_entry_outbox
    ORDER BY created_at DESC, id ASC
"#;

const OUTBOX_SELECT_BY_STATUS: &str = r#"
    SELECT
        id, idempotency_key, user_id, payload_json, backend_cuid, status, attempts, last_error,
        retry_after, created_at, sent_at, correlation_id, local_status, remote_status, sap_entry_id,
        next_attempt_at, error_code, last_forwarded_at, wbs_code, target, description, auto_applied,
        version, last_modified_by, last_modified_at
    FROM time_entry_outbox
    WHERE status = ?1
    ORDER BY created_at DESC, id ASC
"#;

// =============================================================================
// Suggestion Management Commands (Phase 4 - Legacy Migration)
// =============================================================================

/// Delete all suggestions from the outbox
///
/// Replaces legacy `clear_outbox` command
#[tauri::command]
pub async fn clear_suggestions(
    ctx: State<'_, Arc<AppContext>>,
) -> std::result::Result<usize, String> {
    let command_name = "suggestions::clear_suggestions";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Clearing all suggestions");

    let db = Arc::clone(&app_ctx.db);
    let result = task::spawn_blocking(move || -> DomainResult<usize> {
        let conn = db.get_connection()?;
        let count = conn
            .execute("DELETE FROM time_entry_outbox", rusqlite::params![])
            .map_err(|err| map_rusqlite_error("clear suggestions", err))?;
        Ok(count)
    })
    .await
    .map_err(|err| format!("Task join error: {err}"))?;

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("clear_failed") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// Delete a specific suggestion by ID
///
/// Replaces legacy `delete_outbox_entry` command
#[tauri::command]
pub async fn delete_suggestion(
    ctx: State<'_, Arc<AppContext>>,
    id: String,
) -> std::result::Result<(), String> {
    let command_name = "suggestions::delete_suggestion";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, id, "Deleting suggestion");

    let db = Arc::clone(&app_ctx.db);
    let result = task::spawn_blocking(move || -> DomainResult<()> {
        let conn = db.get_connection()?;
        let count = conn
            .execute("DELETE FROM time_entry_outbox WHERE id = ?1", rusqlite::params![&id])
            .map_err(|err| map_rusqlite_error("delete suggestion", err))?;

        if count == 0 {
            return Err(PulseArcError::NotFound(format!("Suggestion {} not found", id)));
        }

        Ok(())
    })
    .await
    .map_err(|err| format!("Task join error: {err}"))?;

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("delete_failed") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// Dismiss a suggestion (mark as dismissed)
#[tauri::command]
pub async fn dismiss_suggestion(
    ctx: State<'_, Arc<AppContext>>,
    id: String,
    reason: Option<String>,
) -> std::result::Result<(), String> {
    let command_name = "suggestions::dismiss_suggestion";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, id, "Dismissing suggestion");

    let db = Arc::clone(&app_ctx.db);
    let now = chrono::Utc::now().timestamp();

    let result = task::spawn_blocking(move || -> DomainResult<()> {
        let conn = db.get_connection()?;
        let count = conn
            .execute(
                "UPDATE time_entry_outbox SET status = ?1, last_error = ?2, last_modified_at = ?3 WHERE id = ?4",
                rusqlite::params![
                    OutboxStatus::Dismissed.to_string(),
                    reason.unwrap_or_else(|| "User dismissed".to_string()),
                    now,
                    &id
                ],
            )
            .map_err(|err| map_rusqlite_error("dismiss suggestion", err))?;

        if count == 0 {
            return Err(PulseArcError::NotFound(format!("Suggestion {} not found", id)));
        }

        Ok(())
    })
    .await
    .map_err(|err| format!("Task join error: {err}"))?;

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("dismiss_failed") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// Restore a dismissed suggestion back to pending
#[tauri::command]
pub async fn restore_suggestion(
    ctx: State<'_, Arc<AppContext>>,
    id: String,
) -> std::result::Result<(), String> {
    let command_name = "suggestions::restore_suggestion";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, id, "Restoring suggestion");

    let db = Arc::clone(&app_ctx.db);
    let now = chrono::Utc::now().timestamp();

    let result = task::spawn_blocking(move || -> DomainResult<()> {
        let conn = db.get_connection()?;
        let count = conn
            .execute(
                "UPDATE time_entry_outbox SET status = ?1, last_error = NULL, last_modified_at = ?2 WHERE id = ?3",
                rusqlite::params![OutboxStatus::Pending.to_string(), now, &id],
            )
            .map_err(|err| map_rusqlite_error("restore suggestion", err))?;

        if count == 0 {
            return Err(PulseArcError::NotFound(format!("Suggestion {} not found", id)));
        }

        Ok(())
    })
    .await
    .map_err(|err| format!("Task join error: {err}"))?;

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("restore_failed") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// Update a suggestion entry
#[tauri::command]
pub async fn update_suggestion(
    ctx: State<'_, Arc<AppContext>>,
    id: String,
    payload_json: String,
) -> std::result::Result<(), String> {
    let command_name = "suggestions::update_suggestion";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, id, "Updating suggestion");

    let db = Arc::clone(&app_ctx.db);
    let now = chrono::Utc::now().timestamp();

    let result = task::spawn_blocking(move || -> DomainResult<()> {
        let conn = db.get_connection()?;
        let count = conn
            .execute(
                "UPDATE time_entry_outbox SET payload_json = ?1, last_modified_at = ?2 WHERE id = ?3",
                rusqlite::params![&payload_json, now, &id],
            )
            .map_err(|err| map_rusqlite_error("update suggestion", err))?;

        if count == 0 {
            return Err(PulseArcError::NotFound(format!("Suggestion {} not found", id)));
        }

        Ok(())
    })
    .await
    .map_err(|err| format!("Task join error: {err}"))?;

    let elapsed = start.elapsed();
    let success = result.is_ok();

    log_command_execution(command_name, "new", elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation: "new",
            elapsed,
            success,
            error_type: if !success { Some("update_failed") } else { None },
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pulsearc_common::testing::TempDir;
    use pulsearc_core::OutboxQueue;
    use pulsearc_domain::types::classification::{ActivityBreakdown, ProposedBlock};
    use pulsearc_domain::{Config, DatabaseConfig};
    use pulsearc_infra::database::SqlCipherOutboxRepository;
    use uuid::Uuid;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_proposed_blocks_filters_status() {
        let (ctx, temp_dir) = create_app_context().await;
        let day_start = Utc::now().date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc().timestamp();

        let mut suggested = sample_block("block-suggested", day_start, "suggested");
        suggested.status = "suggested".into();
        ctx.block_repository.save_proposed_block(&suggested).await.expect("save suggested");

        let mut accepted = sample_block("block-accepted", day_start + 3600, "accepted");
        accepted.status = "accepted".into();
        ctx.block_repository.save_proposed_block(&accepted).await.expect("save accepted");

        let blocks =
            fetch_proposed_blocks(&ctx, day_start, Some("suggested".into())).await.expect("fetch");
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].id, "block-suggested");

        ctx.shutdown().await.expect("shutdown succeeds");
        drop(temp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_dismissed_suggestions_returns_only_dismissed() {
        let (ctx, temp_dir) = create_app_context().await;
        let outbox_repo = SqlCipherOutboxRepository::new(Arc::clone(&ctx.db));

        let dismissed_entry =
            sample_outbox("dismissed-entry", OutboxStatus::Dismissed, 1_700_000_000);
        outbox_repo.enqueue(&dismissed_entry).await.expect("enqueue dismissed");

        let pending_entry = sample_outbox("pending-entry", OutboxStatus::Pending, 1_700_000_100);
        outbox_repo.enqueue(&pending_entry).await.expect("enqueue pending");

        let dismissed =
            fetch_outbox_entries(&ctx, Some(OutboxStatus::Dismissed)).await.expect("fetch");
        assert_eq!(dismissed.len(), 1);
        assert_eq!(dismissed[0].id, dismissed_entry.id);
        assert_eq!(dismissed[0].status, OutboxStatus::Dismissed);

        ctx.shutdown().await.expect("shutdown succeeds");
        drop(temp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_outbox_status_returns_all_entries() {
        let (ctx, temp_dir) = create_app_context().await;
        let outbox_repo = SqlCipherOutboxRepository::new(Arc::clone(&ctx.db));

        outbox_repo
            .enqueue(&sample_outbox("entry-one", OutboxStatus::Pending, 1_700_000_000))
            .await
            .expect("enqueue first");
        outbox_repo
            .enqueue(&sample_outbox("entry-two", OutboxStatus::Failed, 1_700_000_100))
            .await
            .expect("enqueue second");

        let entries = fetch_outbox_entries(&ctx, None).await.expect("fetch entries");
        assert_eq!(entries.len(), 2);
        let ids: Vec<_> = entries.iter().map(|e| e.id.as_str()).collect();
        assert!(ids.contains(&"entry-one"));
        assert!(ids.contains(&"entry-two"));

        ctx.shutdown().await.expect("shutdown succeeds");
        drop(temp_dir);
    }

    fn sample_block(id: &str, start_ts: i64, status: &str) -> ProposedBlock {
        ProposedBlock {
            id: id.into(),
            start_ts,
            end_ts: start_ts + 1_800,
            duration_secs: 1_800,
            inferred_project_id: Some("PRJ-001".into()),
            inferred_wbs_code: Some("PRJ-001.001".into()),
            inferred_deal_name: Some("Project Alpha".into()),
            inferred_workstream: Some("Engineering".into()),
            billable: true,
            confidence: 0.95,
            classifier_used: None,
            activities: vec![ActivityBreakdown {
                name: "VSCode".into(),
                duration_secs: 1_200,
                percentage: 66.7,
            }],
            snapshot_ids: vec!["snap-1".into()],
            segment_ids: vec![],
            reasons: vec![],
            status: status.into(),
            created_at: start_ts,
            reviewed_at: None,
            total_idle_secs: 0,
            idle_handling: "exclude".into(),
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

    fn sample_outbox(id: &str, status: OutboxStatus, created_at: i64) -> TimeEntryOutbox {
        TimeEntryOutbox {
            id: id.into(),
            idempotency_key: format!("{id}-idem"),
            user_id: "user-123".into(),
            payload_json: "{}".into(),
            backend_cuid: None,
            status,
            attempts: 0,
            last_error: None,
            retry_after: None,
            created_at,
            sent_at: None,
            correlation_id: Some(Uuid::now_v7().to_string()),
            local_status: Some(status.to_string()),
            remote_status: None,
            sap_entry_id: None,
            next_attempt_at: None,
            error_code: None,
            last_forwarded_at: None,
            wbs_code: None,
            target: "sap".into(),
            description: None,
            auto_applied: false,
            version: 1,
            last_modified_by: "system".into(),
            last_modified_at: Some(created_at),
        }
    }

    async fn create_app_context() -> (Arc<AppContext>, TempDir) {
        std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

        let temp_dir =
            TempDir::new("suggestions-command-test").expect("failed to create temporary directory");
        let db_path = temp_dir.path().join("pulsearc.db");
        let lock_dir = temp_dir.create_dir("lock").expect("failed to create lock directory");
        let config = Config {
            database: DatabaseConfig {
                path: db_path.to_string_lossy().to_string(),
                pool_size: 5,
                encryption_key: None,
            },
            ..Config::default()
        };

        let ctx = AppContext::new_with_config_in_lock_dir(config, lock_dir)
            .await
            .expect("failed to create AppContext");

        (Arc::new(ctx), temp_dir)
    }
}
