//! Application context - dependency injection container

use std::fs;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pulsearc_core::classification::ports::BlockRepository as BlockRepositoryPort;
use pulsearc_core::sync::ports::OutboxQueue as OutboxQueuePort;
use pulsearc_core::tracking::ports::{
    IdlePeriodsRepository as IdlePeriodsRepositoryPort, SegmentRepository as SegmentRepositoryPort,
    SnapshotRepository as SnapshotRepositoryPort,
};
use pulsearc_core::user::ports::UserProfileRepository as UserProfileRepositoryPort;
use pulsearc_core::{CommandMetricsPort, DatabaseStatsPort, FeatureFlagsPort, TrackingService};
use pulsearc_domain::types::{ActivitySegment, ActivitySnapshot};
use pulsearc_domain::{Config, PulseArcError, Result};
use pulsearc_infra::api::{AccessTokenProvider, ApiClientConfig, ApiError, ForwarderConfig};
#[cfg(feature = "calendar")]
use pulsearc_infra::calendar::{
    CalendarClient, CalendarOAuthManager, CalendarOAuthSettings, CalendarSyncWorker,
};
#[cfg(feature = "calendar")]
use pulsearc_infra::database::SqlCipherCalendarEventRepository;
use pulsearc_infra::observability::metrics::PerformanceMetrics;
use pulsearc_infra::scheduling::block_scheduler::BlockJob;
use pulsearc_infra::scheduling::classification_scheduler::ClassificationJob;
use pulsearc_infra::scheduling::sync_scheduler::{
    ActivitySegmentRepository, ActivitySnapshotRepository,
};
#[cfg(feature = "calendar")]
use pulsearc_infra::CalendarScheduler;
use pulsearc_infra::{
    ApiClient, ApiCommands, ApiForwarder, BlockScheduler, BlockSchedulerConfig,
    ClassificationScheduler, ClassificationSchedulerConfig, DbManager, FeatureFlagService,
    InfraError, InstanceLock, KeyManager, MacOsActivityProvider, SqlCipherActivityRepository,
    SqlCipherBlockRepository, SqlCipherCommandMetricsRepository, SqlCipherDatabaseStatsRepository,
    SqlCipherIdlePeriodsRepository, SqlCipherOutboxRepository, SqlCipherSegmentRepository,
    SqlCipherUserProfileRepository, SyncScheduler, SyncSchedulerConfig,
};

/// Type alias for database stats port trait object
type DynDatabaseStatsPort = dyn DatabaseStatsPort + Send + Sync + 'static;

/// Type alias for command metrics port trait object
type DynCommandMetricsPort = dyn CommandMetricsPort + Send + Sync + 'static;

/// Type alias for feature flag port trait object
type DynFeatureFlagsPort = dyn FeatureFlagsPort + Send + Sync + 'static;

/// Type alias for snapshot repository port trait object
type DynSnapshotRepositoryPort = dyn SnapshotRepositoryPort + Send + Sync + 'static;

/// Type alias for user profile repository port trait object
type DynUserProfileRepositoryPort = dyn UserProfileRepositoryPort + Send + Sync + 'static;

/// Type alias for block repository port trait object
type DynBlockRepositoryPort = dyn BlockRepositoryPort + Send + Sync + 'static;

/// Type alias for segment repository port trait object
type DynSegmentRepositoryPort = dyn SegmentRepositoryPort + Send + Sync + 'static;

/// Type alias for outbox queue port trait object
type DynOutboxQueuePort = dyn OutboxQueuePort + Send + Sync + 'static;

/// Type alias for idle periods repository port trait object
type DynIdlePeriodsRepositoryPort = dyn IdlePeriodsRepositoryPort + Send + Sync + 'static;

/// Application context - holds all services and dependencies
pub struct AppContext {
    // Core services
    pub config: Config,
    pub db: Arc<DbManager>,
    pub tracking_service: Arc<TrackingService>,
    pub feature_flags: Arc<DynFeatureFlagsPort>,
    pub database_stats: Arc<DynDatabaseStatsPort>,
    pub command_metrics: Arc<DynCommandMetricsPort>,
    pub snapshots: Arc<DynSnapshotRepositoryPort>,
    pub user_profile: Arc<DynUserProfileRepositoryPort>,
    pub block_repository: Arc<DynBlockRepositoryPort>,
    pub segment_repository: Arc<DynSegmentRepositoryPort>,
    pub outbox_queue: Arc<DynOutboxQueuePort>,
    pub idle_periods: Arc<DynIdlePeriodsRepositoryPort>,

    // Schedulers (Phase 4.1.2: Added for command migration)
    pub block_scheduler: Arc<BlockScheduler>,
    pub classification_scheduler: Arc<ClassificationScheduler>,
    pub sync_scheduler: Arc<SyncScheduler>,

    #[cfg(feature = "calendar")]
    pub calendar_scheduler: Arc<CalendarScheduler>,

    // Calendar integration (Phase 4B.2)
    #[cfg(feature = "calendar")]
    pub calendar_oauth: Arc<CalendarOAuthManager>,

    #[cfg(feature = "calendar")]
    pub calendar_events: Arc<dyn pulsearc_core::tracking::ports::CalendarEventRepository>,

    // TODO(Phase 4): Add ML infrastructure when Phase 3E is completed
    // #[cfg(feature = "tree-classifier")]
    // pub hybrid_classifier: Arc<HybridClassifier>,
    // #[cfg(feature = "tree-classifier")]
    // pub metrics_tracker: Arc<MetricsTracker>,

    // Telemetry metrics for idle sync (Phase 4C.2)
    pub idle_sync_metrics: Arc<crate::utils::idle_sync_metrics::IdleSyncMetrics>,

    // Keep instance lock alive for the lifetime of the app
    _instance_lock: InstanceLock,
}

async fn create_block_scheduler() -> Result<Arc<BlockScheduler>> {
    // Placeholder job until scheduler wiring lands in Phase 4.1.3
    // (docs/PHASE-4-NEW-CRATE-MIGRATION.md)
    let job: Arc<dyn BlockJob> = Arc::new(NoopBlockJob);
    let metrics = Arc::new(PerformanceMetrics::new());
    let config = BlockSchedulerConfig::default();

    let mut scheduler = BlockScheduler::with_config(config, job, metrics).map_err(|err| {
        tracing::error!(error = %err, "failed to construct BlockScheduler");
        PulseArcError::Internal(format!("failed to construct BlockScheduler: {}", err))
    })?;

    // Start the scheduler with timeout (fail-fast initialization)
    let start_timeout = Duration::from_secs(10);
    tokio::time::timeout(start_timeout, scheduler.start())
        .await
        .map_err(|_| {
            tracing::error!(timeout_secs = 10, "BlockScheduler start timed out");
            PulseArcError::Internal("BlockScheduler start timed out after 10s".into())
        })?
        .map_err(|err| {
            tracing::error!(error = %err, "failed to start BlockScheduler");
            PulseArcError::Internal(format!("failed to start BlockScheduler: {}", err))
        })?;

    Ok(Arc::new(scheduler))
}

async fn create_classification_scheduler() -> Result<Arc<ClassificationScheduler>> {
    // Placeholder job until classifier wiring is implemented
    // (docs/PHASE-4-NEW-CRATE-MIGRATION.md)
    let job: Arc<dyn ClassificationJob> = Arc::new(NoopClassificationJob);
    let metrics = Arc::new(PerformanceMetrics::new());
    let config = ClassificationSchedulerConfig::default();

    let mut scheduler =
        ClassificationScheduler::with_config(config, job, metrics).await.map_err(|err| {
            tracing::error!(error = %err, "failed to construct ClassificationScheduler");
            PulseArcError::Internal(format!("failed to construct ClassificationScheduler: {}", err))
        })?;

    // Start the scheduler with timeout (fail-fast initialization)
    let start_timeout = Duration::from_secs(10);
    tokio::time::timeout(start_timeout, scheduler.start())
        .await
        .map_err(|_| {
            tracing::error!(timeout_secs = 10, "ClassificationScheduler start timed out");
            PulseArcError::Internal("ClassificationScheduler start timed out after 10s".into())
        })?
        .map_err(|err| {
            tracing::error!(error = %err, "failed to start ClassificationScheduler");
            PulseArcError::Internal(format!("failed to start ClassificationScheduler: {}", err))
        })?;

    Ok(Arc::new(scheduler))
}

async fn create_sync_scheduler(config: &Config) -> Result<Arc<SyncScheduler>> {
    let forwarder = build_api_forwarder()?;
    let segment_repo: Arc<dyn ActivitySegmentRepository> = Arc::new(EmptySegmentRepository);
    let snapshot_repo: Arc<dyn ActivitySnapshotRepository> = Arc::new(EmptySnapshotRepository);

    let scheduler_config = SyncSchedulerConfig {
        interval: Duration::from_secs(config.sync.interval_seconds.max(1)),
        ..Default::default()
    };

    let metrics = Arc::new(PerformanceMetrics::new());
    let mut scheduler =
        SyncScheduler::new(forwarder, segment_repo, snapshot_repo, scheduler_config, metrics);

    // Start the scheduler with timeout (fail-fast initialization)
    let start_timeout = Duration::from_secs(10);
    tokio::time::timeout(start_timeout, scheduler.start())
        .await
        .map_err(|_| {
            tracing::error!(timeout_secs = 10, "SyncScheduler start timed out");
            PulseArcError::Internal("SyncScheduler start timed out after 10s".into())
        })?
        .map_err(|err| {
            tracing::error!(error = %err, "failed to start SyncScheduler");
            PulseArcError::Internal(format!("failed to start SyncScheduler: {}", err))
        })?;

    Ok(Arc::new(scheduler))
}

#[cfg(feature = "calendar")]
async fn create_calendar_scheduler(
    db: Arc<DbManager>,
    outbox_queue: Arc<DynOutboxQueuePort>,
) -> Result<Arc<CalendarScheduler>> {
    let metrics = Arc::new(PerformanceMetrics::new());
    let cron_expression = "0 0 * * *".to_string(); // Daily at midnight (placeholder)
    let user_emails = Vec::new(); // Empty list until user management is wired

    let oauth_settings = CalendarOAuthSettings::google("stub-client-id", None);
    let oauth_manager = Arc::new(CalendarOAuthManager::new(oauth_settings));
    let client = CalendarClient::new(
        "stub-user@pulsearc.local".to_string(),
        "google".to_string(),
        oauth_manager,
    )
    .map_err(|err| {
        PulseArcError::Internal(format!("failed to construct placeholder CalendarClient: {err}"))
    })?;

    let calendar_repo: Arc<dyn pulsearc_core::CalendarEventRepository> =
        Arc::new(SqlCipherCalendarEventRepository::new(Arc::clone(db.pool())));
    let sync_worker = Arc::new(CalendarSyncWorker::new(
        client,
        calendar_repo,
        outbox_queue,
        Arc::clone(db.pool()),
    ));

    CalendarScheduler::new(cron_expression, user_emails, sync_worker, metrics)
        .map(Arc::new)
        .map_err(|err| {
            PulseArcError::Internal(format!("failed to construct CalendarScheduler: {}", err))
        })
}

fn build_api_forwarder() -> Result<Arc<ApiForwarder>> {
    let token_provider: Arc<dyn AccessTokenProvider> =
        Arc::new(StaticAccessTokenProvider::new("stub-token"));
    let client =
        Arc::new(ApiClient::new(ApiClientConfig::default(), token_provider).map_err(|err| {
            PulseArcError::Internal(format!("failed to construct ApiClient: {}", err))
        })?);
    let commands = Arc::new(ApiCommands::new(client));
    let forwarder = ApiForwarder::new(commands, ForwarderConfig::default());

    Ok(Arc::new(forwarder))
}

#[derive(Default)]
struct NoopBlockJob;

#[async_trait]
impl BlockJob for NoopBlockJob {
    async fn run(&self) -> std::result::Result<(), InfraError> {
        tracing::trace!("NoopBlockJob::run (placeholder, no-op until Phase 4.1.3)");
        Ok(())
    }
}

#[derive(Default)]
struct NoopClassificationJob;

#[async_trait]
impl ClassificationJob for NoopClassificationJob {
    async fn run(&self) -> std::result::Result<(), InfraError> {
        tracing::trace!("NoopClassificationJob::run (placeholder, no-op until classifier wiring)");
        Ok(())
    }
}

#[derive(Default)]
struct EmptySegmentRepository;

#[async_trait]
impl ActivitySegmentRepository for EmptySegmentRepository {
    async fn get_pending_for_sync(
        &self,
        batch_size: usize,
    ) -> std::result::Result<Vec<ActivitySegment>, PulseArcError> {
        tracing::debug!(
            batch_size,
            "EmptySegmentRepository::get_pending_for_sync (placeholder, returns empty)"
        );
        Ok(Vec::new())
    }

    async fn mark_synced(&self, id: &str) -> std::result::Result<(), PulseArcError> {
        tracing::debug!(id, "EmptySegmentRepository::mark_synced (placeholder, no-op)");
        Ok(())
    }
}

#[derive(Default)]
struct EmptySnapshotRepository;

#[async_trait]
impl ActivitySnapshotRepository for EmptySnapshotRepository {
    async fn get_pending_for_sync(
        &self,
        batch_size: usize,
    ) -> std::result::Result<Vec<ActivitySnapshot>, PulseArcError> {
        tracing::debug!(
            batch_size,
            "EmptySnapshotRepository::get_pending_for_sync (placeholder, returns empty)"
        );
        Ok(Vec::new())
    }

    async fn mark_synced(&self, id: &str) -> std::result::Result<(), PulseArcError> {
        tracing::debug!(id, "EmptySnapshotRepository::mark_synced (placeholder, no-op)");
        Ok(())
    }
}

struct StaticAccessTokenProvider {
    token: String,
}

impl StaticAccessTokenProvider {
    fn new(token: impl Into<String>) -> Self {
        let token = token.into();

        // Safety guard: warn if using placeholder token (potential production
        // misconfiguration)
        #[cfg(not(test))]
        if token == "stub-token" {
            tracing::warn!(
                "StaticAccessTokenProvider initialized with placeholder 'stub-token'; \
                 API calls will fail with authentication errors. This is expected for \
                 scheduler placeholders but should be replaced in production."
            );
        }

        Self { token }
    }
}

#[async_trait]
impl AccessTokenProvider for StaticAccessTokenProvider {
    async fn access_token(&self) -> std::result::Result<String, ApiError> {
        Ok(self.token.clone())
    }
}

impl AppContext {
    /// Create a new application context with default configuration
    pub async fn new() -> Result<Self> {
        Self::new_with_config(Config::default()).await
    }

    /// Create a new application context with custom configuration
    ///
    /// This method is primarily for testing, allowing tests to specify a custom
    /// database path and avoid conflicts with the production database.
    pub async fn new_with_config(config: Config) -> Result<Self> {
        Self::new_with_config_in_lock_dir(config, std::env::temp_dir()).await
    }

    /// Create a new application context with a custom lock directory
    ///
    /// Tests can use this to provide per-test directories and avoid PID file
    /// conflicts.
    pub async fn new_with_config_in_lock_dir<P>(config: Config, lock_dir: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let lock_dir_path = lock_dir.as_ref().to_path_buf();

        fs::create_dir_all(&lock_dir_path).map_err(|err| {
            PulseArcError::Internal(format!(
                "failed to create instance lock directory {}: {}",
                lock_dir_path.display(),
                err
            ))
        })?;

        // Acquire instance lock to prevent multiple instances
        let instance_lock = InstanceLock::acquire(&lock_dir_path)?;

        // Resolve encryption key with test-friendly fallback chain:
        // 1. TEST_DATABASE_ENCRYPTION_KEY (for tests, doesn't touch keychain)
        // 2. DATABASE_ENCRYPTION_KEY (for production override)
        // 3. KeyManager (production default, uses macOS keychain)
        let encryption_key = match std::env::var("TEST_DATABASE_ENCRYPTION_KEY") {
            Ok(value) => {
                tracing::debug!("using TEST_DATABASE_ENCRYPTION_KEY for database encryption");
                value
            }
            Err(_) => match std::env::var("DATABASE_ENCRYPTION_KEY") {
                Ok(value) => {
                    tracing::info!("using DATABASE_ENCRYPTION_KEY for database encryption");
                    value
                }
                Err(_) => {
                    tracing::info!("fetching encryption key from macOS keychain");
                    KeyManager::get_or_create_key().map_err(|e| {
                        tracing::error!(error = %e, "failed to retrieve encryption key from keychain");
                        e
                    })?
                }
            },
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
        let provider = MacOsActivityProvider::new();

        // Initialize activity repository
        let repository = Arc::new(SqlCipherActivityRepository::new(db.clone()));

        // Create tracking service
        let tracking_service = Arc::new(TrackingService::new(provider, repository.clone()));

        // Create feature flags service (cached implementation of FeatureFlagsPort)
        let feature_flags: Arc<DynFeatureFlagsPort> = Arc::new(FeatureFlagService::new(db.clone()));

        // Create database stats repository
        let database_stats = Arc::new(SqlCipherDatabaseStatsRepository::new(db.clone()));

        // Create command metrics repository (Phase 4.1.6: Metrics collection for
        // validation)
        let command_metrics = Arc::new(SqlCipherCommandMetricsRepository::new(db.clone()));

        // Create snapshots repository (Phase 4A.1: Database commands migration)
        let snapshots: Arc<DynSnapshotRepositoryPort> = repository.clone();

        // Create user profile repository (Phase 4A.2: User profile commands migration)
        let user_profile: Arc<DynUserProfileRepositoryPort> =
            Arc::new(SqlCipherUserProfileRepository::new(db.clone()));

        // Create block repository (Phase 4B.1 preparation)
        let block_repository: Arc<DynBlockRepositoryPort> =
            Arc::new(SqlCipherBlockRepository::new(db.clone()));

        // Create segment repository for read access (Phase 4B.1 preparation)
        let segment_repository: Arc<DynSegmentRepositoryPort> =
            Arc::new(SqlCipherSegmentRepository::new(db.clone()));

        // Create outbox queue (replaces placeholder EmptyOutboxQueue)
        let outbox_queue: Arc<DynOutboxQueuePort> =
            Arc::new(SqlCipherOutboxRepository::new(db.clone()));

        // Create idle periods repository (Phase 4B.3 preparation)
        let idle_periods: Arc<DynIdlePeriodsRepositoryPort> =
            Arc::new(SqlCipherIdlePeriodsRepository::new(db.clone()));

        // Initialize and start schedulers (fail-fast)
        let block_scheduler = create_block_scheduler().await?;
        let classification_scheduler = create_classification_scheduler().await?;
        let sync_scheduler = create_sync_scheduler(&config).await?;

        #[cfg(feature = "calendar")]
        let calendar_scheduler =
            create_calendar_scheduler(Arc::clone(&db), Arc::clone(&outbox_queue)).await?;

        // Initialize calendar OAuth manager (Phase 4B.2)
        #[cfg(feature = "calendar")]
        let calendar_oauth = {
            let client_id = std::env::var("GOOGLE_CALENDAR_CLIENT_ID")
                .unwrap_or_else(|_| "stub-client-id".to_string());
            let client_secret = std::env::var("GOOGLE_CALENDAR_CLIENT_SECRET").ok();
            let settings = CalendarOAuthSettings::google(client_id, client_secret);
            Arc::new(CalendarOAuthManager::new(settings))
        };

        // Initialize calendar events repository (Phase 4B.2)
        #[cfg(feature = "calendar")]
        let calendar_events: Arc<
            dyn pulsearc_core::tracking::ports::CalendarEventRepository,
        > = Arc::new(SqlCipherCalendarEventRepository::new(Arc::clone(db.pool())));

        // Initialize idle sync metrics (Phase 4C.2)
        let idle_sync_metrics = Arc::new(crate::utils::idle_sync_metrics::IdleSyncMetrics::new());

        Ok(Self {
            config,
            db,
            tracking_service,
            feature_flags,
            database_stats,
            command_metrics,
            snapshots,
            user_profile,
            block_repository,
            segment_repository,
            outbox_queue,
            idle_periods,
            block_scheduler,
            classification_scheduler,
            sync_scheduler,
            #[cfg(feature = "calendar")]
            calendar_scheduler,
            #[cfg(feature = "calendar")]
            calendar_oauth,
            #[cfg(feature = "calendar")]
            calendar_events,
            idle_sync_metrics,
            _instance_lock: instance_lock,
        })
    }

    /// Check health of all application components
    ///
    /// Returns a HealthStatus with individual component health checks and an
    /// overall health score. The score is calculated as (healthy_components
    /// / total_components), and the application is considered healthy if
    /// score >= 0.8.
    ///
    /// # Example
    /// ```no_run
    /// let context = AppContext::new().await?;
    /// let health = context.health_check().await;
    ///
    /// if health.is_healthy {
    ///     println!("Application is healthy (score: {})", health.score);
    /// } else {
    ///     println!("Application is degraded (score: {})", health.score);
    ///     for component in health.components {
    ///         if !component.is_healthy {
    ///             println!("  - {} is unhealthy: {:?}", component.name, component.message);
    ///         }
    ///     }
    /// }
    /// ```
    pub async fn health_check(&self) -> crate::utils::health::HealthStatus {
        use crate::utils::health::{ComponentHealth, HealthStatus};

        let mut status = HealthStatus::new();

        // Check database connection (async to avoid blocking)
        status = status.add_component(self.check_database_health().await);

        // Check feature flags service (stateless, always healthy)
        status = status.add_component(ComponentHealth::healthy("feature_flags"));

        // Check tracking service (stateless, always healthy)
        status = status.add_component(ComponentHealth::healthy("tracking_service"));

        // Check database stats repository (stateless wrapper)
        status = status.add_component(ComponentHealth::healthy("database_stats"));

        // Check command metrics repository (stateless wrapper)
        status = status.add_component(ComponentHealth::healthy("command_metrics"));

        // Note: Schedulers don't expose health checks, assumed healthy if started
        status = status.add_component(ComponentHealth::healthy("schedulers"));

        // Calculate overall health score
        status.calculate_score();

        status
    }

    /// Check database health by attempting a simple query
    ///
    /// Uses spawn_blocking to avoid blocking the async runtime with synchronous
    /// database operations.
    async fn check_database_health(&self) -> crate::utils::health::ComponentHealth {
        use crate::utils::health::ComponentHealth;

        let db = self.db.clone();
        match tokio::task::spawn_blocking(move || {
            let conn = db.get_connection()?;
            conn.execute("SELECT 1", []).map_err(|e| {
                PulseArcError::Database(format!("health check query failed: {}", e))
            })?;
            Ok::<(), pulsearc_domain::PulseArcError>(())
        })
        .await
        {
            Ok(Ok(())) => ComponentHealth::healthy("database"),
            Ok(Err(e)) => {
                tracing::warn!(error = %e, "database health check failed");
                ComponentHealth::unhealthy("database", format!("query failed: {}", e))
            }
            Err(e) => {
                tracing::error!(error = %e, "database health check task panicked");
                ComponentHealth::unhealthy("database", format!("task panic: {}", e))
            }
        }
    }

    /// Shutdown the application context gracefully
    ///
    /// # Implementation Note
    ///
    /// This method is intentionally a no-op. Most services and schedulers in
    /// AppContext don't require explicit shutdown because they use
    /// `tokio::spawn` tasks that are automatically cancelled when the tokio
    /// runtime shuts down.
    ///
    /// According to the scheduler lifecycle survey (Phase 0.2), all schedulers
    /// implement:
    /// - `start()` - Spawns background tasks
    /// - `stop()` - Cancels tasks via CancellationToken
    /// - `Drop` - Ensures cleanup on drop
    ///
    /// When AppContext is dropped (when the app exits), the following happens
    /// automatically:
    /// 1. All Arc<Scheduler> references are dropped
    /// 2. Scheduler Drop impls trigger cancellation
    /// 3. Tokio runtime shutdown waits for tasks to complete
    /// 4. Resources are cleaned up in reverse dependency order
    ///
    /// Only services with explicit cleanup requirements (database connections,
    /// file handles, OAuth tokens) would need shutdown calls, and currently
    /// none of our services require this.
    ///
    /// # Design Decision
    ///
    /// This approach follows Rust's RAII pattern where cleanup happens via Drop
    /// rather than explicit shutdown methods. This makes the shutdown
    /// process more robust because:
    /// - Cleanup happens even if shutdown() is never called (e.g., on panic)
    /// - No need to remember the correct shutdown order
    /// - Idempotent (can be called multiple times safely)
    ///
    /// See: docs/SCHEDULER-LIFECYCLE-REFERENCE.md for complete scheduler
    /// lifecycle details
    pub async fn shutdown(&self) -> Result<()> {
        use tracing::info;

        info!("shutdown called on AppContext");

        // Log diagnostic information about component states
        self.shutdown_diagnostics();

        // NOTE: Explicit scheduler shutdown is not needed. All schedulers use
        // CancellationToken and tokio::spawn tasks that are automatically
        // cancelled when:
        // 1. The scheduler is dropped (triggers CancellationToken)
        // 2. The tokio runtime shuts down (waits for tasks to complete)
        //
        // This approach is safer than explicit shutdown calls because:
        // - Works even if shutdown() is never called
        // - Handles panic scenarios gracefully
        // - No Arc::get_mut() failures if multiple references exist
        //
        // Survey results (Phase 0.2):
        // - BlockScheduler: No explicit shutdown needed (Drop handles it)
        // - ClassificationScheduler: No explicit shutdown needed (Drop handles it)
        // - SyncScheduler: No explicit shutdown needed (Drop handles it)
        // - CalendarScheduler: No explicit shutdown needed (Drop handles it)
        // - TrackingService: No shutdown method (stateless)
        // - FeatureFlagService: No shutdown method (stateless)
        //
        // If a service is added in the future that requires explicit cleanup
        // (e.g., flushing buffers, closing connections), add the call here.

        Ok(())
    }

    /// Log diagnostic information about component cleanup
    ///
    /// This method provides observability during shutdown by logging the
    /// cleanup approach for each component. Useful for debugging shutdown
    /// issues.
    fn shutdown_diagnostics(&self) {
        use tracing::info;

        info!(
            component = "BlockScheduler",
            cleanup_method = "Drop (CancellationToken)",
            "scheduler_cleanup"
        );

        info!(
            component = "ClassificationScheduler",
            cleanup_method = "Drop (CancellationToken)",
            "scheduler_cleanup"
        );

        info!(
            component = "SyncScheduler",
            cleanup_method = "Drop (CancellationToken)",
            "scheduler_cleanup"
        );

        #[cfg(feature = "calendar")]
        info!(
            component = "CalendarScheduler",
            cleanup_method = "Drop (CancellationToken)",
            "scheduler_cleanup"
        );

        info!(
            component = "TrackingService",
            cleanup_method = "stateless (no cleanup)",
            "service_cleanup"
        );

        info!(
            component = "FeatureFlagService",
            cleanup_method = "stateless (no cleanup)",
            "service_cleanup"
        );

        info!(
            component = "DatabaseManager",
            cleanup_method = "connection pool auto-closes",
            "database_cleanup"
        );
    }
}
