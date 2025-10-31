/// SAP GraphQL client for time entry forwarding and WBS validation
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use pulsearc_core::classification::ports::WbsRepository;
use pulsearc_core::sap_ports::{SapClient as SapClientTrait, SapEntryId, TimeEntry};
use pulsearc_domain::{PulseArcError, Result};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::http::HttpClient;
use super::cache::{WbsCache, WbsCacheConfig};
use super::validation::WbsValidator;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const HEALTH_CHECK_TIMEOUT_SECS: u64 = 5;

/// SAP GraphQL client for interacting with sap-connector API
pub struct SapClient {
    base_url: String,
    http_client: HttpClient,
    wbs_validator: Arc<WbsValidator>,
    user_id: String,
    access_token_provider: Arc<dyn AccessTokenProvider>,
}

/// Provides OAuth access tokens for the SAP connector.
#[async_trait]
pub trait AccessTokenProvider: Send + Sync {
    /// Retrieve a bearer token to authorize SAP API calls.
    async fn access_token(&self) -> Result<String>;
}

impl SapClient {
    /// Create a new SAP client with default cache configuration
    ///
    /// Creates an internal WBS cache with default TTL (5 minutes) and
    /// validator. For custom cache configuration, use `with_cache()`.
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the sap-connector GraphQL API (e.g., "http://localhost:3000")
    /// * `wbs_repository` - Repository for WBS validation
    /// * `user_id` - SAP user ID for time entry submissions
    /// * `access_token_provider` - Async provider that yields OAuth access tokens
    ///
    /// # Returns
    /// A configured SAP client
    pub fn new(
        base_url: String,
        wbs_repository: Arc<dyn WbsRepository>,
        user_id: String,
        access_token_provider: Arc<dyn AccessTokenProvider>,
    ) -> Result<Self> {
        // Create default cache and validator
        let cache_config = WbsCacheConfig::default();
        let cache = Arc::new(WbsCache::new(cache_config));
        let validator = Arc::new(WbsValidator::new(cache, wbs_repository));

        Self::with_validator(
            base_url,
            validator,
            user_id,
            access_token_provider,
        )
    }

    /// Create a new SAP client with custom validator (for testing)
    ///
    /// Allows injection of custom validator instance, useful for
    /// testing with `MockClock` or custom cache configurations.
    ///
    /// # Arguments
    /// * `base_url` - Base URL of the sap-connector GraphQL API
    /// * `wbs_validator` - Custom WBS validator instance (contains cache + repository)
    /// * `user_id` - SAP user ID for time entry submissions
    /// * `access_token_provider` - Async provider that yields OAuth access tokens
    pub fn with_validator(
        base_url: String,
        wbs_validator: Arc<WbsValidator>,
        user_id: String,
        access_token_provider: Arc<dyn AccessTokenProvider>,
    ) -> Result<Self> {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .max_attempts(3)
            .build()?;

        Ok(Self {
            base_url,
            http_client,
            wbs_validator,
            user_id,
            access_token_provider,
        })
    }

    /// Check if SAP connector server is reachable
    ///
    /// Performs a lightweight HEAD request to the /health endpoint with 5s timeout.
    ///
    /// # Returns
    /// * `Ok(true)` - Server is reachable
    /// * `Ok(false)` - Server unreachable or timeout
    pub async fn check_health(&self) -> Result<bool> {
        let health_endpoint = format!("{}/health", self.base_url);

        // Create short-lived client with 5s timeout
        let health_client = HttpClient::builder()
            .timeout(Duration::from_secs(HEALTH_CHECK_TIMEOUT_SECS))
            .max_attempts(1)
            .build()?;

        let request_builder = health_client.request(Method::HEAD, &health_endpoint);
        match health_client.send(request_builder).await {
            Ok(response) => Ok(response.status().is_success()),
            Err(PulseArcError::Network(_)) => {
                warn!("SAP health check failed: network error");
                Ok(false)
            }
            Err(e) => Err(e),
        }
    }

    /// Submit a single time entry to SAP via GraphQL
    ///
    /// # Arguments
    /// * `entry` - Time entry to submit
    /// * `access_token` - JWT access token for authentication
    ///
    /// # Returns
    /// SAP entry ID (correlation ID)
    async fn submit_time_entry(
        &self,
        entry: &TimeEntry,
        access_token: &str,
    ) -> Result<SapEntryId> {
        let correlation_id = uuid::Uuid::new_v4().to_string();

        let query = r#"
            mutation SubmitTimeEntries($entries: [TimeEntryInput!]!) {
                submitTimeEntries(entries: $entries) {
                    acceptedCount
                    duplicateCount
                    errors {
                        correlationId
                        code
                        message
                    }
                }
            }
        "#;

        // Convert duration from hours to seconds for API
        let duration_seconds = (entry.duration_hours * 3600.0).round() as i32;

        let time_entry_input = TimeEntryInput {
            user_id: self.user_id.clone(),
            date: entry.date.clone(),
            wbs_code: entry.wbs_code.clone(),
            duration: duration_seconds,
            note: if entry.description.is_empty() {
                None
            } else {
                Some(entry.description.clone())
            },
            correlation_id: correlation_id.clone(),
        };

        let variables = serde_json::json!({
            "entries": [time_entry_input]
        });

        let result = self
            .execute_graphql::<SubmitTimeEntriesResponse>(query, Some(variables), access_token)
            .await?;

        // Check for errors in the response and include all correlation IDs
        if !result.submit_time_entries.errors.is_empty() {
            let error_details: Vec<String> = result
                .submit_time_entries
                .errors
                .iter()
                .map(|e| {
                    format!(
                        "[correlation_id={}, code={}, message={}]",
                        e.correlation_id, e.code, e.message
                    )
                })
                .collect();

            return Err(PulseArcError::Network(format!(
                "SAP API errors: {}",
                error_details.join(", ")
            )));
        }

        if result.submit_time_entries.accepted_count == 0 {
            return Err(PulseArcError::Network(format!(
                "Time entry was not accepted by SAP (correlation_id={})",
                correlation_id
            )));
        }

        info!(
            correlation_id = %correlation_id,
            accepted = result.submit_time_entries.accepted_count,
            duplicates = result.submit_time_entries.duplicate_count,
            "Successfully submitted time entry to SAP"
        );

        Ok(correlation_id)
    }

    /// Execute a GraphQL query/mutation
    ///
    /// # Arguments
    /// * `query` - GraphQL query string
    /// * `variables` - Optional variables for the query
    /// * `access_token` - JWT access token for authentication
    ///
    /// # Returns
    /// Parsed response data
    async fn execute_graphql<T: for<'de> Deserialize<'de>>(
        &self,
        query: &str,
        variables: Option<serde_json::Value>,
        access_token: &str,
    ) -> Result<T> {
        let graphql_endpoint = format!("{}/graphql", self.base_url);

        let mut request_body = serde_json::json!({
            "query": query
        });

        if let Some(vars) = variables {
            request_body["variables"] = vars;
        }

        let request_builder = self
            .http_client
            .request(Method::POST, &graphql_endpoint)
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Content-Type", "application/json")
            .json(&request_body);

        let response = self.http_client.send(request_builder).await?;

        let status = response.status();
        debug!(status = status.as_u16(), "Received SAP GraphQL response");

        if !status.is_success() {
            let error_text =
                response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(PulseArcError::Network(format!(
                "SAP API error (HTTP {}): {}",
                status, error_text
            )));
        }

        let graphql_response: GraphQLResponse<T> =
            response.json().await.map_err(|e| {
                PulseArcError::Internal(format!("Failed to parse GraphQL response: {}", e))
            })?;

        if let Some(errors) = graphql_response.errors {
            let error_messages: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
            let combined_message = error_messages.join(", ");
            return Err(PulseArcError::Network(format!("GraphQL errors: {}", combined_message)));
        }

        graphql_response
            .data
            .ok_or_else(|| PulseArcError::Internal("GraphQL response missing data field".into()))
    }
}

#[async_trait]
impl SapClientTrait for SapClient {
    async fn forward_entry(&self, entry: &TimeEntry) -> Result<SapEntryId> {
        // Get access token from provider (auto-refreshes if using OAuth service)
        let access_token = self.access_token_provider.access_token().await?;
        self.submit_time_entry(entry, &access_token).await
    }

    async fn validate_wbs(&self, wbs_code: &str) -> Result<bool> {
        // Use validator with caching for performance
        let result = self.wbs_validator.validate(wbs_code)?;

        // Log warnings for visibility
        if let Some(message) = result.message() {
            if result.is_ok() {
                debug!(wbs_code, message, "WBS validation warning");
            } else {
                warn!(wbs_code, message, "WBS validation failed");
            }
        }

        Ok(result.is_ok())
    }
}

// =============================================================================
// GraphQL Types
// =============================================================================

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimeEntryInput {
    user_id: String,
    date: String, // YYYY-MM-DD
    wbs_code: String,
    duration: i32, // seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
    correlation_id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubmitTimeEntriesResponse {
    submit_time_entries: TimeEntryBatchResult,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeEntryBatchResult {
    accepted_count: i32,
    duplicate_count: i32,
    errors: Vec<TimeEntryError>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimeEntryError {
    correlation_id: String,
    code: String,
    message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GraphQLResponse<T> {
    data: Option<T>,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Deserialize)]
struct GraphQLError {
    message: String,
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pulsearc_domain::types::sap::WbsElement;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    // Mock WBS Repository for testing
    struct MockWbsRepository {
        valid_wbs_codes: Vec<String>,
    }

    impl WbsRepository for MockWbsRepository {
        fn count_active_wbs(&self) -> Result<i64> {
            Ok(self.valid_wbs_codes.len() as i64)
        }

        fn get_last_sync_timestamp(&self) -> Result<Option<i64>> {
            Ok(Some(1700000000))
        }

        fn load_common_projects(&self, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn fts5_search_keyword(&self, _keyword: &str, _limit: usize) -> Result<Vec<WbsElement>> {
            Ok(vec![])
        }

        fn get_wbs_by_project_def(&self, _project_def: &str) -> Result<Option<WbsElement>> {
            Ok(None)
        }

        fn get_wbs_by_wbs_code(&self, wbs_code: &str) -> Result<Option<WbsElement>> {
            if self.valid_wbs_codes.contains(&wbs_code.to_string()) {
                Ok(Some(WbsElement {
                    wbs_code: wbs_code.to_string(),
                    project_def: "USC0063201".to_string(),
                    project_name: Some("Test Project".to_string()),
                    description: Some("Test Description".to_string()),
                    status: "REL".to_string(),
                    cached_at: 1700000000,
                    opportunity_id: None,
                    deal_name: None,
                    target_company_name: None,
                    counterparty: None,
                    industry: None,
                    region: None,
                    amount: None,
                    stage_name: None,
                    project_code: None,
                }))
            } else {
                Ok(None)
            }
        }
    }

    #[derive(Clone)]
    struct MockAccessTokenProvider {
        token: Option<String>,
    }

    impl MockAccessTokenProvider {
        fn with_token(token: &str) -> Self {
            Self { token: Some(token.to_string()) }
        }

        fn without_token() -> Self {
            Self { token: None }
        }
    }

    #[async_trait]
    impl AccessTokenProvider for MockAccessTokenProvider {
        async fn access_token(&self) -> Result<String> {
            match &self.token {
                Some(token) => Ok(token.clone()),
                None => Err(PulseArcError::Config(
                    "SAP_ACCESS_TOKEN environment variable is required but not set".to_string(),
                )),
            }
        }
    }

    fn create_test_client(base_url: String) -> SapClient {
        let provider: Arc<dyn AccessTokenProvider> =
            Arc::new(MockAccessTokenProvider::with_token("test-token"));
        create_test_client_with_provider(base_url, provider)
    }

    fn create_test_client_with_provider(
        base_url: String,
        provider: Arc<dyn AccessTokenProvider>,
    ) -> SapClient {
        let wbs_repo = Arc::new(MockWbsRepository {
            valid_wbs_codes: vec!["USC0063201.1.1".to_string()],
        });

        SapClient::new(base_url, wbs_repo, "test-user".to_string(), provider)
            .expect("Failed to create client")
    }

    #[tokio::test]
    async fn validates_wbs_code_successfully() {
        let client = create_test_client("http://localhost:3000".to_string());

        let is_valid = client.validate_wbs("USC0063201.1.1").await.expect("Should validate");
        assert!(is_valid);
    }

    #[tokio::test]
    async fn rejects_invalid_wbs_code() {
        let client = create_test_client("http://localhost:3000".to_string());

        let is_valid =
            client.validate_wbs("INVALID-CODE").await.expect("Should check validation");
        assert!(!is_valid);
    }

    #[tokio::test]
    async fn health_check_returns_true_when_server_healthy() {
        let mock_server = MockServer::start().await;

        Mock::given(method("HEAD"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&mock_server)
            .await;

        let client = create_test_client(mock_server.uri());
        let is_healthy = client.check_health().await.expect("Should check health");

        assert!(is_healthy);
    }

    #[tokio::test]
    async fn health_check_returns_false_when_server_unavailable() {
        // Use a closed port that will immediately refuse connections
        let client = create_test_client("http://localhost:9999".to_string());
        let is_healthy = client.check_health().await.expect("Should handle connection failure");

        assert!(!is_healthy);
    }

    #[tokio::test]
    async fn submits_time_entry_successfully() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/graphql"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "submitTimeEntries": {
                        "acceptedCount": 1,
                        "duplicateCount": 0,
                        "errors": []
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(mock_server.uri());
        let entry = TimeEntry {
            wbs_code: "USC0063201.1.1".to_string(),
            description: "Test work".to_string(),
            duration_hours: 2.5,
            date: "2025-10-31".to_string(),
        };

        let result = client.submit_time_entry(&entry, "test-token").await;

        assert!(result.is_ok());
        let entry_id = result.unwrap();
        assert!(!entry_id.is_empty());
    }

    #[tokio::test]
    async fn handles_api_errors_with_correlation_ids() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/graphql"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "submitTimeEntries": {
                        "acceptedCount": 0,
                        "duplicateCount": 0,
                        "errors": [{
                            "correlationId": "test-correlation-id-123",
                            "code": "INVALID_WBS",
                            "message": "WBS code not found"
                        }]
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        let client = create_test_client(mock_server.uri());
        let entry = TimeEntry {
            wbs_code: "INVALID".to_string(),
            description: "Test".to_string(),
            duration_hours: 1.0,
            date: "2025-10-31".to_string(),
        };

        let result = client.submit_time_entry(&entry, "test-token").await;

        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("test-correlation-id-123"));
        assert!(error_msg.contains("INVALID_WBS"));
        assert!(error_msg.contains("WBS code not found"));
    }

    #[tokio::test]
    async fn fails_fast_without_access_token() {
        let provider: Arc<dyn AccessTokenProvider> =
            Arc::new(MockAccessTokenProvider::without_token());
        let client =
            create_test_client_with_provider("http://localhost:3000".to_string(), provider);
        let entry = TimeEntry {
            wbs_code: "USC0063201.1.1".to_string(),
            description: "Test".to_string(),
            duration_hours: 1.0,
            date: "2025-10-31".to_string(),
        };

        let result = client.forward_entry(&entry).await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, PulseArcError::Config(_)));
        assert!(error.to_string().to_lowercase().contains("required"));
    }
}
