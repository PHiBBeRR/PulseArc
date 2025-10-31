//! Integration tests for sync queue module
//!
//! Covers priority scheduling, deduplication/capacity safeguards, and retry
//! behavior to ensure the queue works end-to-end with metrics tracking.

#![cfg(feature = "runtime")]

use std::time::Duration;

use pulsearc_common::sync::{
    ItemStatus, Priority, QueueConfig, QueueError, QueueResult, SyncItem, SyncQueue,
};
use serde_json::json;

/// Validates that the queue enforces priority ordering while keeping metrics
/// accurate for enqueue/dequeue/complete cycles.
#[tokio::test(flavor = "multi_thread")]
async fn test_queue_priority_ordering_and_metrics() -> QueueResult<()> {
    let config = QueueConfig { max_capacity: 10, batch_size: 5, ..Default::default() };

    let queue = SyncQueue::with_config(config)?;

    // Enqueue items in non-priority order.
    let items = vec![
        SyncItem::with_id("low".to_string(), json!({ "idx": 0 }), Priority::Low),
        SyncItem::with_id("critical".to_string(), json!({ "idx": 1 }), Priority::Critical),
        SyncItem::with_id("high".to_string(), json!({ "idx": 2 }), Priority::High),
    ];

    for item in items {
        queue.push(item).await?;
    }

    // Expected dequeue order: Critical > High > Low.
    for expected_id in ["critical", "high", "low"] {
        let next =
            queue.pop().await?.unwrap_or_else(|| panic!("expected item with id {expected_id}"));
        assert_eq!(next.id, expected_id);
        assert_eq!(next.status, ItemStatus::Processing);
        queue.mark_completed(&next.id).await?;
    }

    let metrics = queue.metrics();
    assert_eq!(metrics.total_enqueued, 3);
    assert_eq!(metrics.total_dequeued, 3);
    assert_eq!(metrics.total_completed, 3);
    assert_eq!(metrics.current_size, 0);
    assert_eq!(metrics.deduplication_hits, 0);
    assert!(metrics.queue_depth_max >= 3);

    queue.shutdown().await?;
    Ok(())
}

/// Ensures deduplication prevents duplicate IDs and capacity limits are
/// enforced, recording appropriate metrics.
#[tokio::test(flavor = "multi_thread")]
async fn test_queue_deduplication_and_capacity_limits() -> QueueResult<()> {
    let config = QueueConfig {
        max_capacity: 2,
        batch_size: 2,
        enable_deduplication: true,
        ..Default::default()
    };

    let queue = SyncQueue::with_config(config)?;

    let first_id = "item-1".to_string();
    queue
        .push(SyncItem::with_id(first_id.clone(), json!({ "payload": 1 }), Priority::Normal))
        .await?;
    // Duplicate ID should be rejected before reaching capacity.
    let duplicate = SyncItem::with_id(first_id.clone(), json!({ "payload": 99 }), Priority::Low);
    let dup_err = queue.push(duplicate).await.unwrap_err();
    match dup_err {
        QueueError::DuplicateItem(id) => assert_eq!(id, first_id),
        other => panic!("expected duplicate item error, got {other:?}"),
    }

    queue
        .push(SyncItem::with_id("item-2".to_string(), json!({ "payload": 2 }), Priority::Normal))
        .await?;

    // Additional item should exceed capacity.
    let overflow = SyncItem::with_id("item-3".to_string(), json!({ "payload": 3 }), Priority::High);
    let cap_err = queue.push(overflow).await.unwrap_err();
    match cap_err {
        QueueError::CapacityExceeded(limit) => assert_eq!(limit, 2),
        other => panic!("expected capacity error, got {other:?}"),
    }

    // Drain queue to keep state clean.
    while let Some(item) = queue.pop().await? {
        queue.mark_completed(&item.id).await?;
    }

    let metrics = queue.metrics();
    assert_eq!(metrics.total_enqueued, 2);
    assert_eq!(metrics.total_completed, 2);
    assert_eq!(metrics.capacity_rejections, 1);
    assert_eq!(metrics.deduplication_hits, 1);
    assert_eq!(metrics.current_size, 0);

    queue.shutdown().await?;
    Ok(())
}

/// Validates retry scheduling: first failure requeues with backoff, final
/// failure exhausts retries and records metrics.
#[tokio::test(flavor = "multi_thread")]
async fn test_queue_retry_flow_reschedules_and_exhausts() -> QueueResult<()> {
    let config = QueueConfig {
        max_capacity: 5,
        batch_size: 5,
        base_retry_delay: Duration::from_millis(20),
        max_retry_delay: Duration::from_secs(2),
        ..Default::default()
    };

    let queue = SyncQueue::with_config(config)?;

    let item =
        SyncItem::with_id("retry-item".to_string(), json!({ "operation": "sync" }), Priority::High)
            .with_max_retries(2);

    queue.push(item).await?;

    let first_attempt = queue.pop().await?.expect("item should be dequeued");
    assert_eq!(first_attempt.retry_count, 0);
    assert_eq!(first_attempt.status, ItemStatus::Processing);

    // First failure should schedule retry.
    let should_retry =
        queue.mark_failed(&first_attempt.id, Some("transient error".to_string())).await?;
    assert!(should_retry);

    // Inspect queued item to confirm pending retry with future timestamp.
    let pending = queue.get_item(&first_attempt.id).expect("item should remain tracked for retry");
    assert_eq!(pending.status, ItemStatus::Pending);
    assert_eq!(pending.retry_count, 1);
    let next_retry_at = pending.next_retry_at.expect("retry timestamp should be set");
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    assert!(next_retry_at > now_ms, "retry timestamp should be in the future");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let second_attempt = queue.pop().await?.expect("retry should be ready");
    assert_eq!(second_attempt.retry_count, 1);

    // Second failure exhausts retries.
    let final_retry =
        queue.mark_failed(&second_attempt.id, Some("permanent failure".to_string())).await?;
    assert!(!final_retry);

    assert!(queue.pop().await?.is_none());
    assert_eq!(queue.size(), 0);

    let metrics = queue.metrics();
    assert_eq!(metrics.total_retried, 1);
    assert_eq!(metrics.total_failed, 1);
    assert_eq!(metrics.total_completed, 0);
    assert_eq!(metrics.current_size, 0);

    queue.shutdown().await?;
    Ok(())
}
