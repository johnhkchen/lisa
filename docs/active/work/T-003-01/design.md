# T-003-01 Design: validate-claude-spawn

## Problem

The Claude spawn path has three issues:

1. `build_claude_command()` in scheduler.rs references a wrong RDSPI path
2. `pane_to_ticket` is never populated, so pane exits are silently lost
3. No test validates command construction

## Approach Options

### Option A: Fix Both Implementations (scheduler.rs + lib.rs)

Fix the RDSPI path in scheduler.rs, fix pane_to_ticket population in lib.rs, and add tests for both.

**Pros**: Both code paths are correct if either is ever used.
**Cons**: The scheduler.rs `Scheduler` struct is entirely dead code from the plugin's perspective. Fixing it validates something that isn't wired up.

### Option B: Fix lib.rs Only, Defer Scheduler Consolidation

Fix the pane_to_ticket population and CommandPaneExited handling in lib.rs (the active path). Fix the RDSPI path in scheduler.rs as a drive-by. Add a test for command construction in the lib.rs path. Leave the Scheduler struct dead code question for a future ticket.

**Pros**: Focuses on what actually runs. Fixes the critical pane tracking bug.
**Cons**: Scheduler struct remains diverged.

### Option C: Consolidate Into Scheduler, Wire lib.rs to Use It

Move the spawning logic out of lib.rs into Scheduler, update Scheduler to match the lib.rs behavior (tiled panes, correct paths, no --print), and wire lib.rs to delegate to Scheduler. Fix pane tracking in the process.

**Pros**: Single source of truth for spawning.
**Cons**: Scope creep beyond the ticket's ACs. Consolidation is a separate concern.

## Decision: Option B

**Rationale**: The ticket's ACs are about validating correct paths, correct context passing, and adding a test. Option B addresses all three directly. The Scheduler consolidation is out of scope — it's a design question (should lib.rs delegate to Scheduler?) that belongs in a separate ticket.

## Design Details

### Fix 1: RDSPI Path in scheduler.rs (AC #1)

Change `docs/rdspi-workflow.md` to `docs/knowledge/rdspi-workflow.md` in `build_claude_command()` at line 422.

### Fix 2: Use CommandPaneExited Context (AC #2)

In `handle_pane_exited()`, extract `ticket_id` from the context BTreeMap instead of looking up `pane_to_ticket`. This is the most direct fix: the data is already there, just being discarded.

Change the `Event::CommandPaneExited` match arm to pass context to `handle_pane_exited`. Update `handle_pane_exited` to accept the context and use it for ticket lookup.

The `pane_to_ticket` field can be kept for now (other event handlers may use it later) or removed — either way, the critical path is fixed by using the context.

### Fix 3: Test Command Construction (AC #3)

Add a test that:
1. Creates a `SchedulerConfig` with known paths
2. Calls `build_claude_command()` (need to make it `pub(crate)` or test via `spawn_claude_session` args)
3. Asserts the args contain the correct ticket path, `--dangerously-skip-permissions`, and the correct RDSPI path in the prompt

Since `build_claude_command` is private, either:
- (a) Make it `pub(crate)` so the test module can call it directly
- (b) Add a method that returns the command args without spawning

Option (a) is simplest and appropriate for a test-only visibility change.

Also add a test in lib.rs that validates the inline command construction logic. Since `schedule_ready_tickets` calls zellij APIs that can't run in tests, extract the command-building portion into a testable helper.

## Scope Boundaries

**In scope**:
- Fix RDSPI path in scheduler.rs
- Fix pane exit handling via context in lib.rs
- Add command construction test(s)
- Make build_claude_command pub(crate) for testability

**Out of scope**:
- Scheduler/lib.rs consolidation (future ticket)
- PaneUpdate handler for pane_id tracking (nice-to-have, not required for ACs)
- Removing pane_to_ticket field
