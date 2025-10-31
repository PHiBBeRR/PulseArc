use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Sync item priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Critical = 0,
    High = 1,
    Normal = 2,
    Low = 3,
    Background = 4,
}

impl From<u8> for Priority {
    fn from(value: u8) -> Self {
        match value {
            0 => Priority::Critical,
            1 => Priority::High,
            2 => Priority::Normal,
            3 => Priority::Low,
            _ => Priority::Background,
        }
    }
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Priority::Critical => write!(f, "Critical"),
            Priority::High => write!(f, "High"),
            Priority::Normal => write!(f, "Normal"),
            Priority::Low => write!(f, "Low"),
            Priority::Background => write!(f, "Background"),
        }
    }
}

/// Item status in the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemStatus {
    Pending,
    Processing,
    Failed,
    Completed,
    Cancelled,
    Scheduled,
}

/// Synchronization item with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncItem {
    pub id: String,
    pub priority: Priority,
    pub data: serde_json::Value,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: u64,
    pub updated_at: u64,
    /// Next retry timestamp in milliseconds since Unix epoch
    pub next_retry_at: Option<u128>,
    pub status: ItemStatus,
    pub error_message: Option<String>,
    pub metadata: HashMap<String, String>,
    pub processing_started_at: Option<u64>,
    pub processing_duration_ms: Option<u64>,
    pub correlation_id: Option<String>,
    pub partition_key: Option<String>,
}

impl SyncItem {
    /// Create a new sync item
    pub fn new(data: serde_json::Value, priority: Priority) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        Self {
            id: Uuid::new_v4().to_string(),
            priority,
            data,
            retry_count: 0,
            max_retries: 5,
            created_at: now,
            updated_at: now,
            next_retry_at: None,
            status: ItemStatus::Pending,
            error_message: None,
            metadata: HashMap::new(),
            processing_started_at: None,
            processing_duration_ms: None,
            correlation_id: None,
            partition_key: None,
        }
    }

    /// Create with specific ID
    pub fn with_id(id: String, data: serde_json::Value, priority: Priority) -> Self {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

        Self {
            id,
            priority,
            data,
            retry_count: 0,
            max_retries: 5,
            created_at: now,
            updated_at: now,
            next_retry_at: None,
            status: ItemStatus::Pending,
            error_message: None,
            metadata: HashMap::new(),
            processing_started_at: None,
            processing_duration_ms: None,
            correlation_id: None,
            partition_key: None,
        }
    }

    /// Set maximum retry attempts
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Set correlation ID for tracing
    pub fn with_correlation_id(mut self, id: String) -> Self {
        self.correlation_id = Some(id);
        self
    }

    /// Set partition key for sharding
    pub fn with_partition_key(mut self, key: String) -> Self {
        self.partition_key = Some(key);
        self
    }

    /// Check if item can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries && self.status != ItemStatus::Cancelled
    }

    /// Calculate next retry time with exponential backoff and jitter.
    ///
    /// Returns an epoch timestamp in milliseconds.
    pub fn calculate_next_retry(&self, base_delay: Duration) -> u128 {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();

        let base_ms = base_delay.as_millis().max(1);

        // Prevent overflow by capping the exponent
        let exp = self.retry_count.min(10);
        let multiplier = 2_u128.saturating_pow(exp);
        let backoff = base_ms.saturating_mul(multiplier);

        // Add jitter (0-25% of backoff)
        let jitter_bound = backoff / 4;
        let jitter_value = if jitter_bound > 0 {
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos()
                % (jitter_bound + 1)
        } else {
            0
        };

        now.saturating_add(backoff).saturating_add(jitter_value)
    }

    /// Mark item as processing
    pub fn mark_processing(&mut self) {
        self.status = ItemStatus::Processing;
        self.processing_started_at =
            Some(SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs());
        self.updated_at =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    }

    /// Mark item as completed
    pub fn mark_completed(&mut self) {
        self.status = ItemStatus::Completed;
        if let Some(started) = self.processing_started_at {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            self.processing_duration_ms = Some((now - started) * 1000);
        }
        self.updated_at =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    }

    /// Mark item as failed
    pub fn mark_failed(&mut self, error: Option<String>) {
        self.status = ItemStatus::Failed;
        self.error_message = error;
        self.retry_count += 1;
        if let Some(started) = self.processing_started_at {
            let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            self.processing_duration_ms = Some((now - started) * 1000);
        }
        self.updated_at =
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
    }
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub max_capacity: usize,
    pub batch_size: usize,
    pub persistence_path: Option<PathBuf>,
    pub persistence_interval: Duration,
    pub enable_deduplication: bool,
    pub enable_compression: bool,
    pub compression_level: u32,
    pub enable_encryption: bool,
    pub encryption_key: Option<Vec<u8>>,
    pub retention_period: Duration,
    pub base_retry_delay: Duration,
    pub max_retry_delay: Duration,
    pub cleanup_interval: Duration,
    pub heap_cleanup_threshold: usize,
    pub enable_partitioning: bool,
    pub partition_count: usize,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_capacity: 10_000,
            batch_size: 100,
            persistence_path: None,
            persistence_interval: Duration::from_secs(30),
            enable_deduplication: true,
            enable_compression: true,
            compression_level: 6,
            enable_encryption: false,
            encryption_key: None,
            retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            base_retry_delay: Duration::from_secs(1),
            max_retry_delay: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            heap_cleanup_threshold: 1000,
            enable_partitioning: false,
            partition_count: 4,
        }
    }
}

impl QueueConfig {
    /// Create a high-performance configuration
    pub fn high_performance() -> Self {
        Self {
            max_capacity: 100_000,
            batch_size: 1000,
            persistence_interval: Duration::from_secs(60),
            enable_compression: false,
            enable_encryption: false,
            cleanup_interval: Duration::from_secs(600),
            heap_cleanup_threshold: 5000,
            ..Default::default()
        }
    }

    /// Create a high-security configuration
    pub fn high_security() -> Self {
        Self {
            enable_encryption: true,
            enable_compression: true,
            compression_level: 9,
            persistence_interval: Duration::from_secs(10),
            ..Default::default()
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.max_capacity == 0 {
            return Err("Max capacity must be greater than 0".to_string());
        }

        if self.base_retry_delay.as_millis() == 0 {
            return Err("Base retry delay must be greater than 0".to_string());
        }

        if self.batch_size > self.max_capacity {
            return Err("Batch size cannot exceed max capacity".to_string());
        }

        if self.enable_encryption && self.encryption_key.is_none() {
            return Err("Encryption key required when encryption is enabled".to_string());
        }

        if let Some(ref key) = self.encryption_key {
            if key.len() != 32 {
                return Err("Encryption key must be 32 bytes".to_string());
            }
        }

        if self.compression_level > 9 {
            return Err("Compression level must be between 0 and 9".to_string());
        }

        if self.enable_partitioning && self.partition_count == 0 {
            return Err("Partition count must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    //! Unit tests for sync::queue::types.
    use super::*;

    /// Tests priority enum ordering for queue processing.
    ///
    /// Verifies:
    /// - Critical has highest priority (lowest value)
    /// - Priority levels follow: Critical < High < Normal < Low < Background
    /// - Ordering enables correct queue dequeue behavior
    #[test]
    fn test_priority_ordering() {
        assert!(Priority::Critical < Priority::High);
        assert!(Priority::High < Priority::Normal);
        assert!(Priority::Normal < Priority::Low);
        assert!(Priority::Low < Priority::Background);
    }

    /// Validates `Priority::from` behavior for the priority from u8 scenario.
    ///
    /// Assertions:
    /// - Confirms `Priority::from(0)` equals `Priority::Critical`.
    /// - Confirms `Priority::from(1)` equals `Priority::High`.
    /// - Confirms `Priority::from(2)` equals `Priority::Normal`.
    /// - Confirms `Priority::from(3)` equals `Priority::Low`.
    /// - Confirms `Priority::from(4)` equals `Priority::Background`.
    /// - Confirms `Priority::from(99)` equals `Priority::Background`.
    #[test]
    fn test_priority_from_u8() {
        assert_eq!(Priority::from(0), Priority::Critical);
        assert_eq!(Priority::from(1), Priority::High);
        assert_eq!(Priority::from(2), Priority::Normal);
        assert_eq!(Priority::from(3), Priority::Low);
        assert_eq!(Priority::from(4), Priority::Background);
        assert_eq!(Priority::from(99), Priority::Background); // Default
    }

    /// Validates `Priority::Critical` behavior for the priority display
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `Priority::Critical.to_string()` equals `"Critical"`.
    /// - Confirms `Priority::High.to_string()` equals `"High"`.
    /// - Confirms `Priority::Normal.to_string()` equals `"Normal"`.
    /// - Confirms `Priority::Low.to_string()` equals `"Low"`.
    /// - Confirms `Priority::Background.to_string()` equals `"Background"`.
    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::Critical.to_string(), "Critical");
        assert_eq!(Priority::High.to_string(), "High");
        assert_eq!(Priority::Normal.to_string(), "Normal");
        assert_eq!(Priority::Low.to_string(), "Low");
        assert_eq!(Priority::Background.to_string(), "Background");
    }

    /// Validates `SyncItem::new` behavior for the sync item new scenario.
    ///
    /// Assertions:
    /// - Confirms `item.priority` equals `Priority::High`.
    /// - Confirms `item.data` equals `data`.
    /// - Confirms `item.retry_count` equals `0`.
    /// - Confirms `item.max_retries` equals `5`.
    /// - Confirms `item.status` equals `ItemStatus::Pending`.
    /// - Ensures `item.error_message.is_none()` evaluates to true.
    /// - Ensures `item.correlation_id.is_none()` evaluates to true.
    #[test]
    fn test_sync_item_new() {
        let data = serde_json::json!({"test": "data"});
        let item = SyncItem::new(data.clone(), Priority::High);

        assert_eq!(item.priority, Priority::High);
        assert_eq!(item.data, data);
        assert_eq!(item.retry_count, 0);
        assert_eq!(item.max_retries, 5);
        assert_eq!(item.status, ItemStatus::Pending);
        assert!(item.error_message.is_none());
        assert!(item.correlation_id.is_none());
    }

    /// Validates `SyncItem::with_id` behavior for the sync item with id
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item.id` equals `id`.
    /// - Confirms `item.priority` equals `Priority::Normal`.
    /// - Confirms `item.data` equals `data`.
    #[test]
    fn test_sync_item_with_id() {
        let data = serde_json::json!({"test": "data"});
        let id = "custom-id-123".to_string();
        let item = SyncItem::with_id(id.clone(), data.clone(), Priority::Normal);

        assert_eq!(item.id, id);
        assert_eq!(item.priority, Priority::Normal);
        assert_eq!(item.data, data);
    }

    /// Validates `SyncItem::new` behavior for the sync item with max retries
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item.max_retries` equals `10`.
    #[test]
    fn test_sync_item_with_max_retries() {
        let data = serde_json::json!({"test": "data"});
        let item = SyncItem::new(data, Priority::High).with_max_retries(10);

        assert_eq!(item.max_retries, 10);
    }

    /// Validates `SyncItem::new` behavior for the sync item with metadata
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item.metadata.get("key1").unwrap()` equals `"value1"`.
    /// - Confirms `item.metadata.get("key2").unwrap()` equals `"value2"`.
    #[test]
    fn test_sync_item_with_metadata() {
        let data = serde_json::json!({"test": "data"});
        let item = SyncItem::new(data, Priority::High)
            .with_metadata("key1".to_string(), "value1".to_string())
            .with_metadata("key2".to_string(), "value2".to_string());

        assert_eq!(item.metadata.get("key1").unwrap(), "value1");
        assert_eq!(item.metadata.get("key2").unwrap(), "value2");
    }

    /// Validates `SyncItem::new` behavior for the sync item with correlation id
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item.correlation_id` equals `Some(correlation_id)`.
    #[test]
    fn test_sync_item_with_correlation_id() {
        let data = serde_json::json!({"test": "data"});
        let correlation_id = "trace-123".to_string();
        let item = SyncItem::new(data, Priority::High).with_correlation_id(correlation_id.clone());

        assert_eq!(item.correlation_id, Some(correlation_id));
    }

    /// Validates `SyncItem::new` behavior for the sync item with partition key
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item.partition_key` equals `Some(partition_key)`.
    #[test]
    fn test_sync_item_with_partition_key() {
        let data = serde_json::json!({"test": "data"});
        let partition_key = "user-456".to_string();
        let item = SyncItem::new(data, Priority::High).with_partition_key(partition_key.clone());

        assert_eq!(item.partition_key, Some(partition_key));
    }

    /// Validates `SyncItem::new` behavior for the can retry scenario.
    ///
    /// Assertions:
    /// - Ensures `item.can_retry()` evaluates to true.
    /// - Ensures `!item.can_retry()` evaluates to true.
    /// - Ensures `!item.can_retry()` evaluates to true.
    #[test]
    fn test_can_retry() {
        let data = serde_json::json!({"test": "data"});
        let mut item = SyncItem::new(data, Priority::High);

        assert!(item.can_retry());

        item.retry_count = 5;
        assert!(!item.can_retry());

        item.retry_count = 3;
        item.status = ItemStatus::Cancelled;
        assert!(!item.can_retry());
    }

    /// Validates `SyncItem::new` behavior for the calculate next retry
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `retry1 > 0` evaluates to true.
    /// - Ensures `retry2 > retry1` evaluates to true.
    /// - Ensures `retry3 > retry2` evaluates to true.
    #[test]
    fn test_calculate_next_retry() {
        let data = serde_json::json!({"test": "data"});
        let mut item = SyncItem::new(data, Priority::High);

        let base_delay = Duration::from_millis(250);
        let retry1 = item.calculate_next_retry(base_delay);

        item.retry_count = 1;
        let retry2 = item.calculate_next_retry(base_delay);

        item.retry_count = 2;
        let retry3 = item.calculate_next_retry(base_delay);

        // Each retry should have longer delay (with jitter variance)
        assert!(retry1 > 0);
        assert!(retry2 > retry1);
        assert!(retry3 > retry2);
    }

    /// Validates `SyncItem::new` behavior for the mark processing scenario.
    ///
    /// Assertions:
    /// - Confirms `item.status` equals `ItemStatus::Processing`.
    /// - Ensures `item.processing_started_at.is_some()` evaluates to true.
    #[test]
    fn test_mark_processing() {
        let data = serde_json::json!({"test": "data"});
        let mut item = SyncItem::new(data, Priority::High);

        item.mark_processing();

        assert_eq!(item.status, ItemStatus::Processing);
        assert!(item.processing_started_at.is_some());
    }

    /// Validates `SyncItem::new` behavior for the mark completed scenario.
    ///
    /// Assertions:
    /// - Confirms `item.status` equals `ItemStatus::Completed`.
    /// - Ensures `item.processing_duration_ms.is_some()` evaluates to true.
    #[test]
    fn test_mark_completed() {
        // Unit test: verify status transitions and fields are set
        let data = serde_json::json!({"test": "data"});
        let mut item = SyncItem::new(data, Priority::High);

        item.mark_processing();
        item.mark_completed();

        assert_eq!(item.status, ItemStatus::Completed);
        assert!(item.processing_duration_ms.is_some());
    }

    /// Validates `SyncItem::new` behavior for the mark completed with duration
    /// scenario.
    ///
    /// Assertions:
    /// - Ensures `duration >= 1000` evaluates to true.
    #[test]
    #[ignore] // Integration test: tests actual time calculation
    fn test_mark_completed_with_duration() {
        let data = serde_json::json!({"test": "data"});
        let mut item = SyncItem::new(data, Priority::High);

        item.mark_processing();
        std::thread::sleep(std::time::Duration::from_secs(1));
        item.mark_completed();

        let duration = item.processing_duration_ms.unwrap();
        assert!(duration >= 1000, "Expected at least 1000ms, got {}", duration);
    }

    /// Validates `SyncItem::new` behavior for the mark failed scenario.
    ///
    /// Assertions:
    /// - Confirms `item.status` equals `ItemStatus::Failed`.
    /// - Confirms `item.error_message` equals `Some("Network
    ///   error".to_string())`.
    /// - Confirms `item.retry_count` equals `initial_retry_count + 1`.
    #[test]
    fn test_mark_failed() {
        let data = serde_json::json!({"test": "data"});
        let mut item = SyncItem::new(data, Priority::High);

        let initial_retry_count = item.retry_count;
        item.mark_processing();
        item.mark_failed(Some("Network error".to_string()));

        assert_eq!(item.status, ItemStatus::Failed);
        assert_eq!(item.error_message, Some("Network error".to_string()));
        assert_eq!(item.retry_count, initial_retry_count + 1);
    }

    /// Validates `QueueConfig::default` behavior for the queue config default
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_capacity` equals `10_000`.
    /// - Confirms `config.batch_size` equals `100`.
    /// - Ensures `config.enable_deduplication` evaluates to true.
    /// - Ensures `config.enable_compression` evaluates to true.
    /// - Confirms `config.compression_level` equals `6`.
    /// - Ensures `!config.enable_encryption` evaluates to true.
    #[test]
    fn test_queue_config_default() {
        let config = QueueConfig::default();

        assert_eq!(config.max_capacity, 10_000);
        assert_eq!(config.batch_size, 100);
        assert!(config.enable_deduplication);
        assert!(config.enable_compression);
        assert_eq!(config.compression_level, 6);
        assert!(!config.enable_encryption);
    }

    /// Validates `QueueConfig::high_performance` behavior for the queue config
    /// high performance scenario.
    ///
    /// Assertions:
    /// - Confirms `config.max_capacity` equals `100_000`.
    /// - Confirms `config.batch_size` equals `1000`.
    /// - Ensures `!config.enable_compression` evaluates to true.
    /// - Ensures `!config.enable_encryption` evaluates to true.
    #[test]
    fn test_queue_config_high_performance() {
        let config = QueueConfig::high_performance();

        assert_eq!(config.max_capacity, 100_000);
        assert_eq!(config.batch_size, 1000);
        assert!(!config.enable_compression);
        assert!(!config.enable_encryption);
    }

    /// Validates `QueueConfig::high_security` behavior for the queue config
    /// high security scenario.
    ///
    /// Assertions:
    /// - Ensures `config.enable_encryption` evaluates to true.
    /// - Ensures `config.enable_compression` evaluates to true.
    /// - Confirms `config.compression_level` equals `9`.
    #[test]
    fn test_queue_config_high_security() {
        let config = QueueConfig::high_security();

        assert!(config.enable_encryption);
        assert!(config.enable_compression);
        assert_eq!(config.compression_level, 9);
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// success scenario.
    ///
    /// Assertions:
    /// - Ensures `config.validate().is_ok()` evaluates to true.
    #[test]
    fn test_queue_config_validate_success() {
        let config = QueueConfig::default();
        assert!(config.validate().is_ok());
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// zero capacity scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("Max capacity")` evaluates to
    ///   true.
    #[test]
    fn test_queue_config_validate_zero_capacity() {
        let config = QueueConfig { max_capacity: 0, ..QueueConfig::default() };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Max capacity"));
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// batch size exceeds capacity scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("Batch size")` evaluates to
    ///   true.
    #[test]
    fn test_queue_config_validate_batch_size_exceeds_capacity() {
        let config = QueueConfig { max_capacity: 100, batch_size: 200, ..QueueConfig::default() };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Batch size"));
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// encryption without key scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("Encryption key required")`
    ///   evaluates to true.
    #[test]
    fn test_queue_config_validate_encryption_without_key() {
        let config = QueueConfig { enable_encryption: true, ..QueueConfig::default() };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Encryption key required"));
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// invalid key length scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("32 bytes")` evaluates to true.
    #[test]
    fn test_queue_config_validate_invalid_key_length() {
        let config = QueueConfig {
            enable_encryption: true,
            encryption_key: Some(vec![0u8; 16]), // Wrong size
            ..QueueConfig::default()
        };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("32 bytes"));
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// invalid compression level scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("Compression level")` evaluates
    ///   to true.
    #[test]
    fn test_queue_config_validate_invalid_compression_level() {
        let config = QueueConfig { compression_level: 10, ..QueueConfig::default() };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Compression level"));
    }

    /// Validates `QueueConfig::default` behavior for the queue config validate
    /// partitioning zero count scenario.
    ///
    /// Assertions:
    /// - Ensures `result.is_err()` evaluates to true.
    /// - Ensures `result.unwrap_err().contains("Partition count")` evaluates to
    ///   true.
    #[test]
    fn test_queue_config_validate_partitioning_zero_count() {
        let config =
            QueueConfig { enable_partitioning: true, partition_count: 0, ..QueueConfig::default() };

        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Partition count"));
    }

    /// Validates `SyncItem::new` behavior for the sync item serialization
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `item.id` equals `deserialized.id`.
    /// - Confirms `item.priority` equals `deserialized.priority`.
    /// - Confirms `item.data` equals `deserialized.data`.
    #[test]
    fn test_sync_item_serialization() {
        let data = serde_json::json!({"test": "data"});
        let item = SyncItem::new(data, Priority::High);

        let serialized = serde_json::to_string(&item).unwrap();
        let deserialized: SyncItem = serde_json::from_str(&serialized).unwrap();

        assert_eq!(item.id, deserialized.id);
        assert_eq!(item.priority, deserialized.priority);
        assert_eq!(item.data, deserialized.data);
    }

    /// Validates `ItemStatus::Pending` behavior for the item status equality
    /// scenario.
    ///
    /// Assertions:
    /// - Confirms `ItemStatus::Pending` equals `ItemStatus::Pending`.
    /// - Confirms `ItemStatus::Pending` differs from `ItemStatus::Processing`.
    #[test]
    fn test_item_status_equality() {
        assert_eq!(ItemStatus::Pending, ItemStatus::Pending);
        assert_ne!(ItemStatus::Pending, ItemStatus::Processing);
    }
}
