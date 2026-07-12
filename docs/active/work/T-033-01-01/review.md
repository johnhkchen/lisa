# Review: recycled-seat assignment state model

## Outcome

Implemented the scheduler assignment-state foundation required by T-033-01-01.
A recycled/reused Codex seat is now represented as `AssignedPendingAck` and reports
not owned at handoff time. Fresh Codex assignments retain their established immediate
ownership behavior, and all Claude scheduling and clear-handshake behavior remains
unchanged.

The source implementation is durable in isolated ticket commit:

- `47e64b4882924b7ccbc3cd4fe9320e707a5e563a`
- `feat: model recycled Codex seat assignments`

The commit contains exactly:

- `crates/lisa-plugin/src/lib.rs`

## What changed

### Explicit assignment vocabulary

Added private `SeatAssignmentState` with three story-level states:

- `AssignedPendingAck`;
- `Owned`;
- `Recovering`.

`Recovering` is defined now so the later bounded-wait ticket can use the same state
contract. This ticket does not enter recovery or implement an acknowledgment deadline.

### Scheduler-owned assignment storage

Added a pane-keyed `seat_assignments` map to scheduler `State`.

This separates four facts that were previously conflated:

- `AgentSlot.ticket_id`: ticket reservation and routing;
- `TransitionState`: `/clear`, `/exit`, and transport sequencing;
- `has_session`/`last_client`: resident provider bookkeeping;
- `SeatAssignmentState`: acknowledged assignment truth.

Absence from the assignment map means the physical seat is unassigned.

### Ownership contract

Added scheduler queries for exact assignment state and acknowledged ownership.
Only `SeatAssignmentState::Owned` satisfies the ownership predicate.

Therefore:

- `AssignedPendingAck` is not owned;
- `Recovering` is not owned;
- a ticket reservation alone no longer means acknowledged seat ownership.

### Schedule-time classification

`schedule_ready_tickets` now captures whether the selected seat already hosted a
resident session before reset/recycle mutation.

The initial assignment classification is:

```text
incoming provider is Codex + selected seat already has a session
    => AssignedPendingAck
otherwise
    => Owned
```

This covers both same-provider Codex reuse and cross-provider physical-seat recycling
into Codex. A fresh Codex launch remains owned. Claude remains owned for fresh, reused,
and recycled paths.

No adapter commands, prompt contents, reset strategies, phase transitions, thread
capacity accounting, pane titles, or activity clocks changed.

### Timeout threading

Existing transport timeout actions preserve assignment truth:

- clear timeout sends the prompt and returns transport state to `Idle`, but a recycled
  Codex assignment remains pending/not-owned;
- exit-grace timeout launches the incoming Codex process, but the assignment remains
  pending/not-owned;
- stop timeout affects only transport state.

This prevents existing fallback timers from acting as false acknowledgment.

### Cleanup

`release_slot_for_ticket` removes assignment state at the same lifecycle boundary that
clears the ticket reservation. It still retains the resident session/provider and
cooldown exactly as before.

The missing-ticket `WaitingForExit` abandonment path also clears assignment state when
it restores an empty shell. This prevents stale ownership after a pending ticket vanishes.

## Files created

- `docs/active/work/T-033-01-01/research.md`
- `docs/active/work/T-033-01-01/design.md`
- `docs/active/work/T-033-01-01/structure.md`
- `docs/active/work/T-033-01-01/plan.md`
- `docs/active/work/T-033-01-01/progress.md`
- `docs/active/work/T-033-01-01/review.md`

## Files modified

- `crates/lisa-plugin/src/lib.rs`

## Files deleted

- None.

## Acceptance-criterion assessment

### Recycled Codex enters assigned-pending-ack

Met. `test_recycled_codex_assignment_is_pending_ack_and_not_owned` schedules a new
Codex ticket into a resident Codex seat and asserts `AssignedPendingAck`.

### Recycled Codex reports not owned

Met. The same regression directly asserts the scheduler ownership predicate is false
while the ticket remains reserved and the clear handshake is in progress.

### State threaded through fresh/reused bookkeeping

Met.

- Fresh Codex is explicitly `Owned`.
- Same-provider reused Codex is `AssignedPendingAck`.
- Cross-provider recycle into Codex is `AssignedPendingAck`.
- Reused Claude is explicitly `Owned`.

### State threaded through timeout handling

Met. Clear-timeout and exit-grace tests assert pending state is preserved and ownership
remains false after transport fallback actions.

### Claude behavior unchanged

Met. The Claude control asserts a resident Claude session still enters
`WaitingForClear`, retains its existing immediate owned contract, and passes the full
preexisting plugin/workspace suite.

## Test coverage

Focused verification:

- `cargo test -p lisa-plugin recycled_codex`: passed.
- `cargo test -p lisa-plugin reused_claude_assignment`: passed.
- `cargo test -p lisa-plugin transition_timeouts`: 5 passed.
- `cargo test -p lisa-plugin pane_title_release_reflects`: passed.

Package verification:

- `cargo test -p lisa-plugin --lib`: 251 passed, 0 failed.

Workspace verification:

- `cargo test --workspace`: passed.
- 268 `lisa-cli` unit tests passed.
- 147 `lisa-core` unit tests passed.
- 251 `lisa-plugin` unit tests passed.
- All doc tests passed.

Static verification:

- `cargo fmt --all`: applied successfully.
- `cargo clippy -p lisa-plugin --all-targets -- -D warnings`: passed.
- Ticket-owned source and artifact whitespace checks passed.
- The primary acceptance regression passed again after commit reconciliation.

## Coverage strengths

- The primary assertion drives the real scheduler selection and scheduling path.
- Controls cover both provider behavior and initial-versus-reused assignment.
- Cross-provider recycle verifies residency is captured before `has_session` mutation.
- Timeout tests prove transport completion is not treated as acknowledgment.
- Release coverage exercises both pending and owned assignment cleanup.
- The full scheduler suite covers surrounding failure, stale, completion, and audit paths.
- Strict Clippy catches dead or malformed state plumbing under all plugin targets.

## Coverage gaps intentionally left to dependent tickets

- No Codex lifecycle event is classified as acknowledgment yet (`T-033-01-02`).
- No pending-to-owned promotion exists yet (`T-033-01-03`).
- No acknowledgment deadline or pending-to-recovering transition exists yet
  (`T-033-01-04`).
- No fresh-session recovery fallback or terminal recovery failure exists yet
  (`T-033-01-04`).
- Assignment state is not projected to `ui.rs` yet (`S-033-02`).
- No live consecutive-reuse proof is part of this ticket (`S-033-03`).

These are explicit DAG boundaries, not missing work from T-033-01-01.

## Open concerns and limitations

### Pending state does not yet change

Until dependent tickets land, a recycled Codex assignment remains pending indefinitely
from the assignment-state model’s perspective. Existing session/stale timeouts still
operate, but they are not the story’s future acknowledgment timeout. This is expected
for the first ticket in the linear story chain.

### Pane-keyed map and slot data are separate

The narrow implementation stores assignment state separately from `AgentSlot` to avoid
broad fixture and scheduler churn. Normal release and explicit abandonment clear it.
Future code that invents a new slot teardown path must use the same cleanup boundary or
remove the corresponding map entry.

### Fresh Codex remains immediately owned

The classification deliberately applies acknowledgment gating only when an existing
seat/session is reassigned. This matches the recycled-seat ticket scope. If future live
evidence shows initial launches also require positive acknowledgment, that should be a
separate contract decision rather than silently broadening this ticket.

### Cross-provider recycle is conservative

A pane switching from Claude to Codex receives a fresh Codex process after `/exit`, but
the physical seat was already resident and reassigned, so it starts pending. This is
safer than declaring ownership at reservation time and provides one consistent Codex
reassignment rule.

## Critical issues requiring human attention

None found in this ticket’s implementation or verification.

The dependent acknowledgment and bounded-recovery tickets remain critical for the
complete story outcome; this state model intentionally does not claim to resolve the
live stall by itself.

## Repository and commit ownership review

- The source commit contains exactly `crates/lisa-plugin/src/lib.rs`.
- The source was committed through the current checkout’s `commit-ticket` transaction.
- The globally installed Lisa binary lacked the subcommand, so the repository CLI was
  invoked through `cargo run -p lisa-cli -- commit-ticket`.
- The source path is clean after the transaction.
- The source path is not staged in the ordinary Git index.
- Unrelated modified and untracked paths were preserved and not included.
- Ticket phase/status frontmatter was not manually edited.
- Lisa retains responsibility for phase transitions and final completion publication.

## Final assessment

T-033-01-01 is complete. The scheduler now distinguishes a reserved recycled Codex
assignment from acknowledged ownership, carries that truth through fresh/reuse/recycle
and existing timeout bookkeeping, cleans it up with slot release, and preserves Claude’s
existing behavior. The contract is ready for the detector, ack promotion, recovery,
and UI tickets that depend on it.
