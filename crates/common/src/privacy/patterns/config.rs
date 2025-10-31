use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::core::EMAIL_PATTERN;
use super::error::PiiResult;
use super::types::{
    ComplianceFramework, ConfidenceScore, DetectionMethod, PatternConfig, PiiType,
    RedactionStrategy, SensitivityLevel,
};
use crate::error::CommonError;

/// Comprehensive PII detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiDetectionConfig {
    pub version: String,
    pub organization_id: String,
    pub enabled: bool,
    pub detection_engine: DetectionEngineConfig,
    pub pattern_configs: HashMap<PiiType, PatternConfig>,
    pub compliance_config: ComplianceConfig,
    pub performance_config: PerformanceConfig,
    pub security_config: SecurityConfig,
    pub audit_config: AuditConfig,
    pub ml_config: MachineLearningConfig,
    pub last_modified: DateTime<Utc>,
    pub modified_by: String,
}

/// Detection engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionEngineConfig {
    pub enabled_methods: Vec<DetectionMethod>,
    pub default_confidence_threshold: ConfidenceScore,
    pub enable_contextual_analysis: bool,
    pub enable_composite_detection: bool,
    pub enable_false_positive_reduction: bool,
    pub max_entity_length: usize,
    pub min_entity_length: usize,
    pub case_sensitive: bool,
    pub unicode_support: bool,
}

/// Compliance-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceConfig {
    pub enabled_frameworks: Vec<ComplianceFramework>,
    pub auto_redaction: bool,
    pub consent_required: bool,
    pub purpose_limitation: bool,
    pub data_minimization: bool,
    pub retention_policies: HashMap<ComplianceFramework, RetentionPolicy>,
    pub jurisdiction_specific_rules: HashMap<String, JurisdictionRules>,
    pub breach_notification: BreachNotificationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    pub retention_period_days: u32,
    pub auto_deletion: bool,
    pub archive_before_deletion: bool,
    pub deletion_method: DeletionMethod,
    pub audit_trail_retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeletionMethod {
    Overwrite,
    SecureWipe,
    Cryptographic,
    PhysicalDestruction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionRules {
    pub jurisdiction: String,
    pub additional_pii_types: Vec<PiiType>,
    pub stricter_consent_requirements: bool,
    pub data_localization_required: bool,
    pub cross_border_transfer_restrictions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreachNotificationConfig {
    pub enabled: bool,
    pub notification_threshold: u32,
    pub notification_timeframe_hours: u32,
    pub recipients: Vec<String>,
    pub automated_reporting: bool,
}

/// Performance optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    pub enable_caching: bool,
    pub cache_size_mb: usize,
    pub cache_ttl_seconds: u64,
    pub enable_parallel_processing: bool,
    pub worker_thread_count: usize,
    pub batch_processing: BatchProcessingConfig,
    pub memory_limits: MemoryLimitsConfig,
    pub timeout_config: TimeoutConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchProcessingConfig {
    pub enabled: bool,
    pub batch_size: usize,
    pub batch_timeout_ms: u64,
    pub max_concurrent_batches: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLimitsConfig {
    pub max_memory_usage_mb: usize,
    pub max_text_size_mb: usize,
    pub enable_memory_monitoring: bool,
    pub gc_threshold_mb: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutConfig {
    pub detection_timeout_ms: u64,
    pub redaction_timeout_ms: u64,
    pub validation_timeout_ms: u64,
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub enable_input_validation: bool,
    pub max_input_size_mb: usize,
    pub enable_rate_limiting: bool,
    pub rate_limit_per_minute: u32,
    pub enable_anomaly_detection: bool,
    pub encryption_config: EncryptionConfig,
    pub access_control: AccessControlConfig,
    pub data_loss_prevention: DlpConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub encrypt_detected_pii: bool,
    pub encryption_algorithm: String,
    pub key_rotation_days: u32,
    pub encrypt_audit_logs: bool,
    pub encrypt_cache: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlConfig {
    pub rbac_enabled: bool,
    pub api_key_required: bool,
    pub session_validation: bool,
    pub ip_whitelist: Vec<String>,
    pub user_blacklist: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DlpConfig {
    pub enabled: bool,
    pub block_sensitive_data_export: bool,
    pub watermark_documents: bool,
    pub monitor_clipboard: bool,
    pub prevent_screenshots: bool,
}

/// Audit and logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_all_detections: bool,
    pub log_false_positives: bool,
    pub log_performance_metrics: bool,
    pub log_compliance_events: bool,
    pub export_format: AuditExportFormat,
    pub retention_days: u32,
    pub real_time_alerting: AlertConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditExportFormat {
    Json,
    Csv,
    Xml,
    Syslog,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub enabled: bool,
    pub alert_on_high_sensitivity: bool,
    pub alert_on_compliance_violation: bool,
    pub alert_on_performance_degradation: bool,
    pub notification_channels: Vec<NotificationChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannel {
    pub channel_type: String,
    pub endpoint: String,
    pub severity_filter: Vec<String>,
    pub rate_limit: Option<u32>,
}

/// Machine learning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineLearningConfig {
    pub enabled: bool,
    pub model_path: Option<PathBuf>,
    pub model_update_interval_hours: u32,
    pub auto_training: bool,
    pub training_data_config: TrainingDataConfig,
    pub inference_config: InferenceConfig,
    pub model_validation: ModelValidationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingDataConfig {
    pub collect_training_data: bool,
    pub anonymize_training_data: bool,
    pub training_data_retention_days: u32,
    pub minimum_training_samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceConfig {
    pub confidence_threshold: ConfidenceScore,
    pub ensemble_voting: bool,
    pub fallback_to_regex: bool,
    pub max_inference_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelValidationConfig {
    pub validate_on_startup: bool,
    pub validation_dataset_path: Option<PathBuf>,
    pub minimum_accuracy: f64,
    pub cross_validation_folds: u32,
}

impl Default for PiiDetectionConfig {
    fn default() -> Self {
        let mut pattern_configs = HashMap::new();

        // Email pattern
        pattern_configs.insert(
            PiiType::Email,
            PatternConfig {
                pattern_type: PiiType::Email,
                regex_patterns: vec![EMAIL_PATTERN.to_string()],
                context_patterns: vec!["email".to_string(), "e-mail".to_string(), "@".to_string()],
                exclusion_patterns: vec!["noreply@".to_string(), "test@".to_string()],
                sensitivity_level: SensitivityLevel::Confidential,
                redaction_strategy: RedactionStrategy::PartialMasking,
                minimum_confidence: ConfidenceScore::new(0.8),
                enabled: true,
                compliance_frameworks: vec![ComplianceFramework::Gdpr, ComplianceFramework::Ccpa],
                custom_validators: vec!["email_domain_validator".to_string()],
            },
        );

        // Phone pattern
        pattern_configs.insert(
            PiiType::Phone,
            PatternConfig {
                pattern_type: PiiType::Phone,
                regex_patterns: vec![
                    r"\b\d{3}-\d{3}-\d{4}\b".to_string(),
                    r"\(\d{3}\)\s*\d{3}-\d{4}\b".to_string(),
                    r"\b\d{10}\b".to_string(),
                    r"\+?\d{1,3}-\d{3}-\d{3}-\d{4}\b".to_string(),
                ],
                context_patterns: vec![
                    "phone".to_string(),
                    "tel".to_string(),
                    "mobile".to_string(),
                ],
                exclusion_patterns: vec!["000-000-0000".to_string()],
                sensitivity_level: SensitivityLevel::Confidential,
                redaction_strategy: RedactionStrategy::PartialMasking,
                minimum_confidence: ConfidenceScore::new(0.7),
                enabled: true,
                compliance_frameworks: vec![ComplianceFramework::Gdpr],
                custom_validators: vec![],
            },
        );

        // SSN pattern
        pattern_configs.insert(
            PiiType::Ssn,
            PatternConfig {
                pattern_type: PiiType::Ssn,
                regex_patterns: vec![
                    r"\b\d{3}-\d{2}-\d{4}\b".to_string(),
                    r"\b\d{9}\b".to_string(),
                ],
                context_patterns: vec!["ssn".to_string(), "social security".to_string()],
                exclusion_patterns: vec!["000-00-0000".to_string()],
                sensitivity_level: SensitivityLevel::Restricted,
                redaction_strategy: RedactionStrategy::FullMasking,
                minimum_confidence: ConfidenceScore::new(0.9),
                enabled: true,
                compliance_frameworks: vec![ComplianceFramework::Gdpr, ComplianceFramework::Ccpa],
                custom_validators: vec!["ssn_checksum_validator".to_string()],
            },
        );

        // IP address pattern
        pattern_configs.insert(
            PiiType::IpAddress,
            PatternConfig {
                pattern_type: PiiType::IpAddress,
                regex_patterns: vec![r"\b(?:\d{1,3}\.){3}\d{1,3}\b".to_string()],
                context_patterns: vec![
                    "ip".to_string(),
                    "address".to_string(),
                    "host".to_string(),
                    "server".to_string(),
                ],
                exclusion_patterns: vec!["0.0.0.0".to_string()],
                sensitivity_level: SensitivityLevel::Internal,
                redaction_strategy: RedactionStrategy::PartialMasking,
                minimum_confidence: ConfidenceScore::new(0.8),
                enabled: true,
                compliance_frameworks: vec![ComplianceFramework::Gdpr],
                custom_validators: vec![],
            },
        );

        // Credit Card pattern
        pattern_configs.insert(
            PiiType::CreditCard,
            PatternConfig {
                pattern_type: PiiType::CreditCard,
                regex_patterns: vec![
                    r"\b4\d{3}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b".to_string(), // Visa
                    r"\b5[1-5]\d{2}[\s-]?\d{4}[\s-]?\d{4}[\s-]?\d{4}\b".to_string(), // MasterCard
                    r"\b3[47]\d{2}[\s-]?\d{6}[\s-]?\d{5}\b".to_string(),        // AmEx
                ],
                context_patterns: vec![
                    "card".to_string(),
                    "credit".to_string(),
                    "payment".to_string(),
                ],
                exclusion_patterns: vec!["0000000000000000".to_string()],
                sensitivity_level: SensitivityLevel::Restricted,
                redaction_strategy: RedactionStrategy::PartialMasking,
                minimum_confidence: ConfidenceScore::new(0.9),
                enabled: true,
                compliance_frameworks: vec![ComplianceFramework::Pci],
                custom_validators: vec!["luhn_checksum_validator".to_string()],
            },
        );

        Self {
            version: "2.0.0".to_string(),
            organization_id: "default".to_string(),
            enabled: true,
            detection_engine: DetectionEngineConfig::default(),
            pattern_configs,
            compliance_config: ComplianceConfig::default(),
            performance_config: PerformanceConfig::default(),
            security_config: SecurityConfig::default(),
            audit_config: AuditConfig::default(),
            ml_config: MachineLearningConfig::default(),
            last_modified: Utc::now(),
            modified_by: "system".to_string(),
        }
    }
}

impl Default for DetectionEngineConfig {
    fn default() -> Self {
        Self {
            enabled_methods: vec![
                DetectionMethod::Regex,
                DetectionMethod::ContextualAnalysis,
                DetectionMethod::ChecksumValidation,
            ],
            default_confidence_threshold: ConfidenceScore::new(0.7),
            enable_contextual_analysis: true,
            enable_composite_detection: true,
            enable_false_positive_reduction: true,
            max_entity_length: 1000,
            min_entity_length: 3,
            case_sensitive: false,
            unicode_support: true,
        }
    }
}

impl Default for ComplianceConfig {
    fn default() -> Self {
        let mut retention_policies = HashMap::new();
        retention_policies.insert(
            ComplianceFramework::Gdpr,
            RetentionPolicy {
                retention_period_days: 30,
                auto_deletion: true,
                archive_before_deletion: true,
                deletion_method: DeletionMethod::SecureWipe,
                audit_trail_retention_days: 90,
            },
        );

        Self {
            enabled_frameworks: vec![ComplianceFramework::Gdpr],
            auto_redaction: true,
            consent_required: true,
            purpose_limitation: true,
            data_minimization: true,
            retention_policies,
            jurisdiction_specific_rules: HashMap::new(),
            breach_notification: BreachNotificationConfig {
                enabled: true,
                notification_threshold: 100,
                notification_timeframe_hours: 72,
                recipients: vec!["dpo@company.com".to_string()],
                automated_reporting: false,
            },
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_size_mb: 128,
            cache_ttl_seconds: 3600,
            enable_parallel_processing: true,
            worker_thread_count: 4,
            batch_processing: BatchProcessingConfig {
                enabled: true,
                batch_size: 100,
                batch_timeout_ms: 5000,
                max_concurrent_batches: 10,
            },
            memory_limits: MemoryLimitsConfig {
                max_memory_usage_mb: 512,
                max_text_size_mb: 50,
                enable_memory_monitoring: true,
                gc_threshold_mb: 256,
            },
            timeout_config: TimeoutConfig {
                detection_timeout_ms: 30000,
                redaction_timeout_ms: 10000,
                validation_timeout_ms: 5000,
            },
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_input_validation: true,
            max_input_size_mb: 10,
            enable_rate_limiting: true,
            rate_limit_per_minute: 1000,
            enable_anomaly_detection: true,
            encryption_config: EncryptionConfig {
                encrypt_detected_pii: true,
                encryption_algorithm: "AES-256-GCM".to_string(),
                key_rotation_days: 30,
                encrypt_audit_logs: true,
                encrypt_cache: false,
            },
            access_control: AccessControlConfig {
                rbac_enabled: true,
                api_key_required: false,
                session_validation: true,
                ip_whitelist: vec![],
                user_blacklist: vec![],
            },
            data_loss_prevention: DlpConfig {
                enabled: false,
                block_sensitive_data_export: false,
                watermark_documents: false,
                monitor_clipboard: false,
                prevent_screenshots: false,
            },
        }
    }
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_all_detections: true,
            log_false_positives: true,
            log_performance_metrics: true,
            log_compliance_events: true,
            export_format: AuditExportFormat::Json,
            retention_days: 90,
            real_time_alerting: AlertConfig {
                enabled: true,
                alert_on_high_sensitivity: true,
                alert_on_compliance_violation: true,
                alert_on_performance_degradation: false,
                notification_channels: vec![],
            },
        }
    }
}

impl Default for MachineLearningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            model_path: None,
            model_update_interval_hours: 24,
            auto_training: false,
            training_data_config: TrainingDataConfig {
                collect_training_data: false,
                anonymize_training_data: true,
                training_data_retention_days: 30,
                minimum_training_samples: 1000,
            },
            inference_config: InferenceConfig {
                confidence_threshold: ConfidenceScore::new(0.8),
                ensemble_voting: false,
                fallback_to_regex: true,
                max_inference_time_ms: 1000,
            },
            model_validation: ModelValidationConfig {
                validate_on_startup: true,
                validation_dataset_path: None,
                minimum_accuracy: 0.9,
                cross_validation_folds: 5,
            },
        }
    }
}

impl PiiDetectionConfig {
    /// Validate the configuration
    pub fn validate(&self) -> PiiResult<()> {
        if self.version.is_empty() {
            return Err(CommonError::config_field("version", "Version cannot be empty").into());
        }

        if self.organization_id.is_empty() {
            return Err(CommonError::config_field(
                "organization_id",
                "Organization ID cannot be empty",
            )
            .into());
        }

        // Validate pattern configurations
        for (pii_type, pattern_config) in &self.pattern_configs {
            if pattern_config.regex_patterns.is_empty() {
                return Err(CommonError::config(format!(
                    "Pattern config for {:?} must have at least one regex pattern",
                    pii_type
                ))
                .into());
            }

            // Validate regex patterns
            for pattern in &pattern_config.regex_patterns {
                regex::Regex::new(pattern).map_err(|e| {
                    CommonError::config(format!("Invalid regex pattern '{}': {}", pattern, e))
                })?;
            }
        }

        // Validate performance limits
        if self.performance_config.memory_limits.max_memory_usage_mb == 0 {
            return Err(CommonError::config_field(
                "max_memory_usage_mb",
                "Memory limit cannot be zero",
            )
            .into());
        }

        Ok(())
    }

    /// Load configuration from file
    pub async fn load_from_file<P: AsRef<Path>>(path: P) -> PiiResult<Self> {
        let content = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| CommonError::persistence_op("read_config", e.to_string()))?;

        let config: Self = serde_json::from_str(&content)?; // Auto-converts via impl_error_conversion

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> PiiResult<()> {
        self.validate()?;

        let content = serde_json::to_string_pretty(self)?; // Auto-converts via impl_error_conversion

        tokio::fs::write(path, content)
            .await
            .map_err(|e| CommonError::persistence_op("write_config", e.to_string()))?;

        Ok(())
    }

    /// Get enabled PII types
    pub fn get_enabled_pii_types(&self) -> Vec<PiiType> {
        self.pattern_configs
            .iter()
            .filter(|(_, config)| config.enabled)
            .map(|(pii_type, _)| pii_type.clone())
            .collect()
    }

    /// Get compliance frameworks
    pub fn get_compliance_frameworks(&self) -> Vec<ComplianceFramework> {
        self.compliance_config.enabled_frameworks.clone()
    }

    /// Update last modified timestamp
    pub fn touch(&mut self, modified_by: String) {
        self.last_modified = Utc::now();
        self.modified_by = modified_by;
    }

    /// Add custom pattern
    pub fn add_custom_pattern(&mut self, pii_type: PiiType, pattern_config: PatternConfig) {
        self.pattern_configs.insert(pii_type, pattern_config);
        self.touch("api".to_string());
    }

    /// Remove pattern
    pub fn remove_pattern(&mut self, pii_type: &PiiType) {
        self.pattern_configs.remove(pii_type);
        self.touch("api".to_string());
    }

    /// Enable/disable specific PII type detection
    pub fn set_pii_type_enabled(&mut self, pii_type: &PiiType, enabled: bool) {
        if let Some(config) = self.pattern_configs.get_mut(pii_type) {
            config.enabled = enabled;
            self.touch("api".to_string());
        }
    }
}
