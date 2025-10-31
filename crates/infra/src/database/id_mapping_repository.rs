//! ID Mapping Repository implementation
//!
//! Maps local UUIDs to backend CUIDs for entity synchronization.

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::{StorageError, StorageResult};
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::sync::ports::IdMappingRepository as IdMappingRepositoryPort;
use pulsearc_domain::{IdMapping, PulseArcError, Result as DomainResult};
use rusqlite::{params, Row};
use tokio::task;

use super::manager::DbManager;

/// SqlCipher-based ID Mapping repository
pub struct SqlCipherIdMappingRepository {
    db: Arc<DbManager>,
}

impl SqlCipherIdMappingRepository {
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl IdMappingRepositoryPort for SqlCipherIdMappingRepository {
    async fn create_id_mapping(&self, mapping: &IdMapping) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let mapping = mapping.clone();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_id_mapping(&conn, &mapping).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_id_mapping_by_local_uuid(&self, uuid: &str) -> DomainResult<Option<IdMapping>> {
        let db = Arc::clone(&self.db);
        let uuid = uuid.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<IdMapping>> {
            let conn = db.get_connection()?;
            query_id_mapping_by_local_uuid(&conn, &uuid).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_backend_cuid_by_local_uuid(&self, uuid: &str) -> DomainResult<Option<String>> {
        let db = Arc::clone(&self.db);
        let uuid = uuid.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<String>> {
            let conn = db.get_connection()?;
            query_backend_cuid(&conn, &uuid).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_local_uuid_by_backend_cuid(&self, cuid: &str) -> DomainResult<Option<String>> {
        let db = Arc::clone(&self.db);
        let cuid = cuid.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<String>> {
            let conn = db.get_connection()?;
            query_local_uuid(&conn, &cuid).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_id_mappings_by_entity_type(
        &self,
        entity_type: &str,
    ) -> DomainResult<Vec<IdMapping>> {
        let db = Arc::clone(&self.db);
        let entity_type = entity_type.to_string();

        task::spawn_blocking(move || -> DomainResult<Vec<IdMapping>> {
            let conn = db.get_connection()?;
            query_mappings_by_entity_type(&conn, &entity_type).map_err(map_storage_error)
        })
        .await
        .map_err(map_join_error)?
    }
}

// ============================================================================
// SQL Operations (synchronous)
// ============================================================================

fn insert_id_mapping(conn: &SqlCipherConnection, mapping: &IdMapping) -> StorageResult<()> {
    conn.execute(
        "INSERT INTO id_mapping (local_uuid, backend_cuid, entity_type, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            &mapping.local_uuid,
            &mapping.backend_cuid,
            &mapping.entity_type,
            &mapping.created_at,
            &mapping.updated_at,
        ],
    )?;
    Ok(())
}

fn query_id_mapping_by_local_uuid(
    conn: &SqlCipherConnection,
    uuid: &str,
) -> StorageResult<Option<IdMapping>> {
    let sql = "SELECT local_uuid, backend_cuid, entity_type, created_at, updated_at
               FROM id_mapping WHERE local_uuid = ?1";

    match conn.query_row(sql, params![uuid], map_id_mapping_row) {
        Ok(mapping) => Ok(Some(mapping)),
        Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
        Err(err) => Err(err),
    }
}

fn query_backend_cuid(conn: &SqlCipherConnection, uuid: &str) -> StorageResult<Option<String>> {
    let sql = "SELECT backend_cuid FROM id_mapping WHERE local_uuid = ?1";

    match conn.query_row(sql, params![uuid], |row| row.get(0)) {
        Ok(cuid) => Ok(Some(cuid)),
        Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
        Err(err) => Err(err),
    }
}

fn query_local_uuid(conn: &SqlCipherConnection, cuid: &str) -> StorageResult<Option<String>> {
    let sql = "SELECT local_uuid FROM id_mapping WHERE backend_cuid = ?1";

    match conn.query_row(sql, params![cuid], |row| row.get(0)) {
        Ok(uuid) => Ok(Some(uuid)),
        Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
        Err(err) => Err(err),
    }
}

fn query_mappings_by_entity_type(
    conn: &SqlCipherConnection,
    entity_type: &str,
) -> StorageResult<Vec<IdMapping>> {
    let sql = "SELECT local_uuid, backend_cuid, entity_type, created_at, updated_at
               FROM id_mapping WHERE entity_type = ?1
               ORDER BY created_at ASC";

    let mut stmt = conn.prepare(sql)?;
    // CRITICAL: query_map returns Vec<T> directly (NOT an iterator)
    stmt.query_map(params![entity_type], map_id_mapping_row)
}

fn map_id_mapping_row(row: &Row<'_>) -> rusqlite::Result<IdMapping> {
    Ok(IdMapping {
        local_uuid: row.get(0)?,
        backend_cuid: row.get(1)?,
        entity_type: row.get(2)?,
        created_at: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

// ============================================================================
// Error Mapping
// ============================================================================

fn map_storage_error(err: StorageError) -> PulseArcError {
    match err {
        StorageError::WrongKeyOrNotEncrypted => {
            PulseArcError::Security("sqlcipher key rejected or database not encrypted".into())
        }
        StorageError::Timeout(seconds) => {
            PulseArcError::Database(format!("database timeout after {seconds}s"))
        }
        StorageError::Connection(message)
        | StorageError::Query(message)
        | StorageError::DatabaseError(message)
        | StorageError::Encryption(message)
        | StorageError::Migration(message)
        | StorageError::Keychain(message)
        | StorageError::InvalidConfig(message) => PulseArcError::Database(message),
        StorageError::SchemaVersionMismatch { expected, found } => PulseArcError::Database(
            format!("schema version mismatch (expected {expected}, found {found})"),
        ),
        StorageError::PoolExhausted => PulseArcError::Database("connection pool exhausted".into()),
        StorageError::Common(common_err) => PulseArcError::Database(common_err.to_string()),
        StorageError::Io(io_err) => PulseArcError::Database(io_err.to_string()),
        StorageError::Rusqlite(sql_err) => PulseArcError::Database(sql_err.to_string()),
        StorageError::R2d2(r2d2_err) => PulseArcError::Database(r2d2_err.to_string()),
        StorageError::SerdeJson(json_err) => PulseArcError::Database(json_err.to_string()),
    }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    if err.is_cancelled() {
        PulseArcError::Internal("blocking task cancelled".into())
    } else {
        PulseArcError::Internal(format!("blocking task failed: {err}"))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    const TEST_KEY: &str = "test_key_64_chars_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_and_get_mapping() {
        let (repo, _manager, _dir) = setup_repository().await;

        let mapping = sample_mapping("uuid-1", "cuid-1", "time_entry");
        repo.create_id_mapping(&mapping).await.expect("mapping created");

        let retrieved = repo
            .get_id_mapping_by_local_uuid("uuid-1")
            .await
            .expect("query succeeded")
            .expect("mapping found");

        assert_eq!(retrieved.local_uuid, "uuid-1");
        assert_eq!(retrieved.backend_cuid, "cuid-1");
        assert_eq!(retrieved.entity_type, "time_entry");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_backend_cuid_by_local_uuid() {
        let (repo, _manager, _dir) = setup_repository().await;

        let mapping = sample_mapping("uuid-2", "cuid-2", "time_entry");
        repo.create_id_mapping(&mapping).await.expect("mapping created");

        let cuid = repo
            .get_backend_cuid_by_local_uuid("uuid-2")
            .await
            .expect("query succeeded")
            .expect("cuid found");

        assert_eq!(cuid, "cuid-2");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_local_uuid_by_backend_cuid() {
        let (repo, _manager, _dir) = setup_repository().await;

        let mapping = sample_mapping("uuid-3", "cuid-3", "time_entry");
        repo.create_id_mapping(&mapping).await.expect("mapping created");

        let uuid = repo
            .get_local_uuid_by_backend_cuid("cuid-3")
            .await
            .expect("query succeeded")
            .expect("uuid found");

        assert_eq!(uuid, "uuid-3");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_mappings_by_entity_type() {
        let (repo, _manager, _dir) = setup_repository().await;

        repo.create_id_mapping(&sample_mapping("uuid-4", "cuid-4", "time_entry"))
            .await
            .expect("mapping 1 created");
        repo.create_id_mapping(&sample_mapping("uuid-5", "cuid-5", "time_entry"))
            .await
            .expect("mapping 2 created");
        repo.create_id_mapping(&sample_mapping("uuid-6", "cuid-6", "project"))
            .await
            .expect("mapping 3 created");

        let time_entry_mappings =
            repo.get_id_mappings_by_entity_type("time_entry").await.expect("query succeeded");

        assert_eq!(time_entry_mappings.len(), 2);
        assert!(time_entry_mappings.iter().any(|m| m.local_uuid == "uuid-4"));
        assert!(time_entry_mappings.iter().any(|m| m.local_uuid == "uuid-5"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_missing_mapping_returns_none() {
        let (repo, _manager, _dir) = setup_repository().await;

        let result =
            repo.get_id_mapping_by_local_uuid("nonexistent-uuid").await.expect("query succeeded");

        assert!(result.is_none());
    }

    // ========================================================================
    // Test Helpers
    // ========================================================================

    async fn setup_repository() -> (SqlCipherIdMappingRepository, Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("temp dir created");
        let db_path = temp_dir.path().join("id_mappings.db");

        let manager =
            Arc::new(DbManager::new(&db_path, 4, Some(TEST_KEY)).expect("db manager created"));
        manager.run_migrations().expect("migrations run");

        let repo = SqlCipherIdMappingRepository::new(manager.clone());
        (repo, manager, temp_dir)
    }

    fn sample_mapping(local_uuid: &str, backend_cuid: &str, entity_type: &str) -> IdMapping {
        IdMapping {
            local_uuid: local_uuid.to_string(),
            backend_cuid: backend_cuid.to_string(),
            entity_type: entity_type.to_string(),
            created_at: 1700000000,
            updated_at: 1700000000,
        }
    }
}
