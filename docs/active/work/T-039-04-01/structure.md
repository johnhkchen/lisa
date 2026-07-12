# Structure: deadline characterization tests

## Change summary

Modify one ticket-owned source file and create no production module. Add six
policy-oriented tests to the inline test module in
`crates/lisa-plugin/src/lib.rs`.

No files are deleted. No configuration, public interface, timeout value, or
runtime behavior changes. Attempt-private RDSPI artifacts are not part of the
source commit.

## Modified source file

### `crates/lisa-plugin/src/lib.rs`

Add a section labeled `Deadline policy characterization (T-039-04-01)` with:

- acknowledgement deadline characterization;
- transition deadline characterization;
- review deadline characterization;
- health deadline characterization;
- session deadline characterization;
- stale-thread deadline characterization.

Inline placement provides private access to `State`, `SeatAssignmentState`,
`AgentSlot`, `FailureTransitionOutcome`, private policy methods, and current
attempt helpers. No test-only public API is introduced.

## Fixture boundaries

### Acknowledgement fixture

Inputs are one installed current lease, one slot, a pending-ack seat with an
absolute deadline, and an awaiting-human marker. Invoke the check before and at
the deadline. Assert pending state before it, then successor generation,
recovery seat, exit transition, revoked predecessor authority, and cleared
awaiting marker at it.

### Transition fixture

Inputs are expired and unexpired `WaitingForExit` slots without tickets, plus
an expired `WaitingForClear` slot with recent activity. One invocation should
restore only the expired exit slot to idle. The fixtures avoid provider launch
and prompt-delivery branches.

### Review fixture

Inputs are three running Review threads past the phase budget: active, quiet
awaiting-human, and quiet eligible. Assert only the eligible ticket is recorded
in `finish_up_sent` and emits the typed finish-up activity event. Use the legacy
work-directory fallback when no current lease exists.

### Health fixture

Input is one running, threshold-silent thread on an awaiting-human pane. Assert
cached and logged `Stuck` health while the thread and human marker remain. This
establishes that human exemption belongs to destructive policies, not display.

### Session fixture

Inputs are active, awaiting-human, and reclaimable over-budget threads. Only the
reclaimable thread needs a current lease and occupied slot. Assert one typed
timeout outcome, removal and fencing for that ticket, survival and warning
tracking for both exemptions.

### Stale fixture

Inputs are an old-phase/recently-active thread, an awaiting-human silent thread,
and a reclaimable silent thread. Only the last needs a current lease and slot.
Assert one typed stale outcome and fencing, while both exempt fixtures remain.

## Interfaces deliberately unchanged

These current signatures remain intact:

- `check_assignment_ack_timeouts_at(SystemTime)`;
- `check_transition_timeouts()`;
- `check_review_timeouts()`;
- `evaluate_health()`;
- `check_session_timeouts()`;
- `detect_stale_threads()`.

The future evaluator, clock abstraction, and additional `_at` seams belong to
`T-039-04-02`.

## Test naming

Use shared `characterizes_...` names so one Cargo filter discovers the suite:

- `characterizes_acknowledgement_deadline_clock_and_recovery_action`;
- `characterizes_transition_deadline_and_active_session_exemption`;
- `characterizes_review_deadline_exemptions_and_finish_up_action`;
- `characterizes_health_deadline_as_observational_for_awaiting_human`;
- `characterizes_session_deadline_exemptions_and_timeout_action`;
- `characterizes_stale_deadline_exemptions_and_reclaim_action`.

## Ordering

1. Establish the unmodified-tree baseline.
2. Add the six-test section.
3. Format the source.
4. Run the shared test prefix.
5. Run the complete plugin and workspace tests.
6. Inspect the diff.
7. Commit the exact source path with Lisa.

The tests are one meaningful source unit because acceptance requires the full
policy matrix and they share one conflict-prone source file.

## Ownership

- Ticket-owned source: `crates/lisa-plugin/src/lib.rs` only.
- Existing `.lisa/provenance.jsonl` and ticket changes are machine-owned.
- Attempt work artifacts are managed by Lisa and excluded from source commit.
- The exact include path is `crates/lisa-plugin/src/lib.rs`.
- No ordinary Git index operation is used.

## Expected final tree

Production code remains behaviorally unchanged. The inline test module gains a
named characterization bracket that dependent evaluator work can run unchanged.
No ticket-owned source remains staged, modified, or untracked at handoff.
