//! Datadog DogStatsD metrics exporter
//!
//! Sends metrics to Datadog agent using raw UDP sockets (no external dependencies).
//! Implements the DogStatsD protocol for gauges, counters, histograms, and timers.
//!
//! ## Design
//! - **Raw UDP sockets** - No cadence dependency, lightweight implementation
//! - **Non-blocking** - Set to non-blocking mode to avoid blocking on send
//! - **Best-effort delivery** - UDP is fire-and-forget, no retry logic
//! - **Tag support** - DogStatsD tags for dimensional metrics
//! - **Float support** - Preserves floating-point precision for gauges/histograms
//!
//! ## DogStatsD Protocol
//! ```text
//! <METRIC_NAME>:<VALUE>|<TYPE>|@<SAMPLE_RATE>|#<TAG1>:<VALUE1>,<TAG2>:<VALUE2>
//! ```
//!
//! Examples:
//! - Gauge: `db.pool.utilization:0.75|g|#env:prod,service:timer`
//! - Counter: `db.connections.acquired:1|c|#env:prod`
//! - Histogram: `db.query.latency:123.45|h|#env:prod`
//! - Timer: `db.connection.acquired:45|ms|#env:prod`

use std::fmt::Display;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

use crate::observability::{metrics::DbStats, MetricsResult};

/// Default Datadog agent address (DogStatsD default port)
pub const DEFAULT_DATADOG_ADDR: &str = "127.0.0.1:8125";

/// Datadog DogStatsD client using raw UDP sockets
///
/// Thread-safe, non-blocking UDP socket for sending metrics to Datadog agent.
#[derive(Debug)]
pub struct DatadogClient {
    /// UDP socket for sending metrics
    socket: UdpSocket,
    /// Datadog agent address
    agent_addr: SocketAddr,
    /// Metric prefix (e.g., "pulsearc")
    prefix: String,
    /// Default tags applied to all metrics
    default_tags: Vec<String>,
}

impl DatadogClient {
    /// Create new Datadog client with default configuration
    ///
    /// Connects to localhost:8125 (DogStatsD default port).
    pub fn new() -> io::Result<Self> {
        Self::with_prefix("pulsearc")
    }

    /// Create new Datadog client with custom prefix
    ///
    /// Prefix is prepended to all metric names (e.g., "pulsearc.db.connections.acquired").
    pub fn with_prefix(prefix: &str) -> io::Result<Self> {
        let addr = DEFAULT_DATADOG_ADDR.to_socket_addrs()?.next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid Datadog address")
        })?;

        Self::with_config(prefix, addr)
    }

    /// Create new Datadog client with custom configuration
    pub fn with_config(prefix: &str, agent_addr: SocketAddr) -> io::Result<Self> {
        // Bind to any available port (OS will assign)
        let socket = UdpSocket::bind("0.0.0.0:0")?;

        // Set non-blocking to avoid blocking on send
        socket.set_nonblocking(true)?;

        // Default tags from environment
        let mut default_tags = Vec::new();
        if let Ok(env) = std::env::var("DD_ENV") {
            default_tags.push(format!("env:{}", env));
        }
        if let Ok(service) = std::env::var("DD_SERVICE") {
            default_tags.push(format!("service:{}", service));
        } else {
            default_tags.push("service:pulsearc".to_string());
        }

        Ok(Self { socket, agent_addr, prefix: prefix.to_string(), default_tags })
    }

    /// Add a default tag applied to all metrics
    pub fn add_default_tag(&mut self, key: &str, value: &str) {
        self.default_tags.push(format!("{}:{}", key, value));
    }

    // ========================================================================
    // Integer API (u64)
    // ========================================================================

    /// Send a gauge metric (current value)
    ///
    /// Gauges represent a value that can increase or decrease.
    pub fn gauge(&self, name: &str, value: u64) -> MetricsResult<()> {
        self.send_metric(name, value, "g", &[])
    }

    /// Send a gauge metric with custom tags
    pub fn gauge_with_tags(
        &self,
        name: &str,
        value: u64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, value, "g", tags)
    }

    /// Send a counter metric (increment)
    ///
    /// Counters track how many times something happened.
    pub fn count(&self, name: &str, value: u64) -> MetricsResult<()> {
        self.send_metric(name, value, "c", &[])
    }

    /// Send a counter metric with custom tags
    pub fn count_with_tags(
        &self,
        name: &str,
        value: u64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, value, "c", tags)
    }

    /// Increment a counter by 1
    pub fn increment(&self, name: &str) -> MetricsResult<()> {
        self.count(name, 1)
    }

    /// Send a histogram metric (statistical distribution)
    ///
    /// Histograms calculate statistics (P50, P95, P99, avg, etc.) on the server side.
    pub fn histogram(&self, name: &str, value: u64) -> MetricsResult<()> {
        self.send_metric(name, value, "h", &[])
    }

    /// Send a histogram metric with custom tags
    pub fn histogram_with_tags(
        &self,
        name: &str,
        value: u64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, value, "h", tags)
    }

    /// Send a timing metric (duration in milliseconds)
    ///
    /// Timers are histograms measured in milliseconds.
    pub fn timing(&self, name: &str, duration_ms: u64) -> MetricsResult<()> {
        self.send_metric(name, duration_ms, "ms", &[])
    }

    /// Send a timing metric with custom tags
    pub fn timing_with_tags(
        &self,
        name: &str,
        duration_ms: u64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, duration_ms, "ms", tags)
    }

    // ========================================================================
    // Float API (f64)
    // ========================================================================

    /// Send a gauge metric with floating-point value
    ///
    /// Use for fractional values like percentages or rates.
    pub fn gauge_f64(&self, name: &str, value: f64) -> MetricsResult<()> {
        self.send_metric(name, value, "g", &[])
    }

    /// Send a gauge metric with floating-point value and custom tags
    pub fn gauge_f64_with_tags(
        &self,
        name: &str,
        value: f64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, value, "g", tags)
    }

    /// Send a histogram metric with floating-point value
    ///
    /// Use for fractional durations or measurements.
    pub fn histogram_f64(&self, name: &str, value: f64) -> MetricsResult<()> {
        self.send_metric(name, value, "h", &[])
    }

    /// Send a histogram metric with floating-point value and custom tags
    pub fn histogram_f64_with_tags(
        &self,
        name: &str,
        value: f64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, value, "h", tags)
    }

    /// Send a timing metric with floating-point value (duration in milliseconds)
    ///
    /// Use for sub-millisecond precision timing.
    pub fn timing_f64(&self, name: &str, duration_ms: f64) -> MetricsResult<()> {
        self.send_metric(name, duration_ms, "ms", &[])
    }

    /// Send a timing metric with floating-point value and custom tags
    pub fn timing_f64_with_tags(
        &self,
        name: &str,
        duration_ms: f64,
        tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        self.send_metric(name, duration_ms, "ms", tags)
    }

    // ========================================================================
    // High-Level API
    // ========================================================================

    /// Send database metrics snapshot to Datadog
    ///
    /// Sends all DbStats fields as DogStatsD metrics. Percentiles are sent as histograms,
    /// counts as gauges, and rates as gauges (0-100). Preserves floating-point precision.
    pub fn send_db_stats(&self, stats: &DbStats) -> MetricsResult<()> {
        // Connection acquisition metrics
        self.gauge("db.connections.acquired", stats.connections_acquired)?;
        self.gauge("db.connections.timeouts", stats.connection_timeouts)?;
        self.gauge("db.connections.errors", stats.connection_errors)?;

        // Connection latency percentiles (optional values)
        if let Some(p50) = stats.p50_connection_time_ms {
            self.histogram("db.connection.latency.p50", p50)?;
        }
        if let Some(p95) = stats.p95_connection_time_ms {
            self.histogram("db.connection.latency.p95", p95)?;
        }
        if let Some(p99) = stats.p99_connection_time_ms {
            self.histogram("db.connection.latency.p99", p99)?;
        }

        // Query execution metrics
        self.gauge("db.queries.executed", stats.queries_executed)?;
        self.gauge("db.queries.errors", stats.query_errors)?;

        // Use f64 API to preserve precision for average query time
        self.histogram_f64("db.query.latency.avg", stats.avg_query_time_ms)?;

        if let Some(p95) = stats.p95_query_time_ms {
            self.histogram("db.query.latency.p95", p95)?;
        }

        // Pool utilization metrics
        self.gauge("db.pool.peak_concurrent_connections", stats.peak_concurrent_connections)?;
        self.gauge("db.pool.total_connections", stats.total_connections_in_pool)?;

        // Pool utilization as percentage (0-100) - use f64 to preserve precision
        let utilization_pct = stats.pool_utilization * 100.0;
        self.gauge_f64("db.pool.utilization_pct", utilization_pct)?;

        // Fallback tracking (dual-path strategy)
        self.gauge("db.fallback.dbmanager_successes", stats.dbmanager_successes)?;
        self.gauge("db.fallback.localdatabase_fallbacks", stats.localdatabase_fallbacks)?;

        // Fallback rate as percentage (0-100) - use f64 to preserve precision
        let fallback_rate_pct = stats.fallback_rate * 100.0;
        self.gauge_f64("db.fallback.rate_pct", fallback_rate_pct)?;

        tracing::debug!(utilization_pct, fallback_rate_pct, "Sent database metrics to Datadog");

        Ok(())
    }

    // ========================================================================
    // Internal
    // ========================================================================

    /// Send metric with custom tags (generic over Display types)
    fn send_metric<V: Display>(
        &self,
        name: &str,
        value: V,
        metric_type: &str,
        custom_tags: &[(&str, &str)],
    ) -> MetricsResult<()> {
        // Build DogStatsD metric string: <PREFIX>.<NAME>:<VALUE>|<TYPE>|#<TAGS>
        let full_name = format!("{}.{}", self.prefix, name);

        // Combine default tags with custom tags
        let mut all_tags = self.default_tags.clone();
        for (key, val) in custom_tags {
            all_tags.push(format!("{}:{}", key, val));
        }

        // Format: metric_name:value|type|#tag1:val1,tag2:val2
        let metric = if all_tags.is_empty() {
            format!("{}:{}|{}", full_name, value, metric_type)
        } else {
            let tags_str = all_tags.join(",");
            format!("{}:{}|{}|#{}", full_name, value, metric_type, tags_str)
        };

        // Send via UDP (non-blocking, best-effort)
        match self.socket.send_to(metric.as_bytes(), self.agent_addr) {
            Ok(_) => {
                tracing::trace!(metric = %full_name, %value, "Sent metric to Datadog");
                Ok(())
            }
            Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                // Non-blocking socket would block, drop metric
                tracing::warn!(
                    metric = %full_name,
                    error = %e,
                    "Dropped metric: send would block"
                );
                Ok(()) // Don't fail on dropped metrics
            }
            Err(e) => {
                tracing::warn!(
                    metric = %full_name,
                    error = %e,
                    "Failed to send metric to Datadog"
                );
                Err(crate::observability::MetricsError::SendFailed { source: e })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datadog_client_creation() {
        // Should not panic even if Datadog agent is not running
        let client = DatadogClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_gauge_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return, // Skip if socket creation fails
        };

        // Should not fail even if agent is not running (UDP is fire-and-forget)
        let result = client.gauge("test.metric", 42);
        // May succeed or fail depending on if agent is running, but shouldn't panic
        let _ = result;
    }

    #[test]
    fn test_gauge_f64_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Test floating-point precision
        let result = client.gauge_f64("test.metric.float", 42.5);
        let _ = result;
    }

    #[test]
    fn test_counter_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result = client.count("test.counter", 1);
        let _ = result;
    }

    #[test]
    fn test_histogram_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result = client.histogram("test.histogram", 123);
        let _ = result;
    }

    #[test]
    fn test_histogram_f64_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result = client.histogram_f64("test.histogram.float", 123.45);
        let _ = result;
    }

    #[test]
    fn test_timing_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result = client.timing("test.timing", 45);
        let _ = result;
    }

    #[test]
    fn test_timing_f64_metric() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result = client.timing_f64("test.timing.float", 45.67);
        let _ = result;
    }

    #[test]
    fn test_increment() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result = client.increment("test.counter");
        let _ = result;
    }

    #[test]
    fn test_custom_tags() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        let result =
            client.gauge_with_tags("test.metric", 42, &[("region", "us-west"), ("env", "test")]);
        let _ = result;
    }

    #[test]
    fn test_default_tags() {
        let mut client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };
        client.add_default_tag("version", "1.0.0");
        client.add_default_tag("datacenter", "us-east-1");

        let result = client.gauge("test.metric", 100);
        let _ = result;
    }

    #[test]
    fn test_send_db_stats() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        let stats = DbStats {
            connections_acquired: 100,
            connection_timeouts: 2,
            connection_errors: 1,
            p50_connection_time_ms: Some(10),
            p95_connection_time_ms: Some(25),
            p99_connection_time_ms: Some(50),
            queries_executed: 500,
            query_errors: 3,
            avg_query_time_ms: 5.5, // Fractional value to test precision
            p95_query_time_ms: Some(15),
            peak_concurrent_connections: 8,
            total_connections_in_pool: 10,
            pool_utilization: 0.8, // Fractional value
            dbmanager_successes: 495,
            localdatabase_fallbacks: 5,
            fallback_rate: 0.0101, // Fractional value with precision
        };

        let result = client.send_db_stats(&stats);
        // Should not panic, may succeed or fail depending on agent availability
        let _ = result;
    }

    #[test]
    fn test_send_db_stats_with_missing_percentiles() {
        let client = match DatadogClient::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        // Stats with no percentiles (empty data case)
        let stats = DbStats {
            connections_acquired: 0,
            connection_timeouts: 0,
            connection_errors: 0,
            p50_connection_time_ms: None,
            p95_connection_time_ms: None,
            p99_connection_time_ms: None,
            queries_executed: 0,
            query_errors: 0,
            avg_query_time_ms: 0.0,
            p95_query_time_ms: None,
            peak_concurrent_connections: 0,
            total_connections_in_pool: 10,
            pool_utilization: 0.0,
            dbmanager_successes: 0,
            localdatabase_fallbacks: 0,
            fallback_rate: 0.0,
        };

        // Should handle missing percentiles gracefully
        let result = client.send_db_stats(&stats);
        let _ = result;
    }
}
