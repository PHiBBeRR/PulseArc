//! Bulkhead pattern for limiting concurrent operations
//!
//! The bulkhead pattern prevents resource exhaustion by limiting the number
//! of concurrent operations. Named after ship bulkheads that contain flooding
//! to specific compartments, this pattern isolates failures and prevents
//! cascading resource exhaustion.

use std::fmt;
use std::future::Future;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{Semaphore, SemaphorePermit};
use tracing::{debug, instrument, warn};

use super::ResilienceError;

/// Configuration for bulkhead behavior
#[derive(Debug, Clone)]
pub struct BulkheadConfig {
    /// Maximum number of concurrent operations allowed
    pub max_concurrent: usize,
    /// Maximum number of operations waiting in queue
    pub max_queue: usize,
    /// Optional timeout for acquiring a permit
    pub acquire_timeout: Option<Duration>,
}

impl Default for BulkheadConfig {
    fn default() -> Self {
        Self { max_concurrent: 10, max_queue: 10, acquire_timeout: Some(Duration::from_secs(5)) }
    }
}

impl BulkheadConfig {
    /// Create a new configuration builder
    pub fn builder() -> BulkheadConfigBuilder {
        BulkheadConfigBuilder::new()
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_concurrent == 0 {
            return Err("max_concurrent must be greater than 0".to_string());
        }
        Ok(())
    }
}

/// Builder for BulkheadConfig
#[derive(Debug)]
pub struct BulkheadConfigBuilder {
    config: BulkheadConfig,
}

impl Default for BulkheadConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl BulkheadConfigBuilder {
    pub fn new() -> Self {
        Self { config: BulkheadConfig::default() }
    }

    pub fn max_concurrent(mut self, max: usize) -> Self {
        self.config.max_concurrent = max;
        self
    }

    pub fn max_queue(mut self, max: usize) -> Self {
        self.config.max_queue = max;
        self
    }

    pub fn acquire_timeout(mut self, timeout: Duration) -> Self {
        self.config.acquire_timeout = Some(timeout);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.config.acquire_timeout = None;
        self
    }

    pub fn build(self) -> Result<BulkheadConfig, String> {
        self.config.validate()?;
        Ok(self.config)
    }
}

/// Metrics for bulkhead monitoring
#[derive(Debug, Clone)]
pub struct BulkheadMetrics {
    /// Total number of operations executed
    pub total_operations: u64,
    /// Total number of operations rejected (full)
    pub rejected_operations: u64,
    /// Total number of timeouts waiting for permit
    pub timeout_count: u64,
    /// Current number of concurrent operations
    pub current_concurrent: usize,
    /// Current number of operations waiting in queue
    pub current_queued: usize,
    /// Maximum concurrent operations allowed
    pub max_concurrent: usize,
}

impl BulkheadMetrics {
    /// Calculate the current utilization as a percentage (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        self.current_concurrent as f64 / self.max_concurrent as f64
    }

    /// Calculate the rejection rate as a percentage (0.0 to 1.0)
    pub fn rejection_rate(&self) -> f64 {
        let total = self.total_operations + self.rejected_operations;
        if total == 0 {
            return 0.0;
        }
        self.rejected_operations as f64 / total as f64
    }

    /// Check if the bulkhead is at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.current_concurrent >= self.max_concurrent
    }

    /// Get a human-readable status message
    pub fn status_message(&self) -> String {
        format!(
            "Bulkhead: {}/{} concurrent ({:.1}% utilized), {} rejected, {} timeouts",
            self.current_concurrent,
            self.max_concurrent,
            self.utilization() * 100.0,
            self.rejected_operations,
            self.timeout_count
        )
    }
}

/// Bulkhead for limiting concurrent operations
///
/// Limits the number of concurrent operations to prevent resource exhaustion.
/// Operations that exceed the limit can either be rejected immediately or
/// queued up to a maximum queue size.
///
/// # Examples
///
/// ```rust
/// use pulsearc_common::resilience::{Bulkhead, BulkheadConfig};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = BulkheadConfig::builder().max_concurrent(5).max_queue(10).build()?;
///
/// let bulkhead = Bulkhead::new(config);
///
/// let result = bulkhead
///     .execute(|| async {
///         // Your operation
///         Ok::<_, std::io::Error>("Success")
///     })
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct Bulkhead {
    config: BulkheadConfig,
    semaphore: Arc<Semaphore>,
    total_operations: Arc<AtomicU64>,
    rejected_operations: Arc<AtomicU64>,
    timeout_count: Arc<AtomicU64>,
}

impl Bulkhead {
    /// Create a new bulkhead with the given configuration
    pub fn new(config: BulkheadConfig) -> Self {
        config.validate().expect("Invalid bulkhead configuration");

        let total_permits = config.max_concurrent + config.max_queue;
        Self {
            semaphore: Arc::new(Semaphore::new(total_permits)),
            total_operations: Arc::new(AtomicU64::new(0)),
            rejected_operations: Arc::new(AtomicU64::new(0)),
            timeout_count: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    /// Create a bulkhead with default configuration
    pub fn with_defaults() -> Self {
        Self::new(BulkheadConfig::default())
    }

    /// Try to acquire a permit without waiting
    ///
    /// Returns `Some(permit)` if available, `None` if at capacity.
    pub fn try_acquire(&self) -> Option<SemaphorePermit<'_>> {
        self.semaphore.try_acquire().ok()
    }

    /// Try to acquire a permit with the configured timeout
    async fn acquire_with_timeout(
        &self,
    ) -> Result<SemaphorePermit<'_>, ResilienceError<std::io::Error>> {
        match self.config.acquire_timeout {
            Some(timeout) => {
                match tokio::time::timeout(timeout, self.semaphore.acquire()).await {
                    Ok(Ok(permit)) => Ok(permit),
                    Ok(Err(_)) => {
                        // Semaphore closed (should never happen)
                        Err(ResilienceError::BulkheadFull { capacity: self.config.max_concurrent })
                    }
                    Err(_) => {
                        // Timeout
                        self.timeout_count.fetch_add(1, Ordering::Relaxed);
                        Err(ResilienceError::Timeout { timeout })
                    }
                }
            }
            None => {
                // No timeout, wait indefinitely
                self.semaphore.acquire().await.map_err(|_| ResilienceError::BulkheadFull {
                    capacity: self.config.max_concurrent,
                })
            }
        }
    }

    /// Execute an operation with bulkhead protection
    ///
    /// This method acquires a permit (waiting if necessary up to the configured
    /// timeout), executes the operation, and releases the permit when done.
    #[instrument(skip(self, operation), fields(concurrent = self.current_concurrent()))]
    pub async fn execute<F, Fut, T, E>(&self, operation: F) -> Result<T, ResilienceError<E>>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, E>>,
        E: std::error::Error + Send + Sync + 'static,
    {
        // Try to acquire permit
        let _permit = match self.acquire_with_timeout().await {
            Ok(permit) => permit,
            Err(e) => {
                self.rejected_operations.fetch_add(1, Ordering::Relaxed);
                debug!("Bulkhead rejected operation: {:?}", e);
                // Map the error type
                return match e {
                    ResilienceError::Timeout { timeout } => {
                        Err(ResilienceError::Timeout { timeout })
                    }
                    ResilienceError::BulkheadFull { capacity } => {
                        Err(ResilienceError::BulkheadFull { capacity })
                    }
                    _ => {
                        Err(ResilienceError::BulkheadFull { capacity: self.config.max_concurrent })
                    }
                };
            }
        };

        self.total_operations.fetch_add(1, Ordering::Relaxed);
        debug!("Bulkhead: executing operation ({} concurrent)", self.current_concurrent());

        // Execute the operation
        match operation().await {
            Ok(result) => Ok(result),
            Err(error) => {
                warn!("Bulkhead: operation failed");
                Err(ResilienceError::OperationFailed { source: error })
            }
        }
        // Permit is automatically released here when dropped
    }

    /// Get the current number of concurrent operations
    pub fn current_concurrent(&self) -> usize {
        let total_permits = self.config.max_concurrent + self.config.max_queue;
        let available = self.semaphore.available_permits();
        total_permits.saturating_sub(available)
    }

    /// Get the current number of operations waiting in queue
    pub fn current_queued(&self) -> usize {
        let concurrent = self.current_concurrent();
        concurrent.saturating_sub(self.config.max_concurrent)
    }

    /// Get bulkhead metrics
    pub fn metrics(&self) -> BulkheadMetrics {
        BulkheadMetrics {
            total_operations: self.total_operations.load(Ordering::Acquire),
            rejected_operations: self.rejected_operations.load(Ordering::Acquire),
            timeout_count: self.timeout_count.load(Ordering::Acquire),
            current_concurrent: self.current_concurrent(),
            current_queued: self.current_queued(),
            max_concurrent: self.config.max_concurrent,
        }
    }

    /// Reset metrics counters
    pub fn reset_metrics(&self) {
        self.total_operations.store(0, Ordering::Release);
        self.rejected_operations.store(0, Ordering::Release);
        self.timeout_count.store(0, Ordering::Release);
    }
}

impl Clone for Bulkhead {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            semaphore: Arc::clone(&self.semaphore),
            total_operations: Arc::clone(&self.total_operations),
            rejected_operations: Arc::clone(&self.rejected_operations),
            timeout_count: Arc::clone(&self.timeout_count),
        }
    }
}

impl fmt::Debug for Bulkhead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Bulkhead")
            .field("max_concurrent", &self.config.max_concurrent)
            .field("max_queue", &self.config.max_queue)
            .field("current_concurrent", &self.current_concurrent())
            .field("current_queued", &self.current_queued())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicU32;

    use super::*;

    #[tokio::test]
    async fn test_bulkhead_basic() {
        let config = BulkheadConfig::builder().max_concurrent(2).max_queue(1).build().unwrap();

        let bulkhead = Bulkhead::new(config);

        let result = bulkhead.execute(|| async { Ok::<_, std::io::Error>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_bulkhead_concurrent_limit() {
        let config = BulkheadConfig::builder()
            .max_concurrent(2)
            .max_queue(0)
            .acquire_timeout(Duration::from_millis(100))
            .build()
            .unwrap();

        let bulkhead = Arc::new(Bulkhead::new(config));
        let counter = Arc::new(AtomicU32::new(0));

        let mut handles = vec![];

        // Spawn 5 operations, only 2 should run concurrently
        for _ in 0..5 {
            let bulkhead = Arc::clone(&bulkhead);
            let counter = Arc::clone(&counter);
            let handle = tokio::spawn(async move {
                bulkhead
                    .execute(|| {
                        let counter = Arc::clone(&counter);
                        async move {
                            counter.fetch_add(1, Ordering::SeqCst);
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            Ok::<_, std::io::Error>(())
                        }
                    })
                    .await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            let _ = handle.await;
        }

        let metrics = bulkhead.metrics();
        assert!(metrics.rejected_operations > 0 || metrics.timeout_count > 0);
    }

    #[tokio::test]
    async fn test_bulkhead_metrics() {
        let config = BulkheadConfig::builder().max_concurrent(5).build().unwrap();

        let bulkhead = Bulkhead::new(config);

        // Execute some successful operations
        for _ in 0..3 {
            let _ = bulkhead.execute(|| async { Ok::<_, std::io::Error>(()) }).await;
        }

        let metrics = bulkhead.metrics();
        assert_eq!(metrics.total_operations, 3);
        assert_eq!(metrics.max_concurrent, 5);
    }

    #[tokio::test]
    async fn test_bulkhead_timeout() {
        let config = BulkheadConfig::builder()
            .max_concurrent(1)
            .max_queue(0)
            .acquire_timeout(Duration::from_millis(50))
            .build()
            .unwrap();

        let bulkhead = Arc::new(Bulkhead::new(config));

        // Start a long-running operation
        let bulkhead1 = Arc::clone(&bulkhead);
        let handle1 = tokio::spawn(async move {
            bulkhead1
                .execute(|| async {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                    Ok::<_, std::io::Error>(())
                })
                .await
        });

        // Wait a bit to ensure first operation starts
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Try to start another - should timeout
        let result = bulkhead.execute(|| async { Ok::<_, std::io::Error>(()) }).await;
        assert!(result.is_err());

        // Wait for first to complete
        let _ = handle1.await;

        let metrics = bulkhead.metrics();
        assert!(metrics.timeout_count > 0 || metrics.rejected_operations > 0);
    }

    #[test]
    fn test_bulkhead_config_validation() {
        assert!(BulkheadConfig::builder().max_concurrent(0).build().is_err());
        assert!(BulkheadConfig::builder().max_concurrent(1).build().is_ok());
    }

    #[test]
    fn test_bulkhead_metrics_methods() {
        let metrics = BulkheadMetrics {
            total_operations: 80,
            rejected_operations: 20,
            timeout_count: 5,
            current_concurrent: 5,
            current_queued: 0,
            max_concurrent: 10,
        };

        assert_eq!(metrics.utilization(), 0.5);
        assert_eq!(metrics.rejection_rate(), 0.2);
        assert!(!metrics.is_at_capacity());
    }
}
