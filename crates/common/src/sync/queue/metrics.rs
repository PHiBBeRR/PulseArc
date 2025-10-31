use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};

use serde::{Deserialize, Serialize};

/// Queue metrics for monitoring
#[derive(Debug, Default)]
pub struct QueueMetrics {
    pub total_enqueued: AtomicU64,
    pub total_dequeued: AtomicU64,
    pub total_completed: AtomicU64,
    pub total_failed: AtomicU64,
    pub total_retried: AtomicU64,
    pub total_cancelled: AtomicU64,
    pub current_size: AtomicUsize,
    pub capacity_rejections: AtomicU64,
    pub deduplication_hits: AtomicU64,
    pub compression_bytes_saved: AtomicU64,
    pub encryption_operations: AtomicU64,
    pub persistence_operations: AtomicU64,
    pub persistence_failures: AtomicU64,
    pub heap_cleanups: AtomicU64,
    pub items_cleaned: AtomicU64,
    pub processing_time_total_ms: AtomicU64,
    pub queue_depth_max: AtomicUsize,
    pub last_operation_time: AtomicU64,
}

impl QueueMetrics {
    /// Create new metrics instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Record enqueue operation
    pub fn record_enqueue(&self, count: u64) {
        self.total_enqueued.fetch_add(count, AtomicOrdering::Relaxed);
        self.update_last_operation();
        self.update_max_depth();
    }

    /// Record dequeue operation
    pub fn record_dequeue(&self, count: u64) {
        self.total_dequeued.fetch_add(count, AtomicOrdering::Relaxed);
        self.update_last_operation();
    }

    /// Record completion
    pub fn record_completion(&self, duration_ms: Option<u64>) {
        self.total_completed.fetch_add(1, AtomicOrdering::Relaxed);
        if let Some(ms) = duration_ms {
            self.processing_time_total_ms.fetch_add(ms, AtomicOrdering::Relaxed);
        }
        self.update_last_operation();
    }

    /// Record failure
    pub fn record_failure(&self) {
        self.total_failed.fetch_add(1, AtomicOrdering::Relaxed);
        self.update_last_operation();
    }

    /// Record retry
    pub fn record_retry(&self) {
        self.total_retried.fetch_add(1, AtomicOrdering::Relaxed);
        self.update_last_operation();
    }

    /// Record cancellation
    pub fn record_cancellation(&self) {
        self.total_cancelled.fetch_add(1, AtomicOrdering::Relaxed);
        self.update_last_operation();
    }

    /// Record capacity rejection
    pub fn record_capacity_rejection(&self, count: u64) {
        self.capacity_rejections.fetch_add(count, AtomicOrdering::Relaxed);
        self.update_last_operation();
    }

    /// Record deduplication hit
    pub fn record_deduplication(&self) {
        self.deduplication_hits.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Record compression savings
    pub fn record_compression_savings(&self, bytes: u64) {
        self.compression_bytes_saved.fetch_add(bytes, AtomicOrdering::Relaxed);
    }

    /// Record encryption operation
    pub fn record_encryption(&self) {
        self.encryption_operations.fetch_add(1, AtomicOrdering::Relaxed);
    }

    /// Record persistence operation
    pub fn record_persistence(&self, success: bool) {
        self.persistence_operations.fetch_add(1, AtomicOrdering::Relaxed);
        if !success {
            self.persistence_failures.fetch_add(1, AtomicOrdering::Relaxed);
        }
    }

    /// Record heap cleanup
    pub fn record_heap_cleanup(&self, items_cleaned: usize) {
        self.heap_cleanups.fetch_add(1, AtomicOrdering::Relaxed);
        self.items_cleaned.fetch_add(items_cleaned as u64, AtomicOrdering::Relaxed);
    }

    /// Update current size
    pub fn update_size(&self, size: usize) {
        self.current_size.store(size, AtomicOrdering::Relaxed);
        self.update_max_depth();
    }

    /// Update maximum depth if current exceeds it
    fn update_max_depth(&self) {
        let current = self.current_size.load(AtomicOrdering::Relaxed);
        let mut max = self.queue_depth_max.load(AtomicOrdering::Relaxed);

        while current > max {
            match self.queue_depth_max.compare_exchange_weak(
                max,
                current,
                AtomicOrdering::Relaxed,
                AtomicOrdering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => max = actual,
            }
        }
    }

    /// Update last operation timestamp
    fn update_last_operation(&self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.last_operation_time.store(now, AtomicOrdering::Relaxed);
    }

    /// Get a snapshot of metrics
    pub fn snapshot(&self) -> QueueMetricsSnapshot {
        QueueMetricsSnapshot {
            total_enqueued: self.total_enqueued.load(AtomicOrdering::Relaxed),
            total_dequeued: self.total_dequeued.load(AtomicOrdering::Relaxed),
            total_completed: self.total_completed.load(AtomicOrdering::Relaxed),
            total_failed: self.total_failed.load(AtomicOrdering::Relaxed),
            total_retried: self.total_retried.load(AtomicOrdering::Relaxed),
            total_cancelled: self.total_cancelled.load(AtomicOrdering::Relaxed),
            current_size: self.current_size.load(AtomicOrdering::Relaxed),
            capacity_rejections: self.capacity_rejections.load(AtomicOrdering::Relaxed),
            deduplication_hits: self.deduplication_hits.load(AtomicOrdering::Relaxed),
            compression_bytes_saved: self.compression_bytes_saved.load(AtomicOrdering::Relaxed),
            encryption_operations: self.encryption_operations.load(AtomicOrdering::Relaxed),
            persistence_operations: self.persistence_operations.load(AtomicOrdering::Relaxed),
            persistence_failures: self.persistence_failures.load(AtomicOrdering::Relaxed),
            heap_cleanups: self.heap_cleanups.load(AtomicOrdering::Relaxed),
            items_cleaned: self.items_cleaned.load(AtomicOrdering::Relaxed),
            processing_time_total_ms: self.processing_time_total_ms.load(AtomicOrdering::Relaxed),
            queue_depth_max: self.queue_depth_max.load(AtomicOrdering::Relaxed),
            last_operation_time: self.last_operation_time.load(AtomicOrdering::Relaxed),
            average_processing_time_ms: self.calculate_average_processing_time(),
            throughput: self.calculate_throughput(),
            success_rate: self.calculate_success_rate(),
        }
    }

    /// Calculate average processing time
    fn calculate_average_processing_time(&self) -> f64 {
        let completed = self.total_completed.load(AtomicOrdering::Relaxed);
        if completed == 0 {
            return 0.0;
        }
        let total_ms = self.processing_time_total_ms.load(AtomicOrdering::Relaxed);
        total_ms as f64 / completed as f64
    }

    /// Calculate throughput (items per second)
    fn calculate_throughput(&self) -> f64 {
        let total = self.total_dequeued.load(AtomicOrdering::Relaxed);
        let last_op = self.last_operation_time.load(AtomicOrdering::Relaxed);

        if last_op == 0 || total == 0 {
            return 0.0;
        }

        // Approximate based on total operations and time since start
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let duration = now.saturating_sub(last_op.saturating_sub(60)); // Use last minute
        if duration == 0 {
            return 0.0;
        }

        total as f64 / duration as f64
    }

    /// Calculate success rate
    fn calculate_success_rate(&self) -> f64 {
        let completed = self.total_completed.load(AtomicOrdering::Relaxed);
        let failed = self.total_failed.load(AtomicOrdering::Relaxed);
        let total = completed + failed;

        if total == 0 {
            return 100.0;
        }

        (completed as f64 / total as f64) * 100.0
    }

    /// Reset all metrics
    pub fn reset(&self) {
        self.total_enqueued.store(0, AtomicOrdering::Relaxed);
        self.total_dequeued.store(0, AtomicOrdering::Relaxed);
        self.total_completed.store(0, AtomicOrdering::Relaxed);
        self.total_failed.store(0, AtomicOrdering::Relaxed);
        self.total_retried.store(0, AtomicOrdering::Relaxed);
        self.total_cancelled.store(0, AtomicOrdering::Relaxed);
        self.current_size.store(0, AtomicOrdering::Relaxed);
        self.capacity_rejections.store(0, AtomicOrdering::Relaxed);
        self.deduplication_hits.store(0, AtomicOrdering::Relaxed);
        self.compression_bytes_saved.store(0, AtomicOrdering::Relaxed);
        self.encryption_operations.store(0, AtomicOrdering::Relaxed);
        self.persistence_operations.store(0, AtomicOrdering::Relaxed);
        self.persistence_failures.store(0, AtomicOrdering::Relaxed);
        self.heap_cleanups.store(0, AtomicOrdering::Relaxed);
        self.items_cleaned.store(0, AtomicOrdering::Relaxed);
        self.processing_time_total_ms.store(0, AtomicOrdering::Relaxed);
        self.queue_depth_max.store(0, AtomicOrdering::Relaxed);
        self.last_operation_time.store(0, AtomicOrdering::Relaxed);
    }
}

/// Immutable metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueMetricsSnapshot {
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_completed: u64,
    pub total_failed: u64,
    pub total_retried: u64,
    pub total_cancelled: u64,
    pub current_size: usize,
    pub capacity_rejections: u64,
    pub deduplication_hits: u64,
    pub compression_bytes_saved: u64,
    pub encryption_operations: u64,
    pub persistence_operations: u64,
    pub persistence_failures: u64,
    pub heap_cleanups: u64,
    pub items_cleaned: u64,
    pub processing_time_total_ms: u64,
    pub queue_depth_max: usize,
    pub last_operation_time: u64,
    pub average_processing_time_ms: f64,
    pub throughput: f64,
    pub success_rate: f64,
}

impl QueueMetricsSnapshot {
    /// Get a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "Queue Metrics:\n\
            - Current Size: {}/{}\n\
            - Total Enqueued: {}\n\
            - Total Processed: {} (Success: {:.1}%)\n\
            - Average Processing Time: {:.2}ms\n\
            - Throughput: {:.2} items/sec\n\
            - Deduplication Hits: {}\n\
            - Compression Savings: {} bytes",
            self.current_size,
            self.queue_depth_max,
            self.total_enqueued,
            self.total_completed + self.total_failed,
            self.success_rate,
            self.average_processing_time_ms,
            self.throughput,
            self.deduplication_hits,
            self.compression_bytes_saved
        )
    }
}
