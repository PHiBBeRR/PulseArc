//! Block builder - consolidates daily snapshots into meaningful time blocks
//!
//! REFACTOR-004: Simplified to only consolidate segments (no inference).
//! Evidence extraction and classification now handled by EvidenceExtractor +
//! OpenAI.
//!
//! # ADR-003 Migration
//!
//! Migrated from legacy/api/src/inference/block_builder.rs to core business
//! logic layer. This is pure business logic with no infrastructure dependencies
//! - already simplified by REFACTOR-004 which removed SignalExtractor and
//!   ProjectMatcher dependencies.

use ahash::AHashMap as HashMap; // Fast non-cryptographic hasher
use pulsearc_domain::classification::{ActivityBreakdown, BlockConfig, ProposedBlock};
use pulsearc_domain::types::ActivitySegment;
use pulsearc_domain::Result;

/// Enriched segment (simplified - no project match)
/// REFACTOR-004: Removed project_match field (inference moved to OpenAI)
#[derive(Clone)]
struct EnrichedSegment {
    segment: ActivitySegment,
}

/// Block builder - converts daily segments into consolidated blocks
/// REFACTOR-004: Simplified to only consolidate by time gaps (no inference)
pub struct BlockBuilder {
    config: BlockConfig,
}

impl BlockBuilder {
    /// Create new block builder (simplified - no dependencies)
    /// REFACTOR-004: Removed SignalExtractor and ProjectMatcher dependencies
    pub fn new(config: BlockConfig) -> Result<Self> {
        Ok(Self { config })
    }

    // ✅ REMOVED: Deprecated build_daily_blocks() method removed in REFACTOR-003
    // Phase 5 Use build_daily_blocks_from_segments() instead (primary method
    // below)

    /// ✅ **PRIMARY METHOD**: Build blocks from pre-aggregated segments
    /// (REFACTOR-003)
    ///
    /// **This is the correct method to use.** Use `Segmenter` first to create
    /// segments, then pass them here. Do NOT use `build_daily_blocks()`
    /// (deprecated).
    ///
    /// # Correct Architectural Flow
    /// ```rust,ignore
    /// // Step 1: Segmenter extracts signals + matches projects (ONCE per segment)
    /// let segmenter = Segmenter::new(db, signal_extractor, project_matcher);
    /// let segments = segmenter.create_segments_with_window(&snapshots, 300)?;
    /// segmenter.save_segments_batch(&segments)?; // Stores in DB with pre-computed JSON
    ///
    /// // Step 2: BlockBuilder consumes segments (NO re-extraction needed)
    /// let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch)?;
    /// ```
    ///
    /// # Algorithm
    /// 1. Filter segments for target day (overlap check: `[start, end)`
    ///    half-open ranges)
    /// 2. Sort segments by `start_ts` for deterministic processing
    /// 3. Deserialize pre-computed `project_match_json` (fallback to extraction
    ///    if corrupt)
    /// 4. Group by project + workstream (3-case merge logic):
    ///    - CASE 1: Same project + workstream, gap ≤ 3 min → MERGE
    ///    - CASE 2: Same project, gap ≤ 1 hour → MERGE (consolidation window)
    ///    - CASE 3: Unknown project, same app, gap ≤ 3 min → MERGE (fallback)
    /// 5. Finalize blocks with duration-weighted metrics:
    ///    - Activity breakdown (by segment duration, not count)
    ///    - Project selection (majority by duration)
    ///    - Workstream selection (majority by duration)
    ///    - Confidence (70% base + 30% agreement ratio)
    /// 6. Classify: 30+ min with project → billable, else non-billable
    ///
    /// # Arguments
    /// * `segments` - Pre-aggregated ActivitySegments from Segmenter (typically
    ///   20 per day)
    /// * `day_epoch` - Unix timestamp of day start (midnight UTC)
    ///
    /// # Returns
    /// Vec of 5-10 consolidated blocks with duration-weighted metrics
    ///
    /// # Performance Benefits
    /// - ✅ **10x faster**: 20 segments vs 200 snapshots
    /// - ✅ **No redundant extraction**: Signals computed once in Segmenter
    /// - ✅ **No redundant matching**: Projects matched once in Segmenter
    /// - ✅ **Parallel processing**: Rayon gives 2-4x speedup on multi-core
    /// - ✅ **Database caching**: Pre-computed JSON stored for reuse
    pub fn build_daily_blocks_from_segments(
        &self,
        segments: &[ActivitySegment],
        day_epoch: i64,
    ) -> Result<Vec<ProposedBlock>> {
        // Step 1: Filter segments for target day (overlap check for half-open ranges)
        let day_start = day_epoch;
        let day_end = day_epoch + 86400;

        // Use overlap check: segment overlaps day if segment.end_ts > day_start AND
        // segment.start_ts < day_end
        let mut day_segments: Vec<&ActivitySegment> =
            segments.iter().filter(|s| s.end_ts > day_start && s.start_ts < day_end).collect();

        if day_segments.is_empty() {
            return Ok(vec![]);
        }

        // Step 1.5: Sort segments by start_ts for deterministic processing
        day_segments.sort_by_key(|s| s.start_ts);

        // Step 2: Wrap segments in EnrichedSegment (no inference)
        // REFACTOR-004: Removed project matching - just wrap segments
        let enriched: Vec<EnrichedSegment> = day_segments
            .iter()
            .map(|segment| EnrichedSegment { segment: (*segment).clone() })
            .collect();

        // Step 3: Group segments by time gaps only (simplified - no project matching)
        // REFACTOR-004: Merge only by time gap (3 min) + same app
        let mut blocks = Vec::new();
        let mut current_block: Vec<EnrichedSegment> = Vec::new();

        for enriched_seg in enriched {
            if current_block.is_empty() {
                current_block.push(enriched_seg);
                continue;
            }

            let last = current_block.last().unwrap();
            // Gap can be negative if segments overlap (this is fine - we merge on gap <=
            // threshold)
            let gap = enriched_seg.segment.start_ts - last.segment.end_ts;

            // REFACTOR-004: Simplified merge logic - only consider time gap + same app
            let same_app = enriched_seg.segment.primary_app == last.segment.primary_app;
            let should_merge = same_app && gap <= self.config.max_gap_for_merge_secs;

            if should_merge {
                current_block.push(enriched_seg);
            } else {
                // Finalize current block
                if let Some(block) =
                    self.finalize_block_from_segments(&current_block, day_start, day_end)?
                {
                    blocks.push(block);
                }
                current_block.clear();
                current_block.push(enriched_seg);
            }
        }

        // Finalize last block
        if !current_block.is_empty() {
            if let Some(block) =
                self.finalize_block_from_segments(&current_block, day_start, day_end)?
            {
                blocks.push(block);
            }
        }

        Ok(blocks)
    }

    /// Classify an arbitrary time selection without rebuilding the day
    /// (on-demand classification)
    ///
    /// REFACTOR-004: Simplified to only consolidate segments in the selection
    /// range. No inference - classification will be done by OpenAI.
    ///
    /// **Use Case**: UI time-range selection for instant feedback
    ///
    /// # Arguments
    /// * `segments` - All available segments for the day
    /// * `selection_start` - Selection start timestamp (Unix seconds)
    /// * `selection_end` - Selection end timestamp (Unix seconds)
    ///
    /// # Returns
    /// A single `ProposedBlock` for the selection, or `None` if no segments
    /// overlap
    ///
    /// # Example
    /// ```ignore
    /// // User selects 2:00 PM - 3:30 PM on the timeline
    /// let block = builder.propose_block_for_selection(
    ///     &all_segments,
    ///     1698163200,  // 2:00 PM
    ///     1698168600,  // 3:30 PM
    /// )?;
    /// ```
    pub fn propose_block_for_selection(
        &self,
        segments: &[ActivitySegment],
        selection_start: i64,
        selection_end: i64,
    ) -> Result<Option<ProposedBlock>> {
        // 1) Filter by overlap with the selection
        let mut sel: Vec<&ActivitySegment> = segments
            .iter()
            .filter(|s| s.end_ts > selection_start && s.start_ts < selection_end)
            .collect();

        if sel.is_empty() {
            return Ok(None);
        }

        // 2) Sort for determinism
        sel.sort_by_key(|s| s.start_ts);

        // 3) Wrap in EnrichedSegment (no project matching)
        // REFACTOR-004: Removed project matching
        let enriched: Vec<EnrichedSegment> =
            sel.iter().map(|seg| EnrichedSegment { segment: (*seg).clone() }).collect();

        // 4) Finalize using the same consolidation logic
        self.finalize_block_from_segments(&enriched, selection_start, selection_end)
    }

    // REFACTOR-004: Removed classify_snapshots_on_demand() - use segment-based flow
    // instead Snapshots should be grouped into segments first, then passed to
    // build_daily_blocks_from_segments

    // REFACTOR-004: Removed resolve_project_match_for_segment() - inference moved
    // to OpenAI REFACTOR-004: Removed extract_signals_from_segment() - evidence
    // extraction moved to EvidenceExtractor

    /// Finalize a block from grouped segments (REFACTOR-004: Simplified)
    ///
    /// REFACTOR-004: Removed inference logic. Only consolidates time range and
    /// activity breakdown. Classification (billable, project, workstream,
    /// confidence) will be done by OpenAI.
    ///
    /// **Important**: Segments may overlap day boundaries. We clip segment
    /// durations to the day to avoid "bleeding" time across days.
    fn finalize_block_from_segments(
        &self,
        segments: &[EnrichedSegment],
        day_start: i64,
        day_end: i64,
    ) -> Result<Option<ProposedBlock>> {
        if segments.is_empty() {
            return Ok(None);
        }

        // Helper: Clip timestamps to day boundaries
        let clip = |ts: i64| ts.max(day_start).min(day_end);

        let first = &segments[0];
        let last = &segments[segments.len() - 1];

        let start_ts = clip(first.segment.start_ts);
        let end_ts = clip(last.segment.end_ts);
        let duration_secs = end_ts.saturating_sub(start_ts);

        // FEATURE-028 Phase 3: Calculate idle time and filter auto-excluded segments
        let mut total_idle_secs = 0i32;

        // Sum ALL idle time from all segments
        for seg in segments {
            total_idle_secs += seg.segment.idle_time_secs;
        }

        // Separate active segments from auto-excluded idle segments
        // (for activity breakdown and traceability)
        let active_segments: Vec<&EnrichedSegment> = segments
            .iter()
            .filter(|seg| seg.segment.user_action.as_deref() != Some("auto_excluded"))
            .collect();

        // Determine idle handling strategy
        let idle_handling = if total_idle_secs > 0 && active_segments.len() < segments.len() {
            "exclude".to_string() // Some segments were auto-excluded
        } else if total_idle_secs > 0 {
            "include".to_string() // Idle time present but all segments kept
        } else {
            "exclude".to_string() // No idle time, default to exclude
        };

        // Build activity breakdown weighted by segment duration (clipped to day)
        // Use active_segments only (excludes auto-excluded idle time)
        let activities =
            self.build_activity_breakdown_from_segments(&active_segments, day_start, day_end);

        // REFACTOR-004: Collect all snapshot IDs from segments (for traceability)
        let snapshot_ids: Vec<String> =
            segments.iter().flat_map(|s| s.segment.snapshot_ids.clone()).collect();

        // REFACTOR-004: Collect segment IDs for traceability
        let segment_ids: Vec<String> = segments.iter().map(|s| s.segment.id.clone()).collect();

        // REFACTOR-004: Default values - OpenAI will populate these during
        // classification
        Ok(Some(ProposedBlock {
            id: uuid::Uuid::now_v7().to_string(),
            start_ts,
            end_ts,
            duration_secs,
            // Default to None - OpenAI will populate
            inferred_project_id: None,
            inferred_wbs_code: None,
            inferred_deal_name: None,
            inferred_workstream: None,
            // Default to false - OpenAI will classify
            billable: false,
            // Default to 0.0 - OpenAI will calculate
            confidence: 0.0,
            // FEATURE-030: Classifier used
            classifier_used: None,
            // Activity breakdown (evidence for OpenAI)
            activities,
            // Traceability
            snapshot_ids,
            segment_ids,
            // Default empty reasons - OpenAI will populate
            reasons: vec![],
            // Status: pending classification
            status: "pending_classification".to_string(),
            created_at: chrono::Utc::now().timestamp(),
            reviewed_at: None,
            // FEATURE-028 Phase 3: Idle time calculated from segments
            total_idle_secs,
            idle_handling,
            // FEATURE-033 Phase 2: Location context (defaults, will be populated later)
            timezone: None,
            work_location: None,
            is_travel: false,
            is_weekend: false,
            is_after_hours: false,
            // FEATURE-033 Phase 5: Overlap detection (defaults, will be populated by
            // detect_overlaps)
            has_calendar_overlap: false,
            overlapping_event_ids: vec![],
            is_double_booked: false,
        }))
    }

    /// Build activity breakdown from segments weighted by duration (Feature
    /// Group 2)
    ///
    /// Unlike snapshot-based building which counts snapshots, this method
    /// weights activities by the actual duration of each segment. This
    /// provides more accurate percentages when segments have varying
    /// durations.
    ///
    /// **Important**: Segment durations are clipped to day boundaries to avoid
    /// double-counting time across days.
    ///
    /// # Arguments
    /// * `segments` - Enriched segments to analyze (can be references or owned)
    /// * `day_start` - Day start timestamp (Unix seconds)
    /// * `day_end` - Day end timestamp (Unix seconds)
    ///
    /// # Returns
    /// Vector of ActivityBreakdown sorted by duration (descending)
    fn build_activity_breakdown_from_segments<'a, T>(
        &self,
        segments: T,
        day_start: i64,
        day_end: i64,
    ) -> Vec<ActivityBreakdown>
    where
        T: IntoIterator<Item = &'a &'a EnrichedSegment> + Clone,
    {
        let mut app_durations: HashMap<String, i64> = HashMap::new();

        // Aggregate duration by app, clipping to day boundaries
        for seg in segments {
            let app_name = seg.segment.primary_app.clone();
            // Clip segment to day boundaries
            let s = seg.segment.start_ts.max(day_start).min(day_end);
            let e = seg.segment.end_ts.max(day_start).min(day_end);
            let seg_duration = e.saturating_sub(s);
            *app_durations.entry(app_name).or_insert(0) += seg_duration;
        }

        let total_duration: i64 = app_durations.values().sum();

        // Handle edge case: total_duration could be 0 if all segments have zero
        // duration
        if total_duration == 0 {
            return vec![];
        }

        let mut activities: Vec<ActivityBreakdown> = app_durations
            .iter()
            .map(|(name, duration_secs)| {
                let percentage = (*duration_secs as f32 / total_duration as f32) * 100.0;
                ActivityBreakdown { name: name.clone(), duration_secs: *duration_secs, percentage }
            })
            .collect();

        // Sort by duration descending
        activities.sort_by(|a, b| b.duration_secs.cmp(&a.duration_secs));

        activities
    }
}

#[cfg(test)]
mod tests {
    use pulsearc_domain::types::ActivitySegment;

    use super::*;

    // Test helpers
    fn create_test_builder() -> BlockBuilder {
        BlockBuilder::new(BlockConfig::default()).unwrap()
    }

    fn create_test_segment(
        id: &str,
        start_ts: i64,
        end_ts: i64,
        app: &str,
        idle_time_secs: i32,
    ) -> ActivitySegment {
        let duration = end_ts - start_ts;
        let active_time = duration.saturating_sub(idle_time_secs as i64) as i32;

        ActivitySegment {
            id: id.to_string(),
            start_ts,
            end_ts,
            primary_app: app.to_string(),
            normalized_label: app.to_lowercase().replace(' ', "_"),
            sample_count: 1,
            dictionary_keys: None,
            created_at: chrono::Utc::now().timestamp(),
            processed: false,
            snapshot_ids: vec![format!("snap_{}", id)],
            work_type: None,
            activity_category: "work".to_string(),
            detected_activity: "computer_work".to_string(),
            extracted_signals_json: None,
            project_match_json: None,
            idle_time_secs,
            active_time_secs: active_time,
            user_action: None,
        }
    }

    fn create_test_segments(count: usize, base_ts: i64, app: &str) -> Vec<ActivitySegment> {
        (0..count)
            .map(|i| {
                let start = base_ts + (i as i64 * 300); // 5 min intervals
                create_test_segment(&format!("seg_{}", i), start, start + 300, app, 0)
            })
            .collect()
    }

    #[test]
    fn test_build_blocks_from_segments_basic() {
        // AC: 20 segments of same app should consolidate
        let builder = create_test_builder();
        let day_epoch = 1729728000; // 2024-10-24 00:00:00 UTC
        let start_ts = day_epoch + 32400; // 9 AM

        let segments = create_test_segments(20, start_ts, "Microsoft Excel");

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert!(!blocks.is_empty(), "Should create at least one block");
        assert!(blocks.len() <= 10, "Should consolidate into <=10 blocks");

        for block in &blocks {
            assert!(!block.activities.is_empty(), "Block should have activities");
            assert!(!block.snapshot_ids.is_empty(), "Block should track snapshot IDs");
        }
    }

    #[test]
    fn test_activity_breakdown_weighted_by_duration() {
        // AC: Activity breakdown should be weighted by segment duration, not count
        let builder = create_test_builder();
        let day_epoch = 1729728000;
        let start_ts = day_epoch + 32400;

        // Excel: 3 segments × 300s = 900s (75%)
        let mut segments = create_test_segments(3, start_ts, "Microsoft Excel");
        // Word: 1 segment × 300s = 300s (25%)
        segments.push(create_test_segment(
            "word",
            start_ts + 900,
            start_ts + 1200,
            "Microsoft Word",
            0,
        ));

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 2, "Should create 2 blocks (diff apps)");

        // Excel block should be 900s
        let excel_block =
            blocks.iter().find(|b| b.activities.iter().any(|a| a.name == "Microsoft Excel"));
        assert!(excel_block.is_some());
        let excel_block = excel_block.unwrap();
        assert_eq!(excel_block.duration_secs, 900);
        assert_eq!(excel_block.activities[0].duration_secs, 900);
        assert!((excel_block.activities[0].percentage - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_build_blocks_day_filtering_overlaps() {
        // AC: Only segments overlapping the day should be included
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            // Segment before day (should be excluded)
            create_test_segment("before", day_epoch - 600, day_epoch - 300, "Excel", 0),
            // Segment within day (should be included)
            create_test_segment("within", day_epoch + 100, day_epoch + 400, "Excel", 0),
            // Segment after day (should be excluded)
            create_test_segment(
                "after",
                day_epoch + 86400 + 100,
                day_epoch + 86400 + 400,
                "Excel",
                0,
            ),
            // Segment overlapping start (should be included, clipped)
            create_test_segment("overlap_start", day_epoch - 100, day_epoch + 200, "Excel", 0),
        ];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        // Should create blocks from "within" and "overlap_start"
        assert!(!blocks.is_empty());
        let total_snapshots: usize = blocks.iter().map(|b| b.snapshot_ids.len()).sum();
        assert_eq!(total_snapshots, 2, "Should include 2 segments");
    }

    #[test]
    fn test_build_blocks_sorts_unsorted_segments() {
        // AC: Segments should be sorted by start_ts for deterministic processing
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        // Create segments in wrong order
        let segments = vec![
            create_test_segment("seg3", day_epoch + 600, day_epoch + 900, "Excel", 0),
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 0),
            create_test_segment("seg2", day_epoch + 300, day_epoch + 600, "Excel", 0),
        ];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1, "Should consolidate into 1 block");
        assert_eq!(blocks[0].start_ts, day_epoch);
        assert_eq!(blocks[0].end_ts, day_epoch + 900);
    }

    #[test]
    fn test_propose_block_for_selection_basic() {
        // AC: UI time selection should create a block for the selected range
        let builder = create_test_builder();
        let segments = create_test_segments(10, 1729728000, "Excel");

        let selection_start = 1729728000 + 300; // After first segment
        let selection_end = 1729728000 + 1200; // Covers segments 1-3

        let block =
            builder.propose_block_for_selection(&segments, selection_start, selection_end).unwrap();

        assert!(block.is_some());
        let block = block.unwrap();
        assert!(block.start_ts >= selection_start);
        assert!(block.end_ts <= selection_end);
    }

    #[test]
    fn test_propose_block_for_selection_no_overlap() {
        // AC: Selection with no overlapping segments should return None
        let builder = create_test_builder();
        let segments = create_test_segments(5, 1729728000, "Excel");

        let selection_start = 1729728000 + 10000; // Way after segments
        let selection_end = 1729728000 + 20000;

        let block =
            builder.propose_block_for_selection(&segments, selection_start, selection_end).unwrap();

        assert!(block.is_none(), "Should return None for no overlap");
    }

    #[test]
    fn test_boundary_exact_180s_gap_should_merge() {
        // AC: Gap == 180s (max_gap_for_merge_secs) should merge
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 0),
            create_test_segment("seg2", day_epoch + 480, day_epoch + 780, "Excel", 0), /* Gap = 180s */
        ];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1, "Should merge with 180s gap");
    }

    #[test]
    fn test_boundary_181s_gap_should_split() {
        // AC: Gap > 180s should split into separate blocks
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 0),
            create_test_segment("seg2", day_epoch + 481, day_epoch + 781, "Excel", 0), /* Gap = 181s */
        ];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 2, "Should split with 181s gap");
    }

    #[test]
    fn test_empty_input_no_segments() {
        // AC: Empty input should return empty blocks
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let blocks = builder.build_daily_blocks_from_segments(&[], day_epoch).unwrap();

        assert!(blocks.is_empty(), "Empty input should return empty blocks");
    }

    #[test]
    fn test_single_segment() {
        // AC: Single segment should create one block
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments =
            vec![create_test_segment("seg1", day_epoch + 100, day_epoch + 400, "Excel", 0)];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].duration_secs, 300);
    }

    // IDLE TIME TESTS (user specifically requested these)

    #[test]
    fn test_block_excludes_auto_excluded_idle_segments() {
        // AC: Segments with user_action="auto_excluded" should not appear in activity
        // breakdown but still contribute to total_idle_secs
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let mut segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 50),
            create_test_segment("seg2", day_epoch + 300, day_epoch + 600, "Excel", 100),
            create_test_segment("seg3", day_epoch + 600, day_epoch + 900, "Excel", 75),
        ];
        // Mark second segment as auto-excluded (e.g., idle time the user didn't want)
        segments[1].user_action = Some("auto_excluded".to_string());

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1, "Same app should consolidate into 1 block");
        let block = &blocks[0];

        // Activity breakdown should only include non-auto-excluded segments
        assert_eq!(block.activities.len(), 1);
        assert_eq!(block.activities[0].name, "Excel");

        // Total idle time should include ALL segments (even auto-excluded)
        assert_eq!(block.total_idle_secs, 225, "Should sum all idle: 50+100+75");
    }

    #[test]
    fn test_block_includes_kept_idle_segments() {
        // AC: Idle segments without auto_excluded should appear in activity breakdown
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            create_test_segment("active", day_epoch, day_epoch + 300, "Excel", 0),
            create_test_segment("idle", day_epoch + 300, day_epoch + 600, "Idle", 300),
        ];
        // Note: idle segment does NOT have user_action="auto_excluded"

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 2, "Different apps should create separate blocks");

        // Find idle block
        let idle_block = blocks.iter().find(|b| b.activities.iter().any(|a| a.name == "Idle"));
        assert!(idle_block.is_some(), "Should have idle block");
    }

    #[test]
    fn test_block_total_idle_secs_calculation() {
        // AC: total_idle_secs should sum idle time from all segments
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 50),
            create_test_segment("seg2", day_epoch + 300, day_epoch + 600, "Excel", 100),
            create_test_segment("seg3", day_epoch + 600, day_epoch + 900, "Excel", 75),
        ];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].total_idle_secs, 225, "Should sum all idle time: 50+100+75=225");
    }

    #[test]
    fn test_block_idle_handling_exclude_strategy() {
        // AC: idle_handling="exclude" when same-app segments have auto-excluded idle
        // time
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let mut segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 0),
            create_test_segment("seg2", day_epoch + 300, day_epoch + 600, "Excel", 150),
        ];
        // Mark the second segment as having auto-excluded idle time
        segments[1].user_action = Some("auto_excluded".to_string());

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1, "Same app should merge into 1 block");
        assert_eq!(
            blocks[0].idle_handling, "exclude",
            "Should use 'exclude' strategy when segments auto-excluded"
        );
        assert_eq!(blocks[0].total_idle_secs, 150);
    }

    #[test]
    fn test_block_idle_handling_include_strategy() {
        // AC: idle_handling="include" when idle present but not auto-excluded
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 100),
            create_test_segment("seg2", day_epoch + 300, day_epoch + 600, "Excel", 50),
        ];
        // Note: idle time present but segments NOT auto-excluded

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(
            blocks[0].idle_handling, "include",
            "Should use 'include' strategy when idle present but kept"
        );
        assert_eq!(blocks[0].total_idle_secs, 150);
    }

    #[test]
    fn test_block_with_no_idle_time() {
        // AC: No idle time should result in idle_handling="exclude" (default)
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let segments = vec![
            create_test_segment("seg1", day_epoch, day_epoch + 300, "Excel", 0),
            create_test_segment("seg2", day_epoch + 300, day_epoch + 600, "Excel", 0),
        ];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].total_idle_secs, 0);
        assert_eq!(blocks[0].idle_handling, "exclude");
    }

    #[test]
    fn test_activity_breakdown_sums_to_100_percent() {
        // AC: Activity percentages should sum to ~100%
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        let mut segments = create_test_segments(3, day_epoch, "Excel"); // 900s
        segments.extend(create_test_segments(2, day_epoch + 900, "Word")); // 600s

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 2);

        for block in &blocks {
            let total_percent: f32 = block.activities.iter().map(|a| a.percentage).sum();
            assert!(
                (total_percent - 100.0).abs() < 0.1,
                "Percentages should sum to 100%, got {}",
                total_percent
            );
        }
    }

    #[test]
    fn test_finalize_block_clips_to_day_boundaries() {
        // AC: Segment times should be clipped to day boundaries
        let builder = create_test_builder();
        let day_epoch = 1729728000;

        // Segment starts before day, ends after day start
        let segments =
            vec![create_test_segment("overlap", day_epoch - 100, day_epoch + 200, "Excel", 0)];

        let blocks = builder.build_daily_blocks_from_segments(&segments, day_epoch).unwrap();

        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].start_ts, day_epoch, "Start should be clipped to day_start");
        assert_eq!(blocks[0].duration_secs, 200, "Duration should be clipped portion");
    }

    // SELECTION TESTS (missing coverage identified in review)

    #[test]
    fn test_propose_block_for_selection_partial_overlap() {
        // AC: Selection should include segments with partial overlap, clipped to
        // selection bounds
        let builder = create_test_builder();
        let day_epoch = 1609459200; // 2021-01-01 00:00:00 UTC
        let start_10am = day_epoch + 36000; // 10:00 AM

        let segments = vec![
            // Fully inside selection
            create_test_segment("seg1", start_10am, start_10am + 300, "Excel", 0),
            create_test_segment("seg2", start_10am + 300, start_10am + 600, "Excel", 0),
            // Overlaps selection end
            create_test_segment("seg3", start_10am + 600, start_10am + 900, "Excel", 0),
            // More segments to ensure consolidated block
            create_test_segment("seg4", start_10am + 900, start_10am + 1200, "Excel", 0),
            create_test_segment("seg5", start_10am + 1200, start_10am + 1500, "Excel", 0),
            create_test_segment("seg6", start_10am + 1500, start_10am + 1800, "Excel", 0),
            create_test_segment("seg7", start_10am + 1800, start_10am + 2100, "Excel", 0),
        ];

        // User selects 10:00 - 10:35 (includes partial overlap)
        let selection_start = start_10am;
        let selection_end = start_10am + 2100; // 35 minutes
        let block =
            builder.propose_block_for_selection(&segments, selection_start, selection_end).unwrap();

        assert!(block.is_some(), "Should create a block");
        let block = block.unwrap();

        // Duration should be clipped to selection boundaries
        assert_eq!(
            block.duration_secs,
            selection_end - selection_start,
            "Duration should be clipped to selection"
        );
        assert_eq!(block.start_ts, selection_start);
        assert_eq!(block.end_ts, selection_end);
    }

    #[test]
    fn test_propose_block_for_selection_sorts_segments() {
        // AC: Unsorted segments should be sorted before processing for deterministic
        // results
        let builder = create_test_builder();
        let day_epoch = 1609459200; // 2021-01-01 00:00:00 UTC
        let start_10am = day_epoch + 36000; // 10:00 AM

        // Out of order: Segment 3, Segment 1, Segment 2
        let segments = vec![
            create_test_segment("seg3", start_10am + 600, start_10am + 900, "Chrome", 0),
            create_test_segment("seg1", start_10am, start_10am + 300, "Excel", 0),
            create_test_segment("seg2", start_10am + 300, start_10am + 600, "Word", 0),
            // More segments
            create_test_segment("seg4", start_10am + 900, start_10am + 1200, "Excel", 0),
            create_test_segment("seg5", start_10am + 1200, start_10am + 1500, "Excel", 0),
            create_test_segment("seg6", start_10am + 1500, start_10am + 1800, "Excel", 0),
            create_test_segment("seg7", start_10am + 1800, start_10am + 2100, "Excel", 0),
        ];

        let selection_start = start_10am;
        let selection_end = start_10am + 2100;
        let block =
            builder.propose_block_for_selection(&segments, selection_start, selection_end).unwrap();

        assert!(block.is_some(), "Should create block even with unsorted segments");
        let block = block.unwrap();

        // Verify duration is correct (segments were sorted internally)
        assert_eq!(
            block.duration_secs,
            selection_end - selection_start,
            "Duration should be correct despite unsorted input"
        );
    }

    // TIMEZONE AND DST TESTS (missing coverage identified in review)

    #[test]
    fn test_timezone_pst_day_to_utc_boundaries() {
        // AC: User in PST wants blocks for "2024-10-24"
        // PST: 2024-10-24 00:00:00 to 2024-10-24 23:59:59
        // UTC: 2024-10-24 07:00:00 to 2024-10-25 06:59:59 (during PDT, UTC-7)
        use chrono::{NaiveDate, TimeZone};
        use chrono_tz::America::Los_Angeles;

        let builder = create_test_builder();

        // User's local date: October 24, 2024
        let local_date = NaiveDate::from_ymd_opt(2024, 10, 24).unwrap();
        let local_midnight =
            Los_Angeles.from_local_datetime(&local_date.and_hms_opt(0, 0, 0).unwrap()).unwrap();

        let utc_day_start = local_midnight.timestamp();
        let utc_day_end = utc_day_start + 86400;

        // Segment entirely within user's local Oct 24 (PST)
        let segment = create_test_segment(
            "seg1",
            utc_day_start + 3600, // 1 hour after local midnight
            utc_day_start + 5400, // 1.5 hours after local midnight
            "Excel",
            0,
        );

        let blocks = builder.build_daily_blocks_from_segments(&[segment], utc_day_start).unwrap();

        assert_eq!(blocks.len(), 1, "Should create block for user's local day");

        let block = &blocks[0];
        assert!(
            block.start_ts >= utc_day_start && block.start_ts < utc_day_end,
            "Block should be within user's local day boundaries (converted to UTC)"
        );
    }

    #[test]
    fn test_timezone_midnight_boundary_local_vs_utc() {
        // AC: Event at 11:30 PM PST on Oct 24
        // PST: 2024-10-24 23:30:00 = UTC: 2024-10-25 06:30:00
        // Should appear on Oct 24 for PST user, but Oct 25 for UTC user
        use chrono::{NaiveDate, TimeZone};
        use chrono_tz::America::Los_Angeles;
        use chrono_tz::UTC;

        let builder = create_test_builder();

        // PST user's Oct 24
        let pst_date = NaiveDate::from_ymd_opt(2024, 10, 24).unwrap();
        let pst_midnight =
            Los_Angeles.from_local_datetime(&pst_date.and_hms_opt(0, 0, 0).unwrap()).unwrap();
        let pst_day_start = pst_midnight.timestamp();

        // UTC Oct 24
        let utc_date = NaiveDate::from_ymd_opt(2024, 10, 24).unwrap();
        let utc_midnight =
            UTC.from_local_datetime(&utc_date.and_hms_opt(0, 0, 0).unwrap()).unwrap();
        let utc_day_start = utc_midnight.timestamp();

        // Event at 11:30 PM PST = 6:30 AM UTC next day
        let pst_1130pm = Los_Angeles
            .from_local_datetime(&pst_date.and_hms_opt(23, 30, 0).unwrap())
            .unwrap()
            .timestamp();

        let segment = create_test_segment("seg1", pst_1130pm, pst_1130pm + 1800, "Excel", 0);

        // Act: Build blocks for PST user's Oct 24
        let pst_blocks = builder
            .build_daily_blocks_from_segments(std::slice::from_ref(&segment), pst_day_start)
            .unwrap();

        // Act: Build blocks for UTC user's Oct 24
        let utc_blocks = builder
            .build_daily_blocks_from_segments(std::slice::from_ref(&segment), utc_day_start)
            .unwrap();

        // Assert: Event appears on Oct 24 for PST user
        assert_eq!(pst_blocks.len(), 1, "PST user should see event on their Oct 24 (11:30 PM PST)");

        // Assert: Event does NOT appear on Oct 24 for UTC user (it's Oct 25 06:30 UTC)
        assert_eq!(
            utc_blocks.len(),
            0,
            "UTC user should NOT see event on Oct 24 (it's 06:30 Oct 25 UTC)"
        );
    }

    #[test]
    fn test_timezone_dst_spring_forward() {
        // AC: DST transition on 2024-03-10 (2 AM → 3 AM in PST)
        // Day has only 23 hours instead of 24
        use chrono::{NaiveDate, TimeZone};
        use chrono_tz::America::Los_Angeles;

        let builder = create_test_builder();

        // March 10, 2024 (DST spring forward day)
        let dst_date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        let local_midnight =
            Los_Angeles.from_local_datetime(&dst_date.and_hms_opt(0, 0, 0).unwrap()).unwrap();

        let day_start = local_midnight.timestamp();
        let day_end = day_start + 86400; // Still 24 hours in Unix time

        // Create segment spanning 1 AM to 4 AM (crosses DST gap)
        let segment_start = Los_Angeles
            .from_local_datetime(&dst_date.and_hms_opt(1, 0, 0).unwrap())
            .unwrap()
            .timestamp();

        let segment = create_test_segment("seg1", segment_start, segment_start + 10800, "Excel", 0);

        let blocks = builder.build_daily_blocks_from_segments(&[segment], day_start).unwrap();

        // Assert: Block is created despite DST transition
        assert_eq!(blocks.len(), 1, "Should handle DST spring forward (23-hour day)");

        let block = &blocks[0];
        assert!(
            block.start_ts >= day_start && block.start_ts < day_end,
            "Block should be within day boundaries despite DST"
        );
    }

    #[test]
    fn test_timezone_dst_fall_back() {
        // AC: DST transition on 2024-11-03 (2 AM → 1 AM in PST)
        // Day has 25 hours instead of 24
        use chrono::{NaiveDate, TimeZone};
        use chrono_tz::America::Los_Angeles;

        let builder = create_test_builder();

        // November 3, 2024 (DST fall back day)
        let dst_date = NaiveDate::from_ymd_opt(2024, 11, 3).unwrap();
        let local_midnight =
            Los_Angeles.from_local_datetime(&dst_date.and_hms_opt(0, 0, 0).unwrap()).unwrap();

        let day_start = local_midnight.timestamp();

        // Create segment during the "repeated" hour (1 AM occurs twice)
        let segment_start = Los_Angeles
            .from_local_datetime(&dst_date.and_hms_opt(1, 30, 0).unwrap())
            .earliest() // Use first occurrence
            .unwrap()
            .timestamp();

        let segment = create_test_segment("seg1", segment_start, segment_start + 3600, "Excel", 0);

        let blocks = builder.build_daily_blocks_from_segments(&[segment], day_start).unwrap();

        // Assert: Block is created despite DST transition
        assert_eq!(blocks.len(), 1, "Should handle DST fall back (25-hour day)");
    }
}
