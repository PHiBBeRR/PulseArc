//! Database commands for Phase 4A.1 migration
//!
//! These commands provide database statistics, health checks, and maintenance
//! operations. All commands support feature flag toggling between new
//! (hexagonal architecture) and legacy implementations.

use std::sync::Arc;
use std::time::Instant;

use chrono::{Duration, Utc};
use pulsearc_domain::types::database::ActivitySnapshot;
use pulsearc_domain::types::stats::{BatchStats, DatabaseStats};
use pulsearc_domain::types::HealthStatus;
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use rusqlite::types::ToSql;
use tauri::State;
use tracing::info;

use crate::adapters::database_stats::build_database_stats;
use crate::context::AppContext;
use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};

// =============================================================================
// Command 1: get_database_stats
// =============================================================================

/// Get database statistics (snapshot counts, segment counts, batch stats).
///
/// This command aggregates database statistics for UI display. It supports
/// feature flag toggling between the new hexagonal architecture and legacy
/// implementation.
///
/// # Feature Flag
///
/// Controlled by `new_database_commands` flag (default: disabled, uses legacy).
#[tauri::command]
pub async fn get_database_stats(ctx: State<'_, Arc<AppContext>>) -> Result<DatabaseStats, String> {
    let command_name = "database::get_database_stats";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag (fail-safe: use legacy on error)
    let use_new =
        app_ctx.feature_flags.is_enabled("new_database_commands", false).await.unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };
    info!(command = command_name, implementation, "Executing get_database_stats");

    let result = if use_new {
        new_get_database_stats(&app_ctx).await
    } else {
        legacy_get_database_stats(&app_ctx).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
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

    result.map_err(|e| e.to_string())
}

async fn new_get_database_stats(ctx: &AppContext) -> DomainResult<DatabaseStats> {
    // Use adapter to build legacy DatabaseStats from new DatabaseStatsPort
    build_database_stats(&ctx.database_stats).await
}

#[allow(dead_code)] // Will be removed in Phase 5
async fn legacy_get_database_stats(ctx: &AppContext) -> DomainResult<DatabaseStats> {
    // Copy from legacy/api/src/commands/database.rs::get_database_stats_new (lines
    // 44-105)
    let db = ctx.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db
            .get_connection()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))?;

        // Count total snapshots
        let snapshot_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM activity_snapshots", &[], |row| row.get(0))
            .map_err(|e| PulseArcError::Database(format!("Failed to count snapshots: {}", e)))?;

        // Count unprocessed snapshots
        let unprocessed_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM activity_snapshots WHERE processed = 0", &[], |row| {
                row.get(0)
            })
            .map_err(|e| {
                PulseArcError::Database(format!("Failed to count unprocessed snapshots: {}", e))
            })?;

        // Count segments
        let segment_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM activity_segments", &[], |row| row.get(0))
            .map_err(|e| PulseArcError::Database(format!("Failed to count segments: {}", e)))?;

        // batch_stats: Not populated in legacy (matches legacy implementation)
        let batch_stats = BatchStats { pending: 0, processing: 0, completed: 0, failed: 0 };

        Ok(DatabaseStats { snapshot_count, unprocessed_count, segment_count, batch_stats })
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}

// =============================================================================
// Command 2: get_recent_snapshots
// =============================================================================

/// Get recent unprocessed snapshots.
///
/// Returns the N most recent unprocessed activity snapshots for UI display.
/// Orders by timestamp descending (most recent first).
///
/// # Feature Flag
///
/// Controlled by `new_database_commands` flag (default: disabled, uses legacy).
#[tauri::command]
pub async fn get_recent_snapshots(
    ctx: State<'_, Arc<AppContext>>,
    limit: i32,
) -> Result<Vec<ActivitySnapshot>, String> {
    let command_name = "database::get_recent_snapshots";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag
    let use_new =
        app_ctx.feature_flags.is_enabled("new_database_commands", false).await.unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };
    info!(command = command_name, implementation, limit, "Executing get_recent_snapshots");

    let result = if use_new {
        new_get_recent_snapshots(&app_ctx, limit).await
    } else {
        legacy_get_recent_snapshots(&app_ctx, limit).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
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

    result.map_err(|e| e.to_string())
}

async fn new_get_recent_snapshots(
    ctx: &AppContext,
    limit: i32,
) -> DomainResult<Vec<ActivitySnapshot>> {
    // Use SqlCipherActivityRepository to query recent snapshots
    // Query last 30 days and filter to unprocessed only
    let end = Utc::now();
    let start = end - Duration::days(30);

    let db = ctx.db.clone();
    let limit = limit as usize;

    tokio::task::spawn_blocking(move || {
        let conn = db
            .get_connection()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))?;

        // Query unprocessed snapshots ordered by timestamp DESC
        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, activity_context_json, detected_activity, work_type,
                    activity_category, primary_app, processed, batch_id, created_at,
                    processed_at, is_idle, idle_duration_secs
             FROM activity_snapshots
             WHERE processed = 0 AND timestamp >= ?1 AND timestamp < ?2
             ORDER BY timestamp DESC
             LIMIT ?3",
            )
            .map_err(|e| PulseArcError::Database(format!("Failed to prepare query: {}", e)))?;

        let start_ts = start.timestamp();
        let end_ts = end.timestamp();
        let limit_i64 = limit as i64;

        let params: &[&dyn ToSql] = &[&start_ts, &end_ts, &limit_i64];
        let snapshots = stmt
            .query_map(params, |row| {
                Ok(ActivitySnapshot {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    activity_context_json: row.get(2)?,
                    detected_activity: row.get(3)?,
                    work_type: row.get(4)?,
                    activity_category: row.get(5)?,
                    primary_app: row.get(6)?,
                    processed: row.get(7)?,
                    batch_id: row.get(8)?,
                    created_at: row.get(9)?,
                    processed_at: row.get(10)?,
                    is_idle: row.get(11)?,
                    idle_duration_secs: row.get(12)?,
                })
            })
            .map_err(|e| PulseArcError::Database(format!("Failed to query snapshots: {}", e)))?;

        Ok(snapshots)
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}

#[allow(dead_code)] // Will be removed in Phase 5
async fn legacy_get_recent_snapshots(
    ctx: &AppContext,
    limit: i32,
) -> DomainResult<Vec<ActivitySnapshot>> {
    // Copy from legacy/api/src/commands/database.rs::get_recent_snapshots (lines
    // 166-210)
    let db = ctx.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db
            .get_connection()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, timestamp, activity_context_json, detected_activity, work_type,
                    activity_category, primary_app, processed, batch_id, created_at,
                    processed_at, is_idle, idle_duration_secs
             FROM activity_snapshots
             WHERE processed = 0
             ORDER BY timestamp DESC
             LIMIT ?",
            )
            .map_err(|e| PulseArcError::Database(format!("Failed to prepare query: {}", e)))?;

        let params: &[&dyn ToSql] = &[&limit];
        let snapshots = stmt
            .query_map(params, |row| {
                Ok(ActivitySnapshot {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    activity_context_json: row.get(2)?,
                    detected_activity: row.get(3)?,
                    work_type: row.get(4)?,
                    activity_category: row.get(5)?,
                    primary_app: row.get(6)?,
                    processed: row.get(7)?,
                    batch_id: row.get(8)?,
                    created_at: row.get(9)?,
                    processed_at: row.get(10)?,
                    is_idle: row.get(11)?,
                    idle_duration_secs: row.get(12)?,
                })
            })
            .map_err(|e| PulseArcError::Database(format!("Failed to query snapshots: {}", e)))?;

        Ok(snapshots)
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}

// =============================================================================
// Command 3: vacuum_database
// =============================================================================

/// Run VACUUM to reclaim unused database space.
///
/// This command rebuilds the database file to remove fragmentation. It's a
/// safe but potentially slow operation that locks the database during
/// execution.
///
/// # Feature Flag
///
/// Controlled by `new_database_commands` flag (default: disabled, uses legacy).
#[tauri::command]
pub async fn vacuum_database(ctx: State<'_, Arc<AppContext>>) -> Result<(), String> {
    let command_name = "database::vacuum_database";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag
    let use_new =
        app_ctx.feature_flags.is_enabled("new_database_commands", false).await.unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };
    info!(command = command_name, implementation, "Executing vacuum_database");

    let result = if use_new {
        app_ctx.database_stats.vacuum_database().await
    } else {
        legacy_vacuum_database(&app_ctx).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
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

    result.map_err(|e| e.to_string())
}

#[allow(dead_code)] // Will be removed in Phase 5
async fn legacy_vacuum_database(ctx: &AppContext) -> DomainResult<()> {
    let db = ctx.db.clone();

    tokio::task::spawn_blocking(move || {
        let conn = db
            .get_connection()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))?;

        conn.execute("VACUUM", [])
            .map_err(|e| PulseArcError::Database(format!("VACUUM failed: {}", e)))?;

        Ok(())
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}

// =============================================================================
// Command 4: get_database_health
// =============================================================================

/// Check database health with a simple connectivity test.
///
/// Executes a trivial query (SELECT 1) to verify the database is responsive.
/// Returns timing information for monitoring purposes.
///
/// # Feature Flag
///
/// Controlled by `new_database_commands` flag (default: disabled, uses legacy).
#[tauri::command]
pub async fn get_database_health(ctx: State<'_, Arc<AppContext>>) -> Result<HealthStatus, String> {
    let command_name = "database::get_database_health";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag
    let use_new =
        app_ctx.feature_flags.is_enabled("new_database_commands", false).await.unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };
    info!(command = command_name, implementation, "Executing get_database_health");

    let result = if use_new {
        app_ctx.database_stats.check_database_health().await
    } else {
        legacy_get_database_health(&app_ctx).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
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

    result.map_err(|e| e.to_string())
}

#[allow(dead_code)] // Will be removed in Phase 5
async fn legacy_get_database_health(ctx: &AppContext) -> DomainResult<HealthStatus> {
    let db = ctx.db.clone();

    tokio::task::spawn_blocking(move || {
        let query_start = Instant::now();

        match db.get_connection() {
            Ok(conn) => {
                // Simple connectivity test
                match conn.query_row("SELECT 1", &[], |row| row.get::<_, i32>(0)) {
                    Ok(1) => {
                        let response_time_ms = query_start.elapsed().as_millis() as u64;
                        Ok(HealthStatus {
                            is_healthy: true,
                            message: "Database is healthy".to_string(),
                            response_time_ms,
                        })
                    }
                    Ok(_) => Ok(HealthStatus {
                        is_healthy: false,
                        message: "Unexpected query result".to_string(),
                        response_time_ms: query_start.elapsed().as_millis() as u64,
                    }),
                    Err(err) => Ok(HealthStatus {
                        is_healthy: false,
                        message: format!("Query failed: {}", err),
                        response_time_ms: query_start.elapsed().as_millis() as u64,
                    }),
                }
            }
            Err(err) => Ok(HealthStatus {
                is_healthy: false,
                message: format!("Connection failed: {}", err),
                response_time_ms: query_start.elapsed().as_millis() as u64,
            }),
        }
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}
