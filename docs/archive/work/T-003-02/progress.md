# T-003-02 Progress: Artifact-Phase Advance

## Completed

### Step 1: `check_artifact_advances()` method
- Added to `State` impl in `crates/lisa-plugin/src/lib.rs`
- Scans running threads for phase artifacts in work dir
- Uses `Phase::artifact_filename()` for mapping, `Phase::next()` for advancement
- Calls `ticket::update_ticket_phase()` to update YAML frontmatter on disk
- Logs `PhaseCompleted` and `TicketPhaseChanged` events
- Parks thread when advancing to Review phase
- Error handling: logs `ActivityEvent::Error` on I/O failure, skips ticket

### Step 2: Wired into `poll_tick()`
- `self.check_artifact_advances()` added as first line of `poll_tick()`
- Runs before `rebuild_dag()` so updated ticket files are picked up in same tick

### Step 3: Unit test — research to design
- `test_check_artifact_advances_research_to_design`: creates tempdir with ticket + research.md artifact, verifies phase advances to design, thread remains running, activity log has correct events, ticket file updated on disk

### Step 4: Review-parking test
- `test_check_artifact_advances_implement_to_review_parks_thread`: creates tempdir with ticket + progress.md artifact, verifies phase advances to review, thread is parked (ThreadStatus::Parked)

### Step 5: No-artifact test
- `test_check_artifact_advances_no_artifact_no_change`: verifies no changes when work dir exists but no artifact file

## Test Results
- 45 core tests pass
- 31 plugin tests pass (28 existing + 3 new)
- WASM compilation succeeds (`cargo check -p lisa-plugin --target wasm32-wasip1`)

## Deviations
- None. Plan followed exactly.

## Files Changed
- `crates/lisa-plugin/src/lib.rs` — added `check_artifact_advances()`, wired into `poll_tick()`, 3 new tests
- `crates/lisa-plugin/Cargo.toml` — added `tempfile = "3"` dev-dependency
