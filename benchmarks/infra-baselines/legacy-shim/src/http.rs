use reqwest::{Client, RequestBuilder, Response, StatusCode};
use std::time::Duration;
use thiserror::Error;
use tracing::{debug, warn};

/// HTTP client errors copied from the legacy implementation.
#[derive(Debug, Error)]
pub enum HttpError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Rate limit exceeded (429). Retry after {0}s")]
    RateLimit(u64),

    #[error("Client error ({status}): {message}")]
    ClientError { status: u16, message: String },

    #[error("Server error ({status}): {message}")]
    ServerError { status: u16, message: String },

    #[error("Request timeout after {0:?}")]
    Timeout(Duration),

    #[error("Max retries ({0}) exceeded")]
    MaxRetriesExceeded(u32),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),
}

/// Retry configuration (mirrors the legacy defaults).
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_backoff_ms: u64,
    pub max_backoff_ms: u64,
    pub backoff_multiplier: f64,
    pub request_timeout_secs: u64,
    pub connect_timeout_secs: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 1000,
            max_backoff_ms: 32_000,
            backoff_multiplier: 2.0,
            request_timeout_secs: 30,
            connect_timeout_secs: 10,
        }
    }
}

impl RetryConfig {
    pub fn calculate_backoff(&self, attempt: u32) -> Duration {
        let delay_ms =
            (self.initial_backoff_ms as f64 * self.backoff_multiplier.powi(attempt as i32)) as u64;
        Duration::from_millis(delay_ms.min(self.max_backoff_ms))
    }
}

/// Copy of the legacy HTTP client with retry + backoff behaviour.
pub struct HttpClient {
    client: Client,
    config: RetryConfig,
}

impl HttpClient {
    pub fn new() -> Result<Self, HttpError> {
        Self::with_config(RetryConfig::default())
    }

    pub fn with_config(config: RetryConfig) -> Result<Self, HttpError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.request_timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .no_proxy()
            .build()
            .map_err(|e| HttpError::Network(format!("Failed to build client: {}", e)))?;

        Ok(Self { client, config })
    }

    pub fn inner(&self) -> &Client {
        &self.client
    }

    pub fn config(&self) -> &RetryConfig {
        &self.config
    }

    pub async fn execute_with_retry(
        &self,
        request_builder: RequestBuilder,
    ) -> Result<Response, HttpError> {
        let mut attempt = 0;

        loop {
            let request = request_builder
                .try_clone()
                .ok_or_else(|| HttpError::Network("Request body not cloneable".to_string()))?
                .build()
                .map_err(|e| HttpError::Network(format!("Failed to build request: {}", e)))?;

            debug!(attempt = attempt, url = %request.url(), "Executing HTTP request");

            let result = self.client.execute(request).await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        debug!(status = %status, "Request succeeded");
                        return Ok(response);
                    }

                    if status == StatusCode::TOO_MANY_REQUESTS {
                        let retry_after = extract_retry_after(&response).unwrap_or(60);
                        warn!(retry_after = retry_after, "Rate limit exceeded (429)");

                        if attempt >= self.config.max_retries {
                            return Err(HttpError::RateLimit(retry_after));
                        }

                        let backoff = Duration::from_secs(retry_after);
                        debug!(backoff_secs = retry_after, "Waiting before retry");
                        tokio::time::sleep(backoff).await;
                        attempt += 1;
                        continue;
                    }

                    if status == StatusCode::UNAUTHORIZED {
                        let error_text = response.text().await.unwrap_or_default();
                        return Err(HttpError::AuthenticationError(error_text));
                    }

                    if status.is_client_error() {
                        let error_text = response.text().await.unwrap_or_default();
                        return Err(HttpError::ClientError {
                            status: status.as_u16(),
                            message: error_text,
                        });
                    }

                    if status.is_server_error() {
                        let error_text = response.text().await.unwrap_or_default();
                        warn!(status = %status, attempt = attempt, "Server error, will retry");

                        if attempt >= self.config.max_retries {
                            return Err(HttpError::ServerError {
                                status: status.as_u16(),
                                message: error_text,
                            });
                        }

                        let backoff = self.config.calculate_backoff(attempt);
                        debug!(backoff_ms = backoff.as_millis(), "Waiting before retry");
                        tokio::time::sleep(backoff).await;
                        attempt += 1;
                        continue;
                    }

                    return Ok(response);
                }
                Err(e) => {
                    warn!(error = %e, attempt = attempt, "Network error, will retry");

                    if attempt >= self.config.max_retries {
                        if e.is_timeout() {
                            return Err(HttpError::Timeout(Duration::from_secs(
                                self.config.request_timeout_secs,
                            )));
                        }
                        return Err(HttpError::Network(e.to_string()));
                    }

                    let backoff = self.config.calculate_backoff(attempt);
                    debug!(backoff_ms = backoff.as_millis(), "Waiting before retry");
                    tokio::time::sleep(backoff).await;
                    attempt += 1;
                }
            }
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default HTTP client")
    }
}

fn extract_retry_after(response: &Response) -> Option<u64> {
    response
        .headers()
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
}
