//! Mock repository implementations for testing
//!
//! Provides in-memory mocks for all core repository ports, enabling
//! deterministic unit tests without database dependencies.

use std::sync::Arc;

use async_trait::async_trait;
use pulsearc_core::tracking::ports::{SegmentRepository, SnapshotRepository};
use pulsearc_domain::{ActivitySegment, ActivitySnapshot, Result as DomainResult};

/// In-memory mock for `SnapshotRepository`.
///
/// Stores a fixed set of snapshots and provides simple query operations.
/// Designed for classification and tracking tests.
#[derive(Default, Clone)]
pub struct MockSnapshotRepository {
    snapshots: Arc<Vec<ActivitySnapshot>>,
}

impl MockSnapshotRepository {
    /// Create a new mock seeded with the provided snapshots.
    pub fn new(snapshots: Vec<ActivitySnapshot>) -> Self {
        Self {
            snapshots: Arc::new(snapshots),
        }
    }

    /// Convenience helper for adding a single snapshot to the mock.
    pub fn with_snapshot(mut self, snapshot: ActivitySnapshot) -> Self {
        Arc::make_mut(&mut self.snapshots).push(snapshot);
        self
    }
}

#[async_trait]
impl SnapshotRepository for MockSnapshotRepository {
    async fn find_snapshots_by_time_range(
        &self,
        start: i64,
        end: i64,
    ) -> DomainResult<Vec<ActivitySnapshot>> {
        Ok(self
            .snapshots
            .iter()
            .filter(|snap| snap.timestamp >= start && snap.timestamp <= end)
            .cloned()
            .collect())
    }

    async fn count_snapshots_by_date(&self, _date: &str) -> DomainResult<i64> {
        // Simple implementation for mocks - just count all snapshots
        Ok(self.snapshots.len() as i64)
    }
}

/// In-memory mock for `SegmentRepository`.
///
/// Stores a fixed set of segments and provides simple CRUD operations.
/// Uses synchronous API to match SqlCipherPool design.
#[derive(Default, Clone)]
pub struct MockSegmentRepository {
    segments: Arc<Vec<ActivitySegment>>,
}

impl MockSegmentRepository {
    /// Create a new mock seeded with the provided segments.
    pub fn new(segments: Vec<ActivitySegment>) -> Self {
        Self {
            segments: Arc::new(segments),
        }
    }

    /// Convenience helper for adding a single segment to the mock.
    pub fn with_segment(mut self, segment: ActivitySegment) -> Self {
        Arc::make_mut(&mut self.segments).push(segment);
        self
    }
}

impl SegmentRepository for MockSegmentRepository {
    fn save_segment(&self, _segment: &ActivitySegment) -> DomainResult<()> {
        // Mock implementation - just acknowledge success
        Ok(())
    }

    fn get_segments_by_date(&self, date: &str) -> DomainResult<Vec<ActivitySegment>> {
        // Filter segments by date (assumes segment has date field)
        Ok(self
            .segments
            .iter()
            .filter(|seg| {
                // Simple date filtering - assumes date is formatted as YYYY-MM-DD
                seg.start_timestamp
                    .to_string()
                    .starts_with(&date.replace('-', ""))
            })
            .cloned()
            .collect())
    }

    fn count_segments(&self) -> DomainResult<i64> {
        Ok(self.segments.len() as i64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_snapshot_repository() {
        // Arrange
        let snapshot1 = create_test_snapshot(1, 1000);
        let snapshot2 = create_test_snapshot(2, 2000);
        let snapshot3 = create_test_snapshot(3, 3000);

        let repo = MockSnapshotRepository::new(vec![snapshot1, snapshot2, snapshot3]);

        // Act - Query snapshots in range
        let results = repo.find_snapshots_by_time_range(1500, 2500).await.unwrap();

        // Assert - Should only return snapshot2
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "test-snap-2");
    }

    #[tokio::test]
    async fn test_mock_snapshot_repository_count() {
        // Arrange
        let repo = MockSnapshotRepository::new(vec![
            create_test_snapshot(1, 1000),
            create_test_snapshot(2, 2000),
        ]);

        // Act
        let count = repo.count_snapshots_by_date("2024-10-30").await.unwrap();

        // Assert
        assert_eq!(count, 2);
    }

    #[test]
    fn test_mock_segment_repository() {
        // Arrange
        let segment1 = create_test_segment(1, 1000, 2000);
        let segment2 = create_test_segment(2, 3000, 4000);

        let repo = MockSegmentRepository::new(vec![segment1.clone(), segment2]);

        // Act
        let count = repo.count_segments().unwrap();
        let save_result = repo.save_segment(&segment1);

        // Assert
        assert_eq!(count, 2);
        assert!(save_result.is_ok());
    }

    // Test helpers
    fn create_test_snapshot(id: i32, timestamp: i64) -> ActivitySnapshot {
        ActivitySnapshot {
            id: format!("test-snap-{}", id),
            timestamp,
            detected_activity: "Working".to_string(),
            work_type: Some("development".to_string()),
            primary_app: "Microsoft Excel".to_string(),
            activity_category: Some("work".to_string()),
            activity_context_json: r#"{"active_app": {"app_name": "Excel", "window_title": "Test"}}"#.to_string(),
            processed: false,
            batch_id: None,
            created_at: timestamp,
            processed_at: None,
            is_idle: false,
            idle_duration_secs: None,
        }
    }

    fn create_test_segment(id: i32, start: i64, end: i64) -> ActivitySegment {
        ActivitySegment {
            id: format!("test-seg-{}", id),
            start_timestamp: start,
            end_timestamp: end,
            duration_seconds: end - start,
            primary_context: "work".to_string(),
            snapshot_ids: vec![],
            gap_before_seconds: Some(0),
            confidence_score: 0.95,
            context_signals_json: r#"{}"#.to_string(),
            created_at: start,
        }
    }
}
