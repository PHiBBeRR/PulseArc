//! API batch forwarder with partial success handling
//!
//! Provides batch submission of segments and snapshots with resilience
//! patterns and partial success handling.

use std::sync::Arc;

use pulsearc_domain::types::{ActivitySegment, ActivitySnapshot};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;
use tracing::{debug, info, instrument, warn};

use super::commands::ApiCommands;
use super::errors::ApiError;

/// Type alias for task list to avoid complexity warnings
type TaskList = Vec<(usize, JoinHandle<Result<(), ApiError>>)>;

/// Configuration for batch forwarder
#[derive(Debug, Clone)]
pub struct ForwarderConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Maximum parallel submissions
    pub max_parallel: usize,
}

impl Default for ForwarderConfig {
    fn default() -> Self {
        Self { max_batch_size: 50, max_parallel: 5 }
    }
}

/// Result of a batch submission
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSubmissionResult {
    /// Number of items successfully submitted
    pub submitted: usize,
    /// Number of items that failed
    pub failed: usize,
    /// Errors encountered (item index, error message)
    pub errors: Vec<(usize, String)>,
}

/// API forwarder for batch operations
pub struct ApiForwarder {
    commands: Arc<ApiCommands>,
    config: ForwarderConfig,
}

impl ApiForwarder {
    /// Create a new API forwarder
    ///
    /// # Arguments
    ///
    /// * `commands` - API commands instance
    /// * `config` - Forwarder configuration
    pub fn new(commands: Arc<ApiCommands>, config: ForwarderConfig) -> Self {
        Self { commands, config }
    }

    /// Forward a batch of segments to the API
    ///
    /// # Arguments
    ///
    /// * `segments` - Segments to forward
    ///
    /// # Returns
    ///
    /// Batch submission result with success/failure counts
    ///
    /// # Errors
    ///
    /// Returns error only if all submissions fail
    #[instrument(skip(self, segments), fields(count = segments.len()))]
    pub async fn forward_segments(
        &self,
        segments: Vec<ActivitySegment>,
    ) -> Result<BatchSubmissionResult, ApiError> {
        if segments.is_empty() {
            return Ok(BatchSubmissionResult { submitted: 0, failed: 0, errors: vec![] });
        }

        debug!(count = segments.len(), "Forwarding segments");

        self.process_batches(segments, "segment", |commands, item| {
            let item = item.clone();
            async move {
                commands.create_segment(&item).await?;
                Ok(())
            }
        })
        .await
    }

    /// Forward a batch of snapshots to the API
    ///
    /// # Arguments
    ///
    /// * `snapshots` - Snapshots to forward
    ///
    /// # Returns
    ///
    /// Batch submission result
    ///
    /// # Errors
    ///
    /// Returns error only if all submissions fail
    #[instrument(skip(self, snapshots), fields(count = snapshots.len()))]
    pub async fn forward_snapshots(
        &self,
        snapshots: Vec<ActivitySnapshot>,
    ) -> Result<BatchSubmissionResult, ApiError> {
        if snapshots.is_empty() {
            return Ok(BatchSubmissionResult { submitted: 0, failed: 0, errors: vec![] });
        }

        debug!(count = snapshots.len(), "Forwarding snapshots");

        self.process_batches(snapshots, "snapshot", |commands, item| {
            let item = item.clone();
            async move {
                commands.create_snapshot(&item).await?;
                Ok(())
            }
        })
        .await
    }

    async fn process_batches<T, F, Fut>(
        &self,
        items: Vec<T>,
        label: &'static str,
        mut submit: F,
    ) -> Result<BatchSubmissionResult, ApiError>
    where
        T: Clone + Send + 'static,
        F: FnMut(Arc<ApiCommands>, &T) -> Fut + Copy + Send + 'static,
        Fut: std::future::Future<Output = Result<(), ApiError>> + Send + 'static,
    {
        let mut submitted = 0;
        let mut failed = 0;
        let mut errors = Vec::new();

        let batch_size = self.config.max_batch_size;
        let max_parallel = self.config.max_parallel;

        for (batch_idx, chunk) in items.chunks(batch_size).enumerate() {
            let mut tasks: TaskList = Vec::new();

            for (idx, item) in chunk.iter().enumerate() {
                let commands = Arc::clone(&self.commands);
                let item_clone = item.clone();
                let global_idx = batch_idx * batch_size + idx;

                tasks.push((
                    global_idx,
                    tokio::spawn(async move { submit(commands, &item_clone).await }),
                ));

                if tasks.len() >= max_parallel {
                    Self::drain_tasks(&mut tasks, &mut submitted, &mut failed, &mut errors).await;
                }
            }

            Self::drain_tasks(&mut tasks, &mut submitted, &mut failed, &mut errors).await;
        }

        if submitted == 0 {
            return Err(ApiError::Server(format!("All {} submissions failed", label)));
        } else if failed > 0 {
            warn!(submitted = submitted, failed = failed, "Batch submission completed with errors");
        } else {
            info!(submitted = submitted, "Batch submission successful");
        }

        Ok(BatchSubmissionResult { submitted, failed, errors })
    }

    async fn drain_tasks(
        tasks: &mut TaskList,
        submitted: &mut usize,
        failed: &mut usize,
        errors: &mut Vec<(usize, String)>,
    ) {
        let mut pending = Vec::new();
        std::mem::swap(tasks, &mut pending);

        for (idx, task) in pending {
            match task.await {
                Ok(Ok(_)) => *submitted += 1,
                Ok(Err(err)) => {
                    *failed += 1;
                    errors.push((idx, err.to_string()));
                }
                Err(join_err) => {
                    *failed += 1;
                    errors.push((idx, format!("Task join error: {}", join_err)));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_empty_batch() {
        // This test would require mocking ApiCommands
        // For now, just verify the config
        let config = ForwarderConfig::default();
        assert_eq!(config.max_batch_size, 50);
        assert_eq!(config.max_parallel, 5);
    }
}
