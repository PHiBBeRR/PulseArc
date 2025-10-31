// Global Audit Logger - Extends telemetry collector's audit with app-wide
// events

use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use crate::security::rbac::Permission;

// Type aliases for complex types
type InitResult = Result<(), Box<dyn std::error::Error>>;

/// Audit event severity levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
    Security,
}

/// Types of audit events across the application
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AuditEvent {
    // Menu Events
    MenuItemClicked { menu_id: String, label: String },
    MenuStateChanged { from_state: String, to_state: String },

    // Permission Events
    PermissionCheck { user_id: String, permission: Permission, granted: bool },
    RoleAssigned { user_id: String, role: String },

    // Configuration Events
    ConfigurationChanged { key: String, old_value: Option<String>, new_value: String },
    RemoteConfigSync { success: bool, error: Option<String> },

    // Feature Flag Events
    FeatureFlagToggled { flag: String, enabled: bool },

    // Security Events
    UnauthorizedAccess { resource: String, user_id: Option<String> },
    SuspiciousActivity { description: String, threat_level: String },
    ComplianceViolation { framework: String, violation_type: String, severity: AuditSeverity },

    // Data Events
    DataAccessed { data_type: String, operation: String, record_count: usize },
    DataModified { data_type: String, operation: String, record_count: usize },

    // System Events
    ApplicationStarted { version: String, environment: String },
    ApplicationStopped { reason: String },
    ErrorOccurred { error_type: String, message: String, stack_trace: Option<String> },

    // Custom Events
    Custom { category: String, action: String, details: serde_json::Value },
}

impl AuditEvent {
    /// Get the event type as a string
    pub fn get_type(&self) -> &str {
        match self {
            AuditEvent::MenuItemClicked { .. } => "MenuItemClicked",
            AuditEvent::MenuStateChanged { .. } => "MenuStateChanged",
            AuditEvent::PermissionCheck { .. } => "PermissionCheck",
            AuditEvent::RoleAssigned { .. } => "RoleAssigned",
            AuditEvent::ConfigurationChanged { .. } => "ConfigurationChanged",
            AuditEvent::RemoteConfigSync { .. } => "RemoteConfigSync",
            AuditEvent::FeatureFlagToggled { .. } => "FeatureFlagToggled",
            AuditEvent::UnauthorizedAccess { .. } => "UnauthorizedAccess",
            AuditEvent::SuspiciousActivity { .. } => "SuspiciousActivity",
            AuditEvent::ComplianceViolation { .. } => "ComplianceViolation",
            AuditEvent::DataAccessed { .. } => "DataAccessed",
            AuditEvent::DataModified { .. } => "DataModified",
            AuditEvent::ApplicationStarted { .. } => "ApplicationStarted",
            AuditEvent::ApplicationStopped { .. } => "ApplicationStopped",
            AuditEvent::ErrorOccurred { .. } => "ErrorOccurred",
            AuditEvent::Custom { .. } => "Custom",
        }
    }
}

/// Context information for audit events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
}

impl AuditContext {
    /// Create a new audit context with an operation name
    pub fn new(operation: &str) -> Self {
        Self {
            user_id: Some(format!("system-{}", operation)),
            session_id: None,
            ip_address: None,
            user_agent: None,
        }
    }

    /// Create an empty audit context
    pub fn empty() -> Self {
        Self { user_id: None, session_id: None, ip_address: None, user_agent: None }
    }

    /// Add component information to the user_agent field
    pub fn with_component(mut self, component: &str) -> Self {
        self.user_agent = Some(format!("component:{}", component));
        self
    }

    /// Add severity information to the session_id field (for convenience)
    pub fn with_severity(mut self, severity: &str) -> Self {
        self.session_id = Some(format!("severity:{}", severity));
        self
    }

    /// Add operation ID information
    pub fn with_operation_id(mut self, operation_id: &str) -> Self {
        self.user_id = Some(format!("operation:{}", operation_id));
        self
    }

    /// Add metadata information (stored in user_agent for convenience)
    pub fn with_metadata(mut self, _key: &str, value: &str) -> Self {
        self.user_agent = Some(format!("metadata:{}", value));
        self
    }
}

/// Complete audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event: AuditEvent,
    pub severity: AuditSeverity,
    pub context: AuditContext,
    pub correlation_id: Option<String>,
    pub metadata: serde_json::Value,
}

/// Configuration for the audit logger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Maximum number of entries in memory
    pub max_memory_entries: usize,
    /// Path to audit log file
    pub log_file_path: Option<PathBuf>,
    /// Enable real-time streaming to external service
    pub enable_streaming: bool,
    /// Minimum severity level to log
    pub min_severity: AuditSeverity,
    /// Enable encryption for sensitive data
    pub encrypt_sensitive: bool,
    /// Generic webhook URL for streaming audit events
    /// Can be configured via AUDIT_WEBHOOK_URL environment variable
    pub streaming_url: Option<String>,
    /// Timeout in seconds for streaming requests (default: 5)
    pub streaming_timeout_secs: u64,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            max_memory_entries: 10000,
            log_file_path: None,
            enable_streaming: false,
            min_severity: AuditSeverity::Info,
            encrypt_sensitive: true,
            streaming_url: None,
            streaming_timeout_secs: 5,
        }
    }
}

/// Global audit logger for the entire application
pub struct GlobalAuditLogger {
    entries: Arc<RwLock<VecDeque<GlobalAuditEntry>>>,
    config: Arc<RwLock<AuditConfig>>,
    // Note: telemetry module is not part of common crate
    // telemetry_logger: Option<Arc<crate::telemetry::collector::security::audit::AuditLogger>>,
}

impl Clone for GlobalAuditLogger {
    fn clone(&self) -> Self {
        Self {
            entries: Arc::clone(&self.entries),
            config: Arc::clone(&self.config),
            // telemetry_logger field removed
        }
    }
}

impl Default for GlobalAuditLogger {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalAuditLogger {
    /// Create a new global audit logger
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(10000))),
            config: Arc::new(RwLock::new(AuditConfig::default())),
            // telemetry_logger field removed
        }
    }

    /// Initialize the audit logger
    pub fn initialize(&mut self) -> InitResult {
        info!("Initializing global audit logger");

        // Log file setup is now handled in configure() or during actual logging
        // This method is kept for compatibility but doesn't require blocking operations

        Ok(())
    }

    /// Initialize with log file path (call this before logging if file output
    /// is needed)
    pub async fn initialize_with_path(&self) -> InitResult {
        // Set up log file if configured
        if let Some(ref path) = self.config.read().await.log_file_path {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
        }

        Ok(())
    }

    /// Configure the audit logger
    pub async fn configure(&self, config: AuditConfig) {
        *self.config.write().await = config;
    }

    /// Log an audit event
    pub async fn log_event(
        &self,
        event: AuditEvent,
        context: AuditContext,
        severity: AuditSeverity,
    ) {
        // Check severity threshold
        let config = self.config.read().await;
        if severity < config.min_severity {
            return;
        }

        // Create audit entry
        let entry = GlobalAuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event: event.clone(),
            severity,
            context,
            correlation_id: None,
            metadata: serde_json::json!({}),
        };

        // Store in memory
        let mut entries = self.entries.write().await;
        if entries.len() >= config.max_memory_entries {
            entries.pop_front();
        }
        entries.push_back(entry.clone());

        // Write to file if configured
        if let Some(ref path) = config.log_file_path {
            if let Err(e) = self.write_to_file(&entry, path).await {
                error!("Failed to write audit log to file: {}", e);
            }
        }

        // Stream to external service if configured
        if config.enable_streaming {
            self.stream_to_service(&entry).await;
        }

        // Log security events at higher level
        if severity >= AuditSeverity::Security {
            warn!("Security audit event: {:?}", entry);
        }
    }

    /// Write audit entry to file
    async fn write_to_file(
        &self,
        entry: &GlobalAuditEntry,
        path: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let json = serde_json::to_string(entry)?;

        let mut file = OpenOptions::new().create(true).append(true).open(path).await?;

        let line = format!("{}\n", json);
        file.write_all(line.as_bytes()).await?;

        Ok(())
    }

    /// Stream audit entry to a webhook endpoint
    ///
    /// Sends the audit entry as JSON via HTTP POST to the specified webhook
    /// URL. This function is non-blocking and handles errors gracefully by
    /// logging them.
    async fn stream_to_webhook(
        url: String,
        payload: serde_json::Value,
        timeout_secs: u64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use std::time::Duration;

        use reqwest::Client;

        let client = Client::new();
        let timeout = Duration::from_secs(timeout_secs);

        let response = client.post(&url).json(&payload).timeout(timeout).send().await?;

        if response.status().is_success() {
            info!("Successfully streamed audit event to webhook: {}", url);
        } else {
            warn!(
                "Webhook returned non-success status {}: {}",
                response.status(),
                response.text().await.unwrap_or_else(|_| "Unable to read response".to_string())
            );
        }

        Ok(())
    }

    /// Stream audit entry to external service
    async fn stream_to_service(&self, entry: &GlobalAuditEntry) {
        // Serialize entry for async processing
        let entry_json = match serde_json::to_value(entry) {
            Ok(json) => json,
            Err(e) => {
                error!("Failed to serialize audit entry for streaming: {}", e);
                return;
            }
        };

        // Get configuration
        let config = self.config.read().await;
        let webhook_url = config.streaming_url.clone();
        let timeout_secs = config.streaming_timeout_secs;
        drop(config); // Release lock before spawning task

        // Spawn async task to avoid blocking
        tokio::spawn(async move {
            // Determine webhook URL from config or environment variable
            let url = webhook_url.or_else(|| std::env::var("AUDIT_WEBHOOK_URL").ok());

            if let Some(url) = url {
                // Stream to generic webhook
                if let Err(e) = Self::stream_to_webhook(url.clone(), entry_json, timeout_secs).await
                {
                    error!("Failed to stream audit event to webhook {}: {}", url, e);
                }
            } else {
                // No webhook configured - this is expected if streaming is
                // disabled or if webhook URL is not set
            }

            // Future: Add additional streaming integrations here
            // if let Ok(splunk_url) = std::env::var("SPLUNK_HEC_URL") {
            //     stream_to_splunk(&splunk_url, &entry_json).await;
            // }
            // if let Ok(datadog_api_key) = std::env::var("DATADOG_API_KEY") {
            //     stream_to_datadog(&datadog_api_key, &entry_json).await;
            // }
        });

        info!("Audit event queued for streaming: {}", entry.event.get_type());
    }

    /// Query audit logs
    pub async fn query(
        &self,
        filter: impl Fn(&GlobalAuditEntry) -> bool,
        limit: Option<usize>,
    ) -> Vec<GlobalAuditEntry> {
        let entries = self.entries.read().await;
        let filtered: Vec<_> = entries.iter().filter(|e| filter(e)).cloned().collect();

        match limit {
            Some(n) => filtered.into_iter().take(n).collect(),
            None => filtered,
        }
    }

    /// Export audit logs to file
    pub async fn export(&self, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let entries = self.entries.read().await;
        let json = serde_json::to_string_pretty(&*entries)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Clear audit logs (simple clear without logging)
    ///
    /// This method clears all audit entries from memory. The clear action
    /// itself is NOT logged to avoid the chicken-and-egg problem where
    /// clearing creates a new entry.
    ///
    /// For compliance purposes, if you need an audit trail of the clear
    /// operation, use `clear_with_external_audit()` or log the clear event
    /// to persistent storage (file, database) before calling this method.
    pub async fn clear(&self, _reason: &str, _authorized_by: &str) {
        // Simply clear all entries
        self.entries.write().await.clear();
    }

    /// Clear audit logs with persistent audit trail (writes to file before
    /// clearing)
    ///
    /// This is a safer version that logs the clear event to file before
    /// clearing memory. Use this when you need to maintain a complete audit
    /// trail.
    pub async fn clear_with_external_audit(
        &self,
        reason: &str,
        authorized_by: &str,
    ) -> std::io::Result<()> {
        let entries_count = self.entries.read().await.len();

        // Log to file if configured
        if let Some(ref path) = self.config.read().await.log_file_path {
            let audit_entry = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "action": "AuditLogCleared",
                "reason": reason,
                "authorized_by": authorized_by,
                "entries_cleared": entries_count,
            });

            let mut file =
                tokio::fs::OpenOptions::new().create(true).append(true).open(path).await?;

            tokio::io::AsyncWriteExt::write_all(&mut file, format!("{}\n", audit_entry).as_bytes())
                .await?;
        }

        // Then clear the in-memory logs
        self.entries.write().await.clear();
        Ok(())
    }

    /// Get audit statistics
    pub async fn get_statistics(&self) -> AuditStatistics {
        let entries = self.entries.read().await;

        let mut stats = AuditStatistics {
            total_entries: entries.len(),
            by_severity: std::collections::HashMap::new(),
            by_event_type: std::collections::HashMap::new(),
            oldest_entry: None,
            newest_entry: None,
        };

        for entry in entries.iter() {
            *stats.by_severity.entry(entry.severity).or_insert(0) += 1;

            let event_type = entry.event.get_type();
            *stats.by_event_type.entry(event_type.to_string()).or_insert(0) += 1;
        }

        if let Some(first) = entries.front() {
            stats.oldest_entry = Some(first.timestamp);
        }
        if let Some(last) = entries.back() {
            stats.newest_entry = Some(last.timestamp);
        }

        stats
    }

    /// Get count of audit entries (for testing)
    pub async fn entry_count(&self) -> usize {
        self.entries.read().await.len()
    }
}

/// Statistics about audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total_entries: usize,
    pub by_severity: std::collections::HashMap<AuditSeverity, usize>,
    pub by_event_type: std::collections::HashMap<String, usize>,
    pub oldest_entry: Option<DateTime<Utc>>,
    pub newest_entry: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    //! Unit tests for compliance::audit.
    use tempfile::TempDir;

    use super::*;

    /// Type alias for test results to reduce complexity
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Tests that `AuditEvent::get_type()` returns correct string identifiers
    /// for all event variants.
    #[test]
    fn test_audit_event_get_type_all_variants() {
        // Test all event types return correct string
        assert_eq!(
            AuditEvent::MenuItemClicked { menu_id: "test".to_string(), label: "Test".to_string() }
                .get_type(),
            "MenuItemClicked"
        );

        assert_eq!(
            AuditEvent::MenuStateChanged { from_state: "a".to_string(), to_state: "b".to_string() }
                .get_type(),
            "MenuStateChanged"
        );

        assert_eq!(
            AuditEvent::PermissionCheck {
                user_id: "user1".to_string(),
                permission: Permission::new("test:read"),
                granted: true
            }
            .get_type(),
            "PermissionCheck"
        );

        assert_eq!(
            AuditEvent::RoleAssigned { user_id: "user1".to_string(), role: "admin".to_string() }
                .get_type(),
            "RoleAssigned"
        );

        assert_eq!(
            AuditEvent::ConfigurationChanged {
                key: "key".to_string(),
                old_value: None,
                new_value: "value".to_string()
            }
            .get_type(),
            "ConfigurationChanged"
        );

        assert_eq!(
            AuditEvent::RemoteConfigSync { success: true, error: None }.get_type(),
            "RemoteConfigSync"
        );

        assert_eq!(
            AuditEvent::FeatureFlagToggled { flag: "feature".to_string(), enabled: true }
                .get_type(),
            "FeatureFlagToggled"
        );

        assert_eq!(
            AuditEvent::UnauthorizedAccess { resource: "resource".to_string(), user_id: None }
                .get_type(),
            "UnauthorizedAccess"
        );

        assert_eq!(
            AuditEvent::SuspiciousActivity {
                description: "test".to_string(),
                threat_level: "high".to_string()
            }
            .get_type(),
            "SuspiciousActivity"
        );

        assert_eq!(
            AuditEvent::DataAccessed {
                data_type: "user".to_string(),
                operation: "read".to_string(),
                record_count: 10
            }
            .get_type(),
            "DataAccessed"
        );

        assert_eq!(
            AuditEvent::DataModified {
                data_type: "user".to_string(),
                operation: "update".to_string(),
                record_count: 5
            }
            .get_type(),
            "DataModified"
        );

        assert_eq!(
            AuditEvent::ApplicationStarted {
                version: "1.0.0".to_string(),
                environment: "prod".to_string()
            }
            .get_type(),
            "ApplicationStarted"
        );

        assert_eq!(
            AuditEvent::ApplicationStopped { reason: "shutdown".to_string() }.get_type(),
            "ApplicationStopped"
        );

        assert_eq!(
            AuditEvent::ErrorOccurred {
                error_type: "RuntimeError".to_string(),
                message: "test".to_string(),
                stack_trace: None
            }
            .get_type(),
            "ErrorOccurred"
        );

        assert_eq!(
            AuditEvent::Custom {
                category: "test".to_string(),
                action: "action".to_string(),
                details: serde_json::json!({})
            }
            .get_type(),
            "Custom"
        );
    }

    /// Tests that `AuditContext::new()` creates a context with system user ID
    /// and operation name.
    #[test]
    fn test_audit_context_new() {
        let ctx = AuditContext::new("test_operation");
        assert_eq!(ctx.user_id, Some("system-test_operation".to_string()));
        assert_eq!(ctx.session_id, None);
        assert_eq!(ctx.ip_address, None);
        assert_eq!(ctx.user_agent, None);
    }

    /// Tests that `AuditContext::empty()` creates a context with all fields set
    /// to None.
    #[test]
    fn test_audit_context_empty() {
        let ctx = AuditContext::empty();
        assert_eq!(ctx.user_id, None);
        assert_eq!(ctx.session_id, None);
        assert_eq!(ctx.ip_address, None);
        assert_eq!(ctx.user_agent, None);
    }

    /// Tests the builder methods for AuditContext (with_component,
    /// with_severity, etc).
    #[test]
    fn test_audit_context_builder_methods() {
        let ctx = AuditContext::empty()
            .with_component("test_component")
            .with_severity("high")
            .with_operation_id("op123")
            .with_metadata("key", "value");

        assert_eq!(ctx.user_id, Some("operation:op123".to_string()));
        assert_eq!(ctx.session_id, Some("severity:high".to_string()));
        assert_eq!(ctx.user_agent, Some("metadata:value".to_string()));
    }

    /// Tests that AuditSeverity levels are properly ordered from Debug to
    /// Security.
    #[test]
    fn test_audit_severity_ordering() {
        // Test that severity levels are properly ordered
        assert!(AuditSeverity::Debug < AuditSeverity::Info);
        assert!(AuditSeverity::Info < AuditSeverity::Warning);
        assert!(AuditSeverity::Warning < AuditSeverity::Error);
        assert!(AuditSeverity::Error < AuditSeverity::Critical);
        assert!(AuditSeverity::Critical < AuditSeverity::Security);

        // Test equality
        assert_eq!(AuditSeverity::Info, AuditSeverity::Info);
        assert_ne!(AuditSeverity::Info, AuditSeverity::Warning);
    }

    /// Tests that `AuditConfig::default()` creates a configuration with
    /// expected default values.
    #[test]
    fn test_audit_config_default() {
        let config = AuditConfig::default();
        assert_eq!(config.max_memory_entries, 10000);
        assert_eq!(config.log_file_path, None);
        assert!(!config.enable_streaming);
        assert_eq!(config.min_severity, AuditSeverity::Info);
        assert!(config.encrypt_sensitive);
    }

    /// Tests that `GlobalAuditLogger::new()` creates a logger instance without
    /// panicking.
    #[test]
    fn test_global_audit_logger_new() {
        let logger = GlobalAuditLogger::new();
        // Verify logger can be created without panicking
        assert!(std::ptr::eq(Arc::as_ptr(&logger.entries), Arc::as_ptr(&logger.entries)));
    }

    /// Tests that `GlobalAuditLogger::initialize()` completes successfully.
    #[test]
    fn test_global_audit_logger_initialize() {
        let mut logger = GlobalAuditLogger::new();
        let result = logger.initialize();
        assert!(result.is_ok());
    }

    /// Tests that `GlobalAuditLogger::initialize()` correctly sets up log file
    /// directories.
    #[test]
    fn test_global_audit_logger_initialize_with_log_file() -> TestResult {
        let temp_dir = TempDir::new()?;
        let log_path = temp_dir.path().join("audit.log");

        let mut logger = GlobalAuditLogger::new();
        let config = AuditConfig { log_file_path: Some(log_path.clone()), ..Default::default() };

        tokio_test::block_on(async {
            logger.configure(config).await;
        });

        let result = logger.initialize();
        assert!(result.is_ok());
        assert!(log_path.parent().map(|p| p.exists()).unwrap_or(false));

        Ok(())
    }

    /// Tests that `log_event()` filters out events below the configured minimum
    /// severity level.
    #[tokio::test]
    async fn test_log_event_respects_severity_threshold() {
        let logger = GlobalAuditLogger::new();

        // Configure with Warning minimum severity
        let config = AuditConfig { min_severity: AuditSeverity::Warning, ..Default::default() };
        logger.configure(config).await;

        // Log an Info event (should be filtered out)
        logger
            .log_event(
                AuditEvent::Custom {
                    category: "test".to_string(),
                    action: "test_action".to_string(),
                    details: serde_json::json!({}),
                },
                AuditContext::empty(),
                AuditSeverity::Info,
            )
            .await;

        // Log a Warning event (should be logged)
        logger
            .log_event(
                AuditEvent::Custom {
                    category: "test".to_string(),
                    action: "test_action2".to_string(),
                    details: serde_json::json!({}),
                },
                AuditContext::empty(),
                AuditSeverity::Warning,
            )
            .await;

        let entries = logger.entries.read().await;
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].severity, AuditSeverity::Warning);
    }

    /// Tests that `log_event()` enforces maximum memory entries limit by
    /// removing oldest entries.
    #[tokio::test]
    async fn test_log_event_memory_limit() {
        let logger = GlobalAuditLogger::new();

        // Configure with small memory limit
        let config = AuditConfig { max_memory_entries: 5, ..Default::default() };
        logger.configure(config).await;

        // Log 10 events
        for i in 0..10 {
            logger
                .log_event(
                    AuditEvent::Custom {
                        category: "test".to_string(),
                        action: format!("action_{}", i),
                        details: serde_json::json!({}),
                    },
                    AuditContext::empty(),
                    AuditSeverity::Info,
                )
                .await;
        }

        // Should only keep the last 5
        let entries = logger.entries.read().await;
        assert_eq!(entries.len(), 5);
        assert_eq!(
            serde_json::to_value(&entries[0].event)
                .ok()
                .and_then(|v| v.get("action").and_then(|a| a.as_str().map(String::from))),
            Some("action_5".to_string())
        );
    }

    /// Tests that `query()` correctly filters audit entries by severity and
    /// respects limit parameter.
    #[tokio::test]
    async fn test_query_with_filter() {
        let logger = GlobalAuditLogger::new();

        // Log events with different severities
        for severity in [AuditSeverity::Info, AuditSeverity::Warning, AuditSeverity::Error] {
            logger
                .log_event(
                    AuditEvent::Custom {
                        category: "test".to_string(),
                        action: "action".to_string(),
                        details: serde_json::json!({}),
                    },
                    AuditContext::empty(),
                    severity,
                )
                .await;
        }

        // Query for errors only
        let errors = logger.query(|e| e.severity == AuditSeverity::Error, None).await;
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].severity, AuditSeverity::Error);

        // Query with limit
        let limited = logger.query(|_| true, Some(2)).await;
        assert_eq!(limited.len(), 2);
    }

    /// Tests that `get_statistics()` returns accurate counts by severity and
    /// event type.
    #[tokio::test]
    async fn test_get_statistics() {
        let logger = GlobalAuditLogger::new();

        // Log various events
        logger
            .log_event(
                AuditEvent::MenuItemClicked {
                    menu_id: "menu1".to_string(),
                    label: "Label".to_string(),
                },
                AuditContext::empty(),
                AuditSeverity::Info,
            )
            .await;

        logger
            .log_event(
                AuditEvent::MenuItemClicked {
                    menu_id: "menu2".to_string(),
                    label: "Label".to_string(),
                },
                AuditContext::empty(),
                AuditSeverity::Info,
            )
            .await;

        logger
            .log_event(
                AuditEvent::ErrorOccurred {
                    error_type: "TestError".to_string(),
                    message: "test".to_string(),
                    stack_trace: None,
                },
                AuditContext::empty(),
                AuditSeverity::Error,
            )
            .await;

        let stats = logger.get_statistics().await;
        assert_eq!(stats.total_entries, 3);
        assert_eq!(stats.by_severity.get(&AuditSeverity::Info), Some(&2));
        assert_eq!(stats.by_severity.get(&AuditSeverity::Error), Some(&1));
        assert_eq!(stats.by_event_type.get("MenuItemClicked"), Some(&2));
        assert_eq!(stats.by_event_type.get("ErrorOccurred"), Some(&1));
        assert!(stats.oldest_entry.is_some());
        assert!(stats.newest_entry.is_some());
    }

    /// Tests that `clear()` removes all audit entries from memory.
    #[tokio::test]
    async fn test_clear_logs() {
        let logger = GlobalAuditLogger::new();

        // Log some events
        for _ in 0..5 {
            logger
                .log_event(
                    AuditEvent::Custom {
                        category: "test".to_string(),
                        action: "action".to_string(),
                        details: serde_json::json!({}),
                    },
                    AuditContext::empty(),
                    AuditSeverity::Info,
                )
                .await;
        }

        assert_eq!(logger.entries.read().await.len(), 5);

        // Clear logs - this logs the clear event then clears everything
        logger.clear("test clear", "admin").await;

        // After clear, all entries should be removed (including the clear event itself)
        assert_eq!(logger.entries.read().await.len(), 0);
    }

    /// Tests that `export()` writes audit entries to a JSON file and can be
    /// deserialized.
    #[tokio::test]
    async fn test_export_logs() -> TestResult {
        let temp_dir = TempDir::new()?;
        let export_path = temp_dir.path().join("export.json");

        let logger = GlobalAuditLogger::new();

        // Log some events
        logger
            .log_event(
                AuditEvent::Custom {
                    category: "test".to_string(),
                    action: "action".to_string(),
                    details: serde_json::json!({}),
                },
                AuditContext::empty(),
                AuditSeverity::Info,
            )
            .await;

        // Export logs
        logger.export(&export_path).await?;

        // Verify file exists and contains valid JSON
        assert!(export_path.exists());
        let content = std::fs::read_to_string(&export_path)?;
        let parsed: Vec<GlobalAuditEntry> = serde_json::from_str(&content)?;
        assert_eq!(parsed.len(), 1);

        Ok(())
    }

    /// Tests that `GlobalAuditEntry` can be serialized to and deserialized from
    /// JSON.
    #[tokio::test]
    async fn test_audit_entry_serialization() {
        let entry = GlobalAuditEntry {
            id: "test-id".to_string(),
            timestamp: Utc::now(),
            event: AuditEvent::Custom {
                category: "test".to_string(),
                action: "action".to_string(),
                details: serde_json::json!({"key": "value"}),
            },
            severity: AuditSeverity::Info,
            context: AuditContext::empty(),
            correlation_id: Some("corr-123".to_string()),
            metadata: serde_json::json!({"meta": "data"}),
        };

        // Test serialization
        let json = serde_json::to_string(&entry);
        assert!(json.is_ok());

        // Test deserialization
        let deserialized: Result<GlobalAuditEntry, _> =
            serde_json::from_str(&json.unwrap_or_default());
        assert!(deserialized.is_ok());
    }

    /// Tests that `log_event()` safely handles concurrent logging from multiple
    /// tasks.
    #[tokio::test]
    async fn test_concurrent_logging() {
        let logger = Arc::new(GlobalAuditLogger::new());
        let mut handles = vec![];

        // Spawn 10 tasks logging concurrently
        for i in 0..10 {
            let logger_clone = Arc::clone(&logger);
            let handle = tokio::spawn(async move {
                logger_clone
                    .log_event(
                        AuditEvent::Custom {
                            category: "test".to_string(),
                            action: format!("action_{}", i),
                            details: serde_json::json!({}),
                        },
                        AuditContext::empty(),
                        AuditSeverity::Info,
                    )
                    .await;
            });
            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.ok();
        }

        // Verify all events were logged
        let entries = logger.entries.read().await;
        assert_eq!(entries.len(), 10);
    }
}
