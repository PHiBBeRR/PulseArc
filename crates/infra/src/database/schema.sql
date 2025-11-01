CREATE TABLE IF NOT EXISTS activity_snapshots (
            id TEXT PRIMARY KEY,
            timestamp INTEGER NOT NULL,
            activity_context_json TEXT NOT NULL,
            detected_activity TEXT NOT NULL,
            work_type TEXT,
            activity_category TEXT,
            primary_app TEXT NOT NULL,
            processed BOOLEAN NOT NULL DEFAULT 0,
            batch_id TEXT,
            created_at INTEGER NOT NULL,
            processed_at INTEGER,
            is_idle INTEGER NOT NULL DEFAULT 0,
            idle_duration_secs INTEGER
        );
CREATE INDEX IF NOT EXISTS idx_activity_unprocessed 
         ON activity_snapshots(processed, timestamp);
CREATE INDEX IF NOT EXISTS idx_activity_batch 
         ON activity_snapshots(batch_id);
CREATE INDEX IF NOT EXISTS idx_activity_type 
         ON activity_snapshots(detected_activity, work_type, timestamp) 
         WHERE processed = 0;
CREATE INDEX IF NOT EXISTS idx_snapshots_recent 
         ON activity_snapshots(timestamp DESC, processed);
CREATE TABLE IF NOT EXISTS time_entries (
            id TEXT PRIMARY KEY,
            start_time INTEGER NOT NULL,
            end_time INTEGER,
            duration_seconds INTEGER,
            description TEXT NOT NULL,
            project_id TEXT,
            wbs_code TEXT
        );
CREATE INDEX IF NOT EXISTS idx_entries_start_time
         ON time_entries(start_time);
CREATE TABLE IF NOT EXISTS activity_segments (
            id TEXT PRIMARY KEY,
            start_ts INTEGER NOT NULL,
            end_ts INTEGER NOT NULL,
            primary_app TEXT NOT NULL,
            normalized_label TEXT NOT NULL,
            sample_count INTEGER NOT NULL,
            dictionary_keys TEXT,
            created_at INTEGER NOT NULL,
            processed BOOLEAN NOT NULL DEFAULT 0,
            snapshot_ids TEXT,
            work_type TEXT,
            activity_category TEXT DEFAULT 'unknown',
            detected_activity TEXT DEFAULT 'working',
            idle_time_secs INTEGER NOT NULL DEFAULT 0,
            active_time_secs INTEGER NOT NULL DEFAULT 0,
            user_action TEXT
        );
CREATE INDEX IF NOT EXISTS idx_segments_window 
         ON activity_segments(start_ts, end_ts);
CREATE INDEX IF NOT EXISTS idx_segments_label 
         ON activity_segments(primary_app, normalized_label);
CREATE INDEX IF NOT EXISTS idx_activity_segments_composite 
         ON activity_segments(primary_app, normalized_label, start_ts);
CREATE TABLE IF NOT EXISTS batch_queue (
            batch_id TEXT PRIMARY KEY,
            activity_count INTEGER NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            processed_at INTEGER,
            error_message TEXT,
            processing_started_at INTEGER,
            worker_id TEXT,
            lease_expires_at INTEGER,
            time_entries_created INTEGER NOT NULL DEFAULT 0,
            openai_cost REAL NOT NULL DEFAULT 0.0
        );
CREATE INDEX IF NOT EXISTS idx_batch_status_created 
         ON batch_queue(status, created_at);
CREATE TABLE IF NOT EXISTS time_entry_outbox (
            id TEXT PRIMARY KEY,
            idempotency_key TEXT NOT NULL,
            user_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            backend_cuid TEXT,
            status TEXT NOT NULL DEFAULT 'pending',
            attempts INTEGER NOT NULL DEFAULT 0,
            last_error TEXT,
            retry_after INTEGER,
            created_at INTEGER NOT NULL,
            sent_at INTEGER,
            correlation_id TEXT NOT NULL DEFAULT '',
            local_status TEXT NOT NULL DEFAULT 'pending',
            remote_status TEXT,
            sap_entry_id TEXT,
            next_attempt_at INTEGER,
            error_code TEXT,
            last_forwarded_at INTEGER,
            wbs_code TEXT,
            target TEXT NOT NULL DEFAULT 'sap' CHECK(target IN ('sap', 'main_api')),
            description TEXT,
            auto_applied INTEGER NOT NULL DEFAULT 1,
            version INTEGER NOT NULL DEFAULT 1,
            last_modified_by TEXT NOT NULL DEFAULT 'DESKTOP',
            last_modified_at INTEGER
        );
CREATE UNIQUE INDEX IF NOT EXISTS idx_outbox_idempotency_target
         ON time_entry_outbox(idempotency_key, target);
CREATE INDEX IF NOT EXISTS idx_outbox_status 
         ON time_entry_outbox(status, created_at);
CREATE INDEX IF NOT EXISTS idx_outbox_user 
         ON time_entry_outbox(user_id, status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_outbox_correlation ON time_entry_outbox(correlation_id) WHERE correlation_id != '';
CREATE INDEX IF NOT EXISTS idx_outbox_next_attempt ON time_entry_outbox(local_status, next_attempt_at) WHERE local_status = 'pending';
CREATE INDEX IF NOT EXISTS idx_outbox_local_status ON time_entry_outbox(local_status);
CREATE TABLE IF NOT EXISTS id_mapping (
            local_uuid TEXT PRIMARY KEY,
            backend_cuid TEXT UNIQUE,
            entity_type TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_mapping_cuid 
         ON id_mapping(backend_cuid);
CREATE INDEX IF NOT EXISTS idx_mapping_type 
         ON id_mapping(entity_type, created_at);
CREATE TABLE IF NOT EXISTS token_usage (
            id TEXT PRIMARY KEY,
            batch_id TEXT NOT NULL,
            user_id TEXT NOT NULL,
            input_tokens INTEGER NOT NULL,
            output_tokens INTEGER NOT NULL,
            estimated_cost_usd REAL NOT NULL,
            is_actual BOOLEAN NOT NULL DEFAULT 0,
            timestamp INTEGER NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_token_usage_user_timestamp 
         ON token_usage(user_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_token_usage_batch 
         ON token_usage(batch_id);
CREATE TABLE IF NOT EXISTS batch_dlq (
            batch_id TEXT PRIMARY KEY,
            activity_count INTEGER NOT NULL,
            original_status TEXT NOT NULL,
            error_message TEXT NOT NULL,
            error_code TEXT,
            created_at INTEGER NOT NULL,
            failed_at INTEGER NOT NULL,
            attempts INTEGER NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_dlq_failed_at 
         ON batch_dlq(failed_at);
CREATE TABLE IF NOT EXISTS metrics_snapshots (
            id TEXT PRIMARY KEY,
            test_id TEXT,
            timestamp INTEGER NOT NULL,
            monthly_cost_usd REAL NOT NULL DEFAULT 0.0,
            total_input_tokens INTEGER NOT NULL DEFAULT 0,
            total_output_tokens INTEGER NOT NULL DEFAULT 0,
            batch_count INTEGER NOT NULL DEFAULT 0,
            batch_success_count INTEGER NOT NULL DEFAULT 0,
            batch_failure_count INTEGER NOT NULL DEFAULT 0,
            dlq_entry_count INTEGER NOT NULL DEFAULT 0,
            retry_attempts_total INTEGER NOT NULL DEFAULT 0,
            avg_fetch_time_ms REAL NOT NULL DEFAULT 0.0,
            cache_hit_rate REAL NOT NULL DEFAULT 0.0,
            created_at INTEGER NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_test
         ON metrics_snapshots(test_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_metrics_snapshots_time
         ON metrics_snapshots(timestamp);
CREATE TABLE IF NOT EXISTS calendar_tokens (
            id TEXT PRIMARY KEY,
            token_ref TEXT NOT NULL UNIQUE,
            user_email TEXT NOT NULL,
            expires_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE,
            provider TEXT NOT NULL DEFAULT 'google'
        );
CREATE INDEX IF NOT EXISTS idx_calendar_tokens_email
         ON calendar_tokens(user_email);
CREATE UNIQUE INDEX IF NOT EXISTS idx_calendar_tokens_provider
         ON calendar_tokens(provider);
CREATE TABLE IF NOT EXISTS calendar_sync_settings (
            id TEXT PRIMARY KEY,
            user_email TEXT NOT NULL UNIQUE,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            sync_interval_minutes INTEGER NOT NULL DEFAULT 30,
            include_all_day_events BOOLEAN NOT NULL DEFAULT 1,
            min_event_duration_minutes INTEGER NOT NULL DEFAULT 15,
            lookback_hours INTEGER NOT NULL DEFAULT 336,
            lookahead_hours INTEGER NOT NULL DEFAULT 168,
            excluded_calendar_ids TEXT NOT NULL DEFAULT '',
            sync_token TEXT,
            last_sync_epoch INTEGER,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            idempotency_key TEXT NOT NULL UNIQUE
        );
CREATE INDEX IF NOT EXISTS idx_calendar_sync_email
         ON calendar_sync_settings(user_email);
CREATE TABLE IF NOT EXISTS calendar_events (
            id TEXT PRIMARY KEY,
            google_event_id TEXT NOT NULL,
            user_email TEXT NOT NULL,
            summary TEXT NOT NULL,
            description TEXT,
            start_ts INTEGER NOT NULL,
            end_ts INTEGER NOT NULL,
            is_all_day BOOLEAN NOT NULL DEFAULT 0,
            recurring_event_id TEXT,
            parsed_project TEXT,
            parsed_workstream TEXT,
            parsed_task TEXT,
            confidence_score REAL,
            meeting_platform TEXT,
            is_recurring_series BOOLEAN NOT NULL DEFAULT 0,
            is_online_meeting BOOLEAN NOT NULL DEFAULT 0,
            has_external_attendees BOOLEAN,
            organizer_email TEXT,
            organizer_domain TEXT,
            meeting_id TEXT,
            attendee_count INTEGER,
            external_attendee_count INTEGER,
            created_at INTEGER NOT NULL,
            UNIQUE(google_event_id, user_email)
        );
CREATE INDEX IF NOT EXISTS idx_calendar_events_time_range
         ON calendar_events(user_email, start_ts, end_ts);
CREATE INDEX IF NOT EXISTS idx_calendar_events_cleanup
         ON calendar_events(created_at);
CREATE INDEX IF NOT EXISTS idx_calendar_today
         ON calendar_events(start_ts);
CREATE INDEX IF NOT EXISTS idx_calendar_organizer_domain
         ON calendar_events(organizer_domain);
CREATE INDEX IF NOT EXISTS idx_calendar_external
         ON calendar_events(has_external_attendees);
CREATE TABLE IF NOT EXISTS proposed_time_blocks (
            id TEXT PRIMARY KEY,
            start_ts INTEGER NOT NULL,
            end_ts INTEGER NOT NULL,
            duration_secs INTEGER NOT NULL,
            inferred_project_id TEXT,
            inferred_wbs_code TEXT,
            inferred_deal_name TEXT,
            inferred_workstream TEXT,
            billable BOOLEAN NOT NULL DEFAULT 0,
            confidence REAL NOT NULL,
            activities_json TEXT NOT NULL,
            snapshot_ids_json TEXT NOT NULL,
            segment_ids TEXT DEFAULT '[]',
            reasons_json TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'pending',
            created_at INTEGER NOT NULL,
            reviewed_at INTEGER,
            classification_status TEXT DEFAULT 'pending',
            batch_id TEXT,
            classification_attempts INTEGER DEFAULT 0,
            last_classification_attempt INTEGER,
            classification_error TEXT,
            total_idle_secs INTEGER NOT NULL DEFAULT 0,
            idle_handling TEXT NOT NULL DEFAULT 'exclude',
            has_calendar_overlap BOOLEAN NOT NULL DEFAULT 0,
            overlapping_event_ids TEXT DEFAULT '[]',
            is_double_booked BOOLEAN NOT NULL DEFAULT 0,
            timezone TEXT,
            work_location TEXT,
            is_travel BOOLEAN NOT NULL DEFAULT 0,
            is_weekend BOOLEAN NOT NULL DEFAULT 0,
            is_after_hours BOOLEAN NOT NULL DEFAULT 0
        );
CREATE INDEX IF NOT EXISTS idx_proposed_blocks_time_range
         ON proposed_time_blocks(start_ts, end_ts);
CREATE INDEX IF NOT EXISTS idx_proposed_blocks_status
         ON proposed_time_blocks(status, start_ts);
CREATE INDEX IF NOT EXISTS idx_proposed_blocks_classification_status
         ON proposed_time_blocks(classification_status, start_ts);
CREATE INDEX IF NOT EXISTS idx_proposed_blocks_batch_id
         ON proposed_time_blocks(batch_id);
CREATE TABLE IF NOT EXISTS classification_batches (
            id TEXT PRIMARY KEY,
            openai_batch_id TEXT,
            day_epoch INTEGER NOT NULL,
            block_count INTEGER NOT NULL,
            status TEXT NOT NULL,
            created_at INTEGER NOT NULL,
            completed_at INTEGER,
            error_message TEXT
        );
CREATE INDEX IF NOT EXISTS idx_classification_batches_day
         ON classification_batches(day_epoch, status);
CREATE INDEX IF NOT EXISTS idx_classification_batches_openai_id
         ON classification_batches(openai_batch_id);
CREATE TABLE IF NOT EXISTS block_config (
            id INTEGER PRIMARY KEY CHECK(id = 1),
            min_block_duration_secs INTEGER NOT NULL DEFAULT 1800,
            max_gap_for_merge_secs INTEGER NOT NULL DEFAULT 180,
            consolidation_window_secs INTEGER NOT NULL DEFAULT 3600,
            min_billing_increment_secs INTEGER NOT NULL DEFAULT 360,
            auto_build_enabled INTEGER NOT NULL DEFAULT 1,
            auto_build_time TEXT NOT NULL DEFAULT '23:00'
        );
INSERT OR IGNORE INTO block_config (id, min_block_duration_secs, max_gap_for_merge_secs, 
         consolidation_window_secs, min_billing_increment_secs, auto_build_enabled, auto_build_time) 
         VALUES (1, 1800, 180, 3600, 360, 1, '23:00');
CREATE TABLE IF NOT EXISTS suggestion_feedback (
            id TEXT PRIMARY KEY,
            outbox_id TEXT NOT NULL,
            action TEXT NOT NULL CHECK(action IN ('accepted','dismissed','edited','restored')),
            reason TEXT,
            edit_type TEXT,
            confidence_before REAL,
            source TEXT,
            context_json TEXT,
            created_at INTEGER NOT NULL,
            FOREIGN KEY (outbox_id) REFERENCES time_entry_outbox(id) ON DELETE CASCADE
        );
CREATE INDEX IF NOT EXISTS idx_feedback_outbox
         ON suggestion_feedback(outbox_id);
CREATE INDEX IF NOT EXISTS idx_feedback_action
         ON suggestion_feedback(action);
CREATE INDEX IF NOT EXISTS idx_feedback_created
         ON suggestion_feedback(created_at);
CREATE TABLE IF NOT EXISTS wbs_cache (
            wbs_code TEXT PRIMARY KEY,
            project_def TEXT NOT NULL,
            project_name TEXT,
            description TEXT,
            status TEXT NOT NULL,
            cached_at INTEGER NOT NULL,
            expires_at INTEGER NOT NULL,
            last_changed_at INTEGER,
            opportunity_id TEXT,
            deal_name TEXT,
            target_company_name TEXT,
            counterparty TEXT,
            industry TEXT,
            region TEXT,
            amount REAL,
            stage_name TEXT,
            project_code TEXT
        );
CREATE VIRTUAL TABLE IF NOT EXISTS wbs_cache_fts USING fts5(
            wbs_code,
            project_def,
            project_name,
            description,
            status,
            deal_name,
            target_company_name,
            counterparty,
            industry,
            content='wbs_cache',
            tokenize='porter unicode61'
        );
CREATE TRIGGER IF NOT EXISTS wbs_cache_ai AFTER INSERT ON wbs_cache BEGIN
            INSERT INTO wbs_cache_fts(rowid, wbs_code, project_def, project_name, description, status, deal_name, target_company_name, counterparty, industry)
            VALUES (new.rowid, new.wbs_code, new.project_def, new.project_name, new.description, new.status, new.deal_name, new.target_company_name, new.counterparty, new.industry);
        END;
CREATE TRIGGER IF NOT EXISTS wbs_cache_ad AFTER DELETE ON wbs_cache BEGIN
            DELETE FROM wbs_cache_fts WHERE rowid = old.rowid;
        END;
CREATE TRIGGER IF NOT EXISTS wbs_cache_au AFTER UPDATE ON wbs_cache BEGIN
            UPDATE wbs_cache_fts SET
                wbs_code = new.wbs_code,
                project_def = new.project_def,
                project_name = new.project_name,
                description = new.description,
                status = new.status,
                deal_name = new.deal_name,
                target_company_name = new.target_company_name,
                counterparty = new.counterparty,
                industry = new.industry
            WHERE rowid = old.rowid;
        END;
CREATE INDEX IF NOT EXISTS idx_wbs_cache_expires ON wbs_cache(expires_at);
CREATE INDEX IF NOT EXISTS idx_wbs_cache_project ON wbs_cache(project_def);
CREATE INDEX IF NOT EXISTS idx_wbs_cache_status ON wbs_cache(status) WHERE status = 'REL';
CREATE TABLE IF NOT EXISTS sap_cursors (
            resource_key TEXT PRIMARY KEY,
            last_sync_cursor TEXT NOT NULL,
            last_sync_epoch INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
CREATE TABLE IF NOT EXISTS sap_sync_settings (
            id INTEGER PRIMARY KEY,
            enabled BOOLEAN NOT NULL DEFAULT 1,
            sync_interval_hours INTEGER NOT NULL DEFAULT 6,
            last_sync_epoch INTEGER,
            last_sync_status TEXT
        );
INSERT OR IGNORE INTO sap_sync_settings (id, enabled, sync_interval_hours) VALUES (1, 1, 6);
CREATE TABLE IF NOT EXISTS idle_periods (
            id TEXT NOT NULL PRIMARY KEY,
            start_ts INTEGER NOT NULL,
            end_ts INTEGER NOT NULL,
            duration_secs INTEGER NOT NULL,
            system_trigger TEXT NOT NULL,
            user_action TEXT,
            threshold_secs INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            reviewed_at INTEGER,
            notes TEXT,
            UNIQUE(start_ts, end_ts)
        );
CREATE INDEX IF NOT EXISTS idx_idle_periods_time_range
         ON idle_periods(start_ts, end_ts);
CREATE INDEX IF NOT EXISTS idx_idle_periods_user_action
         ON idle_periods(user_action, start_ts);
CREATE TABLE IF NOT EXISTS feature_flags (
            flag_name TEXT PRIMARY KEY,
            enabled INTEGER NOT NULL DEFAULT 0,
            description TEXT,
            updated_at INTEGER NOT NULL
        );
-- Default feature flags for Phase 4 rollback control
INSERT OR IGNORE INTO feature_flags (flag_name, enabled, description, updated_at)
VALUES
    ('new_blocks_cmd', 1, 'Use new block builder infrastructure', CAST(strftime('%s','now') AS INTEGER)),
    ('use_new_infra', 1, 'Enable Phase 4 infrastructure globally', CAST(strftime('%s','now') AS INTEGER));
CREATE TABLE IF NOT EXISTS user_profiles (
            id TEXT NOT NULL PRIMARY KEY,
            auth0_id TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            name TEXT,
            first_name TEXT,
            last_name TEXT,
            display_name TEXT,
            avatar_url TEXT,
            phone_number TEXT,
            title TEXT,
            department TEXT,
            location TEXT,
            bio TEXT,
            timezone TEXT NOT NULL,
            language TEXT NOT NULL,
            locale TEXT NOT NULL,
            date_format TEXT NOT NULL,
            is_active INTEGER NOT NULL DEFAULT 1,
            email_verified INTEGER NOT NULL DEFAULT 0,
            two_factor_enabled INTEGER NOT NULL DEFAULT 0,
            last_login_at INTEGER NOT NULL,
            last_synced_at INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        );
CREATE INDEX IF NOT EXISTS idx_user_profiles_auth0_id
         ON user_profiles(auth0_id);
CREATE INDEX IF NOT EXISTS idx_user_profiles_email
         ON user_profiles(email);
CREATE TABLE IF NOT EXISTS schema_version (
            version INTEGER PRIMARY KEY,
            applied_at INTEGER NOT NULL
        );
