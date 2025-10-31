use std::time::Duration;

use pulsearc_domain::PulseArcError;
use reqwest::{Client as ReqwestClient, Method, RequestBuilder, Response};
use tracing::debug;

use crate::errors::InfraError;

/// HTTP client with built-in retry and timeout support.
#[derive(Clone)]
pub struct HttpClient {
    client: ReqwestClient,
    max_attempts: usize,
    base_backoff: Duration,
}

impl HttpClient {
    /// Start building a new HTTP client.
    pub fn builder() -> HttpClientBuilder {
        HttpClientBuilder::default()
    }

    /// Convenience constructor with default configuration.
    pub fn new() -> Result<Self, PulseArcError> {
        Self::builder().build()
    }

    /// Create a request builder using the underlying reqwest client.
    pub fn request<U>(&self, method: Method, url: U) -> RequestBuilder
    where
        U: reqwest::IntoUrl,
    {
        self.client.request(method, url)
    }

    /// Execute the provided request builder with retry semantics.
    pub async fn send(&self, builder: RequestBuilder) -> Result<Response, PulseArcError> {
        let attempts = self.max_attempts.max(1);

        for attempt in 0..attempts {
            let cloned_builder = builder.try_clone().ok_or_else(|| {
                PulseArcError::Internal(
                    "request body cannot be cloned; buffer the body to enable retries".into(),
                )
            })?;

            let request = cloned_builder.build().map_err(|err| {
                let infra: InfraError = err.into();
                PulseArcError::from(infra)
            })?;

            let method = request.method().clone();
            let url = request.url().clone();
            debug!(attempt = attempt + 1, %method, %url, "sending HTTP request");

            match self.client.execute(request).await {
                Ok(response) => {
                    let status = response.status();
                    debug!(attempt = attempt + 1, %method, %url, %status, "received HTTP response");

                    if status.is_server_error() && attempt + 1 < attempts {
                        self.sleep_with_backoff(attempt + 1).await;
                        continue;
                    }

                    return Ok(response);
                }
                Err(err) => {
                    debug!(attempt = attempt + 1, %method, %url, error = %err, "HTTP request failed");

                    if attempt + 1 < attempts && should_retry_error(&err) {
                        self.sleep_with_backoff(attempt + 1).await;
                        continue;
                    }

                    let infra: InfraError = err.into();
                    return Err(PulseArcError::from(infra));
                }
            }
        }

        Err(PulseArcError::Internal(
            "http client exhausted retries without producing a result".into(),
        ))
    }

    fn backoff_delay(&self, retry_number: usize) -> Duration {
        let shift = retry_number.saturating_sub(1).min(8) as u32;
        let multiplier = 1u32 << shift;
        self.base_backoff.saturating_mul(multiplier)
    }

    async fn sleep_with_backoff(&self, retry_number: usize) {
        let delay = self.backoff_delay(retry_number);
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
}

/// Builder for [`HttpClient`].
#[derive(Debug)]
pub struct HttpClientBuilder {
    timeout: Duration,
    max_attempts: usize,
    base_backoff: Duration,
    user_agent: Option<String>,
    default_headers: Option<reqwest::header::HeaderMap>,
    accept_invalid_certs: bool,
}

impl Default for HttpClientBuilder {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            max_attempts: 3,
            base_backoff: Duration::from_millis(200),
            user_agent: None,
            default_headers: None,
            accept_invalid_certs: false,
        }
    }
}

impl HttpClientBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Configure the total number of attempts (initial try + retries).
    pub fn max_attempts(mut self, attempts: usize) -> Self {
        self.max_attempts = attempts.max(1);
        self
    }

    pub fn base_backoff(mut self, backoff: Duration) -> Self {
        self.base_backoff = backoff;
        self
    }

    pub fn user_agent(mut self, agent: impl Into<String>) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    pub fn default_headers(mut self, headers: reqwest::header::HeaderMap) -> Self {
        self.default_headers = Some(headers);
        self
    }

    /// Test-only helper to allow insecure TLS (e.g., self-signed certs).
    #[cfg(test)]
    pub fn accept_invalid_certs(mut self, enabled: bool) -> Self {
        self.accept_invalid_certs = enabled;
        self
    }

    pub fn build(self) -> Result<HttpClient, PulseArcError> {
        let mut builder = ReqwestClient::builder().timeout(self.timeout).no_proxy();

        if let Some(agent) = self.user_agent {
            builder = builder.user_agent(agent);
        }

        if let Some(headers) = self.default_headers {
            builder = builder.default_headers(headers);
        }

        if self.accept_invalid_certs {
            builder = builder.danger_accept_invalid_certs(true);
        }

        let client = builder.build().map_err(|err| {
            let infra: InfraError = err.into();
            PulseArcError::from(infra)
        })?;

        Ok(HttpClient {
            client,
            max_attempts: self.max_attempts.max(1),
            base_backoff: self.base_backoff,
        })
    }
}

fn should_retry_error(err: &reqwest::Error) -> bool {
    if err.is_timeout() || err.is_request() {
        return true;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if err.is_connect() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use reqwest::{Method, StatusCode};
    use wiremock::matchers::method;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    fn client_with_defaults() -> HttpClient {
        HttpClient::builder()
            .base_backoff(Duration::from_millis(10))
            .max_attempts(3)
            .build()
            .expect("http client")
    }

    #[tokio::test]
    async fn returns_successful_response_without_retry() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_with_defaults();
        let response =
            client.send(client.request(Method::GET, server.uri())).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn retries_server_errors_until_success() {
        let server = MockServer::start().await;
        let attempts = Arc::new(AtomicUsize::new(0));
        let attempts_clone = attempts.clone();
        Mock::given(method("GET"))
            .respond_with(move |_req: &wiremock::Request| -> ResponseTemplate {
                let current = attempts_clone.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    ResponseTemplate::new(500)
                } else {
                    ResponseTemplate::new(200)
                }
            })
            .expect(3)
            .mount(&server)
            .await;

        let client = client_with_defaults();
        let response =
            client.send(client.request(Method::GET, server.uri())).await.expect("response");

        assert_eq!(response.status(), StatusCode::OK);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 3);
    }

    #[tokio::test]
    async fn does_not_retry_client_errors() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .expect(1)
            .mount(&server)
            .await;

        let client = client_with_defaults();
        let response =
            client.send(client.request(Method::GET, server.uri())).await.expect("response");

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn retries_on_network_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        drop(listener); // release the port so that requests fail with ECONNREFUSED
        let url = format!("http://{}", addr);

        let client = HttpClient::builder()
            .base_backoff(Duration::from_millis(5))
            .max_attempts(2)
            .build()
            .expect("http client");

        let result = client.send(client.request(Method::GET, &url)).await;
        match result {
            Err(PulseArcError::Network(msg)) => {
                assert!(msg.to_lowercase().contains("http"));
            }
            other => panic!("expected network error, got {:?}", other),
        }
    }
}
