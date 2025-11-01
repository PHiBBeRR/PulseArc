use std::sync::{Arc, Mutex, OnceLock};

use log::{Level, LevelFilter, Log, Metadata, Record};
use pulsearc_common::testing::TempDir;
use pulsearc_domain::{OutboxStatus, TimeEntryOutbox};
use pulsearc_infra::database::DbManager;

type LogRecord = (Level, String);
type LogBuffer = Vec<LogRecord>;

const TEST_DB_KEY: &str = "test_key_64_chars_long_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

/// Temporary database wrapper that keeps the underlying file alive for the
/// duration of a test run.
pub struct TestDatabase {
    pub manager: Arc<DbManager>,
    _temp_dir: TempDir,
}

impl TestDatabase {
    /// Create a new temporary database with default configuration.
    pub fn new() -> Self {
        let temp_dir = TempDir::new("infra-test").expect("temp dir should be created");
        let db_path = temp_dir.path().join("test.db");

        let manager =
            DbManager::new(&db_path, 4, Some(TEST_DB_KEY)).expect("db manager should be created");

        Self { manager: Arc::new(manager), _temp_dir: temp_dir }
    }

    /// Execute a batch of SQL statements against the database.
    pub fn execute_batch(&self, sql: &str) {
        let conn = self
            .manager
            .get_connection()
            .expect("connection should be available for execute_batch");
        conn.execute_batch(sql).expect("SQL batch execution should succeed");
    }
}

impl Default for TestDatabase {
    fn default() -> Self {
        Self::new()
    }
}

/// Create schema for outbox-related tests.
pub fn setup_outbox_db() -> TestDatabase {
    let db = TestDatabase::new();
    db.execute_batch(
        "CREATE TABLE time_entry_outbox (
            id TEXT PRIMARY KEY,
            idempotency_key TEXT NOT NULL,
            user_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            backend_cuid TEXT,
            status TEXT NOT NULL,
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            retry_after INTEGER,
            created_at INTEGER NOT NULL,
            sent_at INTEGER,
            correlation_id TEXT,
            local_status TEXT,
            remote_status TEXT,
            sap_entry_id TEXT,
            next_attempt_at INTEGER,
            error_code TEXT,
            last_forwarded_at INTEGER,
            wbs_code TEXT,
            target TEXT NOT NULL,
            description TEXT,
            auto_applied INTEGER NOT NULL DEFAULT 0,
            version INTEGER NOT NULL DEFAULT 1,
            last_modified_by TEXT NOT NULL,
            last_modified_at INTEGER
        );
        CREATE INDEX idx_outbox_status_retry ON time_entry_outbox(status, retry_after);
    ",
    );
    db
}

/// Create schema for activity segment tests.
pub fn setup_segment_db() -> TestDatabase {
    let db = TestDatabase::new();
    db.execute_batch(
        "CREATE TABLE activity_segments (
            id TEXT PRIMARY KEY,
            start_ts INTEGER NOT NULL,
            end_ts INTEGER NOT NULL,
            primary_app TEXT NOT NULL,
            normalized_label TEXT NOT NULL,
            sample_count INTEGER NOT NULL,
            dictionary_keys TEXT,
            created_at INTEGER NOT NULL,
            processed INTEGER NOT NULL DEFAULT 0,
            snapshot_ids TEXT NOT NULL,
            work_type TEXT,
            activity_category TEXT NOT NULL,
            detected_activity TEXT NOT NULL,
            extracted_signals_json TEXT,
            project_match_json TEXT,
            idle_time_secs INTEGER NOT NULL,
            active_time_secs INTEGER NOT NULL,
            user_action TEXT
        );
        CREATE INDEX idx_activity_segments_start_ts ON activity_segments(start_ts);
    ",
    );
    db
}

/// Create schema for activity snapshot tests.
pub fn setup_snapshot_db() -> TestDatabase {
    let db = TestDatabase::new();
    db.execute_batch(
        "CREATE TABLE activity_snapshots (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            activity_context_json TEXT NOT NULL,
            detected_activity TEXT NOT NULL,
            work_type TEXT,
            activity_category TEXT,
            primary_app TEXT NOT NULL,
            processed INTEGER NOT NULL DEFAULT 0,
            batch_id TEXT,
            created_at INTEGER NOT NULL,
            processed_at INTEGER,
            is_idle INTEGER NOT NULL DEFAULT 0,
            idle_duration_secs INTEGER
        );
        CREATE INDEX idx_activity_snapshots_timestamp ON activity_snapshots(timestamp);
    ",
    );
    db
}

/// Utility helper for constructing outbox entries inside tests.
pub fn make_outbox_entry(id: &str, status: OutboxStatus, created_at: i64) -> TimeEntryOutbox {
    TimeEntryOutbox {
        id: id.to_string(),
        idempotency_key: format!("idem-{}", id),
        user_id: "user-1".to_string(),
        payload_json: "{}".to_string(),
        backend_cuid: None,
        status,
        attempts: 0,
        last_error: None,
        retry_after: None,
        created_at,
        sent_at: None,
        correlation_id: None,
        local_status: None,
        remote_status: None,
        sap_entry_id: None,
        next_attempt_at: None,
        error_code: None,
        last_forwarded_at: None,
        wbs_code: None,
        target: "sap".to_string(),
        description: Some("Test entry".to_string()),
        auto_applied: false,
        version: 1,
        last_modified_by: "tester".to_string(),
        last_modified_at: None,
    }
}

/// Handle for inspecting captured log records during tests.
pub struct LogHandle {
    inner: Arc<LoggerInner>,
}

impl LogHandle {
    /// Return all captured log messages.
    pub fn entries(&self) -> LogBuffer {
        let guard = self.inner.records.lock().expect("log mutex poisoned");
        guard.clone()
    }

    /// Check whether a log message matching the pattern exists.
    pub fn contains(&self, level: Level, needle: &str) -> bool {
        self.entries().into_iter().any(|(lvl, msg)| lvl == level && msg.contains(needle))
    }
}

#[derive(Clone)]
struct TestLogger {
    inner: Arc<LoggerInner>,
}

struct LoggerInner {
    records: Mutex<LogBuffer>,
}

impl Log for TestLogger {
    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn log(&self, record: &Record<'_>) {
        let mut guard = self.inner.records.lock().expect("log mutex poisoned");
        guard.push((record.level(), record.args().to_string()));
    }

    fn flush(&self) {}
}

static LOGGER: OnceLock<TestLogger> = OnceLock::new();

/// Install the test logger (idempotent) and obtain a handle for reading log
/// messages.
pub fn init_test_logger() -> LogHandle {
    let logger = LOGGER.get_or_init(|| {
        let logger = TestLogger {
            inner: Arc::new(LoggerInner { records: Mutex::new(Vec::<LogRecord>::new()) }),
        };

        if log::set_boxed_logger(Box::new(logger.clone())).is_ok() {
            log::set_max_level(LevelFilter::Trace);
        }

        logger
    });

    if let Ok(mut guard) = logger.inner.records.lock() {
        guard.clear();
    }

    LogHandle { inner: Arc::clone(&logger.inner) }
}
