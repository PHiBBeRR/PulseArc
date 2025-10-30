//! Application context - dependency injection container

use pulsearc_core::TrackingService;
use pulsearc_infra::{DbManager, InstanceLock, MacOsActivityProvider, SqliteActivityRepository};
// TODO: Re-add KeyManager when encryption is re-enabled
// use pulsearc_infra::KeyManager;
use pulsearc_shared::{Config, Result};
use std::sync::Arc;

/// Application context - holds all services and dependencies
pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    // Keep instance lock alive for the lifetime of the app
    _instance_lock: InstanceLock,
}

impl AppContext {
    /// Create a new application context
    pub fn new() -> Result<Self> {
        let config = Config::default();

        // Acquire instance lock to prevent multiple instances
        // Use system temp directory for PID file to avoid triggering file watchers
        let lock_dir = std::env::temp_dir();
        let instance_lock = InstanceLock::acquire(&lock_dir)?;

        // Get encryption key from environment variable
        let encryption_key = std::env::var("DATABASE_ENCRYPTION_KEY").ok();

        // Initialize database with encryption
        let db = Arc::new(DbManager::new(
            &config.database.path,
            config.database.pool_size,
            encryption_key.as_deref(),
        )?);

        // Run migrations
        db.run_migrations()?;

        // Initialize activity provider
        let provider = Arc::new(MacOsActivityProvider::new());

        // Initialize activity repository
        let repository = Arc::new(SqliteActivityRepository::new(db.clone()));

        // Create tracking service
        let tracking_service = Arc::new(TrackingService::new(provider, repository));

        Ok(Self {
            config,
            db,
            tracking_service,
            _instance_lock: instance_lock,
        })
    }
}
