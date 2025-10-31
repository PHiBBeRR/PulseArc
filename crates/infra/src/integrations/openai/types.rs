/// OpenAI API types for block classification
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Response from OpenAI block classification API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockClassificationResponse {
    /// List of block classifications returned by OpenAI
    pub classifications: Vec<BlockClassification>,
    /// Total tokens used (prompt + completion)
    pub tokens_used: i32,
    /// Tokens used in the prompt
    pub prompt_tokens: i32,
    /// Tokens used in the completion
    pub completion_tokens: i32,
    /// Estimated cost in USD (computed post-response)
    #[serde(default)]
    pub cost_usd: f64,
}

/// A single block classification result from OpenAI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockClassification {
    /// Block identifier (matches input block ID)
    pub id: String,
    /// Whether the block is billable to a client
    pub billable: bool,
    /// Classification description/justification
    pub description: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
    /// Human-readable reasons for the classification
    #[serde(default)]
    pub reasons: Vec<String>,
    /// Inferred project ID (e.g., "USC0063201")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    /// Inferred WBS code (e.g., "USC0063201.1.1")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wbs_code: Option<String>,
    /// Inferred deal/project name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deal_name: Option<String>,
    /// Inferred workstream (e.g., "modeling", "due_diligence")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workstream: Option<String>,
}

/// OpenAI API error types
#[derive(Debug, thiserror::Error)]
pub enum OpenAIError {
    /// Network-level error (connection failed, timeout, etc.)
    #[error("Network error: {0}")]
    Network(String),

    /// OpenAI API returned an error response
    #[error("API error (status {status}): {message}")]
    Api { status: u16, message: String },

    /// Rate limit exceeded - should retry after delay
    #[error("Rate limit exceeded (retry after {0}s)")]
    RateLimit(u64),

    /// Authentication failed (invalid API key)
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Response body doesn't match expected schema
    #[error("Invalid response schema: {0}")]
    InvalidSchema(String),

    /// Request timeout
    #[error("Request timeout after {0:?}")]
    Timeout(std::time::Duration),
}

/// Internal types for OpenAI Chat Completions API
#[derive(Debug, Serialize)]
pub(crate) struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub response_format: ResponseFormat,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResponseFormat {
    #[serde(rename = "type")]
    pub format_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<JsonSchema>,
}

/// JSON schema wrapper used by OpenAI when `response_format = "json_schema"`.
#[derive(Debug, Serialize)]
pub(crate) struct JsonSchema {
    pub name: String,
    pub schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// Response from OpenAI Chat Completions API
#[derive(Debug, Deserialize)]
pub(crate) struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Message {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Usage {
    pub total_tokens: i32,
    pub prompt_tokens: i32,
    pub completion_tokens: i32,
}

/// Intermediate structure for parsing OpenAI's JSON response
#[derive(Debug, Deserialize)]
pub(crate) struct LLMBlockResponse {
    pub classifications: Vec<BlockClassification>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserializes_block_classification() {
        let json = r#"{
            "id": "block-123",
            "billable": true,
            "description": "Working on client project",
            "confidence": 0.95,
            "reasons": ["VDR access detected"],
            "project_id": "USC0063201",
            "wbs_code": "USC0063201.1.1"
        }"#;

        let classification: BlockClassification =
            serde_json::from_str(json).expect("should deserialize");

        assert_eq!(classification.id, "block-123");
        assert!(classification.billable);
        assert_eq!(classification.confidence, 0.95);
        assert_eq!(classification.project_id, Some("USC0063201".to_string()));
    }

    #[test]
    fn deserializes_block_classification_with_optional_fields() {
        let json = r#"{
            "id": "block-456",
            "billable": false,
            "description": "G&A work",
            "confidence": 0.8
        }"#;

        let classification: BlockClassification =
            serde_json::from_str(json).expect("should deserialize");

        assert_eq!(classification.id, "block-456");
        assert!(!classification.billable);
        assert!(classification.reasons.is_empty());
        assert_eq!(classification.project_id, None);
        assert_eq!(classification.wbs_code, None);
    }

    #[test]
    fn deserializes_full_response() {
        let json = r#"{
            "classifications": [
                {
                    "id": "block-1",
                    "billable": true,
                    "description": "Test",
                    "confidence": 0.9,
                    "reasons": []
                }
            ],
            "tokens_used": 1000,
            "prompt_tokens": 800,
            "completion_tokens": 200
        }"#;

        let response: BlockClassificationResponse =
            serde_json::from_str(json).expect("should deserialize");

        assert_eq!(response.classifications.len(), 1);
        assert_eq!(response.tokens_used, 1000);
        assert_eq!(response.cost_usd, 0.0);
    }
}
