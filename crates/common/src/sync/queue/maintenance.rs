use std::collections::{BinaryHeap, HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tracing::{debug, info, instrument, warn};

use crate::sync::queue::metrics::QueueMetrics;
use crate::sync::queue::types::{ItemStatus, SyncItem};

/// Priority queue item wrapper
#[derive(Clone)]
pub struct PriorityItem {
    pub item: Arc<SyncItem>,
    pub sequence: u64,
}

impl PartialEq for PriorityItem {
    fn eq(&self, other: &Self) -> bool {
        self.item.id == other.item.id
    }
}

impl Eq for PriorityItem {}

impl PartialOrd for PriorityItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PriorityItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Reverse ordering for min-heap behavior (higher priority first)
        other
            .item
            .priority
            .cmp(&self.item.priority)
            .then_with(|| other.sequence.cmp(&self.sequence))
    }
}

/// Mutable queue state for cleanup operations
///
/// Planned feature for queue maintenance - designed for background cleanup
/// tasks
#[allow(dead_code)] // Planned feature - will be integrated with background task scheduler
pub struct CleanupState<'a> {
    pub item_map: &'a mut HashMap<String, Arc<SyncItem>>,
    pub processing: &'a mut HashSet<String>,
    pub heap: &'a mut BinaryHeap<PriorityItem>,
    pub sequence_counter: &'a mut u64,
}

/// Maintenance service for queue operations
///
/// Production-ready maintenance service designed for:
/// - Heap cleanup of orphaned items
/// - Expired item removal based on retention policies
/// - Stuck item recovery
/// - Heap defragmentation
/// - Consistency validation
///
/// Planned integration with background task scheduler for periodic execution.
#[allow(dead_code)] // Planned feature - will be integrated with background task scheduler
pub struct MaintenanceService {
    metrics: Arc<QueueMetrics>,
    cleanup_interval: Duration,
    retention_period: Duration,
    heap_cleanup_threshold: usize,
}

impl MaintenanceService {
    /// Create new maintenance service
    #[allow(dead_code)] // Public API for maintenance service
    pub fn new(
        metrics: Arc<QueueMetrics>,
        cleanup_interval: Duration,
        retention_period: Duration,
        heap_cleanup_threshold: usize,
    ) -> Self {
        Self { metrics, cleanup_interval, retention_period, heap_cleanup_threshold }
    }

    /// Clean orphaned items from heap
    #[allow(dead_code)] // Public API for maintenance service
    #[instrument(skip(self, heap, item_map))]
    pub fn cleanup_heap(
        &self,
        heap: &mut BinaryHeap<PriorityItem>,
        item_map: &HashMap<String, Arc<SyncItem>>,
    ) -> usize {
        let original_size = heap.len();

        // Only cleanup if threshold exceeded
        if original_size < self.heap_cleanup_threshold {
            return 0;
        }

        let start = std::time::Instant::now();

        // Rebuild heap with only valid items
        let valid_items: Vec<PriorityItem> =
            heap.drain().filter(|p| item_map.contains_key(&p.item.id)).collect();

        for item in valid_items {
            heap.push(item);
        }

        let cleaned = original_size - heap.len();
        let duration = start.elapsed();

        if cleaned > 0 {
            self.metrics.record_heap_cleanup(cleaned);
            info!("Heap cleanup: removed {} orphaned items in {:?}", cleaned, duration);
        }

        cleaned
    }

    /// Remove expired items based on retention policy
    #[allow(dead_code)] // Public API for maintenance service
    #[instrument(skip(self, item_map, processing))]
    pub fn cleanup_expired_items(
        &self,
        item_map: &mut HashMap<String, Arc<SyncItem>>,
        processing: &mut HashSet<String>,
    ) -> usize {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        let cutoff = now - self.retention_period.as_secs();

        let expired_items: Vec<String> = item_map
            .iter()
            .filter(|(_, item)| item.created_at < cutoff && item.status == ItemStatus::Completed)
            .map(|(id, _)| id.clone())
            .collect();

        let count = expired_items.len();

        for id in expired_items {
            item_map.remove(&id);
            processing.remove(&id);
        }

        if count > 0 {
            info!("Cleaned up {} expired items", count);
        }

        count
    }

    /// Clean stuck processing items
    #[allow(dead_code)] // Public API for maintenance service
    #[instrument(skip(self, state))]
    pub fn cleanup_stuck_items(&self, state: &mut CleanupState<'_>, timeout: Duration) -> usize {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        let timeout_secs = timeout.as_secs();

        let stuck_items: Vec<String> = state
            .processing
            .iter()
            .filter_map(|id| {
                state.item_map.get(id).and_then(|item| {
                    item.processing_started_at.map(|started| {
                        if now - started > timeout_secs {
                            Some(id.clone())
                        } else {
                            None
                        }
                    })
                })
            })
            .flatten()
            .collect();

        let count = stuck_items.len();

        for id in stuck_items {
            state.processing.remove(&id);

            // Re-add to heap for retry
            if let Some(item_arc) = state.item_map.get(&id) {
                let mut updated_item = (**item_arc).clone();
                updated_item.status = ItemStatus::Pending;
                updated_item.processing_started_at = None;

                let new_arc = Arc::new(updated_item);
                state.item_map.insert(id.clone(), new_arc.clone());

                state.heap.push(PriorityItem { item: new_arc, sequence: *state.sequence_counter });
                *state.sequence_counter += 1;

                warn!("Reset stuck item: {}", id);
            }
        }

        if count > 0 {
            warn!("Reset {} stuck processing items", count);
        }

        count
    }

    /// Defragment heap (complete rebuild for optimal performance)
    #[allow(dead_code)] // Public API for maintenance service
    #[instrument(skip(self, heap))]
    pub fn defragment_heap(&self, heap: &mut BinaryHeap<PriorityItem>) {
        let items: Vec<PriorityItem> = heap.drain().collect();
        for item in items {
            heap.push(item);
        }
        debug!("Heap defragmented");
    }

    /// Validate queue consistency
    #[allow(dead_code)] // Public API for maintenance service
    #[instrument(skip(self, heap, item_map, processing))]
    pub fn validate_consistency(
        &self,
        heap: &BinaryHeap<PriorityItem>,
        item_map: &HashMap<String, Arc<SyncItem>>,
        processing: &HashSet<String>,
    ) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for items in processing that aren't in item_map
        for id in processing {
            if !item_map.contains_key(id) {
                issues.push(format!("Processing item {} not in item_map", id));
            }
        }

        // Check for duplicate sequences in heap
        let mut seen_sequences = HashSet::new();
        for item in heap.iter() {
            if !seen_sequences.insert(item.sequence) {
                issues.push(format!("Duplicate sequence {} in heap", item.sequence));
            }
        }

        // Check for items with invalid status
        for (id, item) in item_map {
            if item.status == ItemStatus::Processing && !processing.contains(id) {
                issues.push(format!("Item {} marked as processing but not in processing set", id));
            }
        }

        if !issues.is_empty() {
            warn!("Queue consistency issues: {:?}", issues);
        }

        issues
    }

    /// Run all maintenance tasks
    #[allow(dead_code)] // Public API for maintenance service
    #[instrument(skip(self, state))]
    pub fn run_maintenance(&self, state: &mut QueueState) -> MaintenanceReport {
        let start = std::time::Instant::now();

        let heap_cleaned = self.cleanup_heap(&mut state.items, &state.item_map);
        let expired_cleaned =
            self.cleanup_expired_items(&mut state.item_map, &mut state.processing);

        let mut cleanup_state = CleanupState {
            item_map: &mut state.item_map,
            processing: &mut state.processing,
            heap: &mut state.items,
            sequence_counter: &mut state.sequence_counter,
        };
        let stuck_reset = self.cleanup_stuck_items(
            &mut cleanup_state,
            Duration::from_secs(300), // 5 minute timeout
        );

        // Defragment if significant changes
        if heap_cleaned > 100 || expired_cleaned > 50 {
            self.defragment_heap(&mut state.items);
        }

        let consistency_issues =
            self.validate_consistency(&state.items, &state.item_map, &state.processing);

        let report = MaintenanceReport {
            heap_items_cleaned: heap_cleaned,
            expired_items_removed: expired_cleaned,
            stuck_items_reset: stuck_reset,
            consistency_issues: consistency_issues.len(),
            duration: start.elapsed(),
        };

        if report.total_actions() > 0 {
            info!("Maintenance completed: {}", report.summary());
        }

        report
    }
}

/// Queue state for maintenance operations
pub struct QueueState {
    pub items: BinaryHeap<PriorityItem>,
    pub item_map: HashMap<String, Arc<SyncItem>>,
    pub processing: HashSet<String>,
    pub sequence_counter: u64,
    pub is_locked: bool,
}

/// Maintenance operation report
#[derive(Debug, Clone)]
#[allow(dead_code)] // Planned feature - return type for maintenance operations
pub struct MaintenanceReport {
    pub heap_items_cleaned: usize,
    pub expired_items_removed: usize,
    pub stuck_items_reset: usize,
    pub consistency_issues: usize,
    pub duration: Duration,
}

impl MaintenanceReport {
    /// Get total number of maintenance actions
    #[allow(dead_code)] // Public API for report analysis
    pub fn total_actions(&self) -> usize {
        self.heap_items_cleaned
            + self.expired_items_removed
            + self.stuck_items_reset
            + self.consistency_issues
    }

    /// Get summary string
    #[allow(dead_code)] // Public API for report formatting
    pub fn summary(&self) -> String {
        format!(
            "heap_cleaned={}, expired_removed={}, stuck_reset={}, issues={}, duration={:?}",
            self.heap_items_cleaned,
            self.expired_items_removed,
            self.stuck_items_reset,
            self.consistency_issues,
            self.duration
        )
    }
}
