// Core PII pattern detection and matching functionality
// Enhanced from the original simple implementation

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use once_cell::sync::Lazy;
use regex::Regex;
use tokio::sync::RwLock;
use tracing::{instrument, warn};

use super::config::PiiDetectionConfig;
use super::error::{PiiError, PiiResult};
use super::types::{
    AnalysisContext, ComplianceFramework, ComplianceStatus, ConfidenceScore, DetectionMethod,
    DetectionResult, PatternConfig, PerformanceMetrics, PiiEntity, PiiType, SensitivityLevel,
};
use crate::error::CommonError;

/// Type aliases for complex types to improve readability
type CompiledPatternStorage = Arc<RwLock<HashMap<PiiType, Arc<CompiledPatternSet>>>>;
type CompiledPatternMap = HashMap<PiiType, Arc<CompiledPatternSet>>;
type PatternCacheStorage = Arc<RwLock<PatternCache>>;
type FalsePositiveStorage = Arc<RwLock<HashMap<PiiType, Vec<Regex>>>>;

/// Compiled regex patterns for efficient reuse
pub(crate) const EMAIL_PATTERN: &str = r"(?u)\b[\p{L}\p{N}._%+-]+@[\p{L}\p{N}.-]+\.[\p{L}]{2,}\b";

static EMAIL_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(EMAIL_PATTERN).expect("EMAIL_REGEX should compile - this is a bug"));

static SSN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").expect("SSN_REGEX should compile - this is a bug")
});

static IP_ADDRESS_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").expect("Invalid IP address regex")
});

const CONTEXT_WINDOW_CHARS: usize = 100;
const CACHE_MAX_ENTRIES: usize = 512;
const CACHE_MAX_BYTES: usize = 5 * 1024 * 1024; // ~5 MiB
const CACHE_ENTITY_OVERHEAD: usize = 128;

/// Collection of compiled regex patterns for a given PII type
#[derive(Debug)]
struct CompiledPatternSet {
    primary: Vec<Regex>,
    context: Vec<Regex>,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    entities: Vec<PiiEntity>,
    byte_size: usize,
}

#[derive(Debug, Default)]
struct PatternCache {
    entries: HashMap<String, CacheEntry>,
    order: VecDeque<String>,
    total_bytes: usize,
}

impl PatternCache {
    fn new() -> Self {
        Self::default()
    }

    fn get(&mut self, key: &str) -> Option<Vec<PiiEntity>> {
        if let Some(entry) = self.entries.get(key) {
            let value = entry.entities.clone();
            self.touch(key);
            Some(value)
        } else {
            None
        }
    }

    fn insert(&mut self, key: String, entities: Vec<PiiEntity>) {
        let size = Self::calculate_size(&key, &entities);

        if let Some(existing) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(existing.byte_size);
            self.remove_from_order(&key);
        }

        self.total_bytes = self.total_bytes.saturating_add(size);
        self.order.push_back(key.clone());
        self.entries.insert(key, CacheEntry { entities, byte_size: size });
        self.evict_if_needed();
    }

    fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.total_bytes = 0;
    }

    fn touch(&mut self, key: &str) {
        self.remove_from_order(key);
        self.order.push_back(key.to_string());
    }

    fn remove_from_order(&mut self, key: &str) {
        self.order.retain(|existing| existing != key);
    }

    fn evict_if_needed(&mut self) {
        while (self.total_bytes > CACHE_MAX_BYTES || self.entries.len() > CACHE_MAX_ENTRIES)
            && !self.order.is_empty()
        {
            if let Some(oldest_key) = self.order.pop_front() {
                if let Some(entry) = self.entries.remove(&oldest_key) {
                    self.total_bytes = self.total_bytes.saturating_sub(entry.byte_size);
                }
            } else {
                break;
            }
        }
    }

    fn calculate_size(key: &str, entities: &[PiiEntity]) -> usize {
        let mut size = key.len();
        for entity in entities {
            size = size.saturating_add(CACHE_ENTITY_OVERHEAD);
            size = size.saturating_add(entity.value.len());
            size = size.saturating_add(entity.context.len());
            size = size.saturating_add(
                entity.metadata.iter().map(|(k, v)| k.len() + v.len()).sum::<usize>(),
            );
            size = size
                .saturating_add(entity.compliance_tags.iter().map(|tag| tag.len()).sum::<usize>());
        }
        size
    }

    #[cfg(test)]
    fn entry_count(&self) -> usize {
        self.entries.len()
    }

    #[cfg(test)]
    fn byte_usage(&self) -> usize {
        self.total_bytes
    }

    #[cfg(test)]
    fn order_len(&self) -> usize {
        self.order.len()
    }
}

/// Core pattern matcher with advanced detection capabilities
pub struct PatternMatcher {
    config: Arc<RwLock<PiiDetectionConfig>>,
    compiled_patterns: CompiledPatternStorage,
    pattern_cache: PatternCacheStorage,
    performance_metrics: Arc<RwLock<PerformanceMetrics>>,
    false_positive_patterns: FalsePositiveStorage,
}

impl std::fmt::Debug for PatternMatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PatternMatcher")
            .field("config_version", &"[REDACTED]")
            .field("compiled_patterns_count", &"[REDACTED]")
            .field("cache_size", &"[REDACTED]")
            .field("metrics_available", &true)
            .finish()
    }
}

impl Clone for PatternMatcher {
    fn clone(&self) -> Self {
        Self {
            config: Arc::clone(&self.config),
            compiled_patterns: Arc::clone(&self.compiled_patterns),
            pattern_cache: Arc::clone(&self.pattern_cache),
            performance_metrics: Arc::clone(&self.performance_metrics),
            false_positive_patterns: Arc::clone(&self.false_positive_patterns),
        }
    }
}

impl PatternMatcher {
    fn compile_patterns(config: &PiiDetectionConfig) -> PiiResult<CompiledPatternMap> {
        let mut compiled: CompiledPatternMap = HashMap::new();

        for (pii_type, pattern_config) in &config.pattern_configs {
            let primary = pattern_config
                .regex_patterns
                .iter()
                .map(|pattern| {
                    Regex::new(pattern).map_err(|e| PiiError::PatternCompilation(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            let context = pattern_config
                .context_patterns
                .iter()
                .map(|pattern| {
                    Regex::new(&format!(r"(?i)\b{}\b", regex::escape(pattern)))
                        .map_err(|e| PiiError::PatternCompilation(e.to_string()))
                })
                .collect::<Result<Vec<_>, _>>()?;

            compiled.insert(pii_type.clone(), Arc::new(CompiledPatternSet { primary, context }));
        }

        Ok(compiled)
    }

    async fn compiled_patterns_for(&self, pii_type: &PiiType) -> Option<Arc<CompiledPatternSet>> {
        self.compiled_patterns.read().await.get(pii_type).cloned()
    }

    fn compile_primary_patterns(pattern_config: &PatternConfig) -> PiiResult<Vec<Regex>> {
        pattern_config
            .regex_patterns
            .iter()
            .map(|pattern| {
                Regex::new(pattern).map_err(|e| PiiError::PatternCompilation(e.to_string()))
            })
            .collect()
    }

    fn compile_context_patterns(pattern_config: &PatternConfig) -> PiiResult<Vec<Regex>> {
        pattern_config
            .context_patterns
            .iter()
            .map(|pattern| {
                Regex::new(&format!(r"(?i)\b{}\b", regex::escape(pattern)))
                    .map_err(|e| PiiError::PatternCompilation(e.to_string()))
            })
            .collect()
    }

    fn resolve_primary_regexes(
        compiled: Option<&CompiledPatternSet>,
        pattern_config: &PatternConfig,
        static_fallback: Option<&Regex>,
    ) -> PiiResult<Vec<Regex>> {
        if let Some(set) = compiled {
            if !set.primary.is_empty() {
                return Ok(set.primary.clone());
            }
        }

        let fallback_primary = Self::compile_primary_patterns(pattern_config)?;
        if fallback_primary.is_empty() {
            if let Some(fallback) = static_fallback {
                Ok(vec![fallback.clone()])
            } else {
                Ok(Vec::new())
            }
        } else {
            Ok(fallback_primary)
        }
    }

    fn resolve_context_regexes(
        compiled: Option<&CompiledPatternSet>,
        pattern_config: &PatternConfig,
    ) -> PiiResult<Vec<Regex>> {
        if let Some(set) = compiled {
            if !set.context.is_empty() {
                return Ok(set.context.clone());
            }
        }

        Self::compile_context_patterns(pattern_config)
    }

    fn context_window_bounds(text: &str, start: usize, end: usize) -> (usize, usize) {
        let window_start = Self::expand_left(text, start, CONTEXT_WINDOW_CHARS);
        let window_end = Self::expand_right(text, end, CONTEXT_WINDOW_CHARS);
        (window_start, window_end)
    }

    fn expand_left(text: &str, mut byte_idx: usize, mut chars: usize) -> usize {
        while byte_idx > 0 && chars > 0 {
            if let Some((prev_idx, _)) = text[..byte_idx].char_indices().next_back() {
                byte_idx = prev_idx;
            } else {
                byte_idx = 0;
                break;
            }
            chars -= 1;
        }
        byte_idx
    }

    fn expand_right(text: &str, mut byte_idx: usize, mut chars: usize) -> usize {
        let len = text.len();
        while byte_idx < len && chars > 0 {
            if let Some(ch) = text[byte_idx..].chars().next() {
                byte_idx = (byte_idx + ch.len_utf8()).min(len);
            } else {
                byte_idx = len;
                break;
            }
            chars -= 1;
        }
        byte_idx
    }

    /// Create new pattern matcher with configuration
    pub async fn new(config: PiiDetectionConfig) -> PiiResult<Self> {
        config.validate()?;

        let compiled = Self::compile_patterns(&config)?;

        let matcher = Self {
            config: Arc::new(RwLock::new(config)),
            compiled_patterns: Arc::new(RwLock::new(compiled)),
            pattern_cache: Arc::new(RwLock::new(PatternCache::new())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            false_positive_patterns: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(matcher)
    }

    /// Create with default configuration
    pub async fn with_defaults() -> PiiResult<Self> {
        Self::new(PiiDetectionConfig::default()).await
    }

    /// Create pattern matcher with enterprise features
    pub fn with_enterprise_features(
        config: PiiDetectionConfig,
        _audit_logger: impl std::fmt::Debug,
        _metrics: impl std::fmt::Debug,
    ) -> PiiResult<Self> {
        config.validate()?;
        let compiled = Self::compile_patterns(&config)?;

        let matcher = Self {
            config: Arc::new(RwLock::new(config)),
            compiled_patterns: Arc::new(RwLock::new(compiled)),
            pattern_cache: Arc::new(RwLock::new(PatternCache::new())),
            performance_metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
            false_positive_patterns: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok(matcher)
    }

    /// Detect PII entities in text with full context
    #[instrument(skip(self, text, context))]
    pub async fn detect_pii_comprehensive(
        &self,
        text: &str,
        context: AnalysisContext,
    ) -> PiiResult<DetectionResult> {
        let start_time = Instant::now();
        let config = self.config.read().await;

        if !config.enabled {
            return Ok(DetectionResult {
                original_text: text.to_string(),
                entities: Vec::new(),
                processing_time: start_time.elapsed(),
                overall_sensitivity: SensitivityLevel::Public,
                compliance_status: ComplianceStatus::default(),
                redaction_applied: false,
                metadata: HashMap::new(),
            });
        }

        // Input validation
        self.validate_input(text, &config).await?;

        // Check cache first
        if config.performance_config.enable_caching {
            if let Some(cached_entities) = self.check_cache(text).await {
                let overall_sensitivity = self.calculate_overall_sensitivity(&cached_entities);
                let compliance_status = self.assess_compliance(&cached_entities, &config).await;

                return Ok(DetectionResult {
                    original_text: text.to_string(),
                    entities: cached_entities,
                    processing_time: start_time.elapsed(),
                    overall_sensitivity,
                    compliance_status,
                    redaction_applied: false,
                    metadata: HashMap::new(),
                });
            }
        }

        // Perform detection
        let mut entities = Vec::new();

        // Run different detection methods
        for method in &config.detection_engine.enabled_methods {
            match method {
                DetectionMethod::Regex => {
                    let regex_entities = self.detect_with_regex(text, &config).await?;
                    entities.extend(regex_entities);
                }
                DetectionMethod::ContextualAnalysis => {
                    let contextual_entities =
                        self.detect_with_context(text, &context, &config).await?;
                    entities.extend(contextual_entities);
                }
                DetectionMethod::ChecksumValidation => {
                    let validated_entities = self.validate_with_checksums(entities.clone()).await?;
                    entities = validated_entities;
                }
                DetectionMethod::Dictionary => {
                    let dict_entities = self.detect_with_dictionary(text, &config).await?;
                    entities.extend(dict_entities);
                }
                _ => {} // Other methods can be implemented
            }
        }

        // Remove false positives
        if config.detection_engine.enable_false_positive_reduction {
            entities = self.remove_false_positives(entities).await?;
        }

        // Deduplicate and merge overlapping entities
        entities = self.deduplicate_entities(entities);

        // Calculate overall sensitivity
        let overall_sensitivity = self.calculate_overall_sensitivity(&entities);

        // Assess compliance
        let compliance_status = self.assess_compliance(&entities, &config).await;

        // Cache results
        if config.performance_config.enable_caching {
            self.cache_results(text, &entities).await;
        }

        // Update performance metrics
        self.update_performance_metrics(&entities, start_time.elapsed()).await;

        let processing_time = start_time.elapsed();

        Ok(DetectionResult {
            original_text: text.to_string(),
            entities,
            processing_time,
            overall_sensitivity,
            compliance_status,
            redaction_applied: false,
            metadata: HashMap::new(),
        })
    }

    /// Detect PII entities in text (convenience method)
    ///
    /// This is a simplified version of `detect_pii_comprehensive` that returns
    /// just the detected entities without additional metadata. Uses default
    /// analysis context.
    pub async fn detect_pii(&self, text: &str) -> PiiResult<Vec<PiiEntity>> {
        let context = AnalysisContext::default();
        let result = self.detect_pii_comprehensive(text, context).await?;
        Ok(result.entities)
    }

    /// Redact PII entities in text (convenience method)
    ///
    /// Detects and redacts all PII entities in the provided text, replacing
    /// them with redaction markers in the format `[REDACTED:type]`.
    pub async fn redact_pii(&self, text: &str) -> PiiResult<String> {
        let entities = self.detect_pii(text).await?;

        if entities.is_empty() {
            return Ok(text.to_string());
        }

        let mut redacted = text.to_string();

        // Sort entities by start position in reverse order to maintain indices during
        // replacement
        let mut sorted_entities = entities;
        sorted_entities.sort_by(|a, b| b.start_position.cmp(&a.start_position));

        // Redact in reverse order to preserve string indices
        for entity in sorted_entities {
            let redaction = format!("[REDACTED:{}]", entity.entity_type);
            redacted.replace_range(entity.start_position..entity.end_position, &redaction);
        }

        Ok(redacted)
    }

    /// Legacy detect_pii_types method for backward compatibility
    pub async fn detect_pii_types(&self, text: &str) -> Vec<PiiType> {
        let context = AnalysisContext::default();
        match self.detect_pii_comprehensive(text, context).await {
            Ok(result) => result.entities.into_iter().map(|e| e.entity_type).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Validate input against security constraints
    async fn validate_input(&self, text: &str, config: &PiiDetectionConfig) -> PiiResult<()> {
        if !config.security_config.enable_input_validation {
            return Ok(());
        }

        let max_size = config.security_config.max_input_size_mb * 1024 * 1024;
        if text.len() > max_size {
            return Err(CommonError::validation(
                "input_size",
                format!("Input size {} exceeds maximum allowed size {}", text.len(), max_size),
            )
            .into());
        }

        // Check for suspicious patterns that might indicate malicious input
        if text.contains('\0') || text.len() != text.chars().count() {
            warn!("Suspicious input detected - may contain null bytes or invalid UTF-8");
        }

        Ok(())
    }

    /// Check pattern cache for previous results
    async fn check_cache(&self, text: &str) -> Option<Vec<PiiEntity>> {
        let cache_key = self.generate_cache_key(text);
        let mut cache = self.pattern_cache.write().await;
        cache.get(&cache_key)
    }

    /// Generate cache key for text
    fn generate_cache_key(&self, text: &str) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Detect PII using regex patterns
    async fn detect_with_regex(
        &self,
        text: &str,
        config: &PiiDetectionConfig,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        // Process each enabled PII type
        for (pii_type, pattern_config) in &config.pattern_configs {
            if !pattern_config.enabled {
                continue;
            }

            let compiled = self.compiled_patterns_for(pii_type).await;

            // Use optimized static patterns for common types
            match pii_type {
                PiiType::Email => {
                    entities
                        .extend(self.detect_emails(text, pattern_config, compiled.clone()).await?);
                }
                PiiType::Phone => {
                    entities
                        .extend(self.detect_phones(text, pattern_config, compiled.clone()).await?);
                }
                PiiType::Ssn => {
                    entities
                        .extend(self.detect_ssns(text, pattern_config, compiled.clone()).await?);
                }
                PiiType::CreditCard => {
                    entities.extend(
                        self.detect_credit_cards(text, pattern_config, compiled.clone()).await?,
                    );
                }
                PiiType::IpAddress => {
                    entities.extend(
                        self.detect_ip_addresses(text, pattern_config, compiled.clone()).await?,
                    );
                }
                _ => {
                    // Use custom patterns for other types
                    entities.extend(
                        self.detect_with_custom_patterns(text, pattern_config, compiled.clone())
                            .await?,
                    );
                }
            }
        }

        Ok(entities)
    }

    /// Detect email addresses
    async fn detect_emails(
        &self,
        text: &str,
        config: &PatternConfig,
        compiled: Option<Arc<CompiledPatternSet>>,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        let regex_sources =
            Self::resolve_primary_regexes(compiled.as_deref(), config, Some(&*EMAIL_REGEX))?;

        for regex in &regex_sources {
            for mat in regex.find_iter(text) {
                let email = mat.as_str();

                // Apply exclusion patterns
                if config.exclusion_patterns.iter().any(|pattern| email.contains(pattern)) {
                    continue;
                }

                // Basic email validation
                let confidence = if self.is_valid_email(email) {
                    ConfidenceScore::new(0.9)
                } else {
                    ConfidenceScore::new(0.6)
                };

                if confidence.value() >= config.minimum_confidence.value() {
                    entities.push(PiiEntity {
                        entity_type: PiiType::Email,
                        value: email.to_string(),
                        start_position: mat.start(),
                        end_position: mat.end(),
                        confidence,
                        sensitivity_level: config.sensitivity_level,
                        context: self.extract_context(text, mat.start(), mat.end()),
                        metadata: HashMap::new(),
                        detection_method: DetectionMethod::Regex,
                        compliance_tags: config
                            .compliance_frameworks
                            .iter()
                            .map(|f| format!("{:?}", f))
                            .collect(),
                    });
                }
            }
        }

        Ok(entities)
    }

    /// Detect phone numbers
    async fn detect_phones(
        &self,
        text: &str,
        config: &PatternConfig,
        compiled: Option<Arc<CompiledPatternSet>>,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        let regex_sources = Self::resolve_primary_regexes(compiled.as_deref(), config, None)?;

        if regex_sources.is_empty() {
            return Ok(entities);
        }

        for regex in &regex_sources {
            for mat in regex.find_iter(text) {
                let phone = mat.as_str();

                // Apply exclusion patterns
                if config.exclusion_patterns.iter().any(|pattern| phone.contains(pattern)) {
                    continue;
                }

                let confidence = ConfidenceScore::new(0.8);

                if confidence.value() >= config.minimum_confidence.value() {
                    entities.push(PiiEntity {
                        entity_type: PiiType::Phone,
                        value: phone.to_string(),
                        start_position: mat.start(),
                        end_position: mat.end(),
                        confidence,
                        sensitivity_level: config.sensitivity_level,
                        context: self.extract_context(text, mat.start(), mat.end()),
                        metadata: HashMap::new(),
                        detection_method: DetectionMethod::Regex,
                        compliance_tags: config
                            .compliance_frameworks
                            .iter()
                            .map(|f| format!("{:?}", f))
                            .collect(),
                    });
                }
            }
        }

        Ok(entities)
    }

    /// Detect SSNs with validation
    async fn detect_ssns(
        &self,
        text: &str,
        config: &PatternConfig,
        compiled: Option<Arc<CompiledPatternSet>>,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        let regex_sources =
            Self::resolve_primary_regexes(compiled.as_deref(), config, Some(&*SSN_REGEX))?;

        for regex in &regex_sources {
            for mat in regex.find_iter(text) {
                let ssn = mat.as_str();

                // Apply exclusion patterns
                if config.exclusion_patterns.iter().any(|pattern| ssn.contains(pattern)) {
                    continue;
                }

                // Validate SSN format and checksum
                let confidence = if self.is_valid_ssn(ssn) {
                    ConfidenceScore::new(0.95)
                } else {
                    ConfidenceScore::new(0.7)
                };

                if confidence.value() >= config.minimum_confidence.value() {
                    entities.push(PiiEntity {
                        entity_type: PiiType::Ssn,
                        value: ssn.to_string(),
                        start_position: mat.start(),
                        end_position: mat.end(),
                        confidence,
                        sensitivity_level: config.sensitivity_level,
                        context: self.extract_context(text, mat.start(), mat.end()),
                        metadata: HashMap::new(),
                        detection_method: DetectionMethod::Regex,
                        compliance_tags: config
                            .compliance_frameworks
                            .iter()
                            .map(|f| format!("{:?}", f))
                            .collect(),
                    });
                }
            }
        }

        Ok(entities)
    }

    /// Detect credit cards with Luhn validation
    async fn detect_credit_cards(
        &self,
        text: &str,
        config: &PatternConfig,
        compiled: Option<Arc<CompiledPatternSet>>,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        let regex_sources = Self::resolve_primary_regexes(compiled.as_deref(), config, None)?;

        for regex in &regex_sources {
            for mat in regex.find_iter(text) {
                let card_number = mat.as_str().replace(&[' ', '-'][..], "");

                // Apply exclusion patterns
                if config.exclusion_patterns.iter().any(|pattern| card_number.contains(pattern)) {
                    continue;
                }

                // Validate using Luhn algorithm
                let confidence = if self.luhn_check(&card_number) {
                    ConfidenceScore::new(0.95)
                } else {
                    ConfidenceScore::new(0.5)
                };

                if confidence.value() >= config.minimum_confidence.value() {
                    entities.push(PiiEntity {
                        entity_type: PiiType::CreditCard,
                        value: mat.as_str().to_string(),
                        start_position: mat.start(),
                        end_position: mat.end(),
                        confidence,
                        sensitivity_level: config.sensitivity_level,
                        context: self.extract_context(text, mat.start(), mat.end()),
                        metadata: HashMap::new(),
                        detection_method: DetectionMethod::ChecksumValidation,
                        compliance_tags: config
                            .compliance_frameworks
                            .iter()
                            .map(|f| format!("{:?}", f))
                            .collect(),
                    });
                }
            }
        }

        Ok(entities)
    }

    /// Detect IP addresses
    async fn detect_ip_addresses(
        &self,
        text: &str,
        config: &PatternConfig,
        compiled: Option<Arc<CompiledPatternSet>>,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        let regex_sources =
            Self::resolve_primary_regexes(compiled.as_deref(), config, Some(&*IP_ADDRESS_REGEX))?;

        for regex in &regex_sources {
            for mat in regex.find_iter(text) {
                let ip = mat.as_str();

                // Apply exclusion patterns
                if config.exclusion_patterns.iter().any(|pattern| ip.contains(pattern)) {
                    continue;
                }

                // Validate IP address format
                let confidence = if self.is_valid_ip(ip) {
                    ConfidenceScore::new(0.9)
                } else {
                    ConfidenceScore::new(0.6)
                };

                if confidence.value() >= config.minimum_confidence.value() {
                    entities.push(PiiEntity {
                        entity_type: PiiType::IpAddress,
                        value: ip.to_string(),
                        start_position: mat.start(),
                        end_position: mat.end(),
                        confidence,
                        sensitivity_level: config.sensitivity_level,
                        context: self.extract_context(text, mat.start(), mat.end()),
                        metadata: HashMap::new(),
                        detection_method: DetectionMethod::Regex,
                        compliance_tags: config
                            .compliance_frameworks
                            .iter()
                            .map(|f| format!("{:?}", f))
                            .collect(),
                    });
                }
            }
        }

        Ok(entities)
    }

    /// Detect with custom patterns
    async fn detect_with_custom_patterns(
        &self,
        text: &str,
        config: &PatternConfig,
        compiled: Option<Arc<CompiledPatternSet>>,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        let regex_sources = Self::resolve_primary_regexes(compiled.as_deref(), config, None)?;

        for regex in &regex_sources {
            for mat in regex.find_iter(text) {
                let value = mat.as_str();

                // Apply exclusion patterns
                if config.exclusion_patterns.iter().any(|excl_pattern| value.contains(excl_pattern))
                {
                    continue;
                }

                let confidence = ConfidenceScore::new(0.7); // Default confidence for custom patterns

                if confidence.value() >= config.minimum_confidence.value() {
                    entities.push(PiiEntity {
                        entity_type: config.pattern_type.clone(),
                        value: value.to_string(),
                        start_position: mat.start(),
                        end_position: mat.end(),
                        confidence,
                        sensitivity_level: config.sensitivity_level,
                        context: self.extract_context(text, mat.start(), mat.end()),
                        metadata: HashMap::new(),
                        detection_method: DetectionMethod::Regex,
                        compliance_tags: config
                            .compliance_frameworks
                            .iter()
                            .map(|f| format!("{:?}", f))
                            .collect(),
                    });
                }
            }
        }

        Ok(entities)
    }

    /// Detect with contextual analysis
    async fn detect_with_context(
        &self,
        text: &str,
        context: &AnalysisContext,
        config: &PiiDetectionConfig,
    ) -> PiiResult<Vec<PiiEntity>> {
        let mut entities = Vec::new();

        for (pii_type, pattern_config) in &config.pattern_configs {
            if !pattern_config.enabled {
                continue;
            }

            let compiled = self.compiled_patterns_for(pii_type).await;

            let primary_regexes =
                Self::resolve_primary_regexes(compiled.as_deref(), pattern_config, None)?;
            let context_regexes =
                Self::resolve_context_regexes(compiled.as_deref(), pattern_config)?;

            if primary_regexes.is_empty() || context_regexes.is_empty() {
                continue;
            }

            for context_regex in &context_regexes {
                for context_match in context_regex.find_iter(text) {
                    let (search_start, search_end) = Self::context_window_bounds(
                        text,
                        context_match.start(),
                        context_match.end(),
                    );

                    if search_start >= search_end {
                        continue;
                    }

                    let search_text = &text[search_start..search_end];

                    for regex in &primary_regexes {
                        for mat in regex.find_iter(search_text) {
                            let absolute_start = search_start + mat.start();
                            let absolute_end = search_start + mat.end();

                            if absolute_start >= absolute_end || absolute_end > text.len() {
                                continue;
                            }

                            let value = &text[absolute_start..absolute_end];

                            let base_confidence = 0.6;
                            let context_boost = 0.2;
                            let confidence = ConfidenceScore::new(base_confidence + context_boost);

                            if confidence.value() >= pattern_config.minimum_confidence.value() {
                                let mut metadata = HashMap::new();
                                if let Some(ref doc_type) = context.document_type {
                                    metadata.insert("document_type".to_string(), doc_type.clone());
                                }
                                if let Some(ref source) = context.source_application {
                                    metadata
                                        .insert("source_application".to_string(), source.clone());
                                }
                                if let Some(ref jurisdiction) = context.jurisdiction {
                                    metadata
                                        .insert("jurisdiction".to_string(), jurisdiction.clone());
                                }
                                if let Some(ref classification) = context.data_classification {
                                    metadata.insert(
                                        "data_classification".to_string(),
                                        classification.clone(),
                                    );
                                }
                                if let Some(ref purpose) = context.processing_purpose {
                                    metadata
                                        .insert("processing_purpose".to_string(), purpose.clone());
                                }

                                entities.push(PiiEntity {
                                    entity_type: pii_type.clone(),
                                    value: value.to_string(),
                                    start_position: absolute_start,
                                    end_position: absolute_end,
                                    confidence,
                                    sensitivity_level: pattern_config.sensitivity_level,
                                    context: self.extract_context(
                                        text,
                                        absolute_start,
                                        absolute_end,
                                    ),
                                    metadata,
                                    detection_method: DetectionMethod::ContextualAnalysis,
                                    compliance_tags: pattern_config
                                        .compliance_frameworks
                                        .iter()
                                        .map(|f| format!("{:?}", f))
                                        .collect(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(entities)
    }

    /// Detect with dictionary matching
    async fn detect_with_dictionary(
        &self,
        _text: &str,
        _config: &PiiDetectionConfig,
    ) -> PiiResult<Vec<PiiEntity>> {
        // Dictionary-based detection would be implemented here
        // For now, return empty results
        Ok(Vec::new())
    }

    /// Validate entities using checksums
    async fn validate_with_checksums(&self, entities: Vec<PiiEntity>) -> PiiResult<Vec<PiiEntity>> {
        let mut validated_entities = Vec::new();

        for mut entity in entities {
            let is_valid = match entity.entity_type {
                PiiType::CreditCard => {
                    let card_number = entity.value.replace(&[' ', '-'][..], "");
                    self.luhn_check(&card_number)
                }
                PiiType::Ssn => self.is_valid_ssn(&entity.value),
                PiiType::IpAddress => self.is_valid_ip(&entity.value),
                PiiType::Email => self.is_valid_email(&entity.value),
                _ => true, // No validation for other types
            };

            if is_valid {
                // Boost confidence for validated entities
                entity.confidence =
                    ConfidenceScore::new((entity.confidence.value() + 0.1).min(1.0));
                entity.detection_method = DetectionMethod::ChecksumValidation;
                validated_entities.push(entity);
            } else if entity.confidence.value() >= 0.8 {
                // Keep high-confidence entities even if validation fails
                entity.confidence = ConfidenceScore::new(entity.confidence.value() - 0.2);
                validated_entities.push(entity);
            }
        }

        Ok(validated_entities)
    }

    /// Remove false positive entities
    async fn remove_false_positives(&self, entities: Vec<PiiEntity>) -> PiiResult<Vec<PiiEntity>> {
        let false_positive_patterns = self.false_positive_patterns.read().await;
        let mut filtered_entities = Vec::new();

        for entity in entities {
            let mut is_false_positive = false;

            if let Some(patterns) = false_positive_patterns.get(&entity.entity_type) {
                for pattern in patterns {
                    if pattern.is_match(&entity.value) {
                        is_false_positive = true;
                        break;
                    }
                }
            }

            // Additional heuristic-based false positive detection
            if !is_false_positive {
                is_false_positive = self.is_heuristic_false_positive(&entity);
            }

            if !is_false_positive {
                filtered_entities.push(entity);
            }
        }

        Ok(filtered_entities)
    }

    /// Heuristic false positive detection
    fn is_heuristic_false_positive(&self, entity: &PiiEntity) -> bool {
        match entity.entity_type {
            PiiType::Phone => {
                // Reject obviously fake phone numbers
                let digits_only: String =
                    entity.value.chars().filter(|c| c.is_ascii_digit()).collect();
                digits_only == "0000000000" || digits_only == "1111111111" || digits_only.len() < 10
            }
            PiiType::Email => {
                // Reject common test emails or malformed emails
                entity.value.contains("test@test")
                    || entity.value.contains("example@example")
                    || !entity.value.contains('.')
            }
            PiiType::CreditCard => {
                // Reject obviously fake card numbers
                let digits_only: String =
                    entity.value.chars().filter(|c| c.is_ascii_digit()).collect();
                digits_only.chars().all(|c| c == '0') || digits_only.chars().all(|c| c == '1')
            }
            _ => false,
        }
    }

    /// Deduplicate overlapping entities
    fn deduplicate_entities(&self, mut entities: Vec<PiiEntity>) -> Vec<PiiEntity> {
        // Sort by start position
        entities.sort_by_key(|e| e.start_position);

        let mut deduplicated = Vec::new();
        let mut i = 0;

        while i < entities.len() {
            let current = &entities[i];
            let mut best_entity = current.clone();

            // Look for overlapping entities
            let mut j = i + 1;
            while j < entities.len() && entities[j].start_position < current.end_position {
                if entities[j].confidence.value() > best_entity.confidence.value() {
                    best_entity = entities[j].clone();
                }
                j += 1;
            }

            deduplicated.push(best_entity);
            i = j;
        }

        deduplicated
    }

    /// Calculate overall sensitivity level
    fn calculate_overall_sensitivity(&self, entities: &[PiiEntity]) -> SensitivityLevel {
        if entities.is_empty() {
            return SensitivityLevel::Public;
        }

        let max_sensitivity =
            entities.iter().map(|e| e.sensitivity_level).max().unwrap_or(SensitivityLevel::Public);

        max_sensitivity
    }

    /// Assess compliance status
    async fn assess_compliance(
        &self,
        entities: &[PiiEntity],
        config: &PiiDetectionConfig,
    ) -> ComplianceStatus {
        let mut frameworks = Vec::new();
        let violations = Vec::new();
        let mut recommendations = Vec::new();

        // Check each enabled compliance framework
        for framework in &config.compliance_config.enabled_frameworks {
            frameworks.push(framework.clone());

            match framework {
                ComplianceFramework::Gdpr => {
                    // GDPR compliance checks
                    if entities.iter().any(|e| {
                        matches!(
                            e.sensitivity_level,
                            SensitivityLevel::Restricted | SensitivityLevel::TopSecret
                        )
                    }) {
                        recommendations.push(
                            "Consider data minimization under GDPR Article 5(1)(c)".to_string(),
                        );
                    }
                }
                ComplianceFramework::Hipaa => {
                    // HIPAA compliance checks
                    if entities.iter().any(|e| e.entity_type == PiiType::MedicalRecord) {
                        recommendations
                            .push("Ensure PHI protection under HIPAA requirements".to_string());
                    }
                }
                ComplianceFramework::Pci => {
                    // PCI DSS compliance checks
                    if entities.iter().any(|e| e.entity_type == PiiType::CreditCard) {
                        recommendations
                            .push("Apply PCI DSS data protection requirements".to_string());
                    }
                }
                _ => {}
            }
        }

        // Calculate risk score based on sensitivity and entity count
        let risk_score = self.calculate_risk_score(entities);

        ComplianceStatus { frameworks, violations, recommendations, risk_score }
    }

    /// Calculate risk score
    fn calculate_risk_score(&self, entities: &[PiiEntity]) -> f64 {
        if entities.is_empty() {
            return 0.0;
        }

        let base_score = entities.len() as f64 * 0.1;
        let sensitivity_multiplier = entities
            .iter()
            .map(|e| match e.sensitivity_level {
                SensitivityLevel::Public => 0.1,
                SensitivityLevel::Internal => 0.3,
                SensitivityLevel::Confidential => 0.6,
                SensitivityLevel::Restricted => 0.8,
                SensitivityLevel::TopSecret => 1.0,
            })
            .sum::<f64>()
            / entities.len() as f64;

        (base_score * sensitivity_multiplier).min(1.0)
    }

    /// Extract context around a match
    fn extract_context(&self, text: &str, start: usize, end: usize) -> String {
        let context_size = 50;
        let context_start = start.saturating_sub(context_size);
        let context_end = (end + context_size).min(text.len());

        text[context_start..context_end].to_string()
    }

    /// Cache detection results
    async fn cache_results(&self, text: &str, entities: &[PiiEntity]) {
        let cache_key = self.generate_cache_key(text);
        let mut cache = self.pattern_cache.write().await;

        cache.insert(cache_key, entities.to_vec());
    }

    /// Update performance metrics
    async fn update_performance_metrics(&self, entities: &[PiiEntity], processing_time: Duration) {
        let mut metrics = self.performance_metrics.write().await;
        metrics.total_patterns_checked += 1;
        metrics.patterns_matched += entities.len();
        metrics.processing_time_ms = processing_time.as_millis() as u64;
    }

    /// Validation helper methods
    fn is_valid_email(&self, email: &str) -> bool {
        email.contains('@') && email.contains('.') && email.len() > 5 && email.len() < 100
    }

    fn is_valid_ssn(&self, ssn: &str) -> bool {
        let digits: String = ssn.chars().filter(|c| c.is_ascii_digit()).collect();
        digits.len() == 9
            && !digits.starts_with("000")
            && !digits[3..5].eq("00")
            && !digits[5..9].eq("0000")
    }

    fn is_valid_ip(&self, ip: &str) -> bool {
        let parts: Vec<&str> = ip.split('.').collect();
        parts.len() == 4 && parts.iter().all(|part| part.parse::<u32>().is_ok_and(|n| n <= 255))
    }

    fn luhn_check(&self, number: &str) -> bool {
        let digits: Vec<u32> = number.chars().filter_map(|c| c.to_digit(10)).collect();

        if digits.len() < 13 || digits.len() > 19 {
            return false;
        }

        let sum: u32 = digits
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &digit)| {
                if i % 2 == 1 {
                    let doubled = digit * 2;
                    if doubled > 9 {
                        doubled - 9
                    } else {
                        doubled
                    }
                } else {
                    digit
                }
            })
            .sum();

        sum.is_multiple_of(10)
    }

    /// Get configuration
    pub async fn get_config(&self) -> PiiDetectionConfig {
        self.config.read().await.clone()
    }

    /// Update configuration
    pub async fn update_config(&self, config: PiiDetectionConfig) -> PiiResult<()> {
        config.validate()?;
        let compiled = Self::compile_patterns(&config)?;

        {
            let mut config_guard = self.config.write().await;
            let mut compiled_guard = self.compiled_patterns.write().await;
            *config_guard = config;
            *compiled_guard = compiled;
        }

        // Clear cache when configuration changes
        self.pattern_cache.write().await.clear();

        Ok(())
    }

    /// Get performance metrics
    pub async fn get_performance_metrics(&self) -> PerformanceMetrics {
        self.performance_metrics.read().await.clone()
    }

    /// Validate enterprise compliance configuration
    pub async fn validate_enterprise_compliance(&self) -> PiiResult<()> {
        // Perform basic validation checks for enterprise compliance
        let config = self.config.read().await;

        // Check if required compliance frameworks are configured
        if config.compliance_config.enabled_frameworks.is_empty() {
            return Err(CommonError::validation(
                "compliance_frameworks",
                "No compliance frameworks configured for enterprise deployment",
            )
            .into());
        }

        // Validate that patterns are loaded for enterprise features
        let patterns = self.compiled_patterns.read().await;
        if patterns.is_empty() {
            return Err(CommonError::validation(
                "compiled_patterns",
                "No patterns compiled for enterprise compliance validation",
            )
            .into());
        }

        // Additional enterprise-specific validations can be added here
        Ok(())
    }
}

// Note: Default trait removed as with_defaults() is async

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use chrono::Utc;

    use super::*;

    fn sample_entity(value: &str) -> PiiEntity {
        PiiEntity {
            entity_type: PiiType::Email,
            value: value.to_string(),
            start_position: 0,
            end_position: value.len(),
            confidence: ConfidenceScore::new(0.9),
            sensitivity_level: SensitivityLevel::Confidential,
            context: "context".to_string(),
            metadata: HashMap::new(),
            detection_method: DetectionMethod::Regex,
            compliance_tags: Vec::new(),
        }
    }

    /// Validates `PatternCache::new` behavior for the cache accounts for
    /// multibyte bytes scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.byte_usage() >= key.len() + "テスト@example.com".len()`
    ///   evaluates to true.
    /// - Confirms `cache.entry_count()` equals `1`.
    #[test]
    fn cache_accounts_for_multibyte_bytes() {
        let mut cache = PatternCache::new();
        let key = "multibyte-entry";
        let entity = sample_entity("テスト@example.com");

        cache.insert(key.to_string(), vec![entity]);

        assert!(cache.byte_usage() >= key.len() + "テスト@example.com".len());
        assert_eq!(cache.entry_count(), 1);
    }

    /// Validates `PatternCache::new` behavior for the cache enforces entry
    /// limit scenario.
    ///
    /// Assertions:
    /// - Ensures `cache.entry_count() <= CACHE_MAX_ENTRIES` evaluates to true.
    /// - Confirms `cache.entry_count()` equals `cache.order_len()`.
    #[test]
    fn cache_enforces_entry_limit() {
        let mut cache = PatternCache::new();
        let entity = sample_entity("test@example.com");

        for idx in 0..(CACHE_MAX_ENTRIES + 100) {
            let key = format!("entry-{idx}");
            cache.insert(key, vec![entity.clone()]);
        }

        assert!(cache.entry_count() <= CACHE_MAX_ENTRIES);
        assert_eq!(cache.entry_count(), cache.order_len());
    }

    /// Validates `PatternMatcher::with_defaults` behavior for the detects
    /// emails in multibyte text without panic scenario.
    ///
    /// Assertions:
    /// - Ensures `result.entities.iter().any(|entity| entity.entity_type ==
    ///   PiiType::Email)` evaluates to true.
    #[tokio::test]
    async fn detects_emails_in_multibyte_text_without_panic() {
        let matcher = PatternMatcher::with_defaults().await.expect("matcher init");
        let analysis_context = AnalysisContext {
            document_type: Some("レポート".into()),
            source_application: Some("integration-test".into()),
            user_id: None,
            department: None,
            data_classification: Some("confidential".into()),
            jurisdiction: Some("JP".into()),
            timestamp: Utc::now(),
            session_id: "session-123".into(),
            compliance_zone: None,
            processing_purpose: Some("qa".into()),
            retention_policy: None,
        };

        let text = "顧客メール email: 顧客@example.com に連絡してください。";

        let result = matcher
            .detect_pii_comprehensive(text, analysis_context)
            .await
            .expect("detection succeeds");

        assert!(result.entities.iter().any(|entity| entity.entity_type == PiiType::Email));
    }
}
