# T-009-01 Plan: Bug Sweep and Dead Code Cleanup

## Step 1: Delete scheduler.rs and remove module declaration

1. Delete `crates/lisa-plugin/src/scheduler.rs`
2. Edit `crates/lisa-plugin/src/lib.rs`: remove line `mod scheduler;`

**Verify:** `cargo check -p lisa-plugin --target wasm32-wasip1` compiles
(7 scheduler warnings gone).

## Step 2: Fix build_claude_command in lib.rs

Edit `crates/lisa-plugin/src/lib.rs` function `build_claude_command` (lines 39-44):

Change the format string from:
```rust
"LISA_TICKET_ID={} claude --dangerously-skip-permissions \"{}\"",
```
to:
```rust
"LISA_TICKET_ID={} claude --dangerously-skip-permissions -p \"{}\"",
```

Update `test_build_claude_command` assertion to expect `-p` in the command string.

**Verify:** `cargo test --workspace` — the 3 build_claude_command tests pass.

## Step 3: Add clarifying comment at phase sync block

In `poll_tick()` at the phase sync block (line ~817), add a brief comment:
```rust
// Defensive reconciliation: catch phase changes from external edits or
// missed transitions. Normally a no-op because check_artifact_advances()
// and check_idle_signals() already update thread.current_phase.
```

**Verify:** No behavioral change, just documentation.

## Step 4: Clean up ui.rs dead code

In `crates/lisa-plugin/src/ui.rs`:

1. Remove `BG_RED` from `colors` module (line 31)
2. Remove `blocks` field from `TicketNode` struct (line 135)
3. Remove `has_session` field from `SlotInfo` struct (line 182)
4. Remove `ThreadParked` variant from `ActivityType` enum (line 193)
5. Remove `selected_ticket` field from `PluginState` struct (line 225)
6. Remove `status_indicator()` function (lines 272-280)
7. Remove `ThreadParked` match arm from `render_activity_log()` (around line 851)

## Step 5: Update ui.rs test constructors

Update all test struct literals that reference removed fields:
- Remove `blocks: vec![...]` from every `TicketNode` literal
- Remove `has_session: ...` from every `SlotInfo` literal
- Remove `selected_ticket: ...` from `PluginState` literals

**Verify:** `cargo test -p lisa-plugin` — all remaining tests pass.

## Step 6: Final verification

1. `cargo check -p lisa-plugin --target wasm32-wasip1` — zero warnings (excluding upstream deps)
2. `cargo test --workspace` — all tests pass
3. Review acceptance criteria:
   - Spawn command does NOT use `--print` ✓ (uses `-p`)
   - `build_claude_command` test updated ✓
   - No duplicate phase-change log entries ✓ (already correct, added comment)
   - Zero plugin-crate warnings ✓
   - All workspace tests pass ✓

## Test strategy

- **Existing tests stay:** 3 `build_claude_command` tests in lib.rs (one updated)
- **Existing tests removed:** 8 tests in scheduler.rs (dead code)
- **Existing tests modified:** ui.rs test constructors lose removed fields
- **No new tests needed:** this is a cleanup/bugfix, not new functionality
