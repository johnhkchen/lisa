# Structure: T-034-03-01 deterministic split-brain regression

## Change inventory

### Modify

`crates/lisa-plugin/src/lib.rs`

Add one test to the existing private native test module.

No production method, struct, enum, constant, or public interface changes.

### Create

`docs/active/work/T-034-03-01/research.md`

`docs/active/work/T-034-03-01/design.md`

`docs/active/work/T-034-03-01/structure.md`

`docs/active/work/T-034-03-01/plan.md`

`docs/active/work/T-034-03-01/progress.md`

`docs/active/work/T-034-03-01/review.md`

### Delete

None.

## Test placement

Place the regression in `crates/lisa-plugin/src/lib.rs` inside `mod tests`.

Keep it near the existing stale-artifact and provenance acceptance regressions,
or at the end of the provenance/lease-focused test group.

The test name will describe the field scenario:

`split_brain_timeline_fences_old_attempt_and_admits_one_winner`

This name makes the scenario discoverable through a focused Cargo filter.

## Existing helpers consumed

### `install_current_attempt`

Used only for the predecessor fixture if production scheduling is not used for
the initial attempt.

It keeps thread, slot, current registry, and high-water registry aligned.

The successor must be created by `schedule_ready_tickets`, not the helper, so
the regression proves real redispatch behavior.

### `with_ledger`

Points provenance at a temp JSONL file.

### `read_ledger`

Returns typed rows for final attribution assertions.

### `fresh_slot` or direct `AgentSlot`

Create two known physical panes.

The predecessor slot is assigned to the ticket.

The replacement slot is unassigned, Idle, resident Codex, out of cooldown, and
quiet enough for immediate reuse.

## Fixture filesystem

The test owns one `tempfile::TempDir`.

Within it:

```text
tickets/
  T-SPLIT.md
work/
  T-SPLIT/
attempts/
  T-SPLIT/
    1/work/review.md
    2/work/review.md
signals/
  pane-1.*
provenance.jsonl
codex/
claude/
```

The canonical `work/T-SPLIT/review.md` must not exist until successor admission.

Predecessor and successor review bodies use different sentinel text.

## Fixture DAG

Write a single ticket file with:

- ID `T-SPLIT`;
- status `review`;
- phase `review`;
- `agent: codex`;
- no dependencies.

Scan it through `lisa_core::ticket::scan_tickets`.

Build `Dag::from_tickets` so scheduler readiness and completion verification use
the same production parsing path as other tests.

## Fixture state

Construct `State` with:

- the fixture DAG;
- fixture ticket/work paths;
- fixture signal and attempt paths;
- fixture provenance paths;
- permissions granted;
- slots discovered;
- `max_threads = 1`;
- `wind_down_secs = 0`;
- `session_timeout_secs = 1`;
- `stuck_threshold_secs = 1`;
- Codex as the default client.

Use pane IDs 1 and 2.

Pane 1 begins assigned, active, and Owned.

Pane 2 begins unassigned, Idle, with a resident Codex session.

## Predecessor thread block

Insert a `Thread::new("T-SPLIT", 1)`.

Set:

- client to Codex;
- phase to Review;
- start time well before the one-second budget;
- last activity well before the two-second hard-silence limit.

Install attempt 1.

Set pane 1 assignment to Owned after lease installation.

Write predecessor staged review bytes.

## Timeout assertion block

Call `check_session_timeouts`.

Assert:

- thread removed;
- current lease removed;
- pane 1 ticket and attempt cleared;
- pane 1 remains Fenced;
- pane 1 has no assignment;
- pane 1 cannot be selected as Idle;
- timeout alert names T-SPLIT;
- lifecycle ordering is revoke, fence, release;
- ledger contains one predecessor TimedOut row;
- row is fenced and non-authoritative.

Avoid asserting incidental activity-log wording except where needed to diagnose
the scenario.

## Redispatch assertion block

Prepare pane 2 as the sole eligible slot.

Call `schedule_ready_tickets`.

Capture attempt 2 from `current_leases`.

Assert:

- attempt 2 is exactly one generation newer;
- pane 1 remains Fenced and unassigned;
- pane 2 carries T-SPLIT and attempt 2;
- the thread runs on pane 2 with attempt 2;
- pane 2 is AssignedPendingAck;
- pane 2 is not Owned;
- only one slot has a ticket reservation;
- only one current lease exists for T-SPLIT.

The test intentionally does not call the successor acknowledgement yet.

## Resume replay block

Record replacement thread and pane liveness clocks before replay.

Create these pane-1 files:

- `.heartbeat` containing serialized attempt 1;
- `.ack` containing a tagged predecessor assignment payload;
- `.idle`;
- `.stopped`;
- `.cleared`;
- `.error`.

Call:

- `check_heartbeat_signals`;
- `check_codex_ack_signals`;
- `check_idle_signals`;
- `check_transition_signals`;
- `check_error_signals`;
- `check_artifact_advances`.

Also call `request_completion` directly with attempt 1 authority.

After replay, assert all candidate files are gone.

The replacement state must be byte-for-byte or value-for-value unchanged on
the safety-critical fields.

The old pane may accumulate a diagnostic activity log entry; log count is not
an ownership invariant and should not be over-specified.

## Artifact assertion block

Before successor staging exists, run artifact advancement.

Assert Review remains current and no pending completion exists.

Assert canonical review is absent.

Assert predecessor staging still contains predecessor sentinel bytes.

This is the negative cross-pane attribution proof.

Then write successor sentinel bytes and rerun artifact advancement.

Assert canonical review equals successor bytes exactly.

Assert pending completion authority equals attempt 2.

## Ownership assertion block

Build a matching Codex ack payload for attempt 2.

Pass it to `acknowledge_codex_assignment`.

Assert pane 2 transitions once to Owned.

Assert pane 1 never regains assignment state.

Count Owned seats in `seat_assignments`; require exactly one before completion.

The stale attempt-1 ack must already have returned false and left the pending
generation unchanged.

## Completion result block

After current artifact admission creates `PendingCompletion`:

1. update the fixture ticket to Done with the core ticket helper;
2. call `handle_completion_result` with exit code zero;
3. use a 40-byte ASCII hex commit ID;
4. call the handler a second time with the same data.

Assert the first callback completes and releases.

Assert the second callback is inert.

Do not shell out to a real Git transaction; that boundary is already covered by
T-031 tests and the provider-contract harness.

## Provenance assertion block

Read the ledger after successful completion.

Require two total rows:

1. predecessor TimedOut, fenced, non-authoritative;
2. successor Done, not fenced, authoritative.

Filter rows by `outcome == Done && authoritative` and require exactly one.

Require that row's attempt lease equals attempt 2.

No row may attribute Done to attempt 1.

## Test-only interface boundary

No new helper is expected.

If setup repetition makes the test unreadable, a small helper may be added
inside `mod tests`, but it must only construct fixture data and must not encode
the behavior under test.

Production visibility remains unchanged.

## Commit boundary

The meaningful implementation unit is the single regression in
`crates/lisa-plugin/src/lib.rs`.

Commit it with:

```text
lisa commit-ticket --ticket-id T-034-03-01 \
  --message "Test deterministic split-brain fencing" \
  --include crates/lisa-plugin/src/lib.rs
```

Do not include work artifacts in that source commit; Lisa owns their phase and
completion publication.

## Final structure

The code change remains a one-file, test-only addition.

Its internal sections mirror the field timeline so future reviewers can trace
the regression from predecessor timeout through the sole authoritative winner.
