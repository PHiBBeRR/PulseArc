//! User profile repository implementation using SQLCipher
//!
//! Provides persistence for user profile data synced from Auth0

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_common::storage::error::StorageError;
use pulsearc_common::storage::sqlcipher::SqlCipherConnection;
use pulsearc_core::user::ports::UserProfileRepository as UserProfileRepositoryPort;
use pulsearc_domain::{PulseArcError, Result as DomainResult, UserProfile};
use rusqlite::{params, Row, ToSql};
use tokio::task;

use super::manager::DbManager;

/// SQLCipher-backed implementation of `UserProfileRepository`
pub struct SqlCipherUserProfileRepository {
    db: Arc<DbManager>,
}

impl SqlCipherUserProfileRepository {
    /// Create a new repository instance
    pub fn new(db: Arc<DbManager>) -> Self {
        Self { db }
    }
}

#[async_trait]
impl UserProfileRepositoryPort for SqlCipherUserProfileRepository {
    async fn get_by_id(&self, id: &str) -> DomainResult<Option<UserProfile>> {
        let db = Arc::clone(&self.db);
        let id = id.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<UserProfile>> {
            let conn = db.get_connection()?;

            let result = conn.query_row(
                "SELECT id, auth0_id, email, name, first_name, last_name, display_name,
                        avatar_url, phone_number, title, department, location, bio,
                        timezone, language, locale, date_format, is_active, email_verified,
                        two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
                 FROM user_profiles WHERE id = ?1",
                params![&id],
                map_user_profile_row,
            );

            match result {
                Ok(profile) => Ok(Some(profile)),
                Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
                Err(err) => Err(map_storage_error(err)),
            }
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_by_auth0_id(&self, auth0_id: &str) -> DomainResult<Option<UserProfile>> {
        let db = Arc::clone(&self.db);
        let auth0_id = auth0_id.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<UserProfile>> {
            let conn = db.get_connection()?;

            let result = conn.query_row(
                "SELECT id, auth0_id, email, name, first_name, last_name, display_name,
                        avatar_url, phone_number, title, department, location, bio,
                        timezone, language, locale, date_format, is_active, email_verified,
                        two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
                 FROM user_profiles WHERE auth0_id = ?1",
                params![&auth0_id],
                map_user_profile_row,
            );

            match result {
                Ok(profile) => Ok(Some(profile)),
                Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
                Err(err) => Err(map_storage_error(err)),
            }
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_by_email(&self, email: &str) -> DomainResult<Option<UserProfile>> {
        let db = Arc::clone(&self.db);
        let email = email.to_string();

        task::spawn_blocking(move || -> DomainResult<Option<UserProfile>> {
            let conn = db.get_connection()?;

            let result = conn.query_row(
                "SELECT id, auth0_id, email, name, first_name, last_name, display_name,
                        avatar_url, phone_number, title, department, location, bio,
                        timezone, language, locale, date_format, is_active, email_verified,
                        two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
                 FROM user_profiles WHERE email = ?1",
                params![&email],
                map_user_profile_row,
            );

            match result {
                Ok(profile) => Ok(Some(profile)),
                Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
                Err(err) => Err(map_storage_error(err)),
            }
        })
        .await
        .map_err(map_join_error)?
    }

    async fn create(&self, profile: UserProfile) -> DomainResult<()> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            insert_user_profile(&conn, &profile).map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn update(&self, profile: UserProfile) -> DomainResult<()> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            update_user_profile(&conn, &profile).map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn get_current_profile(&self) -> DomainResult<Option<UserProfile>> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<Option<UserProfile>> {
            let conn = db.get_connection()?;

            let result = conn.query_row(
                "SELECT id, auth0_id, email, name, first_name, last_name, display_name,
                        avatar_url, phone_number, title, department, location, bio,
                        timezone, language, locale, date_format, is_active, email_verified,
                        two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
                 FROM user_profiles
                 ORDER BY created_at ASC
                 LIMIT 1",
                &[],
                map_user_profile_row,
            );

            match result {
                Ok(profile) => Ok(Some(profile)),
                Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
                Err(err) => Err(map_storage_error(err)),
            }
        })
        .await
        .map_err(map_join_error)?
    }

    async fn upsert(&self, profile: UserProfile) -> DomainResult<()> {
        let db = Arc::clone(&self.db);

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            upsert_user_profile(&conn, &profile).map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }

    async fn delete(&self, id: &str) -> DomainResult<()> {
        let db = Arc::clone(&self.db);
        let id = id.to_string();

        task::spawn_blocking(move || -> DomainResult<()> {
            let conn = db.get_connection()?;
            conn.execute("DELETE FROM user_profiles WHERE id = ?1", params![&id])
                .map_err(StorageError::from)
                .map_err(map_storage_error)?;
            Ok(())
        })
        .await
        .map_err(map_join_error)?
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Map a row to a UserProfile
fn map_user_profile_row(row: &Row) -> rusqlite::Result<UserProfile> {
    Ok(UserProfile {
        id: row.get(0)?,
        auth0_id: row.get(1)?,
        email: row.get(2)?,
        name: row.get(3)?,
        first_name: row.get(4)?,
        last_name: row.get(5)?,
        display_name: row.get(6)?,
        avatar_url: row.get(7)?,
        phone_number: row.get(8)?,
        title: row.get(9)?,
        department: row.get(10)?,
        location: row.get(11)?,
        bio: row.get(12)?,
        timezone: row.get(13)?,
        language: row.get(14)?,
        locale: row.get(15)?,
        date_format: row.get(16)?,
        is_active: int_to_bool(row.get(17)?),
        email_verified: int_to_bool(row.get(18)?),
        two_factor_enabled: int_to_bool(row.get(19)?),
        last_login_at: row.get(20)?,
        last_synced_at: row.get(21)?,
        created_at: row.get(22)?,
        updated_at: row.get(23)?,
    })
}

/// Insert a user profile
fn insert_user_profile(
    conn: &SqlCipherConnection,
    profile: &UserProfile,
) -> Result<(), StorageError> {
    let params: [&dyn ToSql; 24] = [
        &profile.id,
        &profile.auth0_id,
        &profile.email,
        &profile.name,
        &profile.first_name,
        &profile.last_name,
        &profile.display_name,
        &profile.avatar_url,
        &profile.phone_number,
        &profile.title,
        &profile.department,
        &profile.location,
        &profile.bio,
        &profile.timezone,
        &profile.language,
        &profile.locale,
        &profile.date_format,
        &bool_to_int(profile.is_active),
        &bool_to_int(profile.email_verified),
        &bool_to_int(profile.two_factor_enabled),
        &profile.last_login_at,
        &profile.last_synced_at,
        &profile.created_at,
        &profile.updated_at,
    ];

    conn.execute(
        "INSERT INTO user_profiles (
            id, auth0_id, email, name, first_name, last_name, display_name,
            avatar_url, phone_number, title, department, location, bio,
            timezone, language, locale, date_format, is_active, email_verified,
            two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)",
        params.as_slice(),
    )?;

    Ok(())
}

/// Update a user profile
fn update_user_profile(
    conn: &SqlCipherConnection,
    profile: &UserProfile,
) -> Result<(), StorageError> {
    let params: [&dyn ToSql; 23] = [
        &profile.auth0_id,
        &profile.email,
        &profile.name,
        &profile.first_name,
        &profile.last_name,
        &profile.display_name,
        &profile.avatar_url,
        &profile.phone_number,
        &profile.title,
        &profile.department,
        &profile.location,
        &profile.bio,
        &profile.timezone,
        &profile.language,
        &profile.locale,
        &profile.date_format,
        &bool_to_int(profile.is_active),
        &bool_to_int(profile.email_verified),
        &bool_to_int(profile.two_factor_enabled),
        &profile.last_login_at,
        &profile.last_synced_at,
        &profile.updated_at,
        &profile.id, // WHERE clause
    ];

    conn.execute(
        "UPDATE user_profiles SET
            auth0_id = ?1, email = ?2, name = ?3, first_name = ?4, last_name = ?5,
            display_name = ?6, avatar_url = ?7, phone_number = ?8, title = ?9,
            department = ?10, location = ?11, bio = ?12, timezone = ?13, language = ?14,
            locale = ?15, date_format = ?16, is_active = ?17, email_verified = ?18,
            two_factor_enabled = ?19, last_login_at = ?20, last_synced_at = ?21, updated_at = ?22
         WHERE id = ?23",
        params.as_slice(),
    )?;

    Ok(())
}

/// Upsert a user profile (insert or update based on auth0_id conflict)
///
/// This matches the legacy behavior where the unique constraint is on auth0_id.
/// If a profile with the same auth0_id already exists, it will be updated.
fn upsert_user_profile(
    conn: &SqlCipherConnection,
    profile: &UserProfile,
) -> Result<(), StorageError> {
    let params: [&dyn ToSql; 24] = [
        &profile.id,
        &profile.auth0_id,
        &profile.email,
        &profile.name,
        &profile.first_name,
        &profile.last_name,
        &profile.display_name,
        &profile.avatar_url,
        &profile.phone_number,
        &profile.title,
        &profile.department,
        &profile.location,
        &profile.bio,
        &profile.timezone,
        &profile.language,
        &profile.locale,
        &profile.date_format,
        &bool_to_int(profile.is_active),
        &bool_to_int(profile.email_verified),
        &bool_to_int(profile.two_factor_enabled),
        &profile.last_login_at,
        &profile.last_synced_at,
        &profile.created_at,
        &profile.updated_at,
    ];

    conn.execute(
        "INSERT INTO user_profiles (
            id, auth0_id, email, name, first_name, last_name, display_name,
            avatar_url, phone_number, title, department, location, bio,
            timezone, language, locale, date_format, is_active, email_verified,
            two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24)
         ON CONFLICT(auth0_id) DO UPDATE SET
            id = excluded.id,
            email = excluded.email,
            name = excluded.name,
            first_name = excluded.first_name,
            last_name = excluded.last_name,
            display_name = excluded.display_name,
            avatar_url = excluded.avatar_url,
            phone_number = excluded.phone_number,
            title = excluded.title,
            department = excluded.department,
            location = excluded.location,
            bio = excluded.bio,
            timezone = excluded.timezone,
            language = excluded.language,
            locale = excluded.locale,
            date_format = excluded.date_format,
            is_active = excluded.is_active,
            email_verified = excluded.email_verified,
            two_factor_enabled = excluded.two_factor_enabled,
            last_login_at = excluded.last_login_at,
            last_synced_at = excluded.last_synced_at,
            updated_at = excluded.updated_at",
        params.as_slice(),
    )?;

    Ok(())
}

// =============================================================================
// Error Mapping
// =============================================================================

fn map_storage_error(err: StorageError) -> PulseArcError {
    match err {
        StorageError::WrongKeyOrNotEncrypted => {
            PulseArcError::Database("Database key error or not encrypted".into())
        }
        StorageError::Connection(msg) => PulseArcError::Database(msg),
        StorageError::Query(msg) => PulseArcError::Database(msg),
        StorageError::DatabaseError(msg) => PulseArcError::Database(msg),
        StorageError::Encryption(msg) => {
            PulseArcError::Database(format!("Encryption error: {msg}"))
        }
        StorageError::Migration(msg) => PulseArcError::Database(format!("Migration error: {msg}")),
        StorageError::Keychain(msg) => PulseArcError::Database(format!("Keychain error: {msg}")),
        StorageError::Rusqlite(err) => PulseArcError::Database(format!("SQLite error: {err}")),
        _ => PulseArcError::Database(format!("Storage error: {err}")),
    }
}

fn map_join_error(err: task::JoinError) -> PulseArcError {
    PulseArcError::Internal(format!("Task join error: {err}"))
}

// =============================================================================
// Utility Functions
// =============================================================================

fn bool_to_int(value: bool) -> i64 {
    if value {
        1
    } else {
        0
    }
}

fn int_to_bool(value: i64) -> bool {
    value != 0
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::TempDir;

    use super::*;

    fn setup_test_db() -> (Arc<DbManager>, TempDir) {
        let temp_dir = TempDir::new().expect("create temp dir");
        let db_path = temp_dir.path().join("test.db");
        let manager = DbManager::new(db_path.to_str().unwrap(), 5, Some("test-key"))
            .expect("create db manager");
        manager.run_migrations().expect("run migrations");
        (Arc::new(manager), temp_dir)
    }

    fn create_test_profile() -> UserProfile {
        let now = Utc::now().timestamp();
        UserProfile {
            id: "test-id-123".into(),
            auth0_id: "auth0|123456".into(),
            email: "test@example.com".into(),
            name: Some("Test User".into()),
            first_name: Some("Test".into()),
            last_name: Some("User".into()),
            display_name: Some("Test U.".into()),
            avatar_url: Some("https://example.com/avatar.jpg".into()),
            phone_number: Some("+1234567890".into()),
            title: Some("Engineer".into()),
            department: Some("Engineering".into()),
            location: Some("San Francisco".into()),
            bio: Some("Test bio".into()),
            timezone: "America/Los_Angeles".into(),
            language: "en".into(),
            locale: "en-US".into(),
            date_format: "YYYY-MM-DD".into(),
            is_active: true,
            email_verified: true,
            two_factor_enabled: false,
            last_login_at: now,
            last_synced_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_create_and_get_by_id() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);
        let profile = create_test_profile();

        // Create
        repo.create(profile.clone()).await.expect("create profile");

        // Get by ID
        let retrieved = repo.get_by_id(&profile.id).await.expect("get profile");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.id, profile.id);
        assert_eq!(retrieved.email, profile.email);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_auth0_id() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);
        let profile = create_test_profile();

        repo.create(profile.clone()).await.expect("create profile");

        let retrieved = repo.get_by_auth0_id(&profile.auth0_id).await.expect("get profile");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.auth0_id, profile.auth0_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_by_email() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);
        let profile = create_test_profile();

        repo.create(profile.clone()).await.expect("create profile");

        let retrieved = repo.get_by_email(&profile.email).await.expect("get profile");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.email, profile.email);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_nonexistent_returns_none() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);

        let retrieved = repo.get_by_id("nonexistent").await.expect("get profile");
        assert!(retrieved.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);
        let mut profile = create_test_profile();

        repo.create(profile.clone()).await.expect("create profile");

        // Update
        profile.name = Some("Updated Name".into());
        profile.email = "updated@example.com".into();
        repo.update(profile.clone()).await.expect("update profile");

        // Verify
        let retrieved = repo.get_by_id(&profile.id).await.expect("get profile");
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.name, Some("Updated Name".into()));
        assert_eq!(retrieved.email, "updated@example.com");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_delete() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);
        let profile = create_test_profile();

        repo.create(profile.clone()).await.expect("create profile");
        repo.delete(&profile.id).await.expect("delete profile");

        let retrieved = repo.get_by_id(&profile.id).await.expect("get profile");
        assert!(retrieved.is_none());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_boolean_fields() {
        let (db, _temp_dir) = setup_test_db();
        let repo = SqlCipherUserProfileRepository::new(db);
        let mut profile = create_test_profile();
        profile.is_active = false;
        profile.email_verified = false;
        profile.two_factor_enabled = true;

        repo.create(profile.clone()).await.expect("create profile");

        let retrieved = repo.get_by_id(&profile.id).await.expect("get profile").unwrap();
        assert!(!retrieved.is_active);
        assert!(!retrieved.email_verified);
        assert!(retrieved.two_factor_enabled);
    }
}
