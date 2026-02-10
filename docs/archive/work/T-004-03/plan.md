# T-004-03 Plan: Error & Health Alerts

## Step 1: Add derives and event variant to types.rs

**Changes:**
- Add `Serialize, Deserialize` derives to `HealthStatus` enum (line 270)
- Add `HealthStateChanged { ticket_id, old_health, new_health }` variant to `ActivityEvent` (after line 551)
- Add tests for HealthStatus serde round-trip
- Add test for HealthStateChanged event construction

**Verification:** `cargo test -p lisa-core`

## Step 2: Add UI alert types to ui.rs

**Changes:**
- Add `AlertType` enum: `Failed | Stuck`
- Add `HealthAlert` struct: ticket_id, alert_type, detail, suggested_actions
- Add `ActivityType::Warning { ticket_id, message }` variant
- Add `alerts: Vec<HealthAlert>` field to `PluginState`
- Update `PluginState::default()` to include empty alerts vec

**Verification:** `cargo test -p lisa-plugin` (existing tests still pass)

## Step 3: Implement render_attention_banner() in ui.rs

**Changes:**
- Add `render_attention_banner(state, output)` function
  - Skip if `state.alerts` is empty
  - Section header: "⚠ ATTENTION" in red bold
  - For each alert (max 5):
    - Failed: red `✗ {ticket_id} FAILED — {detail}`
    - Stuck: yellow `⚠ {ticket_id} STUCK — {detail}`
    - Below each: dim suggested actions on same or next line
- Add alert count to `render_status_line()`: append ` | ⚠ Alerts: N` in red when N > 0
- Insert `render_attention_banner()` call in `render_dashboard_lines()` between title and DAG
- Handle `ActivityType::Warning` in `render_activity_log()`

**Verification:** Add test for `render_attention_banner` with sample failed/stuck alerts. Test status line with and without alerts. `cargo test -p lisa-plugin`

## Step 4: Add health evaluation to lib.rs

**Changes:**
- Add `HealthAlert` struct (internal, not pub)
- Add `alerts: Vec<HealthAlert>` and `last_health: HashMap<TicketId, HealthStatus>` to State
- Implement `evaluate_health(&mut self)`:
  - Compute `now` and `threshold = Duration::from_secs(config.stuck_threshold_secs)`
  - For each thread in `self.threads`:
    - Compute `health = thread.health(now, threshold)`
    - Look up `last_health` for this ticket
    - If health changed (or new):
      - Log `ActivityEvent::HealthStateChanged`
      - If Stuck: create/update alert with time_in_phase
      - If Failed: create/update alert with exit code (if available)
    - If healthy and alert exists: remove alert
    - Update `last_health`
  - Clean up `last_health` entries for tickets no longer in threads map

**Verification:** Unit test: create thread, advance time past threshold, call evaluate_health, verify alert created and event logged. `cargo test -p lisa-plugin`

## Step 5: Fix detect_stale_threads() threshold and wire up poll_tick

**Changes:**
- In `detect_stale_threads()`: change `30 * 60` to `self.config.stuck_threshold_secs * 2`
- In `poll_tick()`: add `self.evaluate_health()` before `self.detect_stale_threads()`
- When `detect_stale_threads()` removes a thread, don't remove the alert (it persists in `self.alerts`)
- Clean up alerts for threads that are no longer relevant (e.g., ticket moved to Done)

**Verification:** Existing stale thread test still passes (adjust expected threshold). New test: thread at 1x threshold triggers alert but not removal; thread at 2x threshold triggers removal. `cargo test -p lisa-plugin`

## Step 6: Wire alerts to UI in to_ui_state()

**Changes:**
- Convert `self.alerts` → `Vec<ui::HealthAlert>` in `to_ui_state()`
- Map internal HealthAlert to UI HealthAlert with:
  - Failed: suggested_actions = ["Session will be retried automatically", "Check agent pane for errors"]
  - Stuck: suggested_actions = ["Check agent pane for progress", "Press [d] to mark done if complete"]
- Map `ActivityEvent::HealthStateChanged` to UI entry in `activity_event_to_ui_entry()`

**Verification:** Test that to_ui_state() includes alerts. Test activity event mapping. `cargo test -p lisa-plugin`

## Step 7: Integration test and cleanup

**Changes:**
- Full flow test: create state with stuck+failed threads, run evaluate_health, convert to UI, render dashboard, verify banner appears in output
- Remove dead code if any
- Run `cargo test --workspace` and `cargo check -p lisa-plugin --target wasm32-wasip1`

**Verification:** All tests pass, WASM check passes, no new warnings.

## Testing Strategy

| What | Type | Location |
|---|---|---|
| HealthStatus serde | Unit | types.rs tests |
| HealthStateChanged event | Unit | types.rs tests |
| render_attention_banner empty | Unit | ui.rs tests |
| render_attention_banner with alerts | Unit | ui.rs tests |
| render_status_line with alerts | Unit | ui.rs tests |
| evaluate_health stuck transition | Unit | lib.rs tests |
| evaluate_health failed transition | Unit | lib.rs tests |
| evaluate_health healthy→stuck→healthy | Unit | lib.rs tests |
| detect_stale_threads uses config threshold | Unit | lib.rs tests |
| to_ui_state includes alerts | Unit | lib.rs tests |
| activity_event_to_ui_entry for HealthStateChanged | Unit | lib.rs tests |
| Full dashboard render with alerts | Integration | ui.rs tests |
