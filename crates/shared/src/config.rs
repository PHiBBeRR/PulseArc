//! Configuration management

use serde::{Deserialize, Serialize};

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub sync: SyncConfig,
    pub tracking: TrackingConfig,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub path: String,
    pub pool_size: u32,
    #[serde(skip_serializing)]
    pub encryption_key: Option<String>,
}

/// Sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    pub interval_seconds: u64,
    pub enabled: bool,
}

/// Activity tracking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingConfig {
    pub snapshot_interval_seconds: u64,
    pub idle_threshold_seconds: u64,
    pub enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database: DatabaseConfig {
                path: "pulsearc.db".to_string(),
                pool_size: 8,
                encryption_key: None,
            },
            sync: SyncConfig {
                interval_seconds: 10,
                enabled: true,
            },
            tracking: TrackingConfig {
                snapshot_interval_seconds: 30,
                idle_threshold_seconds: 300,
                enabled: true,
            },
        }
    }
}
