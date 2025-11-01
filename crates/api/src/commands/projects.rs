//! Project management commands

use std::sync::Arc;
use std::time::Instant;

use pulsearc_domain::types::database::Project;
use pulsearc_domain::{PulseArcError, Result};
use tauri::State;
use tokio::task;
use tracing::{info, warn};

use crate::context::AppContext;
use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};

const MAX_PROJECT_RESULTS: i64 = 500;
const PROJECT_QUERY: &str = r#"
    SELECT
        wc.project_def,
        COALESCE(
            NULLIF(wc.project_name, ''),
            NULLIF(wc.deal_name, ''),
            wc.project_def
        ) AS project_name
    FROM wbs_cache wc
    WHERE wc.project_def IS NOT NULL
      AND TRIM(wc.project_def) != ''
      AND wc.cached_at = (
          SELECT MAX(inner_wc.cached_at)
          FROM wbs_cache AS inner_wc
          WHERE inner_wc.project_def = wc.project_def
      )
      AND wc.status IN ('REL', 'TECO', 'CLSD')
    GROUP BY wc.project_def, wc.project_name, wc.deal_name, wc.cached_at
    ORDER BY wc.cached_at DESC
    LIMIT ?1
"#;

/// Get list of user projects
#[tauri::command]
pub async fn get_user_projects(ctx: State<'_, Arc<AppContext>>) -> Result<Vec<Project>> {
    let command_name = "projects::get_user_projects";
    let implementation = "new";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    info!(command = command_name, "Fetching user projects");

    let result = fetch_user_projects(&app_ctx).await;
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

async fn fetch_user_projects(ctx: &Arc<AppContext>) -> Result<Vec<Project>> {
    let db = Arc::clone(&ctx.db);

    task::spawn_blocking(move || {
        let conn = db.get_connection()?;
        let mut stmt = conn
            .inner()
            .prepare(PROJECT_QUERY)
            .map_err(|err| map_rusqlite_error("prepare project query", err))?;
        let rows = stmt
            .query_map(rusqlite::params![MAX_PROJECT_RESULTS], |row| {
                Ok(Project { id: row.get::<_, String>(0)?, name: row.get::<_, String>(1)? })
            })
            .map_err(|err| map_rusqlite_error("query projects", err))?;

        let mut projects = Vec::new();
        for row in rows {
            projects.push(row.map_err(|err| map_rusqlite_error("map project row", err))?);
        }

        if projects.is_empty() {
            warn!(
                "Project query returned no results (maybe first run before Neon sync?). \
                 Returning empty list."
            );
        }

        Ok(projects)
    })
    .await
    .map_err(|err| PulseArcError::Internal(format!("project query task failed: {}", err)))?
}

fn map_rusqlite_error(context: &str, err: rusqlite::Error) -> PulseArcError {
    PulseArcError::Database(format!("{context} failed: {err}"))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::Utc;
    use pulsearc_common::testing::TempDir;
    use pulsearc_domain::{Config, DatabaseConfig};
    use tokio::task;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_fetch_user_projects_returns_entries() {
        std::env::set_var("TEST_DATABASE_ENCRYPTION_KEY", TEST_KEY);

        let temp_dir =
            TempDir::new("projects-command-test").expect("failed to create temporary directory");
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

        let ctx = Arc::new(
            AppContext::new_with_config_in_lock_dir(config, lock_dir)
                .await
                .expect("failed to initialise AppContext"),
        );

        let db = Arc::clone(&ctx.db);
        let inserted = task::spawn_blocking(move || {
            let conn = db.get_connection().expect("connection");
            let now = Utc::now().timestamp();

            conn.execute(
                "INSERT INTO wbs_cache (
                    wbs_code, project_def, project_name, description, status, cached_at,
                    expires_at, last_changed_at, opportunity_id, deal_name, target_company_name,
                    counterparty, industry, region, amount, stage_name, project_code
                ) VALUES (?1, ?2, ?3, NULL, 'REL', ?4, ?5, NULL, NULL, ?6, NULL, NULL, NULL, NULL, NULL, NULL, NULL)",
                rusqlite::params![
                    "WBS-001.001",
                    "PRJ-001",
                    "Project Alpha",
                    now,
                    now + 86_400,
                    "Alpha Deal"
                ],
            )
            .expect("insert first project");

            conn.execute(
                "INSERT INTO wbs_cache (
                    wbs_code, project_def, project_name, description, status, cached_at,
                    expires_at, last_changed_at, opportunity_id, deal_name, target_company_name,
                    counterparty, industry, region, amount, stage_name, project_code
                ) VALUES (?1, ?2, NULL, NULL, 'REL', ?3, ?4, NULL, NULL, ?5, NULL, NULL, NULL, NULL, NULL, NULL, NULL)",
                rusqlite::params![
                    "WBS-002.001",
                    "PRJ-002",
                    now - 60,
                    now + 86_400,
                    "Deal Beta"
                ],
            )
            .expect("insert second project");
        })
        .await;

        assert!(inserted.is_ok(), "failed to insert test data");

        let projects = fetch_user_projects(&ctx).await.expect("fetch projects failed");
        assert_eq!(projects.len(), 2);

        let project_alpha = projects.iter().find(|p| p.id == "PRJ-001").expect("PRJ-001 missing");
        assert_eq!(project_alpha.name, "Project Alpha");

        let project_beta = projects.iter().find(|p| p.id == "PRJ-002").expect("PRJ-002 missing");
        assert_eq!(project_beta.name, "Deal Beta");

        ctx.shutdown().await.expect("shutdown should succeed");
    }
}
