//! MDM Remote Configuration Client
//!
//! Fetches MDM configuration from remote servers over HTTPS.

use std::path::Path;
use std::time::Duration;

use reqwest::Certificate;

use super::{MdmConfig, MdmError, MdmResult};

/// Client for fetching remote MDM configuration
pub struct MdmClient {
    client: reqwest::Client,
    config_url: String,
    timeout: Duration,
}

impl MdmClient {
    /// Create a new MDM client with default settings
    ///
    /// # Arguments
    /// * `config_url` - The URL to fetch MDM configuration from
    ///
    /// # Errors
    /// Returns `MdmError::InvalidUrl` if the URL is malformed
    pub fn new(config_url: impl Into<String>) -> MdmResult<Self> {
        let config_url = config_url.into();

        // Validate URL
        url::Url::parse(&config_url).map_err(|_| MdmError::InvalidUrl(config_url.clone()))?;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .no_proxy() // avoid macOS dynamic store panics during tests
            .build()
            .map_err(|e| {
                MdmError::ConfigurationError(format!("Failed to build HTTP client: {}", e))
            })?;

        Ok(Self { client, config_url, timeout: Duration::from_secs(30) })
    }

    /// Create a new MDM client with custom CA certificate
    ///
    /// Use this for testing with self-signed certificates or custom PKI.
    ///
    /// # Arguments
    /// * `config_url` - The URL to fetch MDM configuration from
    /// * `ca_cert_path` - Path to the CA certificate PEM file
    ///
    /// # Example
    /// ```no_run
    /// use pulsearc_infra::mdm::MdmClient;
    ///
    /// let client =
    ///     MdmClient::with_ca_cert("https://mdm.example.com/config", ".mdm-certs/ca-cert.pem")
    ///         .unwrap();
    /// ```
    pub fn with_ca_cert(
        config_url: impl Into<String>,
        ca_cert_path: impl AsRef<Path>,
    ) -> MdmResult<Self> {
        let config_url = config_url.into();

        // Validate URL
        url::Url::parse(&config_url).map_err(|_| MdmError::InvalidUrl(config_url.clone()))?;

        // Load CA certificate
        let ca_cert_bytes = std::fs::read(ca_cert_path.as_ref()).map_err(|e| {
            MdmError::ConfigurationError(format!("Failed to read CA certificate: {}", e))
        })?;

        let ca_cert = Certificate::from_pem(&ca_cert_bytes)
            .map_err(|e| MdmError::ConfigurationError(format!("Invalid CA certificate: {}", e)))?;

        // Build client with custom CA
        let client = reqwest::Client::builder()
            .add_root_certificate(ca_cert)
            .timeout(Duration::from_secs(30))
            .no_proxy()
            .build()
            .map_err(|e| {
                MdmError::ConfigurationError(format!("Failed to build HTTP client: {}", e))
            })?;

        Ok(Self { client, config_url, timeout: Duration::from_secs(30) })
    }

    /// Create a new MDM client for testing (disables certificate validation)
    ///
    /// ⚠️ **WARNING:** This is for testing only! Do not use in production.
    ///
    /// # Arguments
    /// * `config_url` - The URL to fetch MDM configuration from
    ///
    /// # Example
    /// ```no_run
    /// use pulsearc_infra::mdm::MdmClient;
    ///
    /// #[cfg(test)]
    /// let client = MdmClient::with_insecure_tls("https://localhost:8080/config").unwrap();
    /// ```
    #[cfg(test)]
    pub fn with_insecure_tls(config_url: impl Into<String>) -> MdmResult<Self> {
        let config_url = config_url.into();

        // Validate URL
        url::Url::parse(&config_url).map_err(|_| MdmError::InvalidUrl(config_url.clone()))?;

        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(30))
            .no_proxy()
            .build()
            .map_err(|e| {
                MdmError::ConfigurationError(format!("Failed to build HTTP client: {}", e))
            })?;

        Ok(Self { client, config_url, timeout: Duration::from_secs(30) })
    }

    /// Set custom timeout for HTTP requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Fetch MDM configuration from the remote server
    ///
    /// # Errors
    /// Returns `MdmError::ConfigurationError` if:
    /// - Network request fails
    /// - Response is not valid JSON
    /// - Deserialization fails
    /// - Configuration validation fails
    pub async fn fetch_config(&self) -> MdmResult<MdmConfig> {
        tracing::info!(url = %self.config_url, "Fetching MDM configuration");

        let response =
            self.client.get(&self.config_url).timeout(self.timeout).send().await.map_err(|e| {
                MdmError::ConfigurationError(format!("Failed to fetch configuration: {}", e))
            })?;

        // Check HTTP status
        if !response.status().is_success() {
            return Err(MdmError::ConfigurationError(format!(
                "Server returned error: {}",
                response.status()
            )));
        }

        // Parse JSON response
        let config: MdmConfig = response.json().await.map_err(|e| {
            MdmError::ConfigurationError(format!("Failed to parse configuration: {}", e))
        })?;

        // Validate configuration
        config.validate()?;

        tracing::info!("MDM configuration fetched and validated successfully");
        Ok(config)
    }

    /// Fetch and merge remote configuration with local config
    ///
    /// # Arguments
    /// * `local_config` - The local configuration to merge with
    ///
    /// # Returns
    /// The merged configuration
    pub async fn fetch_and_merge(&self, mut local_config: MdmConfig) -> MdmResult<MdmConfig> {
        let remote_config = self.fetch_config().await?;
        local_config.merge_remote(remote_config)?;
        Ok(local_config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdm_client_new_valid_url() {
        let client = MdmClient::new("https://example.com/config");
        assert!(client.is_ok());
    }

    #[test]
    fn test_mdm_client_new_invalid_url() {
        let client = MdmClient::new("not-a-valid-url");
        assert!(client.is_err());
    }

    #[test]
    fn test_mdm_client_with_timeout() {
        let client = MdmClient::new("https://example.com/config")
            .unwrap()
            .with_timeout(Duration::from_secs(60));

        assert_eq!(client.timeout, Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_mdm_client_with_insecure_tls() {
        let client = MdmClient::with_insecure_tls("https://localhost:8080/config");
        assert!(client.is_ok());
    }
}
