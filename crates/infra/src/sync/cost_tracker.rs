//! Cost tracking for API usage and token consumption
//!
//! This module tracks API costs across services (OpenAI, SAP, Calendar, Neon)
//! with persistent storage and observability integration.
//!
//! # Features
//!
//! - Token usage tracking with cost calculation
//! - Monthly cost caps and alerts
//! - Variance tracking (estimated vs actual)
//! - Historical cost queries
//! - Integration with Phase 3F observability
//!
//! # Compliance
//!
//! - **CLAUDE.md §3**: Structured tracing only
//! - **Database**: SqlCipherConnection via DbManager
//! - **Thread-safety**: Arc<Mutex<>> for metrics

use std::sync::{Arc, Mutex};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use pulsearc_common::error::{CommonError, CommonResult};
use pulsearc_common::observability::MetricsTracker;

use crate::database::DbManager;

const THIRTY_DAYS_SECS: i64 = 30 * 86400;

/// Configuration for cost tracking
#[derive(Debug, Clone)]
pub struct CostRateConfig {
    /// Maximum monthly cost in USD (default: $5.00)
    pub max_monthly_cost_usd: f64,
    /// Alert threshold in USD (default: $4.00)
    pub alert_threshold_usd: f64,
    /// GPT-4o-mini input cost per 1M tokens (default: $0.15)
    pub gpt4o_mini_input_cost_per_1m_tokens: f64,
    /// GPT-4o-mini output cost per 1M tokens (default: $0.60)
    pub gpt4o_mini_output_cost_per_1m_tokens: f64,
}

impl Default for CostRateConfig {
    fn default() -> Self {
        Self {
            max_monthly_cost_usd: 5.0,
            alert_threshold_usd: 4.0,
            gpt4o_mini_input_cost_per_1m_tokens: 0.15,
            gpt4o_mini_output_cost_per_1m_tokens: 0.60,
        }
    }
}

/// Token usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub batch_id: String,
    pub user_id: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub timestamp: i64,
    pub is_actual: bool, // false = estimated, true = actual
}

/// Cost metrics (thread-safe)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CostMetrics {
    pub total_api_calls: u64,
    pub total_cost_usd: f64,
    pub openai_calls: u64,
    pub sap_calls: u64,
    pub calendar_calls: u64,
    pub neon_calls: u64,
}

/// Daily cost summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyCost {
    pub date: String, // YYYY-MM-DD
    pub total_cost_usd: f64,
    pub api_calls: u64,
}

/// User cost summary (30-day rolling window)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserCostSummary {
    pub batch_count: i64,
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
}

/// Token variance (estimated vs actual)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenVariance {
    pub input_variance_pct: f64,
    pub output_variance_pct: f64,
}

/// Classification mode based on cost caps
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassificationMode {
    OpenAI,    // Use OpenAI API
    RulesOnly, // Cost cap exceeded, use rule-based only
}

/// Cost tracker with persistent storage and observability
pub struct CostTracker {
    db: Arc<DbManager>,
    config: CostRateConfig,
    metrics: Arc<Mutex<CostMetrics>>,
    metrics_tracker: Arc<MetricsTracker>,
}

impl CostTracker {
    /// Create a new cost tracker
    ///
    /// # Arguments
    ///
    /// * `db` - Database manager for persistent storage
    /// * `config` - Cost rate configuration
    ///
    /// # Returns
    ///
    /// Configured cost tracker
    pub fn new(db: Arc<DbManager>, config: CostRateConfig) -> CommonResult<Self> {
        if config.max_monthly_cost_usd <= 0.0 {
            return Err(pulsearc_common::error::CommonError::config(
                "max_monthly_cost_usd must be positive",
            ));
        }

        let metrics_tracker = Arc::new(MetricsTracker::default());

        Ok(Self {
            db,
            config,
            metrics: Arc::new(Mutex::new(CostMetrics::default())),
            metrics_tracker,
        })
    }

    /// Create a new cost tracker with default configuration
    pub fn with_defaults(db: Arc<DbManager>) -> CommonResult<Self> {
        Self::new(db, CostRateConfig::default())
    }

    /// Calculate cost for given token usage
    ///
    /// # Arguments
    ///
    /// * `input_tokens` - Number of input tokens
    /// * `output_tokens` - Number of output tokens
    ///
    /// # Returns
    ///
    /// Estimated cost in USD
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (f64::from(input_tokens)
            * self.config.gpt4o_mini_input_cost_per_1m_tokens)
            / 1_000_000.0;
        let output_cost = (f64::from(output_tokens)
            * self.config.gpt4o_mini_output_cost_per_1m_tokens)
            / 1_000_000.0;
        input_cost + output_cost
    }

    /// Record API call (for non-token-based APIs like SAP, Calendar)
    ///
    /// # Arguments
    ///
    /// * `service` - Service name (e.g., "sap", "calendar", "neon")
    pub fn record_call(&self, service: &str) -> CommonResult<()> {
        let mut metrics = self
            .metrics
            .lock()
            .map_err(|_| CommonError::lock_resource("CostMetrics", "mutex poisoned"))?;

        metrics.total_api_calls += 1;

        match service {
            "openai" => metrics.openai_calls += 1,
            "sap" => metrics.sap_calls += 1,
            "calendar" => metrics.calendar_calls += 1,
            "neon" => metrics.neon_calls += 1,
            _ => {}
        }

        // Note: MetricsTracker integration would go here when API is available
        debug!(service = service, "Recorded API call");
        Ok(())
    }

    /// Get current metrics snapshot
    pub fn get_metrics(&self) -> CommonResult<CostMetrics> {
        self
            .metrics
            .lock()
            .map_err(|_| CommonError::lock_resource("CostMetrics", "mutex poisoned"))
            .map(|guard| guard.clone())
    }

    /// Record token usage to database
    ///
    /// # Arguments
    ///
    /// * `usage` - Token usage record
    ///
    /// # Errors
    ///
    /// Returns error if database operation fails
    pub async fn record_usage(&self, usage: &TokenUsage) -> CommonResult<()> {
        let db = Arc::clone(&self.db);
        let usage_clone = usage.clone();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            conn.execute(
                r#"
                INSERT INTO token_usage (
                    batch_id, user_id, input_tokens, output_tokens,
                    estimated_cost_usd, timestamp, is_actual
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                "#,
                rusqlite::params![
                    usage_clone.batch_id,
                    usage_clone.user_id,
                    usage_clone.input_tokens,
                    usage_clone.output_tokens,
                    usage_clone.estimated_cost_usd,
                    usage_clone.timestamp,
                    usage_clone.is_actual,
                ],
            )?;

            Ok(())
        })
        .await
        .map_err(|e| {
            pulsearc_common::error::CommonError::Internal(format!("Task join failed: {}", e))
        })??;

        // Update in-memory metrics
        let mut metrics = self
            .metrics
            .lock()
            .map_err(|_| CommonError::lock_resource("CostMetrics", "mutex poisoned"))?;
        metrics.total_cost_usd += usage.estimated_cost_usd;
        metrics.openai_calls += 1; // Assume token usage = OpenAI

        info!(
            batch_id = %usage.batch_id,
            cost_usd = usage.estimated_cost_usd,
            "Recorded token usage"
        );

        Ok(())
    }

    /// Get monthly cost for user (30-day rolling window)
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    ///
    /// # Returns
    ///
    /// Total cost in USD over last 30 days
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get_monthly_cost(&self, user_id: &str) -> CommonResult<f64> {
        let db = Arc::clone(&self.db);
        let user_id = user_id.to_string();
        let now = Utc::now().timestamp();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let thirty_days_ago = now - THIRTY_DAYS_SECS;

            let total: f64 = conn.query_row(
                r#"
                SELECT COALESCE(SUM(estimated_cost_usd), 0.0)
                FROM token_usage
                WHERE user_id = ?1 AND timestamp >= ?2
                "#,
                rusqlite::params![user_id, thirty_days_ago],
                |row| row.get(0),
            )?;

            Ok(total)
        })
        .await
        .map_err(|e| {
            pulsearc_common::error::CommonError::Internal(format!("Task join failed: {}", e))
        })?
    }

    /// Check if monthly cost cap is exceeded
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    ///
    /// # Returns
    ///
    /// `true` if cost cap exceeded
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn is_cost_cap_exceeded(&self, user_id: &str) -> CommonResult<bool> {
        let monthly_cost = self.get_monthly_cost(user_id).await?;
        let exceeded = monthly_cost >= self.config.max_monthly_cost_usd;

        if exceeded {
            warn!(
                user_id = user_id,
                monthly_cost = monthly_cost,
                cap = self.config.max_monthly_cost_usd,
                "Cost cap exceeded"
            );
        }

        Ok(exceeded)
    }

    /// Check if should alert (cost approaching cap)
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    ///
    /// # Returns
    ///
    /// `true` if should alert
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn should_alert_threshold(&self, user_id: &str) -> CommonResult<bool> {
        let monthly_cost = self.get_monthly_cost(user_id).await?;
        let should_alert = monthly_cost >= self.config.alert_threshold_usd
            && monthly_cost < self.config.max_monthly_cost_usd;

        if should_alert {
            warn!(
                user_id = user_id,
                monthly_cost = monthly_cost,
                threshold = self.config.alert_threshold_usd,
                "Cost approaching cap"
            );
        }

        Ok(should_alert)
    }

    /// Get classification mode based on cost cap
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    ///
    /// # Returns
    ///
    /// Classification mode (OpenAI or RulesOnly)
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get_classification_mode(&self, user_id: &str) -> CommonResult<ClassificationMode> {
        if self.is_cost_cap_exceeded(user_id).await? {
            Ok(ClassificationMode::RulesOnly)
        } else {
            Ok(ClassificationMode::OpenAI)
        }
    }

    /// Get user cost summary (30-day rolling window)
    ///
    /// # Arguments
    ///
    /// * `user_id` - User identifier
    ///
    /// # Returns
    ///
    /// Summary of token usage and costs
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get_user_cost_summary(&self, user_id: &str) -> CommonResult<UserCostSummary> {
        let db = Arc::clone(&self.db);
        let user_id = user_id.to_string();
        let now = Utc::now().timestamp();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let thirty_days_ago = now - THIRTY_DAYS_SECS;

            let (batch_count, total_input, total_output, total_cost): (i64, i64, i64, f64) = conn
                .query_row(
                r#"
                    SELECT COUNT(*),
                           COALESCE(SUM(input_tokens), 0),
                           COALESCE(SUM(output_tokens), 0),
                           COALESCE(SUM(estimated_cost_usd), 0.0)
                    FROM token_usage
                    WHERE user_id = ?1 AND timestamp >= ?2
                    "#,
                rusqlite::params![user_id, thirty_days_ago],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )?;

            Ok(UserCostSummary {
                batch_count,
                total_input_tokens: total_input as u64,
                total_output_tokens: total_output as u64,
                total_cost_usd: total_cost,
            })
        })
        .await
        .map_err(|e| {
            pulsearc_common::error::CommonError::Internal(format!("Task join failed: {}", e))
        })?
    }

    /// Get historical costs grouped by day
    ///
    /// # Arguments
    ///
    /// * `days` - Number of days of history (max 90)
    ///
    /// # Returns
    ///
    /// Vec of daily cost summaries
    ///
    /// # Errors
    ///
    /// Returns error if database query fails
    pub async fn get_historical_costs(&self, days: u32) -> CommonResult<Vec<DailyCost>> {
        let db = Arc::clone(&self.db);
        let now = Utc::now().timestamp();
        let days_clamped = days.min(90); // Max 90 days

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            let start = now - (i64::from(days_clamped) * 86400);

            let mut stmt = conn.prepare(
                r#"
                SELECT date(timestamp, 'unixepoch') as date,
                       SUM(estimated_cost_usd) as total_cost,
                       COUNT(*) as api_calls
                FROM token_usage
                WHERE timestamp >= ?1
                GROUP BY date
                ORDER BY date DESC
                "#,
            )?;

            let mut rows = stmt.query_map(rusqlite::params![start], |row| {
                Ok(DailyCost {
                    date: row.get(0)?,
                    total_cost_usd: row.get(1)?,
                    api_calls: row.get(2)?,
                })
            })?;

            let mut daily_costs = Vec::new();
            while let Some(row) = rows.next() {
                daily_costs.push(row?);
            }

            Ok(daily_costs)
        })
        .await
        .map_err(|e| {
            pulsearc_common::error::CommonError::Internal(format!("Task join failed: {}", e))
        })?
    }

    /// Get token variance (estimated vs actual)
    ///
    /// # Arguments
    ///
    /// * `batch_id` - Batch identifier
    ///
    /// # Returns
    ///
    /// Variance percentages for input and output tokens
    ///
    /// # Errors
    ///
    /// Returns error if no data found or database query fails
    pub async fn get_token_variance(&self, batch_id: &str) -> CommonResult<TokenVariance> {
        let db = Arc::clone(&self.db);
        let batch_id = batch_id.to_string();

        tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;

            let (est_input, est_output): (Option<i64>, Option<i64>) = conn.query_row(
                "SELECT input_tokens, output_tokens FROM token_usage WHERE batch_id = ?1 AND is_actual = 0 LIMIT 1",
                rusqlite::params![batch_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).map_err(|e| {
                pulsearc_common::error::CommonError::Storage(format!("No estimated usage: {}", e))
            })?;

            let (act_input, act_output): (Option<i64>, Option<i64>) = conn.query_row(
                "SELECT input_tokens, output_tokens FROM token_usage WHERE batch_id = ?1 AND is_actual = 1 LIMIT 1",
                rusqlite::params![batch_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            ).map_err(|e| {
                pulsearc_common::error::CommonError::Storage(format!("No actual usage: {}", e))
            })?;

            let input_variance_pct = if let (Some(est), Some(act)) = (est_input, act_input) {
                if est == 0 {
                    0.0
                } else {
                    ((act - est) as f64 / est as f64) * 100.0
                }
            } else {
                0.0
            };

            let output_variance_pct = if let (Some(est), Some(act)) = (est_output, act_output) {
                if est == 0 {
                    0.0
                } else {
                    ((act - est) as f64 / est as f64) * 100.0
                }
            } else {
                0.0
            };

            Ok(TokenVariance {
                input_variance_pct,
                output_variance_pct,
            })
        })
        .await
        .map_err(|e| {
            pulsearc_common::error::CommonError::Internal(format!("Task join failed: {}", e))
        })?
    }

    /// Check if variance exceeds threshold (20%)
    ///
    /// # Arguments
    ///
    /// * `batch_id` - Batch identifier
    ///
    /// # Returns
    ///
    /// `true` if variance >20% on input or output
    ///
    /// # Errors
    ///
    /// Returns error if no data found or database query fails
    pub async fn should_alert_variance(&self, batch_id: &str) -> CommonResult<bool> {
        let variance = self.get_token_variance(batch_id).await?;
        let should_alert =
            variance.input_variance_pct.abs() > 20.0 || variance.output_variance_pct.abs() > 20.0;

        if should_alert {
            warn!(
                batch_id = batch_id,
                input_variance = variance.input_variance_pct,
                output_variance = variance.output_variance_pct,
                "Token variance exceeded threshold"
            );
        }

        Ok(should_alert)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cost() {
        let config = CostRateConfig::default();
        let db = Arc::new(DbManager::new(":memory:", 1, Some("test-key")).unwrap());
        let tracker = CostTracker::new(db, config).unwrap();

        // 1M input tokens + 1M output tokens
        let cost = tracker.calculate_cost(1_000_000, 1_000_000);
        assert!((cost - 0.75).abs() < 0.001); // $0.15 + $0.60 = $0.75
    }

    #[test]
    fn test_record_call() {
        let db = Arc::new(DbManager::new(":memory:", 1, Some("test-key")).unwrap());
        let tracker = CostTracker::with_defaults(db).unwrap();

        tracker.record_call("sap").unwrap();
        tracker.record_call("openai").unwrap();

        let metrics = tracker.get_metrics().unwrap();
        assert_eq!(metrics.total_api_calls, 2);
        assert_eq!(metrics.sap_calls, 1);
        assert_eq!(metrics.openai_calls, 1);
    }

    #[test]
    fn test_cost_config_validation() {
        let db = Arc::new(DbManager::new(":memory:", 1, Some("test-key")).unwrap());
        let config = CostRateConfig {
            max_monthly_cost_usd: -1.0, // Invalid
            ..Default::default()
        };

        let result = CostTracker::new(db, config);
        assert!(result.is_err());
    }
}
