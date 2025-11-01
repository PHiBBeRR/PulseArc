//! User profile commands for Phase 4A.2 migration
//!
//! These commands provide user profile management (get and upsert operations).
//! All commands support feature flag toggling between new (hexagonal
//! architecture) and legacy implementations.

use std::sync::Arc;
use std::time::Instant;

use pulsearc_domain::{PulseArcError, Result as DomainResult, UserProfile};
use tauri::State;
use tracing::info;

use crate::context::AppContext;
use crate::utils::logging::{log_command_execution, record_command_metric, MetricRecord};

// =============================================================================
// Command 1: get_user_profile
// =============================================================================

/// Get user profile from local database.
///
/// Returns the user profile if one exists, or None if no profile is found.
/// This is typically used after a user has logged in and synced their profile
/// from Auth0.
///
/// # Feature Flag
///
/// Controlled by `new_user_profile_commands` flag (default: disabled, uses
/// legacy).
#[tauri::command]
pub async fn get_user_profile(
    ctx: State<'_, Arc<AppContext>>,
) -> Result<Option<UserProfile>, String> {
    let command_name = "user_profile::get_user_profile";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag (fail-safe: use legacy on error)
    let use_new =
        app_ctx.feature_flags.is_enabled("new_user_profile_commands", false).await.unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };
    info!(command = command_name, implementation, "Executing get_user_profile");

    let result = if use_new {
        new_get_user_profile(&app_ctx).await
    } else {
        legacy_get_user_profile(&app_ctx).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success,
            error_type: error_label.as_deref(),
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// New implementation using repository port (public for integration tests)
pub async fn new_get_user_profile(ctx: &AppContext) -> DomainResult<Option<UserProfile>> {
    // Use the repository port to get the current profile (single-user system
    // assumption)
    ctx.user_profile.get_current_profile().await
}

/// Legacy implementation using raw SQL (public for integration tests)
#[allow(dead_code)] // Will be removed in Phase 5
pub async fn legacy_get_user_profile(ctx: &AppContext) -> DomainResult<Option<UserProfile>> {
    // Legacy implementation (functional, not the original stub)
    // Uses raw SQL directly instead of the repository pattern
    let db = ctx.db.clone();

    tokio::task::spawn_blocking(move || -> DomainResult<Option<UserProfile>> {
        let conn = db
            .get_connection()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))?;

        let result = conn.query_row(
            "SELECT id, auth0_id, email, name, first_name, last_name, display_name,
                    avatar_url, phone_number, title, department, location, bio,
                    timezone, language, locale, date_format, is_active, email_verified,
                    two_factor_enabled, last_login_at, last_synced_at, created_at, updated_at
             FROM user_profiles
             ORDER BY created_at ASC
             LIMIT 1",
            &[],
            |row| {
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
            },
        );

        use pulsearc_common::storage::error::StorageError;
        match result {
            Ok(profile) => Ok(Some(profile)),
            Err(StorageError::Rusqlite(rusqlite::Error::QueryReturnedNoRows)) => Ok(None),
            Err(err) => {
                Err(PulseArcError::Database(format!("Failed to query user profile: {}", err)))
            }
        }
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}

// =============================================================================
// Command 2: upsert_user_profile
// =============================================================================

/// Upsert user profile to local database.
///
/// Creates a new profile if one doesn't exist with the given ID, or updates
/// the existing profile. This is typically called after syncing from
/// Auth0/Neon.
///
/// # Feature Flag
///
/// Controlled by `new_user_profile_commands` flag (default: disabled, uses
/// legacy).
#[tauri::command]
pub async fn upsert_user_profile(
    ctx: State<'_, Arc<AppContext>>,
    profile: UserProfile,
) -> Result<(), String> {
    let command_name = "user_profile::upsert_user_profile";
    let start = Instant::now();
    let app_ctx = Arc::clone(ctx.inner());

    // Check feature flag
    let use_new =
        app_ctx.feature_flags.is_enabled("new_user_profile_commands", false).await.unwrap_or(false);

    let implementation = if use_new { "new" } else { "legacy" };
    info!(
        command = command_name,
        implementation,
        profile_id = %profile.id,
        "Executing upsert_user_profile"
    );

    let result = if use_new {
        new_upsert_user_profile(&app_ctx, profile).await
    } else {
        legacy_upsert_user_profile(&app_ctx, profile).await
    };

    // Record metrics
    let success = result.is_ok();
    let elapsed = start.elapsed();
    let error_label = result.as_ref().err().map(|e| format!("{:?}", e));
    log_command_execution(command_name, implementation, elapsed, success);
    record_command_metric(
        &app_ctx,
        MetricRecord {
            command: command_name,
            implementation,
            elapsed,
            success,
            error_type: error_label.as_deref(),
        },
    )
    .await;

    result.map_err(|e| e.to_string())
}

/// New implementation using repository port with upsert (public for integration
/// tests)
pub async fn new_upsert_user_profile(ctx: &AppContext, profile: UserProfile) -> DomainResult<()> {
    // Use the repository's upsert method which handles ON CONFLICT(auth0_id)
    // This matches legacy behavior where auth0_id is the unique constraint
    ctx.user_profile.upsert(profile).await
}

/// Legacy implementation using raw SQL with ON CONFLICT (public for integration
/// tests)
#[allow(dead_code)] // Will be removed in Phase 5
pub async fn legacy_upsert_user_profile(
    ctx: &AppContext,
    profile: UserProfile,
) -> DomainResult<()> {
    // Legacy implementation using ON CONFLICT(auth0_id) directly in SQL
    // This matches the original legacy behavior from
    // legacy/api/src/domain/user_profile.rs:109
    let db = ctx.db.clone();

    tokio::task::spawn_blocking(move || -> DomainResult<()> {
        let conn = db
            .get_connection()
            .map_err(|e| PulseArcError::Database(format!("Failed to get connection: {}", e)))?;

        // Use INSERT ... ON CONFLICT(auth0_id) DO UPDATE (matches legacy exactly)
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
            rusqlite::params![
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
            ],
        )
        .map_err(|e| PulseArcError::Database(format!("Failed to upsert profile: {}", e)))?;

        Ok(())
    })
    .await
    .map_err(|e| PulseArcError::Internal(format!("spawn_blocking failed: {}", e)))?
}

// =============================================================================
// Helper Functions
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
