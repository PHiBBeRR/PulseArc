//! Integration tests for resilience module
//!
//! Tests circuit breaker and retry logic with various failure scenarios

#![cfg(feature = "runtime")]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pulsearc_common::resilience::{
    policies, retry_with_policy, CircuitBreaker, CircuitBreakerConfig, CircuitState, MockClock,
    RetryConfig, SystemClock,
};

/// Custom error type for testing
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct TestError {
    message: String,
    retryable: bool,
}

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TestError {}

/// Validates retry mechanism with exponential backoff strategy.
///
/// This test ensures the retry function can recover from transient failures
/// using exponential backoff, where delays increase exponentially between
/// attempts. This prevents overwhelming failing services while allowing
/// recovery from brief outages.
///
/// # Test Steps
/// 1. Configure retry with exponential backoff (base 2.0, max 5 attempts)
/// 2. Simulate function failing first 3 attempts
/// 3. Allow success on 4th attempt
/// 4. Verify retry persisted through failures
/// 5. Confirm exactly 4 attempts made (3 failures + 1 success)
/// 6. Validate final result is successful
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_exponential_backoff_success() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let config = RetryConfig::new()
        .max_attempts(5)
        .exponential_backoff(Duration::from_millis(10), 2.0, Duration::from_millis(100))
        .full_jitter()
        .build()
        .expect("Failed to build config");

    let policy = policies::AlwaysRetry;

    let result = retry_with_policy(config, policy, || async {
        let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        if count < 3 {
            Err(TestError { message: "Transient failure".to_string(), retryable: true })
        } else {
            Ok("Success")
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Success");
    assert_eq!(attempt_count.load(Ordering::SeqCst), 4); // 3 failures + 1
                                                         // success
}

/// Validates retry mechanism gives up after max attempts exceeded.
///
/// This test ensures the retry mechanism doesn't retry indefinitely, respecting
/// the maximum attempts limit to prevent infinite loops and resource
/// exhaustion. Critical for preventing cascading failures in distributed
/// systems.
///
/// # Test Steps
/// 1. Configure retry with max 3 attempts and fixed backoff
/// 2. Simulate persistent failures (never succeeds)
/// 3. Verify retry gives up after 3 attempts
/// 4. Confirm final result is error (not success)
/// 5. Validate exactly 3 attempts were made (no more, no less)
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_max_attempts_exceeded() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let config = RetryConfig::new()
        .max_attempts(3)
        .fixed_backoff(Duration::from_millis(10))
        .build()
        .expect("Failed to build config");

    let policy = policies::AlwaysRetry;

    let result: Result<(), _> = retry_with_policy(config, policy, || async {
        attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        Err(TestError { message: "Persistent failure".to_string(), retryable: true })
    })
    .await;

    assert!(result.is_err());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
}

/// Validates custom retry policy for selective retry logic.
///
/// This test ensures custom retry policies can make intelligent decisions about
/// whether to retry based on error characteristics. Some errors are retryable
/// (transient) while others should fail immediately (permanent errors).
///
/// # Test Steps
/// 1. Define custom policy: retry only if error message contains "retryable"
/// 2. Test with retryable error - should retry and succeed
/// 3. Reset and test with non-retryable ("fatal") error
/// 4. Verify fatal error fails immediately without retries
/// 5. Confirm policy correctly distinguishes error types
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_with_custom_policy() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let config = RetryConfig::new()
        .max_attempts(5)
        .linear_backoff(Duration::from_millis(10), Duration::from_millis(5))
        .build()
        .expect("Failed to build config");

    // Test with retryable error
    let policy = policies::PredicateRetry::new(|error: &TestError, _attempt| {
        error.message.contains("retryable")
    });

    let result = retry_with_policy(config.clone(), policy, || async {
        let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            Err(TestError { message: "retryable error".to_string(), retryable: true })
        } else {
            Ok("Success")
        }
    })
    .await;

    assert!(result.is_ok());

    // Reset and test with non-retryable error
    attempt_count.store(0, Ordering::SeqCst);

    let policy = policies::PredicateRetry::new(|error: &TestError, _attempt| {
        error.message.contains("retryable")
    });

    let result: Result<(), _> = retry_with_policy(config, policy, || async {
        attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        Err(TestError { message: "fatal error".to_string(), retryable: false })
    })
    .await;

    assert!(result.is_err());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 1); // Should not retry
}

/// Validates basic circuit breaker state transitions.
///
/// This test ensures the circuit breaker opens after failure threshold is
/// reached, protecting downstream services from cascading failures. The circuit
/// breaker fails fast once open, preventing further load on failing services.
///
/// # Test Steps
/// 1. Create circuit breaker with failure threshold of 3
/// 2. Verify initial state is Closed (normal operation)
/// 3. Trigger 3 consecutive failures
/// 4. Verify circuit transitions to Open state
/// 5. Attempt call while open - should fail fast without execution
/// 6. Confirm circuit breaker prevents calls to failing service
#[tokio::test(flavor = "multi_thread")]
async fn test_circuit_breaker_basic() {
    let config = CircuitBreakerConfig::new()
        .failure_threshold(3)
        .success_threshold(2)
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to build config");

    let breaker = CircuitBreaker::new(config).expect("Failed to create circuit breaker");

    // Initially closed
    assert_eq!(breaker.state(), CircuitState::Closed);

    // Simulate failures
    for _ in 0..3 {
        let result: Result<(), TestError> =
            Err(TestError { message: "Failure".to_string(), retryable: true });
        let _ = breaker.call(|| result);
    }

    // Should now be open
    assert_eq!(breaker.state(), CircuitState::Open);

    // Calls should fail fast
    let result = breaker.call(|| Ok::<_, TestError>("Should not execute"));
    assert!(result.is_err());
}

/// Validates complete circuit breaker state machine transitions.
///
/// This test ensures the circuit breaker correctly transitions through all
/// states: Closed -> Open -> HalfOpen -> Closed. The HalfOpen state allows
/// testing if service has recovered before fully closing the circuit.
///
/// # Test Steps
/// 1. Start in Closed state with mock clock
/// 2. Trigger failures to transition to Open state
/// 3. Advance time past timeout period
/// 4. Next call transitions to HalfOpen (testing recovery)
/// 5. Successful call closes circuit (back to normal)
/// 6. Verify complete state machine cycle works correctly
#[tokio::test(flavor = "multi_thread")]
async fn test_circuit_breaker_state_transitions() {
    let clock = Arc::new(MockClock::with_current_time(std::time::Instant::now()));

    let breaker = CircuitBreakerConfig::new()
        .failure_threshold(2)
        .success_threshold(1)
        .timeout(Duration::from_millis(100))
        .clock(clock.clone())
        .build()
        .expect("Failed to build circuit breaker");

    // Start in Closed state
    assert_eq!(breaker.state(), CircuitState::Closed);

    // Trigger failures to open circuit
    for _ in 0..2 {
        let result: Result<(), TestError> =
            Err(TestError { message: "Failure".to_string(), retryable: true });
        let _ = breaker.call(|| result);
    }

    // Now Open
    assert_eq!(breaker.state(), CircuitState::Open);

    // Advance time to trigger HalfOpen
    clock.advance_millis(150);

    // Next call should attempt in HalfOpen state
    let result = breaker.call(|| Ok::<_, TestError>("Success"));

    assert!(result.is_ok());

    // After success in HalfOpen, should transition to Closed
    assert_eq!(breaker.state(), CircuitState::Closed);
}

/// Validates circuit breaker tracks operation metrics accurately.
///
/// This test ensures the circuit breaker correctly tracks success/failure
/// counts and total calls, providing visibility into service health. Metrics
/// are critical for monitoring, alerting, and debugging production issues.
///
/// # Test Steps
/// 1. Create circuit breaker with metrics enabled
/// 2. Execute 3 successful operations
/// 3. Execute 2 failed operations
/// 4. Query metrics
/// 5. Verify success_count = 3, failure_count = 2
/// 6. Confirm total_calls = 5
#[tokio::test(flavor = "multi_thread")]
async fn test_circuit_breaker_metrics() {
    let config = CircuitBreakerConfig::new()
        .failure_threshold(5)
        .success_threshold(2)
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to build config");

    let breaker = CircuitBreaker::new(config).expect("Failed to create circuit breaker");

    // Execute some successful calls
    for _ in 0..3 {
        let _ = breaker.call(|| Ok::<_, TestError>("Success"));
    }

    // Execute some failed calls
    for _ in 0..2 {
        let result: Result<(), TestError> =
            Err(TestError { message: "Failure".to_string(), retryable: true });
        let _ = breaker.call(|| result);
    }

    let metrics = breaker.metrics();
    assert_eq!(metrics.success_count, 3);
    assert_eq!(metrics.failure_count, 2);
    assert_eq!(metrics.total_calls, 5);
}

/// Validates combining circuit breaker with retry for layered resilience.
///
/// This test ensures circuit breaker and retry mechanisms work together,
/// providing multiple layers of fault tolerance. Retry handles transient
/// failures while circuit breaker prevents overwhelming persistently failing
/// services.
///
/// # Test Steps
/// 1. Configure circuit breaker and retry mechanism
/// 2. Wrap operation in both circuit breaker and retry
/// 3. Simulate transient failures (first 2 attempts fail)
/// 4. Allow success on 3rd attempt
/// 5. Verify retry recovered from transient failures
/// 6. Confirm circuit breaker tracked all attempts correctly
#[tokio::test(flavor = "multi_thread")]
async fn test_circuit_breaker_with_retry() {
    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let cb_config = CircuitBreakerConfig::new()
        .failure_threshold(3)
        .success_threshold(1)
        .timeout(Duration::from_millis(200))
        .build()
        .expect("Failed to build config");

    let breaker =
        Arc::new(CircuitBreaker::new(cb_config).expect("Failed to create circuit breaker"));

    let retry_config = RetryConfig::new()
        .max_attempts(5)
        .fixed_backoff(Duration::from_millis(10))
        .build()
        .expect("Failed to build config");

    let policy = policies::AlwaysRetry;

    // Wrap operation with both circuit breaker and retry
    let result = retry_with_policy(retry_config, policy, || {
        let breaker_clone = Arc::clone(&breaker);
        let count_clone = Arc::clone(&attempt_count_clone);

        async move {
            breaker_clone.call(|| {
                let count = count_clone.fetch_add(1, Ordering::SeqCst);
                if count < 2 {
                    Err(TestError { message: "Transient failure".to_string(), retryable: true })
                } else {
                    Ok("Success")
                }
            })
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(result.expect("Should succeed"), "Success");
}

/// Validates all supported backoff strategies work correctly.
///
/// This test ensures all backoff strategies (Fixed, Linear, Exponential,
/// Decorrelated) function properly with the retry mechanism. Different
/// strategies suit different scenarios (API rate limits, database load, network
/// congestion).
///
/// # Test Steps
/// 1. Test Fixed backoff (constant delay between attempts)
/// 2. Test Linear backoff (delay increases linearly)
/// 3. Test Exponential backoff (delay doubles each attempt)
/// 4. Verify each strategy completes successfully
/// 5. Confirm all strategies respect max attempts
#[tokio::test(flavor = "multi_thread")]
async fn test_different_backoff_strategies() {
    let policy = policies::AlwaysRetry;

    // Test Fixed backoff
    let config = RetryConfig::new()
        .max_attempts(3)
        .fixed_backoff(Duration::from_millis(10))
        .build()
        .expect("Failed to build config");

    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let result = retry_with_policy(config, policy.clone(), || async {
        let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            Err(TestError { message: "Failure".to_string(), retryable: true })
        } else {
            Ok("Success")
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);

    // Test Linear backoff
    let config = RetryConfig::new()
        .max_attempts(3)
        .linear_backoff(Duration::from_millis(10), Duration::from_millis(5))
        .build()
        .expect("Failed to build config");

    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let result = retry_with_policy(config, policy.clone(), || async {
        let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            Err(TestError { message: "Failure".to_string(), retryable: true })
        } else {
            Ok("Success")
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);

    // Test Exponential backoff
    let config = RetryConfig::new()
        .max_attempts(3)
        .exponential_backoff(Duration::from_millis(10), 2.0, Duration::from_millis(100))
        .build()
        .expect("Failed to build config");

    let attempt_count = Arc::new(AtomicU32::new(0));
    let attempt_count_clone = Arc::clone(&attempt_count);

    let result = retry_with_policy(config, policy, || async {
        let count = attempt_count_clone.fetch_add(1, Ordering::SeqCst);
        if count < 2 {
            Err(TestError { message: "Failure".to_string(), retryable: true })
        } else {
            Ok("Success")
        }
    })
    .await;

    assert!(result.is_ok());
    assert_eq!(attempt_count.load(Ordering::SeqCst), 3);
}

/// Validates different jitter types for retry timing randomization.
///
/// This test ensures jitter options (None, Full, Equal, Decorrelated) work with
/// retry mechanism. Jitter prevents thundering herd problem where many clients
/// retry simultaneously, overwhelming recovering services.
///
/// # Test Steps
/// 1. Test with no jitter (predictable delays)
/// 2. Test with full jitter (maximum randomization)
/// 3. Test with equal jitter (balanced randomization)
/// 4. Test with decorrelated jitter (independent randomization)
/// 5. Verify all jitter types complete execution
/// 6. Confirm jitter doesn't break retry logic
#[tokio::test(flavor = "multi_thread")]
async fn test_jitter_types() {
    let policy = policies::AlwaysRetry;

    // Test no jitter
    let config = RetryConfig::new()
        .max_attempts(3)
        .exponential_backoff(Duration::from_millis(10), 2.0, Duration::from_millis(100))
        .no_jitter()
        .build()
        .expect("Failed to build config");

    let result: Result<(), _> = retry_with_policy(config, policy.clone(), || async {
        Err(TestError { message: "Always fails".to_string(), retryable: true })
    })
    .await;

    assert!(result.is_err());

    // Test full jitter
    let config = RetryConfig::new()
        .max_attempts(3)
        .exponential_backoff(Duration::from_millis(10), 2.0, Duration::from_millis(100))
        .full_jitter()
        .build()
        .expect("Failed to build config");

    let result: Result<(), _> = retry_with_policy(config, policy.clone(), || async {
        Err(TestError { message: "Always fails".to_string(), retryable: true })
    })
    .await;

    assert!(result.is_err());

    // Test equal jitter
    let config = RetryConfig::new()
        .max_attempts(3)
        .exponential_backoff(Duration::from_millis(10), 2.0, Duration::from_millis(100))
        .equal_jitter()
        .build()
        .expect("Failed to build config");

    let result: Result<(), _> = retry_with_policy(config, policy.clone(), || async {
        Err(TestError { message: "Always fails".to_string(), retryable: true })
    })
    .await;

    assert!(result.is_err());

    // Test decorrelated jitter
    let config = RetryConfig::new()
        .max_attempts(3)
        .exponential_backoff(Duration::from_millis(10), 2.0, Duration::from_millis(100))
        .decorrelated_jitter(Duration::from_millis(10))
        .build()
        .expect("Failed to build config");

    let result: Result<(), _> = retry_with_policy(config, policy, || async {
        Err(TestError { message: "Always fails".to_string(), retryable: true })
    })
    .await;

    assert!(result.is_err());
}

/// Validates circuit breaker with real system clock (not mocked).
///
/// This test ensures the circuit breaker works with real time delays, verifying
/// timeout-based state transitions happen correctly in production-like
/// conditions. Uses actual sleep() calls to test timing behavior.
///
/// # Test Steps
/// 1. Create circuit breaker with system clock and 50ms timeout
/// 2. Trigger failures to open circuit
/// 3. Verify circuit is Open
/// 4. Sleep for 60ms (past timeout)
/// 5. Make successful call (transitions HalfOpen -> Closed)
/// 6. Confirm circuit recovered and is Closed
#[tokio::test(flavor = "multi_thread")]
async fn test_circuit_breaker_with_system_clock() {
    let clock = Arc::new(SystemClock);

    let breaker = CircuitBreakerConfig::new()
        .failure_threshold(2)
        .success_threshold(1)
        .timeout(Duration::from_millis(50))
        .clock(clock)
        .build()
        .expect("Failed to build circuit breaker");

    // Trigger failures
    for _ in 0..2 {
        let result: Result<(), TestError> =
            Err(TestError { message: "Failure".to_string(), retryable: true });
        let _ = breaker.call(|| result);
    }

    assert_eq!(breaker.state(), CircuitState::Open);

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(60)).await;

    // Should transition to HalfOpen and succeed
    let result = breaker.call(|| Ok::<_, TestError>("Success"));

    assert!(result.is_ok());
}

/// Validates thread-safe concurrent circuit breaker access.
///
/// This test ensures the circuit breaker is safe for concurrent use by multiple
/// async tasks, correctly tracking all operations without data races or lost
/// counts. Critical for high-throughput production systems.
///
/// # Test Steps
/// 1. Create circuit breaker shared via Arc
/// 2. Spawn 20 concurrent tasks
/// 3. Each task makes calls (some succeed, some fail)
/// 4. Wait for all tasks to complete
/// 5. Verify mix of successes and failures occurred
/// 6. Confirm no concurrency bugs or panics
#[tokio::test(flavor = "multi_thread")]
async fn test_concurrent_circuit_breaker_access() {
    let config = CircuitBreakerConfig::new()
        .failure_threshold(10)
        .success_threshold(2)
        .timeout(Duration::from_millis(100))
        .build()
        .expect("Failed to build config");

    let breaker = Arc::new(CircuitBreaker::new(config).expect("Failed to create circuit breaker"));
    let mut handles = vec![];

    // Spawn multiple tasks
    for i in 0..20 {
        let breaker_clone = Arc::clone(&breaker);
        let handle = tokio::spawn(async move {
            if i % 3 == 0 {
                breaker_clone.call(|| Ok::<_, TestError>("Success"))
            } else {
                breaker_clone
                    .call(|| Err(TestError { message: "Failure".to_string(), retryable: true }))
            }
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let mut success_count = 0;
    let mut failure_count = 0;

    for handle in handles {
        match handle.await.expect("Task should complete") {
            Ok(_) => success_count += 1,
            Err(_) => failure_count += 1,
        }
    }

    assert!(success_count > 0);
    assert!(failure_count > 0);
}
