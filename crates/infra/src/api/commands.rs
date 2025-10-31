//! API commands for CRUD operations
//!
//! Provides high-level command interface for API operations on segments,
//! snapshots, and blocks.

use std::sync::Arc;

use pulsearc_domain::types::{ActivitySegment, ActivitySnapshot, Block};
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use urlencoding::encode;

use super::client::ApiClient;
use super::errors::ApiError;

/// Request/response types for API operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSegmentRequest {
    pub segment: ActivitySegment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSegmentResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotRequest {
    pub snapshot: ActivitySnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSnapshotResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlockRequest {
    pub block: Block,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlockResponse {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListResponse<T> {
    pub items: Vec<T>,
    pub total: usize,
    pub page: usize,
}

/// API commands for domain operations
pub struct ApiCommands {
    client: Arc<ApiClient>,
}

impl ApiCommands {
    /// Create a new commands instance
    ///
    /// # Arguments
    ///
    /// * `client` - API client
    pub fn new(client: Arc<ApiClient>) -> Self {
        Self { client }
    }

    // === Segment Operations ===

    /// Create a new activity segment
    ///
    /// # Arguments
    ///
    /// * `segment` - Segment to create
    ///
    /// # Returns
    ///
    /// ID of the created segment
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self, segment), fields(segment_id = %segment.id))]
    pub async fn create_segment(&self, segment: &ActivitySegment) -> Result<String, ApiError> {
        let request = CreateSegmentRequest { segment: segment.clone() };

        let response: CreateSegmentResponse = self.client.post("/segments", &request).await?;

        debug!(segment_id = %response.id, "Segment created");
        Ok(response.id)
    }

    /// Get a segment by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Segment ID
    ///
    /// # Returns
    ///
    /// Segment data
    ///
    /// # Errors
    ///
    /// Returns error if segment not found or API request fails
    #[instrument(skip(self), fields(segment_id = %id))]
    pub async fn get_segment(&self, id: &str) -> Result<ActivitySegment, ApiError> {
        let path = format!("/segments/{}", encode(id));
        self.client.get(&path).await
    }

    /// List segments with pagination
    ///
    /// # Arguments
    ///
    /// * `limit` - Max number of segments to return
    ///
    /// # Returns
    ///
    /// List of segments
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self))]
    pub async fn list_segments(
        &self,
        limit: u32,
    ) -> Result<ListResponse<ActivitySegment>, ApiError> {
        let path = format!("/segments?limit={}", limit);
        let response: ListResponse<ActivitySegment> = self.client.get(&path).await?;

        debug!(count = response.items.len(), "Segments listed");
        Ok(response)
    }

    // === Snapshot Operations ===

    /// Create a new activity snapshot
    ///
    /// # Arguments
    ///
    /// * `snapshot` - Snapshot to create
    ///
    /// # Returns
    ///
    /// ID of the created snapshot
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self, snapshot), fields(snapshot_id = %snapshot.id))]
    pub async fn create_snapshot(&self, snapshot: &ActivitySnapshot) -> Result<String, ApiError> {
        let request = CreateSnapshotRequest { snapshot: snapshot.clone() };

        let response: CreateSnapshotResponse = self.client.post("/snapshots", &request).await?;

        debug!(snapshot_id = %response.id, "Snapshot created");
        Ok(response.id)
    }

    /// List snapshots with pagination metadata
    ///
    /// # Arguments
    ///
    /// * `limit` - Max number of snapshots to return
    ///
    /// # Returns
    ///
    /// Paginated snapshots response
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self))]
    pub async fn list_snapshots(
        &self,
        limit: u32,
    ) -> Result<ListResponse<ActivitySnapshot>, ApiError> {
        let path = format!("/snapshots?limit={}", limit);
        let response: ListResponse<ActivitySnapshot> = self.client.get(&path).await?;

        debug!(count = response.items.len(), "Snapshots listed");
        Ok(response)
    }

    // === Block Operations ===

    /// Create a new block
    ///
    /// # Arguments
    ///
    /// * `block` - Block to create
    ///
    /// # Returns
    ///
    /// ID of the created block
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self, block), fields(block_id = %block.id))]
    pub async fn create_block(&self, block: &Block) -> Result<String, ApiError> {
        let request = CreateBlockRequest { block: block.clone() };

        let response: CreateBlockResponse = self.client.post("/blocks", &request).await?;

        debug!(block_id = %response.id, "Block created");
        Ok(response.id)
    }

    /// Get a block by ID
    ///
    /// # Arguments
    ///
    /// * `id` - Block ID
    ///
    /// # Returns
    ///
    /// Block data
    ///
    /// # Errors
    ///
    /// Returns error if block not found or API request fails
    #[instrument(skip(self), fields(block_id = %id))]
    pub async fn get_block(&self, id: &str) -> Result<Block, ApiError> {
        let path = format!("/blocks/{}", encode(id));
        self.client.get(&path).await
    }

    /// List blocks with pagination
    ///
    /// # Arguments
    ///
    /// * `limit` - Max number of blocks to return
    ///
    /// # Returns
    ///
    /// List of blocks
    ///
    /// # Errors
    ///
    /// Returns error if API request fails
    #[instrument(skip(self))]
    pub async fn list_blocks(&self, limit: u32) -> Result<ListResponse<Block>, ApiError> {
        let path = format!("/blocks?limit={}", limit);
        let response: ListResponse<Block> = self.client.get(&path).await?;

        debug!(count = response.items.len(), "Blocks listed");
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use chrono::Utc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::api::auth::AccessTokenProvider;
    use crate::api::client::ApiClientConfig;

    #[derive(Clone)]
    struct MockAuthProvider;

    #[async_trait]
    impl AccessTokenProvider for MockAuthProvider {
        async fn access_token(&self) -> Result<String, ApiError> {
            Ok("test-token".to_string())
        }
    }

    fn create_test_segment() -> ActivitySegment {
        let now = Utc::now().timestamp();
        ActivitySegment {
            id: "seg-123".to_string(),
            start_ts: now - 3600,
            end_ts: now,
            primary_app: "VSCode".to_string(),
            normalized_label: "main.rs".to_string(),
            sample_count: 10,
            dictionary_keys: None,
            created_at: now,
            processed: false,
            snapshot_ids: vec![],
            work_type: None,
            activity_category: "development".to_string(),
            detected_activity: "coding".to_string(),
            extracted_signals_json: None,
            project_match_json: None,
            idle_time_secs: 0,
            active_time_secs: 3600,
            user_action: None,
        }
    }

    #[tokio::test]
    async fn test_create_segment() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/segments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "seg-123"
            })))
            .mount(&mock_server)
            .await;

        let config = ApiClientConfig { base_url: mock_server.uri(), ..Default::default() };

        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = ApiCommands::new(client);

        let segment = create_test_segment();
        let result = commands.create_segment(&segment).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "seg-123");
    }

    #[tokio::test]
    async fn test_create_segment_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/segments"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let config = ApiClientConfig { base_url: mock_server.uri(), ..Default::default() };

        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = ApiCommands::new(client);

        let segment = create_test_segment();
        let result = commands.create_segment(&segment).await;

        assert!(matches!(result, Err(ApiError::Server(_))));
    }

    #[tokio::test]
    async fn test_get_segment_encodes_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/segments/foo%2Fbar"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "foo/bar",
                "start_ts": 0,
                "end_ts": 1,
                "primary_app": "app",
                "normalized_label": "label",
                "sample_count": 1,
                "dictionary_keys": null,
                "created_at": 0,
                "processed": true,
                "snapshot_ids": [],
                "work_type": null,
                "activity_category": "cat",
                "detected_activity": "act",
                "extracted_signals_json": null,
                "project_match_json": null,
                "idle_time_secs": 0,
                "active_time_secs": 1,
                "user_action": null
            })))
            .mount(&mock_server)
            .await;

        let config = ApiClientConfig { base_url: mock_server.uri(), ..Default::default() };

        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = ApiCommands::new(client);

        let result = commands.get_segment("foo/bar").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "foo/bar");
    }

    #[tokio::test]
    async fn test_list_segments() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/segments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [],
                "total": 0,
                "page": 1
            })))
            .mount(&mock_server)
            .await;

        let config = ApiClientConfig { base_url: mock_server.uri(), ..Default::default() };

        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = ApiCommands::new(client);

        let result = commands.list_segments(10).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert_eq!(response.items.len(), 0);
        assert_eq!(response.total, 0);
        assert_eq!(response.page, 1);
    }

    #[tokio::test]
    async fn test_list_segments_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/segments"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let config = ApiClientConfig { base_url: mock_server.uri(), ..Default::default() };

        let client = Arc::new(ApiClient::new(config, Arc::new(MockAuthProvider)).unwrap());
        let commands = ApiCommands::new(client);

        let result = commands.list_segments(10).await;

        assert!(matches!(result, Err(ApiError::Server(_))));
    }
}
