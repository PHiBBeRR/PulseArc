use std::path::{Path, PathBuf};
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{debug, info, instrument, warn};

use crate::error::CommonError;
use crate::sync::queue::compression::{CompressionAlgorithm, CompressionService};
use crate::sync::queue::encryption::EncryptionService;
use crate::sync::queue::errors::{QueueError, QueueResult};
use crate::sync::queue::metrics::QueueMetrics;
use crate::sync::queue::types::SyncItem;
use crate::EncryptedData;

/// Persistence format version
const PERSISTENCE_VERSION: u32 = 1;

/// Persistence metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistenceMetadata {
    pub version: u32,
    pub created_at: u64,
    pub item_count: usize,
    pub compressed: bool,
    pub encrypted: bool,
    pub compression_algorithm: Option<String>,
    pub encryption_algorithm: Option<String>,
    pub checksum: Option<String>,
}

/// Persisted queue data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedQueue {
    pub metadata: PersistenceMetadata,
    pub items: Vec<SyncItem>,
}

/// Queue persistence service
pub struct PersistenceService {
    path: PathBuf,
    compression: Option<CompressionService>,
    encryption: Option<EncryptionService>,
    metrics: Option<std::sync::Arc<QueueMetrics>>,
}

impl PersistenceService {
    /// Create new persistence service
    pub fn new(path: PathBuf) -> Self {
        Self { path, compression: None, encryption: None, metrics: None }
    }

    /// Enable compression
    pub fn with_compression(mut self, level: u32) -> Self {
        self.compression = Some(CompressionService::new(CompressionAlgorithm::Gzip, level));
        self
    }

    /// Enable encryption
    pub fn with_encryption(mut self, key: Vec<u8>) -> QueueResult<Self> {
        self.encryption = Some(EncryptionService::new(key)?);
        Ok(self)
    }

    /// Set metrics reference
    pub fn with_metrics(mut self, metrics: std::sync::Arc<QueueMetrics>) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Save queue items to disk
    #[instrument(skip(self, items), fields(item_count = items.len()))]
    pub async fn save(&self, items: Vec<SyncItem>) -> QueueResult<()> {
        let start = std::time::Instant::now();

        // Create metadata
        let metadata = PersistenceMetadata {
            version: PERSISTENCE_VERSION,
            created_at: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            item_count: items.len(),
            compressed: self.compression.is_some(),
            encrypted: self.encryption.is_some(),
            compression_algorithm: self.compression.as_ref().map(|_| "gzip".to_string()),
            encryption_algorithm: self.encryption.as_ref().map(|_| "AES-256-GCM".to_string()),
            checksum: None,
        };

        let queue_data = PersistedQueue { metadata, items };

        // Serialize to JSON
        let mut data = serde_json::to_vec(&queue_data)?;
        let original_size = data.len();

        // Apply compression if enabled
        if let Some(ref compression) = self.compression {
            data = compression.compress(&data)?;
            let saved = original_size.saturating_sub(data.len()) as u64;
            if let Some(ref metrics) = self.metrics {
                metrics.record_compression_savings(saved);
            }
            debug!("Compressed {} bytes to {} bytes", original_size, data.len());
        }

        // Apply encryption if enabled
        if let Some(ref encryption) = self.encryption {
            let encrypted = encryption.encrypt(&data)?;
            data = serde_json::to_vec(&encrypted)?;
            if let Some(ref metrics) = self.metrics {
                metrics.record_encryption();
            }
            debug!("Encrypted {} bytes", data.len());
        }

        // Calculate checksum
        let checksum = self.calculate_checksum(&data);

        // Write to temporary file first for atomicity
        let temp_path = self.path.with_extension("tmp");

        // Ensure parent directory exists
        if let Some(parent) = temp_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Write data
        let mut file =
            fs::OpenOptions::new().write(true).create(true).truncate(true).open(&temp_path).await?;

        file.write_all(&data).await?;
        file.sync_all().await?;
        drop(file);

        // Atomic rename
        fs::rename(&temp_path, &self.path).await?;

        // Write checksum file
        if let Some(checksum) = checksum {
            let checksum_path = self.path.with_extension("sha256");
            fs::write(&checksum_path, checksum).await.ok();
        }

        let duration = start.elapsed();
        if let Some(ref metrics) = self.metrics {
            metrics.record_persistence(true);
        }

        info!(
            "Persisted {} items in {:?} ({} bytes)",
            queue_data.items.len(),
            duration,
            data.len()
        );

        Ok(())
    }

    /// Load queue items from disk
    #[instrument(skip(self))]
    pub async fn load(&self) -> QueueResult<Vec<SyncItem>> {
        let start = std::time::Instant::now();

        if !self.path.exists() {
            debug!("Persistence file does not exist: {:?}", self.path);
            return Ok(Vec::new());
        }

        // Read data
        let mut data = fs::read(&self.path).await?;

        // Verify checksum if available
        let checksum_path = self.path.with_extension("sha256");
        if checksum_path.exists() {
            let expected = fs::read_to_string(&checksum_path).await.ok();
            if let Some(expected) = expected {
                let actual = self.calculate_checksum(&data);
                if actual.as_ref() != Some(&expected) {
                    warn!("Checksum mismatch, file may be corrupted");
                }
            }
        }

        // Decrypt if needed
        if self.encryption.is_some() {
            let encrypted: EncryptedData = serde_json::from_slice(&data)?;
            if let Some(ref encryption) = self.encryption {
                data = encryption.decrypt(&encrypted)?;
                debug!("Decrypted {} bytes", data.len());
            }
        }

        // Decompress if needed
        if self.compression.is_some() {
            if let Some(ref compression) = self.compression {
                data = compression.decompress(&data)?;
                debug!("Decompressed to {} bytes", data.len());
            }
        }

        // Deserialize
        let queue_data: PersistedQueue = serde_json::from_slice(&data)?;

        // Validate version
        if queue_data.metadata.version != PERSISTENCE_VERSION {
            warn!(
                "Persistence version mismatch: expected {}, got {}",
                PERSISTENCE_VERSION, queue_data.metadata.version
            );
        }

        let duration = start.elapsed();
        if let Some(ref metrics) = self.metrics {
            metrics.record_persistence(true);
        }

        info!("Loaded {} items in {:?}", queue_data.items.len(), duration);

        Ok(queue_data.items)
    }

    /// Delete persistence file
    #[allow(dead_code)] // Public API for queue management
    pub async fn delete(&self) -> QueueResult<()> {
        if self.path.exists() {
            fs::remove_file(&self.path).await?;
            debug!("Deleted persistence file: {:?}", self.path);
        }

        let checksum_path = self.path.with_extension("sha256");
        if checksum_path.exists() {
            fs::remove_file(&checksum_path).await.ok();
        }

        Ok(())
    }

    /// Create backup of persistence file
    #[allow(dead_code)] // Public API
    pub async fn backup(&self) -> QueueResult<PathBuf> {
        let timestamp =
            SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs();

        let backup_path = self.path.with_extension(format!("backup.{}", timestamp));

        if self.path.exists() {
            fs::copy(&self.path, &backup_path).await?;
            info!("Created backup: {:?}", backup_path);
        }

        Ok(backup_path)
    }

    /// Restore from backup
    #[allow(dead_code)] // Public API
    pub async fn restore(&self, backup_path: &Path) -> QueueResult<()> {
        if !backup_path.exists() {
            return Err(QueueError::Common(CommonError::persistence_op(
                "queue_restore",
                format!("Backup file not found: {:?}", backup_path),
            )));
        }

        // Create backup of current file if it exists
        if self.path.exists() {
            let current_backup = self.path.with_extension("backup.current");
            fs::copy(&self.path, &current_backup).await?;
        }

        // Restore from backup
        fs::copy(backup_path, &self.path).await?;
        info!("Restored from backup: {:?}", backup_path);

        Ok(())
    }

    /// List available backups
    #[allow(dead_code)] // Public API
    pub async fn list_backups(&self) -> QueueResult<Vec<PathBuf>> {
        let parent = self
            .path
            .parent()
            .ok_or_else(|| CommonError::persistence_op("queue_restore", "Invalid path"))?;

        let stem = self
            .path
            .file_stem()
            .ok_or_else(|| CommonError::persistence_op("queue_restore", "Invalid filename"))?;

        let mut backups = Vec::new();
        let mut entries = fs::read_dir(parent).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(filename) = path.file_name() {
                let filename = filename.to_string_lossy();
                if filename.starts_with(stem.to_string_lossy().as_ref())
                    && filename.contains(".backup.")
                {
                    backups.push(path);
                }
            }
        }

        backups.sort_by(|a, b| b.cmp(a)); // Sort newest first
        Ok(backups)
    }

    /// Clean old backups
    #[allow(dead_code)] // Public API
    pub async fn cleanup_backups(&self, keep_count: usize) -> QueueResult<usize> {
        let backups = self.list_backups().await?;
        let mut deleted = 0;

        for backup in backups.iter().skip(keep_count) {
            if let Err(e) = fs::remove_file(backup).await {
                warn!("Failed to delete backup {:?}: {}", backup, e);
            } else {
                deleted += 1;
                debug!("Deleted old backup: {:?}", backup);
            }
        }

        if deleted > 0 {
            info!("Cleaned up {} old backups", deleted);
        }

        Ok(deleted)
    }

    /// Calculate SHA256 checksum
    fn calculate_checksum(&self, data: &[u8]) -> Option<String> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Some(format!("{:x}", result))
    }

    /// Verify file integrity
    #[allow(dead_code)] // Public API
    pub async fn verify_integrity(&self) -> QueueResult<bool> {
        if !self.path.exists() {
            return Ok(false);
        }

        let checksum_path = self.path.with_extension("sha256");
        if !checksum_path.exists() {
            warn!("No checksum file found");
            return Ok(false);
        }

        let data = fs::read(&self.path).await?;
        let expected = fs::read_to_string(&checksum_path).await?;
        let actual = self.calculate_checksum(&data);

        Ok(actual == Some(expected))
    }
}
