# T-009-01 Research: Bug Sweep and Dead Code Cleanup

## Problem 1: Non-interactive Claude sessions

### Current behavior

There are **two** `build_claude_command` functions:

1. **`lib.rs:39-44` (active)** — Used by `schedule_ready_tickets()` at lib.rs:335/340. Builds:
   ```
   LISA_TICKET_ID={id} claude --dangerously-skip-permissions "{prompt}"
   ```
   The prompt is passed as a positional argument. In Claude Code CLI, a positional argument
   triggers **non-interactive (print) mode** — the session processes the prompt and exits.

2. **`scheduler.rs:411-428` (dead code)** — Part of the unused `Scheduler` struct. Uses explicit
   `--print` flag. Same effect: non-interactive mode.

### Impact

Sessions exit after processing the prompt. They never reach "Awaiting User Input", so:
- The `idle_prompt` notification hook (S-008) never fires
- `.lisa/signals/{ticket_id}.idle` files are never written
- `check_idle_signals()` in poll_tick has nothing to process
- Implement phase can only complete if the agent manually sets `phase: done` in frontmatter

### Usage sites

- `lib.rs:335` — session reuse path (send command after `/exit`)
- `lib.rs:340` — fresh pane path (send command to shell)
- Both paths use `send_line_to_pane()` which types the command + Enter

### How to fix

Change `lib.rs:39-44` to use `-p` (interactive mode with initial prompt):
```
LISA_TICKET_ID={id} claude --dangerously-skip-permissions -p "{prompt}"
```

The scheduler.rs version is dead code and will be removed in Problem 3.

### Tests affected

- `lib.rs:1779` `test_build_claude_command` — asserts `starts_with("LISA_TICKET_ID=T-042-01 claude --dangerously-skip-permissions")`
- `lib.rs:1790` `test_build_claude_command_includes_env_var`
- `lib.rs:1802` `test_build_claude_command_includes_rdspi_reference`
- `scheduler.rs:758` `test_build_claude_command` — dead code, will be removed

---

## Problem 2: Double state transitions in poll_tick

### Current flow (lib.rs:774-827)

```
poll_tick():
  1. check_artifact_advances()   ← advances phase, updates thread.current_phase
  2. check_idle_signals()        ← advances phase, updates thread.current_phase
  3. evaluate_health()
  4. detect_stale_threads()
  5. rebuild_dag()               ← re-reads tickets from disk (sees updated frontmatter)
  6. done_tickets check          ← marks Done tickets complete
  7. phase sync block (817-827)  ← catches thread.current_phase != ticket.phase
```

### The double-transition mechanism

1. `check_artifact_advances()` (line 422):
   - Detects artifact, computes `next_phase`
   - Writes `next_phase` to ticket YAML on disk (line 466)
   - Logs PhaseCompleted + TicketPhaseChanged events (474-482)
   - Updates `thread.current_phase = next_phase` (486)
   - Updates `thread.last_phase_change` (487)

2. `rebuild_dag()` (line 787):
   - Calls `Dag::from_tickets()` which re-reads all ticket files from disk
   - The ticket now has the new phase (written in step 1)

3. Phase sync block (lines 817-827):
   ```rust
   for (tid, thread) in &mut self.threads {
       if thread.status == Running {
           if let Some(ticket) = self.dag.get_ticket(tid) {
               if thread.current_phase != ticket.phase {
                   thread.current_phase = ticket.phase;
                   thread.last_phase_change = SystemTime::now();
               }
           }
       }
   }
   ```
   Since step 1 already updated `thread.current_phase`, the condition
   `thread.current_phase != ticket.phase` is **false** — so the sync block
   does NOT fire again. The phases already match.

### Reassessment

After reading the code carefully: the double-transition described in the ticket
**does not actually occur** under normal conditions. The sync block at line 817-827
checks `thread.current_phase != ticket.phase`, and since `check_artifact_advances()`
already set `thread.current_phase` to the new phase, the guard prevents the
duplicate update.

The sync block WOULD catch discrepancies if:
- An external tool edits the ticket YAML (not through check_artifact_advances)
- A phase change happens between poll cycles that wasn't caught by check_artifact_advances

This is actually **correct defensive behavior**, not a bug. The sync block is a
fallback reconciliation mechanism. No duplicate events are logged because the
events are only logged in `check_artifact_advances()` and `check_idle_signals()`.

### Recommendation

The sync block is harmless and provides defensive reconciliation. It could be
made more explicit with a comment, but no code change is needed for correctness.
The `last_phase_change` reset is the only side effect — it would reset the
stuck-detection timer. But since the phases already match, the `!=` guard
prevents this from happening in the normal case.

**No fix needed** — the ticket's concern is based on a scenario that the code
already guards against.

---

## Problem 3: Dead code warnings

### Inventory (13 warnings from `cargo check -p lisa-plugin --target wasm32-wasip1`)

#### scheduler.rs — Entire `Scheduler` subsystem is unused (7 warnings)

| Warning | Item | Analysis |
|---------|------|----------|
| 1 | `CommitLock` struct never constructed | Part of old scheduler design. Commit locking is handled differently in lib.rs |
| 2 | `CommitLock` associated items never used | Same — `new`, `acquire`, `try_acquire`, `release`, `is_held`, `path` |
| 3 | `SchedulerConfig` never constructed | Replaced by `PluginConfig` in types.rs / lib.rs State |
| 4 | `Scheduler` never constructed | The scheduling logic moved into `State` methods in lib.rs |
| 5 | `Scheduler` associated items never used | All methods: `new`, `spawn_thread`, `schedule`, etc. |
| 6 | `SpawnResult` never constructed | Only used by `Scheduler::spawn_thread` |
| 7 | `ClaudeCommand` never constructed | Only used by `Scheduler::build_claude_command` |

**Verdict: Remove entirely.** The entire scheduler.rs file contains dead code.
All scheduling logic has been moved into lib.rs `State` methods. The `CommitLock`
concept isn't used anywhere. The `build_claude_command` in scheduler.rs is the
old version with `--print` that was superseded by the lib.rs version.

The scheduler.rs `mod tests` block has 8 tests that test the dead code. These
should also be removed.

#### ui.rs — UI-only dead code (6 warnings)

| Warning | Item | Location | Analysis |
|---------|------|----------|----------|
| 8 | `BG_RED` constant never used | ui.rs:31 | Color constant defined but never referenced. Remove. |
| 9 | `TicketNode::blocks` field never read | ui.rs:135 | Set in sample_state() tests but never read in rendering. The DAG renderer uses `depends_on`, not `blocks`. Remove field. |
| 10 | `SlotInfo::has_session` field never read | ui.rs:182 | Set in tests but never used in `render_slots()`. The slot renderer only checks `ticket_id.is_some()`. Remove field. |
| 11 | `ActivityType::ThreadParked` variant never constructed | ui.rs:193 | Defined but never created. `render_activity_log()` handles it in the match, but no code path creates it. Remove variant + match arm. |
| 12 | `PluginState::selected_ticket` field never read | ui.rs:225 | Set to `None` in Default but never used anywhere. Remove field. |
| 13 | `status_indicator()` function never used | ui.rs:272 | Helper function that was superseded by inline status rendering in `render_dag()`. Remove. |

**Verdict: Remove all 6.** None of these are part of planned features. The
`blocks` field on `TicketNode` is for display only and is never used by any
renderer. `has_session` on `SlotInfo` is never checked. `ThreadParked` has no
construction site. `selected_ticket` has no consumer. `status_indicator()` is
unused. `BG_RED` is unused.

### Test impact

Removing scheduler.rs eliminates 8 tests:
- `test_scheduler_creation`
- `test_thread_lifecycle`
- `test_commit_lock_path`
- `test_ticket_work_dir`
- `test_spawn_thread_capacity`
- `test_handle_pane_exit`
- `test_phase_artifact_check`
- `test_build_claude_command`

Removing ui.rs fields affects test constructors (`sample_state()` and others)
that set `blocks`, `has_session`, and `selected_ticket`. These fields just need
to be removed from the struct literals.

---

## Files involved

| File | Changes needed |
|------|---------------|
| `crates/lisa-plugin/src/lib.rs` | Fix `build_claude_command()` to use `-p`; update 3 tests |
| `crates/lisa-plugin/src/scheduler.rs` | Remove entirely (or gut to empty module) |
| `crates/lisa-plugin/src/ui.rs` | Remove 6 dead items; update test struct literals |

## Dependencies

None — this ticket has no `depends_on`.
