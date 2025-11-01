# Command Lookup Table (Verified)

**Last Verified:** 2025-11-01

---

### Frontend Commands That Exist in Backend (MIGRATED)

| Frontend Command | Backend Location |
|-----------------|------------------|
| `accept_proposed_block` | New crate |
| `animate_window_resize` | New crate |
| `disconnect_calendar` | New crate |
| `dismiss_proposed_block` | New crate |
| `get_calendar_connection_status` | New crate |
| `get_calendar_events_for_timeline` | New crate |
| `get_calendar_sync_settings` | New crate |
| `initiate_calendar_auth` | New crate |
| `pause_tracker` | New crate |
| `record_activity_wake` | New crate |
| `record_auto_start_tracker_rule` | New crate |
| `record_idle_detection` | New crate |
| `record_invalid_payload` | New crate |
| `record_state_transition` | New crate |
| `record_timer_event_emission` | New crate |
| `record_timer_event_reception` | New crate |
| `resume_tracker` | New crate |
| `sync_calendar_events` | New crate |
| `update_calendar_sync_settings` | New crate |

### Migrated Commands Used by Frontend 

| Backend Command | Frontend Usage |
|----------------|----------------|
| `accept_proposed_block` | ✅ 1 use |
| `animate_window_resize` | ✅ 1 use |
| `disconnect_calendar` | ✅ 3 uses |
| `dismiss_proposed_block` | ✅ 1 use |
| `get_calendar_connection_status` | ✅ 4 uses |
| `get_calendar_events_for_timeline` | ✅ 5 uses |
| `get_calendar_sync_settings` | ✅ 3 uses |
| `initiate_calendar_auth` | ✅ 1 use |
| `pause_tracker` | ✅ 4 uses |
| `record_activity_wake` | ✅ 1 use |
| `record_auto_start_tracker_rule` | ✅ 1 use |
| `record_idle_detection` | ✅ 1 use |
| `record_invalid_payload` | ✅ 1 use |
| `record_state_transition` | ✅ 1 use |
| `record_timer_event_emission` | ✅ 1 use |
| `record_timer_event_reception` | ✅ 1 use |
| `resume_tracker` | ✅ 5 uses |
| `sync_calendar_events` | ✅ 2 uses |
| `update_calendar_sync_settings` | ✅ 5 uses |


### Frontend Commands That DON'T Exist in Backend 

| Frontend Command | Issue |
|-----------------|-------|
| `disconnect_google_calendar` | ⚠️ Old name - should use `disconnect_calendar` |
| `initiate_google_calendar_auth` | ⚠️ Old name - should use `initiate_calendar_auth` |
| `open_ai_entry` | ❌ NOT IMPLEMENTED (2 uses) | ❌ Deprecated 
| `save_manual_activity` | ❌ NOT IMPLEMENTED | -> Create save_time_entry
| `set_idle_enabled` | ❌ NOT IMPLEMENTED | --> ❌ Deprecated 
| `set_idle_threshold` | ❌ NOT IMPLEMENTED | --> ❌ Deprecated
| `clear_local_activities` | Legacy only | -> Create clear_database
| `clear_outbox` | Legacy only | --> Create clear_suggestions
| `delete_outbox_entry` | Legacy only | --> Create delete_suggestion
| `get_cost_summary` | Legacy only | --> ❌ Deprecated
| `update_suggestion` | Legacy only | --> Create update_suggestion
| `restore_suggestion` | Legacy only | --> Create restore_suggestion
| `dismiss_suggestion` | Legacy only | --> Create dismiss_suggestion

### Migrated Commands NOT Used by Frontend

| Backend Command | Status |
|----------------|--------|
| `build_my_day` | ❌ Not invoked |
| `get_activity` | ❌ Not invoked |
| `get_app_health` | ❌ Not invoked |
| `get_database_health` | ❌ Not invoked |
| `get_database_stats` | ❌ Not invoked |
| `get_dismissed_suggestions` | ❌ Not invoked |
| `get_idle_periods` | ❌ Not invoked |
| `get_idle_summary` | ❌ Not invoked |
| `get_outbox_status` | ❌ Not invoked |
| `get_proposed_blocks` | ❌ Not invoked |
| `get_recent_snapshots` | ❌ Not invoked |
| `get_user_profile` | ❌ Not invoked |
| `get_user_projects` | ❌ Not invoked |
| `is_feature_enabled` | ❌ Not invoked |
| `list_feature_flags` | ❌ Not invoked |
| `seed_activity_snapshots` | ❌ Not invoked |
| `toggle_feature_flag` | ❌ Not invoked |
| `update_idle_period_action` | ❌ Not invoked |
| `upsert_user_profile` | ❌ Not invoked |
| `vacuum_database` | ❌ Not invoked |
