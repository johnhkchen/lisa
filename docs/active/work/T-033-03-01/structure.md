# T-033-03-01 Structure — deterministic stall reproduction

## Change inventory

### Modified source

`crates/lisa-plugin/src/lib.rs`

- Add one `#[test]` function in the existing private scheduler test module.
- Reuse existing test builders, scheduler methods, state enums, activity enums,
  and Codex marker helper.
- Add no production items and change no production branches.

### Created workflow artifacts

`docs/active/work/T-033-03-01/research.md`

- Codebase and historical behavior map.
- Acceptance-event loss seam and existing test coverage.

`docs/active/work/T-033-03-01/design.md`

- Options and tradeoffs.
- Selected explicit dropped-signal regression.

`docs/active/work/T-033-03-01/structure.md`

- File and test organization blueprint.

`docs/active/work/T-033-03-01/plan.md`

- Ordered implementation and verification steps.

`docs/active/work/T-033-03-01/progress.md`

- Implementation log, checks, commit, and deviations.

`docs/active/work/T-033-03-01/review.md`

- Final reviewer handoff.

### Deleted files

None.

### Explicitly unchanged

- `docs/active/tickets/T-033-03-01.md` frontmatter;
- `crates/lisa-plugin/src/codex_ack.rs`;
- Codex acknowledgment fixtures;
- hook templates and generated hook documentation;
- scheduler configuration and timeout defaults;
- CLI and core crates;
- dashboard/UI code;
- live-style harness artifacts reserved for `T-033-03-02`.

## Module boundary

The implementation stays inside `crates/lisa-plugin/src/lib.rs`'s existing
`#[cfg(test)] mod tests`. That module already has privileged access to private
scheduler state and supplies all required imports through `use super::*`.

The new test is behavioral evidence, not a reusable production abstraction.
It should not add a public simulation API, widen enum visibility, or move
scheduler internals into a test support crate.

The only filesystem operation is against the temporary signal directory
already owned by the state builder's `TempDir` lifetime.

## Placement

Insert the new regression in the recycled-seat contract cluster, directly
after:

```text
test_recycled_codex_ownership_requires_matching_ack_exactly_once
```

and directly before:

```text
test_bounded_ack_wait_recovers_once_then_fails_actionably
```

This ordering reads as:

1. exact acknowledgment produces ownership;
2. the field failure drops that acknowledgment and demonstrates recovery;
3. generic scheduler recovery invariants remain exhaustively covered;
4. recovery acknowledgment can still produce ownership;
5. Claude reuse stays unchanged.

## Test interface

The test function has no parameters and returns unit:

```rust
#[test]
fn test_dropped_post_prompt_ack_reproduces_open_loop_stall_and_recovers_boundedly()
```

It uses the existing builder:

```rust
pane_name_schedule_state(
    "codex",
    AgentClient::Claude,
    Some(AgentClient::Codex),
)
```

The builder returns `(State, tempfile::TempDir)`. Retaining the temporary
directory binding keeps ticket and signal paths alive for the test duration.

## Internal organization of the test

The function is divided into five narrative blocks with short comments.

### 1. Deliver a generation-tagged reused-seat prompt

- Set the configured acknowledgment timeout to one second.
- Schedule `T-NAME` into resident Codex pane 10.
- Assert the clear handshake is pending and generation 1 has no deadline.
- Call `handle_cleared_signal(10)` to represent prompt delivery.
- Extract the exact generation-1 deadline from
  `AssignedPendingAck { generation: 1, ack_deadline: Some(...) }`.

This establishes the post-prompt boundary. The clock must be armed only here,
not while `/clear` is outstanding.

### 2. Materialize and drop the acceptance event

- Construct a `UserPromptSubmit` JSON object.
- Build its prompt with `codex_ack::tag_codex_assignment` for `T-NAME` and
  generation 1.
- Write it to `state.signal_dir.join("pane-10.ack")`.
- Assert the path exists, then remove it.
- Call `state.check_codex_ack_signals()`.
- Assert no acknowledgment activity entry was recorded.

No new fixture is needed because the dynamically generated identity is the
important part of this scheduler regression.

### 3. Contrast historical and current ownership

Derive the legacy open-loop ownership observation from actual state:

- the pane is reserved for `T-NAME`;
- the thread exists and is `Running`;
- the pane still has a session;
- transport is `Idle`;
- the acceptance signal is absent.

Bind this conjunction to
`legacy_open_loop_would_claim_ownership_without_ack` and assert it is true.
The name is part of the regression documentation.

Then assert current state remains generation-1 pending with the extracted
deadline and `seat_is_owned(10) == false`.

The historical oracle remains local to the test. It does not become a helper
or enum because no other behavior should depend on the deleted contract.

### 4. Prove finite one-shot recovery

- Evaluate `check_assignment_ack_timeouts_at(first_deadline)`.
- Assert generation 2, `Recovering`, no recovery deadline yet, and
  `WaitingForExit`.
- Assert the same ticket remains reserved and ownership remains false.
- Backdate `transition_started_at` past `AGENT_EXIT_GRACE_SECS`.
- Run `check_transition_timeouts()`.
- Extract the generation-2 recovery deadline.
- Count `ActivityEvent::SessionLaunch` records for `T-NAME` whose command
  contains the escaped generation-2 marker.
- Assert the count is one.
- Run transition evaluation again and assert the count remains one.

The launch count filter stays local to avoid expanding production or test
support APIs. Repetition is acceptable twice in this one test if it keeps the
assertion criteria visible.

### 5. Prove no silent fallback stall

- Do not write a generation-2 acknowledgment file.
- Evaluate at the recovery deadline.
- Assert `RecoveryFailed` and false ownership.
- Assert pane 10 still reserves `T-NAME`.
- Assert the retained thread is `Failed`.
- Assert `error_alerts` contains `(T-NAME, 10)`.
- Assert an `ActivityEvent::Error` mentions recovery failure and resetting the
  ticket.
- Evaluate again after the deadline and assert no additional recovery launch.

This final state is named, visible, and operator-actionable. It is not a silent
wait or automatic retry loop.

## Production interfaces exercised

The test directly calls existing private methods:

- `schedule_ready_tickets`;
- `handle_cleared_signal`;
- `check_codex_ack_signals`;
- `seat_assignment`;
- `seat_is_owned`;
- `check_assignment_ack_timeouts_at`;
- `check_transition_timeouts`.

It reads existing fields:

- `State::signal_dir`;
- `State::agent_slots`;
- `State::threads`;
- `State::activity_log`;
- `State::error_alerts`.

It matches existing types:

- `SeatAssignmentState`;
- `TransitionState`;
- `ActivityEvent`;
- `ThreadStatus`;
- `AgentClient`.

No signature, visibility, trait, serialization, or configuration contract
changes.

## State sequence blueprint

The expected current sequence is:

```text
unassigned resident Codex seat
  -> AssignedPendingAck { generation: 1, ack_deadline: None }
  -> prompt delivered
  -> AssignedPendingAck { generation: 1, ack_deadline: Some(D1) }
  -> valid pane-10.ack created then dropped
  -> scanner observes no event; still pending and unowned
  -> evaluate D1
  -> Recovering { generation: 2, ack_deadline: None }
  -> exit grace
  -> fresh generation-2 launch
  -> Recovering { generation: 2, ack_deadline: Some(D2) }
  -> no recovery ack; evaluate D2
  -> RecoveryFailed
```

Ticket identity remains `T-NAME` throughout. Generation identity changes once
at recovery. Ownership is false at every state in this failure scenario.

The legacy comparison is taken at the third line, where transport is idle and
the slot/thread appear active but the acceptance event is absent.

## Error and assertion boundaries

Pattern mismatches should panic with state-rich messages such as:

```text
expected armed original assignment, got {other:?}
expected armed recovery assignment, got {other:?}
```

Behavioral assertions should explain the invariant:

- legacy open-loop facts falsely claim ownership;
- dropped event cannot promote the current seat;
- current seat stays unowned before exact acceptance;
- one deadline initiates recovery;
- repeated polls cannot relaunch;
- the fallback terminates actionably.

Using precise messages makes CI failure useful without reproducing locally.

## Dependency and ordering constraints

The test depends on source landed by `T-033-01-04` at commit `f907a76`:

- `ack_deadline` in pending/recovering states;
- `check_assignment_ack_timeouts_at`;
- fresh recovery generation and launch;
- terminal recovery failure.

It also depends on earlier `S-033-01` commits for native event parsing,
generation tagging, acknowledgment signal scanning, and exact ownership gating.

No source change should precede the workflow artifacts because the requested
RDSPI sequence requires Research, Design, Structure, and Plan before Implement.

## Commit boundary

The source commit owns exactly:

```text
crates/lisa-plugin/src/lib.rs
```

Workflow artifacts and the ticket are not part of the agent's source commit.
Unrelated modified or untracked paths remain untouched. The isolated
`commit-ticket` transaction must leave the ordinary index unchanged.

## Structural conclusion

The ticket is a one-file, one-test source change supported by six workflow
artifacts. Its value comes from arranging existing production seams into a
single incident narrative, not from adding new scheduler architecture.
