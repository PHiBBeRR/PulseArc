//! API sync scheduler
//!
//! Provides periodic synchronization scheduler for API operations with
//! lifecycle management.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, instrument, warn};

use super::forwarder::ApiForwarder;

/// Type alias for task handle to avoid complexity warnings
type TaskHandle = Arc<Mutex<Option<JoinHandle<()>>>>;

/// Configuration for API scheduler
#[derive(Debug, Clone)]
pub struct SchedulerConfig {
    /// Sync interval
    pub interval: Duration,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(900), // 15 minutes
        }
    }
}

/// API sync scheduler
pub struct ApiScheduler {
    forwarder: Arc<ApiForwarder>,
    config: SchedulerConfig,
    cancellation_token: CancellationToken,
    task_handle: TaskHandle,
}

impl ApiScheduler {
    /// Create a new API scheduler
    ///
    /// # Arguments
    ///
    /// * `forwarder` - API forwarder
    /// * `config` - Scheduler configuration
    pub fn new(forwarder: Arc<ApiForwarder>, config: SchedulerConfig) -> Self {
        Self {
            forwarder,
            config,
            cancellation_token: CancellationToken::new(),
            task_handle: Arc::new(Mutex::new(None)),
        }
    }

    /// Start the scheduler
    ///
    /// Spawns a background task that runs sync periodically.
    ///
    /// # Errors
    ///
    /// Returns error if scheduler is already running
    #[instrument(skip(self))]
    pub async fn start(&mut self) -> Result<(), String> {
        if self.is_running().await {
            return Err("Scheduler already running".to_string());
        }

        info!("Starting API scheduler");

        // Create a new cancellation token (supports restart after stop)
        self.cancellation_token = CancellationToken::new();

        let forwarder = Arc::clone(&self.forwarder);
        let interval = self.config.interval;
        let cancel = self.cancellation_token.clone();

        let handle = tokio::spawn(async move {
            Self::sync_loop(forwarder, interval, cancel).await;
        });

        *self.task_handle.lock().await = Some(handle);

        info!("API scheduler started");

        Ok(())
    }

    /// Stop the scheduler gracefully
    ///
    /// Cancels the background task and awaits completion.
    ///
    /// # Errors
    ///
    /// Returns error if scheduler is not running
    #[instrument(skip(self))]
    pub async fn stop(&mut self) -> Result<(), String> {
        if !self.is_running().await {
            return Err("Scheduler not running".to_string());
        }

        info!("Stopping API scheduler");

        // Cancel background task
        self.cancellation_token.cancel();

        // Await handle with timeout
        if let Some(handle) = self.task_handle.lock().await.take() {
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => {
                    warn!("Scheduler task panicked: {}", e);
                    return Err("Scheduler task panicked".to_string());
                }
                Err(_) => {
                    warn!("Scheduler task did not complete within timeout");
                    return Err("Scheduler task timeout".to_string());
                }
            }
        }

        info!("API scheduler stopped");

        Ok(())
    }

    /// Check if scheduler is running
    ///
    /// A scheduler is considered running if it has an active task handle.
    pub async fn is_running(&self) -> bool {
        self.task_handle.lock().await.is_some()
    }

    /// Background sync loop
    async fn sync_loop(
        _forwarder: Arc<ApiForwarder>,
        interval: Duration,
        cancel: CancellationToken,
    ) {
        loop {
            tokio::select! {
                _ = cancel.cancelled() => {
                    debug!("Sync loop cancelled");
                    break;
                }
                _ = tokio::time::sleep(interval) => {
                    // Sync would happen here
                    // In a real implementation, this would:
                    // 1. Fetch pending data from local database
                    // 2. Forward to API using forwarder
                    // 3. Mark as synced
                    debug!("Periodic sync triggered (placeholder)");
                }
            }
        }
    }
}

/// Ensure scheduler is stopped when dropped
impl Drop for ApiScheduler {
    fn drop(&mut self) {
        // Note: Can't check task_handle (async), so check if token is not cancelled
        // This is best-effort cleanup in Drop
        if !self.cancellation_token.is_cancelled() {
            warn!("ApiScheduler dropped while running; cancelling");
            self.cancellation_token.cancel();
        }
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::api::auth::AccessTokenProvider;
    use crate::api::client::{ApiClient, ApiClientConfig};
    use crate::api::commands::ApiCommands;
    use crate::api::errors::ApiError;
    use crate::api::forwarder::ForwarderConfig;

    #[derive(Clone)]
    struct MockAuthProvider;

    #[async_trait]
    impl AccessTokenProvider for MockAuthProvider {
        async fn access_token(&self) -> Result<String, ApiError> {
            Ok("test-token".to_string())
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_scheduler_lifecycle() {
        let config = ApiClientConfig::default();
        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = Arc::new(ApiCommands::new(client));
        let forwarder = Arc::new(ApiForwarder::new(commands, ForwarderConfig::default()));

        let mut scheduler = ApiScheduler::new(forwarder, SchedulerConfig::default());

        // Initially not running
        assert!(!scheduler.is_running().await);

        // Start succeeds
        scheduler.start().await.unwrap();
        assert!(scheduler.is_running().await);

        // Stop succeeds
        scheduler.stop().await.unwrap();
        assert!(!scheduler.is_running().await);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_double_start_fails() {
        let config = ApiClientConfig::default();
        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = Arc::new(ApiCommands::new(client));
        let forwarder = Arc::new(ApiForwarder::new(commands, ForwarderConfig::default()));

        let mut scheduler = ApiScheduler::new(forwarder, SchedulerConfig::default());

        scheduler.start().await.unwrap();

        // Second start should fail
        let result = scheduler.start().await;
        assert!(result.is_err());

        scheduler.stop().await.unwrap();
    }
}
