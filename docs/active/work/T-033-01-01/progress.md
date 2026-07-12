# Progress: recycled-seat assignment state model

## Current state

Implementation is complete, verified, and committed through Lisa's isolated ticket
transaction. Review is the only remaining phase artifact.

## Completed: Research

- Read `AGENTS.md` and its required source of truth, `CLAUDE.md`.
- Read `docs/knowledge/rdspi-workflow.md`.
- Read `docs/active/tickets/T-033-01-01.md`.
- Read the parent story and all dependent story tickets.
- Mapped `AgentSlot`, `TransitionState`, `Thread`, and scheduler `State`.
- Traced fresh, same-provider reuse, and cross-provider recycle scheduling paths.
- Traced clear, stop, and exit timeout fallbacks.
- Traced normal release and missing-ticket recycle abandonment.
- Identified `ticket_id` as reservation/routing state rather than acknowledged ownership.
- Identified `TransitionState` as transport state rather than assignment truth.
- Recorded findings in `research.md`.

## Completed: Design

- Evaluated retaining implicit ownership through `ticket_id`.
- Evaluated delaying ticket/thread binding until acknowledgment.
- Evaluated merging assignment states into `TransitionState`.
- Evaluated adding a required field to every `AgentSlot`.
- Selected a pane-keyed assignment map in scheduler `State`.
- Defined absence as unassigned.
- Defined `Owned` as the only state satisfying the ownership predicate.
- Defined recycled/reused Codex as pending acknowledgment.
- Preserved fresh Codex and all Claude assignments as immediately owned.
- Kept assignment truth orthogonal to reset/exit transport mechanics.
- Recorded the decision and tradeoffs in `design.md`.

## Completed: Structure

- Limited source ownership to `crates/lisa-plugin/src/lib.rs`.
- Defined the enum placement beside other slot lifecycle vocabulary.
- Defined `State` storage and pane-keyed invariants.
- Defined query helpers for state and ownership.
- Defined schedule-time classification ordering.
- Defined release and abandonment cleanup.
- Defined timeout preservation expectations.
- Defined focused unit-test placement and scenarios.
- Recorded the blueprint in `structure.md`.

## Completed: Plan

- Sequenced enum, storage, scheduling, cleanup, and tests.
- Defined fresh Codex and Claude compatibility controls.
- Defined clear-timeout and exit-timeout preservation checks.
- Defined focused, package, workspace, lint, and diff verification.
- Defined the exact isolated commit command and include path.
- Recorded the execution plan in `plan.md`.

## Completed: assignment vocabulary

Added private `SeatAssignmentState` with:

- `AssignedPendingAck`;
- `Owned`;
- `Recovering`.

`Recovering` is intentionally representable but not entered by production logic in
this ticket. `T-033-01-04` owns the bounded acknowledgment timeout and recovery action.
A narrow dead-code allowance documents that planned boundary.

## Completed: scheduler-owned assignment map

Added `State::seat_assignments`, keyed by physical terminal pane ID.

The map is intentionally separate from:

- `AgentSlot.ticket_id`, which reserves and routes the ticket;
- `AgentSlot.transition_state`, which sequences reset/exit commands;
- `AgentSlot.has_session`, which tracks resident TUI presence;
- `ThreadStatus`, which tracks run lifecycle.

No entry represents an unassigned seat.

## Completed: authoritative queries

Added:

- `seat_assignment(pane_id)` for the exact named state;
- `seat_is_owned(pane_id)` for the ownership contract.

The ownership query returns true only for `SeatAssignmentState::Owned`.
Pending and recovering assignments therefore report not owned even while their
ticket reservation and running thread remain present.

## Completed: schedule-time classification

`schedule_ready_tickets` now captures whether the selected physical seat already had
a resident session before any recycle mutation.

Classification is:

- incoming Codex plus resident session -> `AssignedPendingAck`;
- otherwise -> `Owned`.

This produces:

- fresh Codex -> owned;
- same-provider Codex reuse -> pending/not-owned;
- cross-provider recycle into Codex -> pending/not-owned;
- fresh Claude -> owned;
- reused/recycled Claude -> owned.

Existing adapter selection, commands, pane titles, activity clocks, capacity counting,
thread creation, and ticket phase behavior are unchanged.

## Completed: lifecycle cleanup

- `release_slot_for_ticket` now removes assignment state with the ticket reservation.
- It continues retaining the resident session, provider affinity, cooldown, and idle name.
- The missing-ticket `WaitingForExit` abandonment path removes any stale assignment.
- Normal clear, stop, and exit transition paths deliberately preserve assignment state.

## Completed: test coverage

Added or extended scheduler tests proving:

- fresh Codex is explicitly owned;
- recycled same-provider Codex is `AssignedPendingAck`;
- recycled Codex reports not owned;
- reused Claude remains owned;
- reused Claude still enters `WaitingForClear`;
- cross-provider recycle into Codex is pending/not-owned;
- clear timeout preserves pending/not-owned state;
- exit-grace launch preserves pending/not-owned state;
- release removes both pending and owned map entries.

The primary acceptance regression is:

`test_recycled_codex_assignment_is_pending_ack_and_not_owned`

## Verification results

Formatting:

- `cargo fmt --all`: completed.
- `cargo fmt --all -- --check`: clean before implementation formatting identified only
  the newly edited source; formatting was then applied.

Focused tests:

- `cargo test -p lisa-plugin recycled_codex`: 1 passed.
- `cargo test -p lisa-plugin reused_claude_assignment`: 1 passed.
- `cargo test -p lisa-plugin transition_timeouts`: 5 passed.
- `cargo test -p lisa-plugin pane_title_release_reflects`: 1 passed.

Package tests:

- `cargo test -p lisa-plugin --lib`: 251 passed, 0 failed.

Workspace tests:

- `cargo test --workspace`: passed.
- The run covered 268 CLI tests, 147 core tests, and 251 plugin tests.
- Doc tests passed.

Lint:

- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.

## Deviations from plan

- The implementation extended several existing pane lifecycle and timeout tests rather
  than duplicating their setup in wholly new test functions.
- The existing same-provider pane-title test was renamed into the explicit acceptance
  regression because it already exercised the exact scheduling path.
- The globally installed `/opt/homebrew/bin/lisa` predates the `commit-ticket`
  subcommand. The same current-checkout CLI was invoked as
  `cargo run -p lisa-cli -- commit-ticket ...`, preserving the required isolated
  transaction and exact include semantics.
- No behavioral or scope deviation was required.

## Completed: isolated source commit

- Commit: `47e64b4882924b7ccbc3cd4fe9320e707a5e563a`.
- Message: `feat: model recycled Codex seat assignments`.
- Included source path: `crates/lisa-plugin/src/lib.rs`.
- The commit contains exactly that one source path.
- Ticket-scoped diff whitespace checks passed before commit.
- The source path is clean after commit reconciliation.
- The source path has no entry in the ordinary Git index.
- The primary recycled-Codex regression passed again after the commit.
- Unrelated modified and untracked repository paths remain untouched.

## Remaining step

- Write `review.md`, then stop without editing ticket phase/status or publishing
  completion.
