//! Application context - dependency injection container

use std::sync::Arc;

use pulsearc_core::TrackingService;
use pulsearc_domain::{Config, Result};
use pulsearc_infra::{
    DbManager, FeatureFlagService, InstanceLock, KeyManager, MacOsActivityProvider,
    SqlCipherActivityRepository,
};

/// Application context - holds all services and dependencies
pub struct AppContext {
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<FeatureFlagService>,
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

        // Resolve encryption key from environment or secure storage
        let encryption_key = match std::env::var("DATABASE_ENCRYPTION_KEY") {
            Ok(value) => value,
            Err(_) => KeyManager::get_or_create_key()?,
        };

        // Initialize database with encryption
        let db = Arc::new(DbManager::new(
            &config.database.path,
            config.database.pool_size,
            Some(encryption_key.as_str()),
        )?);

        // Run migrations
        db.run_migrations()?;

        // Initialize activity provider
        let provider = Arc::new(MacOsActivityProvider::new());

        // Initialize activity repository
        let repository = Arc::new(SqlCipherActivityRepository::new(db.clone()));

        // Create tracking service
        let tracking_service = Arc::new(TrackingService::new(provider, repository));

        // Create feature flags service
        let feature_flags = Arc::new(FeatureFlagService::new(db.clone()));

        Ok(Self { config, db, tracking_service, feature_flags, _instance_lock: instance_lock })
    }
}
