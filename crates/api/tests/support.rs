use std::sync::Arc;

use pulsearc_common::testing::TempDir;
use pulsearc_infra::database::{DbManager, SqlCipherBlockRepository, SqlCipherSegmentRepository};

const TEST_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Shared context for integration tests that need direct database access.
pub struct TestContext {
    /// Block repository under test.
    pub block_repository: Arc<SqlCipherBlockRepository>,
    /// Segment repository for read/write helpers.
    pub segment_repository: Arc<SqlCipherSegmentRepository>,
    /// Keep temporary directory alive for the lifetime of the context.
    _temp_dir: TempDir,
}

/// Create a new test context with fresh SQLCipher database state.
pub async fn setup_test_context() -> Arc<TestContext> {
    let temp_dir =
        TempDir::new("block-command-tests").expect("failed to create temporary database directory");
    let db_path = temp_dir.path().join("pulsearc.db");

    let db = Arc::new(
        DbManager::new(&db_path, 8, Some(TEST_KEY))
            .expect("failed to initialise SQLCipher manager"),
    );
    db.run_migrations().expect("failed to run schema migrations");

    let block_repository = Arc::new(SqlCipherBlockRepository::new(Arc::clone(&db)));
    let segment_repository = Arc::new(SqlCipherSegmentRepository::new(Arc::clone(&db)));

    Arc::new(TestContext { block_repository, segment_repository, _temp_dir: temp_dir })
}
