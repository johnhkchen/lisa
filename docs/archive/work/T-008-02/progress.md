# T-008-02 Progress: Idle-Aware Phase Advancement

## Completed

### Step 1: AlertType::IdleWithoutArtifact (ui.rs)
- Added `IdleWithoutArtifact` variant to `AlertType` enum
- Added match arm in `render_attention_banner()`: displays as "⏸ IDLE" in yellow

### Step 2: State fields and signal_dir (lib.rs)
- Added `signal_dir: PathBuf` and `idle_alerts: Vec<(TicketId, String)>` fields to `State`
- Initialized `signal_dir` as `host.join(".lisa/signals")` in `load()`

### Step 3: check_idle_signals() method (lib.rs)
- Implemented full idle signal scanning and phase advancement logic
- Scans `.lisa/signals/` for `*.idle` files using `std::fs::read_dir()`
- Signal files always deleted after processing (prevents re-trigger)
- **Implement phase**: idle signal alone advances to Review, parks thread
- **Research/Design/Structure/Plan**: requires artifact + idle signal to advance
- **Idle-without-artifact**: generates alert and Warning activity event
- **Non-running/missing threads**: signal cleaned up silently

### Step 4: Wiring (lib.rs)
- Added `self.check_idle_signals()` call in `poll_tick()` after `check_artifact_advances()`
- In `to_ui_state()`, idle alerts are appended to the health alerts vector

### Step 5: Verification
- `cargo test --workspace` — 256 tests pass (77 cli + 77 core + 102 plugin)
- `cargo check -p lisa-plugin --target wasm32-wasip1` — compiles successfully
- All warnings are pre-existing (dead code in scheduler.rs, ui.rs)

## Test Summary
- New tests added: 7
  - `test_idle_signal_implement_advances_to_review`
  - `test_idle_signal_research_with_artifact_advances`
  - `test_idle_signal_research_without_artifact_alerts`
  - `test_idle_signal_no_thread_ignored`
  - `test_idle_signal_nonrunning_thread_ignored`
  - `test_idle_signal_missing_dir_no_panic`
  - `test_to_ui_state_includes_idle_alerts`

## Acceptance Criteria Status
- [x] Implement -> Review advances automatically when idle signal detected
- [x] Earlier phases advance when idle signal + artifact both present
- [x] Alert surfaces when idle signal arrives but artifact is missing
- [x] Signal files are cleaned up after processing
- [x] Existing artifact-based detection still works independently
- [x] All existing tests continue to pass
