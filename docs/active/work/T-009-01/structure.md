# T-009-01 Structure: Bug Sweep and Dead Code Cleanup

## File changes

### 1. DELETE `crates/lisa-plugin/src/scheduler.rs`

Remove the entire file. Contents (all dead code):
- `CommitLock` struct + impl (unix and wasm32 variants)
- `SchedulerConfig` struct + Default impl
- `Scheduler` struct + impl (all methods)
- `SpawnResult` struct
- `ClaudeCommand` struct
- 8 tests in `mod tests`

### 2. MODIFY `crates/lisa-plugin/src/lib.rs`

**Remove module declaration:**
- Line 7: delete `mod scheduler;`

**Fix `build_claude_command` (lines 39-44):**
- Change: pass prompt via `-p` flag instead of positional argument
- Before: `claude --dangerously-skip-permissions "{prompt}"`
- After: `claude --dangerously-skip-permissions -p "{prompt}"`

**Add clarifying comment at phase sync block (lines 817-827):**
- Document that the sync block is a defensive fallback that normally doesn't
  fire because `check_artifact_advances()` and `check_idle_signals()` already
  update `thread.current_phase` to match

**Update tests:**
- `test_build_claude_command` (line 1779): update assertion to expect `-p` in command string
- `test_build_claude_command_includes_env_var` (line 1790): no change needed (tests env var prefix)
- `test_build_claude_command_includes_rdspi_reference` (line 1802): no change needed (tests content)

### 3. MODIFY `crates/lisa-plugin/src/ui.rs`

**Remove dead items:**
- Line 31: remove `pub const BG_RED: &str = "\x1b[41m";` from `colors` module
- Line 135: remove `pub blocks: Vec<String>` from `TicketNode`
- Line 182: remove `pub has_session: bool` from `SlotInfo`
- Lines 193: remove `ThreadParked { ticket_id: String, phase: Phase }` variant from `ActivityType`
- Line 225: remove `pub selected_ticket: Option<String>` from `PluginState`
- Lines 272-280: remove `fn status_indicator()` function

**Remove match arm:**
- In `render_activity_log()` (around line 851): remove the `ActivityType::ThreadParked` match arm

**Update test struct literals** (numerous locations):
- Remove `blocks: vec![...]` from all `TicketNode` literals in tests
- Remove `has_session: ...` from all `SlotInfo` literals in tests
- Remove `selected_ticket: ...` from `PluginState` literals in tests (only in `sample_state()` and inline)

## Module boundaries

No new modules, interfaces, or public API changes. All changes are internal
to the `lisa-plugin` crate. The `lisa-core` and `lisa-cli` crates are unaffected.

## Ordering constraints

1. Remove `scheduler.rs` and `mod scheduler;` first (eliminates 7 warnings)
2. Fix `build_claude_command` in lib.rs (addresses the functional bug)
3. Clean up ui.rs dead code (eliminates remaining 6 warnings)
4. Run `cargo check -p lisa-plugin --target wasm32-wasip1` to verify zero warnings
5. Run `cargo test --workspace` to verify all tests pass
