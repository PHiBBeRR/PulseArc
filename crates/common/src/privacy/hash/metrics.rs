// Hash-specific metrics integration with existing telemetry system
// Extends the existing metrics collection for hash operations

// Note: enterprise and telemetry modules are not part of common crate
// use super::enterprise::KdfAlgorithm;
use std::time::{Duration, SystemTime};

// Note: telemetry is provided by a separate crate
// use crate::telemetry::collector::metrics::METRICS_ERRORS;
// use crate::telemetry::manager::SpanManager;
// Simplified metrics without telemetry dependency
use lazy_static::lazy_static;
use prometheus::{
    register_counter_vec, register_gauge_vec, register_histogram_vec, CounterVec, GaugeVec,
    HistogramVec, TextEncoder,
};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, instrument};

use super::error::{HashError, HashResult};

lazy_static! {
    /// Hash operation latency histogram
    pub static ref HASH_OPERATION_DURATION: HistogramVec = register_histogram_vec!(
        "hash_operation_duration_seconds",
        "Duration of hash operations in seconds",
        &["algorithm", "compliance_mode", "operation_type"],
        vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
    ).expect("Failed to register hash_operation_duration histogram");

    /// Hash operation counter
    pub static ref HASH_OPERATIONS_TOTAL: CounterVec = register_counter_vec!(
        "hash_operations_total",
        "Total number of hash operations",
        &["algorithm", "compliance_mode", "status"]
    ).expect("Failed to register hash_operations_total counter");

    /// Salt rotation counter
    pub static ref SALT_ROTATIONS_TOTAL: CounterVec = register_counter_vec!(
        "salt_rotations_total",
        "Total number of salt rotations",
        &["trigger_reason", "compliance_mode"]
    ).expect("Failed to register salt_rotations_total counter");

    /// Current salt age gauge
    pub static ref SALT_AGE_HOURS: GaugeVec = register_gauge_vec!(
        "salt_age_hours",
        "Current salt age in hours",
        &["hasher_id", "compliance_mode"]
    ).expect("Failed to register salt_age_hours gauge");

    /// Security policy violations
    pub static ref SECURITY_VIOLATIONS: CounterVec = register_counter_vec!(
        "hash_security_violations_total",
        "Total number of security policy violations",
        &["violation_type", "compliance_mode"]
    ).expect("Failed to register security_violations counter");

    /// KDF parameter metrics
    pub static ref KDF_MEMORY_COST: GaugeVec = register_gauge_vec!(
        "kdf_memory_cost_kb",
        "KDF memory cost in KB",
        &["algorithm", "compliance_mode"]
    ).expect("Failed to register kdf_memory_cost gauge");

    pub static ref KDF_TIME_COST: GaugeVec = register_gauge_vec!(
        "kdf_time_cost_iterations",
        "KDF time cost in iterations",
        &["algorithm", "compliance_mode"]
    ).expect("Failed to register kdf_time_cost gauge");

    /// Compliance status gauge
    pub static ref COMPLIANCE_STATUS: GaugeVec = register_gauge_vec!(
        "hash_compliance_status",
        "Compliance status (1=compliant, 0=non-compliant)",
        &["compliance_mode", "check_type"]
    ).expect("Failed to register compliance_status gauge");
}

/// Hash operation performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HashPerformanceMetrics {
    pub operation_id: String,
    pub algorithm: String,
    pub compliance_mode: String,
    pub duration_ms: u64,
    pub input_size_bytes: usize,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub timestamp: SystemTime,
}

/// Salt management metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaltMetrics {
    pub hasher_id: String,
    pub age_hours: u64,
    pub rotation_count: u32,
    pub last_rotation: SystemTime,
    pub algorithm: String, // Changed from KdfAlgorithm (enterprise module not in common)
    pub compliance_mode: String,
}

/// Security metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    pub policy_violations: u32,
    pub failed_operations: u32,
    pub rate_limit_hits: u32,
    pub weak_input_rejections: u32,
    pub compliance_failures: u32,
}

/// Parameters for recording hash operations
#[derive(Debug, Clone)]
pub struct HashOperationParams {
    pub operation_id: String,
    pub algorithm: String,
    pub compliance_mode: String,
    pub duration: Duration,
    pub input_size: usize,
}

/// Compliance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceMetrics {
    pub mode: String,
    pub last_audit: SystemTime,
    pub violations: Vec<String>,
    pub health_score: f64,
    pub certification_status: String,
}

/// Hash metrics collector
///
/// Collects and aggregates hashing performance and security metrics.
/// Note: Telemetry integration removed - not part of common crate
pub struct HashMetricsCollector {
    // Note: telemetry_manager removed - telemetry module not in common crate
    // telemetry_manager: Option<SpanManager>,
    performance_history: Vec<HashPerformanceMetrics>,
    /// Security events for monitoring and alerting
    #[allow(dead_code)] // TODO: Implement security event tracking
    security_events: Vec<SecurityMetrics>,
    /// Compliance status tracking
    #[allow(dead_code)] // TODO: Implement compliance status tracking
    compliance_status: Vec<ComplianceMetrics>,
    /// How often to collect and aggregate metrics
    #[allow(dead_code)] // TODO: Implement periodic collection
    collection_interval: Duration,
}

impl HashMetricsCollector {
    /// Create new metrics collector
    pub fn new(collection_interval: Duration) -> Self {
        Self {
            performance_history: Vec::new(),
            security_events: Vec::new(),
            compliance_status: Vec::new(),
            collection_interval,
        }
    }

    /// Initialize with telemetry manager
    pub async fn initialize(&mut self) -> HashResult<()> {
        // Initialization simplified
        info!("Hash metrics collector initialized");
        Ok(())
    }

    /// Record hash operation metrics
    #[instrument(skip(self))]
    pub fn record_operation(&mut self, params: HashOperationParams) {
        let HashOperationParams { operation_id, algorithm, compliance_mode, duration, input_size } =
            params;
        let duration_seconds = duration.as_secs_f64();

        // Record Prometheus metrics
        let algorithm_str = algorithm.to_string();
        let compliance_mode_str = compliance_mode.to_string();
        let operation_str = "hash".to_string();
        HASH_OPERATION_DURATION
            .with_label_values(&[&algorithm_str, &compliance_mode_str, &operation_str])
            .observe(duration_seconds);

        let success_str = "success".to_string();
        HASH_OPERATIONS_TOTAL
            .with_label_values(&[&algorithm_str, &compliance_mode_str, &success_str])
            .inc();

        // Store detailed metrics
        let performance_metric = HashPerformanceMetrics {
            operation_id,
            algorithm: algorithm.to_string(),
            compliance_mode: compliance_mode.to_string(),
            duration_ms: duration.as_millis() as u64,
            input_size_bytes: input_size,
            memory_usage_mb: Self::get_memory_usage(),
            cpu_usage_percent: Self::get_cpu_usage(),
            timestamp: SystemTime::now(),
        };

        self.performance_history.push(performance_metric);

        // Trim history to prevent memory growth
        if self.performance_history.len() > 10000 {
            self.performance_history.drain(0..1000);
        }

        debug!("Recorded hash operation metrics for {}", algorithm);
    }

    /// Record operation failure
    pub fn record_failure(&mut self, algorithm: &str, compliance_mode: &str, error_type: &str) {
        let algorithm_str = algorithm.to_string();
        let compliance_mode_str = compliance_mode.to_string();
        let failure_str = "failure".to_string();
        HASH_OPERATIONS_TOTAL
            .with_label_values(&[&algorithm_str, &compliance_mode_str, &failure_str])
            .inc();

        // Note: METRICS_ERRORS is part of the central telemetry system
        // which is not available in the common crate. Using SECURITY_VIOLATIONS
        // instead.
        let error_type_str = error_type.to_string();
        SECURITY_VIOLATIONS.with_label_values(&[&error_type_str, &compliance_mode_str]).inc();

        debug!("Recorded hash operation failure: {}", error_type);
    }

    /// Record salt rotation
    pub fn record_salt_rotation(
        &mut self,
        hasher_id: &str,
        compliance_mode: &str,
        trigger_reason: &str,
    ) {
        let trigger_reason_str = trigger_reason.to_string();
        let compliance_mode_str = compliance_mode.to_string();
        SALT_ROTATIONS_TOTAL.with_label_values(&[&trigger_reason_str, &compliance_mode_str]).inc();

        let hasher_id_str = hasher_id.to_string();
        SALT_AGE_HOURS.with_label_values(&[&hasher_id_str, &compliance_mode_str]).set(0.0); // Reset to 0 after rotation

        info!("Recorded salt rotation for hasher {}: {}", hasher_id, trigger_reason);
    }

    /// Update salt age metrics
    pub fn update_salt_age(&self, hasher_id: &str, compliance_mode: &str, age_hours: u64) {
        let hasher_id_str = hasher_id.to_string();
        let compliance_mode_str = compliance_mode.to_string();
        SALT_AGE_HOURS
            .with_label_values(&[&hasher_id_str, &compliance_mode_str])
            .set(age_hours as f64);
    }

    /// Record security violation
    pub fn record_security_violation(
        &mut self,
        violation_type: &str,
        compliance_mode: &str,
        details: &str,
    ) {
        let violation_type_str = violation_type.to_string();
        let compliance_mode_str = compliance_mode.to_string();
        SECURITY_VIOLATIONS.with_label_values(&[&violation_type_str, &compliance_mode_str]).inc();

        info!(
            "Recorded security violation: {} in {} - Details: {}",
            violation_type, compliance_mode, details
        );
    }

    /// Update KDF parameters metrics
    pub fn update_kdf_metrics(
        &self,
        algorithm: &str,
        compliance_mode: &str,
        memory_cost: u32,
        time_cost: u32,
    ) {
        let algorithm_str = algorithm.to_string();
        let compliance_mode_str = compliance_mode.to_string();
        KDF_MEMORY_COST
            .with_label_values(&[&algorithm_str, &compliance_mode_str])
            .set(memory_cost as f64);

        KDF_TIME_COST
            .with_label_values(&[&algorithm_str, &compliance_mode_str])
            .set(time_cost as f64);
    }

    /// Update compliance status
    pub fn update_compliance_status(
        &self,
        compliance_mode: &str,
        check_type: &str,
        is_compliant: bool,
    ) {
        let status_value = if is_compliant { 1.0 } else { 0.0 };

        let compliance_mode_str = compliance_mode.to_string();
        let check_type_str = check_type.to_string();
        COMPLIANCE_STATUS
            .with_label_values(&[&compliance_mode_str, &check_type_str])
            .set(status_value);
    }

    /// Get performance summary
    pub fn get_performance_summary(&self, duration: Duration) -> PerformanceSummary {
        let cutoff = SystemTime::now() - duration;
        let recent_metrics: Vec<_> =
            self.performance_history.iter().filter(|m| m.timestamp >= cutoff).collect();

        if recent_metrics.is_empty() {
            return PerformanceSummary::default();
        }

        let total_operations = recent_metrics.len();
        let total_duration: u64 = recent_metrics.iter().map(|m| m.duration_ms).sum();
        let avg_duration = total_duration as f64 / total_operations as f64;

        let durations: Vec<u64> = recent_metrics.iter().map(|m| m.duration_ms).collect();
        let mut sorted_durations = durations.clone();
        sorted_durations.sort();

        let p50 = Self::percentile(&sorted_durations, 50.0);
        let p95 = Self::percentile(&sorted_durations, 95.0);
        let p99 = Self::percentile(&sorted_durations, 99.0);

        PerformanceSummary {
            total_operations,
            avg_duration_ms: avg_duration,
            p50_duration_ms: p50,
            p95_duration_ms: p95,
            p99_duration_ms: p99,
            operations_per_second: total_operations as f64 / duration.as_secs_f64(),
        }
    }

    /// Export metrics in Prometheus format
    pub fn export_prometheus_metrics(&self) -> HashResult<String> {
        let encoder = TextEncoder::new();
        let metric_families = prometheus::gather();

        encoder
            .encode_to_string(&metric_families)
            .map_err(|e| HashError::ConfigurationError(format!("Metrics export failed: {}", e)))
    }

    /// Get current hash metrics snapshot
    pub fn get_current_metrics(&self) -> HashMetricsSnapshot {
        HashMetricsSnapshot {
            performance: self.get_performance_summary(Duration::from_secs(3600)),
            security: self.get_security_summary(),
            compliance: self.get_compliance_summary(),
            timestamp: SystemTime::now(),
        }
    }

    /// Get security metrics summary
    fn get_security_summary(&self) -> SecuritySummary {
        // Implementation would aggregate security metrics
        SecuritySummary {
            total_violations: 0,
            rate_limit_hits: 0,
            weak_inputs_rejected: 0,
            policy_violations_by_type: std::collections::HashMap::new(),
        }
    }

    /// Get compliance metrics summary
    fn get_compliance_summary(&self) -> ComplianceSummary {
        // Implementation would aggregate compliance metrics
        ComplianceSummary {
            overall_score: 100.0,
            frameworks_compliant: vec!["GDPR".to_string()],
            frameworks_non_compliant: vec![],
            last_audit: SystemTime::now(),
        }
    }

    /// Calculate percentile
    fn percentile(sorted_values: &[u64], percentile: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }

        let index = (percentile / 100.0) * (sorted_values.len() - 1) as f64;
        let lower = index.floor() as usize;
        let upper = index.ceil() as usize;

        if lower == upper {
            sorted_values[lower] as f64
        } else {
            let weight = index - lower as f64;
            (sorted_values[lower] as f64) * (1.0 - weight) + (sorted_values[upper] as f64) * weight
        }
    }

    /// Get current memory usage (simplified)
    fn get_memory_usage() -> f64 {
        // Implementation would get actual memory usage
        0.0
    }

    /// Get current CPU usage (simplified)
    fn get_cpu_usage() -> f64 {
        // Implementation would get actual CPU usage
        0.0
    }
}

#[derive(Debug, Serialize)]
pub struct PerformanceSummary {
    pub total_operations: usize,
    pub avg_duration_ms: f64,
    pub p50_duration_ms: f64,
    pub p95_duration_ms: f64,
    pub p99_duration_ms: f64,
    pub operations_per_second: f64,
}

impl Default for PerformanceSummary {
    fn default() -> Self {
        Self {
            total_operations: 0,
            avg_duration_ms: 0.0,
            p50_duration_ms: 0.0,
            p95_duration_ms: 0.0,
            p99_duration_ms: 0.0,
            operations_per_second: 0.0,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SecuritySummary {
    pub total_violations: u32,
    pub rate_limit_hits: u32,
    pub weak_inputs_rejected: u32,
    pub policy_violations_by_type: std::collections::HashMap<String, u32>,
}

#[derive(Debug, Serialize)]
pub struct ComplianceSummary {
    pub overall_score: f64,
    pub frameworks_compliant: Vec<String>,
    pub frameworks_non_compliant: Vec<String>,
    pub last_audit: SystemTime,
}

#[derive(Debug, Serialize)]
pub struct HashMetricsSnapshot {
    pub performance: PerformanceSummary,
    pub security: SecuritySummary,
    pub compliance: ComplianceSummary,
    pub timestamp: SystemTime,
}
