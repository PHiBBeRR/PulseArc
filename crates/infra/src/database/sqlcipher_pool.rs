//! SQLCipher pool helpers
//!
//! Thin wrapper around the shared SQLCipher connection pool that converts
//! storage errors into the domain error type used by infrastructure code.

use std::path::Path;
use std::sync::Arc;

use pulsearc_common::storage::sqlcipher::{
    SqlCipherPool as CommonSqlCipherPool, SqlCipherPoolConfig,
};
use pulsearc_common::storage::StorageError;
use pulsearc_domain::{PulseArcError, Result as DomainResult};

/// Re-export the common SQLCipher pool so callers can depend on the shared
/// type.
pub type SqlCipherPool = CommonSqlCipherPool;

/// Convenience helper for creating an `Arc<SqlCipherPool>` using domain error
/// semantics.
pub fn create_sqlcipher_pool<P: AsRef<Path>>(
    path: P,
    encryption_key: String,
    config: SqlCipherPoolConfig,
) -> DomainResult<Arc<SqlCipherPool>> {
    SqlCipherPool::new(path.as_ref(), encryption_key, config)
        .map(Arc::new)
        .map_err(map_storage_error)
}

fn map_storage_error(err: StorageError) -> PulseArcError {
    PulseArcError::Database(err.to_string())
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    #[test]
    fn create_pool_successfully() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let pool = create_sqlcipher_pool(
            &db_path,
            "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_string(),
            SqlCipherPoolConfig::default(),
        )
        .expect("pool should be created");

        // Smoke test: acquire a connection and create a table
        let conn = pool.get_sqlcipher_connection().expect("connection should be acquired");
        conn.execute("CREATE TABLE test (id INTEGER PRIMARY KEY)", rusqlite::params![])
            .expect("table creation should succeed");
    }
}
