//! Integration tests for sync retry module
//!
//! Exercises retry strategies, budgets, and circuit breaker adapters together
//! to validate realistic retry orchestration scenarios.

#![cfg(feature = "runtime")]

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use pulsearc_common::sync::retry::circuit_breaker_adapter::CircuitBreakerBuilder;
use pulsearc_common::sync::retry::time::MockClock as BudgetMockClock;
use pulsearc_common::sync::retry::{
    CircuitState, MockClock as BreakerMockClock, RetryBudget, RetryError, RetryPolicies,
    RetryStrategy,
};

/// Ensures the network retry policy retries transient network-style failures
/// and eventually succeeds.
#[tokio::test(flavor = "multi_thread")]
async fn test_network_policy_retries_transient_errors() {
    let strategy = RetryPolicies::network_policy()
        .with_base_delay(Duration::from_millis(5))
        .unwrap()
        .with_max_delay(Duration::from_millis(15))
        .unwrap();

    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = Arc::clone(&attempts);

    let result = strategy
        .execute(move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                let current = attempts.fetch_add(1, Ordering::SeqCst);
                if current < 2 {
                    Err(std::io::Error::other("connection reset by peer"))
                } else {
                    Ok::<_, std::io::Error>("ok")
                }
            }
        })
        .await;

    let value = result.expect("network policy should recover");
    assert_eq!(value, "ok");
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
}

/// Validates that non-network errors are not retried by the network policy.
#[tokio::test(flavor = "multi_thread")]
async fn test_network_policy_stops_on_non_retryable_error() {
    let strategy = RetryPolicies::network_policy()
        .with_base_delay(Duration::from_millis(5))
        .unwrap()
        .with_max_delay(Duration::from_millis(15))
        .unwrap();

    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = Arc::clone(&attempts);

    let error = strategy
        .execute(move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                attempts.fetch_add(1, Ordering::SeqCst);
                Err::<(), _>(std::io::Error::other("validation failed"))
            }
        })
        .await
        .expect_err("non-retryable error should bubble up");

    match error {
        RetryError::OperationFailed { .. } => {}
        other => panic!("expected OperationFailed, got {other:?}"),
    }
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

/// Confirms retry metrics capture attempts and success information.
#[tokio::test(flavor = "multi_thread")]
async fn test_retry_strategy_metrics_collection() {
    let strategy = RetryStrategy::new()
        .with_max_attempts(3)
        .unwrap()
        .with_base_delay(Duration::from_millis(5))
        .unwrap()
        .with_max_delay(Duration::from_millis(10))
        .unwrap();

    let attempts = Arc::new(AtomicU32::new(0));
    let attempts_clone = Arc::clone(&attempts);

    let (result, metrics) = strategy
        .execute_with_metrics("metrics_test", move || {
            let attempts = Arc::clone(&attempts_clone);
            async move {
                let current = attempts.fetch_add(1, Ordering::SeqCst);
                if current == 0 {
                    Err(std::io::Error::other("flaky once"))
                } else {
                    Ok::<_, std::io::Error>("done")
                }
            }
        })
        .await;

    let value = result.expect("operation should succeed on retry");
    assert_eq!(value, "done");
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(metrics.attempts, 2);
    assert!(metrics.succeeded);
}

/// Verifies retry budget token depletion and refill behavior with a mock
/// clock.
#[test]
fn test_retry_budget_refill_with_mock_clock() {
    let clock = BudgetMockClock::new();
    let budget = RetryBudget::with_clock(3, 1.0, clock.clone()); // 1 token/sec

    assert!(budget.try_acquire());
    assert!(budget.try_acquire());
    assert!(budget.try_acquire());
    assert!(!budget.try_acquire()); // exhausted

    clock.advance(Duration::from_secs(2)); // Should refill two tokens

    assert!(budget.try_acquire());
    assert_eq!(budget.available(), 1);
}

/// Ensures circuit breaker transitions to open after failures and allows
/// recovery after the timeout when using a mock clock.
#[test]
fn test_circuit_breaker_opens_and_recovers() {
    let clock = BreakerMockClock::new();
    let breaker = CircuitBreakerBuilder::new()
        .with_failure_threshold(2)
        .with_success_threshold(1)
        .with_timeout(Duration::from_secs(1))
        .build_with_clock(clock.clone());

    assert!(breaker.should_allow_request().unwrap());

    breaker.record_failure().unwrap();
    breaker.record_failure().unwrap();

    assert_eq!(breaker.state().unwrap(), CircuitState::Open);
    assert!(!breaker.should_allow_request().unwrap());

    // Advance beyond timeout to transition to half-open.
    clock.advance_millis(1500);
    assert!(breaker.should_allow_request().unwrap());
    assert_eq!(breaker.state().unwrap(), CircuitState::HalfOpen);

    breaker.record_success().unwrap();
    assert_eq!(breaker.state().unwrap(), CircuitState::Closed);
    assert!(breaker.should_allow_request().unwrap());
}
