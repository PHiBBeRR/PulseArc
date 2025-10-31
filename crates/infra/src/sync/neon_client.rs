//! Neon API client for remote database synchronization
//!
//! This module provides HTTP-based synchronization to Neon (remote Postgres).
//! It replaces direct database connections with REST API calls for better
//! security, observability, and compliance with CLAUDE.md patterns.
//!
//! # Architecture
//!
//! - Uses Phase 3A HttpClient (no direct reqwest)
//! - Credentials stored in keychain via `pulsearc_common::security`
//! - Structured tracing with request/response logging
//! - Timeout wrapping on all API calls
//!
//! # Compliance
//!
//! - **CLAUDE.md §3**: Structured tracing only (no println!)
//! - **CLAUDE.md §5**: Timeout on all external calls
//! - **CLAUDE.md §9**: No secrets in code, keychain for credentials

use std::sync::Arc;
use std::time::Duration;

use futures::future::BoxFuture;
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument, warn};

use pulsearc_common::security::KeychainProvider;
use pulsearc_core::domain::ActivitySegment;

use crate::http::HttpClient;

use super::errors::SyncError;

/// Configuration for Neon client
#[derive(Debug, Clone)]
pub struct NeonClientConfig {
    /// Base URL for Neon API (e.g., "https://api.neon.tech/v1")
    pub base_url: String,
    /// Timeout for API requests
    pub timeout: Duration,
    /// Max retry attempts for transient failures
    pub max_retries: usize,
    /// Keychain service name for credential storage
    pub keychain_service_name: String,
}

impl Default for NeonClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.neon.tech/v1".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            keychain_service_name: "PulseArc.neon".to_string(),
        }
    }
}

/// Neon API client for remote sync
///
/// Provides HTTP-based synchronization of activity data to remote Neon database.
/// Uses HttpClient from Phase 3A with automatic retry and timeout handling.
pub struct NeonClient {
    http_client: Arc<HttpClient>,
    config: NeonClientConfig,
    keychain: Arc<KeychainProvider>,
}

/// Request/response types for Neon API
#[derive(Debug, Clone, Serialize)]
struct CreateSegmentRequest {
    segment: ActivitySegment,
    idempotency_key: String,
}

#[derive(Debug, Clone, Deserialize)]
struct CreateSegmentResponse {
    id: String,
    created: bool, // false if idempotency key matched existing
}

#[derive(Debug, Clone, Serialize)]
struct BatchSegmentsRequest {
    segments: Vec<ActivitySegment>,
}

#[derive(Debug, Clone, Deserialize)]
struct BatchSegmentsResponse {
    created: usize,
    duplicates: usize,
    failed: usize,
}

impl NeonClient {
    /// Create a new Neon client with default configuration
    ///
    /// # Errors
    ///
    /// Returns error if HttpClient cannot be built
    pub fn new() -> Result<Self, SyncError> {
        Self::with_config(NeonClientConfig::default())
    }

    /// Create a new Neon client with custom configuration
    ///
    /// # Arguments
    ///
    /// * `config` - Client configuration
    ///
    /// # Errors
    ///
    /// Returns error if HttpClient cannot be built
    pub fn with_config(config: NeonClientConfig) -> Result<Self, SyncError> {
        let http_client = HttpClient::builder()
            .timeout(config.timeout)
            .max_attempts(config.max_retries)
            .build()
            .map_err(|e| SyncError::Config(format!("Failed to build HttpClient: {}", e)))?;

        let keychain = KeychainProvider::new(&config.keychain_service_name);

        Ok(Self {
            http_client: Arc::new(http_client),
            config,
            keychain: Arc::new(keychain),
        })
    }

    /// Get API token from keychain
    ///
    /// # Errors
    ///
    /// Returns error if token is not found in keychain
    fn get_api_token(&self) -> Result<String, SyncError> {
        self.keychain
            .get_secret("api_token")
            .map_err(|e| SyncError::Auth(format!("Failed to get API token: {}", e)))
    }

    /// Set API token in keychain
    ///
    /// # Arguments
    ///
    /// * `token` - API token to store
    ///
    /// # Errors
    ///
    /// Returns error if keychain operation fails
    pub fn set_api_token(&self, token: &str) -> Result<(), SyncError> {
        self.keychain
            .set_secret("api_token", token)
            .map_err(|e| SyncError::Config(format!("Failed to store API token: {}", e)))
    }

    /// Sync a single activity segment to Neon
    ///
    /// # Arguments
    ///
    /// * `segment` - Activity segment to sync
    /// * `idempotency_key` - Unique key for deduplication
    ///
    /// # Returns
    ///
    /// Remote segment ID (existing or newly created)
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self, segment), fields(segment_id = %segment.id))]
    pub async fn sync_segment(
        &self,
        segment: &ActivitySegment,
        idempotency_key: String,
    ) -> Result<String, SyncError> {
        let token = self.get_api_token()?;
        let url = format!("{}/segments", self.config.base_url);

        let request_body = CreateSegmentRequest {
            segment: segment.clone(),
            idempotency_key,
        };

        debug!(url = %url, "Syncing segment to Neon");

        let request_builder = self
            .http_client
            .request(Method::POST, &url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request_body);

        let response = self.send_request(request_builder).await?;

        let result: CreateSegmentResponse = response
            .json()
            .await
            .map_err(|e| SyncError::Client(format!("Failed to parse response: {}", e)))?;

        if result.created {
            info!(segment_id = %result.id, "Created new segment in Neon");
        } else {
            debug!(segment_id = %result.id, "Segment already exists (idempotent)");
        }

        Ok(result.id)
    }

    /// Batch sync multiple segments to Neon
    ///
    /// # Arguments
    ///
    /// * `segments` - Activity segments to sync
    ///
    /// # Returns
    ///
    /// Sync statistics (created, duplicates, failed)
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self, segments), fields(count = segments.len()))]
    pub async fn sync_segments_batch(
        &self,
        segments: Vec<ActivitySegment>,
    ) -> Result<BatchSegmentsResponse, SyncError> {
        let token = self.get_api_token()?;
        let url = format!("{}/segments/batch", self.config.base_url);

        let request_body = BatchSegmentsRequest { segments };

        debug!(url = %url, count = request_body.segments.len(), "Batch syncing segments");

        let request_builder = self
            .http_client
            .request(Method::POST, &url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&request_body);

        let response = self.send_request(request_builder).await?;

        let result: BatchSegmentsResponse = response
            .json()
            .await
            .map_err(|e| SyncError::Client(format!("Failed to parse response: {}", e)))?;

        info!(
            created = result.created,
            duplicates = result.duplicates,
            failed = result.failed,
            "Batch sync completed"
        );

        Ok(result)
    }

    /// Health check for Neon API
    ///
    /// # Returns
    ///
    /// `true` if API is reachable and healthy
    ///
    /// # Errors
    ///
    /// Returns error if health check fails
    #[instrument(skip(self))]
    pub async fn health_check(&self) -> Result<bool, SyncError> {
        let url = format!("{}/health", self.config.base_url);

        debug!(url = %url, "Performing health check");

        let request_builder = self.http_client.request(Method::GET, &url);

        let response = self.send_request(request_builder).await?;

        if response.status().is_success() {
            info!("Neon API is healthy");
            Ok(true)
        } else {
            warn!(status = %response.status(), "Neon API returned non-success status");
            Ok(false)
        }
    }

    async fn send_request(&self, builder: RequestBuilder) -> Result<Response, SyncError> {
        let request = builder.build().map_err(|err| SyncError::Client(err.to_string()))?;

        if request.url().scheme() == "file" {
            return Err(SyncError::Config("file:// URLs are not supported".into()));
        }

        let method = request.method().clone();
        let url = request.url().clone();

        let response = tokio::time::timeout(self.config.timeout, self.http_client.execute(request))
            .await
            .map_err(|_| SyncError::Timeout(self.config.timeout))??;

        let status = response.status();

        if status == StatusCode::SERVICE_UNAVAILABLE {
            Err(SyncError::RateLimit("Neon service unavailable".into()))
        } else if ABNORMAL_METHODS.contains(&method) {
            Err(SyncError::Client(format!("HTTP method {} is not allowed", method)))
        } else {
            Ok(response)
        }
    }
}

const ABNORMAL_METHODS: &[Method] = &[Method::TRACE, Method::CONNECT];

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{
        matchers::{header, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn test_health_check_success() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let config = NeonClientConfig {
            base_url: mock_server.uri(),
            ..Default::default()
        };

        let client = NeonClient::with_config(config).unwrap();

        let result = client.health_check().await;
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_health_check_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&mock_server)
            .await;

        let config = NeonClientConfig {
            base_url: mock_server.uri(),
            ..Default::default()
        };

        let client = NeonClient::with_config(config).unwrap();

        let result = client.health_check().await;
        assert!(result.is_ok());
        assert!(!result.unwrap()); // Unhealthy but no error
    }

    #[tokio::test]
    async fn test_sync_segment_requires_auth() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/segments"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let config = NeonClientConfig {
            base_url: mock_server.uri(),
            keychain_service_name: "PulseArc.neon.test".to_string(),
            ..Default::default()
        };

        let client = NeonClient::with_config(config).unwrap();

        // Set a test token
        client.set_api_token("test-token").unwrap();

        let segment = ActivitySegment {
            id: "test123".to_string(),
            user_id: "user1".to_string(),
            start_time: chrono::Utc::now(),
            end_time: chrono::Utc::now() + chrono::Duration::minutes(30),
            duration_secs: 1800,
            application: Some("VSCode".to_string()),
            title: Some("main.rs".to_string()),
            url: None,
            idle: false,
            project_id: None,
            category: None,
        };

        let result = client
            .sync_segment(&segment, "idempotency-key-123".to_string())
            .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), SyncError::Auth(_)));
    }
}
