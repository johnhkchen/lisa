# T-033-03-02 Research — consecutive reuse live proof

## Ticket boundary

This ticket supplies field-style regression evidence for the acknowledged
assignment contract implemented by `S-033-01` and preserved by the deterministic
incident regression in `T-033-03-01`.

The requested proof has four observable requirements:

- at least ten consecutive Codex assignments must reuse physical panes;
- one assignment must lose its initial acknowledgment deliberately;
- every assignment must end in exactly one named outcome:
  `ack-then-owned` or `timeout-then-fallback`;
- an equivalent Claude control must retain its existing transition behavior.

The story explicitly excludes scheduler changes. Its owned surface is a
live-style harness, committed test/harness artifacts, and a run report.

## Current scheduler contract

`crates/lisa-plugin/src/lib.rs` owns the physical-seat state machine.
`State::seat_assignments` is keyed by terminal pane ID and is separate from the
slot reservation and transport state.

The relevant states are:

```text
AssignedPendingAck { generation, ack_deadline }
Owned
Recovering { generation, ack_deadline }
RecoveryFailed
```

Only `Owned` makes `seat_is_owned` true. A ticket ID on a slot, a running
thread, a resident TUI, or an idle transport is not sufficient ownership.

For a same-provider Codex reuse, `schedule_ready_tickets` observes
`has_session = true`, allocates a nonzero assignment generation, sends `/clear`,
records `AssignedPendingAck` with no deadline, and enters `WaitingForClear`.
`handle_cleared_signal` then sends the tagged reuse prompt, returns transport to
`Idle`, and arms the finite acknowledgment deadline.

For a same-provider Claude reuse, the reset strategy is also `ClearHandshake`.
Scheduling sends `/clear`, records `WaitingForClear`, and immediately records
the assignment as `Owned`. `handle_cleared_signal` later submits the ticket
prompt and returns transport to `Idle`. Claude has no Codex generation or
provider-ack wait. The shared transport is unchanged; ownership semantics are
the provider-specific boundary.

This provider distinction is the parity boundary the ticket must protect.

## Reused-pane lifecycle

`release_slot_for_ticket` clears the slot's ticket reservation and removes its
seat-assignment entry while intentionally preserving `has_session = true` and
`last_client`. It also applies the configured wind-down cooldown.

With `wind_down_secs = 0`, the same physical pane is immediately eligible for
the next ticket using the resident provider. A harness can therefore exercise
back-to-back reuse without sleeping by completing the test thread, releasing
its ticket, and polling scheduling again.

`schedule_ready_tickets` removes a completed thread when the same ready ticket
is reconsidered. For a multi-ticket fixture, completed threads for prior
tickets remain in the map but do not count against concurrency. Ready tickets
that have no thread can occupy the released seat.

The DAG returns ready open tickets. Independent ready tickets are sufficient
for a one-seat sequential harness because the global thread limit and single
slot serialize them; no synthetic dependency transition is required.

## Acknowledgment path

Codex acceptance uses `codex_ack::tag_codex_assignment` to place ticket and
generation identity in the submitted prompt. A matching `UserPromptSubmit`
JSON payload passed to `acknowledge_codex_assignment` promotes only the active
ticket/generation pair.

Stale, duplicate, wrong-ticket, and wrong-generation payload behavior is
already covered by focused unit tests. The consecutive harness needs the
positive edge and one intentional missing edge, not another classifier matrix.

For the ordinary assignments, the harness can construct the exact current
payload and call the real promotion method. The resulting state must be
`Owned`, and ownership must be true exactly once.

## Lost-ack fallback path

`T-033-03-01` established the deterministic event-loss seam. If the matching
acceptance event is absent when the original deadline is evaluated,
`check_assignment_ack_timeouts_at` moves the seat to `Recovering`, allocates a
new generation, sends `/exit`, and sets transport to `WaitingForExit`.

After the fixed exit grace, `check_transition_timeouts` sends one fresh Codex
launch for the same ticket and arms the recovery generation's deadline. A
matching recovery-generation acknowledgment then promotes the seat to
`Owned`.

The ticket's named `timeout-then-fallback` outcome is therefore a successful
bounded recovery: original generation times out, one fresh session launches,
and that new generation is acknowledged. It is distinct from the dependency
test's terminal `RecoveryFailed` case.

A launch count from `ActivityEvent::SessionLaunch`, filtered by ticket and the
recovery generation marker, can prove there was exactly one fallback.

## Silent-stall definition

The field failure was a reservation that appeared active without positive
acceptance and without a finite recovery transition. Under the current
contract, a harness row is non-stalled only if it reaches one terminal owned
state through one of the two allowed paths.

The harness can enforce this structurally:

- ordinary row: pending -> matching ack -> owned;
- fault row: pending -> timeout -> recovering -> one fresh launch -> matching
  recovery ack -> owned;
- neither row may end pending, recovering, or recovery-failed;
- every row must retain the same ticket and pane throughout its resolution;
- exactly one outcome label is emitted per row.

This is stronger than checking a final boolean because it records the observed
path and launch cardinality.

## Existing live-style precedents

`docs/active/work/T-031-03/harness/run.sh` is the closest committed harness
shape. It runs deterministic native behavior, writes inspectable evidence,
asserts counts and ordering, and is invoked by a Cargo integration test. It
does not require a live model or Zellij.

`docs/active/work/T-029-01/` is the live-run precedent named by the story. That
work separates empirical host probes and human-readable reports from unit
coverage, records the exact client version, and avoids claiming an interactive
loop that was not actually run.

For this ticket, “live-style” can accurately mean the production scheduler
state and repeated physical-seat lifecycle are driven in native tests, with a
shell runner producing durable output. It must not claim live Codex token use
or a Zellij session when neither occurs.

## Native test boundary

The scheduler `State`, assignment enums, injected-time methods, and activity
log are private to the plugin crate. Tests in the `#[cfg(test)]` module at the
bottom of `crates/lisa-plugin/src/lib.rs` can access them directly.

An external black-box test cannot inspect the transition path without widening
production visibility or running a WASM host. A native test is therefore the
established faithful seam.

Rust test output is visible with `cargo test ... -- --nocapture`. Stable,
prefixed report rows can be captured by a shell harness and transformed into a
Markdown report without parsing ordinary Cargo diagnostics.

## Fixture needs

The existing `pane_name_schedule_state` helper creates one `T-NAME` ticket and
one pane. The consecutive proof needs multiple unique tickets and predictable
routes.

A test-local builder can create:

- ten or more independent Codex tickets;
- one resident Codex pane, or two panes if the proof deliberately alternates;
- one or more independent Claude tickets in a separate control state;
- `wind_down_secs = 0` and a short acknowledgment timeout;
- permissions and slot discovery already enabled.

Using two named pane IDs across the Codex run demonstrates reuse across panes
rather than repeatedly exercising only one hard-coded seat. Each pane can
start resident and the test can serialize assignments by allowing one ready
ticket per state or by controlling which fixture ticket is open.

The simplest faithful arrangement is a helper that creates a fresh state per
physical pane and cycles multiple unique assignments through it. However,
recreating state would not prove consecutive scheduler reuse. A single state
with multiple tickets and released resident slots preserves the consecutive
property.

## Report boundary

The acceptance criterion asks for a harness run report, not only a green test.
The report should include:

- execution date and environment;
- exact command;
- harness type and honest non-live limitations;
- one row per Codex reassignment with sequence, ticket, pane, generation,
  outcome, final state, fallback count, and silent-stall result;
- a Claude control row or rows with reset transition and final assignment;
- totals proving `>= 10`, one forced loss, exactly two allowed outcome names,
  and zero stalls;
- test suite results.

The generated report belongs under
`docs/active/work/T-033-03-02/`, beside the six RDSPI artifacts and harness.

## CI integration options already available

The plugin crate has no binary target, so an integration test cannot use
`CARGO_BIN_EXE`. The shell harness can invoke a focused unit test directly.

The proof remains in the normal `cargo test -p lisa-plugin` suite even without
the wrapper. The wrapper is responsible for report capture and independent
count validation, not for making the underlying regression executable in CI.

Adding a second Cargo integration test merely to spawn Cargo recursively would
be slow and risks Cargo target-lock contention. No such precedent exists in
the plugin crate.

## Determinism constraints

The native proof can avoid all sleeps:

- read the original deadline from assignment state;
- pass that exact deadline into the timeout evaluator;
- backdate transition start beyond exit grace;
- invoke transition evaluation synchronously;
- acknowledge the current generation immediately;
- set cooldown and wind-down to zero before releasing each ticket.

Temporary directories isolate tickets and signals. No network, token, model
installation, shell TUI, terminal rendering, or current wall-clock duration is
part of the verdict.

## Worktree and commit constraints

The repository has many unrelated modified and untracked paths. The ticket's
likely owned implementation paths are:

- `crates/lisa-plugin/src/lib.rs` for the native consecutive lifecycle test;
- `docs/active/work/T-033-03-02/harness/run.sh` for the runner;
- `docs/active/work/T-033-03-02/harness/README.md` for usage and semantics;
- `docs/active/work/T-033-03-02/run-report.md` for observed evidence.

`crates/lisa-plugin/src/lib.rs` is clean at the start of this ticket. The work
directory does not yet exist. Exact-path isolated commits are required; the
ordinary index and unrelated changes must remain untouched.

## Research conclusion

The existing production state machine exposes every transition needed for a
repeatable field-style proof. The narrow missing layer is a multi-assignment
native scenario plus a runner that captures stable outcome rows into a durable
report. No scheduler behavior or provider contract needs to change.
