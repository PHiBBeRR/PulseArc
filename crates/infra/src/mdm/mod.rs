// MDM Integration - Enterprise policy management

use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use url::Url;

pub mod client;

pub use client::MdmClient;

/// Result type for MDM operations
pub type MdmResult<T> = Result<T, MdmError>;

/// MDM-specific errors
#[derive(Debug, Clone)]
pub enum MdmError {
    InvalidUrl(String),
    PolicyViolation(String),
    ComplianceCheckFailed { rule: String, reason: String },
    ConfigurationError(String),
    ValidationError(String),
}

impl fmt::Display for MdmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(url) => write!(f, "Invalid MDM configuration URL: {}", url),
            Self::PolicyViolation(policy) => write!(f, "Policy violation: {}", policy),
            Self::ComplianceCheckFailed { rule, reason } => {
                write!(f, "Compliance check '{}' failed: {}", rule, reason)
            }
            Self::ConfigurationError(msg) => write!(f, "MDM configuration error: {}", msg),
            Self::ValidationError(msg) => write!(f, "Validation error: {}", msg),
        }
    }
}

impl std::error::Error for MdmError {}

/// Main MDM configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MdmConfig {
    /// Whether policy enforcement is enabled
    pub policy_enforcement: bool,

    /// URL for fetching remote configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_config_url: Option<String>,

    /// List of compliance rules to enforce
    #[serde(default)]
    pub compliance_checks: Vec<ComplianceRule>,

    /// Policy settings mapped by policy name
    #[serde(default)]
    pub policies: HashMap<String, PolicySetting>,

    /// Update interval in seconds for remote config
    #[serde(default = "default_update_interval")]
    pub update_interval_secs: u64,

    /// Whether to allow local overrides
    #[serde(default)]
    pub allow_local_override: bool,
}

impl MdmConfig {
    /// Create a new MDM configuration with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a builder for MDM configuration
    pub fn builder() -> MdmConfigBuilder {
        MdmConfigBuilder::new()
    }

    /// Validate the MDM configuration
    pub fn validate(&self) -> MdmResult<()> {
        // Validate remote config URL if present
        if let Some(url) = &self.remote_config_url {
            Url::parse(url).map_err(|_| MdmError::InvalidUrl(url.clone()))?;
        }

        // Validate compliance rules
        for rule in &self.compliance_checks {
            rule.validate()?;
        }

        // Validate policies
        for (name, policy) in &self.policies {
            policy
                .validate()
                .map_err(|e| MdmError::ValidationError(format!("Policy '{}': {}", name, e)))?;
        }

        Ok(())
    }

    /// Check if a specific policy is enabled
    pub fn is_policy_enabled(&self, policy_name: &str) -> bool {
        self.policies.get(policy_name).map(|p| p.enabled).unwrap_or(false)
    }

    /// Get a policy setting value
    pub fn get_policy_value(&self, policy_name: &str) -> Option<&PolicyValue> {
        self.policies.get(policy_name).map(|p| &p.value)
    }

    /// Apply a compliance check
    #[cfg(any(feature = "audit-compliance", test))]
    pub fn check_compliance(&self, context: &ComplianceContext) -> MdmResult<ComplianceReport> {
        let mut report = ComplianceReport::new();

        for rule in &self.compliance_checks {
            let result = rule.check(context)?;
            report.add_result(rule.name.clone(), result);
        }

        Ok(report)
    }

    /// Merge with remote configuration
    pub fn merge_remote(&mut self, remote: MdmConfig) -> MdmResult<()> {
        if !self.allow_local_override {
            // Remote config completely replaces local
            *self = remote;
        } else {
            // Merge remote with local, remote takes precedence for conflicts
            self.policy_enforcement = remote.policy_enforcement;

            if remote.remote_config_url.is_some() {
                self.remote_config_url = remote.remote_config_url;
            }

            // Merge compliance checks
            for remote_rule in remote.compliance_checks {
                if !self.compliance_checks.iter().any(|r| r.name == remote_rule.name) {
                    self.compliance_checks.push(remote_rule);
                }
            }

            // Merge policies (remote overrides)
            self.policies.extend(remote.policies);
        }

        self.validate()?;
        Ok(())
    }
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

fn default_update_interval() -> u64 {
    3600 // 1 hour
}

/// Builder for MDM configuration
pub struct MdmConfigBuilder {
    config: MdmConfig,
}

impl Default for MdmConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn add_compliance_check(mut self, rule: ComplianceRule) -> Self {
        self.config.compliance_checks.push(rule);
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

/// Individual compliance rule
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceRule {
    /// Unique name for the rule
    pub name: String,

    /// Whether this rule is required for compliance
    pub required: bool,

    /// Type of validation to perform
    pub validation_type: ValidationType,

    /// Expected value or criteria
    pub criteria: ComplianceCriteria,

    /// Human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Severity level if the check fails
    #[serde(default)]
    pub severity: ComplianceSeverity,
}

impl ComplianceRule {
    pub fn new(name: impl Into<String>, validation_type: ValidationType) -> Self {
        Self {
            name: name.into(),
            required: true,
            validation_type,
            criteria: ComplianceCriteria::new(),
            description: None,
            severity: ComplianceSeverity::default(),
        }
    }

    pub fn validate(&self) -> MdmResult<()> {
        if self.name.is_empty() {
            return Err(MdmError::ValidationError("Rule name cannot be empty".into()));
        }
        Ok(())
    }

    #[cfg(any(feature = "audit-compliance", test))]
    pub fn check(&self, context: &ComplianceContext) -> MdmResult<ComplianceResult> {
        let passed = match &self.validation_type {
            ValidationType::FieldExists(field) => context.has_field(field),
            ValidationType::FieldEquals { field, value } => {
                context.get_field(field).map(|v| v == value).unwrap_or(false)
            }
            ValidationType::FieldMatches { field, pattern } => {
                context.get_field(field).map(|v| v.contains(pattern)).unwrap_or(false)
            }
            ValidationType::Custom(validator) => {
                // Execute custom validation logic
                self.execute_custom_validation(validator, context)?
            }
        };

        Ok(ComplianceResult {
            rule_name: self.name.clone(),
            passed,
            required: self.required,
            severity: self.severity.clone(),
            message: if !passed {
                Some(format!("Compliance check '{}' failed", self.name))
            } else {
                None
            },
        })
    }

    #[cfg(any(feature = "audit-compliance", test))]
    fn execute_custom_validation(
        &self,
        _validator: &str,
        _context: &ComplianceContext,
    ) -> MdmResult<bool> {
        // This would integrate with a validation engine
        // For now, return true as placeholder
        Ok(true)
    }
}

/// Types of validation that can be performed
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ValidationType {
    FieldExists(String),
    FieldEquals { field: String, value: String },
    FieldMatches { field: String, pattern: String },
    Custom(String),
}

/// Criteria for compliance validation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_values: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub regex_pattern: Option<String>,
}

impl ComplianceCriteria {
    /// Create a new empty ComplianceCriteria
    pub fn new() -> Self {
        Self { min_value: None, max_value: None, allowed_values: None, regex_pattern: None }
    }

    /// Create ComplianceCriteria with min and max values
    pub fn with_range(min: f64, max: f64) -> Self {
        Self {
            min_value: Some(min),
            max_value: Some(max),
            allowed_values: None,
            regex_pattern: None,
        }
    }

    /// Create ComplianceCriteria with allowed values
    pub fn with_allowed_values(values: Vec<String>) -> Self {
        Self { min_value: None, max_value: None, allowed_values: Some(values), regex_pattern: None }
    }
}

/// Severity levels for compliance violations
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ComplianceSeverity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

impl Default for ComplianceSeverity {
    fn default() -> Self {
        Self::Medium
    }
}

/// Policy setting with typed values
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

/// Typed policy values
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", content = "value")]
pub enum PolicyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    List(Vec<String>),
    Object(HashMap<String, String>),
}

/// Context for compliance checking
#[derive(Debug, Clone)]
#[cfg(any(feature = "audit-compliance", test))]
pub struct ComplianceContext {
    fields: HashMap<String, String>,
    metadata: HashMap<String, String>,
}

#[cfg(any(feature = "audit-compliance", test))]
impl Default for ComplianceContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(any(feature = "audit-compliance", test))]
impl ComplianceContext {
    pub fn new() -> Self {
        Self { fields: HashMap::new(), metadata: HashMap::new() }
    }

    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields.insert(key.into(), value.into());
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn has_field(&self, field: &str) -> bool {
        self.fields.contains_key(field)
    }

    pub fn get_field(&self, field: &str) -> Option<&String> {
        self.fields.get(field)
    }

    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
}

/// Result of a compliance check
#[derive(Debug, Clone)]
pub struct ComplianceResult {
    pub rule_name: String,
    pub passed: bool,
    pub required: bool,
    pub severity: ComplianceSeverity,
    pub message: Option<String>,
}

/// Report containing all compliance check results
#[derive(Debug, Clone)]
pub struct ComplianceReport {
    pub results: Vec<ComplianceResult>,
    pub passed: bool,
    pub critical_failures: usize,
    pub warnings: usize,
}

impl Default for ComplianceReport {
    fn default() -> Self {
        Self::new()
    }
}

impl ComplianceReport {
    pub fn new() -> Self {
        Self { results: Vec::new(), passed: true, critical_failures: 0, warnings: 0 }
    }

    pub fn add_result(&mut self, _rule_name: String, result: ComplianceResult) {
        if !result.passed {
            if result.required {
                self.passed = false;
                if matches!(result.severity, ComplianceSeverity::Critical) {
                    self.critical_failures += 1;
                }
            } else {
                self.warnings += 1;
            }
        }
        self.results.push(result);
    }

    pub fn is_compliant(&self) -> bool {
        self.passed && self.critical_failures == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mdm_config_default() {
        let config = MdmConfig::default();

        assert!(!config.policy_enforcement);
        assert!(config.remote_config_url.is_none());
        assert!(config.compliance_checks.is_empty());
        assert!(config.policies.is_empty());
        assert_eq!(config.update_interval_secs, 3600);
        assert!(!config.allow_local_override);
    }

    #[test]
    fn test_mdm_config_builder() {
        let config = MdmConfig::builder()
            .policy_enforcement(true)
            .remote_config_url("https://example.com/config")
            .update_interval_secs(1800)
            .allow_local_override(true)
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert!(config.policy_enforcement);
        assert_eq!(config.remote_config_url, Some("https://example.com/config".to_string()));
        assert_eq!(config.update_interval_secs, 1800);
        assert!(config.allow_local_override);
    }

    #[test]
    fn test_mdm_config_validate_invalid_url() {
        let config = MdmConfig {
            policy_enforcement: true,
            remote_config_url: Some("not-a-valid-url".to_string()),
            compliance_checks: Vec::new(),
            policies: HashMap::new(),
            update_interval_secs: 3600,
            allow_local_override: false,
        };

        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_mdm_config_validate_valid_url() {
        let config = MdmConfig {
            policy_enforcement: true,
            remote_config_url: Some("https://example.com/config".to_string()),
            compliance_checks: Vec::new(),
            policies: HashMap::new(),
            update_interval_secs: 3600,
            allow_local_override: false,
        };

        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_mdm_config_is_policy_enabled() {
        let mut config = MdmConfig::default();

        let policy = PolicySetting::new(PolicyValue::Boolean(true));
        config.policies.insert("test_policy".to_string(), policy);

        assert!(config.is_policy_enabled("test_policy"));
        assert!(!config.is_policy_enabled("nonexistent_policy"));
    }

    #[test]
    fn test_mdm_config_get_policy_value() {
        let mut config = MdmConfig::default();

        let policy = PolicySetting::new(PolicyValue::String("test_value".to_string()));
        config.policies.insert("test_policy".to_string(), policy);

        let value = config.get_policy_value("test_policy");
        assert!(value.is_some());

        let nonexistent = config.get_policy_value("nonexistent");
        assert!(nonexistent.is_none());
    }

    #[test]
    fn test_compliance_rule_new() {
        let rule =
            ComplianceRule::new("test_rule", ValidationType::FieldExists("field1".to_string()));

        assert_eq!(rule.name, "test_rule");
        assert!(rule.required);
        assert!(matches!(rule.severity, ComplianceSeverity::Medium));
    }

    #[test]
    fn test_compliance_rule_validate_empty_name() {
        let rule = ComplianceRule {
            name: "".to_string(),
            required: true,
            validation_type: ValidationType::FieldExists("field1".to_string()),
            criteria: ComplianceCriteria::new(),
            description: None,
            severity: ComplianceSeverity::Medium,
        };

        let result = rule.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_compliance_rule_validate_valid() {
        let rule =
            ComplianceRule::new("valid_rule", ValidationType::FieldExists("field1".to_string()));

        let result = rule.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_compliance_rule_check_field_exists() {
        let rule = ComplianceRule::new(
            "field_exists",
            ValidationType::FieldExists("test_field".to_string()),
        );

        let context = ComplianceContext::new().with_field("test_field", "value");

        let result = rule.check(&context).unwrap();
        assert!(result.passed);
        assert_eq!(result.rule_name, "field_exists");
    }

    #[test]
    fn test_compliance_rule_check_field_exists_fail() {
        let rule = ComplianceRule::new(
            "field_exists",
            ValidationType::FieldExists("missing_field".to_string()),
        );

        let context = ComplianceContext::new().with_field("other_field", "value");

        let result = rule.check(&context).unwrap();
        assert!(!result.passed);
    }

    #[test]
    fn test_compliance_rule_check_field_equals() {
        let rule = ComplianceRule::new(
            "field_equals",
            ValidationType::FieldEquals {
                field: "status".to_string(),
                value: "active".to_string(),
            },
        );

        let context = ComplianceContext::new().with_field("status", "active");

        let result = rule.check(&context).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_compliance_rule_check_field_matches() {
        let rule = ComplianceRule::new(
            "field_matches",
            ValidationType::FieldMatches {
                field: "email".to_string(),
                pattern: "@example.com".to_string(),
            },
        );

        let context = ComplianceContext::new().with_field("email", "user@example.com");

        let result = rule.check(&context).unwrap();
        assert!(result.passed);
    }

    #[test]
    fn test_policy_setting_validate_empty_string() {
        let policy = PolicySetting::new(PolicyValue::String("".to_string()));

        let result = policy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_setting_validate_nan() {
        let policy = PolicySetting::new(PolicyValue::Number(f64::NAN));

        let result = policy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_setting_validate_enforced_empty_list() {
        let mut policy = PolicySetting::new(PolicyValue::List(vec![]));
        policy.enforced = true;

        let result = policy.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_setting_validate_valid() {
        let policy = PolicySetting::new(PolicyValue::String("valid_value".to_string()));

        let result = policy.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_compliance_context_with_field() {
        let context =
            ComplianceContext::new().with_field("key1", "value1").with_field("key2", "value2");

        assert!(context.has_field("key1"));
        assert!(context.has_field("key2"));
        assert!(!context.has_field("key3"));

        assert_eq!(context.get_field("key1"), Some(&"value1".to_string()));
        assert_eq!(context.get_field("key2"), Some(&"value2".to_string()));
        assert_eq!(context.get_field("key3"), None);
    }

    #[test]
    fn test_compliance_report_new() {
        let report = ComplianceReport::new();

        assert!(report.results.is_empty());
        assert!(report.passed);
        assert_eq!(report.critical_failures, 0);
        assert_eq!(report.warnings, 0);
    }

    #[test]
    fn test_compliance_report_add_passing_result() {
        let mut report = ComplianceReport::new();

        let result = ComplianceResult {
            rule_name: "test_rule".to_string(),
            passed: true,
            required: true,
            severity: ComplianceSeverity::Medium,
            message: None,
        };

        report.add_result("test_rule".to_string(), result);

        assert_eq!(report.results.len(), 1);
        assert!(report.passed);
        assert_eq!(report.critical_failures, 0);
        assert_eq!(report.warnings, 0);
    }

    #[test]
    fn test_compliance_report_add_failing_required_result() {
        let mut report = ComplianceReport::new();

        let result = ComplianceResult {
            rule_name: "test_rule".to_string(),
            passed: false,
            required: true,
            severity: ComplianceSeverity::High,
            message: Some("Test failure".to_string()),
        };

        report.add_result("test_rule".to_string(), result);

        assert_eq!(report.results.len(), 1);
        assert!(!report.passed);
        assert_eq!(report.critical_failures, 0);
        assert_eq!(report.warnings, 0);
    }

    #[test]
    fn test_compliance_report_add_failing_critical_result() {
        let mut report = ComplianceReport::new();

        let result = ComplianceResult {
            rule_name: "test_rule".to_string(),
            passed: false,
            required: true,
            severity: ComplianceSeverity::Critical,
            message: Some("Critical failure".to_string()),
        };

        report.add_result("test_rule".to_string(), result);

        assert_eq!(report.results.len(), 1);
        assert!(!report.passed);
        assert_eq!(report.critical_failures, 1);
        assert_eq!(report.warnings, 0);
    }

    #[test]
    fn test_compliance_report_add_failing_non_required_result() {
        let mut report = ComplianceReport::new();

        let result = ComplianceResult {
            rule_name: "test_rule".to_string(),
            passed: false,
            required: false,
            severity: ComplianceSeverity::Low,
            message: Some("Warning".to_string()),
        };

        report.add_result("test_rule".to_string(), result);

        assert_eq!(report.results.len(), 1);
        assert!(report.passed); // Still passes because not required
        assert_eq!(report.critical_failures, 0);
        assert_eq!(report.warnings, 1);
    }

    #[test]
    fn test_compliance_report_is_compliant() {
        let mut report = ComplianceReport::new();

        // Add passing result
        let passing = ComplianceResult {
            rule_name: "passing".to_string(),
            passed: true,
            required: true,
            severity: ComplianceSeverity::Medium,
            message: None,
        };
        report.add_result("passing".to_string(), passing);

        assert!(report.is_compliant());
    }

    #[test]
    fn test_compliance_report_not_compliant_with_critical() {
        let mut report = ComplianceReport::new();

        // Add critical failure
        let critical = ComplianceResult {
            rule_name: "critical".to_string(),
            passed: false,
            required: true,
            severity: ComplianceSeverity::Critical,
            message: Some("Critical error".to_string()),
        };
        report.add_result("critical".to_string(), critical);

        assert!(!report.is_compliant());
    }

    #[test]
    fn test_mdm_config_check_compliance() {
        let rule = ComplianceRule::new(
            "test_rule",
            ValidationType::FieldExists("required_field".to_string()),
        );

        let config = MdmConfig::builder().add_compliance_check(rule).build().unwrap();

        let context = ComplianceContext::new().with_field("required_field", "value");

        let report = config.check_compliance(&context).unwrap();
        assert!(report.is_compliant());
    }

    #[test]
    fn test_mdm_config_merge_remote_no_override() {
        let mut local = MdmConfig { allow_local_override: false, ..Default::default() };

        let remote = MdmConfig {
            policy_enforcement: true,
            update_interval_secs: 7200,
            ..Default::default()
        };

        local.merge_remote(remote).unwrap();

        // Local should be completely replaced
        assert!(local.policy_enforcement);
        assert_eq!(local.update_interval_secs, 7200);
    }

    #[test]
    fn test_mdm_config_merge_remote_with_override() {
        let mut local = MdmConfig {
            allow_local_override: true,
            update_interval_secs: 1800,
            ..Default::default()
        };

        let remote = MdmConfig { policy_enforcement: true, ..Default::default() };

        local.merge_remote(remote).unwrap();

        // Remote policy enforcement should apply
        assert!(local.policy_enforcement);
        // But local update interval should be preserved
        assert_eq!(local.update_interval_secs, 1800);
    }

    #[test]
    fn test_policy_value_serialization() {
        let string_value = PolicyValue::String("test".to_string());
        let json = serde_json::to_string(&string_value).expect("Should serialize");
        assert!(json.contains("\"type\""));

        let number_value = PolicyValue::Number(42.5);
        let json = serde_json::to_string(&number_value).expect("Should serialize");
        assert!(json.contains("42.5"));

        let bool_value = PolicyValue::Boolean(true);
        let json = serde_json::to_string(&bool_value).expect("Should serialize");
        assert!(json.contains("true"));
    }
}
