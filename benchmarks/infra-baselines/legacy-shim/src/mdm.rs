use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Certificate;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

pub type MdmResult<T> = Result<T, MdmError>;

#[derive(Debug, Clone, Error)]
pub enum MdmError {
    #[error("Invalid MDM configuration URL: {0}")]
    InvalidUrl(String),
    #[error("Policy violation: {0}")]
    PolicyViolation(String),
    #[error("Compliance check '{rule}' failed: {reason}")]
    ComplianceCheckFailed { rule: String, reason: String },
    #[error("MDM configuration error: {0}")]
    ConfigurationError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdmConfig {
    pub policy_enforcement: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_config_url: Option<String>,
    #[serde(default)]
    pub compliance_checks: Vec<ComplianceRule>,
    #[serde(default)]
    pub policies: HashMap<String, PolicySetting>,
    #[serde(default = "default_update_interval")]
    pub update_interval_secs: u64,
    #[serde(default)]
    pub allow_local_override: bool,
}

fn default_update_interval() -> u64 {
    3600
}

impl Default for MdmConfig {
    fn default() -> Self {
        Self {
            policy_enforcement: false,
            remote_config_url: None,
            compliance_checks: Vec::new(),
            policies: HashMap::new(),
            update_interval_secs: default_update_interval(),
            allow_local_override: false,
        }
    }
}

impl MdmConfig {
    pub fn builder() -> MdmConfigBuilder {
        MdmConfigBuilder::new()
    }

    pub fn validate(&self) -> MdmResult<()> {
        if let Some(url) = &self.remote_config_url {
            Url::parse(url).map_err(|_| MdmError::InvalidUrl(url.clone()))?;
        }

        for rule in &self.compliance_checks {
            rule.validate()?;
        }

        for (name, policy) in &self.policies {
            policy
                .validate()
                .map_err(|e| MdmError::ValidationError(format!("Policy '{}': {}", name, e)))?;
        }

        Ok(())
    }

    pub fn merge_remote(&mut self, remote: MdmConfig) -> MdmResult<()> {
        if !self.allow_local_override {
            *self = remote;
        } else {
            self.policy_enforcement = remote.policy_enforcement;
            if remote.remote_config_url.is_some() {
                self.remote_config_url = remote.remote_config_url;
            }
            for remote_rule in remote.compliance_checks {
                if !self.compliance_checks.iter().any(|r| r.name == remote_rule.name) {
                    self.compliance_checks.push(remote_rule);
                }
            }
            self.policies.extend(remote.policies);
        }

        self.validate()?;
        Ok(())
    }
}

pub struct MdmConfigBuilder {
    config: MdmConfig,
}

impl MdmConfigBuilder {
    pub fn new() -> Self {
        Self { config: MdmConfig::default() }
    }

    pub fn policy_enforcement(mut self, enabled: bool) -> Self {
        self.config.policy_enforcement = enabled;
        self
    }

    pub fn remote_config_url(mut self, url: impl Into<String>) -> Self {
        self.config.remote_config_url = Some(url.into());
        self
    }

    pub fn add_policy(mut self, name: impl Into<String>, policy: PolicySetting) -> Self {
        self.config.policies.insert(name.into(), policy);
        self
    }

    pub fn update_interval_secs(mut self, secs: u64) -> Self {
        self.config.update_interval_secs = secs;
        self
    }

    pub fn allow_local_override(mut self, allow: bool) -> Self {
        self.config.allow_local_override = allow;
        self
    }

    pub fn build(self) -> MdmResult<MdmConfig> {
        self.config.validate()?;
        Ok(self.config)
    }
}

impl Default for MdmConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceRule {
    pub name: String,
    pub required: bool,
    pub validation_type: ValidationType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub severity: ComplianceSeverity,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub criteria: Option<HashMap<String, String>>,
}

impl ComplianceRule {
    pub fn validate(&self) -> MdmResult<()> {
        if self.name.is_empty() {
            return Err(MdmError::ValidationError("Rule name cannot be empty".into()));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ValidationType {
    FieldExists(String),
    FieldEquals { field: String, value: String },
    FieldMatches { field: String, pattern: String },
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ComplianceSeverity {
    #[default]
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PolicySetting {
    pub enabled: bool,
    pub value: PolicyValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub enforced: bool,
}

impl PolicySetting {
    pub fn new(value: PolicyValue) -> Self {
        Self { enabled: true, value, description: None, enforced: false }
    }

    pub fn validate(&self) -> Result<(), String> {
        match &self.value {
            PolicyValue::String(s) if s.is_empty() => {
                Err("String policy value cannot be empty".into())
            }
            PolicyValue::Number(n) if n.is_nan() => Err("Number policy value cannot be NaN".into()),
            PolicyValue::List(l) if self.enforced && l.is_empty() => {
                Err("Enforced list policy cannot be empty".into())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum PolicyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<String>),
    Object(HashMap<String, String>),
}

pub struct MdmClientBuilder {
    config_url: String,
    ca_cert_path: Option<PathBuf>,
    timeout: Duration,
    no_pool: bool,
    fresh_tls_config: bool,
}

impl MdmClientBuilder {
    pub fn ca_cert_path(mut self, path: impl AsRef<Path>) -> Self {
        self.ca_cert_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn no_pool(mut self) -> Self {
        self.no_pool = true;
        self
    }

    pub fn fresh_tls_config(mut self) -> Self {
        self.fresh_tls_config = true;
        self
    }

    pub fn build(self) -> MdmResult<MdmClient> {
        Url::parse(&self.config_url).map_err(|_| MdmError::InvalidUrl(self.config_url.clone()))?;

        // Ensure the aws-lc crypto provider is installed; ignore errors if it was
        // already set.
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

        let ca_bytes = match &self.ca_cert_path {
            Some(path) => Some(std::fs::read(path).map_err(|e| {
                MdmError::ConfigurationError(format!("Failed to read CA certificate: {e}"))
            })?),
            None => None,
        };

        let mut builder = reqwest::Client::builder().timeout(self.timeout).no_proxy();

        if self.no_pool {
            builder = builder.pool_max_idle_per_host(0).pool_idle_timeout(None);
        }

        if self.fresh_tls_config {
            use rustls::client::Resumption;
            use rustls::{ClientConfig, RootCertStore};

            let mut root_store = RootCertStore::empty();

            let ca_bytes = ca_bytes.as_ref().ok_or_else(|| {
                MdmError::ConfigurationError(
                    "CA certificate path required when using fresh TLS configuration".into(),
                )
            })?;

            let mut reader = Cursor::new(ca_bytes.as_slice());
            let certs =
                rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>().map_err(|e| {
                    MdmError::ConfigurationError(format!("Invalid CA certificate: {e}"))
                })?;

            if certs.is_empty() {
                return Err(MdmError::ConfigurationError(
                    "CA certificate bundle did not contain any certificates".into(),
                ));
            }

            let (added, ignored) = root_store.add_parsable_certificates(certs.iter().cloned());
            if added == 0 {
                return Err(MdmError::ConfigurationError(format!(
                    "CA certificate bundle did not contain any parsable certificates ({} ignored)",
                    ignored
                )));
            }

            let root_store = Arc::new(root_store);
            let mut tls =
                ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth();
            tls.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];
            tls.resumption = Resumption::disabled();

            builder = builder.use_preconfigured_tls(tls);
        } else if let Some(bytes) = ca_bytes.as_ref() {
            let cert = Certificate::from_pem(bytes).map_err(|e| {
                MdmError::ConfigurationError(format!("Invalid CA certificate: {e}"))
            })?;
            builder = builder.add_root_certificate(cert);
        }

        let client = builder.build().map_err(|e| {
            MdmError::ConfigurationError(format!("Failed to build HTTP client: {e}"))
        })?;

        Ok(MdmClient { client, config_url: self.config_url, timeout: self.timeout })
    }
}

pub struct MdmClient {
    client: reqwest::Client,
    config_url: String,
    timeout: Duration,
}

impl MdmClient {
    pub fn new(config_url: impl Into<String>) -> MdmResult<Self> {
        Self::builder(config_url).build()
    }

    pub fn with_ca_cert(
        config_url: impl Into<String>,
        ca_cert_path: impl AsRef<Path>,
    ) -> MdmResult<Self> {
        Self::builder(config_url).ca_cert_path(ca_cert_path).build()
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub async fn fetch_config(&self) -> MdmResult<MdmConfig> {
        let response =
            self.client.get(&self.config_url).timeout(self.timeout).send().await.map_err(|e| {
                MdmError::ConfigurationError(format!("Failed to fetch configuration: {e}"))
            })?;

        if !response.status().is_success() {
            return Err(MdmError::ConfigurationError(format!(
                "Server returned error: {}",
                response.status()
            )));
        }

        let config: MdmConfig = response.json().await.map_err(|e| {
            MdmError::ConfigurationError(format!("Failed to parse configuration: {e}"))
        })?;

        config.validate()?;
        Ok(config)
    }

    pub async fn fetch_and_merge(&self, mut local_config: MdmConfig) -> MdmResult<MdmConfig> {
        let remote_config = self.fetch_config().await?;
        local_config.merge_remote(remote_config)?;
        Ok(local_config)
    }

    pub fn builder(config_url: impl Into<String>) -> MdmClientBuilder {
        MdmClientBuilder {
            config_url: config_url.into(),
            ca_cert_path: None,
            timeout: Duration::from_secs(30),
            no_pool: false,
            fresh_tls_config: false,
        }
    }
}
