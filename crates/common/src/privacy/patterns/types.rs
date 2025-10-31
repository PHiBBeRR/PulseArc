use std::collections::HashMap;
use std::fmt;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Comprehensive enumeration of Personally Identifiable Information (PII) types
///
/// This enum covers a wide range of PII categories as defined by various
/// privacy regulations including GDPR, HIPAA, CCPA, and others. Each variant
/// represents a specific type of sensitive data that may require special
/// handling.
///
/// # Examples
/// ```
/// use pulsearc_common::privacy::patterns::types::PiiType;
///
/// let email_type = PiiType::Email;
/// let is_financial = matches!(email_type, PiiType::CreditCard | PiiType::BankAccount);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive] // Allow for future PII types without breaking changes
pub enum PiiType {
    // Personal Identifiers
    Email,
    Phone,
    Ssn,
    DriverLicense,
    Passport,
    TaxId,

    // Financial Information
    CreditCard,
    BankAccount,
    Iban,
    Swift,
    BitcoinAddress,

    // Location Data
    IpAddress,
    MacAddress,
    GpsCoordinates,
    HomeAddress,
    PostalCode,

    // Health Information
    MedicalRecord,
    HealthInsurance,
    PrescriptionId,

    // Biometric Data
    Fingerprint,
    FaceRecognition,
    VoicePrint,

    // Online Identifiers
    Username,
    Password,
    ApiKey,
    SessionToken,
    Cookie,

    // Legal Documents
    LegalCase,
    ContractId,
    LicenseNumber,

    // Employment
    EmployeeId,
    Salary,
    PerformanceReview,

    // Educational
    StudentId,
    GradeRecord,
    Transcript,

    // Custom patterns
    Custom(String),

    // Composite patterns
    FullName,
    DateOfBirth,
    Age,
    Gender,

    // Sensitive business data
    TradeSecret,
    FinancialData,
    CustomerData,
}

impl fmt::Display for PiiType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // Personal Identifiers
            PiiType::Email => write!(f, "email"),
            PiiType::Phone => write!(f, "phone"),
            PiiType::Ssn => write!(f, "ssn"),
            PiiType::DriverLicense => write!(f, "driver_license"),
            PiiType::Passport => write!(f, "passport"),
            PiiType::TaxId => write!(f, "tax_id"),

            // Financial Information
            PiiType::CreditCard => write!(f, "credit_card"),
            PiiType::BankAccount => write!(f, "bank_account"),
            PiiType::Iban => write!(f, "iban"),
            PiiType::Swift => write!(f, "swift"),
            PiiType::BitcoinAddress => write!(f, "bitcoin_address"),

            // Location Data
            PiiType::IpAddress => write!(f, "ip_address"),
            PiiType::MacAddress => write!(f, "mac_address"),
            PiiType::GpsCoordinates => write!(f, "gps_coordinates"),
            PiiType::HomeAddress => write!(f, "home_address"),
            PiiType::PostalCode => write!(f, "postal_code"),

            // Health Information
            PiiType::MedicalRecord => write!(f, "medical_record"),
            PiiType::HealthInsurance => write!(f, "health_insurance"),
            PiiType::PrescriptionId => write!(f, "prescription_id"),

            // Biometric Data
            PiiType::Fingerprint => write!(f, "fingerprint"),
            PiiType::FaceRecognition => write!(f, "face_recognition"),
            PiiType::VoicePrint => write!(f, "voice_print"),

            // Online Identifiers
            PiiType::Username => write!(f, "username"),
            PiiType::Password => write!(f, "password"),
            PiiType::ApiKey => write!(f, "api_key"),
            PiiType::SessionToken => write!(f, "session_token"),
            PiiType::Cookie => write!(f, "cookie"),

            // Legal Documents
            PiiType::LegalCase => write!(f, "legal_case"),
            PiiType::ContractId => write!(f, "contract_id"),
            PiiType::LicenseNumber => write!(f, "license_number"),

            // Employment
            PiiType::EmployeeId => write!(f, "employee_id"),
            PiiType::Salary => write!(f, "salary"),
            PiiType::PerformanceReview => write!(f, "performance_review"),

            // Educational
            PiiType::StudentId => write!(f, "student_id"),
            PiiType::GradeRecord => write!(f, "grade_record"),
            PiiType::Transcript => write!(f, "transcript"),

            // Custom patterns
            PiiType::Custom(name) => write!(f, "custom_{}", name),

            // Composite patterns
            PiiType::FullName => write!(f, "full_name"),
            PiiType::DateOfBirth => write!(f, "date_of_birth"),
            PiiType::Age => write!(f, "age"),
            PiiType::Gender => write!(f, "gender"),

            // Sensitive business data
            PiiType::TradeSecret => write!(f, "trade_secret"),
            PiiType::FinancialData => write!(f, "financial_data"),
            PiiType::CustomerData => write!(f, "customer_data"),
        }
    }
}

/// Sensitivity level for PII classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SensitivityLevel {
    Public = 0,
    Internal = 1,
    Confidential = 2,
    Restricted = 3,
    TopSecret = 4,
}

impl fmt::Display for SensitivityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Public => write!(f, "Public"),
            Self::Internal => write!(f, "Internal"),
            Self::Confidential => write!(f, "Confidential"),
            Self::Restricted => write!(f, "Restricted"),
            Self::TopSecret => write!(f, "Top Secret"),
        }
    }
}

/// Detection confidence level (0.0 to 1.0)
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct ConfidenceScore(f64);

impl ConfidenceScore {
    /// Creates a new confidence score, clamping the value between 0.0 and 1.0
    pub fn new(score: f64) -> Self {
        debug_assert!(score.is_finite(), "Confidence score must be finite");
        Self(score.clamp(0.0, 1.0))
    }

    /// Creates a confidence score without validation (for trusted input)
    pub(crate) fn new_unchecked(score: f64) -> Self {
        debug_assert!((0.0..=1.0).contains(&score), "Score must be between 0.0 and 1.0");
        Self(score)
    }

    /// Returns the confidence value as f64
    pub fn value(&self) -> f64 {
        self.0
    }

    /// Returns true if this is considered high confidence (>= 0.8)
    pub fn is_high_confidence(&self) -> bool {
        self.0 >= 0.8
    }

    /// Returns true if this is considered medium confidence (0.5 to 0.8)
    pub fn is_medium_confidence(&self) -> bool {
        self.0 >= 0.5 && self.0 < 0.8
    }

    /// Returns true if this is considered low confidence (< 0.5)
    pub fn is_low_confidence(&self) -> bool {
        self.0 < 0.5
    }

    /// Combines two confidence scores using the maximum value
    pub const fn max(self, other: Self) -> Self {
        if self.0 >= other.0 {
            self
        } else {
            other
        }
    }

    /// Combines two confidence scores using the minimum value
    pub const fn min(self, other: Self) -> Self {
        if self.0 <= other.0 {
            self
        } else {
            other
        }
    }

    /// Combines two confidence scores using weighted average
    pub fn weighted_average(self, other: Self, weight: f64) -> Self {
        let weight = weight.clamp(0.0, 1.0);
        let combined = self.0 * weight + other.0 * (1.0 - weight);
        Self::new_unchecked(combined)
    }

    /// Predefined confidence levels as constants
    pub const LOW: Self = Self(0.3);
    pub const MEDIUM: Self = Self(0.6);
    pub const HIGH: Self = Self(0.8);
    pub const MAXIMUM: Self = Self(1.0);
}

impl fmt::Display for ConfidenceScore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}%", self.0 * 100.0)
    }
}

impl From<f64> for ConfidenceScore {
    fn from(score: f64) -> Self {
        Self::new(score)
    }
}

impl From<ConfidenceScore> for f64 {
    fn from(score: ConfidenceScore) -> Self {
        score.0
    }
}

impl std::ops::Add for ConfidenceScore {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.0 + other.0)
    }
}

impl std::ops::Mul<f64> for ConfidenceScore {
    type Output = Self;

    fn mul(self, scalar: f64) -> Self {
        Self::new(self.0 * scalar)
    }
}

/// A detected PII entity with context and metadata
///
/// This structure represents a single piece of detected personally identifiable
/// information, including its location in the text, confidence score, and
/// associated metadata for compliance and audit purposes.
///
/// # Examples
/// ```
/// use pulsearc_common::privacy::patterns::types::{
///     ConfidenceScore, PiiEntity, PiiType, SensitivityLevel,
/// };
///
/// let entity = PiiEntity::builder()
///     .entity_type(PiiType::Email)
///     .value("user@example.com")
///     .position(10, 25)
///     .confidence(ConfidenceScore::HIGH)
///     .build();
/// ```
#[derive(Clone, Serialize, Deserialize)]
pub struct PiiEntity {
    pub entity_type: PiiType,
    pub value: String,
    pub start_position: usize,
    pub end_position: usize,
    pub confidence: ConfidenceScore,
    pub sensitivity_level: SensitivityLevel,
    pub context: String,
    pub metadata: HashMap<String, String>,
    pub detection_method: DetectionMethod,
    pub compliance_tags: Vec<String>,
}

impl PiiEntity {
    /// Creates a new PII entity builder
    pub fn builder() -> PiiEntityBuilder {
        PiiEntityBuilder::default()
    }

    /// Returns the length of the detected entity in characters
    pub fn length(&self) -> usize {
        self.end_position.saturating_sub(self.start_position)
    }

    /// Checks if this entity overlaps with another entity
    pub fn overlaps_with(&self, other: &PiiEntity) -> bool {
        !(self.end_position <= other.start_position || other.end_position <= self.start_position)
    }

    /// Returns true if this entity is considered highly sensitive
    pub fn is_highly_sensitive(&self) -> bool {
        matches!(self.sensitivity_level, SensitivityLevel::Restricted | SensitivityLevel::TopSecret)
    }
}

impl std::fmt::Debug for PiiEntity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiiEntity")
            .field("entity_type", &self.entity_type)
            .field("value", &"[REDACTED]")
            .field("position", &format!("{}..{}", self.start_position, self.end_position))
            .field("confidence", &self.confidence)
            .field("sensitivity_level", &self.sensitivity_level)
            .field("detection_method", &self.detection_method)
            .field("compliance_tags", &self.compliance_tags)
            .finish()
    }
}

/// Builder for PiiEntity with validation
#[derive(Default)]
pub struct PiiEntityBuilder {
    entity_type: Option<PiiType>,
    value: Option<String>,
    start_position: Option<usize>,
    end_position: Option<usize>,
    confidence: Option<ConfidenceScore>,
    sensitivity_level: Option<SensitivityLevel>,
    context: Option<String>,
    metadata: HashMap<String, String>,
    detection_method: Option<DetectionMethod>,
    compliance_tags: Vec<String>,
}

impl PiiEntityBuilder {
    pub fn entity_type(mut self, entity_type: PiiType) -> Self {
        self.entity_type = Some(entity_type);
        self
    }

    pub fn value<S: Into<String>>(mut self, value: S) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn position(mut self, start: usize, end: usize) -> Self {
        self.start_position = Some(start);
        self.end_position = Some(end);
        self
    }

    pub fn confidence(mut self, confidence: ConfidenceScore) -> Self {
        self.confidence = Some(confidence);
        self
    }

    pub fn sensitivity_level(mut self, level: SensitivityLevel) -> Self {
        self.sensitivity_level = Some(level);
        self
    }

    pub fn context<S: Into<String>>(mut self, context: S) -> Self {
        self.context = Some(context.into());
        self
    }

    pub fn detection_method(mut self, method: DetectionMethod) -> Self {
        self.detection_method = Some(method);
        self
    }

    pub fn metadata<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    pub fn compliance_tag<S: Into<String>>(mut self, tag: S) -> Self {
        self.compliance_tags.push(tag.into());
        self
    }

    pub fn build(self) -> Result<PiiEntity, &'static str> {
        let entity_type = self.entity_type.ok_or("entity_type is required")?;
        let value = self.value.ok_or("value is required")?;
        let start_position = self.start_position.ok_or("start_position is required")?;
        let end_position = self.end_position.ok_or("end_position is required")?;

        if start_position >= end_position {
            return Err("start_position must be less than end_position");
        }

        let confidence = self.confidence.unwrap_or(ConfidenceScore::MEDIUM);
        let sensitivity_level = self.sensitivity_level.unwrap_or(SensitivityLevel::Internal);
        let detection_method = self.detection_method.unwrap_or(DetectionMethod::Regex);

        Ok(PiiEntity {
            entity_type,
            value,
            start_position,
            end_position,
            confidence,
            sensitivity_level,
            context: self.context.unwrap_or_default(),
            metadata: self.metadata,
            detection_method,
            compliance_tags: self.compliance_tags,
        })
    }
}

impl std::fmt::Debug for PiiEntityBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PiiEntityBuilder")
            .field("entity_type", &self.entity_type)
            .field("value", &if self.value.is_some() { "[REDACTED]" } else { "None" })
            .field("position", &format!("{:?}..{:?}", self.start_position, self.end_position))
            .field("confidence", &self.confidence)
            .field("detection_method", &self.detection_method)
            .finish()
    }
}

/// Method used to detect PII
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetectionMethod {
    Regex,
    MachineLearning,
    Dictionary,
    ContextualAnalysis,
    ChecksumValidation,
    Custom(String),
    Composite(Vec<DetectionMethod>),
}

impl fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regex => write!(f, "Regex"),
            Self::MachineLearning => write!(f, "Machine Learning"),
            Self::Dictionary => write!(f, "Dictionary"),
            Self::ContextualAnalysis => write!(f, "Contextual Analysis"),
            Self::ChecksumValidation => write!(f, "Checksum Validation"),
            Self::Custom(name) => write!(f, "Custom: {}", name),
            Self::Composite(methods) => {
                let method_names: Vec<String> = methods.iter().map(|m| m.to_string()).collect();
                write!(f, "Composite: [{}]", method_names.join(", "))
            }
        }
    }
}

/// PII detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    pub original_text: String,
    pub entities: Vec<PiiEntity>,
    pub processing_time: std::time::Duration,
    pub overall_sensitivity: SensitivityLevel,
    pub compliance_status: ComplianceStatus,
    pub redaction_applied: bool,
    pub metadata: HashMap<String, String>,
}

/// Compliance status for detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStatus {
    pub frameworks: Vec<ComplianceFramework>,
    pub violations: Vec<ComplianceViolation>,
    pub recommendations: Vec<String>,
    pub risk_score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComplianceFramework {
    Gdpr,
    Hipaa,
    Ccpa,
    Pipeda,
    Lgpd,
    Sox,
    Pci,
    Ferpa,
    Glba,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceViolation {
    pub framework: ComplianceFramework,
    pub rule_id: String,
    pub description: String,
    pub severity: ViolationSeverity,
    pub suggested_action: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ViolationSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Redaction strategy for different PII types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedactionStrategy {
    FullMasking,          // Replace with ***
    PartialMasking,       // Show first/last chars
    Tokenization,         // Replace with token
    Encryption,           // Encrypt the value
    Hashing,              // Hash the value
    Removal,              // Remove completely
    Substitution(String), // Replace with specific text
    FormatPreserving,     // Keep format but change content
}

/// Pattern matching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternConfig {
    pub pattern_type: PiiType,
    pub regex_patterns: Vec<String>,
    pub context_patterns: Vec<String>,
    pub exclusion_patterns: Vec<String>,
    pub sensitivity_level: SensitivityLevel,
    pub redaction_strategy: RedactionStrategy,
    pub minimum_confidence: ConfidenceScore,
    pub enabled: bool,
    pub compliance_frameworks: Vec<ComplianceFramework>,
    pub custom_validators: Vec<String>,
}

/// Analysis context for PII detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisContext {
    pub document_type: Option<String>,
    pub source_application: Option<String>,
    pub user_id: Option<String>,
    pub department: Option<String>,
    pub data_classification: Option<String>,
    pub jurisdiction: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub session_id: String,
    pub compliance_zone: Option<String>,
    pub processing_purpose: Option<String>,
    pub retention_policy: Option<String>,
}

/// Performance metrics for pattern matching
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub total_patterns_checked: usize,
    pub patterns_matched: usize,
    pub false_positives: usize,
    pub false_negatives: usize,
    pub processing_time_ms: u64,
    pub memory_usage_bytes: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
}

/// Quality assurance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetrics {
    pub precision: f64,
    pub recall: f64,
    pub f1_score: f64,
    pub accuracy: f64,
    pub confidence_distribution: HashMap<String, u64>,
    pub entity_type_distribution: HashMap<PiiType, u64>,
}

/// Audit trail entry for PII processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiAuditEntry {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub operation_type: PiiOperationType,
    pub entities_detected: Vec<PiiType>,
    pub sensitivity_levels: Vec<SensitivityLevel>,
    pub user_id: Option<String>,
    pub session_id: String,
    pub source_context: AnalysisContext,
    pub compliance_frameworks: Vec<ComplianceFramework>,
    pub redaction_applied: bool,
    pub processing_time: std::time::Duration,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PiiOperationType {
    Detection,
    Redaction,
    Classification,
    Validation,
    Export,
    Deletion,
    Anonymization,
    Pseudonymization,
}

impl std::fmt::Display for PiiOperationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PiiOperationType::Detection => write!(f, "Detection"),
            PiiOperationType::Redaction => write!(f, "Redaction"),
            PiiOperationType::Classification => write!(f, "Classification"),
            PiiOperationType::Validation => write!(f, "Validation"),
            PiiOperationType::Export => write!(f, "Export"),
            PiiOperationType::Deletion => write!(f, "Deletion"),
            PiiOperationType::Anonymization => write!(f, "Anonymization"),
            PiiOperationType::Pseudonymization => write!(f, "Pseudonymization"),
        }
    }
}

/// Machine learning model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MlModelInfo {
    pub model_name: String,
    pub model_version: String,
    pub training_date: DateTime<Utc>,
    pub accuracy_metrics: QualityMetrics,
    pub supported_languages: Vec<String>,
    pub entity_types: Vec<PiiType>,
}

/// Real-time statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PiiStatistics {
    pub total_documents_processed: u64,
    pub total_entities_detected: u64,
    pub entities_by_type: HashMap<PiiType, u64>,
    pub average_confidence: f64,
    pub processing_rate_per_second: f64,
    pub compliance_violations: u64,
    pub false_positive_rate: f64,
    pub last_updated: DateTime<Utc>,
}

impl Default for AnalysisContext {
    fn default() -> Self {
        Self {
            document_type: None,
            source_application: None,
            user_id: None,
            department: None,
            data_classification: None,
            jurisdiction: None,
            timestamp: Utc::now(),
            session_id: uuid::Uuid::new_v4().to_string(),
            compliance_zone: None,
            processing_purpose: None,
            retention_policy: None,
        }
    }
}

impl AnalysisContext {
    /// Creates a new minimal context for testing or basic use
    pub fn minimal() -> Self {
        Self {
            document_type: None,
            source_application: None,
            user_id: None,
            department: None,
            data_classification: None,
            jurisdiction: None,
            timestamp: Utc::now(),
            session_id: uuid::Uuid::new_v4().to_string(),
            compliance_zone: None,
            processing_purpose: None,
            retention_policy: None,
        }
    }

    /// Creates a context with the specified user and session information
    pub fn with_user(user_id: String, session_id: String) -> Self {
        Self { user_id: Some(user_id), session_id, ..Self::default() }
    }

    /// Creates a context for a specific compliance zone and jurisdiction
    pub fn with_compliance(jurisdiction: String, compliance_zone: String) -> Self {
        Self {
            jurisdiction: Some(jurisdiction),
            compliance_zone: Some(compliance_zone),
            ..Self::default()
        }
    }
}

impl Default for ComplianceStatus {
    fn default() -> Self {
        Self {
            frameworks: vec![ComplianceFramework::Gdpr],
            violations: Vec::new(),
            recommendations: Vec::new(),
            risk_score: 0.0,
        }
    }
}
