//! SQLCipher-backed command metrics repository.
//!
//! Tracks command execution metrics for Phase 4 migration validation.
//! Supports querying statistics, calculating percentiles, and comparing
//! legacy vs new implementation performance.

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_common::storage::types::Connection as ConnectionTrait;
use pulsearc_core::command_metrics_ports::{CommandMetric, CommandMetricsPort, CommandStats};
use pulsearc_domain::{PulseArcError, Result as DomainResult};
use tokio::task;
use tracing::{debug, warn};

use super::manager::DbManager;

/// Command metrics repository backed by SQLCipher.
pub struct SqlCipherCommandMetricsRepository {
    db: Arc<DbManager>,
}

impl SqlCipherCommandMetricsRepository {
    /// Construct a repository backed by the shared database manager.
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl CommandMetricsPort for SqlCipherCommandMetricsRepository {
    async fn record_execution(&self, metric: CommandMetric) -> DomainResult<()> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;

            conn.execute(
                "INSERT INTO command_metrics (
                    id, command, implementation, timestamp, duration_ms, success, error_type
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                &[
                    &metric.id as &dyn rusqlite::ToSql,
                    &metric.command,
                    &metric.implementation,
                    &metric.timestamp,
                    &(metric.duration_ms as i64),
                    &metric.success,
                    &metric.error_type,
                ],
            )
            .map_err(map_storage_error)?;

            debug!(
                command = %metric.command,
                implementation = %metric.implementation,
                duration_ms = metric.duration_ms,
                success = metric.success,
                "Recorded command metric"
            );

            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_stats(
        &self,
        command: &str,
        implementation: Option<&str>,
        start_ts: i64,
        end_ts: i64,
    ) -> DomainResult<CommandStats> {
        let command = command.to_string();
        let implementation_filter = implementation.map(String::from);
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<CommandStats> {
            let conn = db.get_connection()?;

            // Build query with optional implementation filter
            let (query, params): (&str, Vec<Box<dyn rusqlite::ToSql>>) =
                if let Some(impl_name) = &implementation_filter {
                    (
                        "SELECT COUNT(*) as total,
                            SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                            SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as error_count,
                            AVG(duration_ms) as avg_latency
                     FROM command_metrics
                     WHERE command = ?1 AND implementation = ?2
                       AND timestamp >= ?3 AND timestamp <= ?4",
                        vec![
                            Box::new(command.clone()) as Box<dyn rusqlite::ToSql>,
                            Box::new(impl_name.clone()),
                            Box::new(start_ts),
                            Box::new(end_ts),
                        ],
                    )
                } else {
                    (
                        "SELECT COUNT(*) as total,
                            SUM(CASE WHEN success = 1 THEN 1 ELSE 0 END) as success_count,
                            SUM(CASE WHEN success = 0 THEN 1 ELSE 0 END) as error_count,
                            AVG(duration_ms) as avg_latency
                     FROM command_metrics
                     WHERE command = ?1 AND timestamp >= ?2 AND timestamp <= ?3",
                        vec![
                            Box::new(command.clone()) as Box<dyn rusqlite::ToSql>,
                            Box::new(start_ts),
                            Box::new(end_ts),
                        ],
                    )
                };

            let mut stmt = conn.prepare(query).map_err(map_storage_error)?;

            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

            // query_map returns Vec<T>, so we need to take the first result
            let results = stmt
                .query_map(&param_refs[..], |row| {
                    Ok((
                        row.get::<_, i64>(0)? as u64,
                        row.get::<_, Option<i64>>(1)?.unwrap_or(0) as u64,
                        row.get::<_, Option<i64>>(2)?.unwrap_or(0) as u64,
                        row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                    ))
                })
                .map_err(map_storage_error)?;

            let (total_count, success_count, error_count, avg_latency): (u64, u64, u64, f64) =
                results.into_iter().next().ok_or_else(|| {
                    PulseArcError::Internal("no aggregation results found".into())
                })?;

            // Calculate percentiles (P50, P95, P99)
            let percentiles = if total_count > 0 {
                calculate_percentiles(
                    &conn,
                    &command,
                    implementation_filter.as_deref(),
                    start_ts,
                    end_ts,
                )?
            } else {
                (0, 0, 0)
            };

            let error_rate =
                if total_count > 0 { error_count as f64 / total_count as f64 } else { 0.0 };

            Ok(CommandStats {
                command: command.clone(),
                implementation: implementation_filter.unwrap_or_else(|| "all".to_string()),
                total_count,
                success_count,
                error_count,
                error_rate,
                p50_latency_ms: percentiles.0,
                p95_latency_ms: percentiles.1,
                p99_latency_ms: percentiles.2,
                avg_latency_ms: avg_latency,
            })
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_recent_executions(
        &self,
        command: &str,
        limit: usize,
    ) -> DomainResult<Vec<CommandMetric>> {
        let command = command.to_string();
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Vec<CommandMetric>> {
            let conn = db.get_connection()?;

            let mut stmt = conn
                .prepare(
                    "SELECT id, command, implementation, timestamp, duration_ms, success, error_type
                     FROM command_metrics
                     WHERE command = ?1
                     ORDER BY timestamp DESC
                     LIMIT ?2",
                )
                .map_err(map_storage_error)?;

            let metrics = stmt
                .query_map(&[&command as &dyn rusqlite::ToSql, &(limit as i64)], |row| {
                    Ok(CommandMetric {
                        id: row.get(0)?,
                        command: row.get(1)?,
                        implementation: row.get(2)?,
                        timestamp: row.get(3)?,
                        duration_ms: row.get::<_, i64>(4)? as u64,
                        success: row.get(5)?,
                        error_type: row.get(6)?,
                    })
                })
                .map_err(map_storage_error)?;

            Ok(metrics)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn compare_implementations(
        &self,
        command: &str,
        start_ts: i64,
        end_ts: i64,
    ) -> DomainResult<(CommandStats, CommandStats)> {
        // Get stats for both implementations in parallel
        let legacy_stats = self.get_stats(command, Some("legacy"), start_ts, end_ts);
        let new_stats = self.get_stats(command, Some("new"), start_ts, end_ts);

        let (legacy, new_impl) = tokio::try_join!(legacy_stats, new_stats)?;

        Ok((legacy, new_impl))
    }

    async fn cleanup_old_metrics(&self, older_than_ts: i64) -> DomainResult<u64> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<u64> {
            let conn = db.get_connection()?;

            let deleted = conn
                .execute(
                    "DELETE FROM command_metrics WHERE timestamp < ?1",
                    &[&older_than_ts as &dyn rusqlite::ToSql],
                )
                .map_err(map_storage_error)?;

            debug!(deleted_count = deleted, older_than_ts, "Cleaned up old command metrics");

            Ok(deleted as u64)
        })
        .await
        .map_err(map_join_error)?
    }
}

/// Calculate percentiles (P50, P95, P99) for command latency
type LatencyPercentiles = (u64, u64, u64);

fn calculate_percentiles(
    conn: &SqlCipherConnection,
    command: &str,
    implementation: Option<&str>,
    start_ts: i64,
    end_ts: i64,
) -> DomainResult<LatencyPercentiles> {
    type QueryParts = (&'static str, Vec<Box<dyn rusqlite::ToSql>>);

    // Build query with optional implementation filter
    let query_parts: QueryParts = if let Some(impl_name) = implementation {
        (
            "SELECT duration_ms FROM command_metrics
             WHERE command = ?1 AND implementation = ?2
               AND timestamp >= ?3 AND timestamp <= ?4
             ORDER BY duration_ms",
            vec![
                Box::new(command.to_string()) as Box<dyn rusqlite::ToSql>,
                Box::new(impl_name.to_string()),
                Box::new(start_ts),
                Box::new(end_ts),
            ],
        )
    } else {
        (
            "SELECT duration_ms FROM command_metrics
             WHERE command = ?1 AND timestamp >= ?2 AND timestamp <= ?3
             ORDER BY duration_ms",
            vec![
                Box::new(command.to_string()) as Box<dyn rusqlite::ToSql>,
                Box::new(start_ts),
                Box::new(end_ts),
            ],
        )
    };

    let (query, params) = query_parts;

    let mut stmt = conn.prepare(query).map_err(map_storage_error)?;

    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    // query_map returns Vec<u64> directly, not an iterator
    let durations: Vec<u64> = stmt
        .query_map(&param_refs[..], |row| Ok(row.get::<_, i64>(0)? as u64))
        .map_err(map_storage_error)?;

    if durations.is_empty() {
        return Ok((0, 0, 0));
    }

    let len = durations.len();
    let p50 = durations.get(len * 50 / 100).copied().unwrap_or(0);
    let p95 = durations.get(len * 95 / 100).copied().unwrap_or(0);
    let p99 = durations.get(len * 99 / 100).copied().unwrap_or(0);

    Ok((p50, p95, p99))
}

/// Map storage errors to domain errors
fn map_storage_error(err: StorageError) -> PulseArcError {
    warn!(error = %err, "Command metrics storage error");
    PulseArcError::Database(err.to_string())
}

/// Map tokio join errors to domain errors
fn map_join_error(err: tokio::task::JoinError) -> PulseArcError {
    warn!(error = %err, "Command metrics task join error");
    PulseArcError::Internal(format!("Task join failed: {}", err))
}

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use super::*;

    async fn setup_test_db() -> (Arc<DbManager>, SqlCipherCommandMetricsRepository) {
        // Create a unique temporary database for each test to avoid conflicts
        let temp_dir = std::env::temp_dir();
        let test_db_path = temp_dir.join(format!("command_metrics_test_{}.db", Uuid::new_v4()));

        let db = Arc::new(
            DbManager::new(
                test_db_path.to_str().unwrap(),
                5,
                Some("test-key-32-bytes-long-for-aes"),
            )
            .expect("Failed to create DbManager"),
        );
        db.run_migrations().expect("Failed to run migrations");
        let repo = SqlCipherCommandMetricsRepository::new(Arc::clone(&db));
        (db, repo)
    }

    #[tokio::test]
    async fn test_record_and_retrieve_metric() {
        let (_db, repo) = setup_test_db().await;

        let metric = CommandMetric {
            id: Uuid::new_v4().to_string(),
            command: "test::command".to_string(),
            implementation: "new".to_string(),
            timestamp: 1000,
            duration_ms: 150,
            success: true,
            error_type: None,
        };

        repo.record_execution(metric.clone()).await.expect("Failed to record metric");

        let recent = repo
            .get_recent_executions("test::command", 10)
            .await
            .expect("Failed to get recent executions");

        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].command, "test::command");
        assert_eq!(recent[0].duration_ms, 150);
    }

    #[tokio::test]
    async fn test_calculate_stats() {
        let (_db, repo) = setup_test_db().await;

        // Record multiple executions
        for i in 0..10 {
            let metric = CommandMetric {
                id: Uuid::new_v4().to_string(),
                command: "stats::test".to_string(),
                implementation: "new".to_string(),
                timestamp: 1000 + i,
                duration_ms: 100 + (i as u64 * 10),
                success: i < 8, // 8/10 success
                error_type: if i >= 8 { Some("TestError".to_string()) } else { None },
            };
            repo.record_execution(metric).await.expect("Failed to record metric");
        }

        let stats = repo
            .get_stats("stats::test", Some("new"), 1000, 2000)
            .await
            .expect("Failed to get stats");

        assert_eq!(stats.total_count, 10);
        assert_eq!(stats.success_count, 8);
        assert_eq!(stats.error_count, 2);
        assert_eq!(stats.error_rate, 0.2);
    }

    #[tokio::test]
    async fn test_compare_implementations() {
        let (_db, repo) = setup_test_db().await;

        // Record legacy executions (slower, more errors)
        for i in 0..5 {
            let metric = CommandMetric {
                id: Uuid::new_v4().to_string(),
                command: "compare::test".to_string(),
                implementation: "legacy".to_string(),
                timestamp: 1000 + i,
                duration_ms: 200 + (i as u64 * 10),
                success: i < 3,
                error_type: if i >= 3 { Some("LegacyError".to_string()) } else { None },
            };
            repo.record_execution(metric).await.expect("Failed to record metric");
        }

        // Record new executions (faster, fewer errors)
        for i in 0..5 {
            let metric = CommandMetric {
                id: Uuid::new_v4().to_string(),
                command: "compare::test".to_string(),
                implementation: "new".to_string(),
                timestamp: 1000 + i,
                duration_ms: 100 + (i as u64 * 5),
                success: i < 4,
                error_type: if i >= 4 { Some("NewError".to_string()) } else { None },
            };
            repo.record_execution(metric).await.expect("Failed to record metric");
        }

        let (legacy_stats, new_stats) = repo
            .compare_implementations("compare::test", 1000, 2000)
            .await
            .expect("Failed to compare implementations");

        assert_eq!(legacy_stats.total_count, 5);
        assert_eq!(new_stats.total_count, 5);
        assert!(legacy_stats.avg_latency_ms > new_stats.avg_latency_ms);
        assert!(legacy_stats.error_rate > new_stats.error_rate);
    }

    #[tokio::test]
    async fn test_cleanup_old_metrics() {
        let (_db, repo) = setup_test_db().await;

        // Record old metrics
        for i in 0..5 {
            let metric = CommandMetric {
                id: Uuid::new_v4().to_string(),
                command: "cleanup::test".to_string(),
                implementation: "new".to_string(),
                timestamp: 100 + i, // Old timestamps
                duration_ms: 100,
                success: true,
                error_type: None,
            };
            repo.record_execution(metric).await.expect("Failed to record metric");
        }

        // Record recent metrics
        for i in 0..5 {
            let metric = CommandMetric {
                id: Uuid::new_v4().to_string(),
                command: "cleanup::test".to_string(),
                implementation: "new".to_string(),
                timestamp: 1000 + i, // Recent timestamps
                duration_ms: 100,
                success: true,
                error_type: None,
            };
            repo.record_execution(metric).await.expect("Failed to record metric");
        }

        // Clean up metrics older than timestamp 500
        let deleted = repo.cleanup_old_metrics(500).await.expect("Failed to cleanup");

        assert_eq!(deleted, 5); // Should delete the 5 old metrics

        let recent =
            repo.get_recent_executions("cleanup::test", 20).await.expect("Failed to get recent");

        assert_eq!(recent.len(), 5); // Only recent metrics remain
    }
}
