//! User profile types
//!
//! User profile stored in local database, synced from Auth0

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-gen")]
use ts_rs::TS;

/// User profile stored in local database
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-gen", derive(TS))]
#[cfg_attr(feature = "ts-gen", ts(export))]
pub struct UserProfile {
    pub id: String,
    pub auth0_id: String,
    pub email: String,
    /// Organization ID from Auth0 (org_id claim or app_metadata)
    pub org_id: String,
    pub name: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub phone_number: Option<String>,
    pub title: Option<String>,
    pub department: Option<String>,
    pub location: Option<String>,
    pub bio: Option<String>,
    pub timezone: String,
    pub language: String,
    pub locale: String,
    pub date_format: String,
    pub is_active: bool,
    pub email_verified: bool,
    pub two_factor_enabled: bool,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub last_login_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub last_synced_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts-gen", ts(type = "number"))]
    pub updated_at: i64,
}
