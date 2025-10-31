//! SAP health monitoring with proper lifecycle management
//!
//! This module provides background health monitoring for the SAP connector API
//! with:
//! - Explicit lifecycle (start/stop with join handles)
//! - Cancellation support
//! - Timeout protection
//! - Event callbacks for status changes
//! - Structured tracing
//!
//! # Architecture
//!
//! The health monitor follows the worker pattern with clean separation:
//! - `SapHealthMonitor`: Lifecycle coordinator (owns task handle)
//! - `health_worker()`: Pure async worker function (easier to test)
//! - `HealthStatusListener`: Trait for downstream event handling (Tauri in API
//!   layer)
//!
//! # Usage
//!
//! ```no_run
//! use std::sync::Arc;
//!
//! use async_trait::async_trait;
//! use pulsearc_domain::Result;
//! use pulsearc_infra::integrations::sap::client::SapClient;
//! use pulsearc_infra::integrations::sap::health::{
//!     HealthStatus, HealthStatusListener, SapHealthMonitor,
//! };
//!
//! // Implement listener (Tauri layer would emit events here)
//! struct MyListener;
//!
//! #[async_trait]
//! impl HealthStatusListener for MyListener {
//!     async fn on_health_changed(&self, status: HealthStatus) {
//!         println!("SAP health: {:?}", status);
//!     }
//! }
//!
//! # async fn example(client: Arc<SapClient>) -> Result<()> {
//! let listener = Arc::new(MyListener);
//! let mut monitor = SapHealthMonitor::new(client, listener, 30);
//!
//! // Start monitoring
//! monitor.start().await?;
//!
//! // ... do work ...
//!
//! // Graceful shutdown
//! monitor.stop().await?;
//! # Ok(())
//! # }
//! ```

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pulsearc_domain::{PulseArcError, Result};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};

use super::client::SapClient;

/// Health status of the SAP connector
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    /// SAP connector is reachable and responding
    Healthy,

    /// SAP connector is unreachable or not responding
    Unhealthy,

    /// Health status is unknown (initial state or after errors)
    Unknown,
}

/// Listener for health status changes
///
/// Implement this trait to receive health status updates. The Tauri API layer
/// should implement this to emit events to the frontend.
///
/// # Example
///
/// ```no_run
/// use async_trait::async_trait;
/// use pulsearc_infra::integrations::sap::health::{HealthStatus, HealthStatusListener};
///
/// struct TauriHealthListener {
///     app_handle: tauri::AppHandle,
/// }
///
/// #[async_trait]
/// impl HealthStatusListener for TauriHealthListener {
///     async fn on_health_changed(&self, status: HealthStatus) {
///         // Emit Tauri event to frontend
///         let _ = self.app_handle.emit("sap-health-changed", &status);
///     }
/// }
/// ```
#[async_trait]
pub trait HealthStatusListener: Send + Sync {
    /// Called when health status changes
    ///
    /// This is only called when the status actually changes, not on every
    /// check.
    async fn on_health_changed(&self, status: HealthStatus);
}

/// SAP health monitor with explicit lifecycle
///
/// Monitors SAP connector health in the background and emits events on status
/// changes. Follows CLAUDE.md runtime rules:
/// - Spawns via Tokio with join handle
/// - Explicit shutdown via `stop()`
/// - Cancellation support
/// - Timeouts on external calls
pub struct SapHealthMonitor {
    client: Arc<SapClient>,
    listener: Arc<dyn HealthStatusListener>,
    interval_secs: u64,
    task_handle: Option<JoinHandle<()>>,
    cancellation: CancellationToken,
}

impl SapHealthMonitor {
    /// Create a new health monitor
    ///
    /// # Arguments
    ///
    /// * `client` - SAP client to check health
    /// * `listener` - Callback for status changes
    /// * `interval_secs` - How often to check health (recommended: 30-60
    ///   seconds)
    ///
    /// # Returns
    ///
    /// A health monitor ready to start
    pub fn new(
        client: Arc<SapClient>,
        listener: Arc<dyn HealthStatusListener>,
        interval_secs: u64,
    ) -> Self {
        Self {
            client,
            listener,
            interval_secs,
            task_handle: None,
            cancellation: CancellationToken::new(),
        }
    }

    /// Start background health monitoring
    ///
    /// Spawns a Tokio task that checks SAP health at the configured interval.
    /// The task will run until `stop()` is called.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Task started successfully
    /// * `Err(_)` - Already running
    ///
    /// # Tracing
    ///
    /// Emits structured logs with:
    /// - `interval_secs` - Check interval
    /// - Status transitions
    pub async fn start(&mut self) -> Result<()> {
        if self.task_handle.is_some() {
            return Err(PulseArcError::Internal("Health monitor already running".to_string()));
        }

        let cancel = self.cancellation.clone();
        let client = self.client.clone();
        let listener = self.listener.clone();
        let interval = Duration::from_secs(self.interval_secs);

        info!(interval_secs = self.interval_secs, "Starting SAP health monitor");

        let handle = tokio::spawn(async move {
            health_worker(client, listener, interval, cancel).await;
        });

        self.task_handle = Some(handle);
        Ok(())
    }

    /// Stop background health monitoring
    ///
    /// Signals the worker task to stop and waits for it to complete.
    /// Times out after 5 seconds if the task doesn't stop gracefully.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Task stopped successfully
    /// * `Err(_)` - Timeout or task join error
    ///
    /// # Tracing
    ///
    /// Emits shutdown log on successful stop
    pub async fn stop(&mut self) -> Result<()> {
        self.cancellation.cancel();

        if let Some(handle) = self.task_handle.take() {
            tokio::time::timeout(Duration::from_secs(5), handle)
                .await
                .map_err(|_| {
                    PulseArcError::Internal("Health monitor shutdown timeout".to_string())
                })?
                .map_err(|e| PulseArcError::Internal(format!("Task join failed: {}", e)))?;
        }

        info!("SAP health monitor stopped");
        Ok(())
    }

    /// Check if monitor is currently running
    pub fn is_running(&self) -> bool {
        self.task_handle.is_some() && !self.cancellation.is_cancelled()
    }
}

/// Pure async worker function for health monitoring
///
/// This function is separated from `SapHealthMonitor` for testability.
/// It can be tested without Tokio task spawning overhead.
///
/// # Arguments
///
/// * `client` - SAP client for health checks
/// * `listener` - Callback for status changes
/// * `interval` - Check interval duration
/// * `cancel` - Cancellation token for shutdown
///
/// # Behavior
///
/// - Starts in `Unknown` state
/// - Checks health every `interval`
/// - Only emits events on state transitions
/// - Timeouts health checks at 5 seconds
/// - Stops when `cancel` is triggered
/// - Logs all state transitions with structured tracing
async fn health_worker(
    client: Arc<SapClient>,
    listener: Arc<dyn HealthStatusListener>,
    interval: Duration,
    cancel: CancellationToken,
) {
    let mut current_status = HealthStatus::Unknown;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                info!("Health monitor worker shutting down");
                break;
            }
            _ = tokio::time::sleep(interval) => {
                // Timeout external call to avoid hanging
                let check_result = tokio::time::timeout(
                    Duration::from_secs(5),
                    client.check_health()
                ).await;

                let new_status = match check_result {
                    Ok(Ok(true)) => HealthStatus::Healthy,
                    Ok(Ok(false)) => HealthStatus::Unhealthy,
                    Ok(Err(e)) => {
                        warn!(error = %e, "Health check error");
                        HealthStatus::Unknown
                    }
                    Err(_) => {
                        warn!("Health check timeout");
                        HealthStatus::Unknown
                    }
                };

                // Only emit on transition
                if new_status != current_status {
                    info!(
                        previous_status = ?current_status,
                        new_status = ?new_status,
                        "SAP health status changed"
                    );

                    listener.on_health_changed(new_status.clone()).await;
                    current_status = new_status;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use pulsearc_core::classification::ports::WbsRepository;
    use pulsearc_domain::types::sap::WbsElement;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::integrations::sap::client::AccessTokenProvider;

    // Mock WBS Repository
    struct MockWbsRepository;

    impl WbsRepository for MockWbsRepository {
        fn count_active_wbs(&self) -> Result<i64> {
            Ok(0)
        }

        fn get_last_sync_timestamp(&self) -> Result<Option<i64>> {
            Ok(None)
        }

        fn load_common_projects(&self, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn fts5_search_keyword(&self, _keyword: &str, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn get_wbs_by_project_def(&self, _project_def: &str) -> Result<Option<WbsElement>> {
            Ok(None)
        }

        fn get_wbs_by_wbs_code(&self, _wbs_code: &str) -> Result<Option<WbsElement>> {
            Ok(None)
        }
    }

    // Mock Access Token Provider
    struct MockTokenProvider;

    #[async_trait]
    impl AccessTokenProvider for MockTokenProvider {
        async fn access_token(&self) -> Result<String> {
            Ok("test-token".to_string())
        }
    }

    // Mock Listener that records status changes
    struct TestListener {
        statuses: Arc<Mutex<Vec<HealthStatus>>>,
    }

    impl TestListener {
        fn new() -> (Self, Arc<Mutex<Vec<HealthStatus>>>) {
            let statuses = Arc::new(Mutex::new(Vec::new()));
            (Self { statuses: statuses.clone() }, statuses)
        }
    }

    #[async_trait]
    impl HealthStatusListener for TestListener {
        async fn on_health_changed(&self, status: HealthStatus) {
            self.statuses.lock().unwrap().push(status);
        }
    }

    #[tokio::test]
    async fn transitions_to_healthy_when_server_responds() {
        let mock_server = MockServer::start().await;

        // Return 200 OK for health checks
        Mock::given(method("HEAD"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let (listener, statuses) = TestListener::new();
        let client = Arc::new(
            SapClient::new(
                mock_server.uri(),
                Arc::new(MockWbsRepository),
                "test".to_string(),
                Arc::new(MockTokenProvider),
            )
            .unwrap(),
        );

        // Run worker for a short time
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let worker_handle = tokio::spawn(async move {
            health_worker(client, Arc::new(listener), Duration::from_millis(100), cancel_clone)
                .await;
        });

        // Wait for at least one check
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Stop worker
        cancel.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(1), worker_handle).await;

        // Should have transitioned to Healthy
        let recorded = statuses.lock().unwrap();
        assert!(!recorded.is_empty());
        assert_eq!(recorded[0], HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn transitions_to_unhealthy_when_server_fails() {
        // Use invalid URL that will fail immediately
        let (listener, statuses) = TestListener::new();
        let client = Arc::new(
            SapClient::new(
                "http://localhost:9999".to_string(),
                Arc::new(MockWbsRepository),
                "test".to_string(),
                Arc::new(MockTokenProvider),
            )
            .unwrap(),
        );

        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let worker_handle = tokio::spawn(async move {
            health_worker(client, Arc::new(listener), Duration::from_millis(100), cancel_clone)
                .await;
        });

        tokio::time::sleep(Duration::from_millis(150)).await;
        cancel.cancel();
        let _ = tokio::time::timeout(Duration::from_secs(1), worker_handle).await;

        let recorded = statuses.lock().unwrap();
        assert!(!recorded.is_empty());
        // Should be Unhealthy (server unreachable)
        assert_eq!(recorded[0], HealthStatus::Unhealthy);
    }

    #[tokio::test]
    async fn start_and_stop_lifecycle() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let (listener, _) = TestListener::new();
        let client = Arc::new(
            SapClient::new(
                mock_server.uri(),
                Arc::new(MockWbsRepository),
                "test".to_string(),
                Arc::new(MockTokenProvider),
            )
            .unwrap(),
        );

        let mut monitor = SapHealthMonitor::new(client, Arc::new(listener), 1);

        // Should not be running initially
        assert!(!monitor.is_running());

        // Start
        monitor.start().await.unwrap();
        assert!(monitor.is_running());

        // Can't start twice
        assert!(monitor.start().await.is_err());

        // Stop
        monitor.stop().await.unwrap();
        assert!(!monitor.is_running());
    }

    #[tokio::test]
    async fn cancellation_stops_worker() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let (listener, _) = TestListener::new();
        let client = Arc::new(
            SapClient::new(
                mock_server.uri(),
                Arc::new(MockWbsRepository),
                "test".to_string(),
                Arc::new(MockTokenProvider),
            )
            .unwrap(),
        );

        let mut monitor = SapHealthMonitor::new(client, Arc::new(listener), 1);

        monitor.start().await.unwrap();
        assert!(monitor.is_running());

        // Stop should complete within timeout
        let result = tokio::time::timeout(Duration::from_secs(2), monitor.stop()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
        assert!(!monitor.is_running());
    }
}
