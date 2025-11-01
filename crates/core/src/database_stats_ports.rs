//! Database statistics and maintenance port.
//!
//! Provides read-only introspection of database state and maintenance
//! operations for monitoring and optimization.
//!
//! # Example
//!
//! ```no_run
//! use pulsearc_core::DatabaseStatsPort;
//!
//! async fn check_db_health(db_stats: &impl DatabaseStatsPort) {
//!     let health = db_stats.check_database_health().await.unwrap();
//!     if !health.is_healthy {
//!         eprintln!("Database unhealthy: {}", health.message);
//!     }
//! }
//! ```

use async_trait::async_trait;
use pulsearc_domain::types::{DatabaseSize, HealthStatus, TableStats};
use pulsearc_domain::Result;

/// Port for database statistics and maintenance operations.
///
/// All operations are read-only introspection except VACUUM, which is
/// a safe maintenance operation that reclaims unused space.
#[async_trait]
pub trait DatabaseStatsPort: Send + Sync {
    /// Get database size information.
    ///
    /// Queries PRAGMA introspection to gather page counts and sizes,
    /// plus filesystem metadata for total file size.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::DatabaseStatsPort;
    /// # async fn example(db_stats: &impl DatabaseStatsPort) {
    /// let size = db_stats.get_database_size().await.unwrap();
    /// println!("Database: {} MB", size.size_bytes / 1024 / 1024);
    /// println!("Free pages: {}", size.freelist_count);
    /// # }
    /// ```
    async fn get_database_size(&self) -> Result<DatabaseSize>;

    /// Get row counts for all tables in the database.
    ///
    /// Queries sqlite_master for table names, then counts rows in each.
    /// Useful for monitoring data growth and identifying large tables.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::DatabaseStatsPort;
    /// # async fn example(db_stats: &impl DatabaseStatsPort) {
    /// let stats = db_stats.get_table_stats().await.unwrap();
    /// for table in stats {
    ///     println!("{}: {} rows", table.name, table.row_count);
    /// }
    /// # }
    /// ```
    async fn get_table_stats(&self) -> Result<Vec<TableStats>>;

    /// Get count of unprocessed snapshots.
    ///
    /// Counts snapshots that have not been processed into segments yet.
    /// Used for monitoring the snapshot processing queue.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::DatabaseStatsPort;
    /// # async fn example(db_stats: &impl DatabaseStatsPort) {
    /// let count = db_stats.get_unprocessed_count().await.unwrap();
    /// println!("{} snapshots pending processing", count);
    /// # }
    /// ```
    async fn get_unprocessed_count(&self) -> Result<i64>;

    /// Run VACUUM to reclaim unused space.
    ///
    /// Rebuilds the database file to remove fragmentation and unused pages.
    /// This is a safe but potentially slow operation (locks database during
    /// execution). Should be run during maintenance windows.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::DatabaseStatsPort;
    /// # async fn example(db_stats: &impl DatabaseStatsPort) {
    /// db_stats.vacuum_database().await.unwrap();
    /// println!("Database vacuumed successfully");
    /// # }
    /// ```
    async fn vacuum_database(&self) -> Result<()>;

    /// Check database health with a simple connectivity test.
    ///
    /// Executes a trivial query to verify database is responsive.
    /// Returns timing information for monitoring responsiveness.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use pulsearc_core::DatabaseStatsPort;
    /// # async fn example(db_stats: &impl DatabaseStatsPort) {
    /// let health = db_stats.check_database_health().await.unwrap();
    /// if health.is_healthy {
    ///     println!("DB healthy ({}ms)", health.response_time_ms);
    /// }
    /// # }
    /// ```
    async fn check_database_health(&self) -> Result<HealthStatus>;
}
