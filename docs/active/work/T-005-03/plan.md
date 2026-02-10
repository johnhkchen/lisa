# Plan: T-005-03 fix-phase-change-detection

## Step 1: Fix `rebuild_dag()` phase change detection

**File**: `crates/lisa-plugin/src/lib.rs`, lines 164-176

**Action**: Replace the `if let Some(...)` pattern with a `match` that also handles the `None` case.

**Specific change**:
- Replace the `for ticket in dag.tickets()` loop body
- `Some(&old_phase)` arm: keep existing logic (compare, log, set changed)
- `None` arm: if `ticket.phase != Phase::Ready`, set `changed = true` (no log needed — this is seeding, not a user-visible transition)

**Verification**: Existing tests should still pass. New test in Step 4 validates.

**Commit**: "Fix rebuild_dag to detect phase changes for first-seen tickets"

## Step 2: Move done-ticket detection out of `if changed` in `poll_tick()`

**File**: `crates/lisa-plugin/src/lib.rs`, lines 491-530

**Action**: Extract the done-ticket detection + slot release block AND the thread phase sync block from inside `if changed { ... }` to run unconditionally after `rebuild_dag()`. Remove the now-empty `if changed` block.

**Specific changes**:
1. Move lines 495-517 (done_tickets detection + completion + release loop) to right after `let changed = self.rebuild_dag();`, before any `if changed` check
2. Move lines 520-529 (thread phase sync loop) to right after the done-tickets block
3. Remove the empty `if changed { }` wrapper

**Verification**: Existing tests pass. Behavior is identical for the normal case (when `changed` was true, same code runs). For the bug case (when `changed` was false), done tickets are now properly detected.

**Commit**: "Make done-ticket detection and slot release unconditional in poll_tick"

## Step 3: Add `sweep_stale_slots()` method

**File**: `crates/lisa-plugin/src/lib.rs`

**Action**: Add a new `fn sweep_stale_slots(&mut self)` method to `impl State`, after `schedule_ready_tickets()`.

**Logic**:
1. Collect `(pane_id, ticket_id)` pairs where slot has a ticket_id that is Done in the DAG
2. For each, call `release_slot_for_ticket()` and log a warning

**Integration**: Call `sweep_stale_slots()` in `poll_tick()` after the done-ticket detection block, before `schedule_ready_tickets()`.

**Verification**: New test in Step 4 validates.

**Commit**: "Add sweep_stale_slots safety net for orphaned slot assignments"

## Step 4: Add tests

**File**: `crates/lisa-plugin/src/lib.rs`, `mod tests` section

**Test 1**: `test_done_ticket_detected_on_first_poll`
- Setup: Create tempdir with one ticket at `phase: done`, build DAG, create State with empty `last_phases`, add a Running thread and an occupied AgentSlot for that ticket
- Call `rebuild_dag()`, then inline the done-ticket detection logic (since `poll_tick` calls zellij APIs we can't call it directly)
- Assert: `rebuild_dag()` returns true (ticket at done != ready, detected as changed)
- Assert: After running detection logic, thread is completed, slot is released

**Test 2**: `test_done_ticket_detected_between_polls`
- Setup: Create tempdir with ticket at `phase: done`, build DAG, create State with `last_phases` containing the ticket at `Phase::Research`, add Running thread + occupied slot
- Call `rebuild_dag()` → should return true (Research != Done)
- Run detection logic → thread completed, slot released

**Test 3**: `test_sweep_stale_slots_releases_done_ticket`
- Setup: Create tempdir with ticket at `phase: done`, build DAG, create State with an AgentSlot pointing to that ticket but NO thread
- Call `sweep_stale_slots()`
- Assert: Slot released, activity log contains "stale"

**Verification**: `cargo test --workspace`

**Commit**: "Add tests for phase-change detection and stale slot sweep"

## Verification Plan

After all steps:
1. `cargo test --workspace` — all existing + new tests pass
2. `cargo check -p lisa-plugin --target wasm32-wasip1` — WASM compilation succeeds
3. Manual review of `poll_tick()` flow to confirm:
   - Done tickets are detected every tick
   - Slots are released every tick
   - Sweep catches any remaining stale assignments
   - `schedule_ready_tickets()` runs after all cleanup
