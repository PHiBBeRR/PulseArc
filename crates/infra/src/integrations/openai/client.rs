/// OpenAI API client for block classification
use pulsearc_domain::types::classification::ProposedBlock;
use pulsearc_domain::PulseArcError;
use reqwest::Method;
use serde_json::json;
use tracing::{debug, info};

use crate::http::HttpClient;

use super::types::{
    BlockClassificationResponse, ChatCompletionRequest, ChatCompletionResponse, ChatMessage,
    JsonSchema, LLMBlockResponse, OpenAIError, ResponseFormat,
};

const OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_MAX_TOKENS: u32 = 50_000;
const DEFAULT_TEMPERATURE: f32 = 0.3;

/// Cost per 1M tokens for gpt-4o-mini (as of 2025)
const COST_PER_1M_INPUT_TOKENS: f64 = 0.150;
const COST_PER_1M_OUTPUT_TOKENS: f64 = 0.600;

/// OpenAI API client for classifying time blocks
pub struct OpenAIClient {
    http_client: HttpClient,
    api_key: String,
    model: String,
    api_url: String,
}

impl OpenAIClient {
    /// Create a new OpenAI client
    ///
    /// # Arguments
    /// * `api_key` - OpenAI API key (required)
    /// * `http_client` - HTTP client with retry logic (from Phase 3A)
    ///
    /// # Returns
    /// A configured OpenAI client
    pub fn new(api_key: String, http_client: HttpClient) -> Self {
        Self {
            http_client,
            api_key,
            model: DEFAULT_MODEL.to_string(),
            api_url: OPENAI_API_URL.to_string(),
        }
    }

    /// Create a new client with custom model
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Create a new client with custom API URL (for testing)
    #[cfg(test)]
    pub fn with_api_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = url.into();
        self
    }

    /// Classify a batch of blocks using OpenAI API
    ///
    /// # Arguments
    /// * `blocks` - Vector of proposed blocks to classify
    ///
    /// # Returns
    /// Classification response with billable status, confidence, and cost metrics
    ///
    /// # Errors
    /// Returns `OpenAIError` for network failures, API errors, or invalid responses
    pub async fn classify_blocks(
        &self,
        blocks: &[ProposedBlock],
    ) -> Result<BlockClassificationResponse, OpenAIError> {
        if blocks.is_empty() {
            return Ok(BlockClassificationResponse {
                classifications: vec![],
                tokens_used: 0,
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
            });
        }

        info!(block_count = blocks.len(), "Classifying blocks with OpenAI");

        // 1. Build prompt from blocks
        let prompt = self.build_classification_prompt(blocks);

        // 2. Call OpenAI API
        let response = self.call_api(prompt).await?;

        info!(
            tokens = response.tokens_used,
            cost = response.cost_usd,
            "OpenAI classification complete"
        );

        Ok(response)
    }

    /// Build classification prompt from blocks
    ///
    /// Constructs a structured prompt that includes:
    /// - System message defining the task
    /// - User message with block details and activities
    fn build_classification_prompt(&self, blocks: &[ProposedBlock]) -> String {
        let mut prompt = String::from(
            "Classify each time block as billable (client work) or G&A (non-billable).\n\n",
        );

        for block in blocks {
            prompt.push_str(&format!(
                "Block ID: {}\nDuration: {} seconds\nStart: {}\nEnd: {}\n",
                block.id, block.duration_secs, block.start_ts, block.end_ts
            ));

            if !block.activities.is_empty() {
                prompt.push_str("Activities:\n");
                for activity in &block.activities {
                    prompt.push_str(&format!(
                        "  - {} ({:.1}%) - {}s\n",
                        activity.name, activity.percentage, activity.duration_secs
                    ));
                }
            }

            prompt.push('\n');
        }

        prompt.push_str("Return JSON with 'classifications' array. Each item must have: id, billable (bool), description, confidence (0.0-1.0), reasons (array), and optionally: project_id, wbs_code, deal_name, workstream.");

        prompt
    }

    /// Call OpenAI Chat Completions API
    async fn call_api(&self, prompt: String) -> Result<BlockClassificationResponse, OpenAIError> {
        // Build request payload
        let request_payload = ChatCompletionRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage {
                    role: "system".to_string(),
                    content:
                        "You are an M&A tax professional time entry classifier. Analyze work blocks and classify them as billable or G&A (non-billable) based on activity signals."
                            .to_string(),
                },
                ChatMessage { role: "user".to_string(), content: prompt },
            ],
            max_tokens: DEFAULT_MAX_TOKENS,
            temperature: DEFAULT_TEMPERATURE,
            response_format: ResponseFormat {
                format_type: "json_schema".to_string(),
                json_schema: Some(JsonSchema {
                    name: "block_classification_response".to_string(),
                    schema: json!({
                        "type": "object",
                        "properties": {
                            "classifications": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "id": { "type": "string" },
                                        "billable": { "type": "boolean" },
                                        "description": { "type": "string" },
                                        "confidence": {
                                            "type": "number",
                                            "minimum": 0.0,
                                            "maximum": 1.0
                                        },
                                        "reasons": {
                                            "type": "array",
                                            "items": { "type": "string" }
                                        },
                                        "project_id": { "type": "string" },
                                        "wbs_code": { "type": "string" },
                                        "deal_name": { "type": "string" },
                                        "workstream": { "type": "string" }
                                    },
                                    "required": ["id", "billable", "description", "confidence", "reasons"],
                                    "additionalProperties": false
                                }
                            }
                        },
                        "required": ["classifications"],
                        "additionalProperties": false
                    }),
                    strict: Some(true),
                }),
            },
        };

        // Build HTTP request
        let request_builder = self
            .http_client
            .request(Method::POST, &self.api_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request_payload);

        // Execute with retry (handled by HttpClient)
        let response = self.http_client.send(request_builder).await.map_err(|err| match err {
            PulseArcError::Network(msg) => OpenAIError::Network(msg.to_string()),
            PulseArcError::Internal(msg) => OpenAIError::Network(msg.to_string()),
            other => OpenAIError::Network(format!("HTTP error: {}", other)),
        })?;

        let status = response.status();
        debug!(status = status.as_u16(), "Received OpenAI API response");

        // Handle error status codes
        if !status.is_success() {
            return Err(self.handle_error_status(status.as_u16(), response).await);
        }

        // Parse successful response
        let chat_response: ChatCompletionResponse = response
            .json()
            .await
            .map_err(|e| OpenAIError::InvalidSchema(format!("Failed to parse response: {}", e)))?;

        // Extract and parse classifications from JSON content
        let choice = chat_response.choices.first().ok_or_else(|| {
            OpenAIError::InvalidSchema("Response contained no choices".to_string())
        })?;
        let content = &choice.message.content;
        let llm_response: LLMBlockResponse = serde_json::from_str(content).map_err(|e| {
            OpenAIError::InvalidSchema(format!(
                "Failed to parse classifications: {}. Content: {}",
                e, content
            ))
        })?;

        // Extract token usage
        let tokens_used = chat_response.usage.total_tokens;
        let prompt_tokens = chat_response.usage.prompt_tokens;
        let completion_tokens = chat_response.usage.completion_tokens;

        // Calculate cost (gpt-4o-mini pricing)
        let cost_usd = (f64::from(prompt_tokens) * COST_PER_1M_INPUT_TOKENS / 1_000_000.0)
            + (f64::from(completion_tokens) * COST_PER_1M_OUTPUT_TOKENS / 1_000_000.0);

        Ok(BlockClassificationResponse {
            classifications: llm_response.classifications,
            tokens_used,
            prompt_tokens,
            completion_tokens,
            cost_usd,
        })
    }

    /// Handle HTTP error status codes
    async fn handle_error_status(&self, status: u16, response: reqwest::Response) -> OpenAIError {
        let message = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        match status {
            401 | 403 => OpenAIError::Authentication(format!("Invalid API key ({})", status)),
            429 => {
                // Rate limit - extract retry-after header if present
                let retry_after = 60; // Default to 60s
                OpenAIError::RateLimit(retry_after)
            }
            _ => OpenAIError::Api { status, message },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use pulsearc_domain::types::classification::ActivityBreakdown;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn test_client(api_url: String) -> OpenAIClient {
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(5))
            .max_attempts(1) // No retries in tests
            .build()
            .expect("http client");

        OpenAIClient::new("test-api-key".to_string(), http_client).with_api_url(api_url)
    }

    fn sample_block() -> ProposedBlock {
        use chrono::Utc;

        ProposedBlock {
            id: "block-123".to_string(),
            start_ts: 1700000000,
            end_ts: 1700003600,
            duration_secs: 3600,
            inferred_project_id: None,
            inferred_wbs_code: None,
            inferred_deal_name: None,
            inferred_workstream: None,
            billable: false, // Will be updated by classification
            confidence: 0.0,
            classifier_used: None,
            activities: vec![ActivityBreakdown {
                name: "Chrome".to_string(),
                duration_secs: 3600,
                percentage: 100.0,
            }],
            snapshot_ids: vec!["snap-1".to_string()],
            segment_ids: vec![],
            reasons: vec![],
            status: "suggested".to_string(),
            created_at: Utc::now().timestamp(),
            reviewed_at: None,
            total_idle_secs: 0,
            idle_handling: "exclude".to_string(),
            timezone: None,
            work_location: None,
            is_travel: false,
            is_weekend: false,
            is_after_hours: false,
            has_calendar_overlap: false,
            overlapping_event_ids: vec![],
            is_double_booked: false,
        }
    }

    #[tokio::test]
    async fn classifies_blocks_successfully() {
        let mock_server = MockServer::start().await;

        // Mock successful OpenAI response
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .and(header("Authorization", "Bearer test-api-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": {
                        "content": r#"{
                            "classifications": [{
                                "id": "block-123",
                                "billable": true,
                                "description": "Client work on Confluence",
                                "confidence": 0.92,
                                "reasons": ["Confluence access"],
                                "project_id": "USC0063201"
                            }]
                        }"#
                    }
                }],
                "usage": {
                    "total_tokens": 1000,
                    "prompt_tokens": 800,
                    "completion_tokens": 200
                }
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(format!("{}/v1/chat/completions", mock_server.uri()));
        let blocks = vec![sample_block()];

        let response = client.classify_blocks(&blocks).await.expect("should classify");

        assert_eq!(response.classifications.len(), 1);
        assert_eq!(response.classifications[0].id, "block-123");
        assert!(response.classifications[0].billable);
        assert_eq!(response.classifications[0].confidence, 0.92);
        assert_eq!(response.tokens_used, 1000);
    }

    #[tokio::test]
    async fn handles_authentication_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Invalid API key"))
            .mount(&mock_server)
            .await;

        let client = test_client(format!("{}/v1/chat/completions", mock_server.uri()));
        let blocks = vec![sample_block()];

        let result = client.classify_blocks(&blocks).await;

        assert!(matches!(result, Err(OpenAIError::Authentication(_))));
    }

    #[tokio::test]
    async fn handles_rate_limit() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&mock_server)
            .await;

        let client = test_client(format!("{}/v1/chat/completions", mock_server.uri()));
        let blocks = vec![sample_block()];

        let result = client.classify_blocks(&blocks).await;

        assert!(matches!(result, Err(OpenAIError::RateLimit(_))));
    }

    #[tokio::test]
    async fn handles_invalid_response_schema() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "choices": [{
                    "message": {
                        "content": "not valid json"
                    }
                }],
                "usage": {
                    "total_tokens": 100,
                    "prompt_tokens": 80,
                    "completion_tokens": 20
                }
            })))
            .mount(&mock_server)
            .await;

        let client = test_client(format!("{}/v1/chat/completions", mock_server.uri()));
        let blocks = vec![sample_block()];

        let result = client.classify_blocks(&blocks).await;

        assert!(matches!(result, Err(OpenAIError::InvalidSchema(_))));
    }

    #[tokio::test]
    async fn returns_empty_for_empty_blocks() {
        let http_client =
            HttpClient::builder().timeout(Duration::from_secs(5)).build().expect("http client");

        let client = OpenAIClient::new("test-key".to_string(), http_client);
        let response = client.classify_blocks(&[]).await.expect("should handle empty");

        assert_eq!(response.classifications.len(), 0);
        assert_eq!(response.tokens_used, 0);
        assert_eq!(response.cost_usd, 0.0);
    }
}
