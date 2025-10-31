use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::runtime::Handle;
use tokio::sync::Notify;
use tracing::{debug, error, info, instrument, warn};

use super::errors::{QueueError, QueueResult};
use super::maintenance::{MaintenanceService, PriorityItem, QueueState};
use super::metrics::QueueMetrics;
use super::persistence::PersistenceService;
use super::types::{ItemStatus, QueueConfig, SyncItem};
use crate::error::CommonError;
use crate::sync::retry::{CircuitBreaker, RetryStrategy};

/// Enterprise-grade sync queue with all features
///
/// ## Error Handling
///
/// This queue implementation follows Rust best practices for error handling:
///
/// - All public methods that can fail return `QueueResult<T>` instead of
///   panicking
/// - Lock poisoning is handled gracefully and returned as
///   `QueueError::LockPoisoned`
/// - Errors from dependencies (RetryError, IO errors, etc.) are automatically
///   converted
/// - The `?` operator can be used throughout for clean error propagation
///
/// ## Thread Safety
///
/// The queue is thread-safe and uses `Arc<RwLock<_>>` for shared state
/// management. Lock acquisition failures are properly propagated rather than
/// causing panics, ensuring the queue remains robust even in the face of
/// poisoned locks.
pub struct SyncQueue {
    state: Arc<RwLock<QueueState>>,
    config: Arc<QueueConfig>,
    metrics: Arc<QueueMetrics>,
    shutdown: Arc<AtomicBool>,
    notify: Arc<Notify>,
    retry_strategy: Arc<RetryStrategy>,
    circuit_breaker: Arc<CircuitBreaker>,
    persistence_service: Option<Arc<PersistenceService>>,
    maintenance_service: Arc<MaintenanceService>,
    persistence_handle: Option<tokio::task::JoinHandle<()>>,
    maintenance_handle: Option<tokio::task::JoinHandle<()>>,
}

impl SyncQueue {
    /// Create a new sync queue with default configuration
    pub fn new() -> Self {
        Self::with_config(QueueConfig::default()).expect("Default config should be valid")
    }

    /// Create a new sync queue with custom configuration
    pub fn with_config(config: QueueConfig) -> QueueResult<Self> {
        // Validate configuration
        config.validate().map_err(QueueError::InvalidState)?;

        let state = Arc::new(RwLock::new(QueueState {
            items: BinaryHeap::new(),
            item_map: HashMap::new(),
            processing: HashSet::new(),
            sequence_counter: 0,
            is_locked: false,
        }));

        let metrics = Arc::new(QueueMetrics::new());
        let shutdown = Arc::new(AtomicBool::new(false));
        let notify = Arc::new(Notify::new());

        let retry_strategy = Arc::new(
            RetryStrategy::new()
                .with_max_delay(config.max_retry_delay)?
                .with_base_delay(config.base_retry_delay)?
                .with_max_attempts(5)?,
        );

        let circuit_breaker = Arc::new(
            CircuitBreaker::new().with_failure_threshold(10).with_timeout(Duration::from_secs(60)),
        );

        // Setup persistence if configured
        let persistence_service = if let Some(ref path) = config.persistence_path {
            let mut service = PersistenceService::new(path.clone()).with_metrics(metrics.clone());

            if config.enable_compression {
                service = service.with_compression(config.compression_level);
            }

            if config.enable_encryption {
                if let Some(ref key) = config.encryption_key {
                    service = service.with_encryption(key.clone())?;
                }
            }

            Some(Arc::new(service))
        } else {
            None
        };

        // Setup maintenance service
        let maintenance_service = Arc::new(MaintenanceService::new(
            metrics.clone(),
            config.cleanup_interval,
            config.retention_period,
            config.heap_cleanup_threshold,
        ));

        let mut queue = Self {
            state,
            config: Arc::new(config),
            metrics,
            shutdown,
            notify,
            retry_strategy,
            circuit_breaker,
            persistence_service,
            maintenance_service,
            persistence_handle: None,
            maintenance_handle: None,
        };

        // Load persisted items if available
        queue.load_persisted_items();

        // Start background tasks
        queue.start_background_tasks();

        Ok(queue)
    }

    /// Load persisted items on startup
    fn load_persisted_items(&mut self) {
        if let Some(ref service) = self.persistence_service {
            let service = service.clone();
            let state = self.state.clone();
            let metrics = self.metrics.clone();

            // Use blocking task for initial load
            std::thread::spawn(move || {
                let runtime =
                    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();

                runtime.block_on(async {
                    match service.load().await {
                        Ok(items) => match state.write() {
                            Ok(mut state) => {
                                const MILLIS_THRESHOLD: u128 = 1_000_000_000_000;

                                for mut item in items {
                                    if let Some(ts) = item.next_retry_at {
                                        if ts < MILLIS_THRESHOLD {
                                            item.next_retry_at = Some(ts.saturating_mul(1_000));
                                        }
                                    }

                                    let item_arc = Arc::new(item);
                                    let seq = state.sequence_counter;
                                    state.sequence_counter += 1;
                                    state.item_map.insert(item_arc.id.clone(), item_arc.clone());
                                    state
                                        .items
                                        .push(PriorityItem { item: item_arc, sequence: seq });
                                }
                                metrics.update_size(state.item_map.len());
                                info!("Loaded {} persisted items", state.item_map.len());
                            }
                            Err(e) => {
                                error!("Queue state lock poisoned during startup: {}", e);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to load persisted queue: {}", e);
                        }
                    }
                });
            })
            .join()
            .ok();
        }
    }

    /// Start background tasks
    fn start_background_tasks(&mut self) {
        // Start persistence task
        if let Some(ref service) = self.persistence_service {
            let service = service.clone();
            let state = self.state.clone();
            let interval = self.config.persistence_interval;
            let shutdown = self.shutdown.clone();
            let metrics = self.metrics.clone();

            match Handle::try_current() {
                Ok(runtime) => {
                    let handle = runtime.spawn(async move {
                        let mut interval = tokio::time::interval(interval);
                        interval.tick().await;

                        loop {
                            interval.tick().await;

                            if shutdown.load(AtomicOrdering::Relaxed) {
                                break;
                            }

                            let items = {
                                match state.read() {
                                    Ok(state) => state
                                        .item_map
                                        .values()
                                        .map(|item| (**item).clone())
                                        .collect::<Vec<SyncItem>>(),
                                    Err(e) => {
                                        error!(
                                            "Queue state lock poisoned during persistence: {}",
                                            e
                                        );
                                        Vec::new()
                                    }
                                }
                            };

                            if let Err(e) = service.save(items).await {
                                error!("Failed to persist queue: {}", e);
                                metrics.record_persistence(false);
                            }
                        }
                    });

                    self.persistence_handle = Some(handle);
                }
                Err(_) => {
                    warn!(
                        "Skipping persistence background task start: no active Tokio runtime detected"
                    );
                }
            }
        }

        // Start maintenance task
        {
            let maintenance = self.maintenance_service.clone();
            let state = self.state.clone();
            let interval = self.config.cleanup_interval;
            let shutdown = self.shutdown.clone();

            match Handle::try_current() {
                Ok(runtime) => {
                    let handle = runtime.spawn(async move {
                        let mut interval = tokio::time::interval(interval);
                        interval.tick().await;

                        loop {
                            interval.tick().await;

                            if shutdown.load(AtomicOrdering::Relaxed) {
                                break;
                            }

                            match state.write() {
                                Ok(mut sync_state) => {
                                    maintenance.run_maintenance(&mut sync_state);
                                }
                                Err(e) => {
                                    error!("Queue state lock poisoned during maintenance: {}", e);
                                }
                            }
                        }
                    });

                    self.maintenance_handle = Some(handle);
                }
                Err(_) => {
                    warn!(
                        "Skipping maintenance background task start: no active Tokio runtime detected"
                    );
                }
            }
        }
    }

    /// Push an item to the queue
    #[instrument(skip(self, item), fields(item_id = %item.id, priority = %item.priority))]
    pub async fn push(&self, item: SyncItem) -> QueueResult<()> {
        if self.shutdown.load(AtomicOrdering::Relaxed) {
            return Err(QueueError::ShuttingDown);
        }

        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        if state.is_locked {
            return Err(QueueError::Locked);
        }

        // Check capacity
        if state.item_map.len() >= self.config.max_capacity {
            self.metrics.record_capacity_rejection(1);
            return Err(QueueError::CapacityExceeded(self.config.max_capacity));
        }

        // Deduplication check
        if self.config.enable_deduplication && state.item_map.contains_key(&item.id) {
            self.metrics.record_deduplication();
            return Err(QueueError::DuplicateItem(item.id.clone()));
        }

        let item_arc = Arc::new(item);
        let priority_item =
            PriorityItem { item: item_arc.clone(), sequence: state.sequence_counter };

        state.sequence_counter += 1;
        state.items.push(priority_item);
        state.item_map.insert(item_arc.id.clone(), item_arc);

        self.metrics.record_enqueue(1);
        self.metrics.update_size(state.item_map.len());

        // Notify waiters
        self.notify.notify_one();

        debug!("Item enqueued successfully");
        Ok(())
    }

    /// Push multiple items as a batch
    pub async fn push_batch(&self, items: Vec<SyncItem>) -> QueueResult<Vec<String>> {
        if self.shutdown.load(AtomicOrdering::Relaxed) {
            return Err(QueueError::ShuttingDown);
        }

        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        if state.is_locked {
            return Err(QueueError::Locked);
        }

        // Check capacity
        let new_size = state.item_map.len() + items.len();
        if new_size > self.config.max_capacity {
            self.metrics.record_capacity_rejection(items.len() as u64);
            return Err(QueueError::CapacityExceeded(self.config.max_capacity));
        }

        let mut added_ids = Vec::new();

        for item in items {
            // Skip duplicates in batch mode
            if self.config.enable_deduplication && state.item_map.contains_key(&item.id) {
                self.metrics.record_deduplication();
                continue;
            }

            let item_id = item.id.clone();
            let item_arc = Arc::new(item);
            let priority_item =
                PriorityItem { item: item_arc.clone(), sequence: state.sequence_counter };

            state.sequence_counter += 1;
            state.items.push(priority_item);
            state.item_map.insert(item_id.clone(), item_arc);
            added_ids.push(item_id);
        }

        self.metrics.record_enqueue(added_ids.len() as u64);
        self.metrics.update_size(state.item_map.len());

        // Notify waiters
        if !added_ids.is_empty() {
            self.notify.notify_waiters();
        }

        info!("Batch of {} items enqueued", added_ids.len());
        Ok(added_ids)
    }

    /// Pop an item from the queue (non-blocking)
    pub async fn pop(&self) -> QueueResult<Option<SyncItem>> {
        if self.shutdown.load(AtomicOrdering::Relaxed) {
            return Err(QueueError::ShuttingDown);
        }

        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        if state.is_locked {
            return Err(QueueError::Locked);
        }

        // Find next eligible item (not already processing)
        while let Some(priority_item) = state.items.pop() {
            let item_id = &priority_item.item.id;

            // Skip if already processing
            if state.processing.contains(item_id) {
                continue;
            }

            // Skip if item not in map (orphaned)
            if !state.item_map.contains_key(item_id) {
                continue;
            }

            // Check if retry time has passed
            if let Some(mut next_retry_at) = priority_item.item.next_retry_at {
                const MILLIS_THRESHOLD: u128 = 1_000_000_000_000;
                if next_retry_at < MILLIS_THRESHOLD {
                    next_retry_at = next_retry_at.saturating_mul(1_000);
                }

                let now =
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

                if now < next_retry_at {
                    // Re-add to queue for later
                    state.items.push(priority_item);
                    continue;
                }
            }

            // Mark as processing and create updated item
            state.processing.insert(item_id.clone());

            // Create a new item with updated status (fixes Arc mutation issue)
            let mut updated_item = (*priority_item.item).clone();
            if let Some(ts) = updated_item.next_retry_at {
                const MILLIS_THRESHOLD: u128 = 1_000_000_000_000;
                if ts < MILLIS_THRESHOLD {
                    updated_item.next_retry_at = Some(ts.saturating_mul(1_000));
                }
            }
            updated_item.mark_processing();

            // Update the item in the map
            state.item_map.insert(item_id.clone(), Arc::new(updated_item.clone()));

            // Update metrics
            self.metrics.record_dequeue(1);
            self.metrics.update_size(state.item_map.len());

            debug!("Item dequeued: {}", item_id);

            return Ok(Some(updated_item));
        }

        Ok(None)
    }

    /// Pop an item with wait (blocking until available or timeout)
    pub async fn pop_wait(&self, timeout: Duration) -> QueueResult<Option<SyncItem>> {
        let deadline = Instant::now() + timeout;

        loop {
            // Try to pop an item
            if let Some(item) = self.pop().await? {
                return Ok(Some(item));
            }

            // Check timeout
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(None);
            }

            // Wait for notification or timeout
            tokio::select! {
                _ = self.notify.notified() => continue,
                _ = tokio::time::sleep(remaining) => return Ok(None),
            }
        }
    }

    /// Pop multiple items as a batch
    pub async fn pop_batch(&self, max_items: usize) -> QueueResult<Vec<SyncItem>> {
        let mut items = Vec::new();

        for _ in 0..max_items {
            match self.pop().await? {
                Some(item) => items.push(item),
                None => break,
            }
        }

        if !items.is_empty() {
            info!("Batch of {} items dequeued", items.len());
        }

        Ok(items)
    }

    /// Mark an item as completed
    pub async fn mark_completed(&self, item_id: &str) -> QueueResult<()> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        // Remove from processing set
        state.processing.remove(item_id);

        // Remove from item map and update status
        if let Some(item_arc) = state.item_map.remove(item_id) {
            // Create completed version for metrics
            let mut completed_item = (*item_arc).clone();
            completed_item.mark_completed();

            self.metrics.record_completion(completed_item.processing_duration_ms);
            self.metrics.update_size(state.item_map.len());

            // Record success in circuit breaker
            self.circuit_breaker.record_success()?;

            debug!("Item {} marked as completed", item_id);
            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id.to_string()))
        }
    }

    /// Mark an item as failed and schedule retry if applicable
    pub async fn mark_failed(&self, item_id: &str, error: Option<String>) -> QueueResult<bool> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        // Remove from processing set
        state.processing.remove(item_id);

        // Find and update the item
        if let Some(item_arc) = state.item_map.get(item_id) {
            let mut item = (**item_arc).clone();
            item.mark_failed(error);

            let can_retry = item.can_retry();

            if can_retry {
                // Calculate next retry time
                item.next_retry_at = Some(item.calculate_next_retry(self.config.base_retry_delay));
                item.status = ItemStatus::Pending;

                // Replace with updated item
                let retry_count = item.retry_count;
                let new_item_arc = Arc::new(item);
                let sequence = state.sequence_counter;
                state.item_map.insert(item_id.to_string(), new_item_arc.clone());
                state.items.push(PriorityItem { item: new_item_arc, sequence });
                state.sequence_counter += 1;

                self.metrics.record_retry();
                info!("Item {} scheduled for retry (attempt {})", item_id, retry_count);
            } else {
                // Max retries exceeded, remove from queue
                state.item_map.remove(item_id);
                self.metrics.record_failure();
                warn!("Item {} failed after {} retries", item_id, item.retry_count);
            }

            self.metrics.update_size(state.item_map.len());

            // Record failure in circuit breaker
            self.circuit_breaker.record_failure()?;

            Ok(can_retry)
        } else {
            Err(QueueError::ItemNotFound(item_id.to_string()))
        }
    }

    /// Cancel an item
    pub async fn cancel_item(&self, item_id: &str) -> QueueResult<()> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        state.processing.remove(item_id);

        if let Some(mut item) = state.item_map.remove(item_id) {
            if let Some(item_mut) = Arc::get_mut(&mut item) {
                item_mut.status = ItemStatus::Cancelled;
            }

            self.metrics.record_cancellation();
            self.metrics.update_size(state.item_map.len());

            Ok(())
        } else {
            Err(QueueError::ItemNotFound(item_id.to_string()))
        }
    }

    /// Peek at the next item without removing it
    pub fn peek(&self) -> Option<SyncItem> {
        let state = self.state.read().ok()?;
        state.items.peek().map(|p| (*p.item).clone())
    }

    /// Get queue size (excluding items currently being processed)
    pub fn size(&self) -> usize {
        self.state
            .read()
            .map(|state| state.item_map.len().saturating_sub(state.processing.len()))
            .unwrap_or(0)
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// Get item by ID
    pub fn get_item(&self, item_id: &str) -> Option<SyncItem> {
        let state = self.state.read().ok()?;
        state.item_map.get(item_id).map(|item| (**item).clone())
    }

    /// Get items by status
    pub fn get_items_by_status(&self, status: ItemStatus) -> Vec<SyncItem> {
        let state = self.state.read().ok();
        let Some(state) = state else {
            return Vec::new();
        };
        state
            .item_map
            .values()
            .filter(|item| item.status == status)
            .map(|item| (**item).clone())
            .collect()
    }

    /// Get processing items
    pub fn get_processing_items(&self) -> Vec<String> {
        self.state
            .read()
            .map(|state| state.processing.iter().cloned().collect())
            .unwrap_or_else(|_| Vec::new())
    }

    /// Clear all items from the queue
    pub async fn clear(&self) -> QueueResult<usize> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;

        let count = state.item_map.len();
        state.items.clear();
        state.item_map.clear();
        state.processing.clear();
        state.sequence_counter = 0;

        self.metrics.update_size(0);

        info!("Queue cleared: {} items removed", count);
        Ok(count)
    }

    /// Lock the queue for maintenance
    pub fn lock(&self) -> QueueResult<()> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;
        state.is_locked = true;
        info!("Queue locked for maintenance");
        Ok(())
    }

    /// Unlock the queue
    pub fn unlock(&self) -> QueueResult<()> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;
        state.is_locked = false;
        self.notify.notify_waiters();
        info!("Queue unlocked");
        Ok(())
    }

    /// Get queue metrics
    pub fn metrics(&self) -> crate::sync::queue::metrics::QueueMetricsSnapshot {
        self.metrics.snapshot()
    }

    /// Force persistence
    pub async fn persist(&self) -> QueueResult<()> {
        if let Some(ref service) = self.persistence_service {
            let items = {
                let state = self.state.read().map_err(|e| CommonError::lock(e.to_string()))?;
                state.item_map.values().map(|item| (**item).clone()).collect::<Vec<SyncItem>>()
            };

            service.save(items).await?;
        }
        Ok(())
    }

    /// Force maintenance run
    pub fn run_maintenance(&self) -> QueueResult<()> {
        let mut state = self.state.write().map_err(|e| CommonError::lock(e.to_string()))?;
        self.maintenance_service.run_maintenance(&mut state);
        Ok(())
    }

    /// Get health status
    pub fn health_check(&self) -> QueueResult<bool> {
        Ok(!self.shutdown.load(AtomicOrdering::Relaxed)
            && self.circuit_breaker.should_allow_request()?)
    }

    /// Shutdown the queue gracefully
    pub async fn shutdown(&self) -> QueueResult<()> {
        info!("Shutting down sync queue...");

        // Signal shutdown
        self.shutdown.store(true, AtomicOrdering::Relaxed);

        // Notify all waiters
        self.notify.notify_waiters();

        // Final persistence
        if let Err(e) = self.persist().await {
            error!("Failed to persist queue during shutdown: {}", e);
        }

        info!("Sync queue shutdown complete");
        Ok(())
    }
}

impl Clone for SyncQueue {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            config: self.config.clone(),
            metrics: self.metrics.clone(),
            shutdown: self.shutdown.clone(),
            notify: self.notify.clone(),
            retry_strategy: self.retry_strategy.clone(),
            circuit_breaker: self.circuit_breaker.clone(),
            persistence_service: self.persistence_service.clone(),
            maintenance_service: self.maintenance_service.clone(),
            persistence_handle: None,
            maintenance_handle: None,
        }
    }
}

impl Default for SyncQueue {
    fn default() -> Self {
        Self::new()
    }
}
