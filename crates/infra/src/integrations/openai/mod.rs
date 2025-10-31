/// OpenAI integration for block classification
///
/// This module provides an OpenAI API client for classifying time blocks as billable
/// or G&A (non-billable) based on activity signals.
///
/// # Architecture
///
/// - **Client**: `OpenAIClient` - HTTP client wrapper for OpenAI Chat Completions API
/// - **Types**: Request/response types for block classification
/// - **Error Handling**: Structured error types with retry support
/// # Usage
///
/// ```no_run
/// use pulsearc_infra::http::HttpClient;
/// use pulsearc_infra::integrations::openai::OpenAIClient;
/// use pulsearc_domain::types::classification::ProposedBlock;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create HTTP client with retry logic
/// let http_client = HttpClient::new()?;
///
/// // Create OpenAI client
/// let api_key = std::env::var("OPENAI_API_KEY")?;
/// let client = OpenAIClient::new(api_key, http_client);
///
/// // Classify blocks
/// let blocks: Vec<ProposedBlock> = vec![/* ... */];
/// let response = client.classify_blocks(&blocks).await?;
///
/// println!("Classified {} blocks", response.classifications.len());
/// println!("Cost: ${:.4}", response.cost_usd);
/// # Ok(())
/// # }
/// ```
///
/// # API Integration
///
/// Uses OpenAI's Chat Completions API with:
/// - Model: `gpt-4o-mini` (configurable via `with_model()`)
/// - Temperature: 0.3 (low variability for consistent classifications)
/// - Response format: JSON object
/// - Max tokens: 50,000
///
/// # Error Handling
///
/// - **Network errors**: Automatically retried by `HttpClient`
/// - **Server errors (5xx)**: Retried with exponential backoff
/// - **Client errors (4xx)**: Not retried (except 429 rate limits)
/// - **Rate limits (429)**: Should be retried after delay
/// # Cost Tracking
///
/// Token usage and costs are included in responses:
/// - `tokens_used`: Total tokens (prompt + completion)
/// - `cost_usd`: Estimated cost based on gpt-4o-mini pricing
///
/// Current pricing (as of 2025):
/// - Input: $0.150 per 1M tokens
/// - Output: $0.600 per 1M tokens
pub mod client;
pub mod types;

pub use client::OpenAIClient;
pub use types::{BlockClassification, BlockClassificationResponse, OpenAIError};
