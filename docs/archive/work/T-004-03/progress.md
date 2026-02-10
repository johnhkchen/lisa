# T-004-03 Progress: Error & Health Alerts

## Step 1: Add derives and event variant to types.rs
- [x] Add Serialize, Deserialize to HealthStatus
- [x] Add HealthStateChanged variant to ActivityEvent
- [x] Add tests (test_health_status_serde, test_health_state_changed_event)

## Step 2: Add UI alert types to ui.rs
- [x] Add AlertType (Failed, Stuck), HealthAlert struct, ActivityType::Warning
- [x] Add alerts field to PluginState

## Step 3: Implement render_attention_banner() in ui.rs
- [x] Extended existing render_attention_banner to show health alerts
- [x] Status line alert count
- [x] Suggested actions rendering
- [x] Tests (6 banner tests, 2 status line tests)

## Step 4: Add health evaluation to lib.rs
- [x] last_health field on State
- [x] evaluate_health() method with transition detection
- [x] Tests (4 evaluate_health tests)

## Step 5: Fix detect_stale_threads() threshold and wire poll_tick
- [x] Use config.stuck_threshold_secs * 2 as hard timeout
- [x] Add evaluate_health() to poll_tick before detect_stale_threads
- [x] Tests (2 config threshold tests)

## Step 6: Wire alerts to UI in to_ui_state()
- [x] Convert stuck/failed threads to UI HealthAlerts in to_ui_state()
- [x] Map HealthStateChanged events to Warning/Error in activity log
- [x] Tests (3 to_ui_state tests, 3 event mapping tests)

## Step 7: Integration test and cleanup
- [x] Full flow tests (banner with mixed alerts and reviews)
- [x] cargo test --workspace — 168 tests passing (59 core + 60 plugin + 49 CLI)
- [x] cargo check -p lisa-plugin --target wasm32-wasip1 — passes

## Summary of Changes

### types.rs
- `HealthStatus` now derives Serialize, Deserialize with serde rename_all = "lowercase"
- New `ActivityEvent::HealthStateChanged { ticket_id, old_health, new_health }` variant

### ui.rs
- New `AlertType` enum (Failed, Stuck)
- New `HealthAlert` struct with ticket_id, alert_type, detail, suggested_actions
- New `ActivityType::Warning` variant
- `PluginState` gained `alerts: Vec<HealthAlert>` field
- `render_attention_banner()` extended to show health alerts with labels (✗ FAILED, ! STUCK) and suggested actions
- `render_status_line()` shows alert count in red when > 0
- Added `BG_RED` color constant

### lib.rs
- `State` gained `last_health: HashMap<TicketId, HealthStatus>` field
- New `evaluate_health()` method: computes health per thread, detects transitions, logs HealthStateChanged events
- `detect_stale_threads()` now uses `config.stuck_threshold_secs * 2` instead of hardcoded 30min
- `poll_tick()` calls `evaluate_health()` before `detect_stale_threads()`
- `to_ui_state()` computes UI HealthAlerts from stuck/failed threads
- `activity_event_to_ui_entry()` maps HealthStateChanged to Warning (Stuck) or Error (Failed)
