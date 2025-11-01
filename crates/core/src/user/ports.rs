//! Port interfaces for user profile management
//!
//! These traits define the boundaries between core business logic
//! and infrastructure implementations for user profile operations.

use async_trait::async_trait;
use pulsearc_domain::{Result, UserProfile};

/// Trait for user profile persistence and retrieval
#[async_trait]
pub trait UserProfileRepository: Send + Sync {
    /// Get user profile by ID
    async fn get_by_id(&self, id: &str) -> Result<Option<UserProfile>>;

    /// Get user profile by Auth0 ID
    async fn get_by_auth0_id(&self, auth0_id: &str) -> Result<Option<UserProfile>>;

    /// Get user profile by email
    async fn get_by_email(&self, email: &str) -> Result<Option<UserProfile>>;

    /// Create a new user profile
    async fn create(&self, profile: UserProfile) -> Result<()>;

    /// Update an existing user profile
    async fn update(&self, profile: UserProfile) -> Result<()>;

    /// Delete a user profile by ID
    async fn delete(&self, id: &str) -> Result<()>;
}
